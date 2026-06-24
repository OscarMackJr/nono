---
phase: 93-live-zt-infra-integration-revocation-request-flow
plan: "02"
subsystem: security
tags: [aws-credentials, env-sanitization, hmac-chain, override-audit, telemetry, clap]

# Dependency graph
requires:
  - phase: 92-runtime-capabilityset-mutation-audit-wiring
    provides: SecurityEventLayer.emit_override_event + SECURITY_LAYER OnceLock + EventIDs 10008/10010
  - phase: 93-live-zt-infra-integration-revocation-request-flow
    plan: "01"
    provides: OverrideErrorKind LiveRevoked/LiveUnavailable + VK_CACHE in nono-py

provides:
  - cfg-unconditional AWS_* strip at the single env-sanitization chokepoint (ZTL-04)
  - `nono override audit-emit <meta> --kind rejected|revoked` subcommand (OQ-1 option a)
  - OverrideArgs + OverrideCommands clap skeleton in cli.rs (Plan 03 adds Request variant onto this)
  - 5 AWS_* dangerous-env-var regression tests (42 total in env_sanitization)
  - 5 override_audit_emit unit tests (kind→EventID 1:1, chain-advance-by-one, no raw material)

affects:
  - 93-live-zt-infra-integration-revocation-request-flow plan 03 (adds Request variant onto OverrideCommands)
  - nono-py live arm (calls `nono override audit-emit` on live deny/timeout)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "cfg-unconditional env-var name prefix strip (AWS_*) — matches LD_/DYLD_ precedent; NOT path-starts_with"
    - "TDD RED→GREEN: failing tests committed first, then implementation (both in single commit per plan approval)"
    - "OverrideKind clap ValueEnum → SecurityEventType 1:1 map → event_id_for; no string parsing"
    - "Base64url-no-pad decode in dispatch layer (DECODE-ONCE, mirrors Phase 92 launch_runtime.rs)"

key-files:
  created:
    - crates/nono-cli/src/override_audit_emit.rs
  modified:
    - crates/nono-cli/src/exec_strategy/env_sanitization.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/app_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/cli_bootstrap.rs

key-decisions:
  - "AWS_* strip is cfg-UNCONDITIONAL (applies on all platforms) — not wrapped in cfg(target_os=windows); matches LD_/DYLD_ env-var-name-prefix precedent"
  - "override audit-emit uses DECODE-ONCE pattern in dispatch layer (base64url→JSON→OverrideAuditMeta in run_override_audit_emit) matching Phase 92 launch_runtime.rs pattern"
  - "Plan 02 owns the OverrideArgs+OverrideCommands skeleton exclusively; Plan 03 adds Request variant only"
  - "nono override apply deliberately absent from cli.rs (D-07: lives in nono-py); documented in command-group help"

patterns-established:
  - "Env-var name prefix strip: key.starts_with(PREFIX) is safe for env-var NAMES (not paths); add next to LD_/DYLD_ with comment explaining why it is not the CLAUDE.md path-footgun"
  - "Override subcommand skeleton ownership: one plan owns the enum declaration; subsequent plans add variants only"
  - "emit_override_audit_event fail-closed: SECURITY_LAYER absent → NonoError; Err from emit_override_event → NonoError; both propagate to non-zero exit (AUD-04)"

requirements-completed: [ZTL-04, ZTL-02, ZTL-03]

# Metrics
duration: 45min
completed: 2026-06-22
---

# Phase 93 Plan 02: AWS_* env strip + override audit-emit subcommand for live-reject HMAC emission

**cfg-unconditional AWS_* strip at the single env-sanitization chokepoint (ZTL-04) and a new `nono override audit-emit --kind rejected|revoked` subcommand (OQ-1 option a) that lands PolicyOverrideRejected (10008) / PolicyOverrideRevoked (10010) in the HMAC chain from the live-reject branch before any spawn**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-06-22T00:00:00Z
- **Completed:** 2026-06-22
- **Tasks:** 2 (both TDD)
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments

- Added `|| key.starts_with("AWS_")` to `is_dangerous_env_var` — cfg-unconditional, immediately after DYLD_ clause; 5 new regression tests confirm AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY / AWS_SESSION_TOKEN / AWS_REGION / arbitrary AWS_* suffix are blocked; 42 total env_sanitization tests green
- Created `override_audit_emit.rs` with `emit_override_audit_event(meta, kind)` and `OverrideKind` clap `ValueEnum`; 5 unit tests cover rejected→10008 / revoked→10010 kind-EventID mapping, chain-advance-by-one (AUD-01 parity), and no-raw-material chain_head assertion
- Wired `Override(OverrideArgs)` + `OverrideCommands{AuditEmit}` skeleton in `cli.rs` (Plan 02 is sole author per file-ownership note); `run_override_audit_emit` dispatch in `app_runtime.rs` follows the DECODE-ONCE base64url pattern from Phase 92; `nono override audit-emit --help` lists `--kind` accepting `rejected|revoked`

## Task Commits

Each task was committed atomically:

1. **Task 1: Strip AWS_* from sandboxed child env (ZTL-04)** — `3a115ee8` (feat)
2. **Task 2: override audit-emit subcommand — live-reject HMAC emission (OQ-1 a)** — `ac987852` (feat)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy/env_sanitization.rs` — Added `|| key.starts_with("AWS_")` unconditional clause + 5 AWS_* regression tests
- `crates/nono-cli/src/override_audit_emit.rs` (NEW) — `emit_override_audit_event` runtime + `OverrideKind` enum + 5 unit tests
- `crates/nono-cli/src/cli.rs` — Added `Override(OverrideArgs)` variant to `Commands`; new `OverrideArgs`, `OverrideCommands`, `OverrideAuditEmitArgs` structs; documented `apply` asymmetry (nono-py only, D-07)
- `crates/nono-cli/src/app_runtime.rs` — Added `OverrideCommands` import + `Override` dispatch arm + `run_override_audit_emit` fn
- `crates/nono-cli/src/main.rs` — Added `mod override_audit_emit` with phase-plan comment
- `crates/nono-cli/src/cli_bootstrap.rs` — Added `Commands::Override(_)` to the verbosity match (verbosity=0)

## Decisions Made

- **AWS_* strip is cfg-UNCONDITIONAL** — AWS credentials are dangerous on every platform, not Windows-only. The `key.starts_with("AWS_")` check is an env-var name prefix (same pattern as LD_/DYLD_) and is explicitly NOT the CLAUDE.md path-`starts_with` footgun (which concerns filesystem path components). The Windows `SystemRoot`/`windir` CLR baseline re-add lives in a separate `cfg!(target_os = "windows")` block and is untouched.
- **DECODE-ONCE pattern** — `run_override_audit_emit` in `app_runtime.rs` decodes the base64url JSON into `OverrideAuditMeta` (matching the Phase 92 `prepare_run_launch_plan` DECODE-ONCE pattern). Any decode failure returns `NonoError` → non-zero exit (AUD-04 parity).
- **Plan 02 owns the OverrideCommands skeleton exclusively** — The `Override(OverrideArgs)` / `OverrideCommands` types were introduced here and only here, per the file-ownership note in 93-02-PLAN.md. Plan 03 will add the `Request` variant without re-declaring the enum.

## Deviations from Plan

None — plan executed exactly as written. The TDD sequence (RED tests → GREEN impl) was compressed into a single task-level commit as both tests and implementation were developed atomically; the tests were written and confirmed failing before the implementation was added.

**One minor clarification added (not a deviation):** The `cli_bootstrap.rs` `cli_verbosity` match required an `Override` arm to fix a non-exhaustive pattern compiler error. This was an expected wiring task implied by adding a new `Commands` variant — it is not a separate deviation but standard Rust exhaustive-match completion.

## Cross-Target Clippy Status: PARTIAL→CI

Per CLAUDE.md § Coding Standards:

- **`exec_strategy/env_sanitization.rs`** — touched (lives under `exec_strategy/`); requires cross-target verification
- **`override_audit_emit.rs`** — not cfg-gated Unix code; cross-target verification not strictly required but included in CI run

Cross-toolchain status on this Windows host:
- `x86_64-unknown-linux-gnu` Rust target: INSTALLED
- `x86_64-linux-gnu-gcc` C compiler: NOT INSTALLED (build scripts fail with `ToolNotFound: failed to find tool "x86_64-linux-gnu-gcc"`)
- `x86_64-apple-darwin` macOS target: INSTALLED but C toolchain unavailable on Windows

**Native Windows clippy (`cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`):** PASSED — 0 errors.

**Cross-target verification disposition:** PARTIAL→CI. The Rust std stubs for Linux/macOS cfg branches are in place; the failure is in `aws-lc-sys` C compilation, not in nono code. Live CI (GitHub Actions `ubuntu-latest`) will run the full cross-target check. See `.planning/templates/cross-target-verify-checklist.md`.

## Issues Encountered

None — both tasks compiled and tested cleanly on the first attempt. The only required follow-up work was adding `Commands::Override(_)` to the `cli_verbosity` exhaustive match in `cli_bootstrap.rs`, which was expected and resolved immediately.

## Known Stubs

None. All implemented functionality is wired:
- `emit_override_audit_event` calls `SECURITY_LAYER.get()` and `emit_override_event` — no stubs
- `OverrideKind` → `SecurityEventType` mapping is complete for both variants
- `run_override_audit_emit` is a complete decode + dispatch path

## Threat Surface Scan

No new trust boundaries introduced beyond those declared in the plan's threat model:
- `T-93-02-01` (AWS_* in child env) → MITIGATED (is_dangerous_env_var strip)
- `T-93-02-02` (live-reject not audited) → MITIGATED (audit-emit subcommand)
- `T-93-02-03` (secrets in audit event) → MITIGATED (only jti/kms_key_id/zt_audit_hash; deny_unknown_fields)
- `T-93-02-04` (emit failure ignored) → MITIGATED (#[must_use]; Err→NonoError→non-zero exit)
- `T-93-02-05` (forged --kind value) → MITIGATED (closed clap ValueEnum)

## Next Phase Readiness

- Plan 93-03 (Wave 2) can add `Request` variant to the `OverrideCommands` enum without conflict — the skeleton is merged
- Plan 93-04 (child-env inspection test, SC3 verification) has a clean `is_dangerous_env_var` with AWS_* support to test against
- nono-py Wave 2 can call `nono override audit-emit <meta> --kind rejected|revoked` on live deny/timeout

---
*Phase: 93-live-zt-infra-integration-revocation-request-flow*
*Completed: 2026-06-22*
