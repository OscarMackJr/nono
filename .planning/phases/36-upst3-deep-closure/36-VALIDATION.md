---
phase: 36
slug: upst3-deep-closure
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-12
---

# Phase 36 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Drawn from `36-RESEARCH.md` § Validation Architecture and D-36-A5 close-gate.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust stable 1.77+ workspace) |
| **Config file** | `Cargo.toml` (workspace) + per-crate `Cargo.toml` |
| **Quick run command** | `cargo test -p nono-cli --lib <touched_module>::tests` |
| **Full suite command** | `cargo test --workspace --all-features` |
| **Estimated runtime** | ~3-4 min quick (per crate lib); ~12-15 min full workspace on Windows host |

Secondary gates (D-36-A5 close-gate steps):

| Gate | Command |
|------|---------|
| Windows host clippy | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` |
| Cross-target Linux clippy | `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` |
| Cross-target macOS clippy | `cargo clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` |
| Format check | `cargo fmt --all -- --check` |
| Detached-console smoke (Phase 15) | `nono run --detached` → `nono ps` → `nono attach` → detach → `nono stop` |
| WFP port integration | `cargo test -p nono-cli --test wfp_port_integration` (or documented-skipped) |
| Learn Windows integration | `cargo test -p nono-cli --test learn_windows_integration` (or documented-skipped) |

---

## Sampling Rate

- **After every task commit:** Run quick command for the touched module (per-plan local test surface).
- **After every plan wave:** Run full workspace `cargo test --workspace --all-features` plus all 3 clippy targets.
- **Before `/gsd-verify-work`:** All 8 D-36-A5 close-gate steps must be green.
- **Max feedback latency:** ~60s for unit-level quick runs; full close-gate ~25 min on Windows host.

---

## Per-Task Verification Map

> Rows below are the locked invariants from `36-RESEARCH.md` § Validation Architecture. Plan IDs are bound at PLAN.md authoring time; task IDs (`{plan}-{task}`) populate once plans exist.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 36-01a-* | 01a | 1 | REQ-PORT-CLOSURE-02 | T-36-01-LEGACY-KEY | `LegacyPolicyPatch` rewriter accepts legacy `override_deny` JSON, emits per-key `DeprecationCounter` warning once-per-process, rewrites to canonical `bypass_protection` | unit | `cargo test -p nono-cli --lib deprecated_schema::tests` | ❌ W0 | ⬜ pending |
| 36-01a-* | 01a | 1 | REQ-PORT-CLOSURE-02 | T-36-01-STRICT-MODE | `nono profile validate --strict` exits non-zero on legacy keys with clear canonical-key error pointer | integration | `cargo test -p nono-cli --test profile_validate_strict` | ❌ W0 | ⬜ pending |
| 36-01b-* | 01b | 2 | REQ-PORT-CLOSURE-02 | T-36-01-CANONICAL | Profile/LoadedProfile structs expose `groups`, `commands.{allow,deny}`, `filesystem.{deny,bypass_protection}` and round-trip JSON through `profile_to_json` Map shape from Plan 35-03 | unit | `cargo test -p nono-cli --lib profile::tests` | ✅ | ⬜ pending |
| 36-01c-* | 01c | 2 | REQ-PORT-CLOSURE-02 | T-36-01-RENAME-ATOMIC | 183-callsite atomic rename across 17 files (corrected from CONTEXT.md's 210/14+); single commit; clippy + fmt + workspace test all green at the commit boundary | gate | `cargo build --workspace --all-features && cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used && cargo test --workspace --all-features` | ✅ | ⬜ pending |
| 36-01d-* | 01d | 2 | REQ-PORT-CLOSURE-02 | T-36-01-DATA-MIGRATE | Built-in profiles (claude-code, codex, opencode, claude-no-keychain) load post-migration with no schema-validation errors; `scripts/test-list-aliases.sh` exits 0; `scripts/regenerate-schema.sh` output matches `nono-profile.schema.json` | integration | `cargo test -p nono-cli --test builtin_profile_load && bash scripts/test-list-aliases.sh` | ❌ W0 (test-list-aliases.sh new) | ⬜ pending |
| 36-02-* | 02 | 1 | REQ-PORT-CLOSURE-04 | T-36-02-YAML-MERGE | `nono profile patch --yaml <overlay>` accepts `yaml_merge:` directives, applies them, and the reversal-failure test from upstream `242d4917` passes | unit + integration | `cargo test -p nono-cli --lib wiring::tests && cargo test -p nono-cli --test yaml_merge_reversal` | ❌ W0 (wiring.rs new) | ⬜ pending |
| 36-02-* | 02 | 1 | REQ-PORT-CLOSURE-04 | T-36-02-PATH-VALIDATE | `yaml_merge` target path validation uses path-component comparison + canonicalization (CLAUDE.md § Path Handling); no string `starts_with` regression | unit | `cargo test -p nono-cli --lib wiring::path_validation_tests` | ❌ W0 | ⬜ pending |
| 36-03-c1-* | 03 (Commit 1) | 1 | REQ-PORT-CLOSURE-05 | T-36-03-DIAG-HELPERS | 4 helpers (`extract_path_after_syscall_word`, `infer_access_from_structured_syscall_line`, `extract_structured_path_property`, `extract_structured_string_property`) restored + wired into `analyze_error_output`; `test_analyze_error_output_detects_node_eperm_mkdir_as_write` passes | unit | `cargo test -p nono diagnostic::tests::test_analyze_error_output_detects_node_eperm_mkdir_as_write` | ❌ W0 | ⬜ pending |
| 36-03-c2-* | 03 (Commit 2) | 1 | REQ-PORT-CLOSURE-05 | T-36-03-EXEC-HELPERS | `should_offer_profile_save()`, `should_apply_startup_timeout()`, `startup_timeout_profile()`, `compute_executable_identity()` exist + new `clear_signal_forwarding_target()` callsite before profile-save prompt; `LearnArgs.trace` field restored | unit | `cargo test -p nono-cli --lib exec_strategy::tests && cargo test -p nono-cli --lib execution_runtime::tests` | ❌ W0 (additive) | ⬜ pending |
| 36-03-c2-* | 03 (Commit 2) | 1 | REQ-PORT-CLOSURE-05 | T-36-03-PTY-QUIET | `POST_EXIT_PTY_DRAIN_TIMEOUT` constant = 100ms; Phase 17 attach-streaming tests still green; Phase 31 broker ConPTY 5-row smoke gate still green | regression | `cargo test -p nono-cli --test attach_streaming_integration && {Phase-15 5-row smoke}` | ✅ | ⬜ pending |
| 36-03-c3-* | 03 (Commit 3) | 1 | REQ-PORT-CLOSURE-05 | T-36-03-ESCAPE-QUOTE | `extract_structured_string_property` handles escape-quoted characters; both new tests pass; D-19 trailer present (smoke check: `git log --format='%B' main~3..main \| grep -c '^Upstream-commit: '` equals exactly 1) | unit + provenance | `cargo test -p nono diagnostic::tests::test_analyze_error_output_detects_structured_node_eperm_mkdir_path && cargo test -p nono diagnostic::tests::test_analyze_error_output_detects_structured_path_with_escaped_quote` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono-cli/src/deprecated_schema.rs` — new module (~824 LOC port); contains `LegacyPolicyPatch` + `DeprecationCounter` + unit-test `mod tests`
- [ ] `crates/nono-cli/tests/profile_validate_strict.rs` — new integration test for `--strict` fail-closed behavior
- [ ] `crates/nono-cli/src/wiring.rs` — new module (~300-400 LOC stripped-down port); contains yaml_merge parser/applier + unit-test `mod tests` + `mod path_validation_tests`
- [ ] `crates/nono-cli/tests/yaml_merge_reversal.rs` — new integration test from upstream `242d4917` reversal failure scenario
- [ ] `crates/nono-cli/tests/builtin_profile_load.rs` — new integration test exercising claude-code/codex/opencode/claude-no-keychain post-migration
- [ ] `scripts/test-list-aliases.sh` — new alias-inventory enforcement script (Plan 36-01d)
- [ ] `scripts/lint-docs.sh` — new docs alias-inventory check (Plan 36-01d)

*Plan 36-03 adds inline unit tests inside `crates/nono/src/diagnostic.rs` and helper-coverage tests inside `crates/nono-cli/src/execution_runtime.rs` — no new test files required.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 5-row detached-console smoke (Phase 15) | REQ-PORT-CLOSURE-05 (D-36-A5 close-gate step 6) | Requires Windows console host with TTY allocation; non-headless | 1. `nono run --detached -- <test command>` 2. `nono ps` shows session 3. `nono attach <id>` enters PTY 4. Detach with Ctrl-A D 5. `nono stop <id>` cleans up |
| PTY-quiet-period perception check | REQ-PORT-CLOSURE-05 acceptance #3 | Real-world workload feel; numbers can pass while UX regresses | Run `nono run -- <interactive shell session>`, exit shell, observe terminal cursor returns promptly (<200ms perceived) |
| Built-in profile docs render | REQ-PORT-CLOSURE-02 (Plan 36-01d) | Docs render in MDX environment; CI workspace test can't render | Open `docs/cli/features/profiles-groups.mdx` + `docs/cli/usage/flags.mdx` in docs preview; verify canonical sections render and flag-deprecation table is accurate |
| Linux/macOS host execution of cross-platform plans | All Phase 36 plans | Phase 36 is host-blocked on Linux/macOS clippy + test execution; Windows host is the dev host per D-36-A6 | After Phase 36 merge, run full suite on Linux (Plan 25-01 host) + macOS host before Phase 37 starts |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (7 new files listed above)
- [ ] No watch-mode flags (`cargo test --watch` not used in CI; on-demand only)
- [ ] Feedback latency < 60s for per-task quick runs
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
