# Phase 83: Machine Policy Spine + Egress Control - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-18
**Phase:** 83-machine-policy-spine-egress-control
**Areas discussed:** WFP enforcement model, Single-struct hand-off, Failure taxonomy + precedence, ADMX preset shape

---

## WFP Enforcement Model

| Option | Description | Selected |
|--------|-------------|----------|
| Force-through-proxy | WFP blocks the agent SID from all direct outbound except the local proxy; proxy is the single L7 chokepoint. Robust against DNS/CDN IP churn; matches existing per-SID WFP + proxy design. | ✓ |
| Resolve FQDN→IP permit filters | At policy-load, resolve allowlist FQDNs→IPs, add per-IP WFP permits, block-all-else. Downside: DNS rotation/CDN drift + breakage. | |
| Both layers independently | WFP resolves IPs AND forces-through-proxy. Maximum defense-in-depth but most complex, IP-churn problem remains. | |

**User's choice:** Force-through-proxy.
**Notes:** Follow-up locked the WFP permit set to **loopback proxy endpoint only** (127.0.0.1 / ::1 on the proxy port; block everything else). DNS is proxied — no direct DNS egress permit. Proxy owns DNS resolution + L7 FQDN filtering. Verification proves dual-layer denial: proxy rejects the proxied out-of-list request; WFP blocks a direct SID→out-of-list-IP bypass attempt.

---

## Single-Struct Hand-Off

| Option | Description | Selected |
|--------|-------------|----------|
| Daemon reads once → hands to both | Daemon/CLI startup is the SOLE HKLM reader; deserializes once, configures in-proc proxy + passes same struct/derived permits to nono-wfp-service over existing control IPC. WFP service never touches the registry. | ✓ |
| Shared struct in core lib, each loads | Same loader CODE in both processes, but each still issues its own HKLM read — weaker no-drift guarantee. | |
| WFP service reads, daemon queries it | WFP service is sole reader, exposes policy to daemon via IPC. Inverts ownership; heavier service coupling. | |

**User's choice:** Daemon reads once → hands to both.
**Notes:** Follow-up locked **read timing = startup snapshot** (read HKLM once at daemon/process startup, hold for process lifetime; GPO change applies on next daemon restart — documented). `MachineEgressPolicy` TYPE lives in the core lib; the daemon is the sole reader.

---

## Failure Taxonomy + Precedence

| Option | Description | Selected |
|--------|-------------|----------|
| Absent=fallthrough; present-but-any-error=abort | Absent key → per-user fallthrough (not a failure). Present-but-unreadable OR malformed → abort with NonoError::PolicyLoadFailed. Any ambiguity once the key exists fails secure. | ✓ |
| Only unreadable aborts; malformed = empty deny | Malformed value treated as empty allowlist (deny-all) rather than abort. | |
| You decide | Let planning derive from how winreg surfaces each error kind. | |

**User's choice:** Absent=fallthrough; present-but-any-error=abort.
**Notes:** Separate precedence question → **wholesale override**: a valid present machine policy fully REPLACES the per-user allowlist (per-user `allow_domain` ignored entirely). Union rejected (lets per-user widen beyond admin policy); intersection considered but override chosen for the cleanest fleet-control story.

---

## ADMX Preset Shape

| Option | Description | Selected |
|--------|-------------|----------|
| ADMX checkboxes → nono expands named groups | ADMX named toggles write stable group TOKENS; nono owns token→FQDN expansion at deserialization. FQDN lists updatable in nono without re-issuing ADMX. Mirrors existing built-in policy groups. | ✓ |
| ADMX checkboxes write literal FQDNs | Toggles expand to literal FQDN strings into the REG_MULTI_SZ allowlist at GPO-apply time. Updating provider domains means re-issuing ADMX fleet-wide. | |
| Documented copy-paste values only | No toggles; admin pastes recommended FQDNs manually. Most error-prone. | |

**User's choice:** ADMX checkboxes → nono expands named groups.
**Notes:** Follow-up locked the **preset token→FQDN map to reuse the existing embedded `policy.json` groups** (`crates/nono-cli/data/policy.json`), same mechanism as current built-in groups — one source of truth for group→FQDN.

---

## Claude's Discretion

- Exact `NonoError::PolicyLoadFailed` variant shape and how winreg error kinds map onto unreadable-vs-malformed (the absent=fallthrough / present-error=abort principle holds regardless).
- Precise serialization of the per-SID WFP permit instructions over the control IPC, provided it originates from the one deserialized `MachineEgressPolicy`.
- Whether to harden `HostFilter` to explicit `.split('.')` label comparison or keep the leading-dot `ends_with` form, provided the SC-4 reject matrix passes.

## Deferred Ideas

- Per-agent-launch policy re-read (rejected in favor of startup snapshot) — revisit only if GPO refresh cadence makes restart-to-apply painful.
- WFP FQDN→IP resolution enforcement (rejected in favor of force-through-proxy).
- Telemetry config deserialized from the same policy source — telemetry side is TELEM-* / Phase 84; structure `MachineEgressPolicy` so Phase 84 can add a telemetry section to the SAME single read.
- Reviewed-not-folded todos: `20260611-msi-vcredist-prereq.md`, `20260611-poc-cert-broker-clean-host.md`, `20260612-macos-rlimit-as-setrlimit-fails.md` — all unrelated to egress policy.
