# Phase 37 deferred items

Out-of-scope discoveries encountered during execution. Logged per executor
scope-boundary rule.

## Plan 37-02

### CI doc-flag-allowlist drift: `--dangerous-force-wfp-ready`

`bash .github/scripts/check-cli-doc-flags.sh` reports:

```
Missing RunArgs flags in docs/cli/usage/flags.mdx:
  --dangerous-force-wfp-ready
```

**Origin:** `--dangerous-force-wfp-ready` was added to `SandboxArgs` by Phase 41
(REQ-CI-02) but never documented in `docs/cli/usage/flags.mdx`. It is gated
behind `NONO_TEST_HARNESS` and is a test-only flag, so the omission may be
intentional.

**Scope:** out-of-scope for Plan 37-02 (the plan adds `--no-auto-pull`, which
IS correctly documented in the same docs file).

**Action:** no fix here; flag for a Phase 41 follow-up plan or a Phase 37
fix-pass if the executor encounters the same CI failure on the green-gate run.

### Pre-existing Phase 41 test failure: `broker_launch_assigns_child_to_job_object`

`cargo test -p nono-cli --bin nono` (Windows host) fails this test with:

```
nono-shell-broker.exe missing at ...\target\x86_64-pc-windows-msvc\release\
  nono-shell-broker.exe and ...\target\release\nono-shell-broker.exe;
  pre-build with `cargo build -p nono-shell-broker --release` (or set the
  broker pre-build via crates/nono-cli/build.rs per Phase 41 D-14).
```

**Origin:** Phase 41 D-14 added a release-mode broker pre-build requirement
that is not satisfied by a `cargo build -p nono-cli` (debug) invocation. The
test asserts Job Object containment is enforced before ResumeThread and was
intentionally written to fail rather than silently skip.

**Scope:** out-of-scope for Plan 37-02 (touches `nono-cli/src/cli.rs` +
`profile/mod.rs` + new `diagnostic_formatter.rs`; does NOT touch
`exec_strategy_windows/launch.rs` or the broker harness).

**Action:** no fix here; this test is independently green on CI when the
release-mode broker pre-build runs. Plan 37-02 verification is satisfied via
the targeted test-name filters that exclude this pre-existing Windows-host
test infrastructure issue.
