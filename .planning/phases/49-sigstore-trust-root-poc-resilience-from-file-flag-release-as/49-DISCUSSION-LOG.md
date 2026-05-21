# Phase 49: Sigstore trust-root POC resilience - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-21
**Phase:** 49-sigstore-trust-root-poc-resilience-from-file-flag-release-as
**Areas discussed:** Plan slicing & parallelism, Cache-write semantics, Smoke script flavor, Test fixture strategy

---

## Plan slicing & parallelism

### Q1: How should Phase 49 be sliced into plans?

| Option | Description | Selected |
|--------|-------------|----------|
| 3 plans, one per REQ | 49-01 `--from-file` (REQ-POC-TRUST-01), 49-02 release-asset (REQ-POC-TRUST-02), 49-03 cadence template + smoke script + docs (REQ-POC-TRUST-03). Mirrors Phase 44 D-44-A1 + Phase 45 D-45-A1 v2.6 cadence. | ✓ |
| 2 plans: code+CI bundled vs docs+template | 49-01 bundles REQ-POC-TRUST-01 + REQ-POC-TRUST-02 (CI proves the code); 49-02 = REQ-POC-TRUST-03. | |
| 1 mega-plan covering all three | Atomic single-commit-stream phase; small-refactor-phase shape. | |

**User's choice:** 3 plans, one per REQ — locks D-49-A1.
**Notes:** Matches v2.6 "one plan per REQ" cadence established by Phase 44 + Phase 45.

### Q2: Within the 3-plan wave, what ordering / parallelism applies?

| Option | Description | Selected |
|--------|-------------|----------|
| All 3 parallel-safe in one wave | Surfaces disjoint (CLI ↔ release.yml ↔ templates+scripts+docs). 49-03 smoke script can be scaffolded against the SPEC without 49-01 landing. | ✓ |
| 49-01 first, then 49-02 + 49-03 parallel | Sequential gate: ship `--from-file` before the asset + before the smoke script. | |
| 49-02 first, then 49-01 + 49-03 parallel | Release-asset ships first so docs can reference an existing URL. | |

**User's choice:** All 3 parallel-safe — locks D-49-A2.
**Notes:** Plan 49-03 smoke script wraps the SPEC'd `--from-file` flag shape; planner scaffolds at plan-execute time and skips live-CLI smoke if 49-01 hasn't landed yet.

---

## Cache-write semantics

### Q3: How should `--from-file` write the validated input to the cache?

| Option | Description | Selected |
|--------|-------------|----------|
| Verbatim byte-copy of validated input | `std::fs::copy(<PATH>, <cache>)` after validation succeeds. Cache file is byte-identical to input — preserves SHA-256 release-asset provenance. | ✓ |
| Validated round-trip via serde_json | `serde_json::to_string_pretty(&trusted_root)` then write (mirrors `--refresh-trust-root` path at setup.rs:849-852). Re-serialization changes whitespace + may reorder map keys. | |
| Atomic tmpfile + rename | `<cache>.tmp` then `rename`. Crash-safety win; introduces inconsistency with `--refresh-trust-root` single-shot `fs::write`. | |

**User's choice:** Verbatim byte-copy — locks D-49-B1.
**Notes:** Preserves SHA-256 byte-identity end-to-end with the release-asset provenance chain (Plan 49-02 CI gate).

### Q4: What happens if validation succeeds but `std::fs::copy` partially writes the cache file?

| Option | Description | Selected |
|--------|-------------|----------|
| Best-effort delete on copy failure + propagate IO error | `let _ = fs::remove_file(<cache>);` on Err; propagate the original `NonoError::Io`. Cache is fully written or absent. | ✓ |
| Atomic rename via tmpfile (revisit prior question) | Reconsider tmpfile+rename pattern for partial-write safety. | |
| Punt: trust the FS, no cleanup | Just propagate the IO error; no cache cleanup. | |

**User's choice:** Best-effort delete + propagate — locks D-49-B2.
**Notes:** Honors SPEC.md REQ-POC-TRUST-01 acceptance criterion (c) "malformed input does NOT create or modify the cache file" extended to mid-copy IO failures.

### Q5: Stdout messaging on `--from-file` success?

| Option | Description | Selected |
|--------|-------------|----------|
| Mirror --refresh shape + add `Source:` line | Existing header + cache-at line + NEW "Source: <abs_path>" breadcrumb. | ✓ |
| Mirror --refresh shape verbatim | Same prints as setup.rs:826-857 with "Loading" verb. | |
| Silent on success (CLI convention) | Print nothing on success. | |

**User's choice:** Mirror shape + `Source:` breadcrumb — locks D-49-B3.
**Notes:** Aids debugging when POC user forgets which fixture they used.

---

## Smoke script flavor

### Q6: What flavor of smoke script?

| Option | Description | Selected |
|--------|-------------|----------|
| Matched `.sh` + `.ps1` pair | `scripts/verify-trust-root-cached.sh` + `.ps1`. First-class Windows POC UX. ~20 lines each. | ✓ |
| Bash-only with Windows note | Single `.sh` script; doc says "use Git Bash on Windows". | |
| Cargo test harness | Integration test driven by env var; zero new scripting languages. | |

**User's choice:** Matched `.sh` + `.ps1` pair — locks D-49-C1.
**Notes:** Mismatch with the fork's Windows-first POC posture is avoided.

### Q7: What known-good bundle + source does the smoke script run against?

| Option | Description | Selected |
|--------|-------------|----------|
| Use existing trust-test fixtures | Reuse the bundle + source pair already maintained for `crates/nono/tests/`. Zero new fixtures per Sigstore rotation. | ✓ |
| Ship dedicated smoke-test fixtures alongside the script | `scripts/fixtures/smoke-bundle.sigstore.json` + `scripts/fixtures/smoke-source.toml`. Self-contained but 2 more fixtures to refresh. | |
| Take both as CLI args | `bash verify-trust-root-cached.sh <trust_root.json> <bundle> <source>`. | |

**User's choice:** Existing trust-test fixtures — locks D-49-C2.
**Notes:** Planner inventories the fixture paths at plan-open via grep.

### Q8: PR CI integration?

| Option | Description | Selected |
|--------|-------------|----------|
| Maintainer-only; not wired into CI | Referenced from `sigstore-rotation-refresh.md`; existing `cargo test` continues as PR CI regression gate. | ✓ |
| Wire smoke script into PR CI as a separate lane | Added GH Actions job; belt-and-suspenders. ~30s added per PR. | |
| Wire smoke script into release workflow only | Run once per release tag. | |

**User's choice:** Maintainer-only — locks D-49-C3.
**Notes:** Avoids duplicate-gate cost; existing `cargo test -p nono trust::bundle::load_test_trusted_root_smoke` already covers the regression class.

---

## Test fixture strategy

### Q9: How to construct expired/malformed fixtures for `--from-file` tests?

| Option | Description | Selected |
|--------|-------------|----------|
| Runtime helper that mutates frozen fixture in TempDir | Test-only helper reads `trust-root-frozen.json`, mutates per case (expired tlog dates / truncation / quote-flip) in TempDir. ONE fixture to maintain. | ✓ |
| Check in dedicated fixtures next to `trust-root-frozen.json` | Add `trust-root-expired.json` + `trust-root-malformed.json`. Failure mode obvious in test output but 2 more fixtures to refresh per rotation. | |
| Synthesize entirely in-test (no fixture base) | Hand-construct `TrustedRoot` JSON literal via `serde_json::json!{...}`. | |

**User's choice:** Runtime mutation in TempDir — locks D-49-D1.
**Notes:** Mutation logic explicit and inspectable in the test file. "Expired" stays robust through Sigstore rotations because the helper sets dates to 1970, not relative-to-current-time.

### Q10: Where does the integration test live?

| Option | Description | Selected |
|--------|-------------|----------|
| `crates/nono-cli/tests/setup_from_file.rs` via `assert_cmd` | New integration test alongside the existing `crates/nono-cli/tests/` suite. Matches the pattern at `auto_pull_e2e_linux.rs` and `resl_nix_linux.rs`. | ✓ |
| Unit-test in setup.rs via factored helper | `from_file_inner` helper, unit test inside `#[cfg(test)] mod tests`. Doesn't exercise clap-mutex. | |
| Mix: unit for validation + integration for clap-mutex | Split test files; harder to find. | |

**User's choice:** Integration test via `assert_cmd` — locks D-49-D2.
**Notes:** Zero new deps (`assert_cmd` + `tempfile` already in workspace via Phase 44 inventory).

---

## Claude's Discretion

- `setup.rs` phase-index threading (cosmetic ordering through `refresh_trust_root_phase_index()` + `total_phases()` arithmetic).
- Exact clap `conflicts_with` attribute spelling for clap v4 derive.
- Per-plan commit shape (likely 1 commit per plan; Plan 49-03 may split into 1-3 commits at planner discretion).
- CI step placement within `release.yml` (before/after existing artifact assembly).
- POC-handoff docs rewrite scope (minimum = the `Known issue` subsection at lines 182-220; may sweep adjacent subsections if they reference `--refresh-trust-root` for consistency).
- Cross-target clippy scope (MUST run on `x86_64-unknown-linux-gnu` AND `x86_64-apple-darwin` per CLAUDE.md MUST/NEVER rule; PARTIAL allowed only per `.planning/templates/cross-target-verify-checklist.md`).

## Deferred Ideas

- Automated `scripts/refresh-trust-root-fixture.sh` harness (v2.7 candidate; SPEC.md out-of-scope).
- `--force` flag or freshness-aware overwrite protection on `--from-file` (v2.7 follow-up).
- Bundling `trusted_root.json` into the Windows MSIs (breaks "rotate independently of binary" invariant).
- Authenticode/Sigstore-signing the `trusted_root.json` release asset itself (redundant; `SHA256SUMS.txt` covers integrity).
- Cross-binding lockstep with `../nono-py/` + `../nono-ts/` siblings (not needed; bindings inherit verify-is-offline).
- Phase 44 follow-up todos (`44-class-d-validator-preflight-investigation.md`, `44-validate-restore-target-fd-relative-hardening.md`) — surfaced by `todo.match-phase 49` with score 0.6 (keyword-only). Not folded; carry-forward to Phase 46+ Linux-host phase.
