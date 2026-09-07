---
quick_id: 260906-q7n
slug: receipt-sink-known-folder-hardening
date: 2026-09-07
status: complete
commits:
  - 4a1aee20  # fix(receipt-sink): resolve ProgramData from the OS
  - 3793655a  # docs(cross-target): correct the runbook claim
milestone: v3.7
security_relevant: true
---

# Quick Task 260906-q7n — Summary

**Status:** complete. Item 2 of 7 from the CI run `34068950597` triage.

## The defect

`validate_and_join_sink_base` returned `Ok` for a `tempdir()` standing in for a
hostile `%PROGRAMDATA%` redirect. The panic text carries the **returned path**,
so this was a confirmed fail-open, not a flaky assert.

Its only test was `path_is_owned_by_current_user`, whose review-sanctioned
rationale assumed **a redirection target will be current-user-owned**. False
under elevation: the helper compares against the token USER SID, and an elevated
process creates objects owned by `BUILTIN\Administrators`. The predicate covered
only the non-elevated subset of the class it claimed to cover.

Same mechanism as `260815-gfd` cluster A, at a new call site — there it failed
*safe*, here it failed *open*.

## Fix

`SHGetKnownFolderPath(FOLDERID_ProgramData)` — the step the original review had
named as "a future hardening step" — is now the primary check, with ownership
retained beneath it as defense in depth. `%PROGRAMDATA%` is attacker-controlled
input; the known-folder table is registry-backed and not user-writable.

`path_is_owned_by_current_user` also gained a scope-limit note **at its
definition**, so the next caller is warned there instead of in CI.

## Verification

- 16/16 `receipt_sink::` tests (non-zero count asserted).
- `accepts_the_os_resolved_root` added — without a positive case, a guard broken
  *shut* would satisfy every other test in the module.
- `rejects_a_system_owned_non_root` added, using `C:\Windows`: not
  current-user-owned, so the **old** predicate would have accepted it. This
  reproduces the CI fail-open on a non-elevated host, with a precondition assert
  so it cannot go vacuous.
- **Perturbation:** forcing `same_existing_dir` to `Ok(true)` fails that test.
  The instructive part: `rejects_a_redirection` **still passes** under the same
  perturbation, because on a non-elevated host the tempdir *is* current-user-owned
  and the secondary check catches it. That is exactly why Phase 118 never caught
  this locally — the bug was only reachable on an elevated runner.
- fmt clean; clippy `-D warnings -D clippy::unwrap_used` exit 0 on both crates.
- **Both mandatory cross-target gates green at `--workspace`:** cargo-zigbuild
  apple-darwin 4m37s exit 0; cross linux-gnu 18m16s exit 0.

## The second defect this uncovered

Running the mandatory gates revealed the toolchain pin (`c18e93db`) had **also
broken both local gates** — the ones CLAUDE.md marks mandatory — in two symptoms
that look unrelated:

| Gate | Symptom |
|---|---|
| apple-darwin | `error[E0463]: can't find crate for 'core'` |
| linux-gnu | `cross` tries to install `1.98.1-x86_64-unknown-linux-gnu` **on the Windows host** |

Root cause is identical to the ci.yml bug fixed in `fd145089`: **installed
targets belong to a toolchain, not a host**, and a pin invalidates installs made
against a different channel. `x86_64-apple-darwin` had been added under `stable`.

`cross-target-verify-checklist.md` had asserted "Both rustup std targets are
already added on this host" — reassuring, and false. Corrected in `3793655a`
with both fix commands, a verify step that checks the target is *listed* rather
than that the command exited 0, and a note that neither symptom qualifies for
PARTIAL.

## Findings worth carrying

1. **A reviewed rationale can carry an unstated premise that only one
   environment falsifies.** The prose was persuasive and the mechanism was
   wrong; the premise "a redirect will be user-owned" was never written down as
   an assumption, so it was never audited. This is the third recurrence of the
   predicate-width lesson.
2. **A guard's remedy text can outlive its own correctness** — the message
   prescribed `std::io::Result<()>`-style specifics elsewhere in this codebase
   too (`260906-tfy`). Guards should pin the invariant, not one implementation.
3. **One toolchain pin broke three verification surfaces** (CI cross-compile,
   local darwin gate, local linux gate) with three different-looking errors.
   When a pin lands, re-run every gate rather than assuming host setup persists.
