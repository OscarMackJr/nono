---
phase: 70-upst8-cherry-pick-sync
verified: 2026-06-13T03:00:00Z
status: human_needed
score: 3/4 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run GH Actions CI on HEAD (35282744) — confirm Linux Clippy + macOS Clippy lanes both exit 0 with -D warnings -D clippy::unwrap_used"
    expected: "Both lanes green; no cfg-gated Unix code in exec_strategy.rs, sandbox_prepare.rs, profile_runtime.rs, diagnostic.rs introduces new warnings"
    why_human: "Windows dev host cannot cross-compile (ring/aws-lc-sys C-toolchain missing). cargo check or Windows-host cargo clippy do not exercise Unix cfg branches. Per CLAUDE.md cross-target policy and cross-target-verify-checklist.md, the live GH Actions lane is the decisive signal."
---

# Phase 70: UPST8 Cherry-pick Sync — Verification Report

**Phase Goal:** The will-sync UPST8 commits (C3: cc21229f + 20cc5df9; C4: db073750; C2: 0fb59375 + bd4c469a) land on fork main with the fork's invariants preserved and the workspace green — mirroring the Phase 55 cherry-pick-wave shape.
**Verified:** 2026-06-13T03:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | All 5 will-sync commits cherry-picked with verbatim D-19/D-20 trailer blocks (range v0.60.0..v0.62.0) | VERIFIED | git log count = 5; each commit's 6-line trailer verified individually (see table below) |
| 2 | Windows-only-files invariant holds; fork-divergence catalog (RouteStore/CredentialStore, validate_path_within) preserved | VERIFIED | `git diff --name-only 6667177e..HEAD -- crates/ bindings/` returns 0 windows-specific files; server.rs auto-merged cleanly preserving RouteStore/CredentialStore decoupling |
| 3 | Cross-target clippy (Linux + macOS) verified per template — VERIFIED or PARTIAL with explicit CI deferral | PARTIAL (human_needed) | Windows dev host cannot cross-compile; confirmed PARTIAL per cross-target-verify-checklist.md policy; GH Actions CI required as decisive signal |
| 4 | Workspace builds + full suite green modulo documented pre-existing baseline failures | VERIFIED | cargo build --workspace exits 0; cargo test --workspace: 5 pre-existing failures (1 nono lib try_set_mandatory_label + 4 nono-cli profile_cmd init + 3 protected_paths); 0 new regressions; 4 new tests added (all pass) |

**Score:** 3/4 truths fully verified (truth 3 is PARTIAL pending CI)

### Commit-Level Trailer Verification

| Cluster | Upstream SHA | Fork SHA | Trailer Type | Author | Tag | Field Count |
|---------|-------------|----------|--------------|--------|-----|-------------|
| C3a | cc21229f | e80a7c45 | D-19 Upstream-commit | Luke Hinds | v0.61.0 | 6/6 |
| C3b | 20cc5df9 | 497101ae | D-19 Upstream-commit | Luke Hinds | v0.61.0 | 6/6 |
| C4 | db073750 | c18dd264 | D-20 Upstream-replayed-from | Luke Hinds | v0.62.0 | 6/6 |
| C2a | 0fb59375 | 1f5b6193 | D-19 Upstream-commit | Luke Hinds | v0.61.0 | 6/6 |
| C2b | bd4c469a | 35282744 | D-19 Upstream-commit | Caio Silva | v0.61.2 | 6/6 |

Phase-wide D-19/D-20 count: `git log --format="%B" 6667177e..HEAD | grep -c -E "^Upstream-(commit|replayed-from):"` = **5** (PASS)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/src/diagnostic.rs` | `suppressed_system_service_operations` field + `with_suppressed_system_service_operations()` builder | VERIFIED | Field at line 694; builder at line 833; filtering via `retain` over `non_fs_violations` only (confirmed FS denials cannot be suppressed) |
| `crates/nono-cli/src/profile/mod.rs` | `diagnostics.suppress_system_services: Vec<String>` deserialization | VERIFIED | Field at line 1953; merged in profile extends at line 3162 |
| `crates/nono-cli/src/sandbox_prepare.rs` | `suppressed_system_service_operations: Vec<String>` field (C2 prerequisite) | VERIFIED | Field at line 91; wired through PreparedProfile -> PreparedSandbox pipeline |
| `crates/nono-cli/src/sandbox_prepare.rs` | `network_block_requested: bool` field | VERIFIED | Field at line 104; extraction logic at line 565 |
| `crates/nono-cli/src/profile_save_runtime.rs` | Registry ref preservation in profile extends | VERIFIED | `is_registry_ref` check at line 622; 2 regression tests added |
| `crates/nono-proxy/src/config.rs` | `strict_filter: bool` field (default false) | VERIFIED | Field at line 45; Default impl sets false at line 73 |
| `crates/nono-proxy/src/filter.rs` | `ProxyFilter::new_strict()` wrapping `HostFilter::new_strict()` | VERIFIED | `new_strict()` at line 41 |
| `crates/nono/src/net_filter.rs` | `HostFilter::new_strict()` denying on empty allowlist | VERIFIED | `new_strict()` at line 155; 3 tests added |
| `crates/nono-cli/data/network-policy.json` | Embedded profiles (opencode, developer, codex, claude-code) with no inline credentials arrays | VERIFIED | All 4 profiles contain only `groups` keys; top-level `credentials` section is credential-type definitions, not profile-level injection |
| `crates/nono-cli/src/wiring.rs` | `ExecuteOptions` struct with `allow_unmanaged_identical_write_files` | VERIFIED (forward-compat stub) | Struct at line 54; used from `install_package` for tracing; deferred to v2.5-FU-3 (documented) |
| `crates/nono-cli/src/package_cmd.rs` | `force: bool` parameter on `install_package` | VERIFIED | Parameter at line 878; constructs ExecuteOptions at line 884 |
| `crates/nono-cli/src/profile_runtime.rs` | Hard-error on missing lockfile entry + .nono-trust.bundle | VERIFIED | Hard PackageVerification errors at line 158-166; 2 regression tests added |
| `.planning/REQUIREMENTS.md` | UPST8-01 body updated to v0.62.0 upper bound | VERIFIED | Line 27 references `v0.62.0` with D-01 parenthetical |
| `.planning/REQUIREMENTS.md` | UPST8-02 marked complete | VERIFIED (partial) | Checkbox `[x]` on line 28; Traceability table at line 61 still reads `Not started` — known SDK defect (feedback_sdk_roadmap_checklist_not_flipped), cosmetic gap |
| `.planning/ROADMAP.md` | Phase 70 completion marked + SC reflects v0.62.0 | VERIFIED | Line 33 `[x]`; Progress table at line 167 `Complete 2026-06-13`; SC #1 at line 86 reflects v0.62.0 |
| `.planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md` | Byte-identical to pre-phase state (immutable) | VERIFIED | `git diff 6667177e..HEAD -- .planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md` returns empty |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `sandbox_prepare.rs` | `proxy_runtime.rs` | `network_block_requested: bool` on PreparedSandbox | VERIFIED | proxy_runtime.rs line 131: `network_block: prepared.network_block_requested` |
| `proxy_runtime.rs` | `config.rs` | `build_proxy_config_from_flags` sets `proxy_config.strict_filter = proxy.network_block` | VERIFIED | proxy_runtime.rs line 220 |
| `config.rs` | `server.rs` | `config.strict_filter` controls `ProxyFilter::new_strict()` vs `ProxyFilter::new()` | VERIFIED | server.rs line 235 |
| `profile/mod.rs` | `sandbox_prepare.rs` | `suppress_system_services` field flows PreparedProfile -> PreparedSandbox pipeline | VERIFIED | sandbox_prepare.rs line 385 and 608 |
| `sandbox_prepare.rs` | `diagnostic.rs` | `suppressed_system_service_operations` flows through ExecConfig | VERIFIED | exec_strategy.rs carries the field; DiagnosticFormatter `with_suppressed_system_service_operations()` builder used |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `diagnostic.rs` DiagnosticFormatter | `suppressed_system_service_operations` | PreparedSandbox pipeline from profile | Yes — profile field deserialized, filtered through `with_suppressed_system_service_operations()` | FLOWING |
| `server.rs` filter selection | `config.strict_filter` | `ProxyConfig.strict_filter` set from `proxy.network_block` | Technically flowing; but `network_block=true` forces proxy inactive in the fork (WR-01 — fail-secure design) | STATIC on documented trigger (by design — see WR-01) |
| `profile_runtime.rs` `verify_profile_packs` | lockfile / trust-bundle presence | filesystem check on install dir | Real filesystem check; hard-error on absence | FLOWING |

### Behavioral Spot-Checks

| Behavior | Check | Result | Status |
|----------|-------|--------|--------|
| Trailer count = 5 across phase | `git log --format="%B" 6667177e..HEAD \| grep -c -E "^Upstream-(commit\|replayed-from):"` | 5 | PASS |
| Windows-only-files invariant (phase-wide) | `git diff --name-only 6667177e..HEAD -- crates/ bindings/ \| grep -E "_windows\.rs\|exec_strategy_windows\|nono-shell-broker"` | 0 lines | PASS |
| suppressed_system_service_operations field exists | `grep -c "suppressed_system_service_operations" crates/nono-cli/src/sandbox_prepare.rs` | 4 | PASS |
| strict_filter field exists | `grep "strict_filter" crates/nono-proxy/src/config.rs` | found at line 45 | PASS |
| network_block_requested field exists | `grep "network_block_requested" crates/nono-cli/src/sandbox_prepare.rs` | found at line 104 | PASS |
| Cargo.toml + Cargo.lock unchanged | `git diff 6667177e..HEAD -- Cargo.toml Cargo.lock` | 0 lines | PASS |
| repo-public invariant | `git status --short \| grep -E "build_notes\|\.gsd"` | 0 lines | PASS |
| No debt markers (TBD/FIXME/XXX) in phase changes | grep over phase diff | 0 matches | PASS |

### Cross-Target Clippy Gate

**Status: PARTIAL — deferred to live CI (per CLAUDE.md cross-target policy)**

Files modified by this phase that contain `#[cfg(target_os = "linux")]` or `#[cfg(target_os = "macos")]` blocks (in-scope per cross-target-verify-checklist.md):

- `crates/nono-cli/src/exec_strategy.rs` — touched by C3a (cc21229f); contains Unix cfg blocks
- `crates/nono-cli/src/sandbox_prepare.rs` — touched by C3a + C2b; contains Linux/macOS cfg blocks
- `crates/nono-cli/src/profile_runtime.rs` — touched by C4 (D-20 replay); carries pre-existing `#[cfg(target_os = "linux")]` blocks
- `crates/nono/src/diagnostic.rs` — touched by C3a; may carry cfg-gated code

Windows dev host cannot cross-compile to Linux/macOS (ring/aws-lc-sys C-toolchain absent). `cargo check` and Windows-host `cargo clippy` do NOT exercise Unix cfg branches per CLAUDE.md policy.

**Decisive signal:** GH Actions Linux Clippy + macOS Clippy lanes on HEAD `35282744` must report clean.

Risk assessment: the changes in cfg-gated blocks are minimal (struct field additions in sandbox_prepare.rs, builder method in diagnostic.rs). The probability of a cross-target clippy failure is low, but per policy the gate cannot be flipped to VERIFIED without CI confirmation.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| UPST8-02 | 70-01, 70-02, 70-03 | Will-sync commits cherry-picked with D-19/D-20 trailers; invariants preserved; suite green | VERIFIED (code complete; cross-target PARTIAL pending CI) | 5 commits on main with correct trailers; D-70-E1 PASS; workspace green vs baseline |

**Orphaned requirements:** None. REQUIREMENTS.md maps UPST8-02 solely to Phase 70; confirmed.

**Traceability table gap:** REQUIREMENTS.md Traceability table line 61 still reads `Not started` for UPST8-02 despite body checkbox being `[x]`. This is the known `feedback_sdk_roadmap_checklist_not_flipped` SDK defect. The requirement body is the authoritative completion signal; the table entry is a cosmetic gap (WARNING, not BLOCKER).

### Anti-Patterns Found

| File | Finding | Severity | Impact |
|------|---------|----------|--------|
| `crates/nono-cli/src/wiring.rs` lines 26-61 | `ExecuteOptions` doc comments describe behavior in present tense ("allows write_file directives to adopt existing unmanaged files") but the struct is a forward-compat stub — `execute_with_options` is not wired and the field is only read by a `tracing::debug!` call | WARNING (WR-02 from code review) | Misleads future auditors about the active security posture; no functional regression |
| `crates/nono-cli/src/proxy_runtime.rs` + `crates/nono-cli/src/capability_ext.rs` | `strict_filter` path is structurally unreachable when `network_block_requested=true` because `network_block=true` forces `proxy.active=false` — the guard never engages on its documented trigger | WARNING (WR-01 from code review) | Fork is already fail-secure (OS-level blocking replaces proxy under network.block); guard is inert but not harmful |
| `crates/nono-cli/src/exec_strategy.rs` vs `crates/nono/src/diagnostic.rs` | Suppression predicate divergence: target-less FS-mapped ops treated differently between the two suppression layers | INFO (WR-03 from code review) | Only affects footer rendering, not enforcement; latent correctness trap if save-prompt gate keys off mismatched visible_sandbox_violations |

No TBD/FIXME/XXX debt markers found in phase changes.
No `.unwrap()` or `.expect()` introduced in conflict-resolution code.
No `#[allow(dead_code)]` added.

`v2.5-FU-3` references in `wiring.rs` comments are tracked forward-compatibility markers referencing a named future release, not unresolved debt markers.

### Human Verification Required

#### 1. Cross-Target Clippy (Linux + macOS)

**Test:** Confirm the GH Actions CI run on HEAD `35282744` shows the Linux Clippy lane and macOS Clippy lane both report exit 0 with `-D warnings -D clippy::unwrap_used`.

**Expected:** Both lanes green. The cfg-gated Unix code modified by Phase 70 (exec_strategy.rs, sandbox_prepare.rs, profile_runtime.rs, diagnostic.rs) introduces no new clippy warnings on either target.

**Why human:** Windows dev host lacks the ring/aws-lc-sys C-toolchain required to cross-compile. Per CLAUDE.md Coding Standards and `.planning/templates/cross-target-verify-checklist.md`, the live CI lane is the decisive signal. The cross-target gate must NOT be flipped to VERIFIED based on Windows-host cargo check alone.

### Gaps Summary

No BLOCKER gaps. All load-bearing must-haves verified in code. The single pending item is the PARTIAL cross-target clippy gate routed to human verification per documented policy.

**Code review warnings (WR-01, WR-02, WR-03)** are inherited from the code review report (70-REVIEW.md) and do not block phase goal achievement:

- WR-01 (`strict_filter` inert): The fork is fail-secure on the documented trigger — `network.block` maps to full OS-level `NetworkMode::Blocked`, so no proxy, no allow-all fallback. The upstream guard does not fire but the security outcome (no outbound network) is stronger than what the guard would provide.
- WR-02 (misleading `ExecuteOptions` docs): Cosmetic; the forward-compat stub is correctly identified as deferred (v2.5-FU-3). Security: no impact.
- WR-03 (suppression predicate divergence): Affects diagnostic footer rendering only, not enforcement.

All three warnings are acceptable as-is for phase goal achievement; they are candidates for cleanup in a follow-on plan if prioritized.

---

_Verified: 2026-06-13T03:00:00Z_
_Verifier: Claude (gsd-verifier)_
