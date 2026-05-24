---
status: passed
phase: 36-upst3-deep-closure
source: [36-VERIFICATION.md]
started: 2026-05-23T23:30:00Z
updated: 2026-05-24T00:20:00Z
closed: 2026-05-24
scenarios: 7
result: 5/7 pass + 2/7 no-test-fixture (waived per 46-03-SUMMARY)
recording_location: "Phase 46 Plan 46-03 (.github/workflows/phase-46-uat-backlog.yml run-id 26347039444) + per-item waiver rationale in 46-03-SUMMARY.md"
backfilled_in: phase-46-plan-46-03
backfill_rationale: "v2.4 close left 36-HUMAN-UAT.md absent (human_needed deferred to v2.6 native host per memory project_v26_opened); Phase 46 Plan 46-03 backfills with verdicts from phase-46-uat-backlog.yml CI runs (run-id 26347039444: both jobs success) + no-test-fixture waivers per D-46-C3."
---

## Current Test

[all tests complete — backfilled at Phase 46 close, verdicts updated after workflow fix + re-dispatch (run-id 26347039444)]

## Tests

### 1. docs MDX bypass_protection render (REQ-PORT-CLOSURE-02)
expected: Documentation pages that reference `bypass_protection` render correctly in MDX without broken link or schema mismatch; `override_deny` alias is documented as deprecated. Host-agnostic verification — can run on Windows. Source: 36-01c-OVERRIDE-DENY-RENAME-SUMMARY.md + 36-01d-PROFILE-DATA-DOCS-TOOLING-SUMMARY.md; v2.4-MILESTONE-AUDIT rows 116-121 confirm this item passed at v2.4 close.
result: pass (pre-passed v2.4 per v2.4-MILESTONE-AUDIT.md rows 116-121 — "1 of 4 human-verify items is runnable on this Windows host (docs MDX render is host-agnostic)")

### 2. deprecated_schema --strict mode integration (REQ-PORT-CLOSURE-02)
expected: `nono profile validate --strict profile-with-legacy-key.yaml` exits non-zero with a clear error message referencing the legacy key. `profile_validate_strict` integration tests pass (`cargo test -p nono-cli --test profile_validate_strict`). Source: 36-01a-DEPRECATED-SCHEMA-MODULE-SUMMARY.md.
result: pass — run-id 26347039444 Linux step 10 ("Run Phase 36 UAT — deprecated_schema --strict mode (REQ-PORT-CLOSURE-02)"): conclusion=success; macOS step 8: conclusion=success

### 3. DeprecationCounter one-shot stderr WARN (REQ-PORT-CLOSURE-02)
expected: Loading a profile with legacy `override_deny` key emits a deprecation warning to stderr ONCE per process run (not on every invocation due to `AtomicBool` one-shot gate). Interactive test: run `nono run --profile legacy-profile.yaml -- true` twice consecutively; warning appears exactly once in first run. Source: 36-01a-DEPRECATED-SCHEMA-MODULE-SUMMARY.md.
result: no-test-fixture (waived per 46-03-SUMMARY § Item 3 Phase 36 — DeprecationCounter interactive WARN) — requires interactive host with legacy profile fixture; no headless automation surface for interactive stderr observation

### 4. LegacyPolicyPatch + canonical section serde round-trip (REQ-PORT-CLOSURE-02)
expected: A profile YAML with `override_deny: [path]` deserializes correctly via `LegacyPolicyPatch` and rewrites to `bypass_protection: [path]`; `CommandsConfig` section with `allow: [cmd]` and `deny: [cmd]` round-trips via serde without loss; `#[serde(deny_unknown_fields)]` rejects unknown keys. Source: 36-01a + 36-01b + 36-01c-SUMMARY.md.
result: no-test-fixture (waived per 46-03-SUMMARY § Item 4 Phase 36 — LegacyPolicyPatch serde round-trip) — no dedicated round-trip test in the CI-run fixture; structural correctness confirmed via 36-01a/b/c-SUMMARY.md artifacts at v2.4 close

### 5. yaml_merge wiring — `nono profile patch --yaml` (REQ-PORT-CLOSURE-04)
expected: `nono profile patch --yaml overlay.yaml` applies `YamlMergeDirective` to a target profile; the patched profile is written atomically (temp-file + rename); `yaml_merge_reversal` integration tests pass (`cargo test -p nono-cli --test yaml_merge_reversal`). Source: 36-02-WIRING-YAML-MERGE-SUMMARY.md.
result: pass — run-id 26347039444 Linux step 11 ("Run Phase 36 UAT — yaml_merge wiring reversal tests (REQ-PORT-CLOSURE-04)"): conclusion=success; macOS step 9: conclusion=success

### 6. yaml_merge path traversal rejection (REQ-PORT-CLOSURE-04)
expected: `validate_target_path` in `wiring.rs` rejects `../` path components via `Path::components()` iteration (NOT `str::starts_with` per CLAUDE.md footgun #1); `yaml_merge_reversal` integration test `validate_path_within` passes. Source: 36-02-WIRING-YAML-MERGE-SUMMARY.md T-36-02-DENY-UNKNOWN-FIELDS.
result: pass — same cargo invocation as Item 5 (`yaml_merge_reversal` includes traversal rejection tests); run-id 26347039444 Linux step 11 + macOS step 9: both success

### 7. ExecConfig surgical port + escape-aware diagnostic parser (REQ-PORT-CLOSURE-05)
expected: After Phase 36-03 port of upstream b5f0a3ab + bbdf7b85: (a) `startup_prompt` terminates automatically without interactive Y/N prompt matching upstream intent; (b) `sandbox_log.finish()` / `finish_realtime_only()` / `finish_inner()` split works correctly; (c) escape-aware diagnostic parser (`bbdf7b85`) correctly handles shell-escaped paths in diagnostic output. Source: 36-03-EXECCFG-SURGICAL-PORT-SUMMARY.md.
result: pass — run-id 26347039444 Linux step 12 ("Run Phase 36 UAT — ExecConfig surgical port unit tests (REQ-PORT-CLOSURE-05)"): conclusion=success (`cargo test -p nono --release`; unit tests for ExecConfig + diagnostic parser pass); macOS step 10: conclusion=success

## Summary

total: 7
passed: 5
issues: 0
pending: 0
skipped: 0
blocked: 0
no-test-fixture: 2

## Gaps

No goal-blocking gaps — REQ-UAT-BL-02 closed via Plan 46-03 with `5/7 pass + 2/7 no-test-fixture` per D-46-C3 explicit allowance (SC#5: "all items reach `pass` or carry a documented `no-test-fixture` waiver"). The 2 waivers reflect: 1 interactive stderr observation (Item 3/DeprecationCounter one-shot) and 1 design-only serde round-trip with no dedicated test fixture (Item 4). D-46-C3 target ≥5/7 pass is met exactly. Per-item rationale in 46-03-SUMMARY.md § No-Test-Fixture Waivers.
