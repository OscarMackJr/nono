---
phase: 112-security-residual-sync
plan: 04
subsystem: security
tags: [pty, terminal, cursor-position-report, unix-socket, seatbelt, test-infra, upstream-sync]

# Dependency graph
requires:
  - phase: 112-01
    provides: The Wave 1 reality-check disposition table (112-DISPOSITION-TABLE.md) that carried RES-02's three commit dispositions from research (outside 112-01's own gate scope, re-confirmed live by this plan)
provides:
  - "discard_late_terminal_input()/CprReplyParse teardown drain in pty_proxy.rs: a late cursor-position-report reply (ESC[<row>;<col>R) an exiting TUI child's ESC[6n query leaves in the input queue is consumed on final teardown, before it can be pasted into the next shell prompt"
  - "RES-02's socket.rs /tmp-vs-target/ commit (4cc0af2c52) explicitly disposition-amended to SKIP with corrected, symbol-level-verified reasoning: the target socket_test_dir() tempdir-choice pattern does not exist anywhere in this fork"
  - "RES-02's socket_access_run.rs denial-marker assertion tightened (adapted, not ported verbatim, from 9840a16f35) plus NONO_NO_SAVE_PROMPT=1 test-runner env guard"
affects: [112-08-ledger-addendum]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Byte-grammar teardown drain: a small Clone+Copy enum state machine (CprReplyParse) driving a bounded poll/EINTR-retry byte-at-a-time read loop, stopping at the first byte that breaks the grammar so real type-ahead is never scanned away"
    - "Symbol-level disposition re-verification before executing a plan's literal action text: file-presence checks (git show / ls) are necessary but not sufficient — grep the fork for the actual function/comment the plan's action instructs editing before assuming it exists"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/pty_proxy.rs
    - crates/nono-cli/src/timeouts.rs
    - crates/nono/src/supervisor/socket.rs
    - crates/nono-cli/tests/socket_access_run.rs

key-decisions:
  - "503045801a (adopt): ported verbatim from upstream — CprReplyParse + discard_late_terminal_input, called only on the final release_terminal_for_prompt teardown path, never on pause_terminal_for_prompt (suspend/resume) or the Drop-impl path; restore_terminal() body byte-for-byte unchanged"
  - "4cc0af2c52 (disposition amendment: SKIP, corrected reasoning): 112-DISPOSITION-TABLE.md/112-RESEARCH.md assumed at file-presence level that the fork's socket.rs still carries the target/-vs-Seatbelt socket_test_dir() tempdir-choice comment to annotate. Live symbol-level grep found socket_test_dir() absent from socket.rs AND from open_url_runtime.rs (the commit's other target file, whose test module the fork never carried); the commit's third file (tests/url_open_integration.rs) doesn't exist in the fork at all. The fork's socket.rs tests use SupervisorSocket::pair() (anonymous socketpair(), no filesystem path) — there is no SUN_LEN-limited tempdir choice left to switch. Recorded via a comment-only addition (git diff --stat confirms zero functional change)."
  - "9840a16f35 (adopt, adapted not verbatim): NONO_NO_SAVE_PROMPT=1 ported as-is into run_nono()'s env. The denial-marker assertion tightening was adapted rather than copied verbatim — upstream's `\"send {socket_arg}\"` marker matches upstream's own single-line \"<op> <path>\" denial log format, which this fork's diagnostic formatter (crates/nono-cli/src/diagnostic/formatter.rs) does not produce (it renders a block header + one denied-path-per-line instead). The fork-appropriate marker is the socket path itself."

patterns-established: []

requirements-completed: [RES-02]

# Metrics
duration: ~110min
completed: 2026-08-05
---

# Phase 112 Plan 04: RES-02 — PTY Teardown Drain, Socket Tempdir Skip, Test Tightening Summary

**Adopted upstream's late-CPR-reply PTY teardown drain verbatim, corrected and re-recorded RES-02's socket.rs `/tmp` skip disposition after symbol-level re-verification proved its target function doesn't exist in this fork, and adapted (not ported verbatim) a denial-marker test assertion tightening to match this fork's own diagnostic message format.**

## Performance

- **Duration:** ~110 min (dominated by four Docker-containerized cross-target clippy/test runs, ~50 min of that serialized due to shared target-dir lock contention between two concurrent `cross` invocations)
- **Started:** 2026-08-05 (session start, not separately timestamped)
- **Completed:** 2026-08-05
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Verified, then committed, a prior interrupted executor's uncommitted Task 1 work (`CprReplyParse`/`discard_late_terminal_input` + `TERMINAL_QUERY_REPLY_TIMEOUT`) after independently re-deriving the disposition against upstream `503045801a` and confirming placement, `restore_terminal()` non-mutation, and grammar correctness byte-for-byte against the live upstream diff.
- Live-verified all 38 `pty_proxy` unit tests (including the 5 new ones) actually pass on Linux via `cross test --target x86_64-unknown-linux-gnu` — not just "compiles."
- Caught a disposition-table error at symbol level: `4cc0af2c52`'s target (`socket_test_dir()`) does not exist anywhere in the fork, in either of the two files the disposition table believed it modified. Corrected the record rather than mechanically executing a factually-false plan instruction (annotating a nonexistent comment).
- Adapted (not blindly ported) `9840a16f35`'s denial-marker tightening, because a literal port would have silently defeated the assertion (upstream's marker text never appears in this fork's diagnostic output format).

## Task Commits

Each task was committed atomically:

1. **Task 1: RES-02 adopt — late CPR-reply teardown drain** - `f9199b37` (feat)
2. **Task 2: RES-02 skip decision (socket.rs) + RES-02 adopt (test tightening)** - `f45ec8fc` (docs)

_No TDD-cycle multi-commit tasks — Task 1's tests were authored by the interrupted prior executor and are ported byte-for-byte from upstream's own RED-then-GREEN-equivalent commit history (upstream's commit already contains both the tests and the implementation together); this session's job was verification, not authorship, so a single commit was correct._

## Files Created/Modified
- `crates/nono-cli/src/pty_proxy.rs` - `CprReplyParse` enum/`step()`, `discard_late_terminal_input()`, call site in `release_terminal_for_prompt` before `self.restore_terminal()`, 5 new unit tests
- `crates/nono-cli/src/timeouts.rs` - `TERMINAL_QUERY_REPLY_TIMEOUT: Duration = 30ms`, `#[cfg(unix)]`-gated matching the file's existing `pty_proxy`-section convention
- `crates/nono/src/supervisor/socket.rs` - comment-only disposition-amendment note above `mod tests`, zero functional change
- `crates/nono-cli/tests/socket_access_run.rs` - `NONO_NO_SAVE_PROMPT=1` env guard in `run_nono()`; denial assertion in `af_unix_mediation_pathname_allows_connect_to_listed_socket` scoped to the specific socket path

## Decisions Made
- **Verify-then-commit, not re-implement, for the resumed Task 1 work.** Independently re-derived the disposition against `git show 503045801a`, confirmed the call site sits in `release_terminal_for_prompt` (the "final supervisor-owned prompt" path, matching upstream's own function name and placement exactly — not the suspend/resume `pause_terminal_for_prompt` path, and not the `Drop`-impl path), and confirmed `restore_terminal()`'s own 722-731 body is untouched. Judged the prior work correct and salvageable; committed as-is after live verification rather than discarding it.
- **Disposition amendment for `4cc0af2c52`, not a mechanical skip-comment insertion.** The plan's Task 2 action instructed adding a comment "immediately above the existing tempdir-choice comment" in `socket.rs` — that comment does not exist in this fork (`grep -rn socket_test_dir crates/` returns zero hits anywhere in the tree). Rather than silently working around the false premise or inventing a comment that never existed, recorded the corrected finding as a standalone comment block explaining exactly what's absent and why the disposition is still SKIP (there's no code path left to apply the upstream switch to), citing this SUMMARY per the disposition_verification_warning's symbol-level mandate.
- **Adapted `9840a16f35`'s assertion marker instead of porting verbatim.** Traced the fork's actual IPC-denial rendering (`crates/nono-cli/src/diagnostic/formatter.rs:1596-1616`) and confirmed it never emits `"send {path}"` — it emits a block header line followed by one denied-path line per entry. Using upstream's literal marker text would have made the tightened assertion vacuously true (never triggered), silently defeating the point of the tightening. Used the bare socket path as the fork-appropriate marker instead.
- **Ran both mandatory cross-target clippy gates (`--all-targets`) twice** — once for Task 1's state alone (both GREEN), once again for the combined Task 1+2 state after Task 2's edits landed (both GREEN again) — because this plan's file set (`pty_proxy.rs`, `socket.rs`) is explicitly called out by name in the mandatory-gate instructions, and `--all-targets` was required per this plan's context since most of Task 1's diff is test code that canonical (non-`--all-targets`) clippy would not exercise (112-02's precedent).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1/Disposition correction] `4cc0af2c52`'s skip-reasoning target comment does not exist in the fork**
- **Found during:** Task 2 read_first
- **Issue:** The plan's action text instructed annotating "the existing tempdir-choice comment" in `crates/nono/src/supervisor/socket.rs`. Live `grep -rn "socket_test_dir\|tempdir_in\|target/\|SUN_LEN\|Seatbelt" crates/nono/src/supervisor/socket.rs` returned zero hits; the function upstream's commit modifies is entirely absent. Further grep confirmed it's also absent from `crates/nono-cli/src/open_url_runtime.rs` (present as a file, but with no test module at all) and that `crates/nono-cli/tests/url_open_integration.rs` (the commit's third file) doesn't exist in the fork.
- **Fix:** Recorded a corrected disposition-amendment comment in `socket.rs` explaining the target is absent and why SKIP is still the right call (no code path exists to apply the switch to), rather than annotating a nonexistent original comment. Did not touch `open_url_runtime.rs` (nothing there to skip-annotate either — no test module, no tempdir logic).
- **Files modified:** `crates/nono/src/supervisor/socket.rs`
- **Verification:** `git diff --stat -- crates/nono/src/supervisor/socket.rs` shows a pure comment addition (17 insertions, 0 deletions); both cross-target clippy gates GREEN on the resulting file.
- **Committed in:** `f45ec8fc` (Task 2 commit)

**2. [Rule 1/Fork-appropriate adaptation] `9840a16f35`'s literal denial marker text does not match the fork's diagnostic format**
- **Found during:** Task 2 read_first
- **Issue:** Upstream's tightened assertion checks `stderr.contains(&format!("send {socket_arg}"))`, matching upstream's own single-line `"<op> <path>"` denial log format. This fork's `diagnostic/formatter.rs` renders a two-part block (`"[nono] IPC denial: N pathname Unix socket(s) blocked."` header + one `"[nono]   {path}"` line per entry) — it never emits the string `"send <path>"`. Porting the marker verbatim would make the assertion pass unconditionally (stderr would never contain that substring), silently defeating the tightening's purpose.
- **Fix:** Used the bare `socket_arg` (the socket path under test) as the fork-appropriate marker, matching how the fork's formatter actually renders a denied path.
- **Files modified:** `crates/nono-cli/tests/socket_access_run.rs`
- **Verification:** `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox-cli --test socket_access_run -- af_unix_mediation_pathname_allows_connect_to_listed_socket` → 1 passed (see Known Limitation below regarding what this run actually exercised).
- **Committed in:** `f45ec8fc` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1-class disposition/adaptation corrections; neither is scope creep — both are corrections to factually incorrect premises in the plan's action text, discovered via the mandated symbol-level re-verification)
**Impact on plan:** Both corrections keep RES-02's disposition record accurate and the ported test assertion actually load-bearing. No architectural changes, no Rule 4 escalations.

## Issues Encountered

**Known limitation — `af_unix_mediation_pathname_allows_connect_to_listed_socket`'s runtime path not end-to-end exercised locally.** The `cross test` run for this integration test reports `1 passed` in `0.01s` — too fast to have actually spawned `nono run` + `python3` + a socket connect. Probed the `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5` base image directly (`docker run --rm <image> sh -c 'which python3'`) and confirmed `python3` is absent from that minimal image. The test's pre-existing `python3_available()` guard (unchanged by this plan) causes it to `eprintln!` and return early in this environment — the tightened assertion itself was therefore not runtime-exercised against a real denial/non-denial scenario in this local session. This is a pre-existing test-environment limitation (present before this plan's changes too, since the guard is untouched), not a regression introduced here. `cargo build`/`cargo test` compilation of the new assertion logic (type-checks, borrow-checks, string comparison against `&String`) was confirmed via both cross-target clippy `--all-targets` gates and the successful `test` profile build. The live GH Actions CI lane (which does have `python3`) is the decisive signal for this specific assertion's runtime behavior, per this project's established Docker/cross-limitation-fallback posture.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All three RES-02 commits are now individually dispositioned with recorded reasoning (ROADMAP SC2 satisfied for RES-02): `503045801a` adopted verbatim and live-tested; `4cc0af2c52` skipped with corrected, symbol-level-verified reasoning; `9840a16f35` adopted with a necessary fork-specific adaptation.
- `112-DISPOSITION-TABLE.md`'s `4cc0af2c52` row (MEDIUM confidence, "adapt with caution, or skip") is now superseded by this plan's live finding — the target is absent, not merely risky to adapt. Flagging for whoever writes the `112-08` D-05 ledger addendum: cite this SUMMARY's corrected reasoning, not the disposition table's original file-presence-only note.
- No blockers for remaining Phase 112 waves (Wave 3: `112-05`/`112-06`/`112-07`; Wave 4: `112-08`).

## Verification

- `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox-cli --bin nono -- pty_proxy::` → 38 passed, 0 failed (includes all 5 new `discard_late_terminal_input`/`CprReplyParse` tests)
- `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox-cli --test socket_access_run -- af_unix_mediation_pathname_allows_connect_to_listed_socket` → 1 passed (see Known Limitation above — exercised via the pre-existing python3-unavailable skip path in this minimal container, not end-to-end)
- `cross clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` → clean, run twice (Task 1 state: exit 0, 14m08s; combined Task 1+2 state: exit 0, 8m41s)
- `cargo-zigbuild clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (SDKROOT unset) → clean, run twice (Task 1 state: exit 0, 42s; combined state: exit 0, 18s)
- `cargo fmt --all --check` → clean, run after each task's edits
- `git diff --diff-filter=D --name-only HEAD~1 HEAD` → empty after each commit (no unexpected deletions)

## Self-Check: PASSED

Verified files exist:
- FOUND: `crates/nono-cli/src/pty_proxy.rs` (contains `fn discard_late_terminal_input` at line 1565)
- FOUND: `crates/nono-cli/src/timeouts.rs` (contains `TERMINAL_QUERY_REPLY_TIMEOUT` at line 32)
- FOUND: `crates/nono/src/supervisor/socket.rs` (comment addition above `mod tests`)
- FOUND: `crates/nono-cli/tests/socket_access_run.rs` (contains `NONO_NO_SAVE_PROMPT` at line 47)

Verified commits exist:
- FOUND: `f9199b37` (Task 1)
- FOUND: `f45ec8fc` (Task 2)

---
*Phase: 112-security-residual-sync*
*Completed: 2026-08-05*
