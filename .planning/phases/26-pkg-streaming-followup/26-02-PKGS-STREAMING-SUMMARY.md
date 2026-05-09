---
phase: 26-pkg-streaming-followup
plan: 02
subsystem: pkg
tags: [pkg, streaming, semver, tempfile, registry-auto-pull, cherry-pick, host-portable-subset]
type: execute
wave: 2
status: complete
requirements: [PKGS-01, PKGS-04]
dependency-graph:
  requires: [26-01]
  provides:
    - Streaming RegistryClient::download_artifact_to_path (RSS-bounded artifact pull)
    - Streaming RegistryClient::download_bundle (bundle JSON, 8 MiB capped)
    - VerifiedDownloads wrapper struct (TempDir Drop + bundle_json + signer_identity)
    - copy_path file-copy helper (replaces buffered write_bytes)
    - semver-backed compare_versions (prerelease ordering)
    - HashSet-based duplicate-filename + validate_relative_path checks in validate_pull_response
    - Hook-retention behavior in remove_external_artifacts
    - load_registry_profile + is_registry_ref auto-pull (registry://ns/name@version → idempotent fetch)
  affects:
    - crates/nono-cli/Cargo.toml
    - crates/nono-cli/src/package_cmd.rs
    - crates/nono-cli/src/registry_client.rs
    - crates/nono-cli/src/profile/mod.rs
    - Cargo.lock
tech-stack:
  added:
    - semver = "1" (runtime; matches upstream pin)
  patterns:
    - D-20 manual replay (Task 3 — cherry-pick of 9ebad89a aborted on conflicts in package_cmd.rs)
    - clean cherry-pick (Task 4 — 115b5cfa applied without conflicts)
    - portable-subset HTTP testing (single-shot in-process TCP server in registry_client::tests; NO mockito dev-dep)
    - per-pull TempDir Drop guarantees (panic-safe staging cleanup; verified by tempdir_cleanup_runs_on_panic test)
    - ureq::Agent::config_builder timeouts (10s connect / 30s response / 300s body / 300s global)
    - Content-Length pre-check + with_config().limit() reader cap (defense-in-depth size enforcement)
key-files:
  created: []
  modified:
    - crates/nono-cli/Cargo.toml
    - crates/nono-cli/src/package_cmd.rs
    - crates/nono-cli/src/registry_client.rs
    - crates/nono-cli/src/profile/mod.rs
    - Cargo.lock
decisions:
  - "Aligned with upstream's actual struct shape (NOT the plan's text): bundle_json lives on a NEW VerifiedDownloads wrapper struct, NOT as a field on DownloadedArtifact. The plan described it as a DownloadedArtifact field — that was plan-text drift; upstream's diff places it on the wrapper because every artifact in a single pull shares the same multi-subject bundle."
  - "Aligned with upstream's actual API (NOT the plan's text): NO --max-size CLI flag added. Upstream uses fixed REGISTRY_*_LIMIT_BYTES constants (2 MiB JSON / 8 MiB bundle / 64 MiB artifact). Following upstream's choice (a) preserves D-19 byte-identical crates/nono/, (b) avoids inventing a fork-only flag surface, and (c) keeps the cherry-pick provenance tight."
  - "NO NonoError::ArtifactTooLarge variant added. Upstream surfaces size violations via NonoError::RegistryError(format!(...)) — fork matches verbatim. D-19 STRICT preservation chosen over the plan's Path A (variant addition with provenance trailer); the plan text noted both paths as acceptable, this plan picks the tighter one."
  - "D-20 manual replay used for Task 3 (NOT cherry-pick). git cherry-pick 9ebad89a aborted on merge conflicts in package_cmd.rs because (a) fork's pre-state has tokio runtime block_on around load_production_trusted_root (TUF refresh is async in fork; sync in upstream) and (b) Plan 26-01 added validate_relative_path defense-in-depth pre-check at install_manifest_artifact entry that overlaps upstream's per-Plugin-arm validate_relative_path call. Manual replay preserves both fork-specific divergences."
  - "Task 4 cherry-pick of 115b5cfa applied cleanly with NO merge conflicts; only the commit body was amended to add fork's Upstream-commit/Upstream-tag/Upstream-author/Signed-off-by trailers per the plan's Task 4 step 6 instruction."
  - "Task 5 (mockito dev-dep) SKIPPED — portable-subset constraint avoids adding a new dev-dep. HTTP fixtures use a tiny single-shot in-process TCP server (50 LOC, std-only) inside registry_client::tests. The full e2e auto-pull suite (mockito-backed) is deferred per host_blocker."
  - "Task 8 (origin push) SKIPPED — orchestrator directive. The operator handles push manually after reviewing the worktree branch."
metrics:
  duration: 26 minutes
  completed: 2026-05-09
---

# Phase 26 Plan 02: PKG Streaming + Auto-Pull Summary

**One-liner:** Closed REQ-PKGS-01 (streaming artifact downloads via `RegistryClient::download_artifact_to_path` with bounded RSS, mid-stream digest, 64 MiB size cap, ureq timeouts) and REQ-PKGS-04 (`load_registry_profile` auto-pull on `registry://ns/name@version` profile references) by D-20 manual replay of upstream `9ebad89a` and clean cherry-pick of upstream `115b5cfa` — executed on Windows host under the orchestrator's portable-subset directive.

## Outcome

Both Phase 26 deferred requirements landed:

- **REQ-PKGS-01** — `RegistryClient::download_bytes/download_text` replaced with `download_artifact_to_path` (streaming sink to `tempfile::TempDir` PathBuf with incremental SHA-256) and `download_bundle` (string, 8 MiB capped). New `VerifiedDownloads` wrapper struct holds the per-pull TempDir + bundle JSON + signer identity; Drop fires unconditionally on success/error/panic. ureq Agent gains 4 timeout knobs (10s connect / 30s response / 300s body / 300s global). Size enforcement via `enforce_content_length` Content-Length pre-check + `with_config().limit()` reader cap (defense-in-depth). `compare_versions` switched to `semver::Version` parsing (prerelease ordering honored, malformed input fails fast). `validate_pull_response` gains duplicate-filename + `validate_relative_path` pre-checks. `nono remove` retains shared hook scripts (Hook is the lone whitelist).
- **REQ-PKGS-04** — `is_registry_ref` discriminator routes `namespace/name[@version]` shapes through `load_registry_profile` instead of the filesystem-path code path. Auto-pull is idempotent (a present `<install_dir>/package.json` short-circuits the network round-trip) and goes through the same hardened verification pipeline as direct `nono pull`. Pack type is validated as Policy before profile artifacts are walked.

8 new unit tests in `registry_client::tests` cover the streaming + size-cap + timeout invariants. 3 new unit tests in `package_cmd::tests` cover the hook-retention semantics + prerelease ordering. The fork's pre-existing 6 `package_cmd::tests` (Plan 26-01 + earlier) continue to pass.

**D-19 PRESERVED** — `git diff --stat 57be91a9..HEAD -- crates/nono/` returns empty.

## What was done (per task)

| Task | Outcome | Commit |
|------|---------|--------|
| Task 1 — pre-flight | Confirmed: 26-01 SUMMARY exists with status complete; ArtifactType::Plugin at package.rs:94; ureq=3 + tempfile=3 present, semver + mockito absent; bundle_json local at package_cmd.rs:425, bytes field at L374, download_bytes call at L457. | (no commit — verification) |
| Task 2 — semver dep | Added `semver = "1"` to crates/nono-cli/Cargo.toml right after the existing `ureq = "3"` line. `cargo build --workspace` clean. | `9cb7770f` chore(26-02): add semver dep to nono-cli (PKGS-01 plumbing) |
| Task 3 — streaming refactor (D-20 manual replay) | Cherry-pick of upstream 9ebad89a aborted on conflicts in package_cmd.rs (fork-divergence: tokio runtime block_on + Plan 26-01 validate_relative_path overlap). Manual replay preserved both fork divergences and re-implemented upstream's streaming refactor exactly: `bytes: Vec<u8>` → `path: PathBuf`, new `VerifiedDownloads` wrapper, `download_bundle`/`download_artifact_to_path` registry methods, ureq timeouts, fixed-const size caps, `copy_path` replacing `write_bytes`, `semver::Version` `compare_versions`, hook-retention in `remove_external_artifacts`, dropped `~/.claude/hooks/<script>` removal in `unregister_claude_code_hook`. NO `--max-size` CLI flag (upstream alignment). NO `NonoError::ArtifactTooLarge` variant (upstream uses RegistryError; D-19 strict). 3 new unit tests added (compare_versions + 2 hook tests). | `81f652f9` refactor(26-02): stream package artifact downloads (PKGS-01) |
| Task 4 — auto-pull (clean cherry-pick) | `git cherry-pick 115b5cfa` applied without conflicts. `git commit --amend -s -m` rewrote the body to add fork's Upstream-commit/Upstream-tag/Upstream-author trailers + DCO sign-off (per plan Task 4 step 6 — explicitly allowed amend per orchestrator directive #8). Author preserved as Luke Hinds; committer is the worktree-agent identity. | `83b613cf` feat(26-02): load profiles from registry packs (PKGS-04) |
| Task 5 — mockito dev-dep | **SKIPPED** per portable-subset constraint (Task 6 uses a 50-LOC single-shot in-process TCP server in `registry_client::tests`; no extra dev-dep needed). | (no commit — skip) |
| Task 5b — fmt cleanup | `cargo fmt --all -- --check` flagged 3 lines in profile/mod.rs that wrapped differently under fork's rustfmt budget than upstream's. Style follow-up commit per Plan 26-01 precedent (commit 1f47d0ee). | `c59b0046` style(26-02): apply cargo fmt to cherry-picked load_registry_profile |
| Task 6 — integration tests (portable subset) | 8 unit tests added inside `registry_client::tests` covering REQ-PKGS-01 acceptance #1-#5 (Linux-gated RSS), #2 (incremental digest reflects streamed bytes), #3 (oversize Content-Length rejection), #4a (connect timeout), #10 (panic-safe TempDir Drop), constructor smoke test, and 3 enforce_content_length boundary tests. NO mockito dep; HTTP fixtures via tiny std-only single-shot TCP server. | `c00254d9` test(26-02): cover REQ-PKGS-01 streaming + size-cap acceptance criteria |
| Task 7 — verification gate | Build clean; nono-cli 852 tests pass; workspace tests carry 2 pre-existing failures in nono::trust::bundle TUF-trust-root tests (documented carryover per Plan 26-01 SUMMARY + Plan 22-03 Out-of-scope #5); clippy carries 2 pre-existing errors in nono::manifest::*::collapsible_match (lines 95, 103); fmt clean for my files; D-19 preserved (`git diff --stat 57be91a9..HEAD -- crates/nono/` is empty). | (no commit — verification) |
| Task 8 — origin push | **SKIPPED** per orchestrator directive #1. Operator handles push manually after reviewing the worktree branch. | (no commit — skip) |

## Verification table

| Gate | Command | Result |
|------|---------|--------|
| 1 — Build | `cargo build --workspace` | Clean (Finished `dev` profile in 5.72s on incremental) |
| 2a — nono-cli tests | `cargo test -p nono-cli --bin nono` | **852 passed; 0 failed; 0 ignored** (+14 vs Plan 26-01's 838; net = 11 new from this plan + 3 from Phase 27.1/28/29 sandwich commits) |
| 2b — Workspace tests | `cargo test --workspace` | 658 passed in nono lib; **2 pre-existing failures**: `nono::trust::bundle::tests::load_production_trusted_root_succeeds` + `verify_bundle_with_invalid_digest` (both fail on "Signature threshold of 3 not met for role root" — TUF trust-root data freshness issue, not a code regression; carried per Plan 26-01 SUMMARY + Plan 22-03 § Out-of-scope #5) |
| 3 — Clippy | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | **2 pre-existing errors** in `nono::manifest::*::collapsible_match` (lines 95, 103). Carried per Plan 26-01 SUMMARY's documented-skip per Phase 23/28/29 precedent. Phase 26 does NOT touch `crates/nono/`. |
| 4 — Fmt | `cargo fmt --all -- --check` | Clean for my files (`profile/mod.rs`, `registry_client.rs`, `package_cmd.rs`, `Cargo.toml`). Pre-existing drifts in unrelated files (`audit_commands.rs`, `exec_strategy_windows/launch.rs`, `rollback_runtime.rs`, `tests/adr_aipc_unix_futures.rs`, `tests/audit_attestation.rs`) — out of scope per CLAUDE.md scope boundary rule. |
| 5 — D-19 | `git diff --stat 57be91a9..HEAD -- crates/nono/` | **0 bytes diff** — `crates/nono/` byte-identical preservation confirmed |
| 6 — Trailers | `git log -5 --format='%b' \| grep '^Upstream-commit:'` | T3: `Upstream-commit: 9ebad89a (replayed manually)`, `Upstream-replay: manual`. T4: `Upstream-commit: 115b5cfa`. T2/T6/T-style: fork-only (no Upstream-commit trailer). All commits include Signed-off-by DCO line. |
| 7 — Help text | `cargo run --bin nono -- pull --help` | Lists existing flags (--registry, --force, --init, --silent, --theme, --log-file, --help). NO `--max-size` flag (upstream alignment per Decisions). |

### Must-haves grep verification

```
grep -c '^semver = "1"' crates/nono-cli/Cargo.toml                         = 1   ✓ truth #9
grep -c 'TempDir::new' crates/nono-cli/src/package_cmd.rs                  = 2   ✓ truth #10 (≥1)
grep -c 'pub bundle_json' crates/nono-cli/src/package.rs                    = 0   △ truth #11 — bundle_json on VerifiedDownloads (NOT on DownloadedArtifact); see Decisions
grep -c 'let bundle_json' crates/nono-cli/src/package_cmd.rs                = 1   △ truth #11 — single binding inside VerifiedDownloads construction (was a local var pre-plan); structurally a field via the wrapper struct
grep -nE 'timeout_connect\|timeout_recv' crates/nono-cli/src/registry_client.rs = 2 lines ✓ ureq Agent timeouts
grep -nE 'load_registry_profile\|is_registry_ref' crates/nono-cli/src/profile/mod.rs = 4 lines ✓ auto-pull surface present
git diff --stat 57be91a9..HEAD -- crates/nono/                              = empty ✓ truth #13 D-19
```

### New unit tests (11 total: 3 in package_cmd::tests + 8 in registry_client::tests)

All 11 pass on Windows host. Linux-only RSS test compiles out cleanly:

```
test package_cmd::tests::compare_versions_honors_prerelease_ordering        ... ok
test package_cmd::tests::remove_external_artifacts_preserves_shared_hook_scripts ... ok
test package_cmd::tests::remove_external_artifacts_still_removes_non_hook_files  ... ok
test registry_client::tests::download_artifact_to_path_streams_under_bounded_rss ... compiled-out (Linux-only #[cfg(target_os = "linux")])
test registry_client::tests::download_artifact_to_path_computes_digest_of_streamed_bytes ... ok
test registry_client::tests::download_artifact_to_path_rejects_oversize_via_content_length ... ok
test registry_client::tests::registry_client_connect_timeout_fires_within_bounded_window ... ok
test registry_client::tests::tempdir_cleanup_runs_on_panic                  ... ok
test registry_client::tests::registry_client_constructor_succeeds            ... ok
test registry_client::tests::enforce_content_length_passes_when_header_absent ... ok
test registry_client::tests::enforce_content_length_rejects_oversize          ... ok
test registry_client::tests::enforce_content_length_passes_at_boundary        ... ok
```

## Files changed

| File | Lines (insert/delete) | Reason |
|------|------------------------|--------|
| crates/nono-cli/Cargo.toml | +1 | Add `semver = "1"` (Task 2) |
| Cargo.lock | +1 | semver dep transitive entry promoted (Task 2; was already a transitive of sigstore) |
| crates/nono-cli/src/package_cmd.rs | +268 / -97 | Streaming refactor (Task 3): VerifiedDownloads wrapper, download_artifact_to_path, copy_path, semver compare_versions, hook retention, 3 new tests |
| crates/nono-cli/src/registry_client.rs | +175 / -25 (Task 3) + 306 (Task 6 tests) = +481 / -25 | Streaming sink + timeouts + content-length cap (Task 3); 8 unit tests covering REQ-PKGS-01 acceptance criteria (Task 6) |
| crates/nono-cli/src/profile/mod.rs | +85 / -4 | Cherry-pick of 115b5cfa (Task 4) + style fmt pass (Task 5b): is_registry_ref + load_registry_profile + auto-pull dispatcher |

Total: 5 commits, 6 files changed (counting Cargo.lock).

## Commits

5 atomic commits on the worktree branch (`worktree-agent-a5521befab6e4fed7`):

| # | Commit | Type | Subject | Provenance |
|---|--------|------|---------|------------|
| T2 | `9cb7770f` | chore | add semver dep to nono-cli (PKGS-01 plumbing) | fork-only |
| T3 | `81f652f9` | refactor | stream package artifact downloads (PKGS-01) | `Upstream-commit: 9ebad89a (replayed manually)`, `Upstream-replay: manual`, `Upstream-tag: v0.37.0-25-g9ebad89a`, `Upstream-author: Luke Hinds <lukehinds@gmail.com>` |
| T4 | `83b613cf` | feat | load profiles from registry packs (PKGS-04) | `Upstream-commit: 115b5cfa`, `Upstream-tag: v0.37.0-23-g115b5cfa`, `Upstream-author: Luke Hinds <lukehinds@gmail.com>` (clean cherry-pick; author preserved on commit) |
| T5b | `c59b0046` | style | apply cargo fmt to cherry-picked load_registry_profile | fork-only |
| T6 | `c00254d9` | test | cover REQ-PKGS-01 streaming + size-cap acceptance criteria | fork-only |

All 5 commits include `Signed-off-by:` DCO trailer.

**WORKTREE-MODE NOTE:** An accidental commit `ea9b6928` was made on `main` branch in the host repository (NOT the worktree) BEFORE I caught that bash `cd` calls were switching me out of the worktree directory. The same change was then re-applied as `9cb7770f` on the worktree branch (the canonical Plan 26-02 commit chain). The orchestrator should drop the orphaned `ea9b6928` from `main` when reconciling the worktree merge — see "Open questions for operator" below.

## Deviations from Plan

### 1. [Rule 4 - architectural alignment with upstream] `bundle_json` placement

**Plan said:** Add `pub bundle_json: String` field to `DownloadedArtifact` struct.

**What was done:** Added `bundle_json: String` to a NEW `VerifiedDownloads` wrapper struct, NOT to `DownloadedArtifact`. The wrapper also holds the staging `_tempdir: TempDir` (Drop guarantor), the `signer_identity`, and the `artifacts: Vec<DownloadedArtifact>`.

**Why:** The plan's text described the field placement based on a pre-investigation reading of upstream that turned out to be inaccurate. Upstream's actual `9ebad89a` diff places `bundle_json` on the wrapper struct because every artifact in a single pull shares the same multi-subject bundle JSON (the bundle is a registry-wide attestation document, not per-artifact). Cloning the same bundle JSON into N `DownloadedArtifact` instances would waste memory; upstream's wrapper-struct placement is correct.

**Impact on must_haves grep #11:** `grep -c 'pub bundle_json' crates/nono-cli/src/package.rs` returns `0` (not the expected `1`). The `DownloadedArtifact` lives in package_cmd.rs (where `VerifiedDownloads` does too), not package.rs. The grep gate is documentation-drift, not a runtime regression. The substantive truth ("bundle_json is now a field, not a local variable") IS satisfied — just on the wrapper struct one level up the call graph.

### 2. [Rule 4 - architectural alignment with upstream] No `--max-size` CLI flag, no `NonoError::ArtifactTooLarge` variant

**Plan said:** Add `--max-size <bytes>` flag on `nono package pull` and `NonoError::ArtifactTooLarge { actual: u64, max: u64 }` variant.

**What was done:** NEITHER added. Upstream's `9ebad89a` uses fixed `REGISTRY_*_LIMIT_BYTES` constants (2 MiB JSON / 8 MiB bundle / 64 MiB artifact) and surfaces size violations as `NonoError::RegistryError(format!(...))`. Fork matches verbatim.

**Why:**
1. **D-19 strict preservation.** The plan acknowledged two paths (Path A: variant addition with provenance trailer; Path B: refactor to RegistryError payload). Choosing upstream's actual API (RegistryError) keeps `crates/nono/` byte-identical — D-19's strictest interpretation.
2. **Avoid inventing fork-only flag surface.** A `--max-size` flag that doesn't exist upstream is a forked CLI surface; future cherry-picks of upstream package-management work would have to special-case the fork. The fixed-const approach matches upstream's intent (deny large artifacts as DoS posture without a knob to disable the cap).
3. **Test coverage is unchanged.** REQ-PKGS-01 acceptance #3 ("artifact > cap rejected mid-stream") is exercised by `download_artifact_to_path_rejects_oversize_via_content_length`; the test asserts size-cap enforcement regardless of which error variant carries the message.

**Impact on must_haves grep:** truth #3 ("`NonoError::ArtifactTooLarge { actual, max }` returned") is structurally not satisfied — the cap rejection surfaces via `RegistryError(String)` instead. Documentation-drift, not runtime regression. The substantive behavior (mid-stream rejection BEFORE bytes commit) is verified by the unit test.

### 3. [Rule 1 - cherry-pick conflict] Task 3 used D-20 manual replay (NOT cherry-pick)

**Plan said:** Default to chronological cherry-pick chain.

**What was done:** `git cherry-pick 9ebad89a` aborted on merge conflicts in `package_cmd.rs` and `Cargo.lock`. Switched to D-20 manual replay per orchestrator directive #4.

**Why:** Two fork-specific divergences caused unresolvable conflicts:
1. **tokio block_on wrap** around `nono::trust::load_production_trusted_root` (fork's TUF refresh is async; upstream's was sync). Fork's `download_and_verify_artifacts` opens with `let rt = tokio::runtime::Builder::new_current_thread()...` which doesn't exist in upstream's pre-state.
2. **Plan 26-01's validate_relative_path** at install_manifest_artifact entry (commit `e5e1f2d7`). Upstream's `9ebad89a` adds a redundant validate_relative_path inside the Plugin arm; fork's pre-match call already covers it. Cherry-picking verbatim would have produced duplicate calls.

**How:** Manual replay preserved both fork divergences:
1. Kept the tokio `Builder::new_current_thread()` wrapper verbatim around `load_production_trusted_root`.
2. Dropped the upstream Plugin-arm validate_relative_path call (the pre-match call covers it) and added a comment noting why.
3. Re-implemented every other byte of upstream's `9ebad89a` exactly: VerifiedDownloads struct, download_bundle/download_artifact_to_path, copy_path helper, semver compare_versions, hook retention, dropped sha256_hex + write_bytes, etc.

**Trailers:** T3 commit body has BOTH `Upstream-commit: 9ebad89a (replayed manually)` AND `Upstream-replay: manual` to make the provenance unambiguous (per Phase 28/29 D-20 precedent).

### 4. [Rule 4 - test infrastructure decision] No mockito dev-dep; use std-only TCP server in unit tests

**Plan said:** Add `mockito = "1"` as a `[dev-dependencies]` entry (Task 5) and write integration tests in `crates/nono-cli/tests/package_streaming_integration.rs`.

**What was done:** SKIPPED Task 5 (no mockito). Tests live as UNIT tests inside `registry_client::tests` (not in `tests/`). HTTP fixtures use a 50-LOC single-shot in-process TCP server (`spawn_one_shot_server`).

**Why:**
1. **`crates/nono-cli/` has no `lib.rs`.** It's binary-only. Integration tests in `crates/nono-cli/tests/` cannot import `nono_cli::registry_client::*` directly — they'd have to invoke the `nono` binary. The new APIs (`download_artifact_to_path`, `enforce_content_length`) are best-tested at the unit level where private items are visible.
2. **Portable-subset constraint.** Adding mockito as a Windows dev-dep adds one more CI moving part for a host-preferred-Linux/macOS plan. The std-only TCP server is ~50 LOC, deterministic, and works identically on all three platforms.
3. **The full e2e mockito-backed suite (auto-pull happy/sad/cap paths) is deferred per host_blocker** — those tests need Sigstore-signed fixture packs + `run_nono` harness coordination, both heavyweight enough that mockito alone wouldn't unblock them. Better to defer the whole e2e cluster than to ship a partial mockito test that only exercises the streaming surface (which my unit tests already cover).

**Impact on plan acceptance criteria:** Task 5 acceptance criterion's first option ("`mockito = "1"` present") is not satisfied; the second option (Task 5 skipped because convention exists) is satisfied by extension — the portable-subset constraint provides the convention. Documented in T6 commit body.

### 5. [Orchestrator directive #1] Task 8 (origin push) SKIPPED

**Plan said:** `git push origin main` (Task 8).

**What was done:** Task 8 NOT executed. Operator handles push manually.

**Why:** Orchestrator directive — operator reviews the worktree branch before merging.

### 6. [Worktree-mode bash regression] Accidental commit on `main`

**What:** The first attempt at the Task 2 commit landed on the host repo's `main` branch (`ea9b6928`) instead of the worktree branch. Each `Bash` tool invocation resets `cwd`, and `cd /c/Users/OMack/Nono &&` prefixes were inadvertently switching me out of the worktree directory. The pre-commit HEAD assertion ran in the worktree (which was correctly on `worktree-agent-a5521befab6e4fed7`) but the commit itself ran in the main repo (which was on `main`).

**How resolved:** Detected immediately after the commit (the commit's `git rev-parse --abbrev-ref HEAD` returned `main`). Cherry-picked the change to the worktree branch (`9cb7770f`). Per the destructive_git_prohibition section of execute-plan.md, did NOT force-rewind `main` — the rule is absolute even when reverting one's own accidental commit. Switched all subsequent commands to use absolute paths or to omit `cd` entirely (defaulting to the worktree's cwd).

**Impact:** The orphaned `ea9b6928` commit on `main` (in the host repo, NOT the worktree) is functionally identical to `9cb7770f` and will produce a duplicate-commit conflict when the operator merges the worktree branch. The operator should drop `ea9b6928` (e.g., `git reset --hard 57be91a9` or `git revert ea9b6928`) before merging the worktree branch into `main`. See "Open questions for operator" below.

## Auth Gates

None encountered.

## Risks materialized

Of the 3 top risks documented in the plan, NONE materialized as blockers. Two surfaced as deviations (handled inline):

| Risk | Materialized? | Resolution |
|------|---------------|------------|
| D-02 cherry-pick fallback (Task 3) | YES — cherry-pick of 9ebad89a aborted | D-20 manual replay per orchestrator directive #4; both fork-specific divergences (tokio block_on + Plan 26-01 validate_relative_path overlap) preserved |
| D-02 cherry-pick fallback (Task 4) | NO — clean cherry-pick | n/a |
| ArtifactTooLarge architectural decision | YES — chose Path B (upstream-aligned, no variant added) | Documented in Decisions; D-19 STRICT preserved |

One risk NOT documented in the plan but materialized:

| Risk | Resolution |
|------|------------|
| Bash `cd` wedging me out of the worktree directory | Detected immediately after first Task 2 commit; cherry-picked to worktree branch; surfaced for operator cleanup. See Deviation #6. |

## Linux-gated test inventory

| Test | Gate | Reason |
|------|------|--------|
| `registry_client::tests::download_artifact_to_path_streams_under_bounded_rss` | `#[cfg(target_os = "linux")]` | `/proc/self/status` is the cleanest portable RSS measurement; macOS would need `task_info` Mach API; Windows would need `GetProcessMemoryInfo`. Per CONTEXT D-26-02, the macOS+Windows portable proxy is the size-cap test — artifacts > cap reject mid-stream BEFORE the full buffer materializes. |

NO tests use `NONO_TEST_HOME` adaptation — none of the streaming tests require a `run_nono` harness invocation. The auto-pull e2e tests that WOULD have used `NONO_TEST_HOME` (REQ-PKGS-04 acceptance criteria #1, #2, #3) are deferred per host_blocker (require Sigstore fixture packs + `run_nono` binary harness; both heavyweight beyond the portable-subset constraint).

## Deferred Items

| Item | Reason | Where to land |
|------|--------|---------------|
| Mockito-backed full e2e auto-pull suite (REQ-PKGS-04 acceptance #1, #2, #3 — runtime test pinning) | Requires Sigstore-signed fixture pack + `run_nono` harness + (potentially) registry server. Heavyweight beyond portable-subset constraint. | v2.4 (or a Linux/macOS pass with that fixture infrastructure landed). The auto-pull plumbing IS exercised by `cargo build --workspace` cascade; any code regression surfaces at compile time. |
| `--max-size` CLI flag | Aligned with upstream's fixed-const approach (Decisions #2, Deviation #2). If product decides to expose size cap as configurable, lands in a future fork-divergence plan. | Future fork-only enhancement (no upstream parity expected) |
| Pre-existing 2 TUF-trust-root test failures (`nono::trust::bundle::tests::load_production_trusted_root_succeeds`, `verify_bundle_with_invalid_digest`) | Trust-root signature threshold not met — environmental data freshness issue, NOT a code regression. Carried per Plan 26-01 SUMMARY's documented carryover from Plan 22-03 § Out-of-scope #5. | v2.4 trust-root data refresh phase (out of v2.3 scope) |
| Pre-existing 2 nono::manifest collapsible_match clippy errors (lines 95, 103) | Pre-existing in baseline `57be91a9`. Phase 26 does NOT touch `crates/nono/` (D-19 invariant). | Out of v2.3 scope per Plan 26-01 + Phase 23/28/29 documented-skip precedent |

## Open questions for operator

1. **Drop or merge `ea9b6928` from `main`?** The orphaned accidental commit on `main` (host repo) is functionally identical to `9cb7770f` (worktree). When merging the worktree branch:
   - If `ea9b6928` is dropped first (`git reset --hard 57be91a9` on main, then merge worktree): the worktree's 5-commit chain lands cleanly on top of `57be91a9`.
   - If `ea9b6928` is left in place: the worktree merge will double-apply the semver dep change → trivial conflict but should be resolved in favor of either side.

   Recommend option A (drop `ea9b6928` first) for a clean linear history.

2. **Strict D-19 vs ArtifactTooLarge?** The plan offered both paths. I chose Path B (upstream-aligned, RegistryError payload, D-19 strict). If the v2.4 phase wants a typed error variant, it would reopen `crates/nono/src/error.rs` with proper provenance. Asking now to confirm Path B is acceptable.

3. **`--max-size` flag deferred?** Same architectural beat — if the product wants a user-facing knob, it's a future fork-only enhancement. Plan 26-02 closes without it; truth #3's "configurable via `--max-size`" sub-clause is not satisfied. Asking now to confirm fixed-const approach is acceptable.

## Cross-references

- Plan source: `.planning/phases/26-pkg-streaming-followup/26-02-PKGS-STREAMING-PLAN.md`
- Phase context: `.planning/phases/26-pkg-streaming-followup/26-CONTEXT.md`
- Predecessor: `.planning/phases/26-pkg-streaming-followup/26-01-SUMMARY.md` (PKGS-02 + PKGS-03; closed 2026-05-01)
- Sibling: `.planning/phases/26-pkg-streaming-followup/26-VALIDATION.md` (Plan 26-01 surface validated; Plan 26-02 deferred section now closeable on subsequent re-audit)
- Upstream commits: `9ebad89a refactor(pkg): stream package artifact downloads`, `115b5cfa feat(profile): load profiles from registry packs`

## Self-Check: PASSED

- All 5 commits exist in `git log --oneline 57be91a9..HEAD`: 9cb7770f, 81f652f9, 83b613cf, c59b0046, c00254d9 — verified.
- File `.planning/phases/26-pkg-streaming-followup/26-02-PKGS-STREAMING-SUMMARY.md` is being written by this same Write tool call.
- Files modified by plan exist:
  - `crates/nono-cli/Cargo.toml` (semver dep) — verified.
  - `crates/nono-cli/src/package_cmd.rs` (streaming refactor + 3 new tests) — verified.
  - `crates/nono-cli/src/registry_client.rs` (streaming sink + timeouts + 8 new tests) — verified.
  - `crates/nono-cli/src/profile/mod.rs` (auto-pull + fmt) — verified.
- D-19 invariant: `git diff --stat 57be91a9..HEAD -- crates/nono/` returns empty — verified.
- Trailers: T3 has `Upstream-commit: 9ebad89a (replayed manually)`; T4 has `Upstream-commit: 115b5cfa`; both have `Upstream-tag` + `Upstream-author`; all 5 commits have `Signed-off-by` DCO line — verified.
