# 77-04 SUMMARY — CPLT-01 workspace-ancestor RA gap closure

**Plan:** 77-04 (`gap_closure: true`, `autonomous: false`)
**Requirements:** CPLT-01, CPLT-03
**Status:** Code deliverables (Task 1, Task 2) COMPLETE + tested. Host proof (Task 3): **the CPLT-01 confinement gap is VERIFIED CLOSED**; a real end-to-end PASS is **blocked by GitHub org policy** (Copilot CLI access denied for this account) → recorded **SKIP_HOST_UNAVAILABLE** (D-07), exit 3.

## Self-Check: PASS (code) / host-PASS blocked by external org policy

The gap this plan targeted is closed and proven by the failure-mode advance (below). The only thing standing between this and a green `copilot-e2e` PASS is an account/org limitation outside nono's control.

## What was delivered

### Task 1 — multi-target ancestor-RA guard (TDD) ✅
`AppliedAncestorReadAttributesGuard::snapshot_and_apply` refactored into `snapshot_and_apply_targets(&[&Path], sid)` (single-target is now a thin wrapper). It grants `FILE_READ_ATTRIBUTES` on the user-owned ancestors of **each** walk target, with **cross-target dedup** (a shared ancestor like `C:\Users\<user>` is granted/recorded/reverted exactly once) and the per-chain **D-04 stop** preserved (never touches `C:\Users`/`C:\` at runtime). Fail-closed revert-all + LIFO Drop unchanged.
- New Windows-gated tests: `ancestor_read_attributes_dedups_shared_ancestor_across_targets`, `ancestor_read_attributes_multi_target_covers_each_chain_and_stops_at_root`.
- The existing two RA tests were made **elevation-robust** (`take_ownership_for_current_user`): in an elevated session `tempdir()` is owned by `BUILTIN\Administrators`, which would stop the ownership walk and empty `applied` — a session artifact, not a logic failure. **4/4 `ancestor_read_attributes` tests pass.**
- Commits: `1f3ec278` (RED), `f1bf83ec` (GREEN).

### Task 2 — wire the workspace chain ✅
The AppContainer-arm RA guard build site in `mod.rs` now calls `snapshot_and_apply_targets(&[config.resolved_program, config.current_dir], sid)` — adding the `--workspace` (child CWD) chain so the workspace's user-owned ancestors get RA. Drop-order and AppContainer-arm gating unchanged. `cargo build --bin nono` clean. Commit `f370e2bb`.

### Task 3 — host proof ⚠️ gap VERIFIED CLOSED; real PASS org-policy-blocked → SKIP
Re-ran `verify-dark.ps1 --gate copilot-e2e` on Win11 with the fresh release build (nono 0.62.2), the CPLT-02 admin grant in place (idempotent), and the gate's workspace owned by the user.

**Failure-mode advance = proof the gap is closed.** Across the host runs the `realpathSync`/`lstat` denial climbed and then disappeared:
- pre-77-04 (77-03): `lstat 'C:\'` EPERM → after CPLT-02 grant: `lstat 'C:\Users'` → `lstat 'C:\Users\OMack'` EPERM (module-resolution crash, the recorded 77-03 FAIL).
- **post-77-04: the `lstat 'C:\Users\OMack'` EPERM is GONE.** The confined copilot now resolves its self-extracted Node package and runs **past** module resolution into Copilot's actual runtime. The CPLT-01 multi-target RA guard demonstrably closes the gap.

**Why no green PASS on this host:** the GitHub org (`TWGGLOBAL`) **denies Copilot CLI access** — unconfined, `copilot -p` exits with `Error: Access denied by policy settings … Your organization has restricted Copilot access`. A real suggestion is therefore unobtainable on this account regardless of confinement. This is a D-07 host/account precondition gap, not a confinement failure.

**Gate hardened on-host (OQ-3, commit `b94df657`):**
- `--allow-all-tools` added to the copilot invocation (`-p` alone is not fully non-interactive — Copilot prompts for tool permission and hangs the harness; this governs Copilot's own permission model only, NOT nono's OS confinement).
- Org-policy denial now classified as **SKIP_HOST_UNAVAILABLE** (D-07) via a bounded unconfined precondition probe + an Invoke-Gate output-marker check — so an org-restricted account SKIPs cleanly (exit 3) instead of being misread as a confinement-timeout FAIL. A provisioned host with Copilot CLI enabled proceeds to the confined run and a real PASS.

**Recorded verdict** (`.nono-runtime/verdicts/copilot-e2e.json`): `SKIP_HOST_UNAVAILABLE` — "GitHub Copilot CLI access denied by org policy — not a confinement failure (D-07)". Runner exit 3 (verified clean via `pwsh -File`).

## Commits
- `1f3ec278` test(77-04): multi-target ancestor-RA guard tests (RED)
- `f1bf83ec` feat(77-04): multi-target ancestor-RA guard (GREEN) — cover workspace chain
- `f370e2bb` feat(77-04): walk both binary + workspace chains in the RA guard
- `b94df657` fix(77-04): gate non-interactive flag + org-policy SKIP detection (OQ-3)

## Key files
- modified: `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` (multi-target guard + tests)
- modified: `crates/nono-cli/src/exec_strategy_windows/mod.rs` (dual-chain wiring)
- modified: `scripts/gates/copilot-e2e.ps1` (non-interactive flag + org-policy SKIP)

## Verification status vs. CPLT-03 success criteria
The plan's success criteria allow "PASS on a provisioned host **or a clearly-reasoned SKIP_HOST_UNAVAILABLE otherwise**." The SKIP here is clearly reasoned (org policy denies Copilot CLI). The substantive CPLT-01 confinement claim — confined Node/Copilot resolves modules with no `STATUS_ACCESS_DENIED`/module-resolution crash — is **verified** by the failure-mode advance past `lstat 'C:\Users\OMack'`. A literal green PASS requires an account/org with Copilot CLI enabled.

## Cross-target clippy
PARTIAL — all new symbols are `#[cfg(target_os = "windows")]`-gated; Linux/macOS cross-toolchain not installed on the Windows dev host. Deferred to live CI per `.planning/templates/cross-target-verify-checklist.md` (same disposition as 77-01).

## Next step (operator decision)
A real green `copilot-e2e` PASS requires running the gate on an account/host where GitHub Copilot CLI is org-enabled. The code gap is closed; the gate is correct (SKIP on org-restricted, PASS on enabled). Options: accept the verified-gap-closed + SKIP disposition to close CPLT-03/Phase 77, or defer the literal PASS to a Copilot-CLI-enabled account.
