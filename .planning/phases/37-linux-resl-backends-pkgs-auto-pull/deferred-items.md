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
