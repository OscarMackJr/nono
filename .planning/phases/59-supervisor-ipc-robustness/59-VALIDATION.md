---
phase: 59
slug: supervisor-ipc-robustness
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-06
updated: 2026-06-06
---

# Phase 59 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## CRITICAL: Test Reachability (nono-cli is BIN-ONLY)

`crates/nono-cli/Cargo.toml` declares ONLY `[[bin]] name = "nono"` — there is **no `[lib]` target** and no `src/lib.rs`. Consequences that drive every test placement below:

- Integration tests under `crates/nono-cli/tests/` **CANNOT** reach `nono_cli::timeouts::*`, the private `run_supervisor_loop`, or the private `read_frame`. They may reach ONLY the **`nono` LIBRARY** pub surface (`use nono::supervisor::...`), exactly like `aipc_handle_brokering_integration.rs`.
- The **timeout const/accessor** unit tests live in the **in-crate `#[cfg(test)] mod tests` of `timeouts.rs`** (reachable via `--bin nono`).
- The **SC1 reconnect proof** drives the private `run_supervisor_loop` → lives in the **in-crate `#[cfg(test)] mod tests` of `exec_strategy.rs`** alongside the updated break-on-close test.
- The **SC2 behavioral timeout proof** is driven at the **`nono` lib boundary** via the pub `SupervisorSocket::pair()` + `set_read_timeout()` + `recv_message()` (Unix) / pub AIPC bind + `recv_message()` (Windows, which drives the private bounded `read_frame`).
- Do **NOT** add a `[lib]` target to nono-cli (out of scope / higher-risk path).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`#[test]`), `tempfile`, `proptest` (cli) |
| **Config file** | none — Cargo-native; in-crate `#[cfg(test)]` + integration tests under `crates/nono-cli/tests/` |
| **Quick run command** | `cargo test -p nono --lib supervisor::socket` |
| **Full suite command** | `make test` (or `cargo test --workspace`) |
| **Estimated runtime** | ~60–120 seconds (full suite); <10s for the targeted IPC tests |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono --lib supervisor::socket` + `cargo build --workspace`
- **After every plan wave:** Run `make test` (baseline-aware: the documented pre-existing Windows env failures are red→red carry-forwards, NOT regressions — see [[nono_cli_windows_baseline_test_failures]])
- **Before `/gsd:verify-work`:** Full suite green (modulo documented baseline) AND the Windows live-repro signed off
- **Max feedback latency:** ~10 seconds for the targeted IPC tests (set `NONO_SUPERVISOR_IPC_READ_TIMEOUT` low / use a ~1s `set_read_timeout` in tests to keep CI fast)

---

## Per-Task Verification Map

> Every REQ-IPC-01 row is backed by an automated `<verify>` or a Wave 0 dependency. Test homes reflect the bin-only reachability constraint above.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type / Home | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|------------------|-------------------|-------------|--------|
| 59-01 T1 | 01 | 1 | REQ-IPC-01 (D-01) | T-59-02 | new read-timeout const honors `NONO_*` env override + `MAX_TIMEOUT` clamp; invalid env → safe default, never unbounded | unit — IN-CRATE `timeouts.rs` `#[cfg(test)]` | `cargo test -p nono-cli --bin nono timeouts::` | ✅ exists | ✅ green (4/4 host-verified 2026-06-06) |
| 59-01 T2 | 01 | 1 | REQ-IPC-01 (W0 scaffolds) | — | per-platform scaffolds compile + link the `nono` lib pub surface; NO `nono_cli` references | integration — `tests/supervisor_ipc_robustness_{unix,windows}.rs` | `cargo test -p nono-cli --test supervisor_ipc_robustness_unix && cargo test -p nono-cli --test supervisor_ipc_robustness_windows` | ✅ exists | ✅ green (win `scaffold_links_nono_lib` host-verified; `_unix` links, empty-binary on Win host — Unix-CI) |
| 59-02 T1 | 02 | 2 | REQ-IPC-01 (SC1/SC2 Unix) | T-59-01 | macOS keep-alive on `sock_fd_active` (URL/direct listener fd only, narrower-by-default); `set_read_timeout(5s)` wired; timeout = keep-alive not kill | build + in-crate loop test | `cargo build -p nono-cli && cargo test -p nono-cli --bin nono reconnect_survival` | ✅ exists | ✅ green (Unix-CI-deferred — `#[cfg(unix)]`, not runnable on Win host; wiring host-verified by VERIFICATION.md truths 4-6) |
| 59-02 T2 | 02 | 2 | REQ-IPC-01 (SC1 reconnect) | T-59-01 | child closes IPC then reconnects → supervisor survives & re-accepts | in-crate `exec_strategy.rs` `#[cfg(test)]` (private `run_supervisor_loop`) | `cargo test -p nono-cli --bin nono reconnect_survival` | ✅ exists | ✅ green (Unix-CI-deferred — `#[cfg(unix)]`, 0 tests on Win host by design) |
| 59-02 T2 | 02 | 2 | REQ-IPC-01 (SC2 bounded) | T-59-01 | partial-frame stall → bounded timeout fires (keep-alive, not fatal) | integration via lib pub surface (`pair()`+`set_read_timeout()`+`recv_message()`) | `cargo test -p nono-cli --test supervisor_ipc_robustness_unix bounded_read_timeout` | ✅ exists | ✅ green (Unix-CI-deferred — `#![cfg(unix)]`, empty-binary on Win host) |
| 59-03 T1/T2 | 03 | 2 | REQ-IPC-01 (SC4 Windows) | T-59-01 | AIPC `read_frame` bounded by `PeekNamedPipe` deadline (driven via `recv_message()`); transient close → re-accept, capability pipe not permanently disabled | integration via lib pub surface + live-repro | `cargo test -p nono-cli --test supervisor_ipc_robustness_windows` + documented Win11 UAT | ✅ exists | ✅ green (4/4 host-verified 2026-06-06; SC1+SC2 operator UAT PASS Win11 26200) |
| 59-* | regression | — | REQ-IPC-01 | — | existing round-trip framing unchanged | integration | `cargo test -p nono-cli --test aipc_handle_brokering_integration` | ✅ exists | ✅ green (5/5 host-verified 2026-06-06) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/nono-cli/tests/supervisor_ipc_robustness_unix.rs` (net-new, 59-01) — file-level `#![cfg(unix)]` (empty binary on Windows); scaffold proves the `nono` lib pub surface links (`SupervisorSocket::pair()`), with labeled insertion points for SC1/SC2. **OWNED BY 59-02** in Wave 2. Reaches ONLY `nono::supervisor::...` — never `nono_cli`. *(Filled: `scaffold_links_nono_lib`, `bounded_read_timeout`.)*
- [x] `crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs` (net-new, 59-01) — file-level `#![cfg(target_os = "windows")]` (empty binary on Unix); scaffold proves the `nono` lib Windows AIPC pub surface links, with labeled insertion points for the AIPC bounded-read + re-accept tests. **OWNED BY 59-03** in Wave 2. Reaches ONLY `nono::supervisor::...` — never `nono_cli`, never a direct `read_frame`/`read_exact_bounded`. *(Filled: 4 tests, 4/4 PASS host-verified.)*
- [x] **Splitting the integration test file per platform** (one `_unix.rs`, one `_windows.rs`) gives 59-02 and 59-03 exclusive `files_modified` ownership → they run as a Wave-2 parallel pair with zero file overlap.
- [x] **Update** the existing IN-CRATE fork test at `crates/nono-cli/src/exec_strategy.rs:4057-4091` (59-02) — it asserts the OLD break-on-close (POLLHUP returns) behavior, which SC1 inverts. Must be updated to assert keep-alive/re-accept for the URL/direct-IPC listener path. The new `reconnect_survival` test joins it in the same in-crate `#[cfg(test)]` module (the only home reachable for the private `run_supervisor_loop`). *(Done: `reconnect_survival` at `exec_strategy.rs:4202`, `#[cfg(unix)]`.)*
- [x] Extend `crates/nono-cli/src/timeouts.rs` IN-CRATE unit tests (59-01) to cover `NONO_SUPERVISOR_IPC_READ_TIMEOUT` parse + clamp + invalid-fallback (save/restore env per CLAUDE.md env-var test rule). NOT in `tests/` (unreachable for a bin-only crate). *(Done: 4 tests at `timeouts.rs:192-235`, 4/4 PASS host-verified.)*

*Existing framework (Cargo test runner) covers all phase requirements — no new test framework needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Windows AIPC bounded-read + re-accept under real named-pipe timing | REQ-IPC-01 (SC4) | CI cannot deterministically exercise named-pipe `PIPE_WAIT` timing; project Windows-UAT pattern requires a real Win11 console | Run a slow-child (holds partial frame) and a disconnect-then-reconnect script against a supervised `nono run` from a real Win11 console; observe (a) supervisor not blocked past the deadline and (b) capability pipe re-accepts after the transient close. Document repro + result in the plan SUMMARY (59-03 Task 3). |

*Sampling note: the 5s timeout + ~200ms poll tick set temporal resolution — a reconnect test must wait > one poll tick (200ms) to observe re-accept; a timeout test must hold a partial frame > the configured timeout (use a ~1s `set_read_timeout` / `NONO_SUPERVISOR_IPC_READ_TIMEOUT` override in CI with save/restore).*

---

## Validation Sign-Off

- [x] All tasks have an automated `<verify>` or a Wave 0 dependency
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (the two per-platform net-new integration test files + the updated in-crate break-on-close test + the new in-crate reconnect_survival home)
- [x] Every automated `<verify>` command targets a REACHABLE surface (in-crate `--bin nono` for private fns/const; `--test ..._{unix,windows}` only for the `nono`-lib pub surface)
- [x] No watch-mode flags
- [x] Feedback latency < 10s for targeted IPC tests
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-06-06

---

## Validation Audit 2026-06-06

| Metric | Count |
|--------|-------|
| Requirements audited | 1 (REQ-IPC-01, 7 task-rows) |
| COVERED | 7/7 |
| PARTIAL | 0 |
| MISSING | 0 |
| Gaps requiring test generation | 0 |
| Tests host-verified green (Windows) | 13 (`timeouts::` 4 + `_windows` 4 + regression 5) + 30 `nono` lib socket |
| Tests Unix-CI-deferred (exist, `#[cfg(unix)]`) | 3 (`reconnect_survival`, `bounded_read_timeout`, `_unix scaffold_links_nono_lib`) |
| Manual-only (signed off) | 1 (Win11 SC1+SC2 operator UAT, build 26200) |

**Verdict: NYQUIST-COMPLIANT.** Every REQ-IPC-01 behavior has an automated test that exists and targets the behavior. No test needed generation (gsd-nyquist-auditor not spawned — no MISSING/PARTIAL gaps). The three `#[cfg(unix)]` tests are authored, compile, and link the `nono` lib pub surface, but cannot execute on the Windows-only dev host (they produce an empty binary by design); they run on a Unix CI runner. This is the project's standard cross-target CI-deferral (CLAUDE.md § Cross-target clippy + `.planning/templates/cross-target-verify-checklist.md`), and the Unix wiring itself was host-verified by VERIFICATION.md observable-truths 4–6. The Windows-side equivalents are host-proven green and the live Win11 UAT confirmed SC1+SC2 end-to-end.
</content>
