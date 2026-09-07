---
quick_id: 260907-bwq
slug: ci-tests-lagging-deliberate-changes
date: 2026-09-07
status: complete
commits:
  - 38c03b86  # R-B3 workspace ownership (item 4)
  - dce4e45f  # four macOS integration suites (item 7)
milestone: v3.7
covers_triage_items: [4, 7]
unblocks_jobs:
  - Windows Smoke
  - Windows Integration
  - Windows Regression
  - Integration Tests (4 of 5 suites)
---

# Quick Task 260907-bwq — Summary

**Status:** complete. Items 4 and 7 of the CI run `34068950597` triage, combined
because every finding has the same shape: **a test asserting behaviour the
project had deliberately changed.** No product defect among them.

## Item 4 — R-B3 workspace ownership (`38c03b86`)

Three *independent* jobs (`needs: changes`, not chained) — Windows Smoke,
Integration and Regression — all died on **one** test, because
`windows-test-harness.ps1` re-throws on the first non-zero cargo exit:

```
R-B3: the current user lacks WRITE_OWNER (0x00080000) on the workspace:
\\?\D:\a\nono\nono\crates\nono-cli
```

R-B3 is **correct**; its own error text predicts this case ("created from an
elevated console (owned by BUILTIN\Administrators)"). The workspace is the child
CWD (D-06), the test inherited the process CWD, and the runner's checkout is
Administrators-owned. The fix belongs in the test.

A bare `tempdir()` would **not** work — under elevation a fresh directory is also
Administrators-owned, the same fact `260906-q7n` turned on. The workspace must be
owned explicitly, the remedy R-B3's message recommends.

`take_ownership_for_current_user` is duplicated into `tests/common/mod.rs`.
Structural, not lazy: the canonical copy is `#[cfg(test)] mod` in `main.rs` — the
unit-test config of a **binary** crate — and `nono-cli` has no lib target, so
integration tests compile as separate crates and cannot reach it.

**Verification limit, stated not glossed.** This session is non-elevated, so a
tempdir is already user-owned and `icacls /setowner` is a no-op. **The failure
cannot be reproduced locally and the fix is unproven against the real condition
until CI runs it.** What *is* verified: compiles; 1 passed / 0 failed, exit 0;
the helper is genuinely on the executed path (perturbation-proved by making it
panic, seeing the failure name the tempdir, reverting, re-confirming green);
clippy and fmt clean.

## Item 7 — four macOS Integration Tests suites (`dce4e45f`)

`Integration Tests` is `runs-on: macos-latest` (not ubuntu — an earlier reading
of mine had that wrong).

| Suite | Cause | Fix |
|---|---|---|
| **Profiles** | `2652e256` deliberately removed the `opencode` profile (→ registry pack) and said it would *"substitute openclaw or swival in tests that previously used opencode"* — it updated the Rust tests and missed this shell suite | substituted `openclaw`; its `$HOME/.local/share/openclaw` is the exact structural analogue of the OpenTUI data dir the assertion checked, so the same property is still asserted |
| **Pack Resolution** | `test_pack_resolution.sh` **never existed in the fork**; the registry line arrived via upstream `0a09ff41` (PR #1152) absorbed **without** its companion script — a partial-absorb gap | entry disabled with evidence; the script exists upstream (`f1243c75`) and is **deliberately not imported** — absorbing upstream test code is a dispositioned sync decision, and a Unix bash suite can't be verified from Windows |
| **Audit Trail** | asserted `grep -q "command"`, a literal word the current output no longer emits — it failed while `audit list` worked and printed three sessions | session-id row regex `<YYYYMMDD>-<HHMMSS>-<pid>`; a header alone can no longer pass, so **strictly stronger** than what it replaced |
| **Trust CLI** | went sign → verify expecting VERIFIED, but the fork refuses: *"a repository cannot nominate its own signer"* — the anchor set is user-level only, by design | added `trust init --user`, the remedy `trust_cmd.rs:734` names |

On Trust CLI I first wrote that `with_test_env` isolates `HOME`. It does not — it
sets `XDG_CONFIG_HOME` and `NONO_TRUST_TEST_USER_POLICY_PATH`. Because that was
load-bearing for a "cannot clobber your real policy" claim, I traced the resolver
(`trust_cmd.rs:66` → `:1677`, live under `--features test-trust-overrides` per
`run_integration_tests.sh:31`) and corrected the comment to cite the real
mechanism.

Gates: `bash -n` clean on all four scripts. The suites are macOS/Linux and cannot
be executed from this Windows host.

## Deliberately NOT fixed — a real product defect

The **Shell** suite is not a stale test. `nono shell` on macOS is broken:

```
nono: setsid() failed while configuring child PTY   -> _exit(126)
```

`exec_strategy.rs:1166-1174` (macOS-only) makes the child a process-group leader
via `setpgid(0,0)`; `pty_proxy.rs:329` then calls `setsid()`, which POSIX
requires to fail with `EPERM` for a group leader. The comment at `:1156-1159`
asserts *"Both orderings are safe"* — that premise is false and is the bug. The
parent's `setpgid … ESRCH` warning is downstream noise: the child was already
dead.

Left for its own task; it is macOS-only code that cannot be executed here, and a
blind reorder of post-fork session setup is exactly the change that needs real
verification.

## Finding worth carrying

**Every one of these five was a test lagging a deliberate change** — a removed
profile, an unabsorbed upstream file, a changed output format, a tightened trust
rule. Three of them cited the intended remedy *in the very commit or error
message that broke them*. The gap is not knowledge, it is that nothing re-runs
the affected suite at the moment of the change: `2652e256` updated the Rust tests
it could see and missed a `.sh` file; `0a09ff41` brought a registry line without
its script. A change that alters a profile name, an output format, or a trust
rule should grep the `tests/` tree for the old string before landing.
