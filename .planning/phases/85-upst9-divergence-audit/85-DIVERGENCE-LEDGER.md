---
phase: 85-upst9-divergence-audit
plan: 01
ledger_type: upst9-audit
range: 52809dda..01537570 (v0.62.0..v0.64.0)
range_note: "No v0.65.0 at audit-open — window unchanged (D-07 tip-check: clean)"
upstream_head_at_audit: 0153757001d21805a8218213e32add462d3322a1
refetch_date: 2026-06-19
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'bash scripts/check-upstream-drift.sh --from 52809dda --to 01537570 --format json'
fork_baseline: v0.62.0 (upstream SHA 52809dda; Phase 69 UPST8 sync point)
total_unique_commits: 35
date: 2026-06-19
---

## Headline

35 substantive commits in window `52809dda..01537570` (upstream `v0.62.0..v0.64.0`),
distributed across 14 ledger rows: clusters A through M plus a release/dep-bumps row.
Dispositions: `will-sync` 11 (A B C D E G H I J K L) / `split` 2 (F M) / `won't-sync` 1
(release/dep-bumps). Windows-touch clusters: **B** (diagnostics FFI — a6aa5995 adds
`bindings/c/src/diagnostic.rs` with 3 `pub extern "C"` fns; `bindings/c/src/types.rs` gains
`NonoDiagnosticCode` repr-C enum; Windows diagnostic output references these FFI types) — all
other clusters are `windows-touch: no`. Re-export scan (actual-diff, clusters A/B/F): Cluster A
adds `pub mod audit` to `crates/nono/src/lib.rs` (intra-cluster; no cross-cluster dep); Cluster
B adds `pub mod diagnostic` + `pub use diagnostic::*` and introduces 3 new `pub extern "C"`
functions in `bindings/c/src/diagnostic.rs` (ordering constraint: 4ad8ba92 → a6aa5995 in Phase
86 cherry-pick); Cluster F — `b5f8db5c` adds `pub use handle::...` re-export from `tls_intercept/`
which is absent in the fork (won't-apply). ADR review outcome: A=H, B=H, C=H (security dominant),
D=L, E=M, F=M, G=L, H=L, I=L, J=L, K=L, L=L, M=M. Confirm drain-then-sync shape; Phase 86 ADR
required for A/B boundary-convergence rationale. Downstream phase gates: A,B → Phase 86; C →
Phase 87; F → Phase 89; D,E,G,H,I,J,K,L,M,dep-bumps → Phase 88.

---

## Reproduction

```bash
# D-07 tip-check (2026-06-19): upstream/main = 0153757001d21805a8218213e32add462d3322a1
# git ls-remote upstream 'refs/tags/v0.65*' returned empty; window unchanged
git fetch upstream
git rev-parse upstream/main
# → 0153757001d21805a8218213e32add462d3322a1

# SHA collision guard: local fork v0.62.0 = 3c5e9025 (WRONG); upstream v0.62.0 = 52809dda (CORRECT)
# NEVER use tag names in git commands for this window; always use explicit SHAs
git cat-file -t 52809dda  # expect: commit
git cat-file -t 01537570  # expect: commit
# local fork v0.64.0 tag resolves to 0153757001... (same as upstream/main = v0.64.0) — safe to use SHA

# Drift tool sha pin
git log -1 --format=%H -- scripts/check-upstream-drift.sh
# → 0834aa664fbaf4c5e41af5debece292992211559

# Run drift tool
mkdir -p ci-logs-local/drift
bash scripts/check-upstream-drift.sh --from 52809dda --to 01537570 --format json \
  > ci-logs-local/drift/20260619T000000Z-v062-v064-upst9.json
# Output count: 35 (verified at execution time 2026-06-19)

# Full commit counts (verified at execution time)
git log --oneline 52809dda..01537570 | wc -l           # → 90
git log --merges --oneline 52809dda..01537570 | wc -l  # → 23
git log --no-merges --oneline 52809dda..01537570 | wc -l  # → 67

# Fork state: tls_intercept/ dir absent (Phase 34 C11 fork-preserve)
ls crates/nono-proxy/src/  # confirms no tls_intercept/ directory
```

**Auditor rerun:** Run the drift tool command above verbatim. The 35-commit JSON output matches
the per-cluster inventories in this ledger. Any new output indicates an upstream push after
`2026-06-19`; re-run the full audit if the count changes.

---

## Cluster Summary

| cluster_id | theme | commits | disposition | windows-touch | rationale |
|------------|-------|---------|-------------|---------------|-----------|
| A | Audit→core (attestation + ledger + clippy) | 4 | will-sync | no | D-03 locked; adopts upstream boundary change; Phase 86 |
| B | Diagnostics→core+FFI (refactor + structured model + blocked-op fix + deprecated-cmd fix) | 4 | will-sync | yes | D-03 locked; FFI surface adds 3 pub extern "C" fns; Phase 86 |
| C | AF_UNIX bypass + procfs-remap dedup | 2 | will-sync | no | Security fix (bypass closure); Linux cfg-gated; cross-target clippy required; Phase 87 |
| D | set_vars static env injection | 1 | will-sync | no | Additive; no conflict; Phase 88 |
| E | XDG state dirs (runtime state + config path) | 2 | will-sync | no | Windows-path reconciliation needed (XDG vs %LOCALAPPDATA%); Phase 88 |
| F | Proxy hardening (route/403/TLS-CONNECT/reactive-auth/intent-refactor/customCredentials) | 7 | split | no | tls_intercept/ absent in fork; additive route/403/server.rs bits sync; TLS-CONNECT and forward_inner_request require reconciliation; Phase 89 |
| G | AWS auth config | 1 | will-sync | no | Additive; also touches tls_intercept/handle.rs (won't-apply hunk); Phase 88 |
| H | Keyring timeout NONO_KEYRING_TIMEOUT_SECS | 1 | will-sync | no | Additive; no conflict; Phase 88 |
| I | $PACK_DIR session_hooks support | 1 | will-sync | no | Additive; no conflict; Phase 88 |
| J | PTY ctrl-z hang fix | 1 | will-sync | no | Additive; not in fork yet; Phase 88 |
| K | update-check CI environment discovery | 1 | will-sync | no | Additive; no conflict; Phase 88 |
| L | Profile namespace standardization | 1 | will-sync | no | Additive rename; no conflict; Phase 88 |
| M | Misc fixes (truthy env flags, macOS blocked grants, schema domain, sigstore removal, pack-verification, env_clear removal) | 6 | split | no | e54cf9cb env_clear Unix-only (Windows retains); other 5 additive; Phase 88 |
| Release/dep-bumps | v0.63.0 release, v0.64.0 release, typify bump | 3 | won't-sync | no | Version management; dep versions absorbed in Phase 88 DEPS-02 |

---

## Cluster A: Audit Integrity to Core Library

**Commits:** 4 — a5b2a516, aed35bec, 0b27cfc2, e9529312

**Disposition:** will-sync (D-03 locked)

**Windows-touch:** no

**Inspection depth:** actual-diff (git show) — required per D-05 (shared-surface boundary refactor)

**Rationale:** Upstream moves ~1773 LOC of audit/attestation logic from CLI-side
(`crates/nono-cli/src/audit_*.rs`) into the core `crates/nono/src/audit.rs`. The fork has
audit CLI-side (4 files: `audit_attestation.rs`, `audit_commands.rs`, `audit_integrity.rs`,
`audit_session.rs`; no `crates/nono/src/audit.rs` exists in the fork). D-03 locks this as
`will-sync` — the boundary change is the deliberate UPST9 decision. Phase 86 carries the full
boundary-convergence ADR (BND-03). The A commits belong to the `v0.63.0` release window.

**Cross-cluster re-export check (actual-diff):**

git show commands run for each commit:
```
git show a5b2a516 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show aed35bec | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show 0b27cfc2 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show e9529312 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings:
- `a5b2a516`: adds `pub mod audit` to `crates/nono/src/lib.rs` — introduces the new audit
  module into the core crate public API. CLI-side gains `pub(crate) use nono::audit::{AuditRecorder,
  verify_audit_log}` and `pub(crate) use nono::audit::{AUDIT_EVENTS_FILENAME, AUDIT_HASH_ALGORITHM,
  AuditEventRecord}`. Intra-cluster re-export only; no cross-cluster dep detected.
- `aed35bec`: no new `pub use`/`pub mod`/`extern crate` additions. Clean.
- `0b27cfc2`: adds `pub(crate) use nono::audit::AUDIT_ATTESTATION_BUNDLE_FILENAME` in CLI.
  Intra-cluster; no cross-cluster dep.
- `e9529312`: no new `pub use`/`pub mod`/`extern crate` additions. Clean (clippy fixes only).

**Finding:** No cross-cluster pub-use deps in Cluster A. lib.rs gains `pub mod audit`
(A-internal). Ordering note: `a5b2a516` must land before `aed35bec` and `0b27cfc2` since
those reference `nono::audit::*` symbols introduced in `a5b2a516`. Downstream phase: **Phase 86 (BND-01)**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| a5b2a516 | refactor(audit): move audit integrity logic to nono crate | v0.63.0 | audit | 3 files, 798+/396- | no |
| aed35bec | refactor(audit-ledger): move audit ledger logic to library crate | v0.63.0 | audit | 2 files, 507+/294- | no |
| 0b27cfc2 | refactor(audit): move attestation logic to core library (#1148) | v0.63.0 | audit | 2 files, 531+/273- | no |
| e9529312 | fix(audit): address ledger review and clippy | v0.63.0 | audit | 1 file, 21+/25- | no |

---

## Cluster B: Structured Diagnostics to Core and FFI

**Commits:** 4 — 4ad8ba92, a6aa5995, f867aba2, 7f319b9e

**Disposition:** will-sync (D-03 locked)

**Windows-touch:** yes (a6aa5995 adds `bindings/c/src/diagnostic.rs` with 3 `pub extern "C"` fns;
`bindings/c/src/types.rs` gains `NonoDiagnosticCode` repr-C enum; Windows diagnostic output
references these FFI types)

**Inspection depth:** actual-diff (git show) — required per D-05 (shared-surface + FFI additions;
Pitfall 3 closure: explicit `git show a6aa5995 -- bindings/c/src/` run)

**Rationale:** Upstream's B refactor: (1) moves diagnostic UX OUT of core (4ad8ba92, v0.63.0);
(2) adds a richer structured diagnostic model back as `crates/nono/src/diagnostic/` module
directory (a6aa5995, v0.64.0); (3) fixes blocked-operation reporting (f867aba2, v0.63.0); (4)
fixes deprecated nono-learn CLI string (7f319b9e, v0.64.0 — not in SEED-006; folded into B
because it touches `crates/nono-cli/src/diagnostic/formatter.rs`, a diagnostic CLI surface).
The fork has `crates/nono/src/diagnostic.rs` (single file); upstream replaces it with a module
directory (`codes.rs`, `observation.rs`, `records.rs`, `report.rs`, `detail.rs`, `mod.rs`).
`crates/nono-proxy/src/diagnostic.rs` and `bindings/c/src/diagnostic.rs` are both net-new in
theme B (neither exists in fork today). D-03 locked `will-sync`.

**Cross-cluster re-export check (actual-diff, Pitfall 3 closure):**

git show commands run for each commit:
```
git show 4ad8ba92 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show a6aa5995 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show a6aa5995 -- bindings/c/src/   # explicit FFI check (Pitfall 3)
git show f867aba2 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show 7f319b9e | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings:
- `4ad8ba92`: adds `pub use formatter::{...}` (CLI diagnostic) and `pub use diagnostic::{DenialReason,
  DenialRecord, IpcDenialRecord, SandboxViolation}` (core re-export in CLI). This is a refactor
  moving types; no new cross-cluster dep (B-internal migration).
- `a6aa5995`: adds `pub mod diagnostic` and `pub use diagnostic::*` to `crates/nono/src/lib.rs`.
  In CLI: `pub(crate) fn suggested_flag_for_remediation(rem: &nono::NonoRemediation)` — references
  the new `nono::NonoRemediation` type from the diagnostic module. In proxy lib.rs: `pub use
  diagnostic::{ProxyDiagnostic, ProxyDiagnosticCode, ProxyDiagnosticSeverity}`. In core lib.rs:
  `pub use codes::*`, `pub use detail::*`, `pub use observation::*`, `pub use records::*`,
  `pub use report::*`. **FFI surface (Pitfall 3 closure):** `bindings/c/src/diagnostic.rs` is
  created with these `pub extern "C"` functions: `nono_last_diagnostic_code()` → `NonoDiagnosticCode`,
  `nono_last_remediation_json()` → `*mut c_char`, and `pub unsafe extern "C" fn
  nono_session_diagnostic_report_to_json(...)`. These reference types from the new `diagnostic`
  module. **Ordering constraint:** the diagnostic module (`a6aa5995`) must land before any consumer
  that references `nono::NonoRemediation`, `ProxyDiagnostic*`, or the FFI diagnostic code functions.
- `f867aba2`: no new `pub use`/`pub mod`/`extern crate` additions. Clean.
- `7f319b9e`: no new `pub use`/`pub mod`/`extern crate` additions. Clean (string replacement only).

**Finding:** Cross-cluster re-export dep detected within B: `a6aa5995` adds `pub mod diagnostic`
+ `pub use diagnostic::*` to core lib.rs and 3 new `pub extern "C"` FFI functions. These are
B-internal (the diagnostic module IS the B cluster) but create an ordering constraint for Phase 86:
Phase 86 cherry-pick sequence: `4ad8ba92` → `f867aba2` → `a6aa5995` → `7f319b9e`. `4ad8ba92`
first because it migrates the existing diagnostic surface out of core (prerequisite for the new
module layout). `a6aa5995` third because CLI and FFI consumers reference it. No external
cross-cluster dep (B→A or B→C). Downstream phase: **Phase 86 (BND-02)**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 4ad8ba92 | refactor(diagnostic): move diagnostic UX out of core nono crate (#1155) | v0.63.0 | other | 8 files, 3910+/3944- | no |
| a6aa5995 | feat(diagnostics): expose structured diagnostics for library and FFI clients (#1171) | v0.64.0 | other | 30 files, 2907+/478- | yes |
| f867aba2 | fix: report the actual blocked operation instead of the readable target path (#1150) | v0.63.0 | other | 3 files, 144+/24- | no |
| 7f319b9e | fix(diagnostic): replace deprecated nono learn with nono run (#1170) | v0.64.0 | other | 1 file, 8+/6- | no |

Note on 7f319b9e: **Not in SEED-006**; folded into Cluster B because it touches
`crates/nono-cli/src/diagnostic/formatter.rs` (a diagnostic CLI surface). Standalone string
replacement; no structural change.

---

## Cluster C: AF_UNIX Datagram Bypass and Procfs-Remap Dedup

**Commits:** 2 — e2086877, 6b3eb013

**Disposition:** will-sync (D-04 lean confirmed by diff — security fix, must absorb)

**Windows-touch:** no (both commits are `#[cfg(target_os = "linux")]` only)

**Inspection depth:** --name-only (Linux security fix; no re-export risk in Linux-gated blocks)

**Rationale:** `e2086877` closes AF_UNIX datagram bypass in `crates/nono/src/sandbox/linux.rs`
(#1096 — traps sendto/sendmsg to prevent sandbox escape via AF_UNIX datagrams). `6b3eb013`
guards `deduplicate()` against inheriting procfs-remap originals (#1064). Both are security fixes
on Linux cfg-gated code; both are absent from the fork (confirmed at research time). D-04 lean
confirmed: security=H dominates; will-sync. Empirical check: `git log 52809dda..01537570 --
crates/nono/src/sandbox/linux.rs` returns only `e2086877` (correct cluster).

**Phase 87 executor cross-target clippy note:** `e2086877` and `6b3eb013` touch
`#[cfg(target_os = "linux")]` blocks. Per CLAUDE.md MUST/NEVER rule, the Phase 87 executor
MUST run `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D
clippy::unwrap_used` AND `--target x86_64-apple-darwin`. Windows-host `cargo check` is NOT
an accepted substitute. If cross-toolchain unavailable, mark affected REQs PARTIAL and defer
to live CI per `.planning/templates/cross-target-verify-checklist.md`.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| e2086877 | fix(linux): trap sendto/sendmsg to prevent AF_UNIX datagram bypass (#1096) | v0.64.0 | other | 4 files, 684+/138- | no |
| 6b3eb013 | fix: guard deduplicate() against inheriting procfs-remap originals (#1064) | v0.63.0 | other | 1 file, 90+/2- | no |

---

## Cluster D: set_vars Static Environment Injection

**Commits:** 1 — d48aeb7b

**Disposition:** will-sync (D-04 lean confirmed — additive, no conflict)

**Windows-touch:** no

**Inspection depth:** --name-only (additive feature; no re-export risk)

**Rationale:** Adds `set_vars` for static environment variable injection across CLI exec_strategy
and sandbox layers. Confirmed absent from fork. Purely additive; no fork-conflict surface.
Downstream phase: **Phase 88**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| d48aeb7b | feat(environment): add set_vars for static env injection (#1134) | v0.64.0 | other | 13 files, 434+/5- | no |

---

## Cluster E: XDG State Directories

**Commits:** 2 — e8293b36, 8e0d94f9

**Disposition:** will-sync (D-04 lean confirmed — additive XDG convention; Windows reconciliation note)

**Windows-touch:** no (neither commit touches `*_windows.rs`; Windows-path reconciliation needed)

**Inspection depth:** --name-only (additive state-paths module; no re-export risk)

**Rationale:** `e8293b36` introduces `crates/nono-cli/src/state_paths.rs` (+422 LOC, absent in fork)
and moves runtime state to XDG state dirs (`$XDG_STATE_HOME` / `~/.local/state`). `8e0d94f9`
uses XDG config paths consistently. The fork's Windows v3.0 provisioner resolves runtime paths to
`%LOCALAPPDATA%\nono\` — the Phase 88 executor must reconcile `user_state_dir()` (XDG convention)
with the Windows LOCALAPPDATA path. This is a Windows-path reconciliation need, not a blocking conflict.
D-04 lean confirmed. Downstream phase: **Phase 88**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| e8293b36 | feat(cli): move runtime state to XDG state dirs (#1152) | v0.63.0 | other | 21 files, 876+/263- | no |
| 8e0d94f9 | fix(cli): use XDG config paths consistently (#1179) | v0.64.0 | other | 22 files, 228+/55- | no |

---

## Cluster F: Proxy Hardening

**Commits:** 7 — b0b2c743, a5d623fd, b5f8db5c, 7c9abd3b, 76b7b695, bd4b6b7f, 724bb207

**Disposition:** split (D-04 lean confirmed — additive route/403/server.rs bits sync;
tls_intercept/ bits won't-apply; fork has no `tls_intercept/` directory)

**Windows-touch:** no

**Inspection depth:** actual-diff (git show per commit) — required per D-05 (fork-divergent TLS surface;
Phase 34 C11 fork-preserve)

**Rationale:** The fork does NOT carry `crates/nono-proxy/src/tls_intercept/` (verified at
execution time: `ls crates/nono-proxy/src/` confirms no `tls_intercept/` dir — Phase 34 C11
fork-preserve). Upstream's proxy hardening touches both shared surfaces (`route.rs`, `server.rs`,
`config.rs`, `credential.rs`, `proxy_runtime.rs`, `network_policy.rs`) and the fork-absent
`tls_intercept/handle.rs` and `tls_intercept/mod.rs`. The `split` disposition means: sync the
shared-surface changes; do not apply tls_intercept/ changes. Downstream phase: **Phase 89 (PROXY-01/02)**.

**Cross-cluster re-export check (actual-diff per commit):**

git show commands run:
```
git show b0b2c743 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show b5f8db5c | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show bd4b6b7f | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
# (a5d623fd, 7c9abd3b, 76b7b695, 724bb207 returned no pub-use additions)
```

Findings per commit:
- `b0b2c743`: files — `config.rs`, `route.rs`, `tls_intercept/handle.rs`. Adds `pub(crate) enum
  RouteSelection<'a>` and `pub(crate) fn select_route<'a>()` in `route.rs`. The `tls_intercept/handle.rs`
  hunk is won't-apply (absent in fork). Shared-surface `route.rs` and `config.rs` hunks: sync-safe.
- `a5d623fd`: files — `server.rs` only. No new `pub use`/`pub mod` additions. Sync-safe (additive
  403+audit return for denied non-CONNECT requests).
- `b5f8db5c`: files — `server.rs`, `tls_intercept/handle.rs`, `tls_intercept/mod.rs`. Adds `pub use
  handle::{InterceptCtx, InterceptUpstreamProxy, handle_intercept_connect}` in `tls_intercept/mod.rs`
  — this re-export is won't-apply (entire tls_intercept/ module absent in fork). The `server.rs`
  hunk (upstream_proxy in TLS CONNECT path) requires reconciliation against fork's CONNECT implementation.
- `7c9abd3b`: files — `server.rs` only. No new `pub use`/`pub mod` additions. Reactive proxy auth
  keep-connection-open fix — sync-safe pending diff reconciliation against fork's server.rs.
- `76b7b695`: files — `tls_intercept/handle.rs` only. Entire commit is won't-apply (tls_intercept/
  absent in fork). Refactor of `forward_inner_request` lives entirely inside the missing module.
- `bd4b6b7f`: files — `command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`,
  `proxy_runtime.rs`, `supervised_runtime.rs` (all CLI). Adds `pub(crate)` structs:
  `DomainFilterIntent`, `EndpointFilterIntent`, `CredentialProxyIntent`, `UpstreamProxyIntent`,
  `TlsInterceptIntent`, `OpenUrlIntent`. The `TlsInterceptIntent` struct references the tls_intercept
  feature; these are CLI intent types. Phase 89 must assess whether `TlsInterceptIntent` can be
  defined in the fork without the backing `tls_intercept/` module.
- `724bb207`: files — `network_policy.rs`, `proxy_runtime.rs`. No new `pub use`/`pub mod` additions.
  Fixes proxy activation with `customCredentials` — touches `proxy_runtime.rs` activation path.
  Fork carries `credential.rs`; sync-safe pending diff review of credential activation path.

**Finding:** `76b7b695` is fully won't-apply (tls_intercept/ only). `b5f8db5c`'s tls_intercept
hunks are won't-apply; server.rs hunk needs reconciliation. `b0b2c743`'s route.rs and config.rs
hunks are sync-safe; tls_intercept/handle.rs hunk won't-apply. `bd4b6b7f`'s `TlsInterceptIntent`
struct needs Phase 89 review. `a5d623fd`, `7c9abd3b`, `724bb207` are sync-safe.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| b0b2c743 | fix(proxy): stop allow_domain endpoint route from shadowing credential catch-all (#1132) | v0.63.0 | proxy | 3 files, 246+/119- | no |
| a5d623fd | fix(proxy): return 403 + audit for denied non-CONNECT requests (#1077) | v0.63.0 | proxy | 1 file, 107+/3- | no |
| b5f8db5c | fix(proxy): respect upstream_proxy in TLS CONNECT intercept path (#1048) (#1091) | v0.63.0 | proxy | 3 files, 149+/4- | no |
| 7c9abd3b | fix(proxy): keep connection open for reactive proxy auth on CONNECT (#1151) | v0.63.0 | proxy | 1 file, 209+/43- | no |
| 76b7b695 | Refactor forward_inner_request (#1192) | v0.64.0 | proxy | 1 file, 137+/88- | no |
| bd4b6b7f | refactor(proxy): separate proxy intent from activation (#1199) | v0.64.0 | proxy | 5 files, 303+/80- | no |
| 724bb207 | fix: proxy should activate with customCredentials set (#1197) | v0.64.0 | proxy | 2 files, 217+/3- | no |

---

## Cluster G: AWS Authentication Config

**Commits:** 1 — 5bb098cd

**Disposition:** will-sync (D-04 lean confirmed — additive AWS auth feature)

**Windows-touch:** no

**Inspection depth:** --name-only (additive feature; primary surface is config/credential/route/network_policy)

**Rationale:** Implements `aws_auth` config for AWS credential injection in the proxy. Additive
feature across `network_policy.rs`, `profile/mod.rs`, `proxy/config.rs`, `proxy/credential.rs`,
`proxy/reverse.rs`, `proxy/route.rs`, `proxy/server.rs`. Note: `5bb098cd` also touches
`crates/nono-proxy/src/tls_intercept/handle.rs` (adds AWS auth header injection path); that hunk
is won't-apply in the fork (tls_intercept/ absent). The core `route.rs`, `credential.rs`,
`server.rs` hunks are sync-safe. Phase 88 executor must verify the tls_intercept hunk scope
matches AWS auth behavior on the fork's non-TLS proxy path. Downstream phase: **Phase 88**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 5bb098cd | feat: [aws] implement aws_auth config (#1166) | v0.64.0 | other | 9 files, 833+/59- | no |

---

## Cluster H: Keyring Timeout

**Commits:** 1 — c6b13345

**Disposition:** will-sync (D-04 lean confirmed — additive env var)

**Windows-touch:** no

**Inspection depth:** --name-only (additive env var; no structural change)

**Rationale:** Adds `NONO_KEYRING_TIMEOUT_SECS` for configurable keychain access timeout in
`crates/nono/src/keystore.rs`. Confirmed absent from fork. Purely additive.
Downstream phase: **Phase 88**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| c6b13345 | feat(keyring): add NONO_KEYRING_TIMEOUT_SECS for keychain access (#977) | v0.63.0 | other | 2 files, 298+/69- | no |

---

## Cluster I: $PACK_DIR Session Hooks

**Commits:** 1 — 7d274cf7

**Disposition:** will-sync (D-04 lean confirmed — additive hook feature)

**Windows-touch:** no

**Inspection depth:** --name-only (additive env var injection; no structural change)

**Rationale:** Adds `$PACK_DIR` support to `session_hooks` for store pack support. Touches
`crates/nono-cli/src/hook_runtime.rs` (Unix path — same file as Cluster M's e54cf9cb). Phase 88
executor must sequence I before M to avoid a rebase conflict on `hook_runtime.rs`. Additive.
Downstream phase: **Phase 88**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 7d274cf7 | feat: add $PACK_DIR support to session_hooks for store pack support (#1073) | v0.63.0 | other | 3 files, 1002+/78- | no |

---

## Cluster J: PTY Ctrl-Z Hang Fix

**Commits:** 1 — 4179ce03

**Disposition:** will-sync (additive; not in fork yet)

**Windows-touch:** no

**Inspection depth:** --name-only (PTY-specific fix; no structural change)

**Rationale:** Fixes ctrl-z hang when running with a PTY (`crates/nono-cli/src/pty_proxy.rs`).
Confirmed absent from fork. Additive. Downstream phase: **Phase 88**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 4179ce03 | fix(pty): ctrl-z hangs when running with a PTY (#1135) | v0.64.0 | other | 2 files, 289+/9- | no |

---

## Cluster K: Update-Check CI Environment Discovery

**Commits:** 1 — cc11b389

**Disposition:** will-sync (D-04 lean confirmed — additive CI detection)

**Windows-touch:** no

**Inspection depth:** --name-only (additive feature; no structural change)

**Rationale:** Adds `detect_ci_provider()` to `crates/nono-cli/src/update_check.rs`. Confirmed
absent from fork. Additive. Downstream phase: **Phase 88**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| cc11b389 | feat(update-check): discover ci environments on update (#1113) | v0.64.0 | other | 2 files, 197+/4- | no |

---

## Cluster L: Profile Namespace Standardization

**Commits:** 1 — 6d88638e

**Disposition:** will-sync (D-04 lean confirmed — rename refactor)

**Windows-touch:** no

**Inspection depth:** --name-only (rename refactor across profile files; no structural change)

**Rationale:** Standardizes profile names with namespace convention across 30 files (120+/266-).
Additive rename; fork profiles use the pre-namespace naming. Downstream phase: **Phase 88**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 6d88638e | refactor(profiles): standardize profile names with namespace | v0.63.0 | profile | 30 files, 120+/266- | no |

---

## Cluster M: Miscellaneous Fixes

**Commits:** 6 — 42e5bf73, a0bba5eb, ee7a3bda, 7e076d2d, 9800f307, e54cf9cb

**Disposition:** split (D-04 lean confirmed — e54cf9cb env_clear Unix-only carve-out; others additive)

**Windows-touch:** no (none of the 6 commits touch `*_windows.rs`; see e54cf9cb annotation)

**Inspection depth:** --name-only (misc additive fixes; env_clear carve-out noted in inventory)

**Rationale:** Five of six commits are additive misc fixes: truthy env flag parsing (42e5bf73),
macOS blocked grants display (a0bba5eb), schema domain replacement (ee7a3bda), sigstore provenance
removal (7e076d2d), pack-verification dry-run skip (9800f307). The sixth, `e54cf9cb`, removes
`env_clear()` from `crates/nono-cli/src/hook_runtime.rs` — the Unix hook path only. This is a
`split` because the Windows path (`hook_runtime_windows.rs`) RETAINS `env_clear()` plus
`SystemRoot`, `windir`, `SystemDrive` baseline restore to prevent CLR init failure (`0xFFFF0000` /
`-65536`). Downstream phase: **Phase 88**.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 42e5bf73 | fix(cli): accept truthy env values for bool flags (#1136) | v0.64.0 | other | 2 files, 108+/3- | no |
| a0bba5eb | feat(output): show blocked macos grants in capability summary (#1178) | v0.64.0 | other | 4 files, 136+/26- | no |
| ee7a3bda | fix: replace stale nono.dev schema domains with nono.sh | v0.63.0 | other | 3 files, 3+/3- | no |
| 7e076d2d | refactor(pull_ui): remove sigstore provenance display (#1144) | v0.64.0 | other | 1 file, 5+/108- | no |
| 9800f307 | feat(pack-verification): skip pack verification on dry runs | v0.63.0 | other | 2 files, 53+/19- | no |
| e54cf9cb | Remove env_clear from the session_hook subprocess | v0.63.0 | other | 1 file, 1+/2- | no |

**SPLIT annotation for e54cf9cb:** PARTIAL-SYNC — apply `env_clear()` removal to
`hook_runtime.rs` (Unix path only); `hook_runtime_windows.rs` RETAINS `env_clear()` plus
`SystemRoot`/`windir`/`SystemDrive` baseline restore per `windows_hook_interpreter_spawn_gotchas`
to prevent CLR init failure (`0xFFFF0000` / `-65536`). Do NOT apply e54cf9cb to the Windows
hook path. Phase 88 executor: cherry-pick `7d274cf7` (Cluster I) before `e54cf9cb` to avoid
rebase conflict on `hook_runtime.rs`.

---

## Release/Dep-Bumps

**Commits:** 3 — 084638b7, 01537570, 6d452b00

**Disposition:** won't-sync (version management; dep versions absorbed in Phase 88 DEPS-02)

**Windows-touch:** no

**Inspection depth:** --name-only (release metadata; dep version bumps)

**Rationale:** Release commits (`084638b7` = v0.63.0, `01537570` = v0.64.0) bump CHANGELOG,
Cargo.toml versions, and release metadata — not cherry-picked into the fork (fork uses its own
version leapfrog convention per `project_v28_opened`). `6d452b00` bumps `typify` from `0.6.2`
to `0.7.0` — the dep version target is absorbed as part of Phase 88 DEPS-02 dep-version review,
not via direct cherry-pick of this commit.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 084638b7 | chore: release v0.63.0 (#1161) | v0.63.0 | other | 6 files, 112+/10- | no |
| 01537570 | chore: release v0.64.0 (#1201) | v0.64.0 | other | 6 files, 80+/10- | no |
| 6d452b00 | chore(deps): bump typify from 0.6.2 to 0.7.0 (#1156) | v0.63.0 | package | 2 files, 24+/24- | no |

---

## ADR Review

Overall phase ADR assessment across five standard dimensions (per AUDIT-02 rubric):

| dimension | score | notes |
|-----------|-------|-------|
| security | H | Cluster C (e2086877 AF_UNIX datagram bypass closure — real security fix); Cluster A (moves audit integrity into core, security-critical primitive); additive features in D/G/H/I/K/L pose negligible security risk; net: H driven by C + A |
| windows | H | Cluster B (a6aa5995 adds bindings/c/src/diagnostic.rs with 3 new pub extern "C" fns — Windows diagnostic output depends on these FFI types); Cluster E (XDG vs %LOCALAPPDATA% reconciliation needed); Cluster M (e54cf9cb Windows CLR baseline retention required via hook_runtime_windows.rs); net: H driven by B FFI + M carve-out requirement |
| maintenance | H | Clusters A and B are major structural boundary changes (audit + diagnostics moved into core); these create an ongoing reconciliation surface for future upstream syncs; Cluster F (split disposition) leaves a controlled fork surface around tls_intercept/; net: H driven by A+B structural weight |
| divergence | H | A and B deliberately change the policy-free-library boundary (D-03 locked will-sync); these are the highest divergence points in the fork's history. F split adds a controlled fork surface. M split is small. Net: H driven by A+B boundary convergence |
| contributor | H | A and B require ADR-level decisions (Phase 86 BND-03 boundary-convergence ADR); F requires Phase 89 proxy reconciliation plan; C requires cross-target clippy gate in Phase 87. High human review overhead for A+B; net: H |

**Per-cluster risk matrix:**

| Cluster | security | windows | maintenance | divergence | contributor | Overall |
|---------|----------|---------|-------------|------------|-------------|---------|
| A | H | L | H | H | H | H |
| B | M | H | H | H | H | H |
| C | H | L | L | L | L | H (security dominant) |
| D | L | L | L | L | L | L |
| E | L | M | M | L | L | M |
| F | M | L | M | M | M | M |
| G | L | L | L | L | L | L |
| H | L | L | L | L | L | L |
| I | L | L | L | L | L | L |
| J | L | L | L | L | L | L |
| K | L | L | L | L | L | L |
| L | L | L | L | L | L | L |
| M | L | M | L | L | L | M (env_clear collision) |
| Release/dep-bumps | L | L | L | L | L | L |

**Outcome:** Confirm. Drain-then-sync shape remains valid. Phase 86 ADR required for A/B
boundary-convergence rationale (BND-03 — what stays CLI-side, what moves to core). Clusters
A and B represent the highest-risk boundary changes and must be sequenced before C/D/E/G-L/M
to avoid merge ordering conflicts. Cluster F (split) proceeds independently in Phase 89 with
a TLS-intercept reconciliation plan.

---

## Empirical Cross-Check

Spot-check of 6 files via `git log` walk to confirm cluster classification accuracy:

**Check 1: `crates/nono/src/audit.rs` → expect Cluster A only**
```
git log --oneline 52809dda..01537570 -- crates/nono/src/audit.rs
0b27cfc2 refactor(audit): move attestation logic to core library (#1148)
e9529312 fix(audit): address ledger review and clippy
aed35bec refactor(audit-ledger): move audit ledger logic to library crate
a5b2a516 refactor(audit): move audit integrity logic to nono crate
```
Result: All 4 commits are Cluster A. **CORRECT.**

**Check 2: `crates/nono/src/diagnostic/` → expect Cluster B only**
```
git log --oneline 52809dda..01537570 -- crates/nono/src/diagnostic/
a6aa5995 feat(diagnostics): expose structured diagnostics for library and FFI clients (#1171)
```
Result: 1 commit, Cluster B. **CORRECT.**

**Check 3: `crates/nono/src/sandbox/linux.rs` → expect Cluster C only**
```
git log --oneline 52809dda..01537570 -- crates/nono/src/sandbox/linux.rs
e2086877 fix(linux): trap sendto/sendmsg to prevent AF_UNIX datagram bypass (#1096)
```
Result: 1 commit, Cluster C. **CORRECT.**

**Check 4: `crates/nono-proxy/src/route.rs` → expect Cluster F and possibly G**
```
git log --oneline 52809dda..01537570 -- crates/nono-proxy/src/route.rs
5bb098cd feat: [aws] implement aws_auth config (#1166)
b0b2c743 fix(proxy): stop allow_domain endpoint route from shadowing credential catch-all (#1132)
```
Result: `b0b2c743` (Cluster F) and `5bb098cd` (Cluster G). **CORRECT** — G's AWS auth
touches route.rs as a secondary file. Classification stands; tls_intercept hunk in 5bb098cd
is won't-apply per Cluster G rationale.

**Check 5: `crates/nono/src/keystore.rs` → expect Cluster H only**
```
git log --oneline 52809dda..01537570 -- crates/nono/src/keystore.rs
c6b13345 feat(keyring): add NONO_KEYRING_TIMEOUT_SECS for keychain access (#977)
```
Result: 1 commit, Cluster H. **CORRECT.**

**Check 6: `crates/nono-cli/src/hook_runtime.rs` → expect Clusters I and M**
```
git log --oneline 52809dda..01537570 -- crates/nono-cli/src/hook_runtime.rs
7d274cf7 feat: add $PACK_DIR support to session_hooks for store pack support (#1073)
e54cf9cb Remove env_clear from the session_hook subprocess
```
Result: `7d274cf7` (Cluster I) and `e54cf9cb` (Cluster M). **CORRECT** — both touch
`hook_runtime.rs`. Phase 88 executor must sequence I before M.

No classification errors detected. All 6 spot-check results match expected clusters.

---

## Cross-cluster Re-export Deps Detected

Summary of actual-diff re-export findings from clusters A, B, and F:

**Cluster A:** `a5b2a516` adds `pub mod audit` to `crates/nono/src/lib.rs` and CLI gains
`pub(crate) use nono::audit::*` re-exports. These are all A-internal (the audit module IS the
A cluster). No cross-cluster dep where A's symbols are consumed by B, C, or other clusters.
**Finding: No cross-cluster re-export dep in Cluster A.**

**Cluster B:** `a6aa5995` adds `pub mod diagnostic` + `pub use diagnostic::*` to core lib.rs,
and in proxy lib.rs adds `pub use diagnostic::{ProxyDiagnostic, ProxyDiagnosticCode,
ProxyDiagnosticSeverity}`. In `bindings/c/src/diagnostic.rs` (net-new file): 3 `pub extern "C"`
functions reference the new diagnostic module types (Pitfall 3 closure verified).
**Ordering constraint detected:** The diagnostic module must land (`a6aa5995`) before any consumer
that references `nono::NonoRemediation`, `ProxyDiagnostic*`, or the FFI diagnostic code functions.
Phase 86 cherry-pick sequence: `4ad8ba92` → `f867aba2` → `a6aa5995` → `7f319b9e`.
`4ad8ba92` first because it migrates the existing diagnostic surface out of core (prerequisite
for the new module layout). Proxy `ProxyDiagnostic*` types added — proxy lib.rs gains a diagnostic
re-export surface that is net-new in the fork.
**Finding: Cluster B has an intra-B ordering constraint; no external cross-cluster dep from B to A or C.**

**Cluster F:** `76b7b695` (tls_intercept/ only) is entirely won't-apply. `b5f8db5c` adds
`pub use handle::{InterceptCtx, InterceptUpstreamProxy, handle_intercept_connect}` inside
`tls_intercept/mod.rs` — won't-apply (tls_intercept/ absent in fork). `bd4b6b7f` adds
`TlsInterceptIntent` struct in CLI proxy_runtime — Phase 89 must assess whether `TlsInterceptIntent`
can be defined in the fork without the backing `tls_intercept/` module. No cross-cluster dep (F to A or F to B).
**Finding: Cluster F — two won't-apply pub-use re-exports (tls_intercept/ absent). No cross-cluster dep.**

**Overall verdict:** Re-export scan clean for cross-cluster deps. Intra-B ordering constraint
identified (Phase 86 must sequence 4ad8ba92 before a6aa5995). Cluster F's tls_intercept
re-exports are won't-apply per fork state. No A to B or B to A circular dep detected.

---

## TLS-Intercept Assessment

Fork state: `crates/nono-proxy/src/tls_intercept/` does NOT exist (verified at execution time:
`ls crates/nono-proxy/src/` confirms no `tls_intercept/` dir). Fork proxy surface: `audit.rs`,
`config.rs`, `connect.rs`, `credential.rs`, `error.rs`, `external.rs`, `filter.rs`, `lib.rs`,
`oauth2.rs`, `reverse.rs`, `route.rs`, `server.rs`, `token.rs`. This is the Phase 34 C11
`fork-preserve` surface — the fork chose a non-TLS-intercepting proxy architecture.

**Cluster F commit split verdict:**

| sha | verdict | reason |
|-----|---------|--------|
| a5d623fd | sync-safe | `server.rs` only; additive 403+audit return for denied non-CONNECT; no tls_intercept dep |
| 7c9abd3b | sync-safe (needs server.rs review) | `server.rs` only; reactive proxy auth keep-connection-open; no tls_intercept dep; review for conflict with fork's CONNECT implementation |
| 724bb207 | sync-safe (needs proxy_runtime review) | `proxy_runtime.rs`, `network_policy.rs`; customCredentials activation fix; review credential activation path in fork |
| b0b2c743 | partial-sync | `route.rs` + `config.rs` hunks sync-safe; `tls_intercept/handle.rs` hunk won't-apply |
| bd4b6b7f | partial-sync | CLI proxy_runtime intent refactor syncs; `TlsInterceptIntent` struct requires Phase 89 assessment (can it exist without backing module?) |
| b5f8db5c | partial-sync | `server.rs` hunk needs reconciliation against fork CONNECT path; `tls_intercept/handle.rs` + `tls_intercept/mod.rs` hunks won't-apply |
| 76b7b695 | won't-apply | `tls_intercept/handle.rs` only; entire commit is absent-module surface |

**Phase 89 guidance:** Apply `a5d623fd`, `7c9abd3b`, `724bb207` as sync candidates (shared
surface only). For `b0b2c743`, `bd4b6b7f`, `b5f8db5c`: extract only the non-tls_intercept hunks
(`route.rs`, `server.rs`, CLI proxy_runtime). Skip `76b7b695` entirely. Assess `TlsInterceptIntent`
in `bd4b6b7f` — if it can be defined without a backing tls_intercept module, include it.

---

## Excluded as Noise

**Filter criteria:** The drift tool applies these path inclusion filters:
- `crates/nono/src/`
- `crates/nono-cli/src/`
- `crates/nono-proxy/src/`
- `crates/nono/Cargo.toml`

Exclusion patterns (even within included paths): `*_windows.rs`,
`crates/nono-cli/src/exec_strategy_windows/`.

Any commit not touching included paths after exclusion patterns is noise. Additionally, all
merge commits are excluded regardless of touched files.

**Counts:**
- Total commits in window: 90
- Substantive (drift-tool filter): 35
- Noise total: **55** = 23 merge commits + 32 non-merge out-of-filter commits

**23 merge commits (all excluded):**
`849cda42`, `c3dc6a42`, `a9f3aefe`, `bb01eb1c`, `0d066b60`, `46f7b23e`, `addc818a`, `172ffffe`,
`a1726551`, `a4709b2e`, `dd518afb`, `2d96b45c`, `e8d16b62`, `4d2359da`, `2f996687`, `6490c209`,
`40f29137`, `e566a067`, `e1d977b9`, `bfb29c1a`, `42b2b783`, `b91d45aa`, `6deb1664`

**32 non-merge out-of-filter commits:**
`89c3785f` (ci: ubuntu integration tests),
`5413a0b3` (fix(policy): go_runtime go-build cache — touches `crates/nono-cli/data/policy.json`
  only; policy.json is NOT in the drift tool path filter; this commit appears in SEED-006 theme M
  but is NOISE per the drift tool D-11 path filter; go-build cache group change deferred to a
  future policy.json sync pass, not UPST9),
`79090c2e` (cbindgen bump 0.29.3 to 0.29.4),
`23b457b8` (which bump 8.0.2 to 8.0.3),
`aad901f6` (docs(allow-cwd)),
`e06f4628` (chore: 3-day Dependabot cooldown),
`4bced737` (docs: credential-injection anchor),
`b24c72fc` (docs: networking),
`3f8d24ad` (docs: install),
`2f4af7d2` (docs: quickstart link),
`8bf56083` (chore: agents.md),
`a793fe2f` (ignore bump 0.4.25 to 0.4.26),
`d603e103` (chrono bump 0.4.44 to 0.4.45),
`d2fc3a6b` (time bump 0.3.47 to 0.3.49),
`cca68d99` (zeroize bump 1.8.2 to 1.9.0),
`5a34a459` (chore: issue template),
`f7836bbe` (docs: macOS diagnostics.suppress_system_services),
`54c551eb` (docs: readme update),
`ec76e15f` (docs: readme refine),
`ad4a26d6` (docs: readme link),
`bf547151` (readme improvement),
`196f911b` (ci: actions/attest),
`314bd74e` (test: WSL2 landlock V4+ detection — touches `tests/integration/test_wsl2.sh` only),
`abae954e` (docs: copr install instructions),
`bba57157` (chore: actions/checkout bump),
`5a257179` (fix: cargo vendor copr srpms),
`2b374525` (build: copr source rpm packaging),
`1be86838` (fix(aur): ssh-keyscan),
`d1fc7d69` (cbindgen bump 0.29.2 to 0.29.3),
`a8ccd160` (hyper bump 1.9.0 to 1.10.1),
`6181a1f9` (docs: cli-quickstart),
`e58de4d6` (x509-parser bump 0.16.0 to 0.18.1)

**Dep-bump note:** The dependency version targets in the above noise commits
(`a793fe2f`, `d603e103`, `d2fc3a6b`, `cca68d99`, `79090c2e`, `23b457b8`, `d1fc7d69`,
`a8ccd160`, `e58de4d6`) touch only `Cargo.lock` and non-core manifests. These dep version
targets are absorbed as part of Phase 88 DEPS-02 dependency review, not cherry-picked as
individual commits.

**Completeness verification:** 35 substantive + 55 noise = 90 total. Matches
`git log --oneline 52809dda..01537570 | wc -l` = 90. Every commit in the window is accounted
for in exactly one of: cluster per-commit tables (35) or this noise section (55).

---

## Phase 87 CR-02 Addendum

**Added:** 2026-06-20 (Phase 87 execution — CR-02 fork hardening)

| Field | Value |
|-------|-------|
| File | `crates/nono/src/audit.rs` |
| Line (fork) | ~1413 (`records_verified` in `verify_audit_log` return struct) |
| Upstream reference commit | `e9529312` |
| Upstream behavior | `records_verified: true` — hardcoded regardless of `event_count` |
| Fork behavior after Phase 87 | `records_verified: event_count > 0` |
| Reason | Audit-integrity bypass: an empty log with `stored: None` returned `is_valid() = true`, falsely signalling "all integrity checks passed" when no records were verified |
| Classification | Deliberate fork-divergence — security hardening |
| ADR | `proj/ADR-87-cr02-audit-bypass.md` |

**Future sync note:** When upstream `e9529312` (or any commit that touches this return block)
reappears during a sync window, the line `records_verified: true` WILL conflict with the fork's
`records_verified: event_count > 0`. This conflict is **expected and intentional**. Sync
auditors must preserve the fork's expression. Do not revert to `records_verified: true`.

The regression test `verify_empty_log_with_no_stored_metadata_is_not_valid` in
`crates/nono/src/audit.rs` (Phase 87) guards against unintentional reversion.

---

## Phase 88 CR-01 Addendum

**Added:** 2026-06-20 (Phase 88 execution — CR-01 FFI stale-diagnostic-state fix)

| Field | Value |
|-------|-------|
| Files | `bindings/c/src/diagnostic.rs` + `bindings/c/src/lib.rs` (and all other `bindings/c/src/` FFI entry points) |
| Fork lines | `nono_session_diagnostic_report_to_json()` entry (~line 43), `nono_merge_diagnostic_report_json()` entry (~line 97), plus 14 other `pub unsafe extern "C"` entry points across `capability_set.rs`, `fs_capability.rs`, `sandbox.rs`, `state.rs`, `query.rs` |
| Upstream reference commit | `a6aa9995` (absorbed in Phase 86 boundary convergence; CR-01 extends the pattern) |
| Upstream behavior | Upstream added the diagnostic module at `a6aa9995`; the stale-code pattern in `set_last_error`-only paths was not addressed at that commit |
| Fork behavior after Phase 88 | `clear_last_call_state()` called at every `pub unsafe extern "C"` fn entry — resets `LAST_ERROR`, `LAST_DIAGNOSTIC_CODE`, and `LAST_REMEDIATION_JSON` atomically before any operation that could set a thread-local |
| Reason | 86-REVIEW.md CR-01 finding: `nono_session_diagnostic_report_to_json` and `nono_merge_diagnostic_report_json` called `set_last_error()` without first resetting `LAST_DIAGNOSTIC_CODE`, leaving C callers with stale diagnostic codes from a prior FFI call on the same thread |
| Classification | Deliberate fork-divergence — correctness hardening |
| Commit | `db0f221d` |

**Future sync note:** When any upstream commit touches the `bindings/c/src/` entry-point bodies
(particularly `nono_session_diagnostic_report_to_json` and `nono_merge_diagnostic_report_json`),
the `crate::clear_last_call_state();` call at entry MUST be preserved. Any upstream commit
removing or reordering these entry-point clears must be evaluated carefully — the stale-code
correctness invariant is the fork's deliberate addition. Do not remove `clear_last_call_state()`
calls without a corresponding upstream fix for the same stale-state class.

The regression test `diagnostic_code_is_cleared_between_calls` in
`bindings/c/src/diagnostic.rs` (Phase 88) guards against unintentional reversion.
