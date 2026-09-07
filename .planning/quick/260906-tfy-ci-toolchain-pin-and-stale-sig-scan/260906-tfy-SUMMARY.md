---
quick_id: 260906-tfy
slug: ci-toolchain-pin-and-stale-sig-scan
date: 2026-09-07
status: complete
commits:
  - fd145089  # fix(ci): pin the 14th toolchain site
  - 65da860b  # fix(test): restore the CR-01-RESIDUAL scan
milestone: v3.7
---

# Quick Task 260906-tfy — Summary

Two independent CI defects from the triage of `workflow_dispatch` run
`34068950597`, fixed and committed separately.

## Task 1 — `ci.yml` 14th toolchain site (`fd145089`)

**Status:** done.

`c18e93db` pinned 13 `toolchain: stable` inputs. The file has **14**
`dtolnay/rust-toolchain` steps; the `cross-compile` job's step carried only
`targets:` and no `toolchain:` key at all, so the string search that found the
other 13 could not see it. The action defaults that input to `stable`, so rustup
added `x86_64-apple-darwin` to **stable** while cargo, reading the new
`rust-toolchain.toml`, built with **1.98.1** — a toolchain with no such target:

```
error[E0463]: can't find crate for `core`
```

`aarch64-apple-darwin` survived because on `macos-14` it is the native host
target, present in every toolchain.

Added `toolchain: "1.98.1"` plus a comment naming the grep-invisibility, so the
next toolchain bump does not re-miss the site.

**Verified:** 14 of 14 steps now carry an explicit `toolchain:` input, 0
implicit; `ci.yml` re-parsed as valid YAML with the step resolving to
`{'toolchain': '1.98.1', 'targets': '${{ matrix.target }}'}`.

## Task 2 — restored the CR-01-RESIDUAL scan (`65da860b`)

**Status:** done.

`resl_nix_async_signal_safety` locates `clear_close_on_exec`'s body by searching
`exec_strategy.rs` for its exact signature, then asserts zero `format!(` in that
body. The locator held the `std::io::Result<()>` form from `0734fa7f`; upstream
absorb **`ae77d198` (#1210, 2026-06-23)** reverted the signature to the crate's
`Result<()>` alias without updating it. `slice_function_body` panics when the
prefix is absent, so the test failed on every platform from that date and the
`format!(` assertion behind it **did not execute for ~2.5 months**.

**The check that changed the fix.** The assertion message stated the signature
"must remain" `std::io::Result<()>`. Read literally that makes `ae77d198` a
re-opened CR-01 allocator-deadlock defect whose correct remedy is reverting the
signature, not editing the test. Verified against the code instead:
`NonoError::Io(std::io::Error)` (`error.rs:344`) wraps the `io::Error` by value,
enum construction does not allocate, `last_os_error()` is a stack-resident
`Repr::Os(i32)`, both error paths use that form, and the call site
(`exec_strategy.rs:1276`) still discards via `if let Err(_e) = ...` behind a
`const MSG_SOCK: &[u8]`. The guard was stale; the code was sound.

Updated three sites (doc example, live locator, assertion message). The message
now defends the **property** — the error must be constructible without heap
allocation — rather than one spelling of the return type, and says explicitly not
to "fix" a future failure by reverting the type. `exec_strategy.rs` is unchanged.

**Verified:**
- `cargo test -p nono-sandbox-cli --test resl_nix_async_signal_safety` → exit 0,
  **5 passed / 0 failed** (was 4/1). Non-zero count asserted per the standing
  test-selector rule.
- **Perturbation proof:** inserting a `format!()` into `clear_close_on_exec`
  failed the **count** assertion at line 209 (`found 1`, `left: 1, right: 0`),
  *not* the locator panic at line 100 — proving the scan now reaches the body
  rather than merely no longer panicking. Reverted; re-confirmed green;
  `git diff` on `exec_strategy.rs` empty.
- `cargo clippy -p nono-sandbox-cli --test resl_nix_async_signal_safety -D
  warnings -D clippy::unwrap_used` → exit 0.
- `cargo fmt --all -- --check` → clean.

No cross-target gate required: the test is deliberately not `cfg`-gated (its
module doc says the source-text check works on any platform), and only string
literals in it changed.

## Findings worth carrying past this task

1. **A grep that finds N sites is not evidence there are only N sites.**
   `c18e93db`'s method — search the literal `toolchain: stable` — is structurally
   blind to a step that omits the key entirely, which is precisely the shape that
   broke. The general form: when pinning or auditing a config key, enumerate the
   *steps* and check each for presence, rather than enumerating occurrences of the
   value you expect to replace. This is the same class as the phase's own
   "audit placement AND predicate width AND class coverage" lesson.

2. **The Windows-host test baseline has a target-shaped blind spot.** The
   documented 12-name baseline is measured with `-p nono-sandbox-cli --bin nono`.
   `resl_nix_async_signal_safety` is a separate integration target, so a test red
   on *every* platform since June never appeared in it, and `cargo test
   --workspace` is fail-fast across targets so it was masked there too. Any claim
   of "known baseline" should name the target scope it was measured at.

3. **A test that prescribes a remedy can outlive the remedy's correctness.**
   The assertion message would have led a reader to revert a sound upstream
   change. Guards should pin the invariant, not the implementation that happened
   to satisfy it.

## Not fixed here (pre-existing, larger than a quick task)

Both failed identically on all three prior dispatch runs:

- **`Cross-compile check x86_64-unknown-linux-gnu`** — `libdbus-sys` needs
  `libdbus-1-dev`; the job installs only `pkg-config`. Not a one-liner: the same
  job ends with a step that fails the build *if the binary links libdbus*, so
  installing the dev package would likely trip that guard. The real question is
  whether `keyring`'s secret-service backend belongs in the graph at all.
- **`Test (ubuntu-latest)`** — 3 failures, all `waitpid` ECHILD
  (`dynamic_tokens::…linked_worktree`,
  `exec_strategy::…proxy_only_v4_no_deadlock`,
  `exec_strategy::…reap_reparented_orphans_…`), consistent with Phase 112
  SEC-06's `PR_SET_CHILD_SUBREAPER` reaping meeting the CI runner's process
  environment. Needs `/gsd:debug`.
