---
phase: 54-upst7-audit
plan: 01
ledger_type: upst7-audit
range: v0.57.0..v0.59.0
upstream_head_at_audit: 48d39f3635f339e439d43869f8c98bc1db9b6dc1
refetch_date: 2026-06-04
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'make check-upstream-drift ARGS="--from v0.57.0 --to v0.59.0 --format json"'
fork_baseline: v0.57.0 (Phase 48 UPST6 sync point — 42 commits across v0.55.0..v0.57.0 absorbed 2026-05-25)
total_unique_commits: 40
date: 2026-06-04
---

# Phase 54 UPST7 Audit — Upstream v0.57.0..v0.59.0 Divergence Ledger

## Headline

UPST7 audits **40 unique upstream commits** (drift-tool count; the 260527-sgo gap analysis
under-counted at ~19) across `v0.57.0..v0.59.0`, grouped into **14 clusters**. Disposition
breakdown: **will-sync 8** (C3, C4, C6, C7, C9, C10, C11, C12), **split 3** (C2 supervisor IPC,
C5 TLS-intercept ordering, C8 session hooks), **fork-preserve 0**, **won't-sync 3** (C1 release
commits, C13 sigstore bump → reclassified split on review, C14 macOS-only). **windows-touch:yes**
clusters: **2** (C2 supervisor IPC, C8 session hooks) — both `split`, with Windows-equivalent work
routed to dedicated phases (59, 58). The highest-divergence item is the SC4 TLS-intercept ordering
fix (`22e6c40`): the fork's `RouteStore`/`CredentialStore` decoupling already satisfies the
ordering intent, so the verdict is **fork-preserve** (+ a small-additive-port rider coupled to
Phase 56's allow_domain absorption). Cross-cluster re-export scan: **clean** for `pub use`
surfaces; one **function-call** prereq detected (C5 → C3, `partition_allow_domain`). ADR review
outcome: **(a) confirm Option A 'continue'**. REQ-UPST7-01 satisfied: 40/40 commits dispositioned.

**v0.60.0 scope:** Range stays `v0.57.0..v0.59.0` per the locked SC. The 2026-06-04 re-fetch
surfaced **v0.60.0 (`9a05a4ff`), v0.61.0, and v0.61.1** — all OUT OF RANGE and deferred to
**UPST8** (the deferred set is `v0.60.0..v0.61.1`, larger than the v0.60.0-alone set the plan
anticipated). These are NOT the unrelated Feb-2026 v0.6.x tag line. Decision + rationale: defer
(human-confirmed 2026-06-04) — keeps Phase 54/55 scope stable; UPST8 absorbs the post-v0.59.0 set.

## Reproduction

- **Invocation (verbatim):** `make check-upstream-drift ARGS="--from v0.57.0 --to v0.59.0 --format json"`
  (Windows-host fallback used: `bash scripts/check-upstream-drift.sh --from v0.57.0 --to v0.59.0 --format json`)
- **JSON output:** `ci-logs-local/drift/20260604T212024Z-v057-v059.json` (gitignored, NOT committed)
- **upstream_head_at_audit:** `48d39f3635f339e439d43869f8c98bc1db9b6dc1`
- **refetch_date:** `2026-06-04`
- **drift_tool sha pin:** `0834aa664fbaf4c5e41af5debece292992211559` (asserted == before run)
- **total_unique_commits:** 40 (source of truth; merges excluded by the tool)
- **tag split:** 28 commits in `v0.57.0..v0.58.0` (tag v0.58.0), 12 in `v0.58.0..v0.59.0` (tag v0.59.0)
- **auditor-rerun:** re-fetch upstream tags, assert the `drift_tool_sh_sha` pin, re-run the
  invocation against `upstream_head_at_audit`; the same range + HEAD reproduces the 40-commit set.

## Cluster Summary

| cluster_id | theme | commits | disposition | windows-touch | rationale |
|------------|-------|---------|-------------|---------------|-----------|
| C1 | Release commits (v0.58/v0.59) | 2 | won't-sync | no | Version bumps are fork-managed; CHANGELOG-ride only |
| C2 | Supervisor named-socket IPC (URL-open #959) | 9 | split | yes | Cross-platform keep-alive/timeout logic ports; AF_UNIX socket mechanism is unix-only, Windows uses AIPC (fork-preserve) → Phase 59 |
| C3 | allow_domain path+method | 2 | will-sync | no | Fine-grained network filtering; cross-platform CLI/proxy → Phase 56 |
| C4 | proxy 502 hardening | 2 | will-sync | no | `connect.rs` cross-platform security hardening → Phase 55 |
| C5 | TLS-intercept ordering (SC4) + rcgen bump | 2 | split | no | Ordering intent already satisfied by fork RouteStore (fork-preserve); proxy_runtime port couples to C3; rcgen is in upstream-only `tls_intercept/` → Phase 56 |
| C6 | Bitwarden `bw://` credential source | 2 | will-sync | no | `keystore.rs` cross-platform credential source → Phase 57 |
| C7 | Profile: JSONC + target_binary + opencode + refactors | 5 | will-sync | no | Profile system cross-platform straight ports → Phase 55 |
| C8 | Session lifecycle hooks (#954) | 2 | split | yes | Upstream `hook_runtime` is unix-only (explicitly gated by `1335351`); Windows needs ADR/equiv; schema cross-platform → Phase 58 |
| C9 | pack-update-hint robustness | 2 | will-sync | no | Atomic state writes + detached refresh; cross-platform → Phase 55 |
| C10 | diagnostic / output / denial polish | 4 | will-sync | no | Cross-platform diagnostics polish → Phase 55 |
| C11 | timeout constants | 3 | will-sync | no | Centralized configurable timeouts + overflow checks; cross-platform → Phase 55 |
| C12 | policy test (ENV_LOCK) | 1 | will-sync | no | Test-only ENV_LOCK hardening; cross-platform → Phase 55 |
| C13 | sigstore dep bump 0.8.0 | 1 | split | no | Cargo.toml bump ports; `scrub.rs` change vs fork Phase-49 trust-root surface needs verify → Phase 55 |
| C14 | macOS-only sandbox fixes | 3 | won't-sync | no | `unix/macOS-only N/A` per REQUIREMENTS § Out of Scope |

### Cluster C1: Release commits

**Commits:** 2 — `chore: release v0.58.0`, `chore: release v0.59.0`
**Disposition:** won't-sync
**Windows-touch:** no
**Rationale:** Both touch only `crates/nono/Cargo.toml` version bumps. The fork manages its own
versioning (MSI version flows from the git tag; crate version stays fork-controlled). Per the
release-ride convention, CHANGELOG entries may be referenced but the Cargo.toml/Cargo.lock version
bumps are dropped.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 54c4deb | chore: release v0.58.0 | v0.58.0 | other | 1 | no |
| e61814f | chore: release v0.59.0 | v0.59.0 | other | 1 | no |

### Cluster C2: Supervisor named-socket IPC (URL-open helpers, #959)

**Commits:** 9 — fd-based IPC → named socket for URL-open helpers (`be7681c` foundation), plus
keep-alive/timeout/blocking-mode hardening on the supervisor listener.
**Disposition:** split
**Windows-touch:** yes
**Rationale:** The cross-platform-core portions (supervisor loop keep-alive conditions, read
timeouts, blocking-mode) carry conceptual intent the fork can absorb, but the mechanism is an
**AF_UNIX named socket** (`crates/nono/src/supervisor/socket.rs`) which is unix-only — the fork's
Windows supervisor IPC uses AIPC/named-pipe (`exec_strategy_windows/`), a divergent surface that
must be **fork-preserved**. Routed to **Phase 59** (REQ-IPC-01: supervisor IPC robustness), which
"may touch supervisor IPC cross-platform-core portions" per the ROADMAP.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| be7681c | fix: replace fd-based IPC with named socket for URL open helpers (#959) | v0.58.0 | other | 6 | yes |
| f956fb6 | fix: set accepted listener connections to blocking mode | v0.58.0 | other | 1 | yes |
| 9820a2e | fix: include URL listener in supervisor loop keep-alive conditions | v0.58.0 | other | 1 | yes |
| 51f56b8 | fix: keep supervisor loop alive when child closes direct IPC socket | v0.58.0 | other | 1 | yes |
| d1851c9 | fix: increase supervisor listener read timeout to 5s for URL open | v0.58.0 | other | 3 | yes |
| c15c76a | fix: address review comments on supervisor socket IPC | v0.58.0 | other | 3 | yes |
| 284ae1d | fix: add read timeout on accepted listener connections | v0.58.0 | other | 1 | yes |
| 4a22e94 | fix: grant UnixSocketCapability for supervisor socket in child sandbox | v0.58.0 | other | 1 | yes |
| ed47520 | style: format debug message for line length | v0.58.0 | other | 1 | no |

### Cluster C3: allow_domain path+method (fine-grained network filtering)

**Commits:** 2 — URL-with-path endpoint restriction + fine-grained method+path restrictions.
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Cross-platform CLI + proxy filtering feature (REQ-NET-01). The marquee absorption,
routed to **Phase 56**. `0ced085` is the large foundation (12 files); `75b2265` is the URL-path
parser. The fork's `RouteStore` already has `endpoint_rules: CompiledEndpointRules`, so the
proxy-side target exists; the CLI-side `partition_allow_domain` is what this cluster introduces.
**Cross-cluster re-export check:** Clean — diff-inspected lead commit `0ced085` (`git show 0ced085`)
for `pub use` / `pub mod` / `extern crate` / `pub(crate)`; only a new intra-cluster
`pub(crate) fn merge_allow_domain` definition, no cross-cluster re-export dep. (Note: C5 has a
*function-call* dependency ON this cluster — see C5 + Cross-cluster summary.)

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 0ced085 | feat(cli): support fine-grained method+path restrictions in allow_domain (#960) | v0.59.0 | other,profile | 12 | no |
| 75b2265 | feat(cli): allow-domain accepts URL with path for endpoint restriction | v0.59.0 | other | 3 | no |

### Cluster C4: proxy 502 hardening

**Commits:** 2 — return 502 + audit entry on upstream connect failure; preserve upstream error and
sanitise the 502 reason line.
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Both touch `crates/nono-proxy/src/connect.rs` (cross-platform). Security/robustness
hardening of the proxy's failure path → **Phase 55**.
**Cross-cluster re-export check:** Clean — diff-inspected lead commit `d11193f` (`git show d11193f`);
no `pub use`/`pub mod`/`extern crate`/`pub(crate)` re-export of any other-cluster symbol.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| d11193f | fix(proxy): return 502 with audit entry on upstream connect failure | v0.58.0 | proxy | 1 | no |
| 4ad708d | fix(proxy): preserve upstream error and sanitise 502 reason line | v0.58.0 | proxy | 1 | no |

### Cluster C5: TLS-intercept ordering (SC4) + rcgen bump

**Commits:** 2 — enforce endpoint rules before credential selection in TLS intercept (`22e6c40`);
rcgen 0.13.2→0.14.8 bump (`8e78daf`, in `tls_intercept/`).
**Disposition:** split
**Windows-touch:** no
**Rationale:** `22e6c40` has two parts: (1) `tls_intercept/handle.rs` ordering logic — the fork
does **not** carry `tls_intercept/`; its `RouteStore`/`CredentialStore` separation already enforces
endpoint-before-credential structurally → **fork-preserve**; (2) a `proxy_runtime.rs` filter-
allowlist snippet that depends on `partition_allow_domain` + `endpoint_routes` (cluster C3) → small
additive port coupled to Phase 56. `8e78daf` bumps rcgen inside `tls_intercept/ca.rs`+`cert_cache.rs`
(fork lacks the module) → won't-sync. See the dedicated **## TLS-intercept clean-apply assessment**.
**Cross-cluster re-export check:** N/A for `pub use` (split, not will-sync), BUT a **function-call**
cross-cluster dep exists: `22e6c40`'s proxy_runtime.rs portion calls `network_policy::partition_allow_domain`
(introduced by C3). **Prerequisite enumeration:** C5 (proxy_runtime port) → prereq **C3** (allow_domain),
symbols `partition_allow_domain` / `endpoint_routes`. `credential.rs` is NOT touched by `22e6c40`
(byte-identical preservation trivially honored).

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 22e6c40 | fix(proxy): enforce endpoint rules before credential selection in TLS intercept | v0.59.0 | other,proxy | 2 | no |
| 8e78daf | chore(deps): bump rcgen from 0.13.2 to 0.14.8 | v0.58.0 | proxy | 2 | no |

### Cluster C6: Bitwarden `bw://` credential source

**Commits:** 2 — add Bitwarden credential source (`bw://` URI scheme); Zeroizing for item fields.
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** `crates/nono/src/keystore.rs` + profile/credential runtime — cross-platform
credential source (REQ-CRED-01) → **Phase 57**. Zeroize hygiene aligns with the fork's secure-memory
standard.
**Cross-cluster re-export check:** Clean — diff-inspected lead commit `47bbd8c` (`git show 47bbd8c`);
no cross-cluster `pub use`/`pub mod`/`extern crate`/`pub(crate)` re-export.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 47bbd8c | feat: add Bitwarden credential source (bw:// URI scheme) | v0.58.0 | other,profile | 4 | no |
| f28a115 | fix(keystore): use Zeroizing<String> for Bitwarden item fields and in-place truncation | v0.58.0 | other | 1 | no |

### Cluster C7: Profile system (JSONC + target_binary + opencode + refactors)

**Commits:** 5 — JSONC profile parsing, target_binary profile field, opencode profile extraction,
chained-if-let refactor, review fixes.
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Cross-platform profile-system straight ports → **Phase 55**. `9398a13` (target_binary)
also touches `policy.rs` (cross-platform, not Windows-specific); `53a0c52` (JSONC) adds `pub(crate) fn resolve_user_profile_path`.
**Cross-cluster re-export check:** Clean — diff-inspected lead commits `9398a13` + `53a0c52`
(`git show`); the `pub(crate) fn resolve_user_profile_path` (53a0c52) and `pub(crate) fn is_file_path_ref`
(e15aa53) are intra-cluster definitions, not cross-cluster re-exports.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 9398a13 | feat(profile): allow profiles to specify a target binary | v0.58.0 | other,policy,profile | 4 | no |
| 53a0c52 | feat(profile): add JSONC support for profile files | v0.58.0 | other,profile | 4 | no |
| 2bd9b4d | refactor(profile): extract opencode profile from built-ins | v0.59.0 | other,profile | 5 | no |
| cfa24f3 | refactor: use chained if let for conditional statements | v0.58.0 | other,profile | 2 | no |
| e15aa53 | fix: review fixes | v0.58.0 | other,profile | 3 | no |

### Cluster C8: Session lifecycle hooks (#954)

**Commits:** 2 — session lifecycle hooks feature; gate `hook_runtime` module unix-only.
**Disposition:** split
**Windows-touch:** yes
**Rationale:** `daa55c8` adds session hooks; `1335351` **explicitly gates `hook_runtime` unix-only**
and drops dead non-unix branches — i.e. upstream's hook runtime does not run on Windows. The schema
+ profile-surface portions are cross-platform (split-able into Phase 55/58), but the runtime needs a
**Windows-equivalent ADR** (REQ-HOOK-01) → **Phase 58** (the highest-risk UPST7 phase per the ROADMAP).
**Cross-cluster re-export check:** N/A (split, not will-sync).

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| daa55c8 | feat: session lifecycle hooks (#954) | v0.58.0 | other,profile | 6 | yes |
| 1335351 | refactor(hook_runtime): gate module unix-only, drop dead non-unix branches | v0.58.0 | other | 3 | yes |

### Cluster C9: pack-update-hint robustness

**Commits:** 2 — atomic state-file writes; refresh in a detached process to avoid threads.
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Cross-platform robustness of the pack-update-hint surface → **Phase 55**.
**Cross-cluster re-export check:** Clean — diff-inspected lead commit `74fbbf1` (`git show 74fbbf1`);
no cross-cluster re-export.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 74fbbf1 | refactor(pack-hints): refresh in detached process to avoid threads | v0.58.0 | other | 6 | no |
| b1a650a | fix(pack-update-hint): make state file writes atomic | v0.58.0 | other | 1 | no |

### Cluster C10: diagnostic / output / denial polish

**Commits:** 4 — rfind access-mode split (+test), pre-compute canonical denial paths, bold-only-path
in footer, annotate suppressed denials (#984).
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Cross-platform diagnostic/output polish → **Phase 55**. `a606b5b` touches
`diagnostic.rs` + `exec_strategy.rs`; `7cb315c` touches `diagnostic.rs` + output/save-prompt.
**Cross-cluster re-export check:** Clean — diff-inspected lead commit `7cb315c` (`git show 7cb315c`);
no cross-cluster re-export.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 7cb315c | fix: annotate suppressed denials and style save prompt paths (#984) | v0.59.0 | other | 3 | no |
| a606b5b | diagnostic: pre-compute canonical denial paths to avoid repeated fs I/O | v0.59.0 | other | 2 | no |
| 668e341 | fix: use rfind for access mode spliting; add test | v0.59.0 | other | 1 | no |
| 8fd8da0 | Bold only path in diagnostic footer, not access type or labels | v0.59.0 | other | 1 | no |

### Cluster C11: timeout constants

**Commits:** 3 — centralize timeout constants + user-facing timeouts; tighten overflow checks;
formatting.
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Cross-platform; centralizes `timeouts.rs` and makes user-facing timeouts configurable
→ **Phase 55**. `194788e` is the large foundation (8 files incl. `pty_proxy.rs`, `session_commands.rs`).
**Cross-cluster re-export check:** Clean — diff-inspected lead commit `194788e` (`git show 194788e`);
no cross-cluster re-export.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 194788e | feat(cli): centralize timeout constants and make user-facing timeouts configurable | v0.59.0 | other | 8 | no |
| 69af73d | fix: tighten up overflow checks | v0.59.0 | other | 2 | no |
| 1442818 | fix: formatting | v0.59.0 | other | 1 | no |

### Cluster C12: policy test (ENV_LOCK)

**Commits:** 1 — lock `ENV_LOCK` in `test_all_groups_no_deny_within_allow_overlap`.
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Test-only hardening of `policy.rs` (ENV_LOCK serialization, aligns with the fork's
EnvVarGuard discipline) → **Phase 55**.
**Cross-cluster re-export check:** Clean — diff-inspected lead commit `1a764d0` (`git show 1a764d0`);
test-only, no cross-cluster re-export.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 1a764d0 | test: lock ENV_LOCK in test_all_groups_no_deny_within_allow_overlap | v0.58.0 | policy | 1 | no |

### Cluster C13: sigstore dep bump 0.8.0

**Commits:** 1 — update sigstore crates to 0.8.0.
**Disposition:** split
**Windows-touch:** no
**Rationale:** Touches `crates/nono/Cargo.toml` (dep bump — portable) AND `crates/nono/src/scrub.rs`.
The fork's sigstore surface is the Phase-49 POC trust-root resilience work (`--from-file`, fixture
cadence, `trusted_root.json`). The Cargo.toml bump is a will-sync straight port, but the `scrub.rs`
change must be **diff-verified against the fork's Phase-49 trust-root surface** before applying →
split (Cargo bump → Phase 55; `scrub.rs` → verify-then-port). Not a clean blind cherry-pick.
**Cross-cluster re-export check:** N/A (split).

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| e581569 | chore(deps): update sigstore crates to 0.8.0 | v0.58.0 | other | 2 | no |

### Cluster C14: macOS-only sandbox fixes

**Commits:** 3 — emit platform rules after user write allows (macOS); `$PWD` symlink-CWD capture;
preserve symlink path when adding CWD capability on macOS.
**Disposition:** won't-sync
**Windows-touch:** no
**Rationale:** All three are macOS/unix Seatbelt-specific (`sandbox/macos.rs`, `sandbox_prepare.rs`
symlink-CWD handling). `unix/macOS-only N/A` per REQUIREMENTS § Out of Scope — the Windows IL
backend has no equivalent symlink-CWD/Seatbelt-ordering surface. The fork's macOS backend is
already fork-preserved from prior phases; these are not Windows-parity items.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 8f84d45 | fix(macos): emit platform rules after user write allows | v0.58.0 | other | 1 | no |
| 362ada2 | fix(sandbox): use $PWD to capture symlink CWD without --workdir | v0.58.0 | other | 1 | no |
| 8f1b0b7 | fix(sandbox): preserve symlink path when adding CWD capability on macOS | v0.58.0 | other | 1 | no |

## ADR review

Phase 33 ADR (`docs/architecture/upstream-parity-strategy.md`, Option A `continue`, Accepted
2026-05-11, re-confirmed at the v2.4 + v2.5 + v2.6 closes) is the subject of this review. Phase 54
UPST7 is the **40-commit v0.58/v0.59 evidence base** (Phase-47 scale, ~42), with the SC4
TLS-intercept surface as the highest-divergence item. Phase 54 verdicts the ADR but does **NOT**
supersede it (stays Accepted).

| dimension | verdict | rationale |
|-----------|---------|-----------|
| security | M | Material security/robustness fixes available to absorb (proxy 502 hardening C4, allow_domain method+path C3, bw:// credential source C6, TLS-intercept ordering C5). All routed to will-sync/dedicated phases — continuing absorption keeps the fork current on cross-platform security; deferring would accrue security debt. Moderate pressure, well-mitigated by the disposition plan. |
| windows | L | Only 2/14 clusters are windows-touch:yes (C2 supervisor IPC, C8 session hooks), both `split` with Windows-equivalent work already scoped to Phases 59/58. No new Windows-specific divergence introduced by this set; the macOS-only C14 is N/A. Low. |
| maintenance | M | 40 commits (Phase-47 scale), but 8 will-sync clusters are low-labor straight ports; 3 splits need care; v0.60.0..v0.61.1 already pending (growing backlog). Manageable per-cycle cost; the cadence keeps each cycle bounded. Moderate. |
| divergence | M | Predominantly will-sync (convergent). Structural divergences (C2 supervisor AIPC, C5 TLS-intercept architecture, C8 unix-only hook runtime, C14 macOS Seatbelt) are all pre-existing, owned, and fork-preserved — no NEW structural fork introduced. Stable moderate. |
| contributor | L | The fork consumes upstream here (absorption, not divergence); contribute-back continues via the Phase 55 umbrella-PR pattern (project_cross_fork_pr_pattern). Low friction. |

**Outcome:** (a) Confirm. Phase 33 ADR Option A 'continue' remains the right call — the evidence
(mostly will-sync, manageable splits routed to dedicated phases, zero new high-severity structural
divergence) supports continuing the parity strategy. No carve-outs required; no future-supersede
trigger flagged this cycle.

## Empirical cross-check

Spot-checks fork-shared files against the upstream `v0.57.0..v0.59.0` log to detect any commits the
drift tool's D-11 path filter (excludes `*_windows.rs` + `crates/nono-cli/src/exec_strategy_windows/`)
may have missed, and to confirm fork-divergent surfaces. Closes the feedback_cluster_isolation_invalid
lesson empirically (≥4 files).

### File: crates/nono-proxy/src/route.rs
- Walked upstream log: `git log v0.57.0..v0.59.0 -- crates/nono-proxy/src/route.rs`
- Commits touching this file in range: **0**
- Cluster mapping: none — `route.rs` is a **fork-original** surface (upstream uses `tls_intercept/` + `forward.rs`).
- Drift-tool coverage: **PASS** — no upstream commits to miss; corroborates the SC4 fork-preserve verdict (the ordering fix lives in upstream's `tls_intercept/`, which the fork replaces with `route.rs`).

### File: crates/nono-proxy/src/credential.rs
- Walked upstream log: `git log v0.57.0..v0.59.0 -- crates/nono-proxy/src/credential.rs`
- Commits touching this file in range: **0**
- Cluster mapping: none — fork's Phase-09/11 Windows credential-injection rewrite is divergent.
- Drift-tool coverage: **PASS** — upstream made no changes to absorb here; byte-identical preservation is safe.

### File: crates/nono/src/keystore.rs
- Walked upstream log: `git log v0.57.0..v0.59.0 -- crates/nono/src/keystore.rs`
- Commits touching this file in range: **2** (`47bbd8c` bw://, `f28a115` Zeroizing)
- Cluster mapping: C6 (Bitwarden bw://).
- Drift-tool coverage: **PASS** — both present in the 40-commit drift inventory.

### File: crates/nono-cli/src/profile/mod.rs
- Walked upstream log: `git log v0.57.0..v0.59.0 -- crates/nono-cli/src/profile/mod.rs`
- Commits touching this file in range: **14** (8 non-merge + 6 merge commits)
- Cluster mapping: C3 (`0ced085`), C6 (`47bbd8c`), C7 (`2bd9b4d`, `cfa24f3`, `e15aa53`, `9398a13`, `53a0c52`), C8 (`daa55c8`).
- Drift-tool coverage: **PASS** — all 8 non-merge commits are in the drift inventory; the 6 merge commits (`df8ff3b2`, `fe233db4`, `7263d2bc`, `4b9fb80d`, `ef3f3fc5`, `4d14ccec`) are correctly excluded by the drift tool (merges are not unique commits).

### File: crates/nono-cli/src/platform.rs
- Walked upstream log: `git log v0.57.0..v0.59.0 -- crates/nono-cli/src/platform.rs`
- Commits touching this file in range: **0**
- Cluster mapping: none — confirms there is **no java-dev / JDK-paths cluster** in this range (the plan's java-dev mention came from the broader gap analysis, not v0.57..v0.59).
- Drift-tool coverage: **PASS** — no missed commits.

**Result:** 5 files walked (≥4 required); zero drift-tool gaps; no missed upstream commits; no
follow-up drift-tool-fix quick-task required.

## Cross-cluster re-export deps detected

Diff-inspect scan (`git show <lead-sha>` for added `pub use` / `pub mod` / `extern crate` /
`pub(crate)`) run on all **8 will-sync** cluster lead commits (C3, C4, C6, C7, C9, C10, C11, C12).

- **`pub use` / `pub mod` / `extern crate` re-export deps:** **none detected.** The only additions
  surfaced were three intra-cluster `pub(crate) fn` *definitions* (`merge_allow_domain` in C3,
  `resolve_user_profile_path` + `is_file_path_ref` in C7) — function-visibility, not cross-cluster
  re-exports. No will-sync cluster was flipped to `split` on re-export grounds. The Phase 43
  `8b888a1c` cross-cluster-isolation trap does **not** recur in this range.

- **Function-call cross-cluster dep (surfaced by the SC4 diff-inspect, not the pub-use scan):**
  - **C5 (TLS-intercept ordering) → C3 (allow_domain):** `22e6c40`'s `proxy_runtime.rs` portion calls
    `network_policy::partition_allow_domain` + consumes `endpoint_routes`, both introduced by C3.
    Recorded as the C5 prerequisite enumeration. Implication for Phase 55/56: absorb C3 (allow_domain)
    **before** porting the C5 `proxy_runtime.rs` filter-allowlist snippet.

## TLS-intercept clean-apply assessment (Phase 34 C11)

**Subject commit:** `22e6c40` — *fix(proxy): enforce endpoint rules before credential selection in
TLS intercept* (v0.59.0). Also relevant: `8e78daf` (rcgen bump inside `tls_intercept/`).

**Phase 34 C11 precedent:** the TLS-interception surface was `fork-preserve`d at Phase 34; a blind
cherry-pick of the upstream `9300de9` pattern previously hit **9 conflicts + 4 modify/delete** and
was escalated to D-20 manual-replay. The fork carries `route.rs` + `connect.rs` + `credential.rs` +
`reverse.rs`; upstream carries `tls_intercept/` + `forward.rs` + `audit_ledger.rs` (the fork has
**no** `tls_intercept/` module — confirmed via `ls crates/nono-proxy/src/tls_intercept/` → absent).

**Diff-inspect findings (`git show 22e6c40`):**
1. **`tls_intercept/handle.rs` (+57):** the actual ordering logic — "endpoint-only routes must gate
   access before credential catch-all routes." The fork's `RouteStore` (`route.rs`) **already**
   decouples this: its doc states *"a route can enforce endpoint restrictions without injecting any
   credential"*, with a pre-compiled `endpoint_rules: CompiledEndpointRules` field; `RouteStore` and
   `CredentialStore` are separate keyed stores. The ordering INTENT is therefore **already
   structurally satisfied** in the fork — there is no `tls_intercept/handle.rs` to patch.
2. **`proxy_runtime.rs` (+12):** adds endpoint-restricted domains to the proxy filter allowlist so
   upstream connections succeed after TLS interception. This snippet calls
   `network_policy::partition_allow_domain` + iterates `endpoint_routes` — both from **C3
   (allow_domain)**, which is **not yet in the fork** (`grep partition_allow_domain crates/` → absent).
   So this is a **small additive port that must ride WITH the Phase 56 allow_domain absorption**, not
   a standalone cherry-pick.
3. **`credential.rs`:** **NOT touched** by `22e6c40` (`git show 22e6c40 --stat` shows only
   `proxy_runtime.rs` + `tls_intercept/handle.rs`). The Phase-09/11 Windows credential-injection
   rewrite (SHA `c9f25164` invariant) is preserved byte-identical — no proposal regresses it.

**Verdict:** fork-preserve. The fork's already-decoupled `RouteStore`/`CredentialStore` design
satisfies the endpoint-before-credential ordering intent; the upstream `tls_intercept/handle.rs`
change targets a module the fork does not carry (do **not** import it). The only portable artifact
is the 12-line `proxy_runtime.rs` filter-allowlist snippet — a **small-additive-port** rider coupled
to C3, to be applied during Phase 56's allow_domain work (after `partition_allow_domain` exists).
`rcgen` bump (`8e78daf`) is won't-sync (lives in the absent `tls_intercept/`). This is the
diff-inspect note **Phase 56 (REQ-NET-01)** requires before implementing the ordering fix.
