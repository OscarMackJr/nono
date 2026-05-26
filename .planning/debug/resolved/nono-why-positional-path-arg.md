---
slug: nono-why-positional-path-arg
status: resolved
fix_decision: |
  Option (c) — positional path + improved help (user-selected 2026-05-26).
  1. Add optional positional `path_arg: Option<PathBuf>` to WhyArgs with
     `#[arg(value_name = "PATH", conflicts_with = "path")]`.
  2. In why_runtime, resolve effective path as `args.path.clone().or_else(|| args.path_arg.clone())`
     before the existing `if let Some(ref path)` branch.
     NOTE from investigation: why_runtime already defaults --op to Read when
     path is Some and op is None, so `nono why <path>` alone works — no op
     default change needed.
  3. Tighten the why help_template USAGE line + after_help examples to show
     BOTH `--path <PATH>` and the bare `<PATH>` forms.
  4. Add a test asserting the positional path resolves (and the conflict guard
     rejects passing both --path and positional).
trigger: |
  PS C:\Users\OMack\nono-poc> nono why ~/.ssh/id_rsa
  error: unexpected argument 'C:\Users\OMack/.ssh/id_rsa' found

  Usage: nono.exe why [OPTIONS]

  For more information, try '--help'.
created: 2026-05-26
updated: 2026-05-26
---

# Debug Session: `nono why` rejects a positional path argument

## Symptoms

- **Expected behavior:** `nono why ~/.ssh/id_rsa` reports whether that path is
  allowed or denied (and why), the way a user naturally expects a `why <path>`
  command to behave.
- **Actual behavior:** clap rejects the bare path with
  `error: unexpected argument 'C:\Users\OMack/.ssh/id_rsa' found`, prints
  `Usage: nono.exe why [OPTIONS]`, and exits non-zero. No path query runs.
- **Error message:** `unexpected argument '...' found` (clap unknown-positional).
- **Timeline:** Behavior of the current `why` command shape; not a regression in
  this session's scope — the command has never accepted a positional path.
- **Reproduction:** `nono why ~/.ssh/id_rsa` (any bare path positional) on Windows.
  The shell expands `~` to `C:\Users\OMack` before nono sees it, but expansion is
  incidental — the failure is that `why` accepts no positional at all.

## Current Focus

hypothesis: |
  `WhyArgs` (crates/nono-cli/src/cli.rs:2511) defines `path` as a NAMED flag
  (`#[arg(long)] pub path: Option<PathBuf>`), not a positional. clap therefore
  treats the bare `~/.ssh/id_rsa` as an unexpected positional argument and
  errors out. The documented form is `nono why --path <PATH> --op <OP>`
  (see after_help examples at cli.rs:734-740). Two distinct gaps:
    1. UX/discoverability: the error + `Usage: why [OPTIONS]` line never mention
       `--path`, so the user has no hint how to fix the invocation.
    2. Ergonomics: a `why <path>` command conventionally accepts the path
       positionally; requiring `--path` is surprising.
test: |
  Confirm WhyArgs has no positional path field; confirm clap config (no
  trailing-var-arg / no positional) produces exactly this error; decide whether
  the fix is (a) add an optional positional path that fills `path` when --path
  is absent, (b) improve the error/usage to surface --path, or (c) both.
expecting: |
  WhyArgs path is `#[arg(long)]` only. No positional binding. Fix is a small,
  contained clap change in cli.rs plus the why-command runtime that reads it.
next_action: COMPLETE
reasoning_checkpoint: |
  Root cause confirmed. Fix implemented, tested, and committed.

## Evidence

- timestamp: 2026-05-26
  finding: |
    cli.rs:2511-2518 — `WhyArgs.path: Option<PathBuf>` carries `#[arg(long, help_heading = "QUERY")]`,
    making `--path` a named option. No `#[arg(...)]` without `long` (i.e. no positional)
    exists for the path. `op` is `Option<WhyOp>` (also `--op`, named). after_help
    examples (cli.rs:735-739) all use `--path`/`--host`. Hence clap's "unexpected
    argument" on a bare positional path.

## Eliminated

(none)

## Resolution

root_cause: |
  `WhyArgs.path` in crates/nono-cli/src/cli.rs was declared with `#[arg(long)]` only,
  making it a named flag. clap had no positional binding for `why`, so any bare argument
  like `nono why /some/path` was rejected as "unexpected argument".
fix: |
  Added `pub path_arg: Option<PathBuf>` to `WhyArgs` with
  `#[arg(value_name = "PATH", conflicts_with = "path")]` (positional, mutually exclusive
  with `--path`). Updated `why_runtime.rs` to resolve the effective path as
  `args.path.clone().or_else(|| args.path_arg.clone())` before the existing query
  dispatch. Updated USAGE + after_help examples to show both forms. Added two tests:
  one asserting positional resolution, one asserting the conflicts_with guard fires.
verification: |
  `cargo build -p nono-cli` — EXIT 0
  `cargo test -p nono-cli test_why` — 2 passed, 0 failed
  `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` — EXIT 0 (clean)
files_changed:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/why_runtime.rs
