# Phase 83: Machine Policy Spine + Egress Control - Context

**Gathered:** 2026-06-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Read a fail-secure machine egress allowlist from `HKLM\SOFTWARE\Policies\nono` into a **single** `MachineEgressPolicy` struct, and enforce it at **both** the `nono-proxy` (L7 FQDN) and `nono-wfp-service` (L3/4 kernel, per-AppContainer-SID) layers with no possible drift between layers. An admin pushes the allowlist via the GPO ADMX template phase 82 already shipped (`nono.admx`/`nono.adml`) or Intune OMA-URI.

Requirements covered: POLICY-01, POLICY-02, POLICY-03, EGRESS-01, EGRESS-02, EGRESS-03, EGRESS-04.

**In scope:** machine-policy registry read (64-bit view, fail-secure), `MachineEgressPolicy` deserialization + single-owner hand-off to both enforcement layers, WFP force-through-proxy SID confinement, proxy L7 FQDN allowlist sourced from machine policy, DNS-component wildcard matching hardening, ADMX AI-provider preset groups, `verify-dark.ps1 --gate egress-policy-deny`.

**Out of scope (other phases):** telemetry/compliance event logging (TELEM-* → Phase 84); any non-Windows machine-policy source (macOS/Linux have no `HKLM`); the daemon/agent lifecycle itself (shipped in earlier milestones).

</domain>

<decisions>
## Implementation Decisions

### WFP Enforcement Model (EGRESS-02, SC-3)
- **D-01:** **Force-through-proxy.** The kernel WFP layer does NOT resolve FQDNs to IPs. Instead, `nono-wfp-service` blocks the agent's AppContainer SID from ALL direct outbound traffic **except** the local proxy's loopback listener. The `nono-proxy` is the single L7 chokepoint that enforces the FQDN allowlist. WFP's job is to make proxy-bypass structurally impossible — robust against DNS/CDN IP churn and consistent with nono's existing per-SID WFP + proxy design.
- **D-02:** **WFP permit set = loopback proxy endpoint only.** For the agent SID, permit `127.0.0.1` / `::1` on the proxy's listener port only; block all other outbound. The agent must use the proxy for everything; the proxy performs DNS resolution and L7 FQDN filtering. (No direct DNS egress permit — DNS is proxied. If research finds an agent path that resolves outside the proxy, surface it; the default is loopback-proxy-only.)
- **D-03:** Verification (SC-3): out-of-list domain must be rejected at BOTH layers — the proxy rejects the proxied request to the out-of-list domain, AND WFP blocks a direct SID→out-of-list-IP connection that attempts to bypass the proxy. Both derive from the same `MachineEgressPolicy`.

### Single-Struct Hand-Off / No Drift (POLICY-03, EGRESS-02)
- **D-04:** **The daemon/CLI startup path is the SOLE HKLM policy reader.** It deserializes `MachineEgressPolicy` exactly once, configures the in-process proxy's allowlist directly, and passes the same struct (or its derived per-SID WFP permit instructions) to `nono-wfp-service` over the **existing control IPC** (the `\\.\pipe\nono-agentd-control` control plane). The WFP service NEVER reads the registry for egress policy. This structurally prevents two layers from reading divergent config.
- **D-05:** The `MachineEgressPolicy` **type lives in the nono core library** (canonical type), even though the daemon is the sole reader. Both layers consume the one deserialized instance.
- **D-06:** **Read timing = startup snapshot.** Read HKLM once at daemon/process startup and hold the snapshot for the process lifetime. A GPO policy change takes effect on the **next daemon restart** — document this restart-to-apply behavior explicitly. (Matches SC-1 "at process/daemon startup" and the read-at-startup decision in project memory; avoids per-read races and split-policy concurrent agents.)

### Failure Taxonomy + Precedence (POLICY-01, POLICY-02)
- **D-07:** **Fail-secure boundary:**
  - Key **ABSENT** → fall through to per-user config normally (this is NOT a failure).
  - Key **PRESENT but unreadable** (e.g. `ERROR_ACCESS_DENIED`) → **abort** with typed `NonoError::PolicyLoadFailed`. Never fall through to a permissive per-user state.
  - Key **PRESENT but malformed** (wrong `REG_*` type, bad UTF-16, unparseable value) → **also abort** with `NonoError::PolicyLoadFailed`. Malformed is treated identically to unreadable: once the key exists, any error fails secure.
- **D-08:** **Precedence = wholesale override.** When a valid machine policy is present, it FULLY REPLACES the per-user egress allowlist — the per-user `allow_domain` list is ignored entirely; only the machine allowlist is in effect. A per-user config can never widen the fleet allowlist. (Rejected: union, which would let per-user widen beyond admin policy and defeat fleet control. Intersection considered but override chosen for the cleanest "admin controls the fleet" story.)
- **D-09:** Registry read uses the **64-bit view** (`KEY_WOW64_64KEY`) regardless of the host process bitness (POLICY-01).
- **D-10:** `nono` gains the **`winreg`** crate as the registry reader (already earmarked in code comments from Phase 82: `provision_windows.rs`, `main.rs`, `health.rs`). Keep it Windows-cfg-gated so non-Windows targets still compile.

### ADMX Presets / Egress Allowlist Shape (EGRESS-01, EGRESS-04)
- **D-11:** **ADMX named toggles write group TOKENS, not literal FQDNs.** The ADMX exposes named enable toggles (e.g. "Allow Anthropic", "Allow OpenAI", "Allow GitHub API"); enabling one writes a stable group token into machine policy. nono owns the token→FQDN mapping and expands it when deserializing `MachineEgressPolicy`. Provider FQDN lists can then be updated in nono without re-issuing the ADMX template fleet-wide.
- **D-12:** **Preset token→FQDN map reuses the existing embedded `policy.json` groups** (`crates/nono-cli/data/policy.json`, embedded at build time via `build.rs`) — the same mechanism as nono's current built-in policy groups. One source of truth for group→FQDN; consistent with existing CLI profiles. (If research finds policy.json groups carry only filesystem/command semantics and not domain semantics, surface that — but default is to extend policy.json with egress/domain groups rather than a separate table.)
- **D-13:** The allowlist itself is authored as wildcard FQDNs (e.g. `*.anthropic.com`) per EGRESS-01; the policy's presence is what switches deny-by-default enforcement on. **RESOLVED 2026-06-18 (Option A, supersedes original `REG_MULTI_SZ` wording):** Phase 82's shipped ADMX uses a `<list>` element, which materializes as **N separate `REG_SZ` values under the `AllowedSuffixes\` / `AllowedHosts\` subkeys** (ADMX `<list>` cannot emit a single `REG_MULTI_SZ`). The Phase-83 reader **enumerates those REG_SZ subkey values** to match the already-shipped, fleet-pushable template — no ADMX rework. (Original "single `REG_MULTI_SZ` value" wording is corrected to "list of `REG_SZ` entries under the AllowedSuffixes/AllowedHosts subkeys"; EGRESS-01 requirement text reworded accordingly.)

### DNS-Component Matching (EGRESS-03)
- **D-14:** **Reuse + harden the existing matcher, don't rebuild.** The core `HostFilter` (`crates/nono/src/net_filter.rs:~222`) already does leading-dot suffix matching (`lower_host.ends_with(".anthropic.com") && len >`), which is already component-safe for the SC-4 reject set (`anthropic.com`, `evilanthropic.com`, `anthropic.com.evil.com` all correctly rejected). Phase 83 hardens this to explicit DNS-label comparison where useful, adds the SC-4 test matrix, and ensures the SAME matcher is the one fed by `MachineEgressPolicy` (proxy side). Any matching ambiguity fails secure (deny).

### Claude's Discretion
- Exact `NonoError::PolicyLoadFailed` variant shape and how `winreg` error kinds map onto unreadable-vs-malformed (D-07 principle holds regardless).
- The precise serialization of the per-SID WFP permit instructions over the control IPC (struct vs derived command), as long as it originates from the one deserialized `MachineEgressPolicy` (D-04).
- Whether to harden `HostFilter` to explicit `.split('.')` label comparison or keep the leading-dot `ends_with` form, provided the SC-4 matrix passes (D-14).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 83 section: goal + 5 success criteria (the prescriptive contract).
- `.planning/REQUIREMENTS.md` §POLICY-01..03, §EGRESS-01..04 — requirement text + the SEED-001/002 sources.

### Existing enforcement code to wire into
- `crates/nono/src/net_filter.rs` — core `HostFilter`: exact + wildcard-suffix matching, link-local + cloud-metadata deny. This is the L7 matcher the proxy wraps (EGRESS-03 lives here).
- `crates/nono-proxy/src/filter.rs` — `ProxyFilter` wrapping `HostFilter` with async DNS + DNS-rebinding protection; `new` / `new_strict` / `allow_all`. The machine allowlist feeds this.
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — the user-mode WFP install/cleanup service (per-SID filters under `NONO_SUBLAYER_GUID`); force-through-proxy SID confinement lands here.
- `crates/nono-cli/src/windows_wfp_contract.rs` — WFP contract types used by the service.
- `crates/nono-cli/src/health.rs` §`probe_machine_policy_windows` (~L375-411) — Phase 82's `reg query HKLM\SOFTWARE\Policies\nono` probe (present / unreadable / not_configured). The new `winreg` reader should align its present/unreadable semantics with this.

### Policy data + ADMX (Phase 82 outputs to extend)
- `crates/nono-cli/data/policy.json` — embedded built-in groups; preset token→FQDN groups (D-12) extend this.
- `dist/windows/nono.admx` + `dist/windows/nono.adml` — Phase 82 GPO template with `AllowedSuffixes`/`AllowedHosts`; EGRESS-04 preset toggles extend it.
- `scripts/build-windows-msi.ps1` — generates the machine `.wxs` and lays down the `HKLM\SOFTWARE\Policies\nono` sentinel (the `.wxs` is GENERATED from here-strings; edit the script, not the `.wxs`).

### Control plane (hand-off path, D-04)
- `crates/nono-cli/src/` daemon control loop (`\\.\pipe\nono-agentd-control`) — the existing IPC the daemon uses to drive `nono-wfp-service`; the `MachineEgressPolicy`-derived permit instructions ride this.

### Dark Factory gate
- `scripts/verify-dark.ps1` + `scripts/gates/` — the runner + gate contract; Phase 83 adds `egress-policy-deny` (SC-2 corrupted-key non-zero exit; SC-3 dual-layer deny). Clone the two-function contract from an existing gate (e.g. `scripts/gates/deploy-silent-install.ps1` / `clean-host-install.ps1`).
- `Skill("spike-findings-nono")` — engine-agnostic confinement patterns, WFP/AppContainer constraints + landmines.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `nono::HostFilter` / `ProxyFilter`: the L7 FQDN matcher + async-DNS wrapper already exist and already reject the SC-4 cases via leading-dot suffix matching. Phase 83 feeds them from machine policy and hardens, not rebuilds.
- Phase 82's `health.rs` `probe_machine_policy_windows` is the reference semantics for present/unreadable/not_configured on the same HKLM key.
- `policy.json` embedded-group mechanism is the home for AI-provider preset groups (D-12).
- The per-SID WFP filter machinery (`nono-wfp-service.rs`, `NONO_SUBLAYER_GUID`, `FwpmFilterAdd`) already adds/purges nono-owned filters — force-through-proxy adds a block-all-except-loopback-proxy filter for the agent SID.

### Established Patterns
- Library is policy-free; CLI/daemon owns policy. `MachineEgressPolicy` TYPE in core lib (D-05), but the READING + policy decisions live in the CLI/daemon (consistent with the library/CLI boundary in CLAUDE.md).
- Windows-cfg-gating: keep `winreg` + WFP wiring behind `#[cfg(target_os = "windows")]` with non-Windows stubs so cross-target clippy (Linux/macOS) still compiles (CLAUDE.md cross-target rule).
- Fail-secure on any unsupported/ambiguous shape (CLAUDE.md security principle) — D-07/D-14 are direct applications.

### Integration Points
- Daemon startup → HKLM read → `MachineEgressPolicy` → (a) `ProxyFilter` allowlist, (b) per-SID WFP permit instructions over control IPC.
- ADMX toggle → group token in `HKLM\SOFTWARE\Policies\nono` → `policy.json` group expansion → FQDN allowlist.

</code_context>

<specifics>
## Specific Ideas

- SC-4 reject matrix is the precise EGRESS-03 test contract: `api.anthropic.com` matches `*.anthropic.com`; `anthropic.com`, `evilanthropic.com`, and `anthropic.com.evil.com` are all rejected.
- SC-2 gate (`verify-dark.ps1 --gate egress-policy-deny`) must assert a **non-zero exit** on the corrupted/permission-denied-key path — the dark-factory proof that fail-secure is wired, not just coded.
- AI-provider presets to ship at minimum: `*.anthropic.com`, `*.openai.com`, `api.github.com` (EGRESS-04).

</specifics>

<deferred>
## Deferred Ideas

- Per-agent-launch policy re-read (rejected for Phase 83 in favor of the startup snapshot, D-06) — revisit only if managed-fleet GPO refresh cadence makes restart-to-apply painful.
- WFP FQDN→IP resolution enforcement (rejected in favor of force-through-proxy, D-01) — would only matter if a future phase needs WFP-level domain enforcement independent of the proxy.
- Telemetry/compliance config deserialized from the same policy source — POLICY-03 mentions "egress allowlist AND telemetry configuration," but the telemetry side is TELEM-* / Phase 84. Phase 83 should structure `MachineEgressPolicy` so Phase 84 can add a telemetry section to the SAME single read without re-architecting.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` — MSI VC++ runtime prerequisite; deployment/MSI concern, not egress policy. Out of scope.
- `20260611-poc-cert-broker-clean-host.md` — POC cert broker on clean host; cert-trust concern (Phase 82 area), not egress. Out of scope.
- `20260612-macos-rlimit-as-setrlimit-fails.md` — macOS RLIMIT defect; unrelated platform/runtime bug. Out of scope.

</deferred>

---

*Phase: 83-machine-policy-spine-egress-control*
*Context gathered: 2026-06-18*
