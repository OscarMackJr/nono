---
phase: 55-upst7-cherry-pick-wave
plan: "07"
subsystem: sigstore-deps
tags: [sigstore, cargo-bump, scrub, c13, upst7, security-sensitive]
dependency_graph:
  requires: [55-02, 55-05, 55-06]
  provides: [sigstore-0.8.0-bump, scrub-cow-deref-port]
  affects: [Cargo.lock, crates/nono/Cargo.toml, crates/nono-cli/Cargo.toml, crates/nono/src/scrub.rs]
tech_stack:
  added: []
  patterns: [D-19-trailer, diff-inspection-first, CLEAR-verdict-port]
key_files:
  created:
    - .planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md
  modified:
    - Cargo.lock
    - crates/nono/Cargo.toml
    - crates/nono-cli/Cargo.toml
    - crates/nono/src/scrub.rs
decisions:
  - "C13 disposition: CLEAR - scrub.rs Cow deref change is purely mechanical (as_ref() -> &*); zero sigstore imports in scrub.rs; does not touch Phase-49 trust-root surface or D-32-15 invariant"
  - "Cargo bump applied as manual edits (not cherry-pick): fork at 0.7.0 baseline vs upstream's 0.6.x start; Phase 50 restructured sigstore deps across nono/nono-cli Cargo.toml files"
  - "D-32-15 verify-is-offline invariant: NOT REGRESSED - scrub.rs has zero TUF/fetch/sigstore references post-bump"
  - "D-55-03 merge gate honored: feature branch held off main until v0.58.0 is tagged + signed"
metrics:
  duration: "~18 minutes"
  completed: "2026-06-05T01:53:40Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 5
---

# Phase 55 Plan 07: C13 sigstore 0.8.0 Bump Summary

**One-liner:** sigstore 0.8.0 bump (verify+sign+trust-root) with CLEAR diff-inspection — Cow deref mechanical port, D-32-15 offline-verify invariant intact, all 5 crates build clean.

## Tasks Completed

| Task | Name | Commit | Key Artifacts |
|------|------|--------|---------------|
| 1 | C13 Disposition Resolution artifact | e117dd47 | 55-07-C13-DISPOSITION-RESOLUTION.md |
| 2 | sigstore 0.8.0 Cargo bump + scrub.rs port | 69197f34 | Cargo.lock, crates/nono/Cargo.toml, crates/nono-cli/Cargo.toml, crates/nono/src/scrub.rs |

## C13 Disposition Resolution Summary

**Verdict: CLEAR — port scrub.rs verbatim + apply Cargo.toml bumps manually**

### Upstream scrub.rs diff (e581569)
Two lines changed, both are Cow deref notation normalization:
- `scrub_value_with_policy`: `query_scrubbed.as_ref() as &str == s` → `&*query_scrubbed == s`
- `scrub_header_arg`: `scrubbed.as_ref() as &str != header_value.trim_start()` → `&*scrubbed != header_value.trim_start()`

Both forms are semantically identical in Rust (`Cow<'_, str>` implements `Deref<Target = str>`).

### Phase-49 Trust-Root Surface Inspection
- Phase-49 additions are entirely in `crates/nono-cli/src/setup.rs` (the `--from-file` flag) and `crates/nono/src/trust/bundle.rs` (load/verify pipeline).
- `scrub.rs` is the diagnostics/audit redaction module — **it is not part of the Phase-49 trust-root surface**.
- `scrub.rs` has zero imports from the `sigstore` ecosystem.
- The two upstream changes do not touch any sigstore API call site.

### D-32-15 Verify-is-Offline Invariant
**STATUS: NOT REGRESSED**

Post-bump verification: `grep -n "TUF|tuf|update_trusted_root|fetch|sigstore|trust" crates/nono/src/scrub.rs` returns zero results. The `trusted_root.json` plain JSON deserialization path in `crates/nono/src/trust/bundle.rs` is completely untouched by the e581569 changes.

### Collision Analysis
| Change | Touches Phase-49 surface? | Touches D-32-15 path? | Verdict |
|--------|--------------------------|----------------------|---------|
| scrub_value_with_policy Cow deref | NO | NO | CLEAR |
| scrub_header_arg Cow deref | NO | NO | CLEAR |

## Cargo Bump Log

**Fork baseline before this plan:** sigstore-verify 0.7.0 (nono), sigstore-sign 0.7.0 (nono-cli), sigstore-trust-root 0.7.0 (nono-cli)

**Applied manually** (not cherry-pick) because:
- Fork was at 0.7.0; upstream e581569 was authored from 0.6.x — cherry-pick would conflict
- Phase 50 restructured sigstore deps: sigstore-sign/sigstore-trust-root live in nono-cli, not nono
- Fork's nono/Cargo.toml has `features = ["tuf"]` addition (Phase 37) not in upstream's e581569 format

**Changes applied:**
- `crates/nono/Cargo.toml`: `sigstore-verify` 0.7.0 → 0.8.0 (features = ["tuf"] preserved)
- `crates/nono-cli/Cargo.toml`: `sigstore-sign` 0.7.0 → 0.8.0
- `crates/nono-cli/Cargo.toml`: `sigstore-trust-root` 0.7.0 → 0.8.0

**Cargo.lock transitive delta:** +291 / -49 lines

New crates added at 0.8.0: sigstore-{bundle,crypto,fulcio,merkle,oidc,rekor,sign,tsa,trust-root,types,verify}

Other updates:
- `reqwest` bumped to 0.13.3 (high-use, well-audited canonical Rust HTTP crate)
- `aws-lc-rs` 1.16.3 → 1.17.0, `aws-lc-sys` 0.40.0 → 0.41.0 (AWS crypto backend)
- `jiff` 0.2.28 added (datetime library, new sigstore 0.8.0 dependency)
- `portable-atomic` 1.13.1 added (jiff transitive dependency)

**Security audit note (T-55-07-02, T-55-07-SC):** All transitive deps are from the canonical Rust crate ecosystem — sigstore-rs is the fork's own upstream dep (already in lockfile at 0.7.0), reqwest is a well-audited canonical HTTP crate. No [ASSUMED] or [SUS] packages. No unexpected new crates from unknown origins. Cargo.lock pin prevents phantom version injection.

## scrub.rs Handling

**Port: verbatim** (CLEAR verdict per 55-07-C13-DISPOSITION-RESOLUTION.md)

Both Cow deref adjustments applied directly to the fork's scrub.rs:
1. `scrub_value_with_policy` (line 188): `query_scrubbed.as_ref() as &str == s` → `&*query_scrubbed == s`
2. `scrub_header_arg` (line 278): `scrubbed.as_ref() as &str != header_value.trim_start()` → `&*scrubbed != header_value.trim_start()`

Note: The fork's scrub.rs uses `as_ref() as &str` idiom in several places (lines 185-186 for `header_scrubbed` and `url_scrubbed`). The upstream e581569 only changed the two comparison sites; the remaining `as_ref() as &str` usages are a pre-existing fork divergence not part of the upstream patch and were not touched.

## D-55-E5: 5-Crate Build (PASS)

`cargo build --workspace` exits 0. All 5 crates compile clean:
- `nono` (core library)
- `nono-cli` (CLI binary)
- `nono-proxy` (network proxy)
- `nono-shell-broker` (Windows broker)
- `nono-ffi` / `bindings/c` (C FFI)

## D-55-E4: Baseline-Aware CI Gate

| Test lane | Before C13 | After C13 | Category |
|-----------|-----------|-----------|----------|
| `try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails` | FAIL | FAIL | red→red (carry-forward; pre-existing, documented in 55-05-SUMMARY.md) |
| All other tests (733) | PASS | PASS | green→green PASS |

Total: 733 passed, 1 failed (pre-existing), 0 introduced by this plan.

## D-55-E1: Windows-Only-Files Invariant (PASS)

`git diff --name-only HEAD~1 HEAD | grep -E "_windows.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines for both commits (Task 1: planning doc only; Task 2: Cargo.lock + 2 Cargo.toml + scrub.rs only).

## D-55-E3: Cross-Target Clippy

**N/A for this plan.**

`crates/nono/src/scrub.rs` contains only `#[cfg(test)]` (test module gate). No `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, or `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks. The CLAUDE.md MUST/NEVER cross-target clippy rule does NOT apply to scrub.rs.

`crates/nono/Cargo.toml` and `crates/nono-cli/Cargo.toml` have platform-specific `[target.'cfg(...)'.dependencies]` sections but these are Cargo.toml dep declarations, not Rust source cfg-gated code — no source-file clippy impact from bumping a version string.

## Merge Gate Note (D-55-03)

**Phase 55 feature branch MUST NOT merge to `main` until v0.58.0 is tagged + signed.**

Per D-55-03 (release-scope guard from `quick-260604-nue`): Phase 55 changes would ride into the signed v2.9 release. The held feature branch accumulates the full wave; the merge-to-main gate is the v0.58.0 tag. This plan's commits are on the `worktree-agent-a3a33cb2127eb4fba` branch, which will be merged to the Phase 55 feature branch by the orchestrator after wave completion.

## D-55-E6 Umbrella PR Note

Phase 55 maintains one umbrella PR to upstream per D-55-E6 (project_cross_fork_pr_pattern). Plan 55-07's contribution is the C13 sigstore 0.8.0 dep bump + scrub.rs port, to be included in the Phase 55 umbrella PR body.

## Deviations from Plan

**Deviation 1: Cargo.toml edits instead of cherry-pick**

- **Found during:** Task 2
- **Issue:** The plan offered "Branch A — cherry-pick e581569 verbatim" as the primary path if disposition was CLEAR. However, the fork's 0.7.0 baseline and Phase 50's restructured sigstore dep layout (sigstore-sign/sigstore-trust-root in nono-cli, not nono) mean that a verbatim cherry-pick would have introduced conflicts and potentially overwritten Phase 50's dep comments.
- **Fix:** Applied Cargo.toml bumps manually — identical net effect but with the fork's existing structure preserved. The plan explicitly anticipated this in its "BRANCH A — Resolve conflicts" notes and in the disposition resolution artifact.
- **Classification:** Rule 1 (pre-empted conflict) — inline fix, no behavioral divergence.

## Stubs

None — this plan is a dependency version bump and mechanical code port. No placeholder values, TODO items, or unconnected data paths were introduced.

## Threat Flags

None beyond what was already in the plan's threat model. All 4 threats mitigated per the disposition resolution artifact:
- T-55-07-01: MITIGATED (diff-inspection confirmed no D-32-15 regression)
- T-55-07-02: MITIGATED (sigstore-rs + reqwest are canonical deps; Cargo.lock pin holds)
- T-55-07-03: MITIGATED (scrub.rs has zero sigstore API imports)
- T-55-07-SC: MITIGATED (no [ASSUMED]/[SUS] packages in Cargo.lock delta)

## Self-Check

### Created Files
- `.planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md`: FOUND
- `.planning/phases/55-upst7-cherry-pick-wave/55-07-SUMMARY.md`: (this file)

### Commits
- e117dd47: docs(55-07): produce C13 disposition resolution artifact (CLEAR verdict): FOUND
- 69197f34: chore(55-07): bump sigstore crates to 0.8.0 + port scrub.rs Cow deref (C13): FOUND
