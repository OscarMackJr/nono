---
quick_id: 260606-mp9
slug: fix-unix-borrow-error-claude-hook
status: complete
commit: 4de294e8
created: 2026-06-06
---

# Quick Task 260606-mp9 — Summary

## Fix Applied

`crates/nono-cli/src/claude_code_hook.rs`, function `wrapped_bash_command`
(`#[cfg(not(target_os = "windows"))]`, line ~409):

Introduced a named binding `nono_exe_display` to hold the `String` produced by
`nono_exe.display().to_string()` before passing a borrow of it to
`shlex::try_quote`. This prevents E0716 "temporary value dropped while borrowed"
because the `Cow<str>` returned by `try_quote` borrows the named binding, which
now outlives the statement.

No behavioral change.

## Commit

`4de294e8` — `fix(60,windows-drift): bind temp String in wrapped_bash_command to fix Unix E0716 (claude_code_hook)`

Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>

## Clippy Results

### Target 1: x86_64-unknown-linux-gnu (decisive — the target that failed in CI run 27031289871)

**PARTIAL / CI-deferred**

Cargo fails at the `ring v0.17.14` and `aws-lc-sys v0.41.0` build scripts before
reaching any Rust source, because the C cross-compiler (`x86_64-linux-gnu-gcc`) is
not installed on this Windows host. This is a pre-existing infrastructure gap —
the same failure occurs on the unmodified code. The rust-std component for
`x86_64-unknown-linux-gnu` IS installed (`rustup target list --installed` confirms
it), but `ring`/`aws-lc-sys` have C build scripts that require a native C
cross-compiler. Per CLAUDE.md §Coding Standards:

> If the cross-toolchain is not installed, the related verification REQ MUST be
> marked PARTIAL and deferred to live CI.

Deferred to live CI (GitHub Actions Ubuntu runners have the cross-toolchain
installed and will exercise this target on the next push).

### Target 2: x86_64-apple-darwin

**PARTIAL / CI-deferred**

Same root cause as Target 1: `ring`/`aws-lc-sys` C build script fails (missing
`cc`). Deferred to live CI (macOS runners).

### Target 3: x86_64-pc-windows-msvc (default, regression guard)

**PASS — clean**

```
Checking nono-cli v0.62.0
Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.81s
```

No warnings, no errors. The `#[cfg(not(target_os = "windows"))]` block is excluded
on this target as expected, but the file parses and the surrounding Windows code is
still clean.

## Correctness Rationale

The fix is canonical for E0716. The original code created a temporary `String`
(`nono_exe.display().to_string()`) inline in the argument position of
`shlex::try_quote(...)`, which returns a `Cow<str>` borrowing that temporary. The
borrow checker rejects this because the temporary drops at the end of the
expression before the `Cow` can be used. Binding the `String` to a named local
(`nono_exe_display`) ensures it lives to the end of the block, satisfying the
borrow checker. This is the minimal, zero-behavioral-change fix.

## Deviations

None. The fix matches the plan exactly.
