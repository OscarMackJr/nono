---
phase: 93-live-zt-infra-integration-revocation-request-flow
plan: "03"
subsystem: security
tags: [override-request, denial-bundle, nonce, clap, cli-01, approver-pipeline]

# Dependency graph
requires:
  - phase: 93-live-zt-infra-integration-revocation-request-flow
    plan: "02"
    provides: OverrideArgs + OverrideCommands { AuditEmit } skeleton in cli.rs

provides:
  - OverrideCommands::Request(OverrideRequestArgs) variant added onto Plan-02 skeleton
  - override_request.rs runtime: denial bundle { scope, repo_context, reason, nonce }
  - D-07 asymmetry documented in code (no Apply variant in nono.exe)
  - 5 unit tests: JSON shape, distinct nonces, nonce length/hex, scope, absent repo_context

affects:
  - nono-py Wave 2 plans (can invoke nono override request to surface denial bundle)
  - 93-live-zt-infra-integration-revocation-request-flow plan 04 (SC3 test harness)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "OverrideCommands additive variant — Plan 03 adds Request onto Plan-02 skeleton without re-declaring the enum (single-author-per-type ownership)"
    - "rand::RngExt::fill for 16-byte nonce generation (rand 0.10 API; cross-platform, no uuid dep)"
    - "build_bundle extracted from run_override_request for testability without I/O"
    - "D-07 asymmetry documented in OverrideCommands doc-comment (no Apply in nono.exe)"

key-files:
  created:
    - crates/nono-cli/src/override_request.rs
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/app_runtime.rs
    - crates/nono-cli/src/main.rs

key-decisions:
  - "rand::RngExt::fill (rand 0.10) used for nonce — already in Cargo.toml; uuid dep is Windows-only so rand is the correct cross-platform choice"
  - "build_bundle extracted as a pure fn for unit testability (no I/O side effects in tests)"
  - "Task commit order: Task 2 (cli.rs + app_runtime.rs) committed first because Task 1 (override_request.rs) imports OverrideRequestArgs from cli.rs; commit graph is buildable at every step"
  - "D-07 asymmetry note moved into OverrideCommands doc-comment (not just the Commands enum comment) so it is visible to any reader of the enum definition"

requirements-completed: [CLI-01]

# Metrics
duration: 25min
completed: 2026-06-22
---

# Phase 93 Plan 03: nono override request — denial bundle for the approver pipeline (CLI-01)

**Additive `Request` variant onto the Plan-02 `OverrideCommands` skeleton; new `override_request.rs` runtime that gathers denial context and emits a structured JSON bundle + fresh nonce for the out-of-nono approver/KMS-signing pipeline**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-06-22
- **Completed:** 2026-06-22
- **Tasks:** 2 (Task 1: runtime + tests; Task 2: cli.rs variant + dispatch)
- **Files modified:** 4 (1 created, 3 modified)

## Accomplishments

- Created `override_request.rs` with `run_override_request(OverrideRequestArgs)` that gathers scope paths/domains, `repo_context`, and denial reason; emits a JSON bundle `{ "scope": { "paths": [...], "domains": [...] }, "repo_context": "<repo>", "reason": "<reason>", "nonce": "<32-hex>" }` plus a human-readable summary to stdout
- Fresh per-invocation nonce from 16 random bytes (hex-encoded, 128-bit entropy) via `rand::RngExt::fill` — addresses T-93-03-01 replay surface; two calls always yield distinct nonces
- No crypto, no live check (D-07 / D-08) — pure context gather and bundle emit
- 5 unit tests: four-key JSON shape, distinct nonces, nonce length/hex validation, scope paths/domains contents, absent repo_context emits empty string
- Added `OverrideCommands::Request(OverrideRequestArgs)` variant onto the Plan-02 skeleton; `OverrideRequestArgs` struct with `--reason`, `--scope-paths`, `--scope-domains`, `--repo-context` clap args
- Extended `OverrideCommands` doc-comment with explicit D-07 asymmetry note: no `Apply` variant in nono.exe — `apply` lives in nono-py `nono-override-apply` console script
- Wired `OverrideCommands::Request` dispatch arm in `app_runtime.rs`; both `audit-emit` (Plan 02) and `request` (Plan 03) reachable under the same `nono override` group

## Task Commits

Each task committed atomically (Task 2 before Task 1 to preserve buildable commit graph):

1. **Task 2: Add ONLY the Request variant onto the Plan-02 OverrideCommands skeleton** — `7f41a335` (feat)
2. **Task 1: override_request runtime — denial bundle + nonce (CLI-01)** — `67af627e` (feat)

## Files Created/Modified

- `crates/nono-cli/src/override_request.rs` (NEW) — `run_override_request` runtime + `build_bundle` helper + `fresh_nonce` + 5 unit tests
- `crates/nono-cli/src/cli.rs` — Added `OverrideCommands::Request(OverrideRequestArgs)` variant + `OverrideRequestArgs` struct; extended OverrideCommands doc-comment with D-07 asymmetry; updated after_help example
- `crates/nono-cli/src/app_runtime.rs` — Added `OverrideCommands::Request` dispatch arm
- `crates/nono-cli/src/main.rs` — Added `mod override_request` with Phase 93 Plan 03 comment

## Decisions Made

- **rand::RngExt::fill for nonce generation** — `rand = "0.10"` is already a direct dep of nono-cli. The `uuid` dep is Windows-only (`[target.'cfg(target_os = "windows")'.dependencies]`), making `rand` the correct cross-platform choice. Used `rand::RngExt::fill` (the rand 0.10 API) rather than the rand 0.8 `RngCore::fill_bytes`.
- **build_bundle extracted for testability** — The JSON assembly logic is extracted into a pure `build_bundle` fn so unit tests can call it without any stdout I/O; `run_override_request` handles stdout output only.
- **Commit order: Task 2 first** — `override_request.rs` imports `OverrideRequestArgs` from `cli.rs`, so the cli.rs changes must land in a commit before the override_request.rs commit for the commit history to be buildable at every step. The plan's numbered order (Task 1, Task 2) reflects logical dependency; commit order is inverted to maintain build integrity.
- **D-07 asymmetry note in OverrideCommands doc-comment** — Moved the "no Apply here" note into the enum's Rust doc-comment (not just the Commands variant attribute) so any future reader or refactoring tool sees it close to the enum definition.

## Deviations from Plan

**One auto-fix (Rule 1 — Bug):**

**[Rule 1 - Bug] rand 0.10 API: RngCore/Rng → RngExt**
- **Found during:** Task 1 first test run
- **Issue:** The initial implementation used `rand::RngCore::fill_bytes` and `rand::Rng::fill` (rand 0.8 API names). `rand = "0.10"` restructured the trait surface: the `fill` method for slices lives on `rand::RngExt`, not `rand::Rng`.
- **Fix:** Changed import to `use rand::RngExt` and call to `rand::rng().fill(&mut bytes)`.
- **Files modified:** `crates/nono-cli/src/override_request.rs`
- **No separate commit** — fixed within the same task session before the first test run succeeded.

## Cross-Target Clippy Status: PARTIAL→CI

Per CLAUDE.md § Coding Standards:

- `override_request.rs` — not cfg-gated Unix code; cross-target verification not strictly required
- `cli.rs` / `main.rs` / `app_runtime.rs` — not cfg-gated Unix code; cross-target verification not required

**Native Windows clippy (`cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`):** PASSED — 0 errors, 0 warnings.

**Cross-target verification:** Not required for these files (no `cfg(target_os)` gating on the touched code). CI on `ubuntu-latest` will run the full workspace cross-target check.

## Known Stubs

None. All functionality is fully wired:
- `run_override_request` calls `fresh_nonce` (real RNG), `build_bundle` (real JSON), and prints to stdout
- No hardcoded or placeholder data in the bundle path

## Threat Surface Scan

No new trust boundaries beyond those declared in the plan's threat model:
- `T-93-03-01` (replay via bundle reuse) → MITIGATED: fresh 128-bit nonce per invocation
- `T-93-03-02` (request implies grant) → ACCEPTED: no verification or capability grant in this command
- `T-93-03-03` (bundle leaks secrets) → MITIGATED: bundle carries only scope/reason/nonce from args; no credentials, no key material; existing CLI redaction policy applies

## Self-Check

- [x] `crates/nono-cli/src/override_request.rs` — created at correct path
- [x] `7f41a335` — cli.rs + app_runtime.rs commit exists
- [x] `67af627e` — override_request.rs + main.rs commit exists
- [x] `nono override request --help` reachable and renders args correctly
- [x] `nono override audit-emit --help` still reachable (Plan 02 unaffected)
- [x] `cargo test -p nono-cli override_request`: 5/5 passed
- [x] `cargo build -p nono-cli --bin nono`: exits 0
- [x] `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`: 0 errors

## Self-Check: PASSED

---
*Phase: 93-live-zt-infra-integration-revocation-request-flow*
*Completed: 2026-06-22*
