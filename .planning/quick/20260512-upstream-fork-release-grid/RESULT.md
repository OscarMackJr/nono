# Upstream Nono → Windows Native Fork Release Mapping (v0.37 → v0.52)

**Generated:** 2026-05-12
**Sources:** Phase 33 DIVERGENCE-LEDGER.md, Phase 34 VERIFICATION.md + 13 plan SUMMARYs, Phase 22 + Phase 20 plan inventory, ROADMAP.md.

Three upstream-sync phases absorbed the v0.37 → v0.52 range:

| Upstream range | Fork phase | Fork milestone | Status |
|---|---|---|---|
| v0.37.x (and v0.36 baseline) | **Phase 20 — UPST-01..04** | v2.1 (Resource Limits / IPC / Cleanup) | ✅ shipped 2026-04-19 |
| v0.38 – v0.40 | **Phase 22 — UPST2** | v2.2 (Windows/macOS Parity Sweep) | ✅ shipped 2026-04-28 |
| v0.41 – v0.52 | **Phase 34 — UPST3** | v2.3 (Linux POC Unblock + Deferreds) | ✅ complete with carry-forward 2026-05-12 |

---

## Phase 20 — v0.37 era (UPST-01..04, v2.1 milestone)

Phase 20 covered the upstream parity gap accumulated through v0.37.x. Cluster shapes pre-date the DIVERGENCE-LEDGER format, so granularity is at plan level.

| Plan | Theme | Status | Notes |
|---|---|---|---|
| **20-01** | UPST-01 — upstream-parity baseline | ✅ shipped | Established the per-commit D-19 trailer convention (`Upstream-commit:` block) reused by Phase 22 + Phase 34 |
| **20-02** | UPST-02 — profile/policy alignment | ✅ shipped | Surface-level alignment of profile schema and policy resolution |
| **20-03** | UPST-03 — env_sanitization (PARTIAL port) | ⚠️ partial | Manual port of upstream `1b412a7` **restricted to `crates/nono-cli/src/cli.rs`** — `--env-allow` / `--env-deny` flag-parsing only. EnvironmentConfig struct + 9 call-site files explicitly deferred. **This deferral surfaced empirically during Phase 34 and was closed by Plan 34-08a.** |
| **20-04** | UPST-04 — release alignment | ✅ shipped | Bookkeeping + cross-platform smoke gates |

**Status:** All 4 plans closed in v2.1 milestone. Phase 20-03's partial-port deferral became the load-bearing reason behind the 34-08 mid-flight split.

---

## Phase 22 — v0.38 – v0.40 (UPST2, v2.2 milestone)

Phase 22 took 78 commits across upstream `v0.38..v0.40.1` and landed them onto fork's `windows-squash` branch (later fast-forwarded into `main` at commit `1ef30c63`). Six plans (one split mid-flight into 22-05a / 22-05b).

| Plan | Cluster | Upstream commits | Status | Notes |
|---|---|---|---|---|
| **22-01** | PROF — profile struct field additions | ~10 commits | ✅ shipped | Locked the `Profile` / `ProfileDeserialize` companion struct pattern inherited by Phase 34 |
| **22-02** | POLY — policy tightening (POLY-01..02) | ~6 commits | ✅ shipped | `override_deny` orphan check; `conflicts_with` clap wiring |
| **22-03** | PKG — package manager (`nono-cli/src/package_cmd.rs`) | ~15 commits | ✅ shipped | **PKG-04 introduced fork's `validate_path_within` defense-in-depth (≥9 callsites) — preserved across all Phase 34 cherry-picks** |
| **22-04** | OAUTH — `nono-proxy/src/oauth2.rs` | ~12 commits | ✅ shipped | OAuth2 + WSAStartup ordering; preserved by Phase 34 Plan 34-10 |
| **22-05a** | AUD-CORE — audit-integrity + attestation + verify | ~20 commits | ✅ shipped | Phase 23 REQ-AUD-05 composes with this surface (used by Plan 34-10's `9300de9` cherry-pick) |
| **22-05b** | AUD-RENAME — `prune` → `session cleanup` + Authenticode | ~15 commits | ✅ shipped | Sets the mid-plan split precedent that Phase 34 re-applied for 34-04 → 34-04b and 34-08 → 34-08a/b |

**Status:** All 6 plans closed in v2.2. Carries-forward into Phase 34 invariants: `validate_path_within`, `ProfileDeserialize`, OAuth2 + WSAStartup ordering, audit-context shape.

---

## Phase 34 — v0.41 – v0.52 (UPST3, v2.3 milestone) — FULL CLUSTER MAPPING

Phase 34 audited upstream `v0.40.1..v0.52.0` (97 commits across 12 clusters) in Phase 33, dispositioned each cluster (8 will-sync / 2 fork-preserve / 2 won't-sync), and executed via 13 plans across 5 waves.

### Cluster-by-cluster grid

| # | Cluster | Upstream tag(s) | Theme | Disposition | Fork Plan(s) | Commits target / landed | Status | Key deferrals |
|---|---|---|---|---|---|---|---|---|
| **C1** | PTY attach/detach + signal handling | v0.41.0 | Cross-platform PTY proxy polish; Unix-side scrollback/alt-screen | 🚫 won't-sync | — (documented in `34-PHASE-OUTCOMES.md`) | 7 / 0 | won't-sync (D-11 rationale: fork's ConPTY structurally different from upstream's portable_pty) | — |
| **C2** | Profile/policy CLI consolidation + denial diagnostics | v0.41.0 | `nono policy` → `nono profile` rename + deprecation alias; richer denial UX | ✅ will-sync | **34-01-CLI-CONSOLIDATION** | 6 / 6 | ✅ complete | — |
| **C3** | Unix-socket capability + `--allow-unix-socket` | v0.42.0 | UnixSocketCapability + Linux seccomp `af_unix` | 🚫 won't-sync | — (documented in `34-PHASE-OUTCOMES.md`) | 4 / 0 | won't-sync (D-19 rationale: Unix-only; Windows IPC uses Named Pipes per Phase 18) | — |
| **C4** | Proxy/network policy hardening | v0.42.0 – v0.45.0 | `NO_PROXY` allow_domain fix; `--allow-connect-port`; native TLS roots | ✅ will-sync | **34-02-PROXY-NET** | 4 / 4 | ✅ complete | D-34-B2 surgical: NO Phase 09 WFP retrofit |
| **C5** | Headless keyring + display/quoting fixes | v0.43.0 – v0.45.0 | Optional system-keyring feature; char-aware truncation; shell-quote in display | ✅ will-sync | **34-03-KEYRING** | 8 / 8 | ✅ complete | `windows-native` keyring v3 backend preserved |
| **C6** | Pack migration + claude-code/codex registry relocation | v0.44.0 | Builtin profile registry pack format + install/uninstall hardening | 🔒 fork-preserve (D-20) | **34-09-FP-PACKS** | 6 / 2 (manual replay) | ✅ complete (4 commits skipped — structural divergence) | **P34-DEFER-09-1** Linux Landlock profiles-dir; **P34-DEFER-09-2** full `wiring.rs` abstraction (2-3 week future plan) |
| **C7** | Path canonicalization + canonical JSON schema restructure | v0.46.0 – v0.47.1 | Unified `try_canonicalize` helper; `override_deny` → `bypass_protection` rename; canonical sections | ✅ will-sync | **34-04-PATH-CANON-SCHEMA + 34-04b-FP-CANONICAL-SCHEMA** (mid-flight split) | 23 / 22 | ✅ complete (1 commit deferred) | **P34-DEFER-04b-1** full Option C deprecated_schema module port (~824 LOC + 210 callsites); **P34-DEFER-04b-2** upstream `829c341a` profile drafts feature (3619-line conflict) |
| **C8** | Shell completion + string-truncation refactor | v0.48.0 | `nono completion <shell>`; truncation panic fix; string-truncation utility | ✅ will-sync | **34-05-COMPLETION** | 8 / 8 | ✅ complete | D-34-B2 surgical: NO MSI integration; NO `$PROFILE.d/` shim |
| **C9** | Trust scan path-traversal hardening + YAML merge directive | v0.49.0 | `crates/nono/src/trust/scan.rs` symlink-escape + path-traversal rejections; `yaml_merge` directive | ✅ will-sync | **34-06-TRUST-SCAN** | 8 / 4 (security-critical only) | ✅ complete (4 commits deferred) | **P34-DEFER-06-1** yaml_merge wiring trio (blocked by unported `24d8b924` base, ~1761 LOC); **P34-DEFER-06-2** v0.49 release-bump (irrelevant — fork tracks own version) |
| **C10** | CLI ps display + `env://` credentials + ioctl native types | v0.50.0 – v0.50.1 | `nono ps` column display; `env://` URI in custom_credentials; Linux ioctl native types | ✅ will-sync | **34-07-PS-ENVURI** | 7 / 7 | ✅ complete | `env://` delegates to fork's existing `keystore::load_secret` (Phase 20 UPST-03) |
| **C11** | Proxy TLS interception + audit-event structured context | v0.51.0 | `tls_intercept` for L7-bearing CONNECT routes; structured audit context | 🔒 fork-preserve (D-20 split) | **34-10-FP-PROXY-TLS** | 6 / 1 cherry-pick + 4 documentation-only commits | ✅ complete (`9300de9` audit-context replayed; TLS-interception read-and-documented as non-port) | Windows credential-injection rewrite preserved byte-identical |
| **C12** | Env `deny_vars` + macOS learn diagnostics + `nono learn` deprecation | v0.52.0 | Operator-controlled deny_vars; empty-allow fail-closed; learn deprecation message; macOS learn enhancements | ✅ will-sync | **34-08a-ENV-SURFACE-PORT + 34-08b-LEARN-DEPRECATION** (mid-flight split) | 10 / 5 (env subset) + 5 (non-env subset) = 10 absorbed via various dispositions | ✅ complete | **P34-DEFER-08a-1** Windows `exec_strategy_windows/` env-filter wiring; **P34-DEFER-08b-1** `b5f0a3ab` deep ExecConfig refactor; **P34-DEFER-08b-2** `bbdf7b85` escape-quote structured-property wiring (depends on 08b-1) |

### Phase 34 wave structure

```
Wave -1: 34-00 G-25-DRIFT-01 closure (no-divergence finding)
   ↓
Wave  0: 34-04 PATH-CANON-SCHEMA  (C7, 17/23 commits — partial)
   ↓
Wave  0.5: 34-04b FP-CANONICAL-SCHEMA  (D-20 manual-replay split — 5/6)
   ↓
Wave  1: 34-01 CLI-CONSOLIDATION (C2)  |  34-03 KEYRING (C5)  |  34-06 TRUST-SCAN (C9, 4/8 partial)
   ↓
Wave  2: 34-02 PROXY-NET (C4)  →  34-05 COMPLETION (C8)  →  34-07 PS-ENVURI (C10)  →  34-08a/b ENV-SURFACE-PORT + LEARN-DEPRECATION (C12)
   ↓
Wave  3: 34-09 FP-PACKS (C6, D-20 manual replay)  →  34-10 FP-PROXY-TLS (C11, D-20 split + 34-PHASE-OUTCOMES.md addendum)
```

### Phase 34 totals

| Metric | Value |
|---|---|
| Plans closed | 13 / 13 |
| Mid-flight plan splits | 2 (34-04 → 34-04b, 34-08 → 34-08a/b) |
| D-20 manual-replay plans | 4 (34-04b, 34-08a, 34-09, 34-10) |
| Commits on `main` | ~75 (`aca306a5..23e70403`) |
| Cluster dispositions resolved | 12 / 12 (8 will-sync + 2 fork-preserve + 2 won't-sync) |
| Deferred items tracked | 13 (3 ACCEPTED-PERMANENT + 10 NEEDS-FOLLOW-UP-PLAN) |
| D-34-E1 invariant violations | 0 (zero `*_windows.rs` edits across all commits) |
| D-34-E2 trailer integrity | 47 `Upstream-commit:` + 3 `Manual-replay:` + 0 `Upstream-Author:` (case-sensitivity holds) |
| `learn_windows.rs` byte-identity | preserved (SHA `aa4d33dc` unchanged from start to end) |

---

## Cross-phase invariants (carry-forward from v2.1 → v2.2 → v2.3)

These survived intact across all three sync phases:

| Invariant | Origin | Preserved through |
|---|---|---|
| D-19 cherry-pick trailer block (6 lines, lowercase `Upstream-author:`) | Phase 20-01 | Phase 22 (78 commits) + Phase 34 (~75 commits) — verified zero `Upstream-Author:` case violations across all 3 phases |
| D-11 `*_windows.rs` invariance | Phase 22 D-17 / Phase 24 D-08 | Phase 34 (zero touches across 75 commits) |
| Phase 22-03 PKG-04 `validate_path_within` defense-in-depth | Phase 22 | Phase 34 (≥9 callsites preserved through every cherry-pick) |
| Phase 18.1-03 Windows widening (`Profile.capabilities.aipc`) | v2.1 | Phase 34 (≥17 `capabilities.aipc`/`loaded_profile` hits preserved; grew to 76 after Plan 34-08a's env-surface port added `Profile.environment`) |
| Phase 22-04 OAuth2 + WSAStartup ordering | Phase 22-04 | Phase 34 Plan 34-10 (14/14 tests green at plan close) |
| Phase 23 REQ-AUD-05 audit ledger | Phase 23 | Phase 34 Plan 34-10 (`9300de9` audit-context composed with REQ-AUD-05; 10/10 tests green) |
| Phase 24 D-08 drift-tool filter (`*_windows.rs` excluded) | Phase 24 | Phase 34 (`make check-upstream-drift` runnable post-phase) |
| Phase 26-01 PKGS-02 `ArtifactType::Plugin` round-trip | Phase 26-01 | Phase 34 Plan 34-09 (preserved through pack-migration manual replay) |
| Phase 28 Authenticode chain-walker (`exec_identity_windows.rs`) | Phase 28 | Phase 34 (D-11 excluded; byte-identical) |
| Phase 32 Sigstore TUF cached-root + broker self-trust-anchor | Phase 32 | Phase 34 (D-11 excluded surface; byte-identical) |
| Fork's own version tracking (currently `0.37.x`, NOT following upstream) | Pre-v2.0 | Phase 34 (all upstream `chore: release vX.Y.Z` commits cherry-picked as CHANGELOG-only; Cargo.toml/Cargo.lock version bumps dropped per fork convention) |

---

## v2.4 forward-look — "Complete the partial ports" theme candidates

Per Phase 34 VERIFICATION.md strategic recommendation, the following NEEDS-FOLLOW-UP-PLAN deferrals could compose into a coherent v2.4 phase:

| Deferral ID | Effort | Theme |
|---|---|---|
| P34-DEFER-04b-1 | ~1-2 weeks | Full Option C `deprecated_schema` module port (~824 LOC + 210-callsite internal rename + JSON schema fixture restructure + docs migration) |
| P34-DEFER-04b-2 | ~1 week | Upstream `829c341a` profile drafts feature (`nono profile promote`, `--draft` flag, package_status.rs, profile-drafts directory infrastructure) |
| P34-DEFER-06-1 | ~2-3 weeks | yaml_merge wiring trio (blocked by unported `24d8b924` base port ~1761 LOC) |
| P34-DEFER-08a-1 | ~3-5 days | Windows `exec_strategy_windows/` env-filter wiring (deny_vars + allowed_env_vars consumption on Windows path) |
| P34-DEFER-08b-1 | ~1-2 weeks | `b5f0a3ab` deep ExecConfig refactor (11 files / 721 insertions; macos learn + run diagnostics) |
| P34-DEFER-08b-2 | ~3-5 days | `bbdf7b85` escape-quote structured-property pipeline (depends on 08b-1) |
| P34-DEFER-09-1 | ~2-3 days | Linux Landlock profiles-dir pre-creation (from upstream `bdf183e9`) |
| P34-DEFER-09-2 | ~2-3 weeks | Full upstream `wiring.rs` abstraction (idempotent JSON-merge install records) |
| P34-DEFER-01-1 / P34-DEFER-10-1 | ~3-5 days | Windows test-harness hygiene (`query_ext::test_query_path_denied` UNC path flake + policy show/diff Rust Debug leak) |

**Accepted-permanent (no follow-up needed):** P34-DEFER-06-2 (v0.49 release-bump irrelevant), P34-DEFER-09-3 (carry-forward duplicate of 01-1), P34-DEFER-10-2 (WSAStartup vacuous-grep).

**Alternative:** Declare the surgical-port shape the new normal. Phase 34 demonstrated that 4 of 13 plans had to use D-20 manual replay; by upstream v0.55+ even bug-fix cherry-picks will likely require D-20 replay against the accumulated divergence base.

---

## Quick reference — fork plan files

```
.planning/phases/20-upstream-parity-sync/                    (v0.37 era; UPST-01..04)
.planning/phases/22-upst2-upstream-v038-v040-parity-sync/    (v0.38-v0.40; UPST2; PROF + POLY + PKG + OAUTH + AUD-01..04)
.planning/phases/33-windows-parity-upstream-0-52-divergence/ (audit of v0.40.1..v0.52.0; DIVERGENCE-LEDGER.md authoritative)
.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/ (v0.41-v0.52; UPST3; 13 plans; this phase's verification artifact at 34-VERIFICATION.md)
```

**Authoritative artifacts for re-reading the mapping:**
- `34-VERIFICATION.md` — per-cluster cluster-disposition matrix + invariant-verification-at-HEAD
- `34-PHASE-OUTCOMES.md` — D-34-A3 C1+C3 won't-sync addendum with Phase 33 ledger citations + future re-evaluation triggers
- `deferred-items.md` — 13 P34-DEFER-* entries with effort estimates + tracking pointers
- `DIVERGENCE-LEDGER.md` (in Phase 33 dir) — per-cluster commit tables with sha / subject / upstream-tag / categories / files-changed
