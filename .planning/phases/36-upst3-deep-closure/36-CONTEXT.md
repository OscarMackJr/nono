# Phase 36: UPST3 deep closure - Context

**Gathered:** 2026-05-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb the three heavy P34 NEEDS-FOLLOW-UP-PLAN deferrals that Phase 35 explicitly punted: full `deprecated_schema` module port (REQ-PORT-CLOSURE-02 / P34-DEFER-04b-1), the `wiring.rs` base abstraction + `yaml_merge` wiring trio (REQ-PORT-CLOSURE-04 / P34-DEFER-06-1 + P34-DEFER-09-2), and the `b5f0a3ab` deep ExecConfig refactor + `bbdf7b85` escape-quote pipeline tail (REQ-PORT-CLOSURE-05 / P34-DEFER-08b-1 + P34-DEFER-08b-2). Phase 36 is upstream-port closure; total estimate ~4-6 weeks across 6 plans organized as Wave 1 parallel (3 plans, disjoint surfaces) + Wave 2 sequential within REQ-02 (3 plans on the same `Profile` / `profile_cmd` / data + docs surface).

**In scope:**
- **REQ-PORT-CLOSURE-02** (P34-DEFER-04b-1, ~1-2 weeks) — Full Option C `deprecated_schema` module port. Verbatim 824-LOC `LegacyPolicyPatch` rewriter + per-key `DeprecationCounter` + `nono profile validate --strict` mode + 210-callsite internal rename `override_deny` → `bypass_protection` + canonical Profile sections (`groups`, `commands.{allow,deny}`, `filesystem.{deny,bypass_protection}`) + `nono-profile.schema.json` restructure + built-in profile data migration (claude-code, codex, opencode, claude-no-keychain) + `scripts/test-list-aliases.sh` + `scripts/lint-docs.sh` + `docs/cli/features/profiles-groups.mdx` + `docs/cli/usage/flags.mdx`. Split into 4 sequential sub-plans 36-01a/b/c/d per deferred-items.md.
- **REQ-PORT-CLOSURE-04** (P34-DEFER-06-1 + P34-DEFER-09-2, ~3-5 days stripped-down scope) — Stripped-down `wiring.rs` port: fork-side `crates/nono-cli/src/wiring.rs` created carrying ONLY the `yaml_merge` directive machinery + `serde_yaml_ng` 0.10.0 pin (`242d4917`) + reversal failure test. Acceptance criterion #1 (idempotent JSON-merge install records) **explicitly scope-trimmed** to v2.5-FU follow-up — see deferred section. Plan 36-02.
- **REQ-PORT-CLOSURE-05** (P34-DEFER-08b-1 + P34-DEFER-08b-2, ~1-1.5 weeks) — Surgical port of `b5f0a3ab` value-adding helpers (NOT ExecConfig struct refactor) + `bbdf7b85` escape-quote pipeline tail. Fork's ExecConfig 8+ load-bearing fields stay intact. Plan 36-03.

**Out of scope (route elsewhere or explicitly defer):**
- **REQ-PORT-CLOSURE-03 (P34-DEFER-04b-2) — profile drafts feature absorption** — Stays in Phase 36.5 per ROADMAP planner-discretion default. NOT folded into Phase 36 (user-locked at discuss). `nono profile promote` + `--draft` + `package_status.rs` + `NonoError::ActionRequired` + profile-drafts directory infrastructure land in their own phase.
- **Full `wiring.rs` base abstraction port** (~1761 LOC upstream surface: `WriteFile` / `JsonMerge` / `JsonArrayAppend` directives + SHA-256-keyed install records + lockfile v3+v4 + idempotent reversal + `--force` on `nono remove`) — Conflicts with fork's "Hooks subsystem ownership" + "validate_path_within retention" invariants (D-34-B1 + multiple catalog entries in `.planning/templates/upstream-sync-quick.md`). Deferred to v2.5-FU-3 (new follow-up registered below); REQ-PORT-CLOSURE-04 acceptance criterion #1 intentionally not satisfied in v2.4.
- **Upstream-shape ExecConfig adoption** — Adopting upstream's `b5f0a3ab` ExecConfig struct shape with explicit fork-side field additions was rejected in favor of surgical helper port (D-36-D1). Fork's `ExecConfig` 8+ fields (`capability_elevation`, `resource_limits`, `audit_signer`, `no_diagnostics`, `threading`, `protected_paths`, `profile_save_base`, `startup_timeout`, `allowed_env_vars`, `denied_env_vars`, `bypass_protection_paths`) stay verbatim. Adoption deferred to v2.5-FU-4.
- **Hard-deprecation of legacy `override_deny` key** — D-36-B3 locks indefinite serde-alias acceptance with one-shot stderr warn (non-strict) + fail-closed (`--strict`). NO hard-deprecation date set in v2.4. v2.5+ may revisit via separate ADR.
- **Audit-event retrofit for env-filter / yaml_merge / profile-validate outcomes** — D-34-B2 surgical-retrofit posture inherited; no new audit-event hooks added beyond what upstream's port carries.
- **All v2.3 host-blocked carry-forwards** — REQ-RESL-NIX-01..03 + REQ-PKGS-01 + REQ-PKGS-04 belong in Phase 37 (Linux/macOS host execution of Plan 25-01 + Plan 26-02), not Phase 36.
- **UPST4 audit + sync execution** — Phase 39 + Phase 40 (upstream v0.52.1 / v0.52.2 / v0.53.0).

</domain>

<decisions>
## Implementation Decisions

### Plan slicing & wave structure (Area A)

- **D-36-A1: Six plans total — 4 sub-plans for REQ-02, 1 for REQ-04, 1 for REQ-05.** Sub-plan slicing mirrors deferred-items.md § P34-DEFER-04b-1 natural 4-way split:
  - **36-01a-DEPRECATED-SCHEMA-MODULE** — `deprecated_schema` module + `LegacyPolicyPatch` + `DeprecationCounter` (~3-4 days).
  - **36-01b-CANONICAL-PROFILE-SECTIONS** — canonical Profile sections (`groups`, `commands.{allow,deny}`, `filesystem.{deny,bypass_protection}`) (~2-3 days).
  - **36-01c-OVERRIDE-DENY-RENAME** — 210-callsite internal rename `override_deny` → `bypass_protection` (~3-4 days).
  - **36-01d-PROFILE-DATA-DOCS-TOOLING** — built-in profile data + `nono-profile.schema.json` + `scripts/test-list-aliases.sh` + `scripts/lint-docs.sh` + docs migration (~2-3 days).
  - **36-02-WIRING-YAML-MERGE** — fork-side `crates/nono-cli/src/wiring.rs` stripped-down port (yaml_merge + serde_yaml_ng pin + reversal failure test).
  - **36-03-EXECCFG-SURGICAL-PORT** — `b5f0a3ab` surgical helpers + `bbdf7b85` escape-quote tail (3 sequenced commits, one plan).

- **D-36-A2: Wave 1 parallel + Wave 2 sequential-within-REQ-02.** Wave 1 (parallel by disjoint surface, Phase 22 D-09/D-10/D-12 + Phase 35 D-35-A3 precedent): **36-01a + 36-02 + 36-03**. Surfaces: 36-01a touches new `deprecated_schema.rs` module + `Profile` struct (additive); 36-02 creates new `crates/nono-cli/src/wiring.rs` + `Cargo.toml` pin; 36-03 touches `exec_strategy.rs` + `execution_runtime.rs` + `cli.rs` + `diagnostic.rs` + `pty_proxy.rs` + `sandbox_log.rs` + `startup_prompt.rs` + `profile_save_runtime.rs`. No file overlap.
  - **Wave 2 (sequential within REQ-02):** **36-01b → 36-01c → 36-01d.** Strict ordering: canonical sections must land before rename (rename targets canonical names); rename must land before data/docs migration (data + docs use canonical field names). Wave 2 starts after 36-01a closes (LegacyPolicyPatch rewriter + DeprecationCounter must exist before canonical sections wire into them). Planner MAY interleave Wave 1's 36-02/36-03 with Wave 2's 36-01b/c/d if dev-host scheduling allows — wave shape is parallelism-allowed-not-mandated.
  - **Estimated wall-clock:** ~4 weeks if Wave 1 lands in parallel; ~5-6 weeks if fully serialized.

- **D-36-A3: Phase 36.5 stays separate — NOT folded into Phase 36.** REQ-PORT-CLOSURE-03 (profile drafts) is a 7th plan candidate explicitly **rejected** in discuss. Reviewer attention concentration argument + Phase 36 surface size already large + planner-discretion default in ROADMAP all weigh against fold-in. Phase 36.5 ships in its own `/gsd-plan-phase 36.5` invocation when Phase 36 closes.

- **D-36-A4: Six PRs, one per plan, direct-on-main.** Inherits Phase 35 D-35-D1 / Phase 34 D-34-D1 ("direct-on-main; one PR per plan"). Wave-parallel plan execution means up to 3 PRs may be open simultaneously in Wave 1; Wave 2's 36-01b/c/d PRs land sequentially after Wave 1 closes the foundation. PR-merge ordering follows surface readiness, not plan-letter ordering, EXCEPT within REQ-02 where 01a → 01b → 01c → 01d is strict.

- **D-36-A5: Per-plan close gate inherits Phase 34 D-34-D2 / Phase 35 D-35-D2 verbatim — all 8 steps.** Before each Phase 36 plan can close on the dev host (Windows):
  1. `cargo test --workspace --all-features` (Windows host).
  2. `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host).
  3. `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — load-bearing for any plan touching cross-platform Linux-gated code. Phase 25 CR-A regression lesson (see `memory/feedback_clippy_cross_target.md`).
  4. `cargo clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` — symmetric coverage for macOS-gated code. **Critical for Plan 36-03** (`b5f0a3ab` introduces macOS `print_macos_run_guidance` + macOS-gated learn diagnostics; PTY-quiet-period change in `pty_proxy.rs` touches macOS-relevant code paths).
  5. `cargo fmt --all -- --check`.
  6. Phase 15 5-row detached-console smoke gate (`nono run --detached` → `nono ps` → `nono attach` → detach → `nono stop`).
  7. `wfp_port_integration` test suite (or documented-skipped).
  8. `learn_windows_integration` test suite (or documented-skipped).
  **STOP triggers (mid-plan):** any gate (1)–(8) fails. Plan freezes; investigate; either split the plan or roll back to last clean state.

- **D-36-A6: D-34-E1 Windows-only-files invariant inherited UNCHANGED.** Phase 36 surface does NOT require touching `*_windows.rs` files or `exec_strategy_windows/`. All 6 plans operate on cross-platform code (deprecated_schema, Profile structs, wiring.rs, exec_strategy.rs, execution_runtime.rs, diagnostic.rs, etc.). If a planner discovers a Windows-only touch is required during research, escalate via the `gsd-plan-checker` D-34-E1 inversion path (Phase 35 D-35-A1 precedent — explicit decision row required).

### REQ-02 deprecated_schema port (Area B)

- **D-36-B1: Full verbatim port — all 824 LOC + 210-callsite rename + canonical sections + JSON schema restructure + built-in profile data migration + docs.** Land everything upstream's `deprecated_schema` module delivers: `LegacyPolicyPatch` rewriter (legacy `override_deny` keys deserialize → rewrite to canonical `bypass_protection` post-parse), per-key `DeprecationCounter` (first-encounter-per-process emission semantics matching upstream), `nono profile validate --strict` mode (fails closed on legacy keys with non-zero exit + clear error pointing to canonical key), 210-callsite internal rename across the 14+ touched files (`capability_ext.rs`, `cli.rs`, `command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `policy.rs`, `policy_cmd.rs`, `profile_cmd.rs`, `profile_runtime.rs`, `query_ext.rs`, `sandbox_prepare.rs`, `sandbox_state.rs`, `why_runtime.rs`, JSON schema fixtures), canonical Profile sections, JSON schema fixture restructure (`scripts/regenerate-schema.sh` produces matching output), built-in profile data migration (claude-code, codex, opencode, claude-no-keychain), `scripts/test-list-aliases.sh` + `scripts/lint-docs.sh` alias inventory enforcement, embedded profile-authoring guide + `docs/cli/features/profiles-groups.mdx` + `docs/cli/usage/flags.mdx`. Maximum byte-for-byte parity with upstream; replaces Phase 34-04b's pragmatic Option C rename-acceptance (serde alias + clap visible_alias + one-time stderr deprecation warning) as the canonical surface. Rationale (user-locked): future P34-DEFER absorptions (Phase 36.5 drafts + any subsequent yaml_merge / pack-migration work) pick up the canonical surface for free; fork stops accumulating divergence on profile-schema naming.

- **D-36-B2: Four sub-plans per deferred-items.md natural split (36-01a/b/c/d).** Sequencing: `36-01a` (deprecated_schema module + LegacyPolicyPatch + DeprecationCounter) → `36-01b` (canonical Profile sections) → `36-01c` (210-callsite rename) → `36-01d` (data + docs + tooling). Each plan owns a single change-class (module, struct fields, callsite rename, data + docs); reviewer attention concentrates per concern; per-plan roll-back is independent if a downstream plan surfaces unexpected breakage. Inter-plan dependency chain documented in PLAN.md `Depends on:` lines.

- **D-36-B3: Indefinite serde-alias acceptance + one-shot stderr warn (non-strict) + fail-closed strict mode. No hard-deprecation date in v2.4.** Both `override_deny` (legacy) and `bypass_protection` (canonical) keys accepted in JSON profile files indefinitely via serde alias + LegacyPolicyPatch rewriter. Non-strict default mode: emits one-shot stderr deprecation warning per legacy key per process (per-key DeprecationCounter tracks first-encounter-per-process emission). `--strict` mode: fails closed with `nono profile validate` exit code != 0 + clear error message pointing to canonical key. No forced migration date; user profiles keep loading after Phase 36 lands. Mirrors upstream's posture exactly. Future hard-deprecation deferred to v2.5+ via a separate ADR (registered as v2.5-FU-5 below).

- **D-36-B4: Atomic single-commit for the 210-callsite rename in Plan 36-01c.** One commit. Mechanical sed/IDE rename across all 14+ files (capability_ext.rs, cli.rs, command_runtime.rs, execution_runtime.rs, launch_runtime.rs, main.rs, policy.rs, policy_cmd.rs, profile_cmd.rs, profile_runtime.rs, query_ext.rs, sandbox_prepare.rs, sandbox_state.rs, why_runtime.rs, JSON schema fixtures). `cargo build --workspace --all-features` + `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` + `cargo test --workspace --all-features` gate guarantees rename is consistent. Reviewer sees one clean diff; roll-back is one revert; matches Phase 33 + Phase 34 atomic-cherry-pick discipline. NO staged file-by-file mini-commits with type-alias scaffolding; NO two-step types-renamed-first commits. Planner MAY split into 2 commits ONLY if the rename + tests don't fit cleanly into a single commit (e.g., test fixtures require their own commit due to file path renames) — but the rename across `.rs` files MUST be atomic.

### REQ-04 wiring.rs base + yaml_merge directive (Area C)

- **D-36-C1: Stripped-down port — yaml_merge directive only.** Create fork-side `crates/nono-cli/src/wiring.rs` carrying ONLY: (1) the yaml_merge parser + applier from upstream `d44f5541` (re-shaped to match fork's profile-patch idioms; upstream's `WriteFile` / `JsonMerge` / `JsonArrayAppend` install directives **explicitly excluded**), (2) `serde_yaml_ng` 0.10.0 pin in `crates/nono-cli/Cargo.toml` from upstream `242d4917`, (3) the reversal failure test from upstream `242d4917`. Total scope: ~300-400 LOC vs upstream's 1761 LOC. Closes REQ-PORT-CLOSURE-04 acceptance criteria #2 (`nono profile patch --yaml <overlay>` accepts `yaml_merge:` directives matching upstream semantics), #3 (`serde_yaml_ng` pinned to 0.10.0), #4 (reversal failure test landed). **Acceptance criterion #1 ("wiring.rs abstraction landed with idempotent JSON-merge install records") explicitly scope-trimmed — see deferred section v2.5-FU-3.** Rationale (user-locked): fork's package system (package.rs 399 LOC + package_cmd.rs 1379 LOC + hooks.rs 630 LOC) is explicitly preserved per D-34-B1 + the "Hooks subsystem ownership" + "validate_path_within retention" catalog entries in `.planning/templates/upstream-sync-quick.md`; full wiring.rs braiding would either delete fork-only retention items or require ~2-3 weeks of careful integration work — not the right cost for v2.4 "Complete the Partial Ports" theme. The HIGH-VALUE SHA-256 install records + idempotent reversal benefits are additive to fork's existing Phase 22-03 PKG-04 + Phase 26-01 PKGS-02 path-validation defense-in-depth, not replacement; defer to v2.5-FU-3 when a dedicated 2-3 week D-20 manual-replay plan can land them properly.

- **D-36-C2: D-20 manual-replay shape — single combined commit citing 3 upstream commits as design source.** One commit creates fork-side `crates/nono-cli/src/wiring.rs` + bumps `serde_yaml_ng` to 0.10.0 in `Cargo.toml` + adds the reversal failure test. **NO `Upstream-commit:` D-19 trailer block** (upstream's commits modified upstream-only `wiring.rs` which doesn't exist in fork; cherry-pick was structurally infeasible). Commit body cites all 3 upstream commits as design-source citations: `242d4917` (serde_yaml_ng pin + reversal failure test), `802c8566` (rustfmt over wiring.rs — noted as no-op for fork's shape), `d44f5541` (yaml_merge directive). Matches Plan 35-01 + Plan 34-08a D-20 manual-replay precedent (`crates/nono-cli/src/exec_strategy_windows/`). Commit body documents WHY a clean cherry-pick was infeasible and what was preserved vs adapted vs intentionally omitted.

### REQ-05 b5f0a3ab + bbdf7b85 (Area D)

- **D-36-D1: Keep fork's ExecConfig shape; surgically port helpers + bbdf7b85 escape-quote tail.** DO NOT refactor `crates/nono-cli/src/exec_strategy.rs::ExecConfig` struct. All 8+ fork-side ExecConfig fields (`capability_elevation`, `resource_limits`, `audit_signer`, `no_diagnostics`, `threading`, `protected_paths`, `profile_save_base`, `startup_timeout`, `allowed_env_vars`, `denied_env_vars`, `bypass_protection_paths`) stay verbatim. Surgical port targets from deferred-items.md:
  - `crates/nono-cli/src/exec_strategy.rs` (244 lines of upstream changes): `should_offer_profile_save()` predicate guarding the profile-save offer, `clear_signal_forwarding_target()` call before profile-save prompt, `POST_EXIT_PTY_DRAIN_TIMEOUT` constant (250ms → 100ms quiet period), startup-timeout machinery integration.
  - `crates/nono-cli/src/execution_runtime.rs` (46 lines): `should_apply_startup_timeout()` helper, `startup_timeout_profile()` helper, `compute_executable_identity()` helper, new tests for startup-timeout interactive-vs-non-interactive arms.
  - `crates/nono-cli/src/cli.rs`: restore `LearnArgs.trace` field (referenced by Plan 34-08b's commit 3/5 `print_learn_deprecation` helper with the TODO marker).
  - `crates/nono-cli/src/profile_save_runtime.rs`, `pty_proxy.rs`, `sandbox_log.rs`, `startup_prompt.rs`: minor refinements per upstream b5f0a3ab.
  - `crates/nono/src/diagnostic.rs`: restore `extract_path_after_syscall_word`, `infer_access_from_structured_syscall_line`, `extract_structured_path_property`, `extract_structured_string_property` (4 helper functions removed during Plan 34-08b commit 4/5 to avoid -D warnings dead-code failures), wire all 4 helpers into `analyze_error_output`, restore `test_analyze_error_output_detects_node_eperm_mkdir_as_write` test.
  - bbdf7b85 escape-quote pipeline tail: body rewrite of `extract_structured_string_property` to handle escape-quoted characters (e.g., `path: '/Users/luke/it\'s/pkg'`); add `test_analyze_error_output_detects_structured_node_eperm_mkdir_path` + `test_analyze_error_output_detects_structured_path_with_escaped_quote` tests.
  Rationale (user-locked): fork's 8+ extra ExecConfig fields are load-bearing for Phase 18 (capability_elevation), Phase 26 (bypass_protection), Phase 27 (audit_signer), Phase 31 (broker ConPTY threading), Phase 34-08a (env-filter), Phase 35 (env-filter Windows wiring); a full struct adoption requires per-field migration audit + cross-platform regression testing of all 5+ fork-defense surfaces. The user-visible improvements (better profile-save UX, faster PTY drain, startup-timeout for stuck agents, macOS learn diagnostics) are absorbable via function-level helpers without restructuring the struct.

- **D-36-D2: Single plan 36-03 with three sequenced commits.** One PLAN.md / one PR. Internal commit sequence:
  - **Commit 1 — b5f0a3ab surgical diagnostic.rs restoration:** restore the 4 helpers (`extract_path_after_syscall_word`, `infer_access_from_structured_syscall_line`, `extract_structured_path_property`, `extract_structured_string_property`) + their wiring into `analyze_error_output` + the 1 test that landed in Plan 34-08b commit 2/5 but failed without wiring (`test_analyze_error_output_detects_node_eperm_mkdir_as_write`). **D-20 manual-replay shape; commit body cites `b5f0a3ab` as design-source. NO D-19 trailer** (fork's exec_strategy.rs / execution_runtime.rs / cli.rs shape differs structurally from upstream's; cherry-pick infeasible).
  - **Commit 2 — b5f0a3ab surgical exec_strategy + execution_runtime + cli.rs + ancillary refinements:** `should_offer_profile_save()`, `clear_signal_forwarding_target()` integration, `POST_EXIT_PTY_DRAIN_TIMEOUT` 250→100ms, startup-timeout machinery, `should_apply_startup_timeout()`, `startup_timeout_profile()`, `compute_executable_identity()`, `LearnArgs.trace` restoration, profile_save_runtime.rs / pty_proxy.rs / sandbox_log.rs / startup_prompt.rs minor refinements. **D-20 manual-replay shape; commit body cites `b5f0a3ab` as design-source. NO D-19 trailer.**
  - **Commit 3 — bbdf7b85 escape-quote body rewrite + new tests:** `extract_structured_string_property` function-body rewrite to handle escape-quoted characters + 2 new tests (`test_analyze_error_output_detects_structured_node_eperm_mkdir_path` from b5f0a3ab, `test_analyze_error_output_detects_structured_path_with_escaped_quote` from bbdf7b85). **D-19 cherry-pick shape with full 6-line `Upstream-commit:` trailer block citing `bbdf7b85`** (lowercase 'a' in `Upstream-author:`). bbdf7b85 can apply cleanly as a near-pure body-rewrite once Commit 1 has restored the helper + its wiring; bbdf7b85's diff target lines exist after Commit 1 lands.
  - **Smoke check at plan close:** `git log --format='%B' main~3..main | grep -c '^Upstream-commit: '` equals exactly 1 (only Commit 3 carries the D-19 trailer).

- **D-36-D3: PTY-quiet-period regression test coverage.** REQ-PORT-CLOSURE-05 acceptance criterion #3 requires "PTY-quiet-period absorbed without regressing fork's Phase 17 attach-streaming or Phase 31 broker ConPTY path". Plan 36-03 MUST include explicit regression coverage:
  - **Phase 17 attach-streaming:** existing tests in `crates/nono-cli/tests/attach_streaming*.rs` (or whatever the current test surface is) must continue to pass post-change. If the `POST_EXIT_PTY_DRAIN_TIMEOUT 250→100ms` change surfaces a flake on the attach-streaming path, the plan blocks; investigate; either reduce the change to 250→150ms as a compromise or roll back the quiet-period rider.
  - **Phase 31 broker ConPTY path:** Windows broker spawn path (`crates/nono-shell-broker/`) must still complete the `CreateProcessAsUserW(EXTENDED_STARTUPINFO_PRESENT)` Low-IL child spawn with the same quiet-period guarantees; Phase 15 5-row detached-console smoke gate (close-gate step 6) double-checks this.
  - **macOS learn diagnostics:** Plan 34-08b absorbed `print_macos_run_guidance` + `command_display::format_command_line` import; Plan 36-03 must NOT regress the Phase 10 / D-02 Windows admin gate that Plan 34-08b preserved.

### Carry-forward from Phase 34 + Phase 35 (still binding, Area E)

- **D-34-D2 close-gate inherited verbatim per D-36-A5** — all 8 steps; macOS clippy gate especially load-bearing for Plan 36-03 (b5f0a3ab macOS surface).
- **D-34-B2 surgical retrofit posture** — every "while we're here, let's also wire it up" temptation creates load-bearing fork surface. Phase 36 stays narrow: REQ-02 = full schema port, full stop; REQ-04 = yaml_merge only; REQ-05 = surgical helpers only. No audit-event retrofits, no WFP composition, no MSI integration, no profile-drafts fold-in.
- **D-34-E1 Windows-only-files invariant** — inherited UNCHANGED per D-36-A6 (no Phase 36 plan requires `*_windows.rs` edits).
- **D-34-E2 `Upstream-commit:` trailer block (verbatim 6-line shape)** — applies ONLY to Plan 36-03 Commit 3 (bbdf7b85 cherry-pick). All other Phase 36 commits use D-20 manual-replay shape. Lowercase 'a' in `Upstream-author:`. Smoke check at plan close per D-36-D2.
- **D-34-E3 D-20 manual-replay shape** — applies to Plan 36-01a/b/c/d (deprecated_schema work; structural rewrites with no clean upstream cherry-pick path), Plan 36-02 (yaml_merge; upstream-only `wiring.rs` file structurally infeasible to cherry-pick), Plan 36-03 Commits 1+2 (b5f0a3ab surgical port; fork's exec_strategy.rs / execution_runtime.rs shape diverges structurally from upstream).
- **D-35-A3 wave-parallel by disjoint surface** — Wave 1 of Phase 36 applies this precedent (3 plans, fully disjoint surfaces).
- **D-35-D3 / Phase 25 cross-target Linux clippy gate** — Phase 36 plans touching cross-platform code (most of them) MUST pass cross-target `--target x86_64-unknown-linux-gnu` clippy + `--target x86_64-apple-darwin` clippy. Phase 25 CR-A regression lesson per `memory/feedback_clippy_cross_target.md`.
- **CLAUDE.md § Coding Standards** — no `.unwrap()`, DCO sign-off (`Signed-off-by:`), `#[must_use]` on critical Results, env-var save/restore in tests (especially Plan 36-01 tests that exercise legacy-vs-canonical profile loading via `XDG_CONFIG_HOME` or similar).
- **CLAUDE.md § Path Handling** — Plan 36-02's yaml_merge must use path component comparison, not string `starts_with`; canonicalize before validation; validate_path_within callsites preserved where they intersect yaml_merge target paths.

### Claude's Discretion

- **Exact plan-letter suffix conventions for 36-01a/b/c/d** — D-36-B2 names the 4 sub-plans by theme; planner may pick the exact suffix shape (`36-01a-DEPRECATED-SCHEMA-MODULE` vs `36-01a-PORT-CLOSURE-02-MODULE` vs `36-01a-DEPSCHEMA`). Recommended: theme-readable suffix from Phase 34/35 precedent.
- **Wave-1-to-Wave-2 transition shape** — D-36-A2 permits wave overlap if 36-01a closes before 36-02 / 36-03 finish; planner may interleave. No correctness implication.
- **Whether to merge 36-01b + 36-01c into a single plan** — D-36-B2 specifies 4 sub-plans, but if 36-01b (canonical sections, ~2-3 days) and 36-01c (210-callsite rename, ~3-4 days) overlap heavily in file scope (`Profile` struct + 14+ callers), planner may merge with a documented escalation. Recommendation: keep separate for reviewer attention concentration.
- **Specific test naming inside Plan 36-03** — the 3 upstream tests (`test_analyze_error_output_detects_node_eperm_mkdir_as_write`, `test_analyze_error_output_detects_structured_node_eperm_mkdir_path`, `test_analyze_error_output_detects_structured_path_with_escaped_quote`) are the locked invariant; any additional regression tests (e.g., a parametric escape-quote enumeration) are planner discretion.
- **PR title conventions, draft vs ready-for-review state at open, reviewer assignment** — inherit Phase 34/35 conventions; not relitigated.
- **Phase 36 SUMMARY closure section in Phase 34 deferred-items.md** — last plan to close (likely 36-01d) appends a "Phase 36 closure" section flipping P34-DEFER-04b-1 / 06-1 / 08b-1 / 08b-2 / 09-2 from open to closed-by-Phase-36. Each plan SUMMARY records its own closure of the matching P34-DEFER-* entries.
- **Whether to add a regression test for the LegacyPolicyPatch rewriter round-trip** (legacy JSON → load → re-serialize → compare to canonical form) — not specified in upstream's surface; planner discretion. Recommendation: yes, as a property-level invariant.
- **PROJECT.md milestone summary line update** — handled by `/gsd-progress` at Phase 36 close (standard milestone tracking update); not a plan-level task.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 36 scope sources
- `.planning/ROADMAP.md` § Phase 36 (lines 145-157) — phase goal + Depends-on + estimate. § Phase 36.5 (lines 159-167) — confirms 36.5 stays separate per planner discretion.
- `.planning/REQUIREMENTS.md` § REQ-PORT-CLOSURE-02 (lines 42-54) — full What/Enforcement/Security/Acceptance/Maps-to for deprecated_schema port.
- `.planning/REQUIREMENTS.md` § REQ-PORT-CLOSURE-04 (lines 69-79) — full What/Enforcement/Security/Acceptance/Maps-to for wiring.rs + yaml_merge. **Acceptance #1 scope-trim per D-36-C1.**
- `.planning/REQUIREMENTS.md` § REQ-PORT-CLOSURE-05 (lines 81-91) — full What/Enforcement/Security/Acceptance/Maps-to for ExecConfig refactor + bbdf7b85.
- `.planning/PROJECT.md` § Current Milestone v2.4 (lines 9-36) — milestone shape; Phase 36 estimate context.

### Phase 34 deferred-items + decisions (binding precedent)
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` § P34-DEFER-04b-1 (lines 6-49) — REQ-02 source; the 4-way 2a/2b/2c/2d split that D-36-B2 inherits as Plan 36-01a/b/c/d.
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` § P34-DEFER-06-1 (lines 127-159) — REQ-04 source for yaml_merge wiring trio (242d4917 / 802c8566 / d44f5541).
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` § P34-DEFER-08b-1 (lines 203-265) — REQ-05 source for b5f0a3ab surgical-port targets (exec_strategy.rs + execution_runtime.rs + cli.rs LearnArgs + ancillary refinements).
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` § P34-DEFER-08b-2 (lines 267-316) — REQ-05 source for bbdf7b85 escape-quote tail and the 4 helpers + 3 tests + body rewrite + wiring it depends on.
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` § P34-DEFER-09-2 (lines 351-388) — REQ-04 source for full wiring.rs base abstraction; explains WHY full port is multi-week and conflicts with fork's package system (D-36-C1 rationale).
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-CONTEXT.md` — D-34-A1 (one-plan-per-cluster precedent), D-34-B1 (fork-only retention items including "Hooks subsystem ownership" + "validate_path_within retention"), D-34-B2 (surgical retrofit posture), D-34-D1 (direct-on-main; one PR per plan), D-34-D2 (8-step close gate), D-34-E1 (Windows-only files invariant), D-34-E2 (D-19 trailer block), D-34-E3 (D-20 manual port).
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-PHASE-OUTCOMES.md` — Phase 34 close ledger; informs how Phase 36 records its closures.
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-SUMMARY.md` + `34-04b-FP-CANONICAL-SCHEMA-SUMMARY.md` — Plan 34-04b shipped Option C rename-acceptance (serde alias + clap visible_alias + AtomicBool one-time stderr deprecation warning); D-36-B1 replaces this with full upstream port.
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-08a-ENV-SURFACE-PORT-SUMMARY.md` + `34-08b-DIAG-EXEC-SUMMARY.md` (or equivalent SUMMARY for Plan 34-08b) — record what was absorbed vs deferred from b5f0a3ab; the deferred surface is Plan 36-03's port target.

### Phase 35 decisions (sister phase; recently closed)
- `.planning/phases/35-upst3-closure-quick-wins/35-CONTEXT.md` — D-35-A1 (D-34-E1 inversion precedent — Phase 36 does NOT need it but the inversion-as-explicit-decision-row pattern is the model if Phase 36 plans later discover a Windows-only edit), D-35-A2 (one-plan-per-REQ discipline; Phase 36 extends to one-plan-per-sub-REQ for REQ-02), D-35-A3 (wave-parallel by disjoint surface — D-36-A2 inherits), D-35-A4 (D-19 trailer scoping rules; D-36-D2 + D-36-C2 inherit), D-35-D1/D2/D3 (PR + close-gate inheritance).

### Sync execution mechanics
- `.planning/templates/upstream-sync-quick.md` § D-19 cherry-pick trailer block — verbatim 6-line shape with lowercase 'a' in `Upstream-author:` (Plan 36-03 Commit 3 uses).
- `.planning/templates/upstream-sync-quick.md` § fork-only retention catalog — "Hooks subsystem ownership" + "validate_path_within defense-in-depth retention" — D-36-C1 rationale source.
- `docs/cli/development/upstream-drift.mdx` — long-form runbook for the cherry-pick + trailer convention.
- `.planning/PROJECT.md` § Upstream Parity Process — 4-step process (relevant for Plan 36-03 Commit 3 and the design-source citation pattern used by Plans 36-01a/b/c/d + 36-02 + 36-03 Commits 1+2).

### Pattern references (prior phases Phase 36 inherits or analogues)
- `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md` — wave-parallel by disjoint surface precedent (D-09/D-10/D-12 → D-36-A2 inherits).
- `.planning/phases/33-windows-parity-upstream-0-52-divergence/` — UPST3 audit phase that produced the DIVERGENCE-LEDGER feeding Phase 34, ultimately surfacing the P34-DEFER entries Phase 36 closes.
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-08b-DIAG-EXEC-PLAN.md` (or equivalent) — D-20 manual-replay precedent for ExecConfig field-shape mismatch handling (the design pattern Plan 36-03 Commits 1+2 mirror).

### Source files Phase 36 will touch
**Plan 36-01a (deprecated_schema module foundation):**
- NEW FILE: `crates/nono-cli/src/deprecated_schema.rs` (~824 LOC port from upstream). Contains `LegacyPolicyPatch` struct, `DeprecationCounter` per-key first-encounter-per-process tracker, `--strict` mode lever.
- `crates/nono-cli/src/main.rs` (or `lib.rs`) — register `mod deprecated_schema;`.
- `crates/nono-cli/src/cli.rs` — add `--strict` flag to `ProfileValidateArgs`.
- `crates/nono-cli/src/profile_cmd.rs` — wire `LegacyPolicyPatch` rewriter + DeprecationCounter into profile-load pipeline.
- Reference (do NOT delete unless explicitly part of plan): `crates/nono-cli/src/deprecated_policy.rs` (existing 50+ LOC `nono policy` subcommand deprecation shim) — different concern (CLI alias for `policy` → `profile`), retained as-is.

**Plan 36-01b (canonical Profile sections):**
- `crates/nono-cli/src/profile/` (mod.rs + builtin.rs) — restructure `Profile` / `LoadedProfile` structs to expose canonical sections (`groups`, `commands.{allow,deny}`, `filesystem.{deny,bypass_protection}`).
- `crates/nono/src/capability.rs` — verify `CapabilitySet` builder shape still composes with restructured Profile sections.

**Plan 36-01c (210-callsite rename `override_deny` → `bypass_protection`):**
- 14+ files per deferred-items.md: `crates/nono-cli/src/capability_ext.rs`, `cli.rs`, `command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `policy.rs`, `policy_cmd.rs`, `profile_cmd.rs`, `profile_runtime.rs`, `query_ext.rs`, `sandbox_prepare.rs`, `sandbox_state.rs`, `why_runtime.rs`, JSON schema fixtures (`crates/nono-cli/data/policy.json` + `crates/nono-cli/data/nono-profile.schema.json` + test fixtures in `crates/nono-cli/tests/fixtures/`).

**Plan 36-01d (data + docs + tooling):**
- `crates/nono-cli/data/policy.json` — built-in profile data migration (claude-code, codex, opencode, claude-no-keychain) to canonical sections.
- `crates/nono-cli/data/nono-profile.schema.json` — JSON schema fixture restructure.
- `scripts/test-list-aliases.sh` (new or extend existing) — alias inventory enforcement.
- `scripts/lint-docs.sh` (new or extend existing) — docs alias-inventory check.
- `scripts/regenerate-schema.sh` — must produce canonical-form output matching upstream.
- `docs/cli/features/profiles-groups.mdx` (new or migrate) — profile-authoring guide canonical surface.
- `docs/cli/usage/flags.mdx` — flag-deprecation surface migration.
- `crates/nono-cli/data/profile-authoring-guide.md` (embedded; new file if not present) — profile-authoring guide.

**Plan 36-02 (wiring.rs stripped-down port):**
- NEW FILE: `crates/nono-cli/src/wiring.rs` (~300-400 LOC) — yaml_merge directive parser + applier only. NO WriteFile / JsonMerge / JsonArrayAppend / install-record directives.
- `crates/nono-cli/Cargo.toml` — pin `serde_yaml_ng = "=0.10.0"`.
- `crates/nono-cli/src/main.rs` (or `lib.rs`) — register `mod wiring;`.
- `crates/nono-cli/src/profile_cmd.rs` (or `profile_runtime.rs`) — wire `yaml_merge` directive into `nono profile patch --yaml <overlay>` handler.

**Plan 36-03 (b5f0a3ab surgical + bbdf7b85):**
- `crates/nono/src/diagnostic.rs` (3368 LOC; Commit 1 surface) — restore 4 helpers + wire into `analyze_error_output` (currently lives at line 215) + restore 1 test; Commit 3 surface — body rewrite of `extract_structured_string_property` + 2 new escape-quote tests. Current state: helpers + test deferred per comment block at lines 403-416 + 2259-2270 of `diagnostic.rs`.
- `crates/nono-cli/src/exec_strategy.rs` (4148 LOC; Commit 2 surface) — `should_offer_profile_save()` predicate, `clear_signal_forwarding_target()` call, `POST_EXIT_PTY_DRAIN_TIMEOUT 250→100ms`, startup-timeout machinery integration. **Do NOT modify `pub struct ExecConfig<'a>` at line 276.**
- `crates/nono-cli/src/execution_runtime.rs` (486 LOC; Commit 2 surface) — `should_apply_startup_timeout()`, `startup_timeout_profile()`, `compute_executable_identity()` helpers + tests.
- `crates/nono-cli/src/cli.rs` (Commit 2 surface) — restore `LearnArgs.trace` field.
- `crates/nono-cli/src/profile_save_runtime.rs`, `pty_proxy.rs`, `sandbox_log.rs`, `startup_prompt.rs` (Commit 2 surface) — minor refinements per upstream b5f0a3ab.

### Upstream source commits (git-resolvable from `upstream` remote at `https://github.com/always-further/nono.git`)
- **`f0abd413`** (upstream, v0.47.0) — canonical JSON schema restructure. Source of REQ-02 + design pattern for Plans 36-01a/b/c/d.
- **`242d4917`** (upstream, v0.49.0) — `fix(yaml-merge): pin serde_yaml_ng to 0.10.0 and add reversal failure test`. Plan 36-02 D-20 design-source.
- **`802c8566`** (upstream, v0.49.0) — `style: apply rustfmt` (over upstream's wiring.rs). Plan 36-02 D-20 design-source (noted as no-op for fork's shape).
- **`d44f5541`** (upstream, v0.49.0) — `feat(wiring): add yaml_merge directive for YAML config patching`. Plan 36-02 D-20 design-source (primary content).
- **`24d8b924`** (upstream, v0.44.0) — base `wiring.rs` introduction (~1761 LOC). NOT ported in v2.4; deferred to v2.5-FU-3.
- **`bdf183e9`** (upstream, v0.44.0) — `fix(package): harden re-pulls against user edits`. The 188/239 lines in upstream's `wiring.rs` are part of the v2.5-FU-3 deferral; the 15-line `profile_runtime.rs` Landlock pre-create hunk was absorbed by Phase 35 Plan 35-02.
- **`b5f0a3ab`** (upstream, v0.52.0; Luke Hinds) — `feat(cli): enhance macos learn and run diagnostics`. 11 files / +721 / -118. Plan 36-03 Commits 1+2 D-20 design-source.
- **`bbdf7b85`** (upstream, v0.52.0; Luke Hinds) — `fix(diagnostic): parse escaped quotes in structured properties`. Plan 36-03 Commit 3 D-19 cherry-pick target.

### Coding & security standards
- `CLAUDE.md` § Coding Standards — no `.unwrap()`, DCO sign-off, `#[must_use]` on critical Results.
- `CLAUDE.md` § Testing § Environment variables in tests — save/restore pattern. **Critical for Plan 36-01a/b** tests that exercise legacy vs canonical profile loading (likely via test fixtures + `XDG_CONFIG_HOME` or `dirs::home_dir()` overrides — though Windows test-harness uses `NONO_TEST_HOME` seam per Phase 27.1).
- `CLAUDE.md` § Path Handling — Plan 36-02 yaml_merge target-path validation MUST use path component comparison + canonicalization.
- `CLAUDE.md` § Security Considerations — fail-secure on unsupported shape; principle of least privilege; defense in depth.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Phase 34-04b Option C rename-acceptance scaffolding** in `crates/nono-cli/src/profile_cmd.rs` — serde alias + clap visible_alias + AtomicBool one-time stderr deprecation warning. Plan 36-01a replaces with full upstream LegacyPolicyPatch + DeprecationCounter; keep the AtomicBool concept as the DeprecationCounter implementation mechanism but rewire it to upstream's per-key counter shape.
- **Existing `crates/nono-cli/src/deprecated_policy.rs`** (50+ LOC) — CLI-level deprecation shim for `nono policy <sub>` → `nono profile <sub>` delegation. Different concern from upstream's `deprecated_schema` module (which handles JSON schema-level legacy key rewriting); retained as-is. Plan 36-01a's new `deprecated_schema.rs` module does NOT replace `deprecated_policy.rs`.
- **`crates/nono/src/capability.rs::CapabilitySet`** — builder pattern unaffected by Profile section restructure; verify in Plan 36-01b that the `LoadedProfile` → `CapabilitySet` construction path still composes after canonical-section migration.
- **Phase 34-08b absorbed surface in `crates/nono-cli/src/learn_runtime.rs`** — macOS `print_macos_run_guidance` helper + `command_display::format_command_line` import. Plan 36-03 Commit 2 builds on this (don't re-absorb; verify still present).
- **Phase 34-08b diagnostic.rs additive fallback at `extract_relative_write_path_from_line`** — `extract_path_from_segment(prefix).or_else(|| extract_path_from_segment(line))`. Plan 36-03 Commit 1 preserves; the 4 restored helpers + their wiring into `analyze_error_output` are separate from this additive fallback.
- **Phase 35's `profile_cmd.rs::profile_to_json` / `::diff_to_json` Map-insertion shape** — Plan 35-03 landed the serde-driven `Option<…>` Map emission (omit-when-None). Plan 36-01b canonical sections SHOULD compose with this shape (no rework expected; canonical sections fit cleanly into the Map shape).

### Established Patterns
- **D-19 cherry-pick trailer (verbatim 6-line shape)** — Plan 36-03 Commit 3 (bbdf7b85) only. Cite `bbdf7b85` upstream commit + lowercase 'a' in `Upstream-author:`. Smoke check at plan close: `git log --format='%B' main~3..main | grep -c '^Upstream-commit: '` equals exactly 1.
- **D-20 manual-replay shape** — Plans 36-01a/b/c/d (deprecated_schema work), Plan 36-02 (yaml_merge stripped-down port), Plan 36-03 Commits 1+2 (b5f0a3ab surgical port). Commit bodies cite upstream commits as design-source; no `Upstream-commit:` trailer.
- **Atomic mechanical rename** (Phase 33 + Phase 34 precedent) — Plan 36-01c's 210-callsite `override_deny` → `bypass_protection` rename uses this pattern; one commit, full clippy + test + build gate.
- **Wave-parallel by disjoint surface** — Phase 22 D-09/D-10/D-12 + Phase 35 D-35-A3 precedent. Wave 1 of Phase 36 (36-01a + 36-02 + 36-03) applies.
- **8-step close gate verbatim** — Phase 34 D-34-D2 + Phase 35 D-35-D2 inheritance. Plan 36-03 macOS clippy gate especially load-bearing (b5f0a3ab introduces macOS-gated code paths).
- **Cross-target Linux clippy gate** — Phase 25 CR-A regression lesson per `memory/feedback_clippy_cross_target.md`. Plans touching cross-platform code (most of Phase 36) MUST pass `--target x86_64-unknown-linux-gnu` clippy.

### Integration Points
- **Plan 36-01a/b/c/d ↔ Phase 35 Plan 35-03 JSON Map-emission** — Phase 35 landed Map-insertion + omit-when-None for `Option<…>` security fields in `profile_cmd.rs::profile_to_json` / `::diff_to_json`. Plan 36-01b canonical Profile sections must preserve this shape; the canonical sections nest cleanly within the Map (no flat-shape regression).
- **Plan 36-02 ↔ fork's existing `crates/nono-cli/src/wiring.rs` (none yet)** — fork has no current `wiring.rs`. Plan 36-02 creates the file fresh with yaml_merge-only scope. Future v2.5-FU-3 extends with WriteFile/JsonMerge/JsonArrayAppend + SHA-256 install records.
- **Plan 36-02 ↔ fork's `crates/nono-cli/src/profile_cmd.rs` (`nono profile patch --yaml <overlay>` handler)** — yaml_merge directive is wired into the existing `--yaml` overlay path. Verify the handler surface composes with the new directive shape.
- **Plan 36-03 ↔ Phase 17 attach-streaming** — `POST_EXIT_PTY_DRAIN_TIMEOUT 250→100ms` change in `pty_proxy.rs` MUST NOT regress attach-streaming tests. Phase 17 test surface in `crates/nono-cli/tests/attach_streaming*.rs` or equivalent.
- **Plan 36-03 ↔ Phase 31 broker ConPTY path** — Windows broker spawn path must complete `CreateProcessAsUserW(EXTENDED_STARTUPINFO_PRESENT)` Low-IL child spawn with same quiet-period guarantees post-change. Phase 15 5-row detached-console smoke gate (close-gate step 6) double-checks.
- **Plan 36-03 ↔ Phase 10 / D-02 Windows admin gate in `learn_runtime.rs`** — Plan 34-08b absorbed `print_macos_run_guidance` + `command_display::format_command_line` import while preserving the Windows admin gate. Plan 36-03 Commit 2 MUST NOT regress this.
- **Plan 36-03 ↔ ExecConfig's 8+ fork-side fields** — every helper restored in Commits 1+2 must compose with `capability_elevation`, `resource_limits`, `audit_signer`, `no_diagnostics`, `threading`, `protected_paths`, `profile_save_base`, `startup_timeout`, `allowed_env_vars`, `denied_env_vars`, `bypass_protection_paths`. Specific check: `should_offer_profile_save()` predicate composes with `profile_save_base: Option<&'a str>`; `startup_timeout` machinery composes with `startup_timeout: Option<StartupTimeoutConfig<'a>>` at exec_strategy.rs line 301.
- **Phase 36 close ledger** — last plan to close (likely 36-01d, the data + docs + tooling tail of REQ-02) appends a "Phase 36 closure" section to `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` flipping P34-DEFER-04b-1, 06-1, 08b-1, 08b-2, 09-2 from open to closed-by-Phase-36. Phase 34's directory remains the source of truth for the deferred-items ledger; Phase 36's per-plan SUMMARY files describe what Phase 36 did, not where the deferrals live.

</code_context>

<specifics>
## Specific Ideas

- **Full verbatim REQ-02 port** (D-36-B1) — user explicitly rejected phased/adapted/minimal options. Rationale: future P34-DEFER absorptions pick up canonical surface for free; fork stops accumulating divergence on profile-schema naming. Maximum byte-for-byte parity with upstream.
- **4 sub-plans per deferred-items.md split** (D-36-B2) — user preferred the natural 4-way 2a/2b/2c/2d split over single mega-plan, 2-way (machinery+surface), or 3-way (collapse 2c+2d). Reviewer attention concentration + per-plan rollback independence + smaller PR sizes were the deciding factors.
- **Indefinite serde-alias acceptance + one-shot warn + strict-mode lever** (D-36-B3) — user explicitly rejected hard-deprecation at v2.5 / strict-default-for-new-installs / defer-decision-via-ADR. Mirrors upstream's exact posture; no forced migration date; user profiles keep working post-Phase-36 indefinitely.
- **Atomic single-commit for 210-callsite rename** (D-36-B4) — user rejected staged file-by-file mini-commits with type-alias scaffolding AND two-step types-first commits. Matches Phase 33 + Phase 34 atomic-cherry-pick discipline; one revert rolls back the rename if regression surfaces post-merge.
- **Stripped-down wiring.rs port (yaml_merge only)** (D-36-C1) — user rejected full braiding (~2-3 wks) + pure cherry-pick + scaffolded-for-v2.5 options. Acceptance #1 explicit scope-trim is the price; fork stays preserved on package/hooks surface.
- **D-20 single combined commit for REQ-04** (D-36-C2) — user preferred single combined commit citing all 3 upstream commits over 3 sequential D-19-trailered commits or hybrid. Cleanest provenance shape given the structural infeasibility of clean cherry-pick.
- **Surgical port keeping fork's ExecConfig shape** (D-36-D1) — user explicitly rejected adopting upstream's ExecConfig shape with explicit fork-side field additions (HIGH-RISK to Phase 18/26/27/31/34-08a/35-01 surfaces). Hybrid option (adopt helpers, keep struct) also rejected — too divergent from the surgical-port discipline. Minimum-risk path; preserves 8+ load-bearing fork fields.
- **Single plan 36-03 with sequenced commits** (D-36-D2) — user rejected 2-plan split (a/b) and 3-plan split-by-domain. Bundling b5f0a3ab + bbdf7b85 in one plan keeps the dependency chain intact (bbdf7b85 depends on b5f0a3ab's analyze_error_output wiring); sequenced commits provide the reviewability of separate plans without the inter-plan ordering ceremony.
- **Mostly parallel wave structure** (D-36-A2) — user rejected pure strict-sequential (~5-6 weeks) and pure wave-parallel-everywhere (REQ-02 sub-plans collide on Profile struct) and folding Phase 36.5 in. The "Wave 1 parallel + Wave 2 sequential-within-REQ-02 + 36.5 stays separate" shape balances reviewer attention with wall-clock efficiency.

</specifics>

<deferred>
## Deferred Ideas

- **v2.5-FU-3: Full wiring.rs base abstraction port** — Port full upstream `crates/nono-cli/src/wiring.rs` (~1761 LOC; WriteFile / JsonMerge / JsonArrayAppend directives + SHA-256-keyed install records + lockfile v3+v4 + idempotent reversal + `--force` on `nono remove`). Closes REQ-PORT-CLOSURE-04 acceptance criterion #1 fully. Estimated 2-3 weeks D-20 manual-replay plan; conflicts with fork's hooks.rs ownership + validate_path_within retention catalog entries — needs careful braiding plan. Trigger for promotion: a fork user blocked by missing idempotent install records, OR a v2.5 milestone with budget for the focused braiding work.
- **v2.5-FU-4: Upstream-shape ExecConfig adoption** — Refactor fork's ExecConfig to upstream's `b5f0a3ab` shape with explicit fork-side field additions (or feature-gated extension struct). Per-field migration audit covering `capability_elevation`, `resource_limits`, `audit_signer`, `no_diagnostics`, `threading`, `protected_paths`, `profile_save_base`, `startup_timeout`, `allowed_env_vars`, `denied_env_vars`, `bypass_protection_paths`. ~1-2 weeks + integration testing. Trigger for promotion: future upstream commit that's HIGH-VALUE but structurally requires the upstream ExecConfig shape (i.e., the next time we'd be deferring on field-shape mismatch).
- **v2.5-FU-5: Hard-deprecation ADR for legacy `override_deny` key** — Decide on a forced-migration date for `override_deny` → `bypass_protection`. Delete LegacyPolicyPatch rewriter + serde aliases at that date. Requires user-profile migration tool (`nono profile migrate --in-place`) + v2.5+ milestone-context note + clear release-notes communication. Currently deferred; no urgency since Phase 36 lands indefinite acceptance per D-36-B3.
- **v2.5-FU-6: Phase 17 attach-streaming + Phase 31 broker ConPTY PTY-quiet-period regression test surface formalization** — If Plan 36-03's `POST_EXIT_PTY_DRAIN_TIMEOUT 250→100ms` change surfaces flakes that require a compromise timeout (e.g., 150ms), formalize a parametric regression test suite covering 50ms / 100ms / 150ms / 250ms quiet-period arms. Currently relies on existing Phase 15 smoke gate + Phase 17 attach-streaming tests; no formal parametric coverage.
- **`run_nono` integration tests for Phase 36 surface** — host-blocked by `dirs::home_dir()` Windows test-harness gap (Phase 27.1 introduced `NONO_TEST_HOME` seam for audit-attestation; other integration paths may still need similar). Plans 36-01a/b/c/d unit tests cover legacy-vs-canonical profile loading invariants; full `run_nono` integration coverage defers to Phase 37/38 when Linux/macOS host is available (mirrors Phase 35 D-35-B1 / Phase 27.1 deferral pattern).
- **Phase 36.5 — REQ-PORT-CLOSURE-03 profile drafts feature absorption** — `nono profile promote` subcommand + `--draft` flag + `package_status.rs` + `NonoError::ActionRequired` + profile-drafts directory infrastructure with atomic ops + base-hash verification. Stays as a separate phase per ROADMAP planner-discretion default; not folded into Phase 36 (D-36-A3). Trigger to run: after Phase 36 closes; if Phase 36 scope strain warrants skipping 36.5, mark it as optional-deferred-to-v2.5.
- **PROJECT.md milestone summary line update** — handled by `/gsd-progress` at Phase 36 close (standard milestone tracking update); not a plan-level task.
- **PTY-quiet-period parametric proptest** — instead of fixed 250→100ms transition, consider proptest-driven coverage of (timeout × workload × shell type) tuples. Adds proptest setup cost to a surgical-port plan; reconsider if PTY-quiet-period interactions reveal complexity post-Phase 36.
- **Audit-event emission for profile-validate `--strict` rejections** — surfacing legacy-key rejections to the audit ledger would aid security-posture monitoring. Currently the rejection lands as stderr + non-zero exit; no audit-event hook. D-34-B2 surgical-retrofit posture defers this; reconsider in v2.5+ if audit visibility for profile-validation becomes a cross-platform requirement.

### Reviewed Todos (not folded)

None — `gsd-sdk query todo.match-phase 36` not run (no `.planning/todos/` artifact exists per scout). No pending todos surfaced for Phase 36 scope.

</deferred>

---

*Phase: 36-UPST3 deep closure*
*Context gathered: 2026-05-12*
