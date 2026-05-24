# Phase 48: UPST6 sync execution — Pattern Map

**Mapped:** 2026-05-24
**Phase shape:** sync-execution (mirrors Phase 34 + 40 + 43); zero new files; all integration-point files are pre-existing fork-side files touched by 42 upstream commits across 9 clusters.
**Files analyzed:** 14 integration-point files + 8 fork-only Windows surfaces (invariant)
**Analog precedents found:** 14 / 14 (100% — every integration-point file has a prior cherry-pick precedent in Phase 34/40/43)
**Cluster dispositions consumed:** 8 will-sync + 1 fork-preserve-upgrade-candidate (per Phase 47 ledger, immutable)
**Convention patterns extracted:** 10

---

## How to read this document

Phase 48 is a sync-execution phase. **No new files are created.** Every entry below maps a fork-side file that upstream commits will touch to (a) the Phase 48 plan(s) responsible, (b) the cluster + commit shas hitting that file, (c) the fork-side defense-in-depth invariants the executor must not regress, (d) the nearest worked-example precedent from Phase 34/40/43, and (e) the pre-flight check the executor should run before applying the upstream hunk.

The planner consumes this document to wire up per-plan `<read_first>` blocks: each plan body cites:
- the integration-point file rows for that cluster (gives the executor concrete fork-side context),
- the Convention Patterns section (gives the executor canonical worked examples for trailer blocks, close gates, etc.).

For the planner's convenience the rows are grouped by Phase 48 plan number.

---

## File Classification by Plan

| Integration-point file | Plan(s) | Cluster | Cherry-pick SHAs | Pattern Row |
|------------------------|---------|---------|------------------|-------------|
| `crates/nono/src/sandbox/linux.rs` | 48-01, 48-04, 48-06 | C4, C5, C7 | C4: c2c6f2ca, a0222be2; C5: 1122c315; C7: 3cd22aa5 | [#1](#1-cratesnonosrcsandboxlinuxrs) |
| `crates/nono/src/sandbox/mod.rs` | 48-01 | C4 | c2c6f2ca (re-export) | [#2](#2-cratesnonosrcsandboxmodrs) |
| `crates/nono/src/lib.rs` | 48-01 | C4 | c2c6f2ca (re-export) | [#3](#3-cratesnonosrclibrs) |
| `crates/nono/src/capability.rs` | 48-01, 48-05 | C4, C6 | C4: c2c6f2ca; C6: abca959a | [#4](#4-cratesnonosrccapabilityrs) |
| `crates/nono-cli/src/cli.rs` | 48-01, 48-03 | C4, C2 | C4: c2c6f2ca, bbc652a0, 858ad009; C2: a8646d26, 2bed3565, 468d3813, 4e0e127a | [#5](#5-cratesnono-clisrcclirs) |
| `crates/nono-cli/src/profile/mod.rs` | 48-01, 48-02, 48-05 | C4, C1, C6 | C1: 750f4653, 316c6a2c, 3d3d239a, c897c8cc, b3556139, 0015f348; C4: a0222be2; C6: abca959a | [#6](#6-cratesnono-clisrcprofilemodrs) |
| `crates/nono-cli/src/policy.rs` | 48-01, 48-04 | C4, C5 | C4: a0222be2; C5: e6215f8b, 4fa9f6a6 | [#7](#7-cratesnono-clisrcpolicyrs) |
| `crates/nono-cli/src/package_cmd.rs` | 48-08 | C9 (fork-preserve-upgrade-candidate) | 5f1c9c73, 8d774753 | [#8](#8-cratesnono-clisrcpackage_cmdrs) |
| `crates/nono/src/trust/policy.rs` | 48-08 | C9 | 5f1c9c73 (touches via trust-bundle schema) | [#9](#9-cratesnonosrctrustpolicyrs) |
| `crates/nono/src/manifest.rs` | 48-08 | C9 | 5f1c9c73 (manifest-driven install pipeline) | [#10](#10-cratesnonosrcmanifestrs) |
| `crates/nono-cli/data/nono-profile.schema.json` | 48-07 | C8 | 57005737, 530306ee | [#11](#11-cratesnono-clidatanono-profileschemajson) |
| `crates/nono-cli/src/exec_strategy.rs` | 48-03, 48-06 | C2, C7 | C2: a8646d26, 2bed3565, 1be97978; C7: 3cd22aa5 | [#12](#12-cratesnono-clisrcexec_strategyrs) |
| `crates/nono-cli/src/pty_proxy.rs` | 48-06 | C7 | 1f552106, 279af554, 3d0ff87f, 3cd22aa5 | [#13](#13-cratesnono-clisrcpty_proxyrs) |
| `crates/nono/CHANGELOG.md` (or fork's CHANGELOG location) | 48-09 | C3 (release-ride) | 35f9fea2, b251c72f, 10cec984 (stacked trailers) | [#14](#14-cratesnonochangelogmd) |
| `crates/nono-cli/src/exec_strategy_windows/` + `crates/nono-shell-broker/` | — (fork-side cleanup carve-out only) | C2 fork-side cleanup via D-48-D3 | none direct; `startup_prompt` references removed if found | [#15-invariant](#15-fork-only-windows-surface-d-48-e1-invariant) |

**Files NOT touched this cycle** (Phase 47 audit walked, zero upstream churn confirmed):
- `crates/nono-cli/src/platform.rs` (Phase 43 absorption surface; verified zero upstream commits in v0.54.0..v0.57.0)
- `crates/nono/src/trust/signing.rs` (Phase 45 source-migration target; verified zero upstream commits in v0.54.0..v0.57.0)
- `crates/nono/src/undo/snapshot.rs` (Phase 43 absorbed `66c69f86` symlink fix; verified zero upstream commits in v0.54.0..v0.57.0 — Phase 43 D-43-D1 invariant `validate_restore_target` per-file TOCTOU gate at `crates/nono/src/undo/snapshot.rs:610` remains untouched)

---

## Pattern Assignments

### 1. `crates/nono/src/sandbox/linux.rs`

- **Phase 48 plan(s):** 48-01 (C4 — primary), 48-04 (C5 — 1 polish commit), 48-06 (C7 — 1 musl Ioctl fix)
- **Role / data-flow:** Linux sandbox driver / fs+syscall enforcement; `#[cfg(target_os = "linux")]`-gated
- **Cluster + SHAs:** C4 lead `c2c6f2ca` introduces `pub struct LandlockScopePolicy` + `pub fn landlock_scope_policy(...)` + `pub struct DetectedAbi` (also touched by `a0222be2` af_unix pathname mediation); C5 polish `1122c315`; C7 musl `3cd22aa5`
- **Fork-side defense-in-depth invariants:**
  - Strictly allow-list per CLAUDE.md § Platform-Specific Notes — Landlock cannot express deny-within-allow. Cherry-pick MUST NOT introduce a deny-style code path on this file.
  - `#[cfg(target_os = "linux")]` gate must be preserved on every new pub item (cross-platform compile-fence).
- **Closest precedent (analog cherry-pick):**
  - **Phase 43 Plan 43-02 `43-02-PRE-CHERRY-PICK-AUDIT.md` § 4 "Fork-side divergence audit"** (lines 67-130) — pre-flight diff inspection methodology for a sandbox/snapshot-adjacent commit. Direct shape for the Plan 48-01 pre-flight artifact `48-01-PRE-CHERRY-PICK-AUDIT.md` (D-48-B2).
  - **Phase 43 Plan 43-01 / 43-01b summary pair** for sequencing a 9-commit single-plan cluster with mid-plan split escalation (D-48-B3 escalation pathway).
- **Pre-flight check (executor):**
  - `git show c2c6f2ca -- crates/nono/src/sandbox/linux.rs | grep '^+pub'` — confirm intra-cluster origin of `LandlockScopePolicy` / `DetectedAbi` / `landlock_scope_policy` (Phase 47 ledger row C4 already verified intra-cluster; re-confirm at cherry-pick time per `feedback_cluster_isolation_invalid` discipline).
  - `git log v0.54.0..v0.57.0 -- crates/nono/src/sandbox/linux.rs` — verify chronological cherry-pick order matches Phase 47 ledger row order (D-48-B1 + Claude's Discretion bullet).
  - `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — D-48-E4 mandatory after any Linux-cfg-gated cherry-pick lands.

---

### 2. `crates/nono/src/sandbox/mod.rs`

- **Phase 48 plan(s):** 48-01 (C4)
- **Role / data-flow:** Sandbox facade re-exporting platform-specific types
- **Cluster + SHAs:** C4 `c2c6f2ca` adds `pub use linux::{DetectedAbi, LandlockScopePolicy, detect_abi, landlock_scope_policy};`
- **Fork-side defense-in-depth invariants:**
  - Re-exports MUST stay intra-cluster per `feedback_cluster_isolation_invalid` — Phase 47 D-47-D2 scan PASSED on this file (re-exported symbols introduced in `c2c6f2ca` itself).
  - `is_supported()` + `support_info()` facade contract from CLAUDE.md § Architecture must not regress.
- **Closest precedent:**
  - **Phase 43 `43-PATTERNS.md` Pattern 1 § "Re-export delta inspection"** (lines 47-138 — re-export scan discipline for Cluster 2 `8b888a1c` which exposed `public_key_id_hex` and `sign_statement_bundle`). Apply same `git show <sha> -- <file> | grep '^+pub use'` rigor.
  - **Phase 47 ledger § Cross-cluster re-export deps detected** — empirical closure rationale verbatim already done; cherry-pick executor cites the audit conclusion.
- **Pre-flight check:**
  - `git show c2c6f2ca -- crates/nono/src/sandbox/mod.rs` — confirm only re-exports added; no logic moved between modules.

---

### 3. `crates/nono/src/lib.rs`

- **Phase 48 plan(s):** 48-01 (C4)
- **Role / data-flow:** Library public API surface; flat re-export layer per CLAUDE.md § Module Design
- **Cluster + SHAs:** C4 `c2c6f2ca` adds `pub use sandbox::{DetectedAbi, LandlockScopePolicy, detect_abi, is_wsl2, landlock_scope_policy};`
- **Fork-side defense-in-depth invariants:**
  - lib.rs is the public API contract — additions must NOT shadow or reorder existing fork-only re-exports (verify with `git show c2c6f2ca -- crates/nono/src/lib.rs` against fork HEAD).
  - Re-exports MUST stay intra-cluster (Phase 47 D-47-D2 PASSED).
- **Closest precedent:**
  - **Phase 43 Plan 43-03 `43-03-PER-SHA-AUDIT.md`** — per-sha audit shape for re-export deltas; serves as secondary template for Plan 48-01 if per-commit conflict prediction needs finer granularity than the cluster-wide pre-flight artifact (D-48-B2 vs D-48-B3 escalation).
- **Pre-flight check:**
  - Same as #2 — `git show c2c6f2ca -- crates/nono/src/lib.rs | grep '^+pub use'` then diff against current fork lib.rs re-export block.

---

### 4. `crates/nono/src/capability.rs`

- **Phase 48 plan(s):** 48-01 (C4 cfg-gated additions), 48-05 (C6 `abca959a` macOS localhost outbound)
- **Role / data-flow:** Builder for `CapabilitySet` / `FsCapability` / `AccessMode`
- **Cluster + SHAs:** C4 cluster (cfg-gated Linux additions via `c2c6f2ca` per Phase 47 ledger); C6 `abca959a` adds macOS `open_port 0 → localhost:*` behavior
- **Fork-side defense-in-depth invariants:**
  - Builder pattern: all paths canonicalized at grant time (CLAUDE.md § Key Design Decisions #3 + § Security Considerations) — additions must not introduce a path string-comparison footgun.
  - `#[must_use]` on critical Results must be preserved.
- **Closest precedent:**
  - **Phase 34 Plan 34-01 `34-01-CLI-CONSOLIDATION-SUMMARY.md`** — `CapabilitySet` builder additions from upstream `v0.41..v0.52` series; precedent for additive surface changes on this file with no logic regression.
- **Pre-flight check:**
  - `git show abca959a -- crates/nono/src/capability.rs` — confirm only macOS-cfg-gated additions; no cross-platform default behavior change.

---

### 5. `crates/nono-cli/src/cli.rs`

- **Phase 48 plan(s):** 48-01 (C4: c2c6f2ca, bbc652a0, 858ad009), 48-03 (C2: a8646d26, 2bed3565, 468d3813, 4e0e127a)
- **Role / data-flow:** Clap CLI definitions — most-touched cross-platform file this cycle (6 upstream commits per Phase 47 § Empirical cross-check File #5)
- **Cluster + SHAs:** C4 lands first (Wave 0 gate); C2 follows in Wave 1 with surface-disjoint flag additions
- **Fork-side defense-in-depth invariants:**
  - Existing fork-only CLI flags (e.g., Windows broker flags) MUST NOT be reordered/removed by cherry-picks.
  - `cli.rs` is the seam between user-facing UX and policy resolution — argument names cannot collide with fork's `nono why --self` and other already-shipped surfaces.
- **Closest precedent:**
  - **Phase 40 Plan 40-02 `40-02-CLI-ALLOW-VALIDATE-SUMMARY.md`** — cherry-pick precedent for `cli.rs` clap-arg additions from upstream UPST4.
  - **Phase 34 Plan 34-01 `34-01-CLI-CONSOLIDATION-SUMMARY.md`** — older precedent for resolving cli.rs surface collisions across multiple clusters; consult if Wave 1 C2 cherry-pick surfaces conflict pressure after Wave 0 C4 lands.
- **Pre-flight check:**
  - Wave-merge discipline: after Wave 0 close, `cd /c/Users/OMack/Nono` + verify pwd + branch BEFORE Wave 1 begins (`feedback_windows_worktree_cwd` recurrence prevention).
  - `git log v0.54.0..v0.57.0 -- crates/nono-cli/src/cli.rs` — confirm 6-commit set matches Phase 47 ledger.

---

### 6. `crates/nono-cli/src/profile/mod.rs`

- **Phase 48 plan(s):** 48-02 (C1 primary — 6 commits hit this file), 48-01 (C4 `a0222be2`), 48-05 (C6 `abca959a`)
- **Role / data-flow:** Profile schema + `From<ProfileDeserialize> for Profile` exhaustive match
- **Cluster + SHAs:** C1: 750f4653, 316c6a2c, 3d3d239a, c897c8cc, b3556139, 0015f348; C4: a0222be2; C6: abca959a
- **Fork-side defense-in-depth invariants (LOAD-BEARING):**
  - **Phase 36-01b extended `From<ProfileDeserialize> for Profile` exhaustively for `CommandsConfig`** — fork-side match arm at `crates/nono-cli/src/profile/mod.rs:2068` `impl From<ProfileDeserialize> for Profile`. Upstream cherry-picks introducing new profile fields MUST extend this exhaustive match or the build breaks. Fork-side test fixtures at lines 289-311 assert `filesystem.deny` and `commands.allow` survive the round-trip — re-run after every C1 + C4 cherry-pick.
  - **Phase 36-01c `override_deny → bypass_protection` atomic rename** — cherry-picks touching profile fields MUST honor the canonical name (`bypass_protection`, not `override_deny`).
- **Closest precedent:**
  - **Phase 34 Plan 34-04 `34-04-PATH-CANON-SCHEMA-SUMMARY.md` + Plan 34-04b `34-04b-FP-CANONICAL-SCHEMA-SUMMARY.md`** — paired worked examples for profile-schema cherry-picks that needed fork-side canonical-section integration.
  - **Phase 47 § Empirical cross-check File #4** — explicitly flags `profile/mod.rs` as the fork-divergence hot spot; the cited mitigation is "Phase 48 plan-phase MUST diff-inspect each commit's profile/mod.rs hunks against fork's CommandsConfig extensions" — this is the Plan 48-01 + 48-02 mandate.
- **Pre-flight check (MANDATORY per Phase 47 hot-spot finding):**
  - For each commit touching profile/mod.rs: `git show <sha> -- crates/nono-cli/src/profile/mod.rs | grep -E 'enum|struct|impl From'` — surface every new variant / field / arm.
  - Diff against fork's exhaustive arm list: `grep -nE 'CommandsConfig|FilesystemConfig|LegacyPolicyPatch|DeprecationCounter|bypass_protection' crates/nono-cli/src/profile/mod.rs`.
  - After cherry-pick: `cargo build -p nono-cli` (compile-time exhaustive-match enforcement); if `non_exhaustive` warning fires, extend fork arms in the same commit body (NOT a new commit — preserves D-19 trailer fidelity).

---

### 7. `crates/nono-cli/src/policy.rs`

- **Phase 48 plan(s):** 48-04 (C5 primary — 2 commits), 48-01 (C4 `a0222be2`)
- **Role / data-flow:** Policy group resolver — parses `policy.json`, expands groups, filters, resolves
- **Cluster + SHAs:** C5: e6215f8b, 4fa9f6a6 (Linux deny-overlap diagnostic quieting); C4: a0222be2 (af_unix pathname mediation)
- **Fork-side defense-in-depth invariants:**
  - Phase 41 Class D Linux deny-overlap regression test (REQ-TEST-HYG-01 closed via Phase 44 Plan 44-02 drain) — cherry-picks MUST NOT regress the deny-overlap protection; they only quiet the diagnostic.
  - Policy group resolver runs BEFORE sandbox apply — security-critical surface per CLAUDE.md § Permission Scope.
- **Closest precedent:**
  - **Phase 40 Plan 40-01 `40-01-PROXY-HARDENING-SUMMARY.md`** — precedent for policy-adjacent cherry-picks that compose additively with fork's existing canonical-validation surface.
- **Pre-flight check:**
  - `cargo test -p nono-cli --test linux_deny_overlap` (or equivalent Phase 41 Class D regression test) — run BEFORE and AFTER cherry-pick; must stay green.
  - `git show 4fa9f6a6 -- crates/nono-cli/src/policy.rs` — confirm pure diagnostic-quieting (no logic-path removal).

---

### 8. `crates/nono-cli/src/package_cmd.rs`

- **Phase 48 plan(s):** 48-08 (C9 — fork-preserve-upgrade-candidate)
- **Role / data-flow:** Package install command — manifest-driven artifact installation pipeline
- **Cluster + SHAs:** C9 `5f1c9c73` (introduces `installed_artifact_relative_path` helper + extends `.nono-trust.bundle` with `installed_path` + `sha256_digest`), `8d774753` (explicit artifact-install-path-conflict prevention)
- **Fork-side defense-in-depth invariants (security-critical):**
  - **D-32-15 verify-is-offline invariant** — cached `trusted_root.json` is read via plain JSON deserialization, not TUF re-verification. New `.nono-trust.bundle` schema fields from C9 MUST NOT break the offline verify path.
  - Phase 35 / 45 trust-bundle schema state must be compared against upstream's `installed_path` + `sha256_digest` extension BEFORE deciding upgrade-vs-D-20.
- **Closest precedent:**
  - **Phase 43 Plan 43-05 `43-05-DISPOSITION-RESOLUTION.md`** — fork-preserve disposition resolution artifact shape; D-48-C2 names `48-08-DISPOSITION-RESOLUTION.md` directly after this.
  - **Phase 43 Plan 43-05 `43-05-PLATFORM-DETECTION-FOUNDATION-PLAN.md`** lines 228 + 356 — D-20 manual-replay commit-body template (5-section body: `Upstream intent:` / `What was replayed:` / `What was NOT replayed and why:` / `Fork-only wiring preserved:` / `Upstream-replayed-from: <sha>`). Used if Plan 48-08 stays at D-20 (no upgrade).
  - **Phase 40 `40-05-FP-PROFILE-SAVE-SUMMARY.md` + `40-06-FP-PROXY-TLS-SUMMARY.md`** — older fork-preserve precedents.
- **Pre-flight check:**
  - **MANDATORY artifact:** produce `.planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md` BEFORE any code change.
  - `git show 5f1c9c73 -- crates/nono-cli/src/package_cmd.rs crates/nono/src/trust/policy.rs crates/nono/src/manifest.rs` — compare hunk-by-hunk against fork's HEAD on each file.
  - Schema collision check: grep fork's trust-bundle field set vs upstream's `installed_path` + `sha256_digest` additions.
  - D-32-15 invariant check: confirm offline-verify code path (`crates/nono-cli/src/setup.rs::trust_refresh` area — Phase 49 + 50 shipped state — D-48-E1 invariant: this file untouched by Phase 48) is structurally unaffected.
  - If upgrade decision: run D-47-D2 re-export scan on `5f1c9c73` + `8d774753` BEFORE cherry-pick.

---

### 9. `crates/nono/src/trust/policy.rs`

- **Phase 48 plan(s):** 48-08 (C9 diff-inspection target)
- **Role / data-flow:** Trust-bundle policy + signer-identity validation
- **Cluster + SHAs:** C9 `5f1c9c73` touches trust-bundle schema indirectly via `installed_path` + `sha256_digest` additions
- **Fork-side defense-in-depth invariants:**
  - `validate_bundle_relative_path` (or fork's analog) — defense-in-depth against unsafe `installed_path` values when reading from trust bundles.
  - D-32-15 offline-verify invariant (cached `trusted_root.json` plain JSON deserialization, no TUF re-verification).
- **Closest precedent:**
  - Same as #8 — Phase 43 Plan 43-05 + 43-06 disposition-resolution artifact convention.
- **Pre-flight check:**
  - `git show 5f1c9c73 -- crates/nono/src/trust/policy.rs` — surface every field added; compare against fork's Phase 35 + 45 schema state (`crates/nono/src/trust/policy.rs` HEAD).
  - Verify fork's `validate_bundle_relative_path` (or equivalent) is preserved if cherry-pick lands; verify it remains the gating call site if D-20 manual-replay is chosen.

---

### 10. `crates/nono/src/manifest.rs`

- **Phase 48 plan(s):** 48-08 (C9)
- **Role / data-flow:** Manifest module — fork's anchor for manifest-driven install pipeline
- **Cluster + SHAs:** C9 `5f1c9c73` (manifest-driven install pipeline)
- **Fork-side defense-in-depth invariants:**
  - Manifest schema additions from C9 (`installed_path`, `sha256_digest`) must compose with fork's Phase 35 / 45 manifest field set — NO field removals via cherry-pick.
- **Closest precedent:**
  - Same as #8 + #9.
- **Pre-flight check:**
  - `git show 5f1c9c73 -- crates/nono/src/manifest.rs` — confirm additive-only schema changes.

---

### 11. `crates/nono-cli/data/nono-profile.schema.json`

- **Phase 48 plan(s):** 48-07 (C8)
- **Role / data-flow:** Fork-shared JSON Schema for nono-profile validation (consumed by `jsonschema` crate per CLAUDE.md § Frameworks)
- **Cluster + SHAs:** C8 `57005737` extends `credential_format` field shape; `530306ee` review-fix follow-up
- **Fork-side defense-in-depth invariants:**
  - Schema extension MUST be additive — existing fork-side schema validators (under `crates/nono-cli/tests/` or `tests/integration/`) must continue to validate.
  - `credential_format` is now `Option<String>` semantically: omitted vs explicit `'Bearer {}'` vs explicit bare token — three distinct cases.
- **Closest precedent:**
  - **Phase 34 Plan 34-04 `34-04-PATH-CANON-SCHEMA-SUMMARY.md`** — precedent for `nono-profile.schema.json` cherry-picks with downstream fork-side validator coverage check.
- **Pre-flight check (D-48-D2 task):**
  - `grep -rE 'credential_format|nono-profile.schema.json' crates/nono-cli/tests/ tests/integration/` — verify existing coverage exercises the 3-case shape (omitted → default-resolution, explicit `'Bearer {}'`, explicit bare token).
  - If coverage gap: add focused fork-side regression test BEFORE cherry-pick lands (cleanup commit, NOT a D-19 trailer commit).
  - If coverage present: cherry-pick lands as-is.

---

### 12. `crates/nono-cli/src/exec_strategy.rs`

- **Phase 48 plan(s):** 48-03 (C2 — startup-timeout primary), 48-06 (C7 musl `3cd22aa5`)
- **Role / data-flow:** Fork+exec with signal forwarding (Direct/Monitor/Supervised) per CLAUDE.md § Architecture
- **Cluster + SHAs:** C2: a8646d26 (interactive-detection), 2bed3565 (--startup-timeout flag), 1be97978 (refactor); C7: 3cd22aa5 (musl Ioctl)
- **Fork-side defense-in-depth invariants:**
  - **`startup_prompt` references exist in fork** (verified: `crates/nono-cli/src/exec_strategy.rs:22, 1809, 1868-1869, 1909, 1915-1916, 2245, 2344-2345, 2425, 2588-2589, 3598` + `crates/nono-cli/src/main.rs:82 mod startup_prompt;`). Upstream `4e0e127a` REMOVES dead `startup_prompt.rs` infrastructure (193 lines). **D-48-D3 pre-flight cleanup MANDATORY:** the fork-side cleanup commit must remove these references BEFORE cherry-picking `4e0e127a`.
  - Execution strategies (Direct/Monitor/Supervised) are CLAUDE.md § Key Design Decisions #2 — cherry-picks MUST NOT regress the fork+wait process model.
- **Closest precedent:**
  - **Phase 40 Plan 40-03 `40-03-SCRUB-MODULE-SUMMARY.md`** — precedent for fork-side cleanup commit BEFORE cherry-picking an upstream removal commit; same shape D-48-D3 requires.
  - **Phase 34 Plan 34-08b `34-08b-LEARN-DEPRECATION-SUMMARY.md`** — older precedent for cleanup-then-cherry-pick sequencing on exec_strategy adjacent code.
- **Pre-flight check (MANDATORY per D-48-D3):**
  - `grep -rn 'startup_prompt' crates/` — record full reference set. Already known: ~13 references in exec_strategy.rs + 1 mod declaration in main.rs + (verify) 0 in exec_strategy_windows/ + 0 in nono-shell-broker/.
  - Author fork-side cleanup commit `cleanup(48-03): remove dead startup_prompt references ahead of upstream 4e0e127a absorption` — NO D-19 trailer; documented in plan SUMMARY.
  - THEN cherry-pick `4e0e127a` (the 193-deletion commit); the cherry-pick lands cleanly because the fork-side references are already gone.

---

### 13. `crates/nono-cli/src/pty_proxy.rs`

- **Phase 48 plan(s):** 48-06 (C7)
- **Role / data-flow:** PTY proxy — child-output forwarding + interactive terminal handling
- **Cluster + SHAs:** C7: 1f552106 (PR #881 child output trailing newline), 279af554 (bare ESC forwarding), 3d0ff87f (musl ioctl cast), 3cd22aa5 (musl Ioctl type mismatches)
- **Fork-side defense-in-depth invariants:**
  - Phase 27 PTY proxy work is upstream-equivalent per Phase 47 ledger C7 rationale; cherry-picks should compose cleanly with no fork-side conflict.
  - Cross-platform Unix-side surface; `#[cfg(unix)]` gate preserved.
- **Closest precedent:**
  - **Phase 34 Plan 34-01 `34-01-CLI-CONSOLIDATION-SUMMARY.md`** — older PTY-adjacent cherry-pick precedent.
- **Pre-flight check (D-48-D4 task):**
  - `cargo check --target x86_64-unknown-linux-musl` — close-gate addition; PARTIAL with `_environmental` if musl-cross-toolchain unavailable on Windows dev host (per `.planning/templates/cross-target-verify-checklist.md`).
  - `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin` per CLAUDE.md MUST/NEVER.

---

### 14. `crates/nono/CHANGELOG.md`

- **Phase 48 plan(s):** 48-09 (C3 release-ride)
- **Role / data-flow:** Fork CHANGELOG absorbing upstream CHANGELOG sections only (NOT version bumps)
- **Cluster + SHAs:** C3: 35f9fea2 (v0.55.0), b251c72f (v0.56.0), 10cec984 (v0.57.0) — three upstream CHANGELOG sections consolidated into ONE fork-side commit with THREE stacked D-19 trailer blocks per D-48-D1.
- **Fork-side defense-in-depth invariants:**
  - **Fork tracks its own version separately** — `crates/nono/Cargo.toml` + `Cargo.lock` version bumps DROPPED per release-ride convention (Phase 34/40/43 precedent commit `64b231a7 chore: release v0.52.0 (CHANGELOG-only; fork tracks own version)`).
  - All 5 workspace `Cargo.toml` files untouched by release-ride per `project_workspace_crates` memory (cross-binding lockstep prevention).
- **Closest precedent:**
  - **Phase 43 Plan 43-04 `43-04-RELEASE-RIDE-SUMMARY.md`** (278 lines) — directly inherited shape; D-48-D1 + D-48-E10 = D-43-D1 + D-43-E10 with the addition that Phase 48 consolidates 3 releases (Phase 43 consolidated only 1).
  - **Phase 40 `40-04-RELEASE-RIDE-SUMMARY.md`** — establishes the release-ride convention with precedent commit `64b231a7`.
  - **Phase 34 Plan 34-00 / 34-08b SUMMARYs** — earliest release-ride absorption precedents in the fork.
- **Pre-flight check:**
  - `git show 35f9fea2 -- crates/nono/Cargo.toml crates/nono/CHANGELOG.md` (and similarly for b251c72f, 10cec984) — record the dropped Cargo.toml hunk and the absorbed CHANGELOG hunk per release.
  - Verify fork's CHANGELOG path: `find . -name CHANGELOG.md -not -path '*/target/*'` — confirm `crates/nono/CHANGELOG.md` is the right anchor (matches upstream commit file walk).
  - Build the ONE commit body with THREE stacked D-19 trailer blocks (sample template under [Convention Pattern A](#a-d-19-6-line-cherry-pick-trailer-block-stacked-multi-sha-shape) below).

---

### 15. Fork-only Windows surface (D-48-E1 invariant)

- **Files in scope of invariant:** `crates/nono-cli/src/exec_strategy_windows/` (entire subdir), `crates/nono-shell-broker/` (entire crate), all `*_windows.rs` files across the workspace.
- **Phase 48 plan(s):** NONE direct. Phase 48 cherry-picks MUST NOT touch these files.
- **Fork-side defense-in-depth invariants:**
  - **D-17 / D-34-E1 / D-40-E1 / D-43-E1 / D-47-E5 cross-phase invariant:** Windows-only files structurally invariant unless the 4-condition addendum applies (required cross-platform struct field; cross-platform default factory only; ≤5 lines; documented in SUMMARY + STATE).
  - Phase 47 audit confirmed zero `windows-touch:yes` commits this cycle → trivially honored at cherry-pick level for all 9 clusters.
- **Carve-out:** D-48-D3 fork-side cleanup commit MAY touch fork-only Windows files (e.g., remove `startup_prompt` references from `exec_strategy_windows/`) UNDER "fork-side cleanup" classification rather than upstream-sync invariant. Pre-flight grep step `grep -rn 'startup_prompt' crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/` produces ZERO references per #12 above — so no Windows-side cleanup hunks expected this cycle.
- **Closest precedent:**
  - **Phase 43 § D-43-E1** (in `43-CONTEXT.md`) — most-recent expression of the invariant; Phase 48 D-48-E1 is identical.
- **Pre-flight check:**
  - After every cherry-pick lands: `git diff --name-only HEAD~1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '**/*_windows.rs' | wc -l` MUST equal `0`.

---

## Convention Patterns

These cross-cutting conventions apply across multiple Phase 48 plans. Each pattern points at the canonical worked example so the executor copies the shape verbatim.

### A. D-19 6-line cherry-pick trailer block (stacked multi-sha shape)

**Origin:** Phase 22 D-19 (single-sha form).
**Standardization:** Phase 40 lowercased `Upstream-author:` (was capitalized in earlier phases).
**Stacked multi-sha extension:** Phase 48 D-48-D1 (3 stacked trailers for C3 release-ride; same form for any consolidated commit).

**Canonical reference:** `.planning/templates/upstream-sync-quick.md` (mandatory scaffold). Phase 43 sample bodies at `.planning/phases/43-upst5-sync-execution/43-PATTERNS.md` line 72 (`Upstream-commit: 8b888a1c`) and line 99 (`Upstream-commit: 66c69f86`).

**Verbatim 6-line trailer shape** (per cherry-pick):
```
Upstream-commit: <40-char sha>
Upstream-author: <name> <email>
Upstream-date: <iso-8601>
Upstream-subject: <verbatim upstream subject>
Upstream-tag: <upstream tag containing this commit>
Upstream-categories: <drift-tool categories from JSON>
```

**Stacked shape for C3 / Plan 48-09** (one fork-side commit, three releases):
```
chore(48-09): absorb upstream v0.55.0..v0.57.0 CHANGELOG entries

[body absorbing all 3 upstream CHANGELOG sections in chronological order]

Upstream-commit: 35f9fea2...
Upstream-author: ... <...>
Upstream-date: ...
Upstream-subject: chore: release v0.55.0
Upstream-tag: v0.55.0
Upstream-categories: other

Upstream-commit: b251c72f...
Upstream-author: ... <...>
Upstream-date: ...
Upstream-subject: chore: release v0.56.0
Upstream-tag: v0.56.0
Upstream-categories: other

Upstream-commit: 10cec984...
Upstream-author: ... <...>
Upstream-date: ...
Upstream-subject: feat(release): release version 0.57.0
Upstream-tag: v0.57.0
Upstream-categories: other

Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
```

**Falsifiability:** `git log -1 --format=%B HEAD | grep -c '^Upstream-commit: '` must equal `1` (will-sync single-sha) OR `3` (Plan 48-09 stacked release-ride).

---

### B. D-20 manual-replay trailer (`Upstream-replayed-from:`)

**Origin:** Phase 43 D-43-C1 convention.
**Body shape:** 5-section body (`Upstream intent:` / `What was replayed:` / `What was NOT replayed and why:` / `Fork-only wiring preserved:` / `Upstream-replayed-from:`) — NO D-19 trailer block.
**Use cases in Phase 48:** Plan 48-08 (C9) if disposition stays D-20 (no upgrade to will-sync); Plan 48-01 (C4) escalation per D-48-B3 if pre-flight surfaces irreconcilable conflict on specific commits.

**Canonical reference:** `.planning/phases/43-upst5-sync-execution/43-05-PLATFORM-DETECTION-FOUNDATION-PLAN.md` line 228 (sample trailer) + line 356 (body template) + Phase 43 `43-PATTERNS.md` Pattern 2.

**Sample tail of D-20 commit body** (Phase 43 worked example):
```
Upstream-replayed-from: ce06bd59

Signed-off-by: <name> <email>
```

**Falsifiability:** `git log -1 --format=%B HEAD | grep -c '^Upstream-replayed-from: '` must equal `1`; `grep -c '^Upstream-commit: '` must equal `0`.

---

### C. Release-ride convention (Cargo.toml + Cargo.lock drops)

**Origin:** Phase 34 / 40 (precedent commit `64b231a7 chore: release v0.52.0 (CHANGELOG-only; fork tracks own version)`).
**Phase 43 inheritance:** D-43-D1.
**Phase 48 inheritance:** D-48-D1 + D-48-E10.

**Canonical reference:** `.planning/phases/40-upst4-sync-execution/40-04-RELEASE-RIDE-SUMMARY.md` (originating precedent); `.planning/phases/43-upst5-sync-execution/43-04-RELEASE-RIDE-SUMMARY.md` (most-recent worked example, 278 lines, single-release version).

**Convention:** For C3 release commits, fork DROPS upstream's `crates/nono/Cargo.toml` + `Cargo.lock` version bumps; absorbs ONLY upstream's `crates/nono/CHANGELOG.md` (or equivalent) entries. Fork tracks its own version separately. Plan 48-09 SUMMARY documents the reverted hunks explicitly.

**Falsifiability:** `git show HEAD --stat | grep -E 'Cargo\.(toml|lock)'` for the Plan 48-09 fork-side commit must return zero matches.

---

### D. Pre-flight diff-inspection artifact (per-plan)

**Origin:** Phase 43 D-43-C1 (introduced for fork-preserve clusters).
**Phase 48 extensions:**
- D-48-B2: extends to **will-sync-with-high-conflict-potential** (Plan 48-01 / C4 because the cluster touches ~29+ fork-shared files cumulatively including supervisor.rs Windows-arm intersection).
- D-48-C1: retains the original fork-preserve usage for Plan 48-08 / C9.

**Canonical references:**
- **Primary template:** `.planning/phases/43-upst5-sync-execution/43-02-PRE-CHERRY-PICK-AUDIT.md` (150 lines; 7-section body: Wave 0a closure → Upstream commit shape verification → Upstream diff shape → Fork-side divergence audit → Audit verdict → Acceptance summary). Plan 48-01 produces `48-01-PRE-CHERRY-PICK-AUDIT.md` mirroring this shape.
- **Secondary template:** `.planning/phases/43-upst5-sync-execution/43-03-PER-SHA-AUDIT.md` (38 lines; lightweight per-sha conflict prediction). Plan 48-01 may use this if per-commit granularity needed after the cluster-wide pre-flight artifact.

---

### E. Disposition-resolution artifact (fork-preserve)

**Origin:** Phase 43 D-43-C1 artifact convention.
**Canonical references:**
- `.planning/phases/43-upst5-sync-execution/43-05-DISPOSITION-RESOLUTION.md` (142 lines; 9-section body: Pre-flight prerequisites → D-43-C1 Q1-Q8 surface-overlap analysis → Trial cherry-pick evidence → Surface-semantics divergence evidence → Verdict → Cleanup → Frontmatter write → Implications for Task 2 → Threat-model alignment).
- `.planning/phases/43-upst5-sync-execution/43-06-DISPOSITION-RESOLUTION.md` (192 lines; same body shape, applied to second fork-preserve cluster).

**Plan 48-08 (C9) produces** `48-08-DISPOSITION-RESOLUTION.md` mirroring this shape. If the verdict is "upgrade to will-sync", the artifact also includes a D-47-D2 re-export scan subsection (per `Claude's Discretion` bullet in CONTEXT.md).

---

### F. Mid-plan split escalation (Plan 48-01 → 48-01a + 48-01b)

**Origin:** Phase 43 Plan 43-01 → 43-01b precedent (Cluster 2 mid-flight split when the Edition 2024 migration surfaced unmechanizable hunks).
**Phase 48 inheritance:** D-48-B3.

**Canonical references:**
- `.planning/phases/43-upst5-sync-execution/43-01-EDITION-2024-FOUNDATION-SUMMARY.md` (193 lines; documents the WHY of the mid-flight split).
- `.planning/phases/43-upst5-sync-execution/43-01b-EDITION-WORKSPACE-ONLY-SUMMARY.md` (317 lines; the split-second-half plan that delivered the mechanically-resolvable portion fork-authored).

**Phase 48 mechanism:** If Plan 48-01 pre-flight (artifact `48-01-PRE-CHERRY-PICK-AUDIT.md`) surfaces irreconcilable conflicts on specific C4 commits, planner splits into `48-01a-...-PLAN.md` (cleanly-resolvable commits, close-gate pass) + `48-01b-...-PLAN.md` (deferred commits with explicit per-commit resolution strategy: fork-authored partial-advancement per D-43-E1 4-condition addendum OR D-20 manual-replay with `Upstream-replayed-from:` trailer per Pattern B above). Cluster atomicity preserved at the cluster level; per-commit recovery via plan split. Phase 47 cluster dispositions remain IMMUTABLE — split is execution-level granularity refinement, NOT disposition change.

---

### G. Per-plan close-gate (Phase 34 D-34-D2 8-check format)

**Origin:** Phase 34 D-34-D2.
**Phase 48 inheritance:** D-48-E9.

**Canonical references:**
- Each Phase 43 plan has its own `43-NN-CLOSE-GATE.md` worked example (`.planning/phases/43-upst5-sync-execution/43-02-CLOSE-GATE.md`, `43-03-CLOSE-GATE.md`, `43-04-CLOSE-GATE.md`, etc.) — copy the 8-check table shape verbatim.

**8 checks (Phase 34 D-34-D2 standard):**
1. `cargo test --workspace`
2. `cargo clippy` on host
3. `cargo clippy --target x86_64-unknown-linux-gnu` (per Pattern J below — required for cfg-gated Unix code)
4. `cargo clippy --target x86_64-apple-darwin` (per Pattern J)
5. `cargo fmt --all -- --check`
6. Phase 15 smoke harness
7. `wfp_port_integration` (Windows lane)
8. `learn_windows_integration` (Windows lane)

**Plan-specific adjustments allowed** with explicit `skipped_gates_load_bearing` / `_environmental` categorization (Phase 40 anti-pattern #3):
- Plan 48-09 release-ride is CHANGELOG-only → may skip 7 + 8 with `_environmental` categorization.
- Plan 48-05 (macOS-only changes) → may skip 7 + 8 with `_environmental` categorization (Windows-only tests irrelevant).
- Plan 48-08 close-gate ADDS the D-48-C3 mandatory regression test (`tests/integration/offline_verify_extended_trust_bundle.rs` or similar).
- Plan 48-07 MAY add D-48-D2 schema test if coverage gap.
- Plan 48-06 ADDS D-48-D4 musl-target verification.

---

### H. Baseline-aware CI gate vs SHA `3f638dc6`

**Origin:** Phase 41 close (REQ-CI-FU-03 first reset to `13cc0628`); Phase 46 close re-reset to `3f638dc6` for v2.6.
**Phase 48 inheritance:** D-48-E3.

**Canonical reference:** `.planning/templates/upstream-sync-quick.md:102` — `**Current baseline SHA:** `3f638dc6`` (verified at lines 96-112 inclusive of the gate result interpretation rules).

**Convention** (verbatim from template lines 106-111):
- Lane green on baseline AND green on PR head → PASS.
- Lane green on baseline AND red on PR head → FAIL (real regression).
- Lane red on baseline AND red on PR head → PASS (carry-forward).
- Lane red on baseline AND green on PR head → PASS + IMPROVEMENT.

**Falsifiability:** Every Wave 1+ head commit MUST gate against `3f638dc6`; zero `success → failure` transitions allowed.

---

### I. PR umbrella convention (one per phase, per-plan section appends)

**Origin:** Phase 22 + 34 + 40 + 43 (memory `project_cross_fork_pr_pattern`).
**Phase 48 inheritance:** D-48-A4 + D-48-E6.

**Canonical references:**
- `.planning/phases/43-upst5-sync-execution/43-UMBRELLA-PR.txt` (the umbrella PR body assembled from per-plan contribution sections).
- Per-plan section examples: `43-01b-PR-SECTION.md`, `43-02-PR-SECTION.md`, `43-03-PR-SECTION.md`, `43-04-PR-SECTION.md`, `43-05-PR-SECTION.md`, `43-06-PR-SECTION.md`.

**Convention:**
- ONE upstream PR umbrella per Phase 48 (per memory `project_cross_fork_pr_pattern`).
- Umbrella opens AFTER Wave 0 (Plan 48-01) close per D-48-A4 — substantive content from day one.
- Per-plan feature branches feed into the umbrella PR body via per-plan `48-NN-PR-SECTION.md` artifacts.
- Each plan close appends its `48-NN-PR-SECTION.md` content to the umbrella PR body.
- Section template (per Phase 40 D-40-A1 + Phase 43 D-43-E6): subject + sha range + cluster disposition + key decisions.

---

### J. Cross-target clippy verification (CLAUDE.md MUST/NEVER)

**Origin:** Phase 41 Wave 5 close (memory `feedback_clippy_cross_target` — promoted from advisory to MUST/NEVER).
**Phase 48 inheritance:** D-48-E4.

**Canonical reference:** `.planning/templates/cross-target-verify-checklist.md` (77 lines; Phase 41 Class F template).

**Convention:**
```bash
cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace --target x86_64-apple-darwin       -- -D warnings -D clippy::unwrap_used
```

**Mandatory for Plans:** 48-01 (C4 Linux-only cfg-gated), 48-04 (C5 Linux policy polish), 48-05 (C6 macOS-only), 48-06 (C7 Linux/macOS PTY + musl).
**PARTIAL acceptable only if cross-toolchain unavailable on Windows dev host** — categorize as `skipped_gates_environmental` per Phase 40 anti-pattern #3; defer verification to live CI per template shape.

---

## Shared Pre-flight Discipline (applies to every plan)

1. **Wave-merge CWD discipline** (memory `feedback_windows_worktree_cwd`): after every wave-merge, `cd /c/Users/OMack/Nono` + verify `pwd` + branch BEFORE the next bash invocation. Observed twice in Phase 50; mandatory for Phase 48 wave-merge sequence (Wave 0 → 1 → 2 → 3).
2. **DCO sign-off** (CLAUDE.md § Commits): every cherry-pick / D-20 manual-replay / fork-side cleanup commit ends with `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` (from MEMORY `user_identity`).
3. **5-crate workspace discipline** (memory `project_workspace_crates`): no workspace-edit clusters this cycle per Phase 47 audit; D-48-E5 trivially honored. If any cherry-pick incidentally touches root `Cargo.toml`, the executor MUST also touch the 4 sibling crate `Cargo.toml` files + internal path-dep `version` pins (per 260514-0gu lesson).
4. **No `.unwrap()` / `.expect()`** (CLAUDE.md § Coding Standards): clippy `-D clippy::unwrap_used` enforced; any cherry-pick introducing `.unwrap()` MUST be fork-side-rewritten to `?` propagation in the same cherry-pick commit body.
5. **Re-export scan on every will-sync lead commit** (Phase 47 D-47-D1..D4 discipline): even though Phase 47 detected zero cross-cluster re-export deps this cycle, the discipline is preventive — re-run `git show <lead-sha> -- <touched-files> | grep '^+pub use'` at cherry-pick time to confirm the audit conclusion still holds. C9 upgrade-to-will-sync path MUST perform the scan on `5f1c9c73` + `8d774753` BEFORE cherry-pick (deferred from Phase 47 because C9 was fork-preserve at audit time).

---

## No Analog Found

**None.** Every Phase 48 integration-point file has a prior cherry-pick precedent in Phase 34/40/43. The phase is structurally cleaner than Phase 43 (zero `windows-touch:yes`, zero cross-cluster re-export deps, no foundation wrinkle like MSRV bump + Edition 2024 migration), so the precedent surface is more than sufficient.

---

## Metadata

**Analog search scope:**
- `.planning/phases/43-upst5-sync-execution/` (PRIMARY — all 43 artifacts walked; 43-01..43-06 plan/summary pairs + 43-02-PRE-CHERRY-PICK-AUDIT.md + 43-03-PER-SHA-AUDIT.md + 43-05/43-06-DISPOSITION-RESOLUTION.md + 43-04-RELEASE-RIDE-SUMMARY.md + 43-PATTERNS.md + 43-UMBRELLA-PR.txt + per-plan PR-SECTION.md set + per-plan CLOSE-GATE.md set)
- `.planning/phases/40-upst4-sync-execution/` (SECONDARY — 40-01..40-06 SUMMARYs for transitive precedents)
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/` (TERTIARY — 34-00..34-10 SUMMARYs for oldest cherry-pick precedents)
- `.planning/templates/upstream-sync-quick.md` (276 lines; Convention Pattern A + H source)
- `.planning/templates/cross-target-verify-checklist.md` (77 lines; Convention Pattern J source)
- Fork source files: `crates/nono-cli/src/profile/mod.rs` (verified exhaustive match at line 2068); `crates/nono-cli/src/exec_strategy.rs` + `main.rs` (verified ~13 `startup_prompt` references for D-48-D3 cleanup); `crates/nono/src/undo/snapshot.rs:610` (verified `validate_restore_target` invariant); `crates/nono-cli/src/exec_strategy_windows/` + `crates/nono-shell-broker/` (verified zero `startup_prompt` references).
- Phase 47 ledger: `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md` (BINDING IMMUTABLE INPUT; cluster + commit shas extracted per integration-point file).
- Phase 47 hand-off: `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md` (wave hints + cluster sequencing rationale).

**Files scanned:** ~50 (Phase 43 artifact set + Phase 47 audit + fork source spot-checks).
**Pattern extraction date:** 2026-05-24.
**Phase 48 wave structure consumed:** D-48-A2 (Wave 0 = 48-01 solo; Wave 1 = 48-02 || 48-03 parallel; Wave 2 = 48-04 || 48-05 || 48-06 || 48-07 || 48-08 5-way parallel; Wave 3 = 48-09 solo).
**Phase 48 cluster dispositions consumed (per Phase 47 ledger, IMMUTABLE):** 8 will-sync (C1, C2, C3, C4, C5, C6, C7, C8) + 1 fork-preserve-upgrade-candidate (C9).

---

*Phase: 48-upst6-sync-execution*
*Pattern map authored: 2026-05-24*
