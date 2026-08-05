---
status: partial
phase: 112-security-residual-sync
source: [112-VERIFICATION.md]
started: 2026-08-05T00:00:00Z
updated: 2026-08-05T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Full 36-binary integration sweep completes

expected: `cargo test -p nono-sandbox-cli --tests` (all 36 integration binaries) runs to completion on a host or CI lane that does not stall. No new failures beyond the documented 11-name Windows-host baseline / 27-name workspace baseline.
result: [pending]

why_human: This phase's fix-pass reported the sweep stalling after ~25min with no output; it was killed and only ~11 relevant binaries were run instead. No artifact documents a completed full sweep. This is a coverage gap on ROADMAP SC4's "`make ci` GREEN" claim, not a demonstrated regression. Best resolved on the GH Actions Linux lane.

### 2. SEC-06 orphan-reap test runs for the right reason

expected: `af_unix_mediation_pathname_allows_orphaned_child_tcp_connect` in `crates/nono-cli/tests/socket_access_run.rs` actually exercises the double-forked-orphan + loopback-TCP-connect scenario and passes — not via the `python3_available()` skip path.
result: [pending]

why_human: The local `cross`-rs container has no python3, so this test short-circuits and reports `ok` trivially in ~0.01s. SEC-06's behavioural assertions (orphan TCP connect succeeds; no "Failed to read sockaddr" ancestry-loss fingerprint) were never runtime-exercised locally. Needs a Linux host/CI lane with python3.

### 3. RES-02 denial-marker tightening confirmed at runtime

expected: `af_unix_mediation_pathname_allows_connect_to_listed_socket`'s tightened bare-socket-path assertion actually distinguishes a denial from a non-denial at runtime.
result: [pending]

why_human: Same python3-unavailable container limitation. The test's compile/type correctness was confirmed; its runtime behaviour was not. Self-flagged in `112-04-SUMMARY.md`.

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps

## Related Open Items (not UAT — tracked separately)

`112-REVIEW.md` carries **12 Warning and 5 Info findings that remain OPEN by deliberate operator scoping** (the fix pass was scoped to the 4 Criticals only). These are not verification gaps and do not block phase closure, but they are real and unfixed. Highest-value among them:

- **WR-06** — `--allow-gpu` now makes seccomp-notify mandatory and fatal (`_exit(126)`) on non-NVIDIA hosts, in containers without `CAP_SYS_ADMIN`, and under WSL2; `nono wrap --allow-gpu` becomes a hard error.
- **WR-01** — `waitpid(-1)` can steal the exit status of unrelated `std::process::Child` handles.
- **WR-02 / WR-03** — `discard_late_terminal_input` always consumes and drops the first byte of genuine type-ahead (the phase's own test asserts the loss), and does not retry `EINTR` on `poll()` — at teardown, when `SIGCHLD` is near-certain.
- **WR-05** — non-loopback `--listen` exposes the session token and injected credentials over plaintext HTTP.
- **WR-11** (partially overtaken by the CR-02 fix) — the residual half is a misleading "failed to parse {path}" message for well-formed foreign JSON.
