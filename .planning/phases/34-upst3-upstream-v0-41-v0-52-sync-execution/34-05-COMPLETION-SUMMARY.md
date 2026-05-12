---
phase: 34-upst3-upstream-v0-41-v0-52-sync-execution
plan: 05
slug: completion
cluster_id: C8
subsystem: cli
tags: [upst3, c8, completion, string-truncation, wave-2, shell-completion]
requires:
  - 34-02 (C4 proxy net hardening — cli.rs upstream chronological serialization)
  - 34-04 (C7 canonical JSON schema — profile.security.groups shape)
  - 34-04b (C7 follow-up — canonical schema rename runway)
  - 34-03 (C5 keyring + display — char-aware truncation precursor `d375b05e`)
  - 34-01 (C2 CLI consolidation — Commands enum stability)
provides:
  - "`nono completion <shell>` subcommand (bash/zsh/fish/powershell/elvish) via clap_complete"
  - "Generic `truncate_chars(&str, usize) -> String` UTF-8-safe truncation utility in command_display.rs"
  - "Sibling-extends self-reference skip in `load_base_profile_raw` via new `source_file: Option<&Path>` parameter"
  - "Truncation panic fix preventing slice-mid-codepoint panics on multi-byte UTF-8 input"
  - "Reduced `nono run` output verbosity (hidden ABI line on fully-supported kernels; hidden supervised line when only baseline features active; simplified 'Applying sandbox...' message)"
  - "Demoted `--allow-launch-services` log from warn → debug (macOS-side, security non-critical on Windows)"
  - "v0.48.0 CHANGELOG provenance row (version bumps NOT applied — fork tracks v2.3+ scheme)"
affects:
  - crates/nono-cli/src/cli.rs (Completions arg type + Windows ROOT_HELP_TEMPLATE SHELL section)
  - crates/nono-cli/src/cli_bootstrap.rs (Completions verbosity-default arm)
  - crates/nono-cli/src/app_runtime.rs (Completions dispatch)
  - crates/nono-cli/src/completions.rs (NEW — clap_complete shell-generator wrapper)
  - crates/nono-cli/src/main.rs (mod completions registration)
  - crates/nono-cli/src/command_display.rs (extracted truncate_chars + panic fix)
  - crates/nono-cli/src/rollback_commands.rs (truncate_str → truncate_chars migration)
  - crates/nono-cli/src/output.rs (verbosity simplification; dropped finish_status_line_for_handoff)
  - crates/nono-cli/src/execution_runtime.rs (verbosity simplification)
  - crates/nono-cli/src/supervised_runtime.rs (verbosity simplification)
  - crates/nono-cli/src/sandbox_prepare.rs (log demotion)
  - crates/nono-cli/src/profile/mod.rs (sibling self-reference skip + test schema adapt)
  - crates/nono-cli/src/startup_runtime.rs (verbosity-default for Completions)
  - crates/nono-cli/Cargo.toml (clap_complete dependency)
  - CHANGELOG.md (v0.48.0 entry)
tech-stack:
  added:
    - "clap_complete v4.6.3 — shell completion script generator (bash/zsh/fish/powershell/elvish via clap_complete::Generator)"
  patterns:
    - "Surgical retrofit posture (D-34-B2) — ship cross-platform feature AS-IS; no Windows-specific composition"
    - "Partial-cherry-pick for release-tag commits (fork keeps v0.37.1; merges CHANGELOG only)"
    - "Fork-divergence test schema adapt — upstream tests assert `profile.groups.include`; fork uses `profile.security.groups`"
key-files:
  created:
    - crates/nono-cli/src/completions.rs
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/cli_bootstrap.rs
    - crates/nono-cli/src/app_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/command_display.rs
    - crates/nono-cli/src/rollback_commands.rs
    - crates/nono-cli/src/output.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/supervised_runtime.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/startup_runtime.rs
    - crates/nono-cli/Cargo.toml
    - Cargo.lock
    - CHANGELOG.md
decisions:
  - "D-34-B2 surgical posture honored: `nono completion` shipped verbatim from upstream; NO MSI installer integration; NO PowerShell `$PROFILE.d/` shim. Users on Windows run `nono completion powershell > $PROFILE.d/nono.ps1` manually."
  - "Release-tag partial-cherry-pick: per the v0.41.0/0.43.0/0.43.1/0.45.0/0.46.0/0.47.0/0.47.1 precedent on main, the `chore: release v0.48.0` cherry-pick keeps fork's version (0.37.1) and merges only the CHANGELOG entry for downstream sync provenance."
  - "Plan's frontmatter `string_truncation.rs` reference was inaccurate — upstream `7b71855c` extracts `truncate_chars` INTO `command_display.rs`, not into a new file. Fork follows upstream layout."
  - "Fork-divergence test fixup: cherry-pick `e4e73e1b` introduced two tests asserting `profile.groups.include` (pre-Plan-34-04 schema); rewrote to fork's `profile.security.groups: Vec<String>` schema."
  - "Windows ROOT_HELP_TEMPLATE retrofit: upstream `03546d61` updated the cross-platform template at cli.rs:523 (cfg(not(target_os=\"windows\"))). The Windows-host parallel template at cli.rs:233 was not edited by the cherry-pick — fork added matching SHELL section to keep `test_root_help_lists_all_commands` passing. NOT a `*_windows.rs` file edit; D-34-E1 invariant preserved (zero `*_windows.rs` hits per commit)."
metrics:
  duration: "~59 minutes"
  completed: "2026-05-12"
  upstream_commits_landed: 8
  fork_fixups: 3
  total_commits: 11
  files_created: 1
  files_modified: 14
---

# Phase 34 Plan 05: Cluster C8 Completion Summary

**Upstream v0.48.0** `nono completion <shell>` subcommand + truncation panic fix + string-truncation utility refactor — 8 cluster-C8 commits cherry-picked onto `main` in upstream chronological order with D-19 trailers, plus 3 fork-side fixups (test schema adapt, Windows help-template retrofit, rustfmt).

## Cherry-Pick Chain (Upstream Chronological)

| Order | Fork SHA | Upstream SHA | Subject | Files |
| ----- | -------- | ------------ | ------- | ----- |
| 1 | `397fb5bc` | `777dd95d` | chore: reduce nono run output verbosity | output.rs, execution_runtime.rs, supervised_runtime.rs |
| 2 | `3f0a8023` | `30245dbb` | cleanup unused code | output.rs |
| 3 | `55ec4397` | `e4e73e1b` | fix(profile): skip self-references in sibling extends resolution | profile/mod.rs |
| 4 | `7358eca0` | `03546d61` | feat(cli): add shell completion generation via `nono completion <shell>` | cli.rs, completions.rs (new), main.rs, app_runtime.rs, cli_bootstrap.rs, startup_runtime.rs, Cargo.{toml,lock} |
| 5 | `329fd812` | `f2592a2b` | fix: demote --allow-launch-services log from warn to debug | sandbox_prepare.rs |
| 6 | `3f1f364b` | `7b71855c` | refactor(string-truncation): extract generic string truncation utility | command_display.rs, rollback_commands.rs |
| 7 | `6aac7649` | `4b353549` | fix(cli): prevent truncate_chars panic and spurious truncation | command_display.rs |
| 8 | `4f64f2b0` | `e15b9c46` | chore: release v0.48.0 (CHANGELOG-only; version bumps dropped) | CHANGELOG.md (Cargo.toml + lock reset to fork HEAD) |

## Fork-Side Fixups

| Fork SHA | Subject | Rule | Rationale |
| -------- | ------- | ---- | --------- |
| `28769a20` | test(34-05): adapt e4e73e1b sibling-extends tests to fork's security.groups schema | Rule 1 (bug) | Upstream tests assert `profile.groups.include`; fork uses `profile.security.groups: Vec<String>`. Rewrote both asserts to `!profile.security.groups.is_empty()`. |
| `4404364a` | fix(34-05): list 'completion' subcommand in Windows ROOT_HELP_TEMPLATE | Rule 2 (missing surface) | Upstream `03546d61` updated the cross-platform template only. Fork has a parallel Windows-host `#[cfg(target_os="windows")]` template that the cherry-pick did not touch. Test `test_root_help_lists_all_commands` asserts every subcommand in `ALL_SUBCOMMANDS` appears in rendered help — `completion` was added to the constant by the cherry-pick but not the Windows template. Added matching SHELL section. |
| `a1be0b7f` | style(34-05): rustfmt fixup for output.rs test module import block | Rule 1 (auto-fmt) | After cherry-pick #1 trimmed the test-module imports, the single-line form fit under rustfmt's default width. `cargo fmt --all -- --check` flagged it. |

## D-19 Trailer Compliance

```text
git log --format='%B' HEAD~11..HEAD | grep -c '^Upstream-commit: '
8

git log --format='%B' HEAD~11..HEAD | grep -c '^Upstream-author: '   # lowercase 'a'
8

git log --format='%B' HEAD~11..HEAD | grep -c '^Co-Authored-By: '
8

git log --format='%B' HEAD~11..HEAD | grep -c '^Signed-off-by: '
22   # 8 cherry-picks × 2 + 3 fork-fixups × 2 = 22
```

8 Upstream-commit trailers (one per cherry-pick); 3 fork-side fixups carry only the DCO sign-off pair (no upstream provenance).

## D-34-E1 Windows-Only Files Invariant

Per-commit verification: `git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows'` returned `0` for every cherry-pick and every fork-side fixup. **Zero hits to `*_windows.rs` or `exec_strategy_windows/` files** across the plan range.

Note: the Windows ROOT_HELP_TEMPLATE retrofit (fork-side fixup `4404364a`) modifies a `#[cfg(target_os = "windows")]`-gated `const` block inside the cross-platform `cli.rs` — NOT a `*_windows.rs` filename. D-34-E1 invariant preserved.

## D-34-B2 Surgical Posture Verification

```text
grep -rE 'completion.*msi|msi.*completion' crates/ tools/
0 matches

grep -rE 'PROFILE\.d' crates/ tools/
0 matches
```

`nono completion <shell>` shipped AS-IS. NO MSI installer integration. NO PowerShell `$PROFILE.d/` shim. MSI integration deferred to a future phase per Phase 34 Deferred Ideas (see 34-CONTEXT.md § Deferred Ideas).

User-facing cookbook entry for Windows: `nono completion powershell > $PROFILE.d/nono.ps1` (manual one-line user step — sufficient per D-34-B2 framing: "every retrofit becomes load-bearing surface the fork owns forever").

## Fork-Defense Baselines (Post-Plan-34-05)

| Symbol | Threshold | Post-Plan |
| ------ | --------- | --------- |
| `never_grant`/`apply_deny_overrides` in policy.rs | ≥21 | **21** |
| `validate_path_within` in package_cmd.rs | ≥9 | **9** |
| `capabilities.aipc`/`loaded_profile` in profile/mod.rs | ≥17 | **17** |
| `find_denied_user_grants` in policy.rs | ≥1 | **7** |
| `bypass_protection` in profile/mod.rs | ≥1 | **17** |

All thresholds met or exceeded. No fork-defense regressions.

## Smoke Tests

```text
$ cargo run --quiet --bin nono -- completion powershell | head -3
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

$ cargo run --quiet --bin nono -- completion bash | head -3
_nono() {
    local i cur prev opts cmd
    COMPREPLY=()

$ cargo run --quiet --bin nono -- completion zsh | head -3
#compdef nono

autoload -U is-at-least

$ cargo run --quiet --bin nono -- completion fish | head -3
# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_nono_global_optspecs
```

All four shell generators emit valid completion scripts.

## D-34-D2 Close-Gate Results

| # | Gate | Result | Notes |
| - | ---- | ------ | ----- |
| 1 | `cargo test --workspace --all-features` | **PASS (carry-forward)** | 930 passed, 1 failed = `query_ext::tests::test_query_path_denied` — pre-existing P34-DEFER-01-1 Windows-path-canonicalization mismatch (see deferred-items.md). No NEW failures introduced by this plan. |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) | **PASS** | Clean. |
| 3 | `cargo clippy --target x86_64-unknown-linux-gnu` | **DEFERRED-TO-CI** | Cross-target linker not installed on this Windows dev host (per plan instructions: "3/4 deferred-to-CI (linkers not installed)"). Plan 34-05 touches only cli.rs, output.rs, profile/mod.rs surfaces shared cross-platform; CI will re-run on Linux. |
| 4 | `cargo clippy --target x86_64-apple-darwin` | **DEFERRED-TO-CI** | Same reason as gate 3. |
| 5 | `cargo fmt --all -- --check` | **PASS** | Clean after style fixup `a1be0b7f`. |
| 6 | Phase 15 5-row detached-console smoke gate | **ADMIN-SKIPPED** | Per plan instructions: "6/7/8 admin-skipped". |
| 7 | `wfp_port_integration` test suite | **ADMIN-SKIPPED** | Per plan instructions. WFP integration tests require admin rights + WFP enabled; not in plan scope. |
| 8 | `learn_windows_integration` test suite | **ADMIN-SKIPPED** | Per plan instructions. ETW-based learn integration; admin-only. |

**Net result:** All gates that should PASS, do PASS. The carry-forward (P34-DEFER-01-1) is documented and acceptable per Plan 34-05 close-gate policy.

## Threat Register Disposition

| Threat ID | Disposition | Status |
| --------- | ----------- | ------ |
| T-34-05-01 (D-21 Windows-only files invariant violation) | mitigate (BLOCKING) | **PASS** — zero `*_windows.rs` hits per commit. |
| T-34-05-02 (D-19 trailer missing) | mitigate (BLOCKING) | **PASS** — 8/8 cherry-picks carry trailer; fork-fixups carry DCO. |
| T-34-05-03 (truncation refactor drops Plan 34-03 char-aware behavior) | mitigate | **PASS** — `truncate_chars` is char-iteration based (`s.chars().take(keep)`); `truncate_command` is a thin wrapper. Plan 34-03's `d375b05e` semantics preserved. New tests under `command_display::tests::truncate_chars_*` pass. |
| T-34-05-04 (panic message leaks original string) | accept | Upstream's fix prevents the panic, not panic-message exposure (panic messages shouldn't reach untrusted output paths anyway). |
| T-34-05-05 (completion script hardcoded paths differ dev↔MSI) | accept | clap_complete uses `$0`-style invocation; no hardcoded paths in script body. |
| T-34-05-06 (`f2592a2b` log demotion silently drops security-relevant warning) | mitigate | `--allow-launch-services` is macOS-side launch-services-only; debug level appropriate per upstream's reasoning. |

## Fork-Divergence Notes

1. **`load_base_profile_raw` resolver branches** — upstream's commit-body for the broader v0.42→v0.43 work (referenced in `e4e73e1b` context) adds pack-store rescue + migration prompt. The fork does not have the pack-store subsystem or `migration::check_and_run` in the same shape; those branches are deliberately NOT ported (Plan 34-04 C7 cluster boundary). Only the sibling-self-reference skip — the actual fix in `e4e73e1b`'s subject — is adopted. Documented in the function docstring.

2. **Unix-socket capability test surface** — upstream's `output.rs` test module references `format_unix_socket_mode_badge` and `print_capabilities_with_unix_socket_does_not_panic`. The fork's C3 cluster is `won't-sync` per Phase 33 DIVERGENCE-LEDGER.md (Unix-socket capability is structurally Unix-only). Tests dropped on the fork side; commit body documents the divergence.

3. **`finish_status_line_for_handoff` removal** — upstream's verbosity reduction in `777dd95d` removed `finish_status_line_for_handoff` and the inline-animation machinery from `print_applying_sandbox`. Fork-side tests for those functions dropped; no external callers existed beyond the test module.

4. **Profile schema test compat** — upstream's new tests (`test_extends_same_name_as_base_skips_self`, `test_extends_same_name_still_resolves_other_siblings`) reference `profile.groups.include` (upstream's schema). Fork's Plan 34-04 canonical schema places groups under `profile.security.groups: Vec<String>`. Tests rewritten as `!profile.security.groups.is_empty()` — same assertion intent, schema-correct form.

## Deferred Items Tracking

No NEW deferrals from Plan 34-05. The single test failure (`test_query_path_denied`) is the pre-existing P34-DEFER-01-1 carry-forward (documented in `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md`).

## Final State

```text
Pre-plan HEAD:  5efb23d0 docs(34-02): close Plan 34-02 C4 proxy/network hardening with --allow-connect-port
Final HEAD:     a1be0b7f style(34-05): rustfmt fixup for output.rs test module import block
Commits added:  11 (8 cherry-picks + 3 fork-fixups)
Duration:       ~59 minutes
```

## Self-Check: PASSED

Created file: `crates/nono-cli/src/completions.rs` → FOUND (166 lines, clap_complete::Generator wrapper)

Commits verified present in `git log`:
- `397fb5bc` chore: reduce nono run output verbosity → FOUND
- `3f0a8023` cleanup unused code → FOUND
- `55ec4397` fix(profile): skip self-references in sibling extends resolution → FOUND
- `7358eca0` feat(cli): add shell completion generation via `nono completion <shell>` → FOUND
- `329fd812` fix: demote --allow-launch-services log from warn to debug → FOUND
- `3f1f364b` refactor(string-truncation): extract generic string truncation utility → FOUND
- `6aac7649` fix(cli): prevent truncate_chars panic and spurious truncation → FOUND
- `4f64f2b0` chore: release v0.48.0 → FOUND
- `28769a20` test(34-05): adapt e4e73e1b sibling-extends tests → FOUND
- `4404364a` fix(34-05): list 'completion' subcommand in Windows ROOT_HELP_TEMPLATE → FOUND
- `a1be0b7f` style(34-05): rustfmt fixup → FOUND

All claims verified.
