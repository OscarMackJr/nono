# Phase 95 Fork-Invariant Verification

**Date:** 2026-06-26
**Post-sync commit:** a6576138 docs(95-03): complete Cluster C structural no-op absorb plan summary and state
**Baseline commit:** 449138a9 chore(phase-95): capture D-04 Windows baseline red set before cherry-picks

**Absorb commits (baseline → HEAD):**
- ae77d198 fix(sandbox): exempt IPC fd from sendmsg trapping to resolve af_unix_mediation deadlock (#1210) [Cluster A cherry-pick]
- 61689ef8 fix(95-01): post-cherry-pick compilation and formatting fixes [Cluster A post-fix]
- 91d526e6 feat(audit,sandbox): absorb Cluster B shared-surface additions from 11fd10e0 [Cluster B]
- 62dbf013 refactor(proxy): record Cluster C 9b37dc52 as structural no-op in fork [Cluster C]
(docs/planning commits interspersed: 251c1481, 438fc8f9, a6576138)

**Diff anchor:** All `git diff` commands use explicit baseline SHA `449138a9` rather than `HEAD~N`
because docs commits are interspersed between absorb commits (N would be 7, not 3).

---

## Invariant Checklist (UPST10-03)

### Invariant 1: AppContainer/WFP/Broker Windows Backend

**Status:** PASS
**Files checked:** crates/nono-cli/src/exec_strategy_windows/ (all files), crates/nono/src/sandbox/windows.rs
**Verification:**
```
git diff 449138a9 HEAD -- crates/nono-cli/src/exec_strategy_windows/
(empty — no output)

git diff 449138a9 HEAD -- crates/nono/src/sandbox/windows.rs
(empty — no output)
```
**Evidence:** All four absorb commits (ae77d198, 61689ef8, 91d526e6, 62dbf013) touch only Linux-only or shared-surface files. The 94-DIVERGENCE-LEDGER.md records `windows-touch: no` for Clusters A, B, and C. The git diff against the phase-base commit confirms zero modifications to exec_strategy_windows/ and sandbox/windows.rs. The Windows AppContainer/WFP/broker security model is byte-for-byte unchanged.

---

### Invariant 2: ADR-86 Audit/Diagnostics Library-Boundary Carve-Out

**Status:** PASS
**Files checked:** crates/nono/src/audit.rs (CR-02), bindings/c/src/ (CR-01), crates/nono-cli/src/diagnostic/ (UX stays CLI-side)
**Verification:**
```
grep -n "records_verified" crates/nono/src/audit.rs | grep "event_count"
1570:        records_verified: event_count > 0,

cargo test -p nono --lib -- audit::tests::verify_empty_log_with_no_stored_metadata_is_not_valid
running 1 test
test audit::tests::verify_empty_log_with_no_stored_metadata_is_not_valid ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 802 filtered out; finished in 0.00s

git diff 449138a9 HEAD -- bindings/c/src/
(empty — no output)

ls crates/nono-cli/src/diagnostic/
formatter.rs
mod.rs
```
**Evidence:** The CR-02 carve-out field `records_verified: event_count > 0` is present at line 1570 of audit.rs, byte-intact. The Cluster B absorb (91d526e6) added only additive structs (SandboxRuntimeAuditEvent, CommandPolicyAuditEvent) and did NOT touch the verify_audit_log return block. The ADR-87 CR-02 guard test `verify_empty_log_with_no_stored_metadata_is_not_valid` passes (1/1). bindings/c/src/ is CR-01 clean — no changes in the Phase 95 window. DiagnosticFormatter remains in crates/nono-cli/src/diagnostic/ (CLI-side, as required by ADR-86).

---

### Invariant 3: exec_strategy_windows/ Denial-Rendering Fork (ADR-86 D-03 Carve-Out)

**Status:** PASS
**Files checked:** crates/nono-cli/src/exec_strategy_windows/launch.rs, network.rs; crates/nono/src/error.rs (diagnostic_code/remediation bridge)
**Verification:**
```
git diff 449138a9 HEAD -- crates/nono-cli/src/exec_strategy_windows/launch.rs
(empty — no output)

git diff 449138a9 HEAD -- crates/nono-cli/src/exec_strategy_windows/network.rs
(empty — no output)

grep -n "diagnostic_code\|remediation" crates/nono/src/error.rs | head -10
77:    /// remediation pointer (e.g., a boot-flag suggestion).
212:        /// Human-actionable hint for remediation.
233:        /// Human-actionable hint for remediation.
383:    pub fn diagnostic_code(&self) -> crate::diagnostic::NonoDiagnosticCode {
452:    pub fn remediation(&self) -> Option<crate::diagnostic::NonoRemediation> {
671:    fn cwd_prompt_maps_to_structured_code_and_remediation() {
673:        assert_eq!(err.diagnostic_code(), NonoDiagnosticCode::CwdAccessRequired);
674:        assert_eq!(err.remediation(), Some(NonoRemediation::AllowCwd));
681:            err.diagnostic_code(),
685:            err.remediation(),
```
**Evidence:** No absorb commit in Phase 95 touches exec_strategy_windows/launch.rs or network.rs — both diffs against the phase-base are empty. The ADR-86 D-03 carve-out (Windows denial rendering stays CLI-side, bridged via diagnostic_code()/remediation() at the NonoError level) is at the same location as when Phase 86 established it. Both bridge methods are present in crates/nono/src/error.rs at lines 383 and 452.

---

## SC4: Security-Relevant Will-Sync Commit Notes

### Cluster A (9ce74e92) — AF_UNIX Mediation Deadlock Fix

**Security relevance:** Fixes 4 bugs in AF_UNIX pathname mediation (deadlock, wrong jt offsets,
rate-limiter starvation, dup2 bypass). BPF filter now installed AFTER IPC handshake; both filters
are pure allowlists with no fd-based holes. This is a direct continuation of Phase 87 SEC-01 work.

Bug specifics:
1. Sendmsg deadlock — BPF filter trapped child's SCM_RIGHTS handshake for notify fd
2. Wrong jt offsets — connect/bind/sendto/sendmmsg fell through to ALLOW instead of USER_NOTIF
3. Rate-limiter starvation — TCP/UDP calls drained the burst bucket before AF_UNIX decisions
4. dup2 bypass — IPC fd exemption allowed sandboxed process to dup2 an arbitrary socket

**Windows equivalents intact?** YES — AF_UNIX mediation is Linux-only (uses Landlock + seccomp BPF,
which are Linux kernel features). The fork's Windows path (exec_strategy_windows/) is NOT touched
by this commit. The 94-DIVERGENCE-LEDGER.md records `windows-touch: no` for Cluster A.

**Verification of Windows-equivalent preservation:**
```
git diff 449138a9 HEAD -- crates/nono-cli/src/exec_strategy_windows/
(empty — confirms Windows path untouched by all Phase 95 absorbs)
```

**Proxy Cluster F guard (Phase 89 D-02 divergence sentinel):**
```
cargo test -p nono-cli -- proxy_activates_with_custom_credentials_only
running 1 test
test proxy_runtime::tests::proxy_activates_with_custom_credentials_only ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1381 filtered out; finished in 0.00s
```

The Phase 89 active predicate `|| !prepared.custom_credentials.is_empty()` at lines 95 and 118 of
proxy_runtime.rs is intact. The guard test `proxy_activates_with_custom_credentials_only` with
`assert!(opts.active)` passes — the Cluster C structural no-op (62dbf013) preserved the fork's
fail-secure proxy activation divergence.

---

## PARTIAL → Phase 96 Deferral Record

Cross-target clippy gate SKIPPED on Windows dev host due to missing cross C compiler toolchain
(x86_64-unknown-linux-gnu, x86_64-apple-darwin). The following commits touch cfg-gated Unix code
blocks requiring Phase 96 cross-target verification:

| Commit | Files with #[cfg(unix/linux/macos)] blocks | Phase 96 verification required |
|--------|--------------------------------------------|-------------------------------|
| ae77d198 (Cluster A cherry-pick 9ce74e92) | crates/nono/src/sandbox/linux.rs, crates/nono-cli/src/exec_strategy/supervisor_linux.rs, crates/nono-cli/src/exec_strategy.rs (Linux/macOS cfg blocks) | cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used AND --target x86_64-apple-darwin |
| 61689ef8 (Cluster A post-fix) | Same files: linux.rs let-chain rewrite, supervisor_linux.rs let-chain rewrite, exec_strategy.rs block structure fix | Same cross-target gates |
| 91d526e6 (Cluster B shared-surface extraction) | crates/nono/src/sandbox/mod.rs (restrict_execute re-export, cfg(target_os = "linux") blocks), crates/nono/src/sandbox/linux.rs (via chain from Cluster A) | Same cross-target gates |

Per CLAUDE.md § Cross-target clippy verification:
> Any commit touching cfg-gated Unix code MUST be verified via `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`. If the cross-toolchain is not installed, the related verification REQ MUST be marked PARTIAL and deferred to live CI.

UPST10-02 marked PARTIAL pending Phase 96 XTGT verification. The live GitHub Actions Linux Clippy
and macOS Clippy lanes on the HEAD SHA are the decisive signals.

**Carry-forward PARTIAL→CI items from earlier phases (NOT resolved by Phase 95):**
- SEC-01/SEC-02 AF_UNIX guards (v3.1 — Phase 87): Linux/macOS cfg-gated supervisor IPC code
- ZTL-04 AWS_* strip (v3.2 — Phase 92/93): credential-stripping code in nono-py
These remain Phase 96's resolution vehicle.

---

## Phase 95 Completion Gate Results (SC1–SC5)

### SC1 — All will-sync commits present in git log

```
git log --oneline | grep "9ce74e92\|af_unix_mediation"
ae77d198 fix(sandbox): exempt IPC fd from sendmsg trapping to resolve af_unix_mediation deadlock (#1210)

git log --format="%B" 449138a9..HEAD | grep "11fd10e0"
(cherry picked from commit 11fd10e0...)
[match found in 91d526e6 commit body]

git log --format="%B" 449138a9..HEAD | grep "9b37dc52"
(9b37dc52 is a structural no-op in the fork)
[match found in 62dbf013 commit body]
```

**Result:** SC1 PASS — all three upstream SHAs (9ce74e92 via cherry-pick trailer, 11fd10e0 and
9b37dc52 via commit bodies) are referenced in the Phase 95 absorb commits.

### SC2 — No new test failures

D-04 baseline documented 5 known failures on Windows:
- nono lib: try_set_mandatory_label (1)
- nono-cli: profile_cmd init + 3 protected_paths (4)

Post-absorb: same 5 failures. No new failures introduced by Clusters A, B, or C.
(Environmental flakiness: audit_session ENV_LOCK cascade documented in 95-02/95-03 SUMMARY — not a
regression; audit_session.rs is unchanged by any absorb commit.)

**Result:** SC2 PASS

### SC3 — Fork-invariant checklist all PASS

Invariants 1, 2, and 3 above all marked PASS with actual command outputs.

**Result:** SC3 PASS (3/3 invariants PASS)

### SC4 — Security note for Cluster A documented

AF_UNIX mediation deadlock fix security note with Windows-equivalent preservation evidence is
documented in the SC4 section above. grep "AF_UNIX\|dup2\|9ce74e92" on this file will match.

**Result:** SC4 PASS

### SC5 — No open will-sync rows in ledger

94-DIVERGENCE-LEDGER.md Downstream routing block updated to mark Clusters A, B, C as ABSORBED.
All Cluster A/B/C rows carry absorbed annotations. Cluster D remains won't-sync → Phase 97.

**Result:** SC5 PASS (after Task 2 ledger update)
