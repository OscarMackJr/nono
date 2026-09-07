---
quick_id: 260906-tfy
slug: ci-toolchain-pin-and-stale-sig-scan
date: 2026-09-07
description: Close the 14th toolchain-pin site in ci.yml and restore the stale CR-01-RESIDUAL signature scan
milestone: v3.7
phase_context: Phase 118 close-out triage of CI run 34068950597
---

# Quick Task 260906-tfy

Fix two independent CI defects surfaced by triaging `workflow_dispatch` run
`34068950597`. They share no code and are committed atomically and separately.

## Task 1 — `ci.yml` cross-compile job: the missed 14th toolchain site

**Defect class:** regression, introduced by `c18e93db`.

`c18e93db` pinned the build toolchain by adding `rust-toolchain.toml` (channel
`1.98.1`) and pinning every `toolchain: stable` input in `.github/workflows/ci.yml`.
It found and pinned 13 sites. There are **14** `dtolnay/rust-toolchain` steps in
that file. The 14th — the `cross-compile` job at `.github/workflows/ci.yml:677-680` —
carries no `toolchain:` input at all, only `targets:`. It was therefore invisible to
a search for the literal string `toolchain: stable`, which is exactly how the other
13 were located.

The action defaults that input to `stable`. The resulting split-brain:

- `rustup` installs **stable** and adds `x86_64-apple-darwin` to **stable**.
- `cargo` then reads the new `rust-toolchain.toml` and builds with **1.98.1**,
  which has no such target installed.

```
error[E0463]: can't find crate for `core`
  = note: the `x86_64-apple-darwin` target may not be installed
```

**Why only that one target broke.** `aarch64-apple-darwin` runs on `macos-14`,
where it is the *native host* target — present in every toolchain, so nothing had
to be added and the mismatch could not bite. That asymmetry is the diagnosis
confirming itself, and it is the reason the failure looks arbitrary from the
outside.

**Evidence it is a regression, not pre-existing:** `Cross-compile check
x86_64-apple-darwin` was `success` on the three prior `workflow_dispatch` runs
(`34065278991` / `b2836044`, `34062489616` / `3c02427a`, `31916670497` / `43442677`
— all pre-pin) and `failure` on `34068950597`, the first post-pin run.

**Fix:** add `toolchain: "1.98.1"` to that step, carrying the same lockstep
comment the other 13 sites use.

**Verify:**
1. `awk` audit over `ci.yml` reports **14 of 14** `dtolnay/rust-toolchain` steps
   carrying an explicit `toolchain:` input — zero implicit-`stable` sites remain.
2. `ci.yml` still parses as valid YAML.

**Out of scope, recorded not fixed:** the 7 `toolchain: stable` inputs in four
one-off phase workflows (`phase-37/45/46`, `spire`) that `c18e93db` deliberately
left unpinned. That decision stands; this task does not revisit it.

## Task 2 — restore the stale CR-01-RESIDUAL signature scan

**Defect class:** pre-existing stale guard, dark since 2026-06-23.

`crates/nono-cli/tests/resl_nix_async_signal_safety.rs` locates the body of
`clear_close_on_exec` by string-searching `exec_strategy.rs` for its exact
signature, then asserts the body contains zero `format!(` invocations — the
CR-01-RESIDUAL check that a helper reachable from the post-fork child arm cannot
heap-allocate and re-open the allocator-mutex deadlock.

The locator string is `fn clear_close_on_exec(fd: i32) -> std::io::Result<()>`.
That form was introduced by `0734fa7f` and **reverted by `ae77d198`**
(2026-06-23, upstream absorb #1210) to the crate alias:

```rust
fn clear_close_on_exec(fd: i32) -> Result<()>          // exec_strategy.rs:4129
```

`slice_function_body` panics when the prefix is not found, so the test has failed
loudly on every platform since that date — and the substantive `format!(`
assertion behind it has not executed in ~2.5 months.

**The assumption that had to be checked first.** The test's own failure message
states the signature *must remain* `std::io::Result<()>`, "so the call site
discards the io::Error via `if let Err(_e) = ...`". Read literally, that makes
`ae77d198` a re-opened CR-01 defect rather than a stale string, and the correct
remedy would be reverting the signature — not editing the test. Verified against
the code instead of the prescription:

- `NonoError::Io(std::io::Error)` (`crates/nono/src/error.rs:344`) wraps the
  `io::Error` **by value**; enum construction does not heap-allocate.
- `std::io::Error::last_os_error()` produces a stack-resident `Repr::Os(i32)`.
- Both error paths in the body (`exec_strategy.rs:4134`, `:4142`) use exactly
  that form; the body contains zero `format!(`.
- The call site (`exec_strategy.rs:1276`) still discards via
  `if let Err(_e) = clear_close_on_exec(fd)` and reports through a
  `const MSG_SOCK: &[u8]` static byte string.

So `ae77d198` is async-signal-safe-equivalent. The guard is stale; the code is
sound. The prescription in the assertion message is what is out of date.

**Fix:** update the three places that name the old signature —
- line ~95: the `slice_function_body` doc comment example
- line ~194: the live locator string (the one that panics)
- line ~217: the assertion message's prescription, reworded to state the real
  contract (the error must be constructed without allocating) and to name the
  current non-allocating form, so the next reader is not told to revert a
  correct change.

**Verify (both required — the second is what makes the first meaningful):**
1. `cargo test -p nono-sandbox-cli --test resl_nix_async_signal_safety` exits 0
   **and reports 5 passed / 0 failed** — a non-zero count, per the standing
   test-selector rule.
2. **Perturbation proof.** Insert a `format!()` into the body of
   `clear_close_on_exec`, re-run, observe the CR-01-RESIDUAL assertion FAIL on
   the count (not on the locator panic), then revert and re-confirm green.
   Without this the fix could merely have silenced the panic while leaving the
   assertion vacuous — the exact failure mode recorded in this phase's
   anti-pattern table.

**Platform note:** this test is deliberately *not* `cfg`-gated. Its module doc
says it lives in the workspace tests because `exec_strategy.rs` is `#[cfg(unix)]`
but "the source-text check works on any platform — we just read the file as
text." Confirmed by reproducing the failure live on this Windows host. No
cross-target gate is required for this task.

## Constraints

- Neither task touches `#[cfg]`-gated Unix code or `exec_strategy/`, so the
  mandatory cross-target clippy gates are not triggered. Task 2 edits a test's
  string literals only; Task 1 edits YAML only.
- `make` is not installed on this host — use direct `cargo` invocations.
- Hand-edit `.planning/STATE.md`. Do **not** run any `gsd-sdk` state writer
  (D-24 / project memory: they have deleted the `parallel_milestone*` block on
  this dual-milestone file).
- Every commit carries `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.

## Deliberately not fixed here

The other two red jobs from run `34068950597` are pre-existing, failed
identically on all three prior dispatch runs, and are each larger than a quick
task:

- `Cross-compile check x86_64-unknown-linux-gnu` — `libdbus-sys` needs
  `libdbus-1-dev`, which the job does not install. Not a one-liner: the same job
  ends with a step that *fails the build if the binary links libdbus*, so
  installing the dev package would likely trip that guard. The real question is
  whether `keyring`'s secret-service backend belongs in the graph at all.
- `Test (ubuntu-latest)` — 3 failures, all `waitpid` ECHILD, consistent with the
  Phase 112 SEC-06 `PR_SET_CHILD_SUBREAPER` reaping work meeting the CI runner's
  process environment. Needs `/gsd:debug`.
