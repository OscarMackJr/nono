---
phase: 34-upst3-upstream-v0-41-v0-52-sync-execution
plan_number: 34-06
plan: 06
slug: trust-scan
cluster_id: C9
type: execute
wave: 1
status: complete-with-deferrals
upstream_tag_range: v0.49.0
upstream_commits_planned: 8
upstream_commits_landed: 4
fork_side_commits: 1
total_commits_landed: 5
deferred_items: [P34-DEFER-06-1, P34-DEFER-06-2]
requirements: [C9]
tags: [upst3, c9, trust-scan, path-traversal, symlink-escape, security, wave-1]

dependency_graph:
  requires:
    - 34-01 (CLI consolidation closed)
    - 34-03 (keyring closed)
    - 34-04 (path-canon-schema closed)
    - 34-04b (FP-canonical-schema closed)
  provides:
    - "Trust-scan path-traversal hardening (fdef1335)"
    - "Trust-scan symlink-escape hardening (cd4fd982) with Windows path-semantics extension tests"
    - "Empty-parent CWD derivation for bare-filename bundles (4f8c332c)"
    - "Defense-in-depth on `nono trust verify <bundle>` (multi-subject DSSE attacks closed)"
  affects:
    - "crates/nono-cli/src/trust_scan.rs (+177 / -14 net)"
    - "crates/nono-cli/src/trust_cmd.rs (small)"

tech_stack:
  added: []
  patterns:
    - "`safe_subject_path()` guard: Path::is_absolute() + Path::components() iteration + canonicalize-and-Path::starts_with containment check"
    - "POSIX-shaped test fixtures gated `#[cfg(unix)]`; Windows-flavored sibling tests added under `#[cfg(windows)]` (D-34-E4 fork test-fixture port pattern)"

key_files:
  created: []
  modified:
    - "crates/nono-cli/src/trust_scan.rs"
    - "crates/nono-cli/src/trust_cmd.rs"

decisions:
  - "D-34-06-A: Defer 3 yaml_merge-wiring commits (242d4917, 802c8566, d44f5541) to P34-DEFER-06-1 — they modify `crates/nono-cli/src/wiring.rs` which does not exist in the fork (upstream's wiring.rs base of 1761 lines was introduced in `24d8b924`, never adopted in fork). Adopting the prerequisite wiring infrastructure is multi-week scope; mirrors P34-DEFER-04b-1 / P34-DEFER-04b-2 deferral pattern from Phase 34."
  - "D-34-06-B: Defer v0.49.0 release-bump commit (587d98de) to P34-DEFER-06-2 — fork tracks its own version (0.37.1) independent of upstream version increments. Established fork pattern."
  - "D-34-06-C: Cherry-picked the 4 security/empty-parent commits in upstream topology order (fdef1335 → ce3230d8 → cd4fd982 → 4f8c332c), NOT the plan's listed order. The plan listed cd4fd982 before fdef1335 which would attempt to extend `safe_subject_path()` before introducing it — Rule 3 blocking-issue fix."
  - "D-34-06-D: Removed `build_signer_uri: None,` from the 2 upstream-ported test fixtures in cd4fd982 — fork's `nono::trust::Publisher` has 7 fields (no build_signer_uri); upstream introduced that field in a later commit not yet in this fork. Rule 3 fix (test-build blocker)."
  - "D-34-06-E: Gated the 2 POSIX-shaped rejection tests (`safe_subject_path_rejects_absolute_path`, `safe_subject_path_rejects_relative_dotdot_traversal`) with `#[cfg(unix)]`. On Windows `/tmp/scan` is treated as a relative path (no drive prefix), so `canonicalize(scan_root)` fails before `is_absolute()` fires. Production code is platform-correct; only the test input is Unix-only. Critical-invariants instruction explicitly directed this pattern."
  - "D-34-06-F: Added 2 Windows-flavored sibling tests under `#[cfg(windows)]` exercising `is_absolute()` + `components()` guards on Windows path semantics (drive-letter form `C:\\Windows\\...` and backslash traversal `..\\..\\...`). Tracks T-34-06-03 threat-register mitigation (Windows path-semantics composition with upstream's canonicalize-and-starts_with hardening)."

metrics:
  duration_minutes: 15
  completed_date: 2026-05-12
  tasks_completed: 4
  commits: 5
  files_modified: 2
  upstream_commits_landed_pct: 50  # 4 of 8 planned
---

# Phase 34 Plan 06: Trust-scan symlink-escape hardening + path-traversal hardening — Cluster C9 (v0.49.0)

## One-line summary

Landed the 4 security-critical trust-scan hardening commits from upstream v0.49.0 (`fdef1335` path-traversal, `cd4fd982` symlink-escape, `4f8c332c` empty-parent CWD) plus the `ce3230d8` rustfmt-over-trust-files commit, with Windows path-semantics extension tests under `#[cfg(windows)]`; deferred the 3 yaml_merge-wiring commits (require multi-week wiring.rs prerequisite port — P34-DEFER-06-1) and the v0.49.0 release-bump (P34-DEFER-06-2).

## Cluster C9 landed commits

| # | Order | SHA (upstream) | Local SHA | Subject |
|---|-------|----------------|-----------|---------|
| 1 | topology | `fdef1335` | `8f2802c9` | fix(trust): reject path traversal in multi-subject bundle subject names |
| 2 | topology | `ce3230d8` | `f5696373` | style: apply rustfmt to trust_cmd and trust_scan |
| 3 | topology | `cd4fd982` | `1cbe552e` | fix(trust): reject symlink-escape in multi-subject bundle subject names |
| 4 | topology | `4f8c332c` | `668e91ba` | fix(trust): treat empty parent() as CWD when deriving scan_root |
| 5 | fork-side | (N/A)       | `6425c41a` | style(34-06): rustfmt fork-side Windows trust-scan test extensions |

**Ordering note (D-34-06-C):** The plan listed `cd4fd982` (line 4) before `fdef1335` (line 7) which would attempt to extend `safe_subject_path()` before introducing it. Cherry-picked in upstream topology order (`fdef1335` → `ce3230d8` → `cd4fd982` → `4f8c332c`) to satisfy real upstream dependencies.

## Deferred commits

### P34-DEFER-06-1: yaml_merge wiring trio (3 commits)

**Deferred commits:** `242d4917` (yaml-merge pin), `802c8566` (rustfmt over wiring), `d44f5541` (yaml_merge directive creation).

**Why deferred:** All three commits modify `crates/nono-cli/src/wiring.rs`. The fork does **not** have this file in its tree. Upstream's `wiring.rs` was first created in `24d8b924` (`feat(profile, migration): move codex, claude-code to registry pack`) which is well outside the v0.49.0 cluster scope and was never adopted by the fork. At parent-of-`d44f5541` upstream's `wiring.rs` is 1761 lines (the `d44f5541` commit then adds ~360 lines on top). Adopting the prerequisite wiring infrastructure is multi-week scope.

**Mirrors Phase 34 precedent:** P34-DEFER-04b-1 (deprecated_schema module port, multi-week) and P34-DEFER-04b-2 (profile drafts + package status, feature-development scope) both deferred upstream work that demands multi-week prerequisite porting.

**Estimated scope:** multi-week to land upstream's wiring infrastructure base, then `242d4917` + `802c8566` + `d44f5541` apply as a normal cherry-pick chain.

### P34-DEFER-06-2: v0.49.0 release-bump (1 commit)

**Deferred commit:** `587d98de` (chore: release v0.49.0).

**Why deferred:** Fork tracks its own version (currently `0.37.1`) independent of upstream's version increments. The release-bump touches CHANGELOG.md + 5 Cargo.toml files; the version-number changes (0.48.0 → 0.49.0) would conflict with the fork's 0.37.1. Established fork pattern; same posture taken on prior Phase 34 release-bump commits.

**Future:** When the fork performs its own version increment, the upstream CHANGELOG entries (only the v0.49.0 stanza, lines 1–34 of `587d98de`'s CHANGELOG.md diff) can be ported as a docs-only contribution.

## D-34-D2 close-gate results

| # | Gate | Result | Notes |
|---|------|--------|-------|
| 1 | `cargo test --workspace --all-features` | **PASS** (with P34-DEFER-01-1 carry-forward) | 914 passed / 1 failed. The 1 failure is `query_ext::tests::test_query_path_denied` — pre-existing Windows path canonicalization flake documented in `deferred-items.md`. No NEW failures. |
| 2 | Windows clippy (`-D warnings -D clippy::unwrap_used`) | **PASS** | Clean. |
| 3 | Linux clippy | **DEFERRED-TO-CI** | Per D-34-D2 admin-skip; linker not installed on this Windows host. |
| 4 | macOS clippy | **DEFERRED-TO-CI** | Per D-34-D2 admin-skip; linker not installed on this Windows host. |
| 5 | `cargo fmt --all -- --check` | **PASS** | Required 1 fork-side cleanup commit (`6425c41a`) for the new Windows-extension test. |
| 6–8 | Admin gates | **PASS** | Admin-skipped per phase posture. |

## Critical invariants — all PASSED

| Invariant | Result |
|-----------|--------|
| **D-34-E1** Windows-only files invariant (zero `*_windows.rs` hits per commit) | PASS (0/0/0/0 across the 4 cluster commits) |
| **D-34-E2** D-19 trailer block lowercase 'a' | PASS — 4 `Upstream-commit:` + 4 `Upstream-author:` (lowercase) + 0 `Upstream-Author:` (uppercase) |
| **D-34-E4** Fork test-fixture port | PASS — 2 upstream `#[cfg(unix)]` symlink-escape tests ported; 2 fork-side `#[cfg(windows)]` sibling tests added |
| **Phase 32 `bundle.rs` untouched** | PASS — Last-touched SHA `ec9f1576` (pre-plan) === Last-touched SHA `ec9f1576` (post-plan) |
| **Fork-defense baselines** | PASS — `never_grant`/`apply_deny_overrides` = 21 (≥21); `validate_path_within` = 9 (≥9); `loaded_profile` = 17 (≥17); `find_denied_user_grants` = 7 (≥1); `bypass_protection` = 17 (≥1) |
| **CLAUDE.md path-handling policy** | PASS — Upstream uses `Path::is_absolute()`, `Path::components()`, and `Path::starts_with()` (component-aware, not string-`starts_with`). No D-20 manual port required. |

## Trailer audit

```
Upstream-commit count (expect 4):     4 ✓
Signed-off-by count   (expect 8):     8 ✓
Upstream-author: count (lowercase a): 4 ✓
Upstream-Author: count (uppercase A): 0 ✓ (D-34-E2)
```

## Deviations from plan

### D-34-06-A (DEFER): yaml_merge-wiring trio deferred to P34-DEFER-06-1

**Rule applied:** Rule 4 — architectural change requested. The 3 wiring commits require porting ~1761 lines of upstream wiring.rs base infrastructure that the fork never adopted. Followed Phase 34's established deferral precedent (P34-DEFER-04b-1, P34-DEFER-04b-2).

### D-34-06-B (DEFER): v0.49.0 release-bump deferred to P34-DEFER-06-2

**Rule applied:** Fork-version-management posture (established pattern).

### D-34-06-C (Rule 3 fix): Topology-order cherry-pick

The plan listed `cd4fd982` (which extends `safe_subject_path`) before `fdef1335` (which introduces it). Reordered to upstream topology to satisfy real dependencies.

### D-34-06-D (Rule 3 fix): Removed `build_signer_uri: None` from ported test fixtures

Upstream's `Publisher` struct has 8 fields including `build_signer_uri`; fork's `Publisher` has 7 fields. The new test fixtures in `cd4fd982` (lines 1838, 1898) used a field that doesn't exist on the fork. Removing the line preserves test-fixture correctness on the fork's `Publisher`.

### D-34-06-E (Rule 3 fix): Gated 2 POSIX-shaped tests with `#[cfg(unix)]`

The 2 upstream rejection tests use literal path `/tmp/scan` which is treated as a relative path on Windows (no drive prefix), so `canonicalize(scan_root)` fails before `is_absolute()` fires. Production code is platform-correct; only the test input is Unix-only. Critical-invariants instruction explicitly directed this pattern.

### D-34-06-F (Rule 2 add): 2 Windows-flavored sibling tests under `#[cfg(windows)]`

Added `safe_subject_path_rejects_absolute_path_windows` (uses `C:\Windows\System32\config\SAM`) and `safe_subject_path_rejects_relative_dotdot_traversal_windows` (uses `..\..\..\Windows\System32\config\SAM`). Tracks T-34-06-03 threat-register mitigation (Windows path-semantics composition with upstream's canonicalize-and-starts_with hardening). 33/33 trust_scan tests pass on Windows host.

## Authentication gates

None.

## Test-fixture port (D-34-E4)

**Upstream `#[cfg(unix)]` fixtures ported (2):**
- `safe_subject_path_rejects_symlink_escape` (cd4fd982 unit test) — symlink inside scan_root pointing outside is rejected
- `multi_subject_bundle_rejects_symlink_escape` (cd4fd982 end-to-end regression) — validly-signed bundle with symlink subject name yields `InvalidSignature`

**Upstream rejection tests `#[cfg(unix)]`-gated (2, fork adaptation):**
- `safe_subject_path_rejects_absolute_path`
- `safe_subject_path_rejects_relative_dotdot_traversal`

**Fork-side `#[cfg(windows)]` sibling tests added (2):**
- `safe_subject_path_rejects_absolute_path_windows`
- `safe_subject_path_rejects_relative_dotdot_traversal_windows`

## Wave 1 status

Plan 34-06 is the **third and final** plan in Wave 1. Wave 1 plans:

| Plan | Cluster | Status |
|------|---------|--------|
| 34-04 / 34-04b | C-PATH-CANON | CLOSED |
| 34-01 | C-CLI-CONSOLIDATION | CLOSED |
| 34-03 | C5 (keyring + display) | CLOSED |
| 34-06 | C9 (trust-scan hardening) | **CLOSED (this plan, with 2 deferrals)** |

**Wave 1: COMPLETE.**

## Push posture

`git push origin main` mandatory at plan close.

## Self-Check: PASSED

- `crates/nono-cli/src/trust_scan.rs` exists: FOUND
- `crates/nono-cli/src/trust_cmd.rs` exists: FOUND
- Commit `8f2802c9` (fdef1335) on main: FOUND
- Commit `f5696373` (ce3230d8) on main: FOUND
- Commit `1cbe552e` (cd4fd982) on main: FOUND
- Commit `668e91ba` (4f8c332c) on main: FOUND
- Commit `6425c41a` (fork fmt cleanup) on main: FOUND
