---
phase: 94-upst10-divergence-audit
plan: 01
ledger_type: upst10-audit
range: 0153757001..1d1c88c9 (v0.64.0..v0.65.1)
range_note: "Window covers v0.64.1 (0551eba27e), v0.65.0 (137bb15c56), v0.65.1 (1d1c88c9f9). No new tag past v0.65.1 at audit-open (D-08 trigger not fired)."
upstream_head_at_audit: 1d1c88c9f98f0a1f3ff79cff1509713aaec7cdb0
refetch_date: 2026-06-26
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'bash scripts/check-upstream-drift.sh --from 0153757001 --to 1d1c88c9 --format json > ci-logs-local/drift/20260626T003610Z-v064-v0651-upst10.json'
fork_baseline: v0.64.0 (SHA 0153757001; Phase 85 UPST9 endpoint — byte-identical to former always-further/nono head)
total_unique_commits: 8
date: 2026-06-26
---

## Headline

8 substantive commits in window `0153757001..1d1c88c9` (upstream `v0.64.0..v0.65.1`),
distributed across 4 ledger clusters: A through D. Dispositions: `will-sync` 1 (A) /
`split` 2 (B, C) / `won't-sync` 1 (D — release/dep-bumps). Windows-touch clusters: none
(all 8 substantive commits are `windows-touch: no`). Re-export scan (actual-diff, all clusters):
Cluster B (`11fd10e0`) adds numerous `pub(crate)` types and re-exports inside `crates/nono-cli/src/`
(tool-sandbox machinery) plus adds `SandboxRuntimeAuditEvent` and `CommandPolicyAuditEvent` to
`crates/nono/src/audit.rs` (additive — touches CR-02 carve-out path); Cluster B also modifies
`crates/nono-proxy/src/route.rs`, `server.rs`, `tls_intercept/ca.rs`, `tls_intercept/cert_cache.rs`,
`tls_intercept/handle.rs` (tls_intercept/ hunks won't-apply — Cluster F ongoing fork-preserve).
Cluster C (`9b37dc52`) modifies `proxy_runtime.rs` in a way that directly conflicts with the
Phase 89 Cluster F deliberate divergence (`0c08e5d2`). The headline fork-specific deliverable is
the **Carve-out Re-touch Check**: Cluster B hits both CR-02 (`crates/nono/src/audit.rs`) and
the Cluster F carve-out (`crates/nono-proxy/src/route.rs`, `server.rs`, `proxy_runtime.rs`);
CR-01 (`bindings/c/src/`) is clean. Phase 95 must handle both hits with preserve-fork-expression
guidance. Downstream routing: A → Phase 95 (will-sync); B → Phase 95 (split, exclude
tls_intercept/ hunks + preserve audit.rs CR-02 expression); C → Phase 95 (split, preserve
Phase 89 activation predicate divergence); D → won't-sync (leapfrog floor >= 0.65.0 for Phase 97).

---

## Reproduction

```bash
# D-02 clean-continuation proof: nolabs-ai/nono v0.64.0 (0153757001) is byte-identical to the
# fork's UPST9 endpoint (Phase 85 upstream_head_at_audit: 0153757001d21805a8218213e32add462d3322a1).
# nolabs-ai/nono is a direct relocation of always-further/nono, NOT a re-fork. Window is
# directly comparable; no history-translation step is needed.

# Fetch the nolabs window non-destructively (do NOT rename/repoint remote -- that is plan 94-02)
git fetch https://github.com/nolabs-ai/nono.git --tags

# SHA guard: all four window tip SHAs must be reachable objects
git cat-file -t 0153757001  # expect: commit (v0.64.0 = Phase 85 UPST9 endpoint)
git cat-file -t 0551eba27e  # expect: commit (v0.64.1)
git cat-file -t 137bb15c56  # expect: commit (v0.65.0)
git cat-file -t 1d1c88c9f9  # expect: commit (v0.65.1 = window tip)
# All four returned: commit (verified 2026-06-26)

# Drift tool SHA pin (D-11)
git log -1 --format=%H -- scripts/check-upstream-drift.sh
# → 0834aa664fbaf4c5e41af5debece292992211559 (verified 2026-06-26)

# Run drift tool against EXPLICIT window SHAs (D-03 SHA-not-tag guard)
mkdir -p ci-logs-local/drift
bash scripts/check-upstream-drift.sh --from 0153757001 --to 1d1c88c9 --format json \
  > ci-logs-local/drift/20260626T003610Z-v064-v0651-upst10.json
# Output total_unique_commits: 8 (verified 2026-06-26)

# Full commit accounting (D-13)
git log --oneline 0153757001..1d1c88c9 | wc -l           # → 17 (total)
git log --merges --oneline 0153757001..1d1c88c9 | wc -l  # → 0 (merges)
git log --no-merges --oneline 0153757001..1d1c88c9 | wc -l  # → 17 (no-merges)

# tls_intercept/ dir absence in fork (Cluster F ongoing carve-out)
ls crates/nono-proxy/src/  # confirms no tls_intercept/ directory in fork
# Returns: audit.rs config.rs connect.rs credential.rs diagnostic.rs error.rs external.rs
#          filter.rs lib.rs oauth2.rs reverse.rs route.rs server.rs token.rs
```

**Auditor rerun:** Run the drift tool command above verbatim. The 8-commit JSON output matches
the per-cluster inventories in this ledger. Any new output indicates an upstream push after
`2026-06-26`; re-run the full audit if the count changes.

---

## Cluster Summary

| cluster_id | theme | commits | disposition | windows-touch | rationale | phase-95-status |
|------------|-------|---------|-------------|---------------|-----------|-----------------|
| A | AF_UNIX mediation deadlock fix (security + supervisor IPC) | 1 | will-sync | no | Security fix: 4 bugs in AF_UNIX pathname mediation (deadlock, wrong jt offsets, rate-limiter starvation, dup2 bypass); Phase 95 | absorbed 2026-06-26 — git cherry-pick -x 9ce74e92 (ae77d198 + post-fix 61689ef8; DCO signed) |
| B | Tool sandbox + proxy/audit surface expansion | 1 | split | no | Massive new feature (tool-sandbox dir absent in fork); tls_intercept/ hunks won't-apply; audit.rs touches CR-02 carve-out; sandbox/linux.rs cfg(linux)-gated; proxy surface touches Cluster F carve-out; Phase 95 | absorbed 2026-06-26 — shared-surface extraction (11fd10e0 hunks; tool-sandbox/tls_intercept/ skipped per D-01; DCO signed; commit 91d526e6) |
| C | Custom credentials explicit-activation refactor | 1 | split | no | Upstream reverses the Phase 89 fail-secure divergence (0c08e5d2); fork must preserve its expression; Phase 95 | absorbed 2026-06-26 — credentials_intent fix / structural no-op per D-02; Phase 89 active predicate preserved; DCO signed; commit 62dbf013 |
| D | Release metadata + dep bump | 5 | won't-sync | no | Version management (v0.64.1, v0.65.0, v0.65.1 releases + sigstore-verify dep bump); fork uses own leapfrog convention; leapfrog floor >= 0.65.0 for Phase 97 | won't-sync → Phase 97 (unchanged) |

---

## Cluster A: AF_UNIX Mediation Deadlock Fix

**Commits:** 1 — 9ce74e92

**Disposition:** will-sync

**Windows-touch:** no (commit is Linux cfg-gated)

**Inspection depth:** actual-diff (git show) — security fix on sandbox/linux.rs + supervisor IPC

**Rationale:** `9ce74e92` fixes four bugs that prevented AF_UNIX pathname mediation from working
correctly in the Phase 87 SEC-01 code path (`crates/nono/src/sandbox/linux.rs`):
(1) sendmsg deadlock — filter trapped child's SCM_RIGHTS handshake for notify fd;
(2) wrong jt offsets — connect/bind/sendto/sendmmsg fell through to ALLOW instead of USER_NOTIF;
(3) rate-limiter starvation — TCP/UDP calls drained the burst bucket before AF_UNIX decisions;
(4) dup2 bypass — IPC fd exemption allowed sandboxed process to dup2 an arbitrary socket.
Fix: IPC handshake completed BEFORE installing the BPF filter; both filters are pure allowlists
with no fd-based holes. Also touches `crates/nono-cli/src/exec_strategy.rs` (supervisor loop
refactor to close the race window) and `crates/nono/src/supervisor/socket.rs` (new IPC socket
helper). This is a direct continuation of Phase 87 SEC-01 work — security critical, must absorb.

**Phase 95 executor cross-target clippy note:** `9ce74e92` touches `crates/nono/src/sandbox/linux.rs`
and `crates/nono-cli/src/exec_strategy.rs` which contain `#[cfg(target_os = "linux")]` and
`#[cfg(target_os = "macos")]` blocks. Per CLAUDE.md MUST/NEVER rule, the Phase 95 executor
MUST run `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
AND `--target x86_64-apple-darwin`. Windows-host `cargo check` is NOT an accepted substitute.
If cross-toolchain unavailable, mark affected REQs PARTIAL and defer to live CI per
`.planning/templates/cross-target-verify-checklist.md`.

**Cross-cluster re-export check (actual-diff):**

```bash
git show 9ce74e92 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings: No new `pub use`/`pub mod`/`extern crate` additions. All changes are internal function
bodies and struct definitions (`#[cfg(target_os = "linux")]` gated). Clean.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 9ce74e92 | fix(sandbox): exempt IPC fd from sendmsg trapping to resolve af_unix_mediation deadlock (#1210) | v0.65.0 | other | 6 files (exec_strategy.rs, supervisor_linux.rs, socket_access_run.rs, sandbox/linux.rs, supervisor/socket.rs, profile-authoring-guide.md), 532+/66- | no |

---

## Cluster B: Tool Sandbox Feature + Proxy/Audit Surface Expansion

**Commits:** 1 — 11fd10e0

**Disposition:** split (tool-sandbox dir absent in fork; tls_intercept/ hunks won't-apply;
shared audit.rs + proxy route/server surfaces need careful extraction with CR-02 and Cluster F
preserve-fork-expression guidance)

**Windows-touch:** no

**Inspection depth:** actual-diff (git show) — large multi-surface commit (50+ files); required
per D-09 uniform actual-diff; re-export scan covers all touched surfaces

**Rationale:** `11fd10e0` is the largest commit in the window: it introduces the `tool-sandbox`
subsystem — a per-tool command mediation layer that intercepts individual tool invocations via a
local approval registry. The `crates/nono-cli/src/tool-sandbox/` directory does NOT exist in the
fork (confirmed: `ls crates/nono-cli/src/tool-sandbox/` → absent). The commit also:
- Adds `SandboxRuntimeAuditEvent` and `CommandPolicyAuditEvent` to `crates/nono/src/audit.rs` —
  this is an ADDITIVE extension to the audit module; the CR-02 carve-out field (`records_verified:
  event_count > 0`) is NOT touched by this commit (confirmed via diff inspection: `records_verified`
  does not appear in the 11fd10e0 diff for audit.rs). The additive audit.rs changes ARE syncable.
- Modifies `crates/nono-proxy/src/route.rs`, `server.rs`, `credential.rs`, `config.rs`,
  `reverse.rs` — proxy surface overlapping the Cluster F carve-out; needs split inspection.
- Modifies `crates/nono-proxy/src/tls_intercept/ca.rs`, `cert_cache.rs`, `handle.rs` —
  tls_intercept/ files are MODIFIED (not newly added) in upstream; fork lacks this dir entirely
  → all tls_intercept/ hunks are won't-apply.
- Modifies `crates/nono/src/sandbox/linux.rs` (additional tool-sandbox-related changes) and
  `crates/nono/src/sandbox/mod.rs`.
- Contains `#[cfg(target_os = "linux")]` and `#[cfg(target_os = "macos")]` blocks.

**Phase 95 executor cross-target clippy note:** `11fd10e0` touches `crates/nono/src/sandbox/linux.rs`,
`crates/nono/src/sandbox/mod.rs`, and `crates/nono-cli/src/tool-sandbox/platform/linux.rs` (and macos.rs).
Per CLAUDE.md MUST/NEVER rule, the Phase 95 executor MUST run cross-target clippy for
`x86_64-unknown-linux-gnu` AND `x86_64-apple-darwin`. Windows-host `cargo check` is NOT an
accepted substitute.

**Cross-cluster re-export check (actual-diff):**

```bash
git show 11fd10e0 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings (summarized — full output in drift JSON):
- `crates/nono-cli/src/`: Numerous `pub(crate)` struct/fn definitions for tool-sandbox machinery
  (`CommandPolicyAuditRecord`, `CommandPolicyFinding`, `ResolvedCommandBinaries`, etc.) — all
  intra-cluster, no cross-cluster dep detected.
- `crates/nono/src/audit.rs`: No new `pub use`/`pub mod` additions; adds `SandboxRuntimeAuditEvent`
  and `CommandPolicyAuditEvent` structs (additive struct additions, not re-exports).
- `crates/nono/src/lib.rs`: No new `pub mod`/`pub use` additions to core lib.rs.
- `crates/nono-proxy/src/lib.rs`: No new `pub use`/`pub mod` additions.
- `pub use nono::audit::{...}` — `pub(crate)` use statements in CLI layer (intra-cluster);
  specifically `pub(crate) use nono::audit::SandboxRuntimeAuditEvent` (references the new struct
  added to audit.rs in the same commit — ordering constraint: audit.rs additions must land before
  CLI consumers).

**Finding:** No cross-cluster pub-use deps detected. Intra-B ordering: audit.rs struct additions
must be applied before CLI consumers that reference `SandboxRuntimeAuditEvent` and
`CommandPolicyAuditEvent`. tls_intercept/ re-exports in upstream (`tls_intercept/mod.rs`) are
won't-apply (fork lacks tls_intercept/ dir).

**Phase 95 split guidance:**
- APPLY: All `crates/nono/src/audit.rs` additive changes (new event types — additive, no CR-02 conflict).
- APPLY: Non-tls_intercept proxy hunks in `route.rs`, `server.rs` (after Cluster F carve-out
  review per Phase 89 Cluster F Reconciliation Addendum in Phase 85 ledger).
- APPLY: `crates/nono/src/sandbox/linux.rs` and `mod.rs` hunks (after cross-target clippy gate).
- APPLY: `crates/nono/src/keystore.rs` and `crates/nono/src/scrub.rs` changes (additive).
- SKIP: All `crates/nono-proxy/src/tls_intercept/` hunks (fork lacks this dir — Cluster F ongoing).
- SKIP: `crates/nono-cli/src/tool-sandbox/` directory (new feature; absent in fork; significant
  new subsystem requiring its own phase scope; may require fork-specific policy integration).
  This skip should be documented in Phase 95 with an explicit stub tracking note.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 11fd10e0 | feat(sandbox): tool sandbox (#1105) | v0.65.0 | audit, other, package, policy, profile, proxy | 50+ files (tool-sandbox/ dir, audit.rs, sandbox/linux.rs, proxy surface, tls_intercept/), ~3000+/- | no |

---

## Cluster C: Custom Credentials Explicit-Activation Refactor

**Commits:** 1 — 9b37dc52

**Disposition:** split (upstream reverses the Phase 89 fail-secure divergence; fork expression
must be preserved — this is a direct conflict with fork commit `0c08e5d2`)

**Windows-touch:** no

**Inspection depth:** actual-diff (git show) — proxy_runtime.rs change directly collides with fork's
Phase 89 deliberate divergence

**Rationale:** `9b37dc52` refactors custom credentials to require explicit activation in the proxy
(`prepare_proxy_launch_options` in `crates/nono-cli/src/proxy_runtime.rs`). Upstream's stated
rationale: merely defining custom credentials should not auto-activate the proxy (fixing issue #1214).

**Critical fork conflict:** The fork's Phase 89 deliberate divergence (`0c08e5d2`, guard test
`proxy_activates_with_custom_credentials_only`) adds `!prepared.custom_credentials.is_empty()` to
the ACTIVE branch of `prepare_proxy_launch_options` — precisely so a `customCredentials`-only
config DOES activate the proxy (fail-secure: credentials injected via proxy, not left exposed).

`9b37dc52` removes the `has_custom_credentials` variable entirely and sets `has_credentials =
!credentials.is_empty()` (no custom_credentials disjunct), and renames the test from
`test_proxy_is_active_when_only_custom_credentials_are_set` to
`test_proxy_is_inactive_when_only_custom_credentials_are_set` with inverted semantics.

This directly inverts the fork's fail-secure guarantee. The fork's Phase 89 Cluster F
Reconciliation Addendum (in Phase 85 ledger) is the authoritative reference:
"activation predicate includes `!prepared.custom_credentials.is_empty()` in BOTH the ACTIVE
branch and the WARN branch" — that is the fork expression.

However, the upstream fix addresses a different aspect: upstream only wants credentials to activate
when explicitly listed in the `credentials` section (not merely defined). Phase 95 must reconcile
by splitting: keep the proxy activation for `custom_credentials` that ARE actively providing
credentials (the `CredentialProxyIntent` path), but remove the route-creation side effect. The
`credentials_intent` fix in `9b37dc52` (`if has_credentials || !prepared.custom_credentials.is_empty()`
for the `credentials_intent` block) is COMPATIBLE with the fork and should be applied.

**Cross-cluster re-export check (actual-diff):**

```bash
git show 9b37dc52 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings: No new `pub use`/`pub mod`/`extern crate` additions. Clean.

**Phase 95 split guidance:**
- APPLY: The `credentials_intent` block change (`if has_credentials || !prepared.custom_credentials.is_empty()`
  ensures `CredentialProxyIntent` is always constructed when custom credentials are defined) —
  this part is compatible with the fork.
- PRESERVE (do NOT apply): The removal of `has_custom_credentials` from the `would_activate` /
  ACTIVE branch. The fork's `proxy_activates_with_custom_credentials_only` guard test must be
  kept. Do not rename or invert it.
- SKIP: The test rename from `test_proxy_is_active_when_only_custom_credentials_are_set` to
  `test_proxy_is_inactive_when_only_custom_credentials_are_set` — directly contradicts
  the fork invariant.
- Guard test: `proxy_activates_with_custom_credentials_only` in `crates/nono-cli/src/proxy_runtime.rs`
  (Phase 89 `73bd03a6`) guards against reversion.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 9b37dc52 | refactor(credentials): require explicit activation for custom credentials (#1215) | v0.64.1 | other | 3 files (launch_runtime.rs, network_policy.rs, proxy_runtime.rs), +35/-15 | no |

---

## Cluster D: Release Metadata and Dep Bump

**Commits:** 5 — cd17570f, 9e084cbb, ae6c431a, 137bb15c, 1d1c88c9

**Disposition:** won't-sync (version management; fork uses own crate leapfrog convention;
leapfrog floor >= 0.65.0 for Phase 97)

**Windows-touch:** no

**Inspection depth:** actual-diff (git show per commit)

**Rationale:** Three release commits bump CHANGELOG, Cargo.toml versions across all crates
(nono, nono-cli, nono-proxy, nono-ffi), and `Cargo.lock`. One dependency commit bumps
`sigstore-verify` from 0.8.0 to 0.9.0 (touching `Cargo.lock` and `crates/nono/Cargo.toml`).
One commit (`137bb15c`) is a one-line fix (`crates/nono-cli/src/tool-sandbox/platform/linux.rs`)
landed between the v0.65.0 and v0.65.1 release tags — it patches the tool-sandbox Linux platform
file (which is absent in the fork → won't-apply as standalone; Phase 95 must absorb this fix as
part of the Cluster B tool-sandbox split scope if tool-sandbox is ever introduced). The version
metadata commits establish the Phase 97 leapfrog floor.

**D-10 leapfrog-floor cross-ref for Phase 97:** Commits `cd17570f` (v0.64.1), `ae6c431a`
(v0.65.0), and `1d1c88c9` (v0.65.1) are the upstream version-bump commits in this window. Phase 97
must set the fork's crate version to >= 0.65.0 (the leapfrog convention) when publishing.
The sigstore-verify bump (`9e084cbb`, 0.8.0→0.9.0) should be evaluated in Phase 95 DEPS review
as a direct dependency update.

**Cross-cluster re-export check:** All five commits are release/dep-bump commits; no `pub use`/
`pub mod` additions. Clean.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| cd17570f | chore: release v0.64.1 (#1217) | v0.64.1 | other | 6 files (CHANGELOG, Cargo.lock, all Cargo.toml), 16+/10- | no |
| 9e084cbb | chore(deps): bump sigstore-verify from 0.8.0 to 0.9.0 (#1228) | v0.65.0 | other | 2 files (Cargo.lock, crates/nono/Cargo.toml), 179+/51- | no |
| ae6c431a | chore: release v0.65.0 (#1237) | v0.65.0 | other | 6 files (CHANGELOG, Cargo.lock, all Cargo.toml), 43+/10- | no |
| 137bb15c | fix(sandbox): use syscall() for execveat to avoid glibc 2.34 linker dependency (#1239) | v0.65.1 | other | 1 file (tool-sandbox/platform/linux.rs), 8+/2- | no |
| 1d1c88c9 | chore: release v0.65.1 (#1240) | v0.65.1 | other | 6 files (CHANGELOG, Cargo.lock, all Cargo.toml), 11+/10- | no |

---

## Carve-out Re-touch Check

This section records the mandatory D-04/D-05 check for the three deliberate fork-divergence
carve-out points established in the Phase 85 ledger addenda.

### CR-02: `crates/nono/src/audit.rs` (records_verified: event_count > 0)

```bash
git log 0153757001..1d1c88c9 -- crates/nono/src/audit.rs
# → 11fd10e0 feat(sandbox): tool sandbox (#1105)
```

**Result: HIT — expected conflict — preserve fork expression.**

Commit `11fd10e0` adds `SandboxRuntimeAuditEvent` and `CommandPolicyAuditEvent` structs to
`crates/nono/src/audit.rs`. Inspection confirms these are **additive** struct additions; they do
NOT modify the `records_verified: event_count > 0` line (CR-02 carve-out, ~line 1435 in fork).
The `verify_audit_log` return block is untouched by this commit.

**Phase 95 guidance:** Apply the additive `SandboxRuntimeAuditEvent`/`CommandPolicyAuditEvent`
struct additions. The CR-02 fork expression (`records_verified: event_count > 0`) does not
conflict with these additions and must be preserved. Guard test:
`verify_empty_log_with_no_stored_metadata_is_not_valid` in `crates/nono/src/audit.rs`
(Phase 87, `proj/ADR-87-cr02-audit-bypass.md`) guards against unintentional reversion.

### CR-01: `bindings/c/src/` FFI entry points (clear_last_call_state() at entry)

```bash
git log 0153757001..1d1c88c9 -- \
  bindings/c/src/diagnostic.rs bindings/c/src/lib.rs \
  bindings/c/src/capability_set.rs bindings/c/src/fs_capability.rs \
  bindings/c/src/sandbox.rs bindings/c/src/state.rs bindings/c/src/query.rs
# → (no output)
```

**Result: clean — no re-touch in window.**

No commit in the `0153757001..1d1c88c9` window touches any of the CR-01 carve-out FFI files.
The fork's `clear_last_call_state()` at-entry invariant is not threatened by this window.
Guard test: `diagnostic_code_is_cleared_between_calls` in `bindings/c/src/diagnostic.rs`
(Phase 88, commit `db0f221d`) remains in force.

### Cluster F: Proxy fork-preserve surface

```bash
git log 0153757001..1d1c88c9 -- \
  crates/nono-proxy/src/route.rs \
  crates/nono-proxy/src/connect.rs \
  crates/nono-proxy/src/reverse.rs \
  crates/nono-proxy/src/server.rs \
  crates/nono-cli/src/proxy_runtime.rs
# → 11fd10e0 feat(sandbox): tool sandbox (#1105)
# → 9b37dc52 refactor(credentials): require explicit activation for custom credentials (#1215)
```

**Result: HIT (two commits) — expected conflict — preserve fork expression.**

`11fd10e0` modifies `route.rs`, `server.rs`, `reverse.rs`, `credential.rs`, `config.rs` (shared
proxy surface) AND `crates/nono-proxy/src/tls_intercept/ca.rs`, `cert_cache.rs`, `handle.rs`
(tls_intercept/ — won't-apply; fork lacks this dir). The shared proxy surface changes need
split inspection per the Phase 89 Cluster F Reconciliation Addendum. Guard tests from Phase 89:
`proxy_activates_with_custom_credentials_only`, `block_net_overrides_custom_credentials_activation`,
`build_proxy_config_maps_upstream_proxy_to_external_proxy` (proxy_runtime.rs),
`connect_keeps_open_on_missing_proxy_auth` (connect.rs),
`denied_endpoint_returns_403_and_audit` (reverse.rs),
`allow_domain_endpoint_route_does_not_shadow_credential_route` (route.rs).

`9b37dc52` modifies `proxy_runtime.rs` in a way that directly inverts the fork's Phase 89
deliberate divergence (`0c08e5d2`). Fork must preserve its `proxy_activates_with_custom_credentials_only`
activation predicate. See Cluster C for full split guidance.

**Absent `tls_intercept/` dir confirmation (2026-06-26):**
```bash
ls crates/nono-proxy/src/
# → audit.rs  config.rs  connect.rs  credential.rs  diagnostic.rs  error.rs  external.rs
#   filter.rs  lib.rs  oauth2.rs  reverse.rs  route.rs  server.rs  token.rs
# No tls_intercept/ directory.
```

The `EffectiveProxySettings` model and the fork's exact-prefix `RouteStore` + `_ep_{domain}` key
namespace (Phase 89 Reconciliation Addendum) are the deliberate divergence. Do NOT import
`RouteSelection` or `TlsInterceptIntent`.

---

## ADR Review

Overall phase ADR assessment across five standard dimensions:

| dimension | score | notes |
|-----------|-------|-------|
| security | H | Cluster A (9ce74e92 closes 4 bugs in AF_UNIX pathname mediation including a dup2 bypass — real security fix on Phase 87 SEC-01 code path); Cluster B tool-sandbox adds command policy mediation (security feature, but split disposition); Cluster C conflict is security-adjacent (fail-secure proxy activation); net: H driven by A |
| windows | L | No commit in the window touches Windows-specific files; all cfg-gated changes are Linux/macOS; no exec_strategy_windows/ changes; net: L |
| maintenance | H | Cluster B is a very large squashed commit (50+ files, tool-sandbox subsystem absent in fork); extracting shared-surface hunks while preserving Cluster F and CR-02 invariants requires careful split work; Cluster C has a direct behavioral inversion requiring Phase 95 human review; net: H |
| divergence | M | Cluster C adds a new active conflict with Phase 89 deliberate divergence (direct test-name inversion); Cluster B's tls_intercept/ hunks extend the existing Cluster F fork surface; CR-02 additive-only hit (no direct conflict); net: M |
| contributor | M | Cluster A is straightforward will-sync; Cluster B split requires Phase 95 careful extraction; Cluster C requires explicit rationale documentation; D is mechanical won't-sync; net: M (lower than Phase 85 because no boundary-convergence ADR-level decisions needed in this window) |

**Per-cluster risk matrix:**

| Cluster | security | windows | maintenance | divergence | contributor | Overall |
|---------|----------|---------|-------------|------------|-------------|---------|
| A | H | L | L | L | L | H (security dominant; will-sync) |
| B | M | L | H | M | M | H (large squash with multiple split surfaces including tls_intercept/ and CR-02 hit) |
| C | M | L | M | H | M | H (direct behavioral inversion of Phase 89 deliberate divergence; split required) |
| D | L | L | L | L | L | L (mechanical won't-sync; release metadata and dep bump) |

**Outcome:** Confirm drain-then-sync shape. No boundary-convergence ADR required (this window
has no library-boundary changes — no audit/diagnostics relocation, no FFI surface expansion).
Phase 95 is the primary action vehicle: Cluster A (straightforward will-sync), Cluster B (split
extraction with CR-02/Cluster F guidance), Cluster C (split with Phase 89 preserve-expression
requirement). Phase 97 picks up the >= 0.65.0 leapfrog floor from Cluster D. Escalation
threshold not reached; continue.

**Downstream routing:**
- Cluster A → Phase 95 ABSORBED (9ce74e92 cherry-picked as ae77d198 + post-fix 61689ef8; DCO signed; cross-target clippy PARTIAL→96)
- Cluster B → Phase 95 ABSORBED (shared-surface extraction from 11fd10e0; tool-sandbox/tls_intercept/ skipped per D-01; commit 91d526e6; DCO signed; cross-target clippy PARTIAL→96)
- Cluster C → Phase 95 ABSORBED (D-02 split: structural no-op confirmed; Phase 89 fail-secure active predicate preserved; commit 62dbf013; DCO signed)
- Cluster D → Phase 97 (won't-sync; leapfrog floor ref; sigstore-verify dep evaluation in Phase 95 DEPS)

---

## Empirical Cross-Check

Spot-check of 8 files via `git log --oneline <window> -- <file>` to validate cluster
classification accuracy (D-11). Includes all three carve-out path families (mandatory per D-11).

**Check 1: `crates/nono/src/audit.rs` → expect Cluster B (CR-02 carve-out path)**
```bash
git log --oneline 0153757001..1d1c88c9 -- crates/nono/src/audit.rs
# → 11fd10e0 feat(sandbox): tool sandbox (#1105)
```
Cluster B. **CORRECT** — CR-02 hit recorded in Carve-out Re-touch Check.

**Check 2: `bindings/c/src/lib.rs` → expect NO hits (CR-01 carve-out path)**
```bash
git log --oneline 0153757001..1d1c88c9 -- bindings/c/src/lib.rs
# → (no output)
```
Zero hits. **CORRECT** — CR-01 carve-out clean for this window.

**Check 3: `crates/nono-proxy/src/route.rs` → expect Cluster B (Cluster F carve-out path)**
```bash
git log --oneline 0153757001..1d1c88c9 -- crates/nono-proxy/src/route.rs
# → 11fd10e0 feat(sandbox): tool sandbox (#1105)
```
Cluster B. **CORRECT** — Cluster F hit recorded in Carve-out Re-touch Check.

**Check 4: `crates/nono-cli/src/proxy_runtime.rs` → expect Clusters B and C (Cluster F carve-out path)**
```bash
git log --oneline 0153757001..1d1c88c9 -- crates/nono-cli/src/proxy_runtime.rs
# → 11fd10e0 feat(sandbox): tool sandbox (#1105)
# → 9b37dc52 refactor(credentials): require explicit activation for custom credentials (#1215)
```
Both Cluster B and Cluster C. **CORRECT** — both recorded in Carve-out Re-touch Check.

**Check 5: `crates/nono/src/sandbox/linux.rs` → expect Clusters A and B**
```bash
git log --oneline 0153757001..1d1c88c9 -- crates/nono/src/sandbox/linux.rs
# → 9ce74e92 fix(sandbox): exempt IPC fd from sendmsg trapping to resolve af_unix_mediation deadlock (#1210)
# → 11fd10e0 feat(sandbox): tool sandbox (#1105)
```
Cluster A (9ce74e92) and Cluster B (11fd10e0). **CORRECT** — both are in their respective clusters.

**Check 6: `crates/nono-cli/src/exec_strategy.rs` → expect Clusters A and B**
```bash
git log --oneline 0153757001..1d1c88c9 -- crates/nono-cli/src/exec_strategy.rs
# → 9ce74e92 fix(sandbox): exempt IPC fd from sendmsg trapping to resolve af_unix_mediation deadlock (#1210)
# → 11fd10e0 feat(sandbox): tool sandbox (#1105)
```
Cluster A and Cluster B. **CORRECT.**

**Check 7: `crates/nono/Cargo.toml` → expect Cluster D (release/dep metadata)**
```bash
git log --oneline 0153757001..1d1c88c9 -- crates/nono/Cargo.toml
# → 1d1c88c9 chore: release v0.65.1 (#1240)
# → 9e084cbb chore(deps): bump sigstore-verify from 0.8.0 to 0.9.0 (#1228)
# → ae6c431a chore: release v0.65.0 (#1237)
# → cd17570f chore: release v0.64.1 (#1217)
```
All Cluster D. **CORRECT.**

**Check 8: `crates/nono/src/sandbox/mod.rs` → expect Cluster B**
```bash
git log --oneline 0153757001..1d1c88c9 -- crates/nono/src/sandbox/mod.rs
# → 11fd10e0 feat(sandbox): tool sandbox (#1105)
```
Cluster B. **CORRECT.**

All 8 spot-check results match expected clusters. No classification errors detected. Drift tool
re-run with explicit SHAs (`0153757001..1d1c88c9`) confirmed 8 substantive commits matching
the per-cluster inventories. Spot-check covers all three carve-out path families per D-11.

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
- Total commits in window: 17
- Merge commits: 0
- Non-merge commits: 17
- Substantive (drift-tool filter): 8
- Noise total: **9** = 0 merge commits + 9 non-merge out-of-filter commits

**0 merge commits:** None in this window. No merge commits to enumerate.

**9 non-merge out-of-filter commits:**
- `ed43d08f` — fix(deps): bump quinn-proto (#1238) — Cargo.lock and non-core Cargo.toml only
- `5bd0f90a` — chore(deps): bump syn from 2.0.117 to 2.0.118 (#1230) — Cargo.lock only
- `530a1ba8` — chore(deps): bump regex from 1.12.3 to 1.12.4 (#1231) — Cargo.lock only
- `8397ad44` — chore(deps): bump softprops/action-gh-release from 3.0.0 to 3.0.1 (#1227) — CI workflow yaml only
- `355d2737` — chore(deps): bump actions/checkout from 6.0.3 to 7.0.0 (#1226) — CI workflow yaml only
- `ded24756` — Update README.md (#1223) — docs only
- `1c8df5b9` — fix(docs): replace broken link in readme (#1221) — docs only
- `7a7fccd0` — Update README with curl installation command (#1219) — docs only
- `02100c3b` — chore(docs): improve profile documentation (#1212) — docs only

**Dep-bump note:** `ed43d08f` (quinn-proto), `5bd0f90a` (syn), `530a1ba8` (regex) touch only
`Cargo.lock` and non-core manifests. These dep version targets should be absorbed as part of
Phase 95 DEPS review, not cherry-picked as individual commits.

**Completeness verification:** 8 substantive + 9 noise = 17 total. Matches
`git log --oneline 0153757001..1d1c88c9 | wc -l` = 17. Every commit in the window is accounted
for in exactly one of: cluster per-commit tables (8) or this noise section (9).
