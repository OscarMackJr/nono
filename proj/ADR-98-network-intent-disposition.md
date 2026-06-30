# ADR-98: #1225 NetworkIntent Disposition — Full-Sync-Adopt vs Fork-Diverge Carve-Out

**Status:** Accepted
**Phase:** 98 — UPST11 Divergence Audit
**Date:** 2026-06-30
**Authors:** Phase 98 execution

---

## Context

Upstream `nolabs-ai/nono` PR #1225 (`72bcfd66`, "refactor(network): introduce NetworkIntent and
remove ProxyOnly placeholders") introduces a new `pub(crate) enum NetworkIntent` in the CLI
layer of the v0.65.1 → v0.66.0 window. This ADR settles whether the fork adopts the refactor
(full-sync-adopt) or writes a deliberate fork-divergence carve-out preserving the existing
`NetworkMode::ProxyOnly { port: 0 }` placeholder pattern.

The decision gates Phase 99 (UPST11 absorb). It is settled here — before any cherry-pick —
per D-04 (open, diff-grounded, no pre-commitment). See the DIVERGENCE-LEDGER
(`.planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md`) Cluster A for the
full per-commit audit; this ADR is the standalone analysis citable after ledger archival (D-06).

### Upstream #1225 Shape (from `git show 72bcfd66`)

Commit `72bcfd66` (author: Aleksy Siek, date: 2026-06-24) is a CLI-only refactor across
**11 files, all under `crates/nono-cli/src/`**. No changes to the core library
(`crates/nono/src/`), no changes to `crates/nono-proxy/src/`.

**New type introduced:**

```rust
// crates/nono-cli/src/sandbox_prepare.rs (upstream post-#1225)
pub(crate) enum NetworkIntent {
    Unrestricted,
    BlockAll,
    ProxyFiltered(BoxedProxyLaunchOptions),
}
```

Additional `pub(crate)` items added within `crates/nono-cli/src/`:
- `pub(crate) fn is_proxy_active()` — replaces checking `NetworkMode::ProxyOnly { .. }` match
- `pub(crate) fn proxy_options()` — extracts `BoxedProxyLaunchOptions` from `NetworkIntent`
- `pub(crate) strict_filter: bool` field on `ProxyLaunchOptions` (renamed from `network_block`)
- `pub(crate) network: NetworkIntent` field on the pending-sandbox struct

**What #1225 replaces:**

The `ProxyOnly { port: 0 }` placeholder pattern — setting `CapabilitySet` to
`NetworkMode::ProxyOnly` with `port: 0` as a pre-start signal before the proxy has a real port:

```rust
// Upstream pre-#1225 (also: current fork pattern in capability_ext.rs:1027)
} else if profile.network.has_proxy_flags() {
    caps = caps.set_network_mode(nono::NetworkMode::ProxyOnly {
        port: 0,       // placeholder: real port written when proxy starts
        bind_ports,
    });
}
```

Post-#1225, the CLI layer represents resolved user intent as `NetworkIntent` before any proxy
starts. `CapabilitySet` only receives `NetworkMode::ProxyOnly` with a real port once the proxy
is actually listening. The `profile_network_block` side channel on `PreparedSandbox` is replaced
by reading `args.block_net` directly at proxy-launch time; `ProxyLaunchOptions.network_block`
is renamed to `strict_filter`.

**Upstream files touched (11):** `capability_ext.rs`, `command_runtime.rs`,
`execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `output.rs`, `profile/mod.rs`,
`proxy_runtime.rs`, `sandbox_prepare.rs`, `supervised_runtime.rs`, `terminal_approval.rs`.

**Windows-touch:** 7 of 11 files contain `cfg(target_os = "windows")` blocks: `capability_ext.rs`,
`supervised_runtime.rs`, `command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`,
`output.rs`, `terminal_approval.rs`.

**Companion commit `d457ecc3` (#1263):** Adds `validate_block_net_conflicts()` in
`sandbox_prepare.rs`, called from `command_runtime` and `launch_runtime` before any sandbox or
proxy setup. Uses `NetworkIntent` types directly. Its adoption is gated on #1225's outcome:
if adopted together, both commits apply; if fork-diverge, `d457ecc3`'s validation logic must
be adapted to work without `NetworkIntent`.

**Confirmed absent from fork:** `grep -rl NetworkIntent crates/` → empty. The fork has no
`NetworkIntent` type.

---

## Fork Touchpoint Map

The surface #1225 replaces has deep roots across all three areas of the fork. Every touchpoint
is categorized below.

### Category 1: Library Core — `crates/nono/src/` (NOT touched by #1225; must remain unchanged)

`NetworkMode::ProxyOnly` is a **library type** defined in the policy-free sandbox primitive.
#1225 leaves it entirely intact; the ADR-86 boundary is not threatened by either option.

| File | Line(s) | Role |
|------|---------|------|
| `crates/nono/src/capability.rs` | 707 | Doc: defines `ProxyOnly` network mode semantics |
| `crates/nono/src/capability.rs` | 827 | `NetworkMode::ProxyOnly { port: u16, bind_ports: Vec<u16> }` variant definition |
| `crates/nono/src/capability.rs` | 843 | Match arm in `is_network_blocked()` |
| `crates/nono/src/capability.rs` | 1045 | `set_network_mode(NetworkMode::ProxyOnly { port: 0, bind_ports })` constructor |
| `crates/nono/src/capability.rs` | 1065 | `set_network_mode(NetworkMode::ProxyOnly { port, ... })` constructor |
| `crates/nono/src/capability.rs` | 1380, 1386 | Doc + match arm: `ProxyOnly` counts as restricted in `is_network_restricted()` |
| `crates/nono/src/capability.rs` | 2711–2856 | Test uses of `NetworkMode::ProxyOnly` |
| `crates/nono/src/manifest_convert.rs` | 47 | `NetworkMode::Proxy => InternalNetworkMode::ProxyOnly { ... }` — manifest conversion |
| `crates/nono/src/sandbox/linux.rs` | 386, 620 | Doc comments: ProxyOnly handled by Landlock V4+ or seccomp fallback |
| `crates/nono/src/sandbox/linux.rs` | 700, 763–775 | `SeccompNetFallback::ProxyOnly` — pre-V4 kernel seccomp fallback for ProxyOnly |
| `crates/nono/src/sandbox/linux.rs` | 985 | Note: ProxyOnly not installed inline (requires notify fd) |
| `crates/nono/src/sandbox/linux.rs` | 1996, 2023 | `SeccompNetFallback::ProxyOnly` variant definition + conversion from `NetworkMode::ProxyOnly` |
| `crates/nono/src/sandbox/linux.rs` | 3890–4558 | Tests for ProxyOnly + Landlock V4 / pre-V4 |
| `crates/nono/src/sandbox/macos.rs` | 720, 755, 790, 795, 1276, 1513 | Seatbelt ProxyOnly: TCP localhost allow rule generation |
| `crates/nono/src/sandbox/mod.rs` | 305, 649 | `WindowsNetworkPolicyMode::ProxyOnly` variant + doc (separate type for Windows sandbox) |
| `crates/nono/src/sandbox/windows.rs` | 173, 333 | `WindowsNetworkPolicyMode::ProxyOnly` match arms (Windows sandbox application) |

**Library note:** `NetworkMode::ProxyOnly` is a stable, library-defined type. It carries the
enforcement-time values (`port`, `bind_ports`) and is the signal to the sandbox backend to apply
network restriction. This type is NOT a "placeholder" — it is the canonical enforcement enum.
The placeholder usage exists only in the CLI layer (`capability_ext.rs:1027`) where `port: 0`
is written before the proxy has started. #1225 eliminates that placeholder from the CLI; it does
not change the library type or how any sandbox backend (Linux seccomp, macOS Seatbelt, Windows
WFP/AppContainer) interprets it.

### Category 2: CLI Conflict Zone — `crates/nono-cli/src/` (the surface #1225 modifies)

These are the files the fork holds today that conflict with or are refactored by #1225.

| File | Line(s) | Role | #1225 conflict? |
|------|---------|------|----------------|
| `capability_ext.rs` | 1027 | `ProxyOnly { port: 0 }` placeholder set on profile proxy-flag path | YES — removed |
| `capability_ext.rs` | 1182 | `ProxyOnly { port, bind_ports }` constructor (real port path) | YES — adapted |
| `capability_ext.rs` | 635, 1043, 1263 | Error strings referencing ProxyOnly mode | Minor — text |
| `command_runtime.rs` | 342 | `nono::NetworkMode::ProxyOnly { .. }` match arm for dry-run path | YES — adapted |
| `execution_runtime.rs` | 230, 235, 416 | ProxyOnly match arms for proxy activation detection | YES — key conflict |
| `exec_strategy.rs` | 332, 426 | `LinuxNetworkNotifyMode::ProxyOnly` variant (Linux seccomp notify) | No conflict (different enum) |
| `exec_strategy.rs` | 1377, 4910–5593 | ProxyOnly usage in supervisor loop + tests | No conflict (library type) |
| `exec_strategy/supervisor_linux.rs` | 1531 | `LinuxNetworkNotifyMode::ProxyOnly` | No conflict |
| `output.rs` | 186 | `NetworkMode::ProxyOnly { port, bind_ports }` display arm | YES — `proxy_pending` param added |
| `proxy_runtime.rs` | 113, 297, 500 | ProxyOnly match arms and constructor (post-proxy-start activation) | YES — NetworkIntent-based |
| `profile/mod.rs` | 1975, 1984, 2097 | `WSL2ProxyFallback` — fork-only WSL2 ProxyOnly fallback policy | Fork-only; #1225 doesn't touch it |
| `query_ext.rs` | 243, 266, 1009–1169 | ProxyOnly query routing + test uses | Minor (library type) |
| `sandbox_prepare.rs` | 105 | `network_block_requested` field reference | YES — renamed to `profile_network_block` |
| `sandbox_state.rs` | 153 | `NetworkMode::ProxyOnly` constructor for state restoration | No conflict (library type) |
| `supervised_runtime.rs` | 173, 357, 362, 369 | ProxyOnly display + port extraction | YES — adapted |
| `why_runtime.rs` | 99 | Comment about ProxyOnly filtering | Minor |
| `diagnostic/formatter.rs` | 2027, 2613 | ProxyOnly display + test use | Minor (library type) |

### Category 3: CLI Windows Path — `exec_strategy_windows/network.rs` (ADR-86 carve-out zone)

| File | Line(s) | Role |
|------|---------|------|
| `exec_strategy_windows/network.rs` | 466 | `WindowsNetworkPolicyMode::ProxyOnly { port, bind_ports }` denial message formatting |
| `exec_strategy_windows/network.rs` | 496 | `WindowsNetworkPolicyMode::ProxyOnly { .. }` → "proxy-only" label |
| `exec_strategy_windows/network.rs` | 500 | `WindowsNetworkPolicyMode::ProxyOnly { port, bind_ports }` WFP setup |
| `exec_strategy_windows/network.rs` | 514 | `WindowsNetworkPolicyMode::ProxyOnly { .. }` → "activate_proxy_mode" |
| `exec_strategy_windows/network.rs` | 1512, 1624 | `WindowsNetworkPolicyMode::ProxyOnly` match arms |

**Windows backend note:** The Windows exec path reads `WindowsNetworkPolicyMode::ProxyOnly`
(a separate `#[repr(C)]`-compatible type derived from `NetworkMode::ProxyOnly` at the sandbox
boundary in `sandbox/windows.rs:333`). This type is NOT touched by #1225. Even under
full-sync-adopt, `exec_strategy_windows/network.rs` requires no changes — it reads the
enforcement-time type, not the CLI intent type. The ADR-86 D-02 carve-out for Windows denial
rendering stays fully intact regardless of which option wins.

### Category 4: Phase 95 Endpoint-Policy Surface — `crates/nono-proxy/src/` + `crates/nono-cli/src/network_policy.rs`

The fork's v3.3 Phase 95 `CompiledEndpointPolicy`/`endpoint_policy.evaluate()` surface lives
in the proxy crate and is NOT directly modified by #1225. However, `proxy_runtime.rs` is in
both the #1225 11-file set and the Phase 95 endpoint-policy surface — the two changes must be
reconciled in Phase 99.

| File | Line(s) | Role |
|------|---------|------|
| `crates/nono-proxy/src/config.rs` | 272, 350, 486 | `CompiledEndpointPolicy` struct + impl + Debug |
| `crates/nono-proxy/src/config.rs` | 1239–1359 | `CompiledEndpointPolicy::compile()` and evaluation tests |
| `crates/nono-proxy/src/route.rs` | 13, 41, 91, 487 | `CompiledEndpointPolicy` import, field, construction, test |
| `crates/nono-proxy/src/reverse.rs` | 121 | Comment about `CompiledEndpointPolicy::compile()` deny-default |
| `crates/nono-cli/src/network_policy.rs` | all | `NetworkPolicy`, `CompiledEndpointPolicy` resolver |

**Endpoint-policy note:** `crates/nono-cli/src/network_policy.rs` has zero hits from #1225
(confirmed: Check 9 in the ledger's Empirical Cross-Check). The conflict is indirect: #1225
rewrites `proxy_runtime.rs` (NetworkIntent-based launch logic), and the fork's
`CompiledEndpointPolicy` wiring also passes through `proxy_runtime.rs`. Phase 99 must verify
the endpoint-policy `evaluate()` chain survives the NetworkIntent adoption.

---

## Options Considered

### Option A: Full-Sync-Adopt — Adopt upstream's `NetworkIntent` refactor

**Description:** Cherry-pick `72bcfd66` (#1225) and `d457ecc3` (#1263) in Phase 99, adapting
the fork-specific surfaces (WSL2ProxyFallback, CompiledEndpointPolicy wiring) as deviations.

**Affected fork invariants:**

| Invariant | Impact | Non-regression requirement |
|-----------|--------|--------------------------|
| ADR-86 policy-free library boundary | None — `NetworkMode::ProxyOnly` in `crates/nono/src/` is untouched; the library applies only what clients add to `CapabilitySet` | Preserved by construction: #1225 is CLI-side only |
| Windows WFP/AppContainer network model | Minimal — `exec_strategy_windows/network.rs` reads `WindowsNetworkPolicyMode::ProxyOnly` (enforcement-time, from library), not the CLI intent type | `exec_strategy_windows/` requires no changes; ADR-86 D-02 carve-out stays intact |
| v3.3 Phase 95 `CompiledEndpointPolicy`/`evaluate()` surface | Moderate — `proxy_runtime.rs` is in both the #1225 diff and the endpoint-policy surface | Phase 99 MUST verify endpoint-policy chain: apply #1225 `proxy_runtime.rs` changes, then confirm `CompiledEndpointPolicy::compile()` → `evaluate()` chain operates correctly; Phase 89 proxy guard tests must pass |
| `WSL2ProxyFallback` (fork-only, `profile/mod.rs:1975`) | Minor — `profile/mod.rs` is in the #1225 diff; the fork's WSL2 policy block must be preserved as a fork-only extension | Phase 99 MUST preserve `Wsl2ProxyFallback::Refuse` / `Wsl2ProxyFallback::Allow` alongside `NetworkIntent` |

**cfg-gated surface flag for Phase 99:** 7 of 11 #1225-touched files contain
`cfg(target_os = "windows")` blocks. Phase 99 executor MUST run:
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`

Windows-host `cargo clippy` is NOT a substitute (CLAUDE.md MUST rule; see
`.planning/templates/cross-target-verify-checklist.md`).

**Pros:**
- Maximum convergence: eliminates the `ProxyOnly { port: 0 }` placeholder ambiguity across the
  entire CLI layer — same benefit upstream obtained.
- Consistent with Phase 86 precedent: adopted upstream's high-conflict 11-file + 2200-LOC
  boundary refactor; a CLI-only 11-file + 184-line refactor is lower-risk.
- Future upstream syncs: the CLI's proxy-intent surface aligns with upstream, reducing per-sync
  friction on `proxy_runtime.rs`, `execution_runtime.rs`, `supervised_runtime.rs` (the files
  most likely to evolve in future upstream proxy work).
- `d457ecc3` (`validate_block_net_conflicts`) is independently valuable safety logic — it can
  be adopted cleanly alongside `72bcfd66`.
- Library boundary is NOT threatened: `NetworkMode::ProxyOnly` in `capability.rs` and all
  sandbox backends remain exactly as-is.

**Cons:**
- Phase 99 cherry-pick is moderately complex: 11 files, 7 with cfg(windows) blocks, mandatory
  cross-target clippy gate, CompiledEndpointPolicy compatibility check.
- WSL2ProxyFallback adaptation requires fork-specific deviation work.
- Companion `d457ecc3` uses NetworkIntent types, so both commits must be absorbed together.

---

### Option B: Fork-Divergence Carve-Out — Preserve `NetworkMode::ProxyOnly { port: 0 }` pattern

**Description:** Skip `72bcfd66` and `d457ecc3` entirely. Record as a permanent fork-divergence
carve-out. The `ProxyOnly { port: 0 }` placeholder pattern remains in `capability_ext.rs` and
throughout the CLI.

**Affected fork invariants:**

| Invariant | Impact | Non-regression requirement |
|-----------|--------|--------------------------|
| ADR-86 policy-free library boundary | None — skipping #1225 does not alter the library boundary either | Preserved trivially |
| Windows WFP/AppContainer network model | None — `exec_strategy_windows/network.rs` is unmodified | Preserved trivially |
| v3.3 Phase 95 `CompiledEndpointPolicy`/`evaluate()` surface | None from skipping #1225 | Preserved trivially; BUT: `d457ecc3`'s validation logic, if desired independently, requires adaptation |
| `WSL2ProxyFallback` | None | Preserved trivially |

**cfg-gated surface flag for Phase 99:** No cross-target clippy obligation from Cluster A;
other clusters (D, Cluster A companion work) still require it.

**Pros:**
- Zero Phase 99 effort for Cluster A beyond documentation.
- No risk of CompiledEndpointPolicy regression during adoption.

**Cons:**
- Creates **permanent divergence** on a 11-file CLI surface. Every future upstream sync
  that touches `proxy_runtime.rs`, `execution_runtime.rs`, `capability_ext.rs`, or
  `supervised_runtime.rs` will generate false conflicts against the preserved `ProxyOnly { port: 0 }`
  pattern — the same reconciliation overhead as the Phase 86 Windows carve-out, but for a much
  more actively evolving surface.
- The `ProxyOnly { port: 0 }` placeholder is a semantic confusion upstream has already corrected.
  Preserving it means the fork carries a known code smell indefinitely.
- Loses `d457ecc3`'s `validate_block_net_conflicts()` safety logic (or requires independent
  re-implementation without `NetworkIntent` types).
- Contradicts the Phase 86 convergence precedent: that decision specifically adopted upstream's
  high-conflict CLI + library boundary refactor to reduce per-sync friction. A carve-out on a
  strictly smaller CLI-only refactor inverts that logic.

---

## Decision

**Full-sync-adopt** (Option A).

The fork adopts upstream `72bcfd66` (#1225) and `d457ecc3` (#1263) in Phase 99, with the fork-specific
deviations enumerated in the Consequences section.

The adopt-vs-diverge question reduces to two key facts established by actual-diff inspection:

1. **The library boundary is not threatened.** `NetworkMode::ProxyOnly` in `crates/nono/src/`
   is untouched by #1225. This is the decisive constraint from ADR-86: adopt is safe precisely
   because the refactor is CLI-only.

2. **The Windows network model is not threatened.** `exec_strategy_windows/network.rs` reads
   `WindowsNetworkPolicyMode::ProxyOnly` — an enforcement-time library type derived from
   `NetworkMode::ProxyOnly` at the sandbox boundary — not the CLI intent type. No changes to
   `exec_strategy_windows/` are required under adoption. The ADR-86 D-02 Windows denial-rendering
   carve-out remains fully intact.

Given these two non-regressions hold regardless of option chosen (since both options leave the
library type untouched), the decision is whether to carry permanent CLI divergence (Option B)
or invest one Phase 99 moderately-complex cherry-pick (Option A). Phase 86 precedent and the
per-sync maintenance cost tip clearly to Option A.

The `CompiledEndpointPolicy` compatibility check and WSL2ProxyFallback preservation are
tractable Phase 99 tasks; they are deviations to manage, not blockers that require a carve-out.

---

## Consequences

### Phase 99 absorb implications (full-sync-adopt)

**Commits to apply (in order):** `72bcfd66` (#1225) then `d457ecc3` (#1263). Both Cluster A
commits apply together — they are designed as a pair.

**Touchpoints that change:**
- `capability_ext.rs`: Remove `ProxyOnly { port: 0 }` placeholder on profile/CLI proxy-flag
  paths; the proxy-intent is now carried by `NetworkIntent::ProxyFiltered` instead.
- `execution_runtime.rs`: Replace `matches!(caps.network_mode(), NetworkMode::ProxyOnly { .. })`
  proxy-activation checks with `NetworkIntent`-based checks.
- `proxy_runtime.rs`: Replace placeholder-based proxy launch with `NetworkIntent`-based launch;
  MUST verify `CompiledEndpointPolicy::compile()` → `evaluate()` chain is preserved through
  the new proxy-launch path.
- `sandbox_prepare.rs`: Rename `network_block_requested` → `profile_network_block`; verify
  `validate_block_net_conflicts()` adapts cleanly.
- `output.rs`: Add `proxy_pending` parameter to `print_capabilities`.
- `supervised_runtime.rs`: Adapt port/bind_ports extraction to `NetworkIntent`-aware path.

**Guard tests / invariants that must be preserved:**

| Guard test | Location | Guard |
|-----------|----------|-------|
| `proxy_activates_with_custom_credentials_only` | Phase 89 / `73bd03a6` | Proxy activation predicate must survive NetworkIntent adoption |
| `block_net_overrides_custom_credentials_activation` | Phase 89 | block-net + proxy conflict handling |
| `build_proxy_config_maps_upstream_proxy_to_external_proxy` | Phase 89 | ProxyConfig construction |
| `connect_keeps_open_on_missing_proxy_auth` | Phase 89 | Credential route on proxy |
| `denied_endpoint_returns_403_and_audit` | Phase 89 | CompiledEndpointPolicy evaluate() chain |
| `allow_domain_endpoint_route_does_not_shadow_credential_route` | Phase 89 | Route resolution order |
| `proxy_no_v4_seccomp` / `proxy_v4_no_seccomp` | Phase 95/96 | ProxyOnly on Linux V4+ kernel |
| `verify_empty_log_with_no_stored_metadata_is_not_valid` | Phase 87 (CR-02) | Audit guard — unrelated to #1225 but must remain passing |

**ADR-86 non-regression guarantee:**
`NetworkMode::ProxyOnly` in `crates/nono/src/capability.rs` (variant definition, constructors,
match arms), `sandbox/linux.rs` (seccomp fallback), `sandbox/macos.rs` (Seatbelt rules), and
`sandbox/windows.rs` (`WindowsNetworkPolicyMode::ProxyOnly` derivation) are ALL UNCHANGED.
The library remains policy-free. No audit, diagnostic, or network-policy logic moves into or
out of the library as a result of adopting #1225.

**Windows network model non-regression guarantee:**
`exec_strategy_windows/network.rs` reads `WindowsNetworkPolicyMode::ProxyOnly` — derived from
the library's `NetworkMode::ProxyOnly` at sandbox application time (not at CLI intent time).
The WFP filter registration path (`activate_proxy_mode`), the ProxyOnly denial message, and the
AppContainer SID-scoped network policy are all driven by the enforcement-time type, not by
`NetworkIntent`. No changes to `exec_strategy_windows/` are required under adoption. The
ADR-86 D-02 carve-out for Windows denial rendering remains in force.

**Fork-specific deviations Phase 99 must track:**

1. **WSL2ProxyFallback preservation** — `profile/mod.rs:1975` (`Wsl2ProxyFallback::Refuse`,
   `Wsl2ProxyFallback::Allow`, `wsl2_proxy_fallback: Wsl2ProxyFallbackPolicy`) is fork-only
   extension. #1225 modifies `profile/mod.rs` (NetworkIntent-related changes). Phase 99
   must preserve the WSL2 policy block as a fork-extension alongside the NetworkIntent adoption.

2. **CompiledEndpointPolicy compatibility** — `proxy_runtime.rs` appears in both the #1225
   diff and the Phase 95 endpoint-policy surface. Phase 99 must apply #1225's
   NetworkIntent-based launch logic AND verify `CompiledEndpointPolicy::compile()` →
   `evaluate()` chain operates through the new path. Guard test
   `denied_endpoint_returns_403_and_audit` is the primary verification signal.

3. **`d457ecc3` adaptation if any conflicts** — `validate_block_net_conflicts()` is additive
   safety logic; it should apply cleanly. If it surfaces conflicts with the fork's
   `profile_network_block`-renamed field, resolve inline (not via skip).

**Cross-target clippy mandate (CLAUDE.md MUST rule):**
Both commits touch files containing `cfg(target_os = "linux")`, `cfg(target_os = "macos")`, and
`cfg(target_os = "windows")` blocks. After applying Cluster A, Phase 99 executor MUST run:

```bash
# linux-gnu gate (Docker + cross)
cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used

# apple-darwin gate (zig + cargo-zigbuild, SDKROOT unset)
cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

Windows-host `cargo clippy` is NOT a substitute for either gate. See
`.planning/templates/cross-target-verify-checklist.md` (single source of truth).

### Future upstream sync guidance

> **Cluster A (both commits) is a full-sync-adopt** — there is no ongoing fork-diverge
> carve-out for the `NetworkIntent` surface. Future upstream sync auditors should NOT treat
> `NetworkMode::ProxyOnly` in `crates/nono/src/` as a conflict source for Cluster A — the
> library type is stable and unchanged by the adoption. The fork divergences that DO require
> ongoing carve-out tracking are: (1) `WSL2ProxyFallback` in `profile/mod.rs` (fork-only
> extension); (2) `CompiledEndpointPolicy` / `endpoint_policy.evaluate()` in
> `crates/nono-proxy/src/` (fork-added, Phase 95); (3) `exec_strategy_windows/` Windows denial
> rendering (ADR-86 D-02 deliberate carve-out). None of these are introduced by adopting #1225;
> all pre-existed and are preserved.

---

## References

- Upstream commit: `72bcfd66` — `refactor(network): introduce NetworkIntent and remove ProxyOnly placeholders (#1225)`
- Upstream commit: `d457ecc3` — `fix(network): error early on contradictory network flag combinations (#1263)` (companion)
- DIVERGENCE-LEDGER Cluster A: `.planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md` §Cluster A
- ADR-86: `proj/ADR-86-library-boundary-convergence.md` — the policy-free boundary + adopt precedent
- ADR-87: `proj/ADR-87-cr02-audit-bypass.md` — CR-02 format reference (ADR naming convention)
- Phase 95 endpoint-policy surface: v3.3 `.planning/milestones/v3.3-phases/95-upst10-absorb/95-06-SUMMARY.md`
- Phase 89 proxy guard tests: commit `73bd03a6` (Phase 89 reconciliation addendum)
- Cross-target verify checklist: `.planning/templates/cross-target-verify-checklist.md`
- Phase 99 routing: DIVERGENCE-LEDGER §ADR Review → "Cluster A → Phase 99 after ADR settles; cross-target clippy MUST"
