# Phase 55: UPST7 Cherry-pick Wave - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 55 executes the **Phase-55-routed `will-sync` cluster set** from Phase 54's
`54-DIVERGENCE-LEDGER.md` (the immutable audit-of-record) against upstream `v0.57.0..v0.59.0`.
Mirrors the Phase 34 + 40 + 43 + 48 sync-execution shape: per-commit cherry-pick of will-sync
clusters with the verbatim D-19 6-line trailer; D-20 manual-replay (`Upstream-replayed-from:`) where
fork divergence prevents a clean pick. Single requirement: **REQ-UPST7-02**.

**The Phase-55 cluster set (from the ledger, NOT the literal SC1 prose):**
- **C4** — proxy 502 hardening (`connect.rs`; 2 commits)
- **C7** — profile system: JSONC parsing + `target_binary` field + opencode pack relocation + chained-if-let refactor + review fixes (5 commits)
- **C9** — pack-update-hint robustness: atomic state writes + detached-process refresh (2 commits)
- **C10** — diagnostic/output/denial polish: rfind access-mode split, canonical denial-path precompute, bold-only-path footer, suppressed-denial annotations (4 commits)
- **C11** — timeout constants: centralized `timeouts.rs` + configurable user-facing timeouts + overflow-check tightening + formatting (3 commits)
- **C12** — policy test: lock `ENV_LOCK` in `test_all_groups_no_deny_within_allow_overlap` (1 commit, test-only)
- **C13 (split)** — sigstore dep bump 0.8.0: `Cargo.toml` bump is will-sync; `scrub.rs` is verify-then-port against the fork's Phase-49 trust-root surface (1 commit, conditional handling per D-55-02)

**In scope:**
- Per-commit cherry-pick of C4, C7, C9, C10, C11, C12 with the D-19 trailer block.
- C13 split handling: diff-inspection-first artifact, then port Cargo bump + `scrub.rs` (or D-20 replay / defer `scrub.rs`) per the resolution.
- Schema-collision checks (SC3): C7's profile schema changes (JSONC, `target_binary`) diff-inspected against the fork's `nono-profile.schema.json` / `policy.json` canonical sections (Phase 36 canonical-sections surface).
- Amend `REQUIREMENTS.md` REQ-UPST7-02 + `ROADMAP.md` Phase 55 SC1 to match the audit-of-record (drop phantom java-dev, add C9/C12/C13) per D-55-01.
- Hold all Phase 55 code off `main` until v0.58.0 is tagged + signed; land as v0.59.0/next per D-55-03.
- One plan per cluster (~7 plans); surface-disjoint clusters run as parallel waves per D-55-04.
- `make ci` / Windows `cargo test --workspace` green relative to the Phase 54 baseline SHA (SC4); cross-target clippy per CLAUDE.md MUST for any cfg-gated Unix code touched.

**Out of scope (route elsewhere or explicitly defer):**
- **Re-litigation of Phase 54 dispositions / cluster boundaries** — the `54-DIVERGENCE-LEDGER.md` is immutable input. Plan-phase may refine execution granularity and exercise C13's diff-inspection upgrade authority, but cannot re-cluster or change dispositions.
- **C3 (allow_domain), C5 (TLS-intercept ordering)** → Phase 56 (REQ-NET-01). The C5 `proxy_runtime.rs` 12-line filter-allowlist snippet rides WITH Phase 56's `partition_allow_domain` absorption (function-call prereq C5→C3), not here.
- **C6 (`bw://` credential source)** → Phase 57 (REQ-CRED-01).
- **C8 (session hooks)** → Phase 58 (REQ-HOOK-01; Windows ADR required).
- **C2 (supervisor named-socket IPC)** → Phase 59 (REQ-IPC-01; Windows AIPC fork-preserve).
- **C1 (release commits), C14 (macOS-only sandbox fixes)** → won't-sync (CHANGELOG-ride only / macOS-N/A).
- **`java-dev` / `java_runtime` profile** — NO commits in `v0.57.0..v0.59.0` (Phase 54 empirical cross-check walked `platform.rs` → 0 commits). It is gap-analysis enumeration drift, not an audited cluster. Not hunted from a later range here (that is UPST8 scope, `v0.60.0..v0.61.1`).
- **rcgen 0.13.2→0.14.8 bump (`8e78daf`)** — won't-sync; lives in the absent `tls_intercept/` module.
- **Closure/replay of fork-only Windows seams** (`exec_strategy_windows/`, `nono-shell-broker/`, Phase 28/32/49/50 trust surfaces) — D-43-E1 invariant; stay byte-identical.

</domain>

<decisions>
## Implementation Decisions

### Scope reconciliation — REQ/SC vs audit routing (Area A — discussed)

- **D-55-01: Execute the ledger's Phase-55 routing AND amend the planning artifacts to match the audit-of-record.**
  The cherry-pick set is the ledger's Phase-55-routed clusters: **C4, C7, C9, C10, C11, C12, + C13 (split)** — NOT the literal SC1/REQ-UPST7-02 prose.
  - **Drop the phantom `java-dev`/`java_runtime` item** from REQ-UPST7-02 + ROADMAP Phase 55 SC1: the Phase 54 empirical cross-check walked `crates/nono-cli/src/platform.rs` over `v0.57.0..v0.59.0` and found **0 commits**. The item descends from the broader 260527-sgo gap analysis, not the audited range. Mark it explicitly as out-of-range / N/A (UPST8 territory if it ever appears).
  - **Add the omitted clusters** C9 (pack-update-hint), C12 (ENV_LOCK test), C13 (sigstore bump) to the REQ/SC enumeration so the written acceptance criteria match what is actually absorbed.
  - The amendment is a planning-artifact edit (REQUIREMENTS.md + ROADMAP.md), made under this phase, citing the ledger as authority. The `54-DIVERGENCE-LEDGER.md` itself stays immutable (audit-of-record).
  - **User explicitly rejected** "java-dev N/A in SUMMARY only" (leaves REQ/SC text divergent from reality) and "hunt java-dev in a later range now" (scope creep; violates audit immutability — that is UPST8 `v0.60.0..v0.61.1`).

### C13 sigstore 0.8.0 split handling (Area B — discussed)

- **D-55-02: Diff-inspection-first, then port+verify — mirror the Phase 48 C9 fork-preserve pattern.**
  The C13 plan opens with a structured diff-inspection artifact comparing upstream `e581569`'s `crates/nono/src/scrub.rs` change against the fork's Phase-49 trust-root surface (`--from-file`, fixture cadence, `trusted_root.json`, and the D-32-15 verify-is-offline invariant).
  - The `crates/nono/Cargo.toml` bump to sigstore 0.8.0 is a will-sync straight port; it ripples `Cargo.lock` workspace-wide (5 crates — `project_workspace_crates`), so the plan updates the lockfile + any internal path-dep `version` pins as needed.
  - If diff-inspection shows **no collision** with the fork's Phase-49 trust-root surface and no D-32-15 offline-verify regression → port both the Cargo bump and `scrub.rs` with the D-19 trailer.
  - If a collision is detected → **D-20 manual-replay** the `scrub.rs` intent (`Upstream-replayed-from:` trailer) or defer the `scrub.rs` hunk with a documented rationale; the Cargo bump still lands.
  - Resolution captured in a separate `55-NN-C13-DISPOSITION-RESOLUTION.md` artifact (Phase 48 D-48-C2 naming convention).
  - **User explicitly rejected** "Cargo bump only; defer scrub.rs unconditionally" (skips the verification opportunity) and "defer entire C13 to UPST8/post-release" (leaves the dependency stale; the conservative diff-inspection default already protects the signing surface without blanket deferral).

### Release-scope timing (Area C — discussed)

- **D-55-03: Execute now on a held feature branch; do NOT merge to `main` until v0.58.0 is tagged + signed; then land as v0.59.0/next.**
  Honors the Phase 54 release-scope guard (`quick-260604-nue`): Phase 55 changes shipped binaries and would otherwise ride into the signed v2.9 release. Execution (cherry-picks, plans, close-gates, umbrella PR prep) proceeds now on the feature branch so the work is not stalled on the Azure signing cert; the **merge-to-main gate** is the v0.58.0 tag.
  - Plan-phase / execute-phase must treat "merge to main" as blocked-until-v0.58.0; the held branch accumulates the full wave.
  - **User explicitly rejected** "block Phase 55 until v0.58.0 ships first" (stalls the wave entirely on the cert) and "proceed straight to main now" (rejects the guard; cherry-picks would ride into the signed v2.9 release).

### Plan slicing (Area D — discussed)

- **D-55-04: One plan per cluster (~7 plans).** `55-NN-{CLUSTER-THEME}-PLAN.md`, mirroring Phase 40 D-40-A1 / Phase 43 D-43-A2 / Phase 48 D-48-A1. Maximum per-cluster traceability; rollback granularity = one cluster. Suggested names (planner may refine):
  - 55-01-PROXY-502-HARDENING (C4, 2 commits)
  - 55-02-PROFILE-JSONC-TARGET-BINARY (C7, 5 commits)
  - 55-03-PACK-HINT-ROBUSTNESS (C9, 2 commits)
  - 55-04-DIAGNOSTIC-DENIAL-POLISH (C10, 4 commits)
  - 55-05-TIMEOUT-CONSTANTS (C11, 3 commits)
  - 55-06-POLICY-ENV-LOCK-TEST (C12, 1 commit)
  - 55-07-SIGSTORE-BUMP (C13 split, opens with the D-55-02 diff-inspection artifact)
  - Surface-disjoint clusters run as parallel waves; the planner runs the surface-overlap analysis at plan-open (known overlaps: **C7 + C12 both touch `policy.rs`**; **C10 may touch `exec_strategy.rs` + `diagnostic.rs`** while **C11 touches `pty_proxy.rs` / `session_commands.rs`** — verify intersection before parallelizing).
  - **User explicitly rejected** "consolidate the polish clusters (C9/C10/C12 into a POLISH-BATCH)" and "defer slicing entirely to the planner" (one-per-cluster is the locked default; the planner still owns wave grouping + overlap analysis within it).

### Carry-Forward From Phase 22 / 33 / 34 / 40 / 43 / 48 (binding — locked, not for re-discussion)

- **D-55-E1 (= D-48-E1 / Phase 43 D-43-E1 / Phase 22 D-17):** Windows-only files structurally invariant. Phase 55 cherry-picks MUST NOT touch `*_windows.rs`, `crates/nono-cli/src/exec_strategy_windows/`, or `crates/nono-shell-broker/` unless the D-43-E1 4-condition addendum applies (cross-platform struct field; default factory only; ≤5 lines; documented in SUMMARY + STATE). The ledger flags **zero windows-touch:yes** for the Phase-55 cluster set (C2 + C8 are the only windows-touch clusters, both routed away to 58/59) — trivially honored.
- **D-55-E2 (= D-48-E2 / Phase 40 standardization):** Every cherry-picked commit carries the verbatim 6-line trailer (lowercase `Upstream-author:`):
  ```
  Upstream-commit: <40-char sha>
  Upstream-author: <name> <email>
  Upstream-date: <iso-8601>
  Upstream-subject: <verbatim upstream subject>
  Upstream-tag: <upstream tag containing this commit>
  Upstream-categories: <drift-tool categories from JSON>
  ```
  D-20 manual-replay commits carry `Upstream-replayed-from: <sha>`. Falsifiable via `git log --format=%B | grep -c "^Upstream-commit:"`.
- **D-55-E3 (= D-48-E4 / CLAUDE.md MUST/NEVER / `feedback_clippy_cross_target`):** Cross-target clippy required for any cfg-gated Unix code touched, from the Windows host: `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`, both `-D warnings -D clippy::unwrap_used`, per `.planning/templates/cross-target-verify-checklist.md`. PARTIAL allowed only if the cross-toolchain is unavailable on the dev host (categorize as `skipped_gates_environmental`). C11 (`pty_proxy.rs`, `session_commands.rs`) and C10 (`exec_strategy.rs`) are the most likely to intersect cfg-gated surface — planner verifies per cluster.
- **D-55-E4 (= D-48-E3 / `.planning/templates/upstream-sync-quick.md`):** Baseline-aware CI gate against the **Phase 54 baseline SHA** (SC4). Zero load-bearing `success → failure` transitions on every wave head commit; categorize transitions (green→green PASS, green→red FAIL, red→red carry-forward, red→green improvement) and skips (`_load_bearing` vs `_environmental`).
- **D-55-E5 (= D-48-E5 / `project_workspace_crates`):** nono workspace has **5 crates** (root + nono, nono-cli, nono-proxy, nono-shell-broker, bindings/c). The C13 sigstore bump (workspace-dep) MUST update all relevant `Cargo.toml` files + `Cargo.lock` + internal path-dep version pins as needed.
- **D-55-E6 (= D-48-E6 / `project_cross_fork_pr_pattern`):** Single upstream umbrella PR per phase. Phase 55 opens one umbrella; per-plan contribution sections append to the PR body. (Note: Phase 55 is inbound absorption FROM upstream; the umbrella is the fork's standing contribute-back vehicle, kept consistent with prior UPST phases.)
- **D-55-E7 (= D-48-E10 / release-ride convention; precedent `64b231a7`):** C1 release commits (won't-sync) — if any CHANGELOG ride is desired, fork DROPS upstream `Cargo.toml` + `Cargo.lock` version bumps and absorbs only CHANGELOG entries. Not a Phase-55 will-sync cluster; CHANGELOG-ride only at most.
- **D-55-E8 (= Phase 54 ADR review outcome (a) / Phase 33 ADR `Accepted`):** Phase 33 ADR Option A `continue` stays Accepted (re-confirmed at Phase 54). Phase 55 does NOT supersede or amend by default.

### Claude's Discretion
- **Plan numbering + cluster-theme names** — D-55-04 suggests `55-01..55-07`; planner may refine names for clarity.
- **Wave grouping within one-plan-per-cluster** — planner runs the surface-overlap analysis at plan-open and groups surface-disjoint plans into parallel waves; serializes same-surface plans (C7↔C12 on `policy.rs`; C10↔C11 if `exec_strategy.rs`/`diagnostic.rs` overlap surfaces).
- **C13 diff-inspection artifact name + upgrade-or-replay outcome** — D-55-02 locks the diff-inspection-first method + `55-NN-C13-DISPOSITION-RESOLUTION.md` shape; planner/executor records the upgrade-or-replay decision with rationale.
- **Per-plan close-gate composition** — inherits the Phase 34 D-34-D2 8-check format; planner may add/skip individual checks per cluster with explicit `_load_bearing`/`_environmental` categorization (e.g., C12 test-only and C13 dep-bump trivially pass most code-quality gates).
- **Exact mechanism of the REQ/SC amendment** (D-55-01) — whether as a standalone planning-docs commit at phase open or folded into the first plan; planner decides.
- **Cherry-pick chronological order within each cluster** — planner verifies via `git log v0.57.0..v0.59.0 -- <cluster files>` at plan-open and records canonical order (chronological wins over the ledger's semantic row-order).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 55 scope sources
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` — **BINDING IMMUTABLE INPUT.** Cluster Summary (14 clusters / 40 commits), per-cluster dispositions, `windows-touch` column, ADR review outcome (a) confirm, Empirical cross-check (5 files; java-dev/platform.rs = 0 commits), Cross-cluster re-export deps (C5→C3 function-call prereq), TLS-intercept clean-apply assessment. Phase 55 plans MUST cite specific ledger rows; cannot re-relitigate.
- `.planning/phases/54-upst7-audit/54-01-SUMMARY.md` — Phase 54 close hand-off; the release-scope guard for D-55-03 (hold Phase 55 off main until v0.58.0 tag, per `quick-260604-nue`); java-dev "0 commits in range" confirmation; v0.60.0..v0.61.1 deferred to UPST8.
- `.planning/REQUIREMENTS.md` § REQ-UPST7-02 — acceptance criteria (D-19 cherry-picks + D-20 replays per audit dispositions; schema-collision checks; D-43-E1 invariant). **Amended by D-55-01** (drop java-dev, add C9/C12/C13).
- `.planning/ROADMAP.md` § Phase 55 — Goal, depends-on Phase 54, SC1–SC4. SC1 enumeration **amended by D-55-01**.

### Sync-execution mechanics (MANDATORY scaffold — Phase 55 inherits verbatim)
- `.planning/templates/upstream-sync-quick.md` — D-19 6-line trailer block (lowercase `Upstream-author:`); baseline-aware CI gate + lane-transition categorization rules.
- `.planning/templates/cross-target-verify-checklist.md` — Phase 41 Class F template; MANDATORY for every Phase 55 plan touching cfg-gated Unix code (cross-target Linux + macOS clippy from the Windows host).

### Execution-shape template (PRIMARY precedent — Phase 55 mirrors with a smaller, zero-windows-touch cluster set)
- `.planning/phases/48-upst6-sync-execution/48-CONTEXT.md` — D-48-A1..E13 decision IDs. Phase 55 D-55-04 inherits D-48-A1 (one plan per cluster); D-55-02 mirrors D-48-C1 (diff-inspection-first with upgrade authority for the split cluster); D-55-E1..E8 inherit the cross-phase invariants. Phase 48 user-rejected options are informative, not binding (different cluster set).
- `.planning/phases/43-upst5-sync-execution/43-CONTEXT.md` — D-43-A1..E10 roots (surface-disjoint parallel waves, fork-preserve diff-inspection, release-ride, Windows-only-files invariant). Transitive precedent via Phase 48.
- `.planning/phases/40-upst4-sync-execution/40-CONTEXT.md` + `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-CONTEXT.md` — execution-shape ROOTS (per-cluster slicing, foundation gate, per-plan close-gate D-34-D2 8-check format, release-ride precedent commit `64b231a7`).

### Canonical-sections context (binding for C7 SC3 schema-collision check)
- `crates/nono-cli/data/nono-profile.schema.json` + `crates/nono-cli/data/policy.json` — the fork's canonical-section sources; C7's JSONC + `target_binary` schema changes diff-inspect against these (SC3).
- `.planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md` — fork's exhaustive `From<ProfileDeserialize> for Profile` match; C7 profile/mod.rs commits diff-inspect against it.
- `.planning/phases/36-upst3-deep-closure/36-01c-OVERRIDE-DENY-RENAME-SUMMARY.md` — `override_deny → bypass_protection` canonical rename; honor if profile fields are touched.

### Phase-49 trust-root surface (binding for C13 D-55-02 diff-inspection)
- `crates/nono/src/scrub.rs` — the upstream `e581569` change target; diff-inspected here.
- `.planning/phases/49-sigstore-trust-root-poc-resilience-from-file-flag-release-as/49-CONTEXT.md` — fork's `--from-file` / fixture-cadence / `trusted_root.json` POC trust-root surface; D-55-02 compares against it.
- D-32-15 verify-is-offline invariant — cached `trusted_root.json` read via plain JSON deserialization (not TUF re-verification); the C13 `scrub.rs` change MUST NOT regress it.

### TLS-intercept boundary awareness (Phase 55 must NOT touch; flows to Phase 56)
- `54-DIVERGENCE-LEDGER.md` § TLS-intercept clean-apply assessment — the C5 `proxy_runtime.rs` 12-line filter-allowlist snippet (calls `partition_allow_domain` from C3) is a Phase-56 rider, NOT a Phase-55 item. The fork's `RouteStore`/`CredentialStore` decoupling (`route.rs` + `credential.rs`) already satisfies endpoint-before-credential ordering — fork-preserve; do not import upstream `tls_intercept/`.

### Strategic ADR (LOCKED — Phase 55 inherits Phase 54 verdict (a))
- `docs/architecture/upstream-parity-strategy.md` — Phase 33 ADR Option A `continue`, `Status: Accepted`, re-confirmed at Phase 54. D-19/D-20 conventions + fork-preserve/won't-sync handling defined here.

### Drift-tool infrastructure (Phase 55 references audit output; does not re-run the tool)
- `scripts/check-upstream-drift.sh` + `scripts/check-upstream-drift.ps1` (sha `0834aa664fbaf4c5e41af5debece292992211559`) — produced the Phase 54 ledger; Phase 55 consumes the ledger, doesn't re-run.

### Coding & security standards
- `CLAUDE.md` § Coding Standards — no `.unwrap()`/`.expect()`, DCO `Signed-off-by:` on every commit, `#[must_use]` on critical Results, env-var save/restore in tests (C12 ENV_LOCK aligns), 5-crate workspace bump discipline.
- `CLAUDE.md` § Cross-target clippy verification — MUST/NEVER; D-55-E3 inheritance.

### Operative memory entries (load-bearing)
- `feedback_clippy_cross_target` — cross-target clippy enforced via CLAUDE.md MUST/NEVER (D-55-E3).
- `project_workspace_crates` — 5 crates, not 3 (D-55-E5; C13 lockfile ripple).
- `project_cross_fork_pr_pattern` — ONE umbrella PR to upstream (D-55-E6).
- `feedback_cluster_isolation_invalid` — DIVERGENCE-LEDGER cluster isolation can be empirically false; Phase 54 re-export scan came back clean for `pub use` but surfaced the C5→C3 function-call prereq (Phase-56 concern, not Phase-55).
- `feedback_windows_worktree_cwd` — after every wave-merge, `cd /c/Users/OMack/Nono` and verify pwd + branch before next bash (Phase 55 wave-merges observe this).

### Upstream source (git-resolvable from `upstream` remote)
- `upstream` = `https://github.com/always-further/nono.git`; range `v0.57.0..v0.59.0`; upstream HEAD at audit `48d39f3635f339e439d43869f8c98bc1db9b6dc1` (re-fetch 2026-06-04). Phase 55 cherry-picks from this history.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`.planning/templates/upstream-sync-quick.md`** — MANDATORY D-19 trailer + CI-gate scaffold; Phase 55 plans inherit verbatim.
- **`.planning/templates/cross-target-verify-checklist.md`** — cross-target clippy template for cfg-gated Unix code (C10/C11 most likely to intersect).
- **Phase 48 sync-execution worked example** — `.planning/phases/48-upst6-sync-execution/` (9 cluster plans, diff-inspection-first for the split cluster, release-ride). Phase 55 is a structurally simpler subset: 6 will-sync + 1 split, zero windows-touch, one diff-inspection cluster (C13).
- **Fork's `RouteStore`/`CredentialStore` decoupling** (`crates/nono-proxy/src/route.rs` + `credential.rs`) — already satisfies the C5 endpoint-before-credential ordering; relevant context for NOT importing upstream `tls_intercept/`.

### Established Patterns
- **One plan per cluster** (Phase 40/43/48 → Phase 55 D-55-04).
- **Surface-disjoint parallel waves** — same-surface plans serialize (C7↔C12 on `policy.rs`).
- **Diff-inspection-first for the split cluster** — C13 opens with a `55-NN-C13-DISPOSITION-RESOLUTION.md` artifact + upgrade-or-replay authority (Phase 48 D-48-C1 pattern).
- **D-19 6-line trailer / D-20 `Upstream-replayed-from:`** — verbatim, lowercase `Upstream-author:`.
- **Baseline-aware CI gate** — against the Phase 54 baseline SHA; categorize transitions + skips.
- **Held-branch / merge-after-tag** — Phase 55 code accumulates on a feature branch; merge-to-main gated on v0.58.0 (D-55-03).

### Integration Points
- **`.planning/phases/55-upst7-cherry-pick-wave/` directory** — plans land as `55-NN-CLUSTER-THEME-PLAN.md` + SUMMARY pairs; C13 disposition-resolution artifact under `55-NN-C13-DISPOSITION-RESOLUTION.md`.
- **`crates/nono-cli/src/profile/` + `data/nono-profile.schema.json` + `policy.json`** — C7 JSONC/`target_binary`/opencode landing surface + SC3 schema-collision check point.
- **`crates/nono-proxy/src/connect.rs`** — C4 502-hardening landing surface.
- **`crates/nono-cli/src/` timeouts/diagnostic/exec_strategy + `crates/nono/src/diagnostic.rs`** — C10/C11 landing surfaces (cfg-gated intersection check).
- **`crates/nono/src/keystore.rs` + pack-hint state surface** — C9 landing surface.
- **`crates/nono/Cargo.toml` + `Cargo.lock` (5-crate workspace) + `crates/nono/src/scrub.rs`** — C13 landing surfaces.
- **`.planning/REQUIREMENTS.md` + `.planning/ROADMAP.md`** — D-55-01 amendment targets.

</code_context>

<specifics>
## Specific Ideas

- The maintainer wants the **written acceptance criteria to track the audit-of-record**, not the older gap-analysis prose — hence the active REQ/SC amendment (D-55-01) rather than a SUMMARY-only footnote.
- The **signing/trust surface is treated as risk-sensitive** during this wave (pending v2.9 signed release): C13's `scrub.rs` gets diff-inspection-first verification rather than a blind pick, and the whole wave is **held off `main` until v0.58.0 is tagged**.
- C13's conservative posture is "verify and port if clean" — not "defer by default" and not "blind port." The upgrade authority (port both Cargo + scrub.rs) is available only when the diff-inspection clears the Phase-49 surface + D-32-15 offline-verify invariant.

</specifics>

<deferred>
## Deferred Ideas

- **`java-dev` / `java_runtime` profile with Windows JDK paths** — has NO source commit in `v0.57.0..v0.59.0` (Phase 54 confirmed `platform.rs` = 0 commits). Not absorbed here; if upstream ships it later it lands in UPST8 (`v0.60.0..v0.61.1`+). Removed from the REQ/SC enumeration per D-55-01.
- **C3 (allow_domain path+method) + C5 (TLS-intercept ordering rider)** → Phase 56 (REQ-NET-01). Absorb C3 BEFORE the C5 `proxy_runtime.rs` snippet (function-call prereq).
- **C6 (`bw://` Bitwarden credential source)** → Phase 57 (REQ-CRED-01).
- **C8 (session lifecycle hooks)** → Phase 58 (REQ-HOOK-01; Windows-equivalent ADR; highest-risk UPST7 phase).
- **C2 (supervisor named-socket IPC keep-alive/timeout)** → Phase 59 (REQ-IPC-01; cross-platform-core ports, Windows AIPC fork-preserve).
- **rcgen 0.13.2→0.14.8 (`8e78daf`)** — won't-sync; lives in the absent `tls_intercept/` module.
- **UPST8 audit** (`v0.60.0..v0.61.1`, growing) — fires after Phase 55 closes, per the Phase 33 cadence rule.

</deferred>

---

*Phase: 55-upst7-cherry-pick-wave*
*Context gathered: 2026-06-04*
