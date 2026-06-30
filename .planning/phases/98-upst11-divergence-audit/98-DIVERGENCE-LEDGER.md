---
phase: 98-upst11-divergence-audit
plan: 01
ledger_type: upst11-audit
range: 1d1c88c9f98f0a1f3ff79cff1509713aaec7cdb0..d817ed53663c6bba4669ee7a5bfb41b35971fd1b (v0.65.1..v0.66.0)
range_note: "Window covers v0.66.0 (d817ed53663c6bba4669ee7a5bfb41b35971fd1b). The release-cut PR #1293 (commit d817ed53, chore: release v0.66.0) merges ~19 referenced PRs; substantive code is in those PRs. 260629-toe referenced SHA 605bb6c5 for #1293, which is a branch commit distinct from the tagged d817ed53; the tag is authoritative."
upstream_head_at_audit: d817ed53663c6bba4669ee7a5bfb41b35971fd1b
refetch_date: 2026-06-29
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'bash scripts/check-upstream-drift.sh --from 1d1c88c9f98f0a1f3ff79cff1509713aaec7cdb0 --to d817ed53663c6bba4669ee7a5bfb41b35971fd1b --format json > ci-logs-local/drift/20260629T000000Z-v0651-v0660-upst11.json'
fork_baseline: v0.65.1 (SHA 1d1c88c9f98f0a1f3ff79cff1509713aaec7cdb0; Phase 94 UPST10 endpoint — absorbed in v3.3 Phases 94-97)
total_unique_commits: 14
date: 2026-06-29
---

## Headline

14 substantive commits in window `1d1c88c9..d817ed53` (upstream `v0.65.1..v0.66.0`),
distributed across 8 ledger clusters: A through H. Dispositions (first-pass scaffold — confirmed
by Plan 02 actual-diff): `needs-decision` 1 (A — #1225 NetworkIntent ADR-98) / `won't-sync`
2 (B tool-sandbox-absent, H release) / `split` 1 (C proxy evolution) / `will-sync` 3 (D 9P
warning, F sigstore dep, G proxy token fix) / `verify` 1 (E org-ref migration). Windows-touch
clusters: **A** (NetworkIntent refactor — `capability_ext.rs`, `supervised_runtime.rs`,
`command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `output.rs`,
`terminal_approval.rs` all have `cfg(target_os = "windows")` blocks). Re-export scan pending
Plan 02 uniform actual-diff.

The headline fork-specific deliverables are: (1) the **#1225 NetworkIntent disposition** (Cluster A
— ADR-98-network-intent-disposition.md; Plan 03), and (2) the **expanded carve-out re-touch check**
(D-07; six carve-out surfaces; this window yields: CR-02 CLEAN, CR-01 CLEAN, Cluster F HIT — 5
commits, linux.rs invariants HIT — 1 commit, endpoint-policy HIT — 3 commits, v3.2 override CLEAN).

---

## Reproduction

```bash
# D-02 fork-baseline confirmation: v0.65.1 (1d1c88c9) was the Phase 94 UPST10 audit endpoint
# (absorbed in v3.3 Phases 94-97). The nolabs-ai/nono upstream remote is already configured.
# No remote relocation work this cycle (done in Phase 94).

# Fetch upstream with tags (non-destructive — upstream remote already set to nolabs-ai/nono)
git fetch upstream --tags

# Resolve window tip SHAs (D-02: pin in ledger, use in drift invocation)
git ls-remote --tags upstream v0.65.1 v0.66.0
# → 1d1c88c9f98f0a1f3ff79cff1509713aaec7cdb0  refs/tags/v0.65.1
# → d817ed53663c6bba4669ee7a5bfb41b35971fd1b  refs/tags/v0.66.0

# SHA guard: verify both window tips are reachable commit objects
git cat-file -t 1d1c88c9f98f0a1f3ff79cff1509713aaec7cdb0  # → commit (v0.65.1 window base)
git cat-file -t d817ed53663c6bba4669ee7a5bfb41b35971fd1b  # → commit (v0.66.0 window tip)

# Release-cut commit (#1293) reachability check
git cat-file -t 605bb6c5  # → commit (260629-toe PR branch SHA; distinct from tagged d817ed53)

# Drift tool SHA pin (D-10)
git log -1 --format=%H -- scripts/check-upstream-drift.sh
# → 0834aa664fbaf4c5e41af5debece292992211559

git log -1 --format=%H -- scripts/check-upstream-drift.ps1
# → 0834aa664fbaf4c5e41af5debece292992211559

# Run drift tool against EXPLICIT window SHAs (D-02 SHA-not-tag guard)
mkdir -p ci-logs-local/drift
bash scripts/check-upstream-drift.sh \
  --from 1d1c88c9f98f0a1f3ff79cff1509713aaec7cdb0 \
  --to d817ed53663c6bba4669ee7a5bfb41b35971fd1b \
  --format json \
  > ci-logs-local/drift/20260629T000000Z-v0651-v0660-upst11.json
# Output total_unique_commits: 14 (verified 2026-06-29)
# by_category: {profile: 3, policy: 0, package: 0, proxy: 4, audit: 0, other: 13}

# Full commit accounting (D-12 — noise reconciliation)
git log --oneline 1d1c88c9..d817ed53 | wc -l           # → 20 (total)
git log --merges --oneline 1d1c88c9..d817ed53 | wc -l  # → 0 (merges)
git log --no-merges --oneline 1d1c88c9..d817ed53 | wc -l  # → 20 (no-merges)

# Fork state: tls_intercept/ dir absent (Cluster F ongoing fork-preserve; Phase 89/94)
ls crates/nono-proxy/src/
# → audit.rs  config.rs  connect.rs  credential.rs  diagnostic.rs  error.rs  external.rs
#   filter.rs  lib.rs  oauth2.rs  reverse.rs  route.rs  server.rs  token.rs
# No tls_intercept/ directory.

# Fork state: tool-sandbox/ dir absent (Phase 94/95 Cluster B skipped)
ls crates/nono-cli/src/tool-sandbox/ 2>&1 || echo "absent"
# → ls: cannot access '...': No such file or directory / absent
```

**Auditor rerun:** Run the drift tool command above verbatim (copy the explicit SHA flags). The
14-commit JSON output matches the per-cluster inventories in this ledger. Any new output indicates
an upstream push after `2026-06-29`; re-run the full audit if the count changes.

---

## Cluster Summary

| cluster_id | theme | commits | disposition | windows-touch | rationale | phase-99-status |
|------------|-------|---------|-------------|---------------|-----------|-----------------|
| A | NetworkIntent refactor + contradictory-flag guard (HIGH-CONFLICT) | 2 | needs-decision (ADR-98) | yes | #1225 introduces CLI-owned NetworkIntent enum, removes ProxyOnly placeholder pattern; touches files with cfg(windows); ADR-98 settles adopt vs fork-diverge carve-out; #1263 pairs with #1225 | Plan 03 ADR-98 |
| B | Tool-sandbox enhancements (won't-sync — fork lacks tool-sandbox/) | 4 | won't-sync | no | Fork lacks tool-sandbox/ dir (skipped in Phase 94/95 Cluster B); all 4 commits patch files absent from fork; carry-forward if tool-sandbox ever adopted | won't-sync → future phase |
| C | Proxy evolution: HTTP/2 + endpoint routing (split — tls_intercept/ absent) | 3 | split | no | #983 large multi-file commit includes tls_intercept/ hunks (won't-apply); Cluster F carve-out applies to route.rs/reverse.rs/server.rs; shared proxy changes need extraction | Plan 02 diff-inspect |
| D | 9P filesystem capability warning | 1 | will-sync | no | Additive diagnostic in sandbox/linux.rs; Linux cfg-gated; touches v3.3 Phase 95 fork invariant path (linux.rs) — additive, no conflict | Plan 02 diff-inspect |
| E | Org reference migration (always-further → nolabs-ai) | 1 | verify | no | Source strings in setup.rs, update_check.rs, migration.rs, command_policy.rs, profile/mod.rs, proxy_runtime.rs, route.rs — fork uses OscarMackJr/nono; mechanical changes need per-file review | Plan 02 diff-inspect |
| F | Sigstore-trust-root dep bump (0.8.0 → 0.9.0) | 1 | will-sync | no | Exact-pin dep in crates/nono/Cargo.toml; evaluate sigstore-verify/sigstore-sign cascade compatibility | Plan 02 diff-inspect |
| G | Proxy docs + deprecated flag metadata fix | 1 | will-sync | no | cli.rs, cli_bootstrap.rs, token.rs source accuracy fixes; touches Cluster F path (token.rs in nono-proxy) but additive accuracy-only changes | Plan 02 diff-inspect |
| H | Release metadata v0.66.0 | 1 | won't-sync | no | CHANGELOG + version bumps (all Cargo.toml files); fork leapfrogs to 0.66.1 (D-15); cross-ref: upstream 0.66.0 floor for Phase 100 | won't-sync → Phase 100 |

---

## Excluded as Noise

**Path filter criteria:** The drift tool applies these path inclusion filters:
- `crates/nono/src/`
- `crates/nono-cli/src/`
- `crates/nono-proxy/src/`
- `crates/nono/Cargo.toml`

Exclusion patterns (even within included paths): `*_windows.rs`,
`crates/nono-cli/src/exec_strategy_windows/`.

Any non-merge commit not touching included paths after exclusion patterns is noise. All merge
commits are excluded regardless of touched files.

**Counts:**
- Total commits in window: 20
- Merge commits: 0
- Non-merge commits: 20
- Substantive (drift-tool filter): 14
- Noise total: **6** = 0 merge commits + 6 non-merge out-of-filter commits

**0 merge commits:** None in this window. No merge commits to enumerate.

**6 non-merge out-of-filter commits:**

| sha | subject | reason |
|-----|---------|--------|
| `b6154818` | fix(ci): downgrade runner to ubuntu-latest (#1259) | CI yaml only (`.github/workflows/ci.yml`) |
| `30cfee67` | feat(tests): add end-to-end integration tests for sandbox execution strategies (#1213) | Test files only (`crates/nono-cli/tests/` — not under `src/`; drift filter covers `src/` only) |
| `84b5e7ce` | ci: fix mapping err in compile step (#1251) | CI yaml only (`.github/workflows/ci.yml`) |
| `ebd94275` | ci: idempotent publish-crates + cross-compile check on release PRs (#1245) | CI yaml only (`.github/workflows/ci.yml` + `release.yml`) |
| `8aee0e77` | docs(proxy): explain proxy activation via custom credentials (#1247) | Data-dir only (`crates/nono-cli/data/profile-authoring-guide.md` — not under `src/`) |
| `5441f4eb` | chore(deps): bump criterion from 0.5.1 to 0.8.2 (#1232) | `Cargo.lock` + `crates/nono-cli/Cargo.toml` (non-core manifest; only `crates/nono/Cargo.toml` is in filter) |

**Disposition notes for noise commits (Phase 99 reference):**
- `30cfee67` (#1213 e2e integration tests): The 260629-toe planned `ADOPT` — cfg-gate Unix-only legs,
  prevent Windows hangs (cf. 6h seccomp-hang lesson). Though filtered as noise (tests/ not src/),
  Phase 99 should evaluate adopting these tests after any cfg-gated Unix absorbs.
- `84b5e7ce` / `ebd94275` (#1251 / #1245 CI changes): Reconcile against fork's prepare-only
  release pipeline in Phase 100.
- `8aee0e77` (#1247 proxy activation docs): Reconcile with fork's proxy-divergence docs.

**Completeness verification:** 14 substantive + 6 noise = 20 total. Matches
`git log --oneline 1d1c88c9..d817ed53 | wc -l` = 20. Every commit in the window is accounted
for in exactly one of: cluster per-commit tables (14) or this noise section (6).

---

## Carve-out Re-touch Check

This section records the expanded D-07 check for the six deliberate fork-divergence carve-out
points. Each surface receives an explicit result; "clean — no re-touch in window" is not silence.

### CR-02: `crates/nono/src/audit.rs` (records_verified: event_count > 0)

```bash
git log 1d1c88c9..d817ed53 -- crates/nono/src/audit.rs
# → (no output)
```

**Result: clean — no re-touch in window.**

No commit in the `v0.65.1..v0.66.0` window touches `crates/nono/src/audit.rs`. The fork's
`records_verified: event_count > 0` invariant (CR-02, `proj/ADR-87-cr02-audit-bypass.md`,
guard test `verify_empty_log_with_no_stored_metadata_is_not_valid`) is not threatened by this
window. Phase 99 executor: no CR-02 conflict work needed.

### CR-01: `bindings/c/src/` FFI entry points (clear_last_call_state() at entry)

```bash
git log 1d1c88c9..d817ed53 -- \
  bindings/c/src/diagnostic.rs bindings/c/src/lib.rs \
  bindings/c/src/capability_set.rs bindings/c/src/fs_capability.rs \
  bindings/c/src/sandbox.rs bindings/c/src/state.rs bindings/c/src/query.rs
# → (no output)
```

**Result: clean — no re-touch in window.**

No commit in this window touches any of the CR-01 FFI files. The fork's
`clear_last_call_state()` at-entry invariant (guard test
`diagnostic_code_is_cleared_between_calls`, Phase 88, commit `db0f221d`) remains unthreated.
Phase 99 executor: no CR-01 conflict work needed.

### Cluster F: Proxy fork-preserve surface

```bash
git log 1d1c88c9..d817ed53 -- \
  crates/nono-proxy/src/route.rs \
  crates/nono-proxy/src/connect.rs \
  crates/nono-proxy/src/reverse.rs \
  crates/nono-proxy/src/server.rs \
  crates/nono-cli/src/proxy_runtime.rs
# → 46bcfbb9 fix(network): wire --allow-endpoint through to credential routes (#1127)
# → cdeeb5b9 feat(proxy): add HTTP/2 support for reverse proxy and credential injection (#983)
# → c808f000 chore: migrate GitHub org references from always-further to nolabs-ai (#1235)
# → 72bcfd66 refactor(network): introduce NetworkIntent and remove ProxyOnly placeholders (#1225)
# → 08ca19a8 fix(proxy): match wildcard credential upstream routes (#1243)
```

**Result: HIT (5 commits) — expected conflict — preserve fork expression.**

All five are in clusters A (72bcfd66), C (cdeeb5b9, 46bcfbb9, 08ca19a8), and E (c808f000).
Each has Cluster F carve-out implications:
- `72bcfd66` (A): proxy_runtime.rs receives NetworkIntent; fork's Phase 89 proxy activation
  predicate (`proxy_activates_with_custom_credentials_only` guard) must survive adoption.
- `cdeeb5b9` (C): route.rs, reverse.rs, server.rs AND tls_intercept/ (won't-apply); shared proxy
  surface needs split extraction; fork's `EffectiveProxySettings` model and `_ep_` key namespace
  (Phase 89 Reconciliation Addendum) must not be overwritten.
- `46bcfbb9` (C): proxy_runtime.rs + launch_runtime.rs endpoint-wiring; examine compatibility
  with fork's `CompiledEndpointPolicy` / `endpoint_policy.evaluate()` path.
- `08ca19a8` (C): route.rs wildcard matching (small fix — likely additive, compatible).
- `c808f000` (E): proxy_runtime.rs + route.rs org-ref string changes; mechanical text changes
  only, but diff must confirm no semantic change.

**Absent `tls_intercept/` dir confirmation (2026-06-29):**
```bash
ls crates/nono-proxy/src/
# → audit.rs  config.rs  connect.rs  credential.rs  diagnostic.rs  error.rs  external.rs
#   filter.rs  lib.rs  oauth2.rs  reverse.rs  route.rs  server.rs  token.rs
# No tls_intercept/ directory.
```

Guard tests in force (Phase 89 / `73bd03a6`):
`proxy_activates_with_custom_credentials_only`, `block_net_overrides_custom_credentials_activation`,
`build_proxy_config_maps_upstream_proxy_to_external_proxy`, `connect_keeps_open_on_missing_proxy_auth`,
`denied_endpoint_returns_403_and_audit`, `allow_domain_endpoint_route_does_not_shadow_credential_route`.

### v3.3 Phase 95 Endpoint-Policy Wiring (CompiledEndpointPolicy / endpoint_policy.evaluate())

```bash
git log 1d1c88c9..d817ed53 -- \
  crates/nono-cli/src/network_policy.rs \
  crates/nono-proxy/src/config.rs \
  crates/nono-proxy/src/credential.rs \
  crates/nono-proxy/src/reverse.rs \
  crates/nono-proxy/src/route.rs \
  crates/nono-proxy/src/server.rs
# → cdeeb5b9 feat(proxy): add HTTP/2 support for reverse proxy and credential injection (#983)
# → c808f000 chore: migrate GitHub org references from always-further to nolabs-ai (#1235)
# → 08ca19a8 fix(proxy): match wildcard credential upstream routes (#1243)
```

**Result: HIT (3 commits) — expected conflict — preserve fork expression.**

`cdeeb5b9` (#983, Cluster C) touches `config.rs`, `credential.rs`, `reverse.rs`, `route.rs`,
`server.rs` — the same files the fork's `CompiledEndpointPolicy` / `endpoint_policy.evaluate()`
wiring lives in (v3.3 Phase 95, the highest re-touch exposure surface per D-07). Phase 99 must
do careful split extraction: skip tls_intercept/ hunks, verify endpoint-policy wiring is
preserved. `c808f000` (E) is org-string only in `route.rs`. `08ca19a8` (C) is a targeted
wildcard-matching fix in `route.rs` — likely additive.

### v3.3 Phase 95 Restored Fork Invariants (linux.rs AF_UNIX/seccomp/cgroup paths)

```bash
git log 1d1c88c9..d817ed53 -- crates/nono/src/sandbox/linux.rs
# → 5b8e94da fix(sandbox): warn when capability path is on a 9P filesystem (#1207)
```

**Result: HIT (1 commit) — additive only — no fork-expression conflict.**

`5b8e94da` (Cluster D, will-sync) adds a 9P filesystem detection warning to `sandbox/linux.rs`.
The diff is additive (new `is_9p_path()` check inserted before the Landlock capability-add
loop). It does NOT touch the AF_UNIX seccomp BPF filter path, the cgroup v2 resource
enforcement module, or the `restrict_self()` / `apply_landlock()` entry points restored in
Phase 95/96. Phase 99 executor: apply `5b8e94da` with cross-target clippy gate (linux.rs is
cfg-gated; CLAUDE.md MUST rule).

### v3.2 Override Surface (PolicyOverrideApplied audit variant / EventIDs 10006-10010)

```bash
git log 1d1c88c9..d817ed53 -- crates/nono/src/audit.rs
# → (no output)
```

**Result: clean — no re-touch in window.**

No core-crate `nono` audit-variant changes in this window. The v3.2 `PolicyOverrideApplied`
audit variant and SecurityEventLayer EventIDs 10006-10010 (which live primarily in `nono-py`)
are not touched. Phase 99 executor: no override surface conflict.

---

## Empirical Cross-Check

Spot-check of 6 files via `git log --oneline <window> -- <file>` to validate cluster
classification accuracy (D-10). Includes all six carve-out path families (mandatory per D-07).

**Check 1: `crates/nono/src/audit.rs` → expect no hits (CR-02 carve-out path)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono/src/audit.rs
# → (no output)
```
Zero hits. **CORRECT** — CR-02 carve-out clean for this window.

**Check 2: `bindings/c/src/lib.rs` → expect no hits (CR-01 carve-out path)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- bindings/c/src/lib.rs
# → (no output)
```
Zero hits. **CORRECT** — CR-01 carve-out clean for this window.

**Check 3: `crates/nono-proxy/src/route.rs` → expect Clusters C and E (Cluster F carve-out path)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono-proxy/src/route.rs
# → cdeeb5b9 feat(proxy): add HTTP/2 support for reverse proxy and credential injection (#983)
# → c808f000 chore: migrate GitHub org references from always-further to nolabs-ai (#1235)
# → 08ca19a8 fix(proxy): match wildcard credential upstream routes (#1243)
```
Clusters C (cdeeb5b9, 08ca19a8) and E (c808f000). **CORRECT** — Cluster F hits recorded.

**Check 4: `crates/nono/src/sandbox/linux.rs` → expect Cluster D (Phase 95 fork invariant path)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono/src/sandbox/linux.rs
# → 5b8e94da fix(sandbox): warn when capability path is on a 9P filesystem (#1207)
```
Cluster D. **CORRECT** — additive; no AF_UNIX/seccomp conflict.

**Check 5: `crates/nono-cli/src/capability_ext.rs` → expect Cluster A (NetworkIntent)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono-cli/src/capability_ext.rs
# → 72bcfd66 refactor(network): introduce NetworkIntent and remove ProxyOnly placeholders (#1225)
```
Cluster A (72bcfd66). **CORRECT** — NetworkIntent refactor, windows-touch confirmed.

**Check 6: `crates/nono-cli/src/proxy_runtime.rs` → expect Clusters A and C (Cluster F carve-out path)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono-cli/src/proxy_runtime.rs
# → 46bcfbb9 fix(network): wire --allow-endpoint through to credential routes (#1127)
# → cdeeb5b9 feat(proxy): add HTTP/2 support for reverse proxy and credential injection (#983)
# → c808f000 chore: migrate GitHub org references from always-further to nolabs-ai (#1235)
# → 72bcfd66 refactor(network): introduce NetworkIntent and remove ProxyOnly placeholders (#1225)
```
Clusters A (72bcfd66), C (cdeeb5b9, 46bcfbb9), E (c808f000). **CORRECT** — Cluster F hits recorded.

All 6 spot-check results match expected clusters. No classification errors detected.

---

## ADR Review (first-pass scaffold — confirmed by Plan 02/03)

| dimension | score | notes |
|-----------|-------|-------|
| security | M | No security-critical bug-fixes in this window (unlike Phase 94's Cluster A AF_UNIX deadlock fix); Cluster A (#1225) restructures how ProxyOnly intent flows to the proxy — adopt-or-not affects proxy bypass risk; Cluster C (#983 H2) adds new proxy code surface; net: M |
| windows | H | Cluster A touches 7 files with cfg(windows) blocks; NetworkIntent adoption requires cross-target clippy on all touched files; tool-sandbox Platform/* are Linux/macOS only; net: H (Cluster A dominates) |
| maintenance | H | Cluster A is a significant CLI refactor (11 files, #1225 + #1263 dependency); Cluster C (#983) is a large multi-file proxy commit with tls_intercept/ (won't-apply) requiring careful split extraction; 14 commits across 8 clusters requires careful ordering in Phase 99; net: H |
| divergence | H | Cluster A (#1225) directly conflicts with fork's ProxyOnly placeholder pattern in capability_ext.rs (confirmed by grep); Cluster C overlaps Cluster F proxy carve-out; Cluster B entirely absent from fork (tool-sandbox/); net: H (#1225 is the headline conflict) |
| contributor | M | Cluster A requires an ADR decision (Plan 03) before Phase 99 can absorb; other clusters are more mechanical; net: M |
