---
phase: 49-sigstore-trust-root-poc-resilience-from-file-flag-release-as
verified: 2026-05-21T00:00:00Z
status: human_needed
score: 13/13 must-haves verified (3 PARTIAL deferrals routed to human for live-host/live-release closure)
overrides_applied: 0
re_verification:
  previous_status: none
  previous_score: n/a
  gaps_closed: []
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Cross-target clippy on x86_64-unknown-linux-gnu + x86_64-apple-darwin"
    expected: "Both targets exit 0 under `cargo clippy --workspace --target <T> -- -D warnings -D clippy::unwrap_used`"
    why_human: "Windows dev host lacks the cross-toolchains (x86_64-linux-gnu-gcc / cc cross-linker absent for native-build dependencies aws-lc-sys + ring). Documented PARTIAL per CLAUDE.md § Cross-target clippy verification rule + `.planning/templates/cross-target-verify-checklist.md`. Decisive signal is the post-merge live GH Actions Linux Clippy + macOS Clippy lanes on the head SHA."
  - test: "Live release-asset verification on the next tagged release (e.g., v2.6.0)"
    expected: "`gh release view <tag> --json assets | jq '.assets[].name'` lists `trusted_root.json`; `gh release download <tag> -p trusted_root.json && diff trusted_root.json crates/nono/tests/fixtures/trust-root-frozen.json` exits 0; `gh release download <tag> -p SHA256SUMS.txt && grep trusted_root.json SHA256SUMS.txt` exits 0"
    why_human: "REQ-POC-TRUST-02 acceptance criteria (d) and (e) intrinsically require a real tagged release. STRUCTURALLY-COMPLETE — CI gate is in place and will exit non-zero on byte-identity drift, so a tag-pushed release with this commit on the release branch cannot ship a non-byte-identical asset. Manual-Only per VALIDATION.md."
  - test: "Live positive `.ps1` smoke-script run on a Windows host with built nono.exe on PATH"
    expected: "`pwsh -NoProfile -File scripts/verify-trust-root-cached.ps1 crates/nono/tests/fixtures/trust-root-frozen.json` exits 0 and prints `PASS: ... cache is byte-identical (SHA-256: <hex>)`"
    why_human: "Scenario 1 of the .ps1 checkpoint (host-dependent positive path) requires a built `nono` binary on PATH with the merged --from-file flag. Scenarios 2 (param-validation early-exit -> 2) and 3 (nono-missing failure-propagation -> 1) already PASS live per 49-03-SUMMARY § Task 4. Manual-Only per VALIDATION.md."
---

# Phase 49: Sigstore Trust-Root POC Resilience — Verification Report

**Phase Goal:** POC users on Windows/Linux/macOS can populate `<nono_home>/.nono/trust-root/trusted_root.json` without depending on `sigstore_verify::TrustedRoot::production()` — by supplying a `--from-file` path that points at a known-good JSON (released as a sibling asset alongside the `nono` binary), with a documented maintainer cadence to keep the fork's frozen fixture fresh through Sigstore root rotations.

**Verified:** 2026-05-21
**Status:** human_needed (all observable truths VERIFIED against the codebase; 3 acknowledged Manual-Only/PARTIAL items routed to human for closure)
**Re-verification:** No — initial verification

## Goal Achievement

The Phase 49 goal is satisfied at the codebase level by three structurally-disjoint mechanisms:

1. **CLI surface:** `nono setup --from-file <PATH>` is implemented end-to-end in `crates/nono-cli/src/{cli,setup}.rs` with a clap-mutex against `--refresh-trust-root`, a fail-closed `load_trusted_root` + `check_trusted_root_freshness` validation pipeline, and a byte-identical `std::fs::copy` with best-effort cleanup on IO error.
2. **Release-asset bundling:** `.github/workflows/release.yml`'s `Generate checksums` step copies `crates/nono/tests/fixtures/trust-root-frozen.json` to `artifacts/trusted_root.json` with a `set -euo pipefail`-guarded SHA-256 byte-identity assert, appends the asset to `SHA256SUMS.txt`, and lists `artifacts/trusted_root.json` in the `softprops/action-gh-release` `files:` glob.
3. **Maintainer cadence + smoke gate + doc rewrite:** `.planning/templates/sigstore-rotation-refresh.md` ships the 6-step rotation procedure; `scripts/verify-trust-root-cached.{sh,ps1}` ship the cross-platform smoke gate; `docs/cli/development/windows-poc-handoff.mdx`'s `Known issue: Sigstore TUF root rotation` subsection is rewritten with `--from-file` as primary and the stale `sigstore-verify 0.6.5` / `P32-DEFER-005` / `deferred-items.md` references purged.

### Observable Truths

| #   | Truth                                                                                                                                                   | Status     | Evidence                                                                                                                                                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `nono setup --from-file <good>.json` validates the supplied JSON (schema + tlog `valid_for` freshness gate) and writes a byte-identical cache file       | VERIFIED   | `crates/nono-cli/src/setup.rs:888-936` implements `from_file_step` with two-stage validation (`load_trusted_root` -> `check_trusted_root_freshness`) before `std::fs::copy(src, &cache_path)`. Test `from_file_happy_path_writes_byte_identical_cache_and_stdout_matches_shape` (line 165) asserts both byte-identity and stdout shape. SUMMARY records `9 passed; 0 failed; 1 ignored`.            |
| 2   | `nono setup --from-file` never invokes `sigstore_verify::TrustedRoot::production()`                                                                     | VERIFIED   | `from_file_step` body (`setup.rs:888-936`) calls only `nono::trust::bundle::load_trusted_root` + `check_trusted_root_freshness` + `std::fs::copy`. The `production()` call site at `setup.rs:refresh_trust_root_step` is reached only via the `refresh_trust_root` branch, which is mutually-exclusive with `--from-file` per clap (`cli.rs:2382`).                                                  |
| 3   | `--from-file` and `--refresh-trust-root` are mutually exclusive at clap-parse time                                                                       | VERIFIED   | `crates/nono-cli/src/cli.rs:2382` declares `conflicts_with = "refresh_trust_root"` on the `from_file` arg. Test `from_file_with_refresh_rejected_by_clap` (line 381) asserts non-zero exit + `cannot be used with` stderr substring + cache absence after the clap-parse rejection.                                                                                                                  |
| 4   | Fail-closed on invalid JSON / schema mismatch / all-tlog-keys-expired / missing path; no partial cache file is written on any failure path              | VERIFIED   | Tests `from_file_expired_fails_closed` (line 257), `from_file_malformed_truncated_fails_closed` (line 294), `from_file_malformed_quote_flipped_fails_closed` (line 328), `from_file_missing_path_no_partial_cache` (line 353) all assert `!status.success()` + `!cache_path.exists()`. D-49-B2 best-effort cleanup at `setup.rs:924-927`.                                                            |
| 5   | After successful `--from-file`, `nono trust verify` runs offline (D-32-15 verify-is-offline invariant preserved)                                          | VERIFIED   | No new code paths in `crates/nono` — only a single-keyword `fn` -> `pub fn` visibility widen at `crates/nono/src/trust/bundle.rs:251` (existing internal callsite preserved). Verify-path lookup unchanged (`load_production_trusted_root` reads cache; no network on this path). Cross-phase invariant from Phase 32 not modified.                                                                  |
| 6   | Every GitHub Release publishes `trusted_root.json` as a sibling asset, byte-identical to `crates/nono/tests/fixtures/trust-root-frozen.json`            | VERIFIED (structural; live-release pending) | `.github/workflows/release.yml:308-348` `Generate checksums` step contains the `cp` + SHA-256 assert + ERROR exit gate + conditional `sha256sum trusted_root.json >> SHA256SUMS.txt`. `files:` glob at line 362 lists `artifacts/trusted_root.json`. Source SHA matches per dry-run: `6494e21ea73fa7ee769f85f57d5a3e6a08725eae1e38c755fc3517c9e6bc0b66`. Live verification routed to human. |
| 7   | Release CI step uses `set -euo pipefail` (no silent-pass on internal pipe failures)                                                                      | VERIFIED   | `.github/workflows/release.yml:310` — `set -euo pipefail` at the top of the `Generate checksums` step. F-02-04 mitigation confirmed.                                                                                                                                                                                                                                                              |
| 8   | `SHA256SUMS.txt` includes a `trusted_root.json` line                                                                                                    | VERIFIED   | `.github/workflows/release.yml:345-347` conditional `sha256sum trusted_root.json >> SHA256SUMS.txt`. The conditional mirrors the existing `*.zip` / `*.msi` / `*.exe` pattern; pattern verified locally via positive + negative dry-run per 49-02-SUMMARY.                                                                                                                                            |
| 9   | `.planning/templates/sigstore-rotation-refresh.md` documents the 6-step maintainer-cadence (trigger / capture / diff / regression / smoke gate / commit + release-asset forward-pointer) | VERIFIED   | File exists at `.planning/templates/sigstore-rotation-refresh.md` (94 lines). 4 H2 sections (Scope / Decision Tree / Anti-Patterns / Enforcement). 6 `**Step N` entries verified. References `verify-trust-root-cached`, `release.yml`, `Signed-off-by`, and `P32-DEFER-005` (supersedes prose).                                                                                                       |
| 10  | `scripts/verify-trust-root-cached.sh` exists, is executable, exits 0 on the current frozen fixture                                                       | VERIFIED (positive self-test pending Plan 49-01-merged + on-PATH binary) | File exists, `git ls-files -s` mode `100755`, `bash -n` syntax-valid, contains `set -euo pipefail`, `trap.*rm -rf`, `cmp -s` byte-identity check. SUMMARY records positive self-test deferred to wave-close integration; live PowerShell scenarios 2+3 already PASS per 49-03-SUMMARY § Task 4.                                  |
| 11  | `scripts/verify-trust-root-cached.ps1` exists and propagates `$LASTEXITCODE` from every native command invocation (F-03-05 mitigation)                   | VERIFIED   | File exists (73 lines). Contains `$ErrorActionPreference = 'Stop'`, explicit `if ($LASTEXITCODE -ne 0) { throw ... }` after the `& nono setup` invocation (line 44), `Get-FileHash` byte-identity, `try/finally` cleanup. 49-03-SUMMARY records Rule 1 fix `f0b48684` that replaced `Write-Error` with `[Console]::Error.WriteLine` + explicit `exit N` to keep exit-code propagation correct. |
| 12  | `docs/cli/development/windows-poc-handoff.mdx` removes the stale `sigstore-verify 0.6.5`, `P32-DEFER-005`, and `deferred-items.md` cross-references         | VERIFIED   | `grep -E '(sigstore-verify 0\.6\.5\|P32-DEFER-005\|deferred-items\.md)' docs/cli/development/windows-poc-handoff.mdx` exits 1 (zero matches). F-03-03 negative-grep gate satisfied.                                                                                                                                                                                                                  |
| 13  | `docs/cli/development/windows-poc-handoff.mdx` `Known issue: Sigstore TUF root rotation` subsection recommends `nono setup --from-file` as the primary recovery path; the `Run once after install` block also mentions `--from-file` | VERIFIED   | `Known issue: Sigstore TUF root rotation` heading at line 188 has no version pin. Primary path block at line 208 promotes `nono setup --from-file` against the release-asset URL; fallback `Invoke-WebRequest` block at line 225 references `.planning/templates/sigstore-rotation-refresh.md`. `Run once after install` (line 166) mentions `--from-file` within 8 lines. F-03-04 satisfied.       |

**Score:** 13/13 truths verified (3 of these carry routine human-finalization steps recorded under § Human Verification Required and routed to the human, not blockers)

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/nono/src/trust/bundle.rs` | `pub fn check_trusted_root_freshness` widened from private | VERIFIED | Line 251 contains `pub fn check_trusted_root_freshness(`; body byte-identical to pre-edit per 49-01-SUMMARY § Deviations 1. |
| `crates/nono/src/trust/mod.rs` | `check_trusted_root_freshness` re-exported | VERIFIED | Line 40 includes `check_trusted_root_freshness` in `pub use bundle::{...}`. |
| `crates/nono-cli/src/cli.rs` | `SetupArgs::from_file` clap field with `conflicts_with` | VERIFIED | Lines 2370-2384 add `pub from_file: Option<PathBuf>` with `conflicts_with = "refresh_trust_root"`. |
| `crates/nono-cli/src/setup.rs` | `SetupRunner::from_file_step` impl + struct field + `run()` branch + shared phase-index slot | VERIFIED | `from_file: Option<std::path::PathBuf>` field (line 25), `from_args` wiring (line 45), `run()` branch (lines 98-99), `from_file_step` impl (lines 888-936), shared phase-index sites (lines 697, 727, 731, 748, 752). |
| `crates/nono-cli/tests/setup_trust_root.rs` | Integration tests for --from-file (happy/expired/malformed/missing/mutex/stdout/phase-index) | VERIFIED | 480 lines total. `grep -c "^#\[test\]"` returns 10 (3 existing + 7 new). All 7 `fn from_file_*` test fns present (lines 165, 220, 257, 294, 328, 353, 381). |
| `.github/workflows/release.yml` | Byte-identity assert + SHA256SUMS extension + files-glob entry | VERIFIED | `set -euo pipefail` at line 310; byte-identity block at 322-333; conditional `sha256sum trusted_root.json` at 345-347; `artifacts/trusted_root.json` in `files:` glob at line 362. YAML loads cleanly. |
| `.planning/templates/sigstore-rotation-refresh.md` | 6-section maintainer-cadence template | VERIFIED | File exists (94 lines, > 60 required). 4 H2 sections + 6 `Step N` entries. References smoke-script + release.yml + DCO. 5 Anti-Patterns. |
| `scripts/verify-trust-root-cached.sh` | Bash smoke script with `set -euo pipefail` + cleanup trap | VERIFIED | 49 lines (> 15 required). Mode `100755`. `bash -n` clean. Contains all required idioms (`set -euo pipefail`, `trap rm -rf`, `cmp -s`). |
| `scripts/verify-trust-root-cached.ps1` | PowerShell smoke script with `$LASTEXITCODE` checks + `[Console]::Error.WriteLine` exit-code propagation | VERIFIED | 73 lines (> 15 required). `$ErrorActionPreference = 'Stop'`, explicit `$LASTEXITCODE` check after `& nono setup`, `Get-FileHash` byte-identity, `try/finally` cleanup. Rule 1 fix shipped per 49-03-SUMMARY. |
| `docs/cli/development/windows-poc-handoff.mdx` | Rewritten `Known issue` subsection with `--from-file` primary + stale refs purged | VERIFIED | Lines 166-244 contain rewritten `Run once after install` + `Known issue: Sigstore TUF root rotation` blocks. Zero matches for the three stale strings; positive `--from-file` and `sigstore-rotation-refresh` cross-references present. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `crates/nono-cli/src/setup.rs::from_file_step` | `crates/nono/src/trust/bundle.rs::load_trusted_root` | validation pipeline call | WIRED | `setup.rs:902` calls `nono::trust::bundle::load_trusted_root(src)` and propagates errors via `NonoError::Setup` mapping. |
| `crates/nono-cli/src/setup.rs::from_file_step` | `crates/nono/src/trust/bundle.rs::check_trusted_root_freshness` | freshness gate call | WIRED | `setup.rs:913` calls `nono::trust::bundle::check_trusted_root_freshness(&trusted_root, &cache_path)` AFTER the parse step but BEFORE the copy. |
| `crates/nono-cli/src/setup.rs::from_file_step` | `std::fs::copy` | byte-identical write (D-49-B1) | WIRED | `setup.rs:924` calls `std::fs::copy(src, &cache_path)` with best-effort cleanup on Err at `setup.rs:925`. |
| `.github/workflows/release.yml::Generate checksums` | `crates/nono/tests/fixtures/trust-root-frozen.json` | `cp` + sha256sum byte-identity assert | WIRED | `release.yml:322-333` performs `cp $SRC $DST` + SHA-256 equality check + non-zero exit on drift. |
| `.github/workflows/release.yml::Create GitHub Release` | `artifacts/trusted_root.json` | `softprops/action-gh-release` files glob | WIRED | `release.yml:362` lists `artifacts/trusted_root.json` (repo-root-relative for the action's glob context). |
| `.planning/templates/sigstore-rotation-refresh.md` | `scripts/verify-trust-root-cached.sh/.ps1` | pre-commit smoke gate (Step 4) | WIRED | Template Step 4 (lines 48-58) names both `.sh` and `.ps1` invocations as the pre-commit gate. |
| `.planning/templates/sigstore-rotation-refresh.md` | `.github/workflows/release.yml` | release-asset gate forward-pointer (Step 6) | WIRED | Template Step 6 (lines 69-77) references the byte-identity assert step from Plan 49-02. |
| `docs/cli/development/windows-poc-handoff.mdx` | `.planning/templates/sigstore-rotation-refresh.md` | inline comment in Invoke-WebRequest fallback block | WIRED | Line 234 references the template path. |

### Data-Flow Trace (Level 4)

Level 4 traces dynamic-data paths. Phase 49 does not produce dashboards or rendering components, but the file-data flow is the corresponding artifact:

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `from_file_step` cache write | `cache_path` content | `std::fs::copy(src, &cache_path)` where `src` was just parsed + freshness-checked | YES — bytes flow from the user-supplied `<PATH>` through validated-in-memory `TrustedRoot` and finally written verbatim | FLOWING |
| `release.yml` `Generate checksums` step | `trusted_root.json` artifact | `cp $SRC $DST` where SRC = `crates/nono/tests/fixtures/trust-root-frozen.json` | YES — the 6.6 KB JSON flows verbatim into `artifacts/`; SHA-256 byte-identity asserted | FLOWING |
| Release files glob | `artifacts/trusted_root.json` | `softprops/action-gh-release` files: list | YES — asset will land on the GitHub Release (live verification pending) | FLOWING (structural) |

(Note: WR-01 in 49-REVIEW.md flags a theoretical TOCTOU between validation and copy. The reviewer correctly classified this as advisory WARNING, not BLOCKER — the threat model is maintainer-as-attacker. Not a goal-blocking gap; routed to follow-up not to verification.)

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Cargo workspace builds | `cargo build --workspace` (post-merge per orchestrator context) | exit 0 (260 s) | PASS |
| Cargo workspace tests | `cargo test --workspace` (post-merge per orchestrator context) | 171 tests, exit 0 | PASS |
| `setup_trust_root.rs` integration suite | `cargo test -p nono-cli --test setup_trust_root` | 9 passed, 0 failed, 1 ignored per 49-01-SUMMARY | PASS |
| YAML release.yml load | `python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` | `YAML valid` | PASS |
| `bash -n` smoke script | `bash -n scripts/verify-trust-root-cached.sh` | exit 0 | PASS |
| Smoke script Scenario 1 (live happy-path) | `bash scripts/verify-trust-root-cached.sh crates/nono/tests/fixtures/trust-root-frozen.json` (orchestrator post-merge) | exit 0 | PASS |
| Frozen fixture SHA-256 | `sha256sum crates/nono/tests/fixtures/trust-root-frozen.json` | `6494e21ea73fa7ee769f85f57d5a3e6a08725eae1e38c755fc3517c9e6bc0b66` (matches 49-02-SUMMARY) | PASS |
| Stale-string negative grep | `grep -E '(sigstore-verify 0\.6\.5\|P32-DEFER-005\|deferred-items\.md)' docs/cli/development/windows-poc-handoff.mdx` | exit 1 (zero matches) | PASS |
| `--from-file` positive grep near `Run once after install` | `grep -A 8 'Run once after install' ... \| grep -c -- '--from-file'` | 2 (>=1 required) | PASS |
| `nono setup --from-file` positive grep in `Known issue` block | `grep -A 30 '#### Known issue: Sigstore TUF root rotation' ... \| grep -c 'nono setup --from-file'` | 2 (>=1 required) | PASS |
| Forward pointer to template | `grep -c 'sigstore-rotation-refresh' docs/cli/development/windows-poc-handoff.mdx` | 1 | PASS |
| Cross-target clippy on Linux + macOS | `cargo clippy --workspace --target {x86_64-unknown-linux-gnu,x86_64-apple-darwin} -- -D warnings -D clippy::unwrap_used` | SKIPPED (cross-toolchain absent per 49-01-SUMMARY) | SKIP — routed to human (live CI) |
| Live release-asset visibility | `gh release view <tag> --json assets` | N/A — no v2.6.x tag yet | SKIP — routed to human (Manual-Only) |
| Live `.ps1` Scenario 1 with `nono` on PATH | `pwsh -NoProfile -File scripts/verify-trust-root-cached.ps1 ...` | N/A — Plan 49-01 binary not present on this worktree's PATH | SKIP — routed to human (Manual-Only) |

### Requirements Coverage

| Requirement       | Source Plan | Description                                                                                                                              | Status      | Evidence |
| ----------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ----------- | -------- |
| REQ-POC-TRUST-01  | 49-01       | `--from-file` flag with fail-closed validation pipeline (parse + freshness), byte-identical copy, clap-mutex with `--refresh-trust-root` | SATISFIED   | Truths 1-5 above; F-01-01..F-01-05 + F-01-07 + F-01-08 covered by integration tests; F-01-06 cross-target clippy PARTIAL deferred to live CI. |
| REQ-POC-TRUST-02  | 49-02       | `trusted_root.json` shipped as release asset with CI-asserted SHA-256 byte-identity gate + SHA256SUMS coverage + files-glob entry        | SATISFIED (structural; live-release pending) | Truths 6-8 above; F-02-01..F-02-05 covered structurally per 49-02-SUMMARY; acceptance criteria (d)/(e) intrinsically Manual-Only per VALIDATION.md. |
| REQ-POC-TRUST-03  | 49-03       | Maintainer-cadence template + cross-platform smoke scripts + POC-handoff doc rewrite                                                     | SATISFIED   | Truths 9-13 above; F-03-01..F-03-05 covered (live Windows-host Scenarios 2 + 3 PASS per 49-03-SUMMARY § Task 4; Scenario 1 deferred to post-merge wave-close integration). |

**Note on REQUIREMENTS.md coverage:** REQUIREMENTS.md last-updated 2026-05-20 (Phase 49 added 2026-05-21) does NOT yet list REQ-POC-TRUST-01/02/03 — but `ROADMAP.md` lines 142, 193-195 explicitly mark these as "(anticipated) Phase 49 POC-TRUST" with the note: "Phase 49 requirement IDs will be formalized at spec-phase time." The IDs were locked in `49-SPEC.md` on 2026-05-21 and the v1 requirement table in REQUIREMENTS.md is one revision behind. This is an inventory-traceability tidy-up, not a goal-coverage gap; flagged as INFO.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| n/a  | n/a  | n/a     | n/a      | No `TODO`/`FIXME`/`PLACEHOLDER`/stub patterns introduced. No `.unwrap()` / `.expect()` in production code paths (`#[cfg(test)]` test helpers exempt per CLAUDE.md). `#[allow(clippy::wrong_self_convention)]` on `from_file_step` is justified inline (mirrors `refresh_trust_root_step` phase-step pattern; `&self` required for `total_phases()`). |

49-REVIEW.md identified 7 WARNING + 7 INFO findings. Per the orchestrator's verification background, these are advisory and do NOT block verification:

- **WR-01 (TOCTOU read-twice between validation and copy):** Acknowledged. Maintainer-as-attacker threat model; doc comment slightly overclaims "verbatim" under concurrent mutation. Hardening fix (read once, write atomically via temp + rename) recorded as a follow-up enhancement, not a Phase 49 closure gap.
- **WR-02 (no release-asset freshness gate):** Acknowledged. CI byte-identity asserts source-equals-dest but does not check that the source fixture is itself fresh. Mitigated structurally by the maintainer-cadence template (Plan 49-03) which mandates the smoke gate at commit time; recorded as a CI-gate-enhancement follow-up.
- **WR-03 (PowerShell env-var leak):** Acknowledged. The `.ps1` smoke script sets `$env:NONO_TEST_HOME` / `$env:XDG_CONFIG_HOME` / `$env:NONO_NO_UPDATE_CHECK` but does not restore prior values. Maintainer-only script; impact limited to in-process invocation patterns. Follow-up fix (capture + restore in `finally`) recorded.
- **WR-04 (GHA template injection via `${{ env.RELEASE_TAG }}` in `run:` blocks):** Pre-existing posture across `release.yml`; Phase 49 did not add new injection sites. Hardening (`env:` mapping + shell-variable usage) recorded as a workflow-wide follow-up.
- **WR-05 (`refresh_trust_root_phase_index` does not include `from_file.is_some()`):** Acknowledged. The function name implies single-flag semantics; the shared-slot invariant is correct at runtime because clap-mutex enforces mutual exclusion. Rename to `trust_root_phase_index` + doc-comment of the shared-slot invariant recorded as a clarity follow-up.
- **WR-06 (doc-comment overclaims `nono trust verify` pipeline equivalence):** Acknowledged. `--from-file` validates with the trust-root-load subpipeline, not the full verify-with-Fulcio/Rekor chain. Doc-comment rephrasing follow-up recorded.
- **WR-07 (Invoke-WebRequest fallback bypasses `--from-file` validation):** Acknowledged. Fallback path's threat coverage narrower than docs imply. Rewrite (Invoke-WebRequest -> temp -> `--from-file <temp>`) recorded as a doc follow-up.

These 7 WARNINGs collectively constitute a "Phase 49 v2 hardening pass" surface for a follow-on phase; they do not invalidate the goal-achievement of v1. The 7 INFO findings are routine commentary and recorded in 49-REVIEW.md.

### Human Verification Required

Three items require human/live-environment closure that the verifier cannot execute from the codebase:

1. **Cross-target clippy on Linux + macOS targets**

   **Test:** `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`
   **Expected:** Both exit 0.
   **Why human:** Windows dev host lacks the cross-toolchains for native-build dependencies (`aws-lc-sys`, `ring`). Documented PARTIAL per CLAUDE.md § Cross-target clippy verification + `.planning/templates/cross-target-verify-checklist.md § PARTIAL Disposition`. Decisive signal lives in the post-merge live GH Actions Linux Clippy + macOS Clippy lanes on the head SHA. Per the orchestrator background: "Cross-target clippy (Linux/macOS) is marked PARTIAL by 49-01 per CLAUDE.md § Cross-target clippy verification — Windows host cannot run the cross-toolchain; deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`. This is the documented protocol, not a verification gap."

2. **Live release-asset verification on the next tagged release (e.g., v2.6.0)**

   **Test:** After the next tag push, run:
   ```bash
   gh release view <tag> --json assets | jq '.assets[].name'   # must list trusted_root.json
   gh release download <tag> -p trusted_root.json
   diff trusted_root.json crates/nono/tests/fixtures/trust-root-frozen.json   # must exit 0
   gh release download <tag> -p SHA256SUMS.txt
   grep trusted_root.json SHA256SUMS.txt                       # must exit 0
   ```
   **Expected:** Asset present in release; downloaded asset byte-identical to source fixture; SHA256SUMS.txt covers the asset.
   **Why human:** REQ-POC-TRUST-02 acceptance criteria (d) and (e) intrinsically require a real tagged release. Per VALIDATION.md § Manual-Only and the orchestrator's verification background: "49-02 acceptance criteria (d)/(e) require an actual tagged release for live verification — intrinsically deferred to next release per VALIDATION.md § Manual-Only." STRUCTURALLY-COMPLETE: the CI gate exits non-zero on byte-identity drift, so a tag-pushed release with this commit on the release branch cannot ship a non-byte-identical asset.

3. **Live `.ps1` smoke-script Scenario 1 on Windows host with built `nono.exe` on PATH**

   **Test:** `pwsh -NoProfile -File scripts/verify-trust-root-cached.ps1 crates/nono/tests/fixtures/trust-root-frozen.json`
   **Expected:** Exit 0 + `PASS: ... cache is byte-identical (SHA-256: <hex>)`.
   **Why human:** Scenario 1 (positive-path with the merged `--from-file` flag) requires a built `nono` binary on PATH. Scenarios 2 (param-validation early-exit -> exit 2) and 3 (nono-missing failure-propagation -> exit 1) already PASS per 49-03-SUMMARY § Task 4 Live Verification. Per the orchestrator background: scenario 1 was implicitly handled by the post-merge run of `bash scripts/verify-trust-root-cached.sh crates/nono/tests/fixtures/trust-root-frozen.json` which exited 0 — the `.sh` Scenario 1 is the equivalent positive path on this Windows-host-with-Git-Bash; the `.ps1` positive path remains a routine on a fresh Windows POC host.

### Gaps Summary

**No codebase-level goal-blocking gaps.** All 13 observable truths are satisfied by code that exists, is wired, and produces real data through the validated pipeline. All 10 required artifacts exist with the expected substance and wiring. All 8 key links are WIRED. Behavioral spot-checks all PASS (or are routinely SKIPPED with explicit human-routing).

Three items are routed to the human for closure under the documented Manual-Only / PARTIAL protocol:

1. Cross-target clippy on Linux + macOS — protocol-documented deferral to live CI.
2. Live release-asset visibility post-tag — intrinsic to release lifecycle.
3. Live `.ps1` Scenario 1 with on-PATH `nono` binary — routine post-merge integration check.

These are NOT failures of the codebase; they are environmental closures the verifier cannot perform from the dev host. Per the goal-backward verification mindset, the phase goal IS achieved at the codebase: POC users can run `nono setup --from-file <PATH>` today against any release-asset `trusted_root.json` and unblock their verify path without depending on `TrustedRoot::production()`. The release-asset and cross-target-clippy items are downstream of that achievement.

### Re-verification Notes

This is the initial verification of Phase 49. No previous VERIFICATION.md exists.

---

_Verified: 2026-05-21_
_Verifier: Claude (gsd-verifier)_
