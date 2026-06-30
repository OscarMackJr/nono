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
distributed across 8 ledger clusters: A through H. Dispositions confirmed by uniform
actual-diff (`git show`) per D-09:

- `needs-decision` **1** cluster — A (2 commits: #1225 NetworkIntent + #1263 contradictory-flag
  guard; disposition settles in ADR-98, Plan 03)
- `won't-sync` **2** clusters — B (4 tool-sandbox commits; fork lacks tool-sandbox/) + H (1
  release commit; leapfrog floor **0.66.1** for Phase 100 — D-15)
- `split` **1** cluster — C (3 proxy evolution commits; tls_intercept/ absent; shared proxy
  surface extractable)
- `will-sync` **4** clusters — D (1 commit, 9P warning); E (1 commit, org-ref migration;
  confirmed from "verify" after D-03 re-confirmation); F (1 commit, sigstore dep bump);
  G (1 commit, proxy docs/token fix)

**Windows-touch clusters:** **A** only — `72bcfd66` (#1225) touches 7 files with
`cfg(target_os = "windows")` blocks (`capability_ext.rs`, `supervised_runtime.rs`,
`command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `output.rs`,
`terminal_approval.rs`). No other cluster touches Windows-specific code paths.

**Re-export scan (complete — actual-diff per D-09):** Cluster A adds `pub(crate) enum
NetworkIntent` + associated methods — all within `crates/nono-cli/src/`; no cross-library
additions to `crates/nono` core. Cluster C adds `pub mod pool` to `crates/nono-proxy/src/lib.rs`
(new connection-pooling module, intra-proxy, won't-apply if tls_intercept/ hunks skipped).
No cross-cluster `pub use` / `pub mod` violations detected.

**Fork-specific deliverables:** (1) **#1225 NetworkIntent disposition** — Cluster A; ADR-98
(`proj/ADR-98-network-intent-disposition.md`); Plan 03. (2) **Expanded carve-out re-touch
check** — D-07; six surfaces; this window: CR-02 CLEAN, CR-01 CLEAN, Cluster F HIT 5 commits,
linux.rs HIT 1 (additive), endpoint-policy HIT 3, v3.2 override CLEAN.

**#1225 note:** actual-diff confirms CLI-side ONLY (11 files all in `crates/nono-cli/src/`; no
changes to `crates/nono/src/capability.rs` or any core library file). The conflict is fork CLI
(`NetworkMode::ProxyOnly` + `CompiledEndpointPolicy`) vs upstream CLI (`NetworkIntent` enum) —
narrower than the 260629-toe "HIGH-CONFLICT" preliminary scan suggested, but still
decision-blocking for Phase 99 absorb.

**Downstream routing:**
- Cluster A → ADR-98 (Plan 03) PENDING; Phase 99 absorb after decision
- Cluster B → won't-sync; future phase if tool-sandbox/ adopted
- Cluster C → Phase 99 split extraction (skip tls_intercept/ hunks; apply pool.rs + shared
  proxy surface + endpoint wiring; Cluster F carve-out review)
- Cluster D → Phase 99 will-sync (apply with cross-target clippy gate; linux.rs cfg(linux))
- Cluster E → Phase 99 will-sync (per-file review; preserve OscarMackJr/nono identity URLs)
- Cluster F → Phase 99 will-sync (sigstore cascade compatibility check)
- Cluster G → Phase 99 will-sync (accuracy fixes; clean apply)
- Cluster H → Phase 100 won't-sync (leapfrog floor 0.66.1; cross-ref: upstream 0.66.0)

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
| A | NetworkIntent refactor + contradictory-flag guard (HIGH-CONFLICT) | 2 | needs-decision (ADR-98) | yes | #1225 CLI-owned NetworkIntent enum replaces ProxyOnly placeholder pattern; #1263 pairs with #1225 (block-net conflict validation); 7 of 11 touched files have cfg(windows) blocks; ADR-98 settles adopt vs fork-diverge carve-out | ADR-98 (Plan 03) → Phase 99 after decision |
| B | Tool-sandbox enhancements (won't-sync — fork lacks tool-sandbox/) | 4 | won't-sync | no | Fork lacks tool-sandbox/ dir (skipped in Phase 94/95 Cluster B); all 4 commits patch files absent from fork; carry-forward if tool-sandbox ever adopted | won't-sync → future phase |
| C | Proxy evolution: HTTP/2 + endpoint routing (split — tls_intercept/ absent) | 3 | split | no | #983 large multi-file commit includes tls_intercept/ h2_forward/h2_probe hunks (won't-apply); shared proxy surfaces (pool.rs, route.rs, server.rs, reverse.rs) extractable; Cluster F carve-out applies | Phase 99 split (skip tls_intercept/; apply pool.rs + shared surface + endpoint wiring; Cluster F carve-out review) |
| D | 9P filesystem capability warning | 1 | will-sync | no | Additive diagnostic in sandbox/linux.rs; Linux cfg-gated; additive-only, no conflict with Phase 95 fork invariants (linux.rs carve-out check HIT 1 — additive) | Phase 99 will-sync (cross-target clippy gate required; linux.rs cfg(linux)) |
| E | Org reference migration (always-further → nolabs-ai) | 1 | will-sync | no | D-03 re-confirmation: mechanical GitHub URL updates; fork already relocated upstream remote to nolabs-ai (Phase 94); per-file review preserves OscarMackJr/nono fork identity URLs | Phase 99 will-sync (per-file review) |
| F | Sigstore-trust-root dep bump (0.8.0 → 0.9.0) | 1 | will-sync | no | Exact-pin dep in crates/nono/Cargo.toml; evaluate sigstore-verify/sigstore-sign cascade compatibility in Phase 99 | Phase 99 will-sync (sigstore cascade check) |
| G | Proxy docs + deprecated flag metadata fix | 1 | will-sync | no | cli.rs, cli_bootstrap.rs, token.rs accuracy fixes; stale X-Nono-Token claim corrected; additive accuracy changes only | Phase 99 will-sync |
| H | Release metadata v0.66.0 | 1 | won't-sync | no | CHANGELOG + version bumps (all Cargo.toml files + bindings/c/Cargo.toml); fork leapfrogs to 0.66.1 (D-15); cross-ref: upstream 0.66.0 is the floor for Phase 100 | won't-sync → Phase 100 (leapfrog floor 0.66.1) |

---

## Cluster A: NetworkIntent Refactor + Contradictory-Flag Guard

**Commits:** 2 — 72bcfd66, d457ecc3

**Disposition:** needs-decision (ADR-98) — `proj/ADR-98-network-intent-disposition.md`
(Plan 03). The disposition for both commits is gated on the ADR outcome: full-sync-adopt
(both commits apply together) vs fork-diverge carve-out (d457ecc3's `validate_block_net_conflicts`
logic is independently valuable but references `NetworkIntent` — requires adaptation if fork-diverge).

**Windows-touch:** yes — `72bcfd66` (#1225) touches 7 files with `cfg(target_os = "windows")`
blocks: `capability_ext.rs`, `supervised_runtime.rs`, `command_runtime.rs`,
`execution_runtime.rs`, `launch_runtime.rs`, `output.rs`, `terminal_approval.rs`. `d457ecc3`
(#1263) touches `command_runtime.rs` and `launch_runtime.rs` which contain `cfg(windows)` blocks
in the file (though the #1263 diff does not modify those blocks).

**Inspection depth:** actual-diff (git show) — 11-file CLI refactor (#1225) + 3-file validation
addition (#1263); both confirmed CLI-side only; no core library changes

**Rationale:** `72bcfd66` (#1225) replaces the `ProxyOnly { port: 0 }` placeholder pattern and
`profile_network_block` side channel with a single CLI-owned `NetworkIntent` enum with three
variants: `Unrestricted`, `BlockAll`, and `ProxyFiltered(BoxedProxyLaunchOptions)`.
`prepare_proxy_launch_options` now returns `NetworkIntent` directly.
`ProxyLaunchOptions.network_block` renamed to `strict_filter`. All proxy intent structs gain
`Debug` derives. `print_capabilities` gains a `proxy_pending` parameter for display accuracy.

Actual-diff confirms: **CLI-side only** — all 11 touched files are in `crates/nono-cli/src/`.
No changes to `crates/nono/src/capability.rs`, `manifest_convert.rs`, `sandbox/linux.rs`, or
any core library file. The conflict is fork CLI (`NetworkMode::ProxyOnly` constructor pattern in
`capability_ext.rs` + `CompiledEndpointPolicy` endpoint-policy wiring) vs upstream CLI
(`NetworkIntent` enum) — narrower than the 260629-toe preliminary scan suggested.

`d457ecc3` (#1263) adds `validate_block_net_conflicts()` in `sandbox_prepare.rs`: called from
both `command_runtime` (dry-run path) and `launch_runtime` (main launch) before any sandbox or
proxy setup. Validates `--block-net` + credential/network-profile/allow-domain conflicts, and
`--proxy-port` without proxy-triggering flags. Also extracts `has_proxy_intent()` to fix a
display gap. This commit pairs with #1225 (uses `NetworkIntent` type in the validation logic).

**Phase 99 executor cross-target clippy note (D-14):** Both commits touch files containing
`cfg(target_os = "linux")` and `cfg(target_os = "macos")` blocks:
- `72bcfd66` (`supervised_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`,
  `command_runtime.rs`, `terminal_approval.rs`): contains `cfg(any(target_os = "linux",
  target_os = "macos"))` and `cfg(target_os = "macos")` and `cfg(target_os = "linux")` blocks
  in the diff.
- `d457ecc3` (`sandbox_prepare.rs`): adds new `#[cfg(target_os = "linux")]` blocks (+2) and
  references `#[cfg(target_os = "macos")]` (existing, in context).

If adopted, Phase 99 executor MUST run `cross clippy --workspace --target
x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `cargo-zigbuild clippy
--workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` per CLAUDE.md
MUST rule. Windows-host `cargo clippy` is NOT a substitute.

**Cross-cluster re-export check (actual-diff):**

```bash
git show 72bcfd66 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show d457ecc3 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings for `72bcfd66`: New `pub(crate)` items — `pub(crate) enum NetworkIntent`,
`pub(crate) fn is_proxy_active()`, `pub(crate) fn proxy_options()`, `pub(crate) strict_filter:
bool` field, `pub(crate) network: NetworkIntent` field, `pub(crate) profile_network_block: bool`
field, `pub(crate) proxy: Option<&'a ProxyLaunchOptions>` field. All are within
`crates/nono-cli/src/`; no additions to `crates/nono` core library or `crates/nono-proxy`.
**No cross-library re-export violation.**

Findings for `d457ecc3`: `pub(crate) fn validate_block_net_conflicts(` — within
`crates/nono-cli/src/sandbox_prepare.rs`. Intra-CLI only. **Clean.**

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 72bcfd66 | refactor(network): introduce NetworkIntent and remove ProxyOnly placeholders (#1225) | v0.66.0 | other | 11 files (capability_ext.rs, command_runtime.rs, execution_runtime.rs, launch_runtime.rs, main.rs, output.rs, profile/mod.rs, proxy_runtime.rs, sandbox_prepare.rs, supervised_runtime.rs, terminal_approval.rs), 184+/130- | yes |
| d457ecc3 | fix(network): error early on contradictory network flag combinations (#1263) | v0.66.0 | other | 3 files (command_runtime.rs, launch_runtime.rs, sandbox_prepare.rs), 254+/8- | no |

---

## Cluster B: Tool-Sandbox Enhancements

**Commits:** 4 — 691e0f4f, 7011bc85, d2252225, 853d5236

**Disposition:** won't-sync (fork lacks `crates/nono-cli/src/tool-sandbox/` directory —
skipped in Phase 94/95 Cluster B; all 4 commits patch files absent from fork; carry-forward
if tool-sandbox ever adopted as a phase)

**Windows-touch:** no (all commits touch `tool-sandbox/platform/linux.rs`,
`tool-sandbox/platform/macos.rs`, `tool-sandbox/env.rs`, `tool-sandbox/dynamic_providers.rs`,
`command_policy.rs`; none touch `*_windows.rs` or `exec_strategy_windows/`)

**Inspection depth:** actual-diff (git show per commit) — confirmed all files are within
tool-sandbox/ subtree absent in fork; command_policy.rs hunk reviewed for cross-cluster leakage

**Rationale:** All four commits are incremental improvements to the tool-sandbox subsystem that
upstream introduced in v0.65.0 (Phase 94 Cluster B, commit `11fd10e0`). The fork skipped the
entire `tool-sandbox/` directory. There is no basis for cherry-picking patch commits to a
subsystem that does not exist in the fork.

- `691e0f4f` (#1268): modifies `resolve_caller` on Linux/macOS to allow self-invocation policy
  inheritance; adds `has_explicit_self_invocation_entry()` to `command_policy.rs`. NOTE: the
  `command_policy.rs` change IS in the drift tool filter — however, it adds logic that references
  tool-sandbox types. If tool-sandbox is eventually adopted, this command_policy.rs change must
  be included.
- `7011bc85` (#1271): adds `@git:common-dir` dynamic token to `tool-sandbox/dynamic_providers.rs`.
  Adds `read_common_dir()` for worktree git common dir discovery. Out-of-filter file
  `CHANGELOG.md` excluded by noise criteria. The `dynamic_providers.rs` file is absent in fork.
- `d2252225` (#1253): adds `add_optional_dir` (warn-and-skip) pattern to `tool-sandbox/platform/
  linux.rs` and `macos.rs` for missing `fs_read`/`fs_write` dirs. Behavior change: missing
  configured dirs emit warnings instead of hard errors. This is the correct fail-open-for-config
  vs fail-closed-for-security distinction. When adopted: confirm this does NOT weaken fail-secure
  for _failed-to-resolve_ dirs (only _missing_ dirs are skipped, per commit message).
- `853d5236` (#1249): passes TLS trust bundle env vars (`SSL_CERT_FILE`, `SSL_CERT_DIR`,
  `CURL_CA_BUNDLE`, `NODE_EXTRA_CA_CERTS`) to tool-sandbox children. Adds `tool-sandbox/env.rs`
  (absent in fork) and extends `platform/macos.rs`. Windows env-handling caveat (from memory
  file `windows_hook_interpreter_spawn_gotchas`): fork's interpreter spawn does `env_clear()` +
  baseline re-add; trust-bundle vars would need explicit Windows env forwarding here.

**Cross-cluster re-export check (actual-diff):**

```bash
git show 691e0f4f | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show 7011bc85 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show d2252225 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show 853d5236 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings: `691e0f4f` adds `pub(crate) fn has_explicit_self_invocation_entry(` in
`command_policy.rs` (intra-CLI). `7011bc85` adds `pub(crate) fn read_common_dir()` in
`dynamic_providers.rs` (intra-tool-sandbox; absent in fork). All other items are intra-B.
No cross-cluster deps detected. **Clean from cross-cluster perspective.**

**Phase 99 guidance (adoption carry-forward):** If tool-sandbox is ever adopted in a future
phase, apply these 4 commits in sequence after the initial tool-sandbox/ scaffolding. Note the
`d2252225` skip-vs-fail semantics check and the `853d5236` Windows env-forwarding gap.

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 691e0f4f | feat(tool-sandbox): simplify self-invocation policy (#1268) | v0.66.0 | profile, other | 5 files (command_policy.rs, tool-sandbox/platform/linux.rs, macos.rs, data/schema.json, docs — data+docs out of filter), 169+/42- in filter | no |
| 7011bc85 | feat(tool-sandbox): add @git:common-dir dynamic token (#1271) | v0.66.0 | other | 1 in-filter file (tool-sandbox/dynamic_providers.rs, 116+), 3 total (CHANGELOG + docs out of filter) | no |
| d2252225 | fix(tool-sandbox): skip missing fs_read/fs_write dirs instead of erroring (#1253) | v0.66.0 | other | 2 in-filter files (tool-sandbox/platform/linux.rs +15/-2, macos.rs +15/-2), 3 total (CHANGELOG out of filter) | no |
| 853d5236 | fix(tool-sandbox): pass TLS trust bundle env vars to tool-sandbox children (#1249) | v0.66.0 | other | 2 in-filter files (tool-sandbox/env.rs +28, tool-sandbox/platform/macos.rs +40), 3 total (CHANGELOG out of filter) | no |

---

## Cluster C: Proxy Evolution — HTTP/2 + Endpoint Routing

**Commits:** 3 — cdeeb5b9, 46bcfbb9, 08ca19a8

**Disposition:** split (tls_intercept/ dir absent in fork; shared proxy surfaces extractable;
Cluster F carve-out review required for all three commits)

**Windows-touch:** no (no cfg(windows) blocks in diffs; proxy is "not implemented for Windows
supervised runs" note in fork)

**Inspection depth:** actual-diff (git show per commit) — cdeeb5b9 is a large 26-file commit
requiring hunk-by-hunk inspection; tls_intercept/ files confirmed won't-apply; shared surfaces
confirmed compatible; endpoint wiring reviewed for CompiledEndpointPolicy compatibility

**Rationale:**

`cdeeb5b9` (#983) adds H2 ALPN support to the reverse proxy (gRPC credential injection,
bidirectional streaming, 8 new tests, bounded buffers, connection pooling). The commit touches
26 files: the tls_intercept/ files (`tls_intercept/acceptor.rs`, `tls_intercept/h2_forward.rs`
[new, 2607 lines], `tls_intercept/h2_probe.rs` [new, 352 lines], `tls_intercept/handle.rs`,
`tls_intercept/mod.rs`) are **won't-apply** (fork lacks tls_intercept/ dir). The shared proxy
surfaces (`pool.rs` [new, 365 lines], `reverse.rs`, `route.rs`, `server.rs`, `credential.rs`,
`config.rs`) and CLI surfaces (`cli.rs`, `launch_runtime.rs`, `main.rs`, `profile/mod.rs`,
`proxy_runtime.rs`, `sandbox_prepare.rs`) ARE extractable. The new `pool.rs` module adds HTTP
connection pooling (keep-alive by default, optional `--allow-http2` multiplexing) — beneficial
independently of tls_intercept/. Note: `pub mod pool` added to `nono-proxy/src/lib.rs`
(cross-cluster visible public module addition).

`46bcfbb9` (#1127) wires `--allow-endpoint` through to credential routes.
`ProxyLaunchOptions` gains `endpoint_restrictions: Vec<(String, EndpointRule)>`. The
`proxy_runtime.rs` change (+225/-8 lines) parses `SERVICE:METHOD:PATH` at prepare stage and
applies to matching credential routes before proxy config is built. Compatibility concern:
fork's `CompiledEndpointPolicy` / `endpoint_policy.evaluate()` wiring (v3.3 Phase 95) lives in
`crates/nono-cli/src/network_policy.rs` and `crates/nono-proxy/src/{config,reverse,route,
server}.rs` — Phase 99 must verify `endpoint_restrictions` applies through the fork's own
`compile()` / `endpoint_rules` path without duplicating or bypassing the fork's
`CompiledEndpointPolicy` evaluation chain.

`08ca19a8` (#1243) adds wildcard matching for credential upstream routes in `route.rs`
(+102/-4 lines). The fix ensures wildcard patterns like `*.example.com` match credential
upstream routes correctly. This is a targeted, additive fix — review confirms it does not
touch the `EffectiveProxySettings` model or the fork's `RouteStore` + `_ep_{domain}` key
namespace (Phase 89 Reconciliation Addendum).

**Phase 99 split guidance:**
- APPLY: `crates/nono-proxy/src/pool.rs` (new module; connection pooling; additive)
- APPLY: Shared proxy surface hunks in `reverse.rs`, `route.rs`, `server.rs`, `credential.rs`,
  `config.rs` (post Cluster F carve-out review; guard tests must pass)
- APPLY: CLI surface changes in `crates/nono-cli/src/` (--allow-http2 flag, proxy launch opts)
- APPLY: `46bcfbb9` endpoint wiring (verify CompiledEndpointPolicy compatibility first)
- APPLY: `08ca19a8` wildcard route fix (additive; confirm _ep_ namespace preserved)
- SKIP: All `crates/nono-proxy/src/tls_intercept/` hunks (fork lacks this dir — Cluster F
  ongoing; do not create tls_intercept/ dir as a side effect of the split)

**Cross-cluster re-export check (actual-diff):**

```bash
git show cdeeb5b9 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show 46bcfbb9 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
git show 08ca19a8 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings for `cdeeb5b9`: `pub mod pool` added to `crates/nono-proxy/src/lib.rs` (new public
module — cross-crate visible to nono-proxy consumers; no direct library violation since proxy
is already a separate crate). Numerous `pub(crate)` items in `tls_intercept/` (won't-apply) and
`pool.rs`, `reverse.rs` (intra-proxy). No additions to `crates/nono` core library. The `pub mod
pool` in lib.rs is intentional — connection pooling is a first-class proxy feature. **Net:
intra-proxy, no cross-cluster library violation.**

Findings for `46bcfbb9`: `pub(crate) endpoint_restrictions: Vec<(String,
nono_proxy::config::EndpointRule)>` field added to `ProxyLaunchOptions`. Cross-cluster
implication: this field in the CLI struct (`crates/nono-cli/`) references a type from
`crates/nono-proxy/`. This is an existing cross-crate dependency pattern (nono-cli already
depends on nono-proxy). **No library boundary violation; intra-cluster dep structure.**

Findings for `08ca19a8`: No new `pub use`/`pub mod`/`pub(crate)` additions. **Clean.**

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| cdeeb5b9 | feat(proxy): add HTTP/2 support for reverse proxy and credential injection (#983) | v0.66.0 | proxy, other | 26 files (tls_intercept/* won't-apply; pool.rs new; reverse.rs, route.rs, server.rs, credential.rs, config.rs, lib.rs; CLI + Cargo files), 4534+/344- | no |
| 46bcfbb9 | fix(network): wire --allow-endpoint through to credential routes (#1127) | v0.66.0 | other | 2 files (launch_runtime.rs +3, proxy_runtime.rs +225/-8) | no |
| 08ca19a8 | fix(proxy): match wildcard credential upstream routes (#1243) | v0.66.0 | proxy | 1 file (route.rs, 102+/4-) | no |

---

## Cluster D: 9P Filesystem Capability Warning

**Commits:** 1 — 5b8e94da

**Disposition:** will-sync

**Windows-touch:** no (Linux-only change; `crates/nono/src/sandbox/linux.rs`)

**Inspection depth:** actual-diff (git show) — confirmed additive; no conflict with Phase 95
AF_UNIX/seccomp/cgroup fork invariants; carve-out check result: HIT 1 (additive)

**Rationale:** `5b8e94da` (#1207) adds a 9P filesystem detection warning to
`crates/nono/src/sandbox/linux.rs` (+128 lines, pure addition). Landlock grants for paths on
9P mounts (WSL2 `/mnt/c`, QEMU virtfs) are silently ineffective or incompletely enforced.
Detection uses `statfs(2)` checking `f_type` against `V9FS_MAGIC (0x01021997)`. Syscall is
skipped for paths not under `/mnt` (common case zero overhead). Warnings deduplicated per path.

Actual-diff confirms: additive only — new `is_9p_path()` check inserted before the Landlock
capability-add loop. Does NOT touch the AF_UNIX seccomp BPF filter path, the cgroup v2
resource-enforcement module, or the `restrict_self()` / `apply_landlock()` entry points
restored in Phase 95/96 (confirmed by carve-out check in Plan 01).

**Phase 99 executor cross-target clippy note (D-14):** `5b8e94da` modifies
`crates/nono/src/sandbox/linux.rs` which contains `#[cfg(target_os = "linux")]` blocks (lines
432, 481, 908, 1185, 1187 in current fork). Per CLAUDE.md MUST rule, Phase 99 executor MUST
run `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D
clippy::unwrap_used` after applying this commit. Windows-host `cargo clippy` is NOT a
substitute. apple-darwin cross-target SHOULD also be run (CLAUDE.md MUST for any cfg-gated
Unix file change).

**Cross-cluster re-export check (actual-diff):**

```bash
git show 5b8e94da | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings: No new `pub use`/`pub mod`/`extern crate` additions. The added function
`is_9p_path()` is not exported. **Clean.**

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 5b8e94da | fix(sandbox): warn when capability path is on a 9P filesystem (#1207) | v0.66.0 | other | 1 file (crates/nono/src/sandbox/linux.rs, 128+/0-) | no |

---

## Cluster E: Org Reference Migration (always-further → nolabs-ai)

**Commits:** 1 — c808f000

**Disposition:** will-sync (D-03 re-confirmation from "verify" scaffold — actual-diff confirms
mechanical GitHub URL string updates; no behavioral change; per-file review in Phase 99)

**Windows-touch:** no (no cfg(windows) blocks; no Windows-specific code paths)

**Inspection depth:** actual-diff (git show) — 31-file commit; drift-filter-relevant files:
`command_policy.rs`, `migration.rs`, `profile/mod.rs`, `proxy_runtime.rs`, `route.rs`,
`setup.rs`, `test_env.rs`, `update_check.rs`; confirmed mechanical string replacements only

**Rationale:** `c808f000` (#1235) updates all GitHub URLs, CI repository guards, container
image tags, downstream repo targets, and tooling references from `always-further/nono` to
`nolabs-ai/nono`. Commit author explicitly preserved registry package namespaces
(`always-further/claude`, `always-further/codex`, etc.) — these are on-disk install identifiers
that must not change.

Actual-diff of drift-filter-relevant files confirms:
- `update_check.rs`: test fixture URL in a test (`release_url` JSON field) — safe to apply
- `route.rs`: test case fixtures using `/always-further/nono.git/` paths → `/nolabs-ai/nono.git/`
  — safe to apply (test reference, not fork identity)
- `setup.rs` (+4/-4): GitHub URL references in setup instructions → nolabs-ai
- `proxy_runtime.rs` (+4/-2): proxy-related URL references → nolabs-ai
- `command_policy.rs` (+2/-1): policy reference strings → nolabs-ai
- `migration.rs` (+2/-1): migration guidance strings → nolabs-ai
- `profile/mod.rs` (+2/-1): profile loading references → nolabs-ai
- `test_env.rs` (+2/-1): test environment references → nolabs-ai

Fork's upstream remote was repointed to `nolabs-ai/nono` in Phase 94 — these source string
changes are consistent with that decision.

**Phase 99 per-file review instruction:** Confirm the fork's own release and update-check URLs
remain `OscarMackJr/nono` (not `nolabs-ai/nono`) in production code paths (not test fixtures).
The `update_check.rs` production `NONO_UPDATE_URL` / default URL should already point to
`OscarMackJr/nono` — verify this is not overwritten by the `c808f000` hunk. The diff only
changes a test fixture JSON value, not the production URL constant, so this should be safe.

**Cross-cluster re-export check (actual-diff):**

```bash
git show c808f000 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings: No new `pub use`/`pub mod`/`extern crate` additions. **Clean.**

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| c808f000 | chore: migrate GitHub org references from always-further to nolabs-ai (#1235) | v0.66.0 | proxy, other | 31 total files; 8 in drift-filter (command_policy.rs, migration.rs, profile/mod.rs, proxy_runtime.rs, route.rs, setup.rs, test_env.rs, update_check.rs), 69+/73- total | no |

---

## Cluster F: Sigstore-Trust-Root Dependency Bump

**Commits:** 1 — 2e64798d

**Disposition:** will-sync

**Windows-touch:** no (Cargo.toml + Cargo.lock only; no platform code change)

**Inspection depth:** actual-diff (git show) — confirmed simple dep version bump; 2 files;
cascade reviewed for sigstore-verify/sigstore-sign compatibility

**Rationale:** `2e64798d` (#1229) bumps `sigstore-trust-root` from 0.8.0 to 0.9.0 in
`crates/nono/Cargo.toml` (dependabot auto-generated). `Cargo.lock` changes are extensive
(269 insertions / 223 deletions) reflecting transitive dep resolution.

Fork uses `sigstore-verify` and `sigstore-sign` from the same `prefix-dev/sigstore-rust`
family (Phase 86/88 absorb). The `sigstore-trust-root` dep is a direct dependency of the core
nono library for attestation verification. Phase 99 DEPS review must confirm: (1) `sigstore-
trust-root 0.9.0` is API-compatible with the fork's current `sigstore-verify` + `sigstore-sign`
pins; (2) the `Cargo.lock` update does not introduce any RUSTSEC advisories (cf. 260624-p1c
quinn-proto precedent).

**Cross-cluster re-export check (actual-diff):**

```bash
git show 2e64798d | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings: No new `pub use`/`pub mod`/`extern crate` additions. **Clean.**

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 2e64798d | chore(deps): bump sigstore-trust-root from 0.8.0 to 0.9.0 (#1229) | v0.66.0 | package | 2 files (Cargo.lock 269+/223-, crates/nono/Cargo.toml 2+/1-) | no |

---

## Cluster G: Proxy Documentation + Token Metadata Fix

**Commits:** 1 — a4d68189

**Disposition:** will-sync

**Windows-touch:** no (no cfg(windows) blocks; no Windows-specific code paths)

**Inspection depth:** actual-diff (git show) — confirmed accuracy fixes; no behavioral change;
token.rs change reviewed for Cluster F carve-out interaction

**Rationale:** `a4d68189` (#1246) corrects stale documentation about how the reverse proxy
authenticates requests. The README incorrectly claimed that proxy requests use an
`X-Nono-Token` header and `NONO_PROXY_TOKEN` env var — neither exists. The actual mechanism
is transparent credential shadowing. The commit also fixes `cli.rs` + `cli_bootstrap.rs`
(deprecated flag metadata) and `token.rs` (metadata accuracy).

Actual-diff of `crates/nono-proxy/src/token.rs` (+11/-7): corrects internal token handling
metadata (not behavioral change). This file is in the Cluster F proxy carve-out surface but
the change is annotation/metadata only — does not touch `EffectiveProxySettings`, the
`_ep_{domain}` key namespace, or the activation predicate (Phase 89 guard tests unaffected).

`crates/nono-cli/src/cli_bootstrap.rs` (+62/-30): the large delta is formatting/restructuring
around deprecated flag display, not behavioral changes.

**Cross-cluster re-export check (actual-diff):**

```bash
git show a4d68189 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'
```

Findings: No new `pub use`/`pub mod`/`extern crate` additions. **Clean.**

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| a4d68189 | docs(proxy): fix stale X-Nono-Token authentication claim (#1246) | v0.66.0 | proxy, other | 4 files (cli.rs +2/-2, cli_bootstrap.rs +62/-30, nono-proxy/README.md +4/-2, token.rs +11/-7) | no |

---

## Cluster H: Release Metadata v0.66.0

**Commits:** 1 — d817ed53

**Disposition:** won't-sync (version management; fork uses own leapfrog convention)

**Windows-touch:** no (CHANGELOG + Cargo.toml files; no platform code)

**Inspection depth:** actual-diff (git show) — confirmed CHANGELOG + 5 Cargo.toml version
bumps; no source code changes

**Rationale:** `d817ed53` (#1293) is the release-cut commit: CHANGELOG entry (60 lines
documenting the v0.66.0 release) and version bumps `0.65.1 → 0.66.0` across all 5 workspace
crates (`nono`, `nono-cli`, `nono-proxy`, `bindings/c`, plus internal path-dep pins).
`Cargo.lock` reflects the version update.

The fork is already at crate version `0.66.0` (leapfrogged in v3.3 Phase 97 to be strictly
above upstream v0.65.1). Upstream has now shipped `0.66.0`, spending the fork's leapfrog.

**D-15 leapfrog-floor cross-ref for Phase 100:** Upstream's `0.66.0` is now the floor. The
fork MUST bump to `0.66.1` (collision-free above upstream `0.66.0`) in Phase 100. Do NOT
publish `0.66.0` from the fork. The `0.67.0` floor referenced in the 260629-toe SUMMARY is
superseded by D-15 (`0.66.1` is the correct minimum, not `0.67.0`).

**Cross-cluster re-export check:** Release commit; no `pub use`/`pub mod` additions. **Clean.**

**Per-commit table:**

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| d817ed53 | chore: release v0.66.0 (#1293) | v0.66.0 | other | 6 files (CHANGELOG.md 60+, Cargo.lock 6+/-6, bindings/c/Cargo.toml, nono-cli/Cargo.toml, nono-proxy/Cargo.toml, nono/Cargo.toml), 70+/10- | no |

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
`diagnostic_code_is_cleared_between_calls`, Phase 88, commit `db0f221d`) remains unthreatened.
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

Spot-check of 12 files via `git log --oneline <window> -- <file>` to validate cluster
classification accuracy (D-10). Plan 01 established checks 1-6 (carve-out path families,
mandatory per D-07). Plan 02 adds checks 7-12 (mandatory #1225-surface files per D-04 + key
phase-99 routing confirmation).

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

**Check 6: `crates/nono-cli/src/proxy_runtime.rs` → expect Clusters A, C, and E (Cluster F carve-out path)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono-cli/src/proxy_runtime.rs
# → 46bcfbb9 fix(network): wire --allow-endpoint through to credential routes (#1127)
# → cdeeb5b9 feat(proxy): add HTTP/2 support for reverse proxy and credential injection (#983)
# → c808f000 chore: migrate GitHub org references from always-further to nolabs-ai (#1235)
# → 72bcfd66 refactor(network): introduce NetworkIntent and remove ProxyOnly placeholders (#1225)
```
Clusters A (72bcfd66), C (cdeeb5b9, 46bcfbb9), E (c808f000). **CORRECT** — Cluster F hits recorded.

**Check 7: `crates/nono/src/capability.rs` → expect NO hits (#1225 CLI-only confirmation per D-04)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono/src/capability.rs
# → (no output)
```
Zero hits. **CORRECT** — confirms #1225 is CLI-side only; `NetworkMode::ProxyOnly` in core
library is not touched by this window. The fork's `capability.rs` ProxyOnly constructors
(~lines 1045/1065), match arms (~843/1386/2711+), and doc (~707) are untouched.

**Check 8: `crates/nono/src/manifest_convert.rs` → expect NO hits (#1225 CLI-only confirmation)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono/src/manifest_convert.rs
# → (no output)
```
Zero hits. **CORRECT** — confirms #1225 does not touch `manifest_convert.rs:47`
(secondary ProxyOnly consumer per D-04). Fork's manifest path is unaffected.

**Check 9: `crates/nono-cli/src/network_policy.rs` → expect NO hits (CompiledEndpointPolicy surface)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono-cli/src/network_policy.rs
# → (no output)
```
Zero hits. **CORRECT** — the `CompiledEndpointPolicy` / `endpoint_policy.evaluate()` surface
(v3.3 Phase 95 wiring) is NOT directly touched by any window commit. The carve-out re-touch
check above records the indirect hits via the proxy-side files (config.rs, reverse.rs, route.rs,
server.rs) which coordinate with network_policy.rs.

**Check 10: `crates/nono/Cargo.toml` → expect Clusters F and H (dep bump + release)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono/Cargo.toml
# → d817ed53 chore: release v0.66.0 (#1293)
# → 2e64798d chore(deps): bump sigstore-trust-root from 0.8.0 to 0.9.0 (#1229)
```
Clusters H (d817ed53) and F (2e64798d). **CORRECT** — won't-sync release + will-sync dep bump.

**Check 11: `crates/nono-proxy/src/server.rs` → expect Cluster C (HTTP/2 proxy evolution)**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono-proxy/src/server.rs
# → cdeeb5b9 feat(proxy): add HTTP/2 support for reverse proxy and credential injection (#983)
```
Cluster C (cdeeb5b9). **CORRECT** — split disposition confirmed; Phase 89 guard tests apply.

**Check 12: `crates/nono-cli/src/sandbox_prepare.rs` → expect Clusters A and C**
```bash
git log --oneline 1d1c88c9..d817ed53 -- crates/nono-cli/src/sandbox_prepare.rs
# → d457ecc3 fix(network): error early on contradictory network flag combinations (#1263)
# → cdeeb5b9 feat(proxy): add HTTP/2 support for reverse proxy and credential injection (#983)
```
Clusters A (d457ecc3) and C (cdeeb5b9). **CORRECT** — validate_block_net_conflicts() pairs
with NetworkIntent (#1263 → Cluster A); H2 CLI flag surfacing (#983 → Cluster C).

All 12 spot-check results match expected clusters. No classification errors detected.

---

## ADR Review

Overall phase ADR assessment across five standard dimensions (uniform actual-diff confirmed;
all cluster bodies complete; per-cluster risk matrix below):

| dimension | score | notes |
|-----------|-------|-------|
| security | M | No security-critical bug-fixes (unlike Phase 94's AF_UNIX deadlock fix); Cluster A (#1225) restructures how ProxyOnly intent flows to proxy — adoption affects proxy bypass risk; Cluster C (#983 H2) adds new proxy code surface with bounded buffers (+DoS mitigation noted); Cluster F sigstore dep bump is a security dependency; net: M |
| windows | H | Cluster A touches 7 files with cfg(windows) blocks; NetworkIntent adoption requires cross-target clippy on all 11 touched files; all other clusters are windows-touch:no; net: H (Cluster A dominates) |
| maintenance | H | Cluster A is 11-file CLI refactor (NetworkIntent cascades through ProxyLaunchOptions, all runtime files); Cluster C (#983) is a 26-file proxy commit requiring careful split extraction (tls_intercept/ won't-apply; pool.rs + shared surface extractable); 14 commits across 8 clusters requires careful ordering in Phase 99; net: H |
| divergence | H | Cluster A (#1225) directly conflicts with fork's ProxyOnly placeholder pattern in capability_ext.rs + CompiledEndpointPolicy wiring; Cluster C overlaps Cluster F proxy carve-out (5 commits); Cluster B entirely absent from fork; net: H (#1225 is the headline conflict) |
| contributor | H | Cluster A requires ADR decision (Plan 03) before Phase 99 can absorb — this is the sole blocking gate; other clusters have clear dispositions; Cluster C split work is well-defined but labor-intensive; net: H (ADR-98 is blocking) |

**Per-cluster risk matrix:**

| Cluster | security | windows | maintenance | divergence | contributor | Overall |
|---------|----------|---------|-------------|------------|-------------|---------|
| A | M | H | H | H | H | H (NetworkIntent restructures proxy intent flow; 7 cfg(windows) files; ADR-98 blocking gate; fork-CLI conflict is real and deep) |
| B | L | L | L | L | L | L (won't-sync; patches to absent subsystem; no Phase 99 work; carry-forward note only) |
| C | M | L | H | H | M | H (tls_intercept/ won't-apply; 26-file commit; Cluster F carve-out on all 3 commits; pool.rs + shared surface extractable; endpoint wiring CompiledEndpointPolicy compat check required) |
| D | L | L | L | L | L | L (additive linux.rs warning; cross-target clippy gate is only risk; no fork-invariant conflict) |
| E | L | L | L | L | L | L (mechanical nolabs-ai org ref update; per-file review is the only risk; package namespace identifiers correctly preserved) |
| F | M | L | L | L | L | M (sigstore-trust-root is a security dep; cascade compatibility check required; otherwise straightforward) |
| G | L | L | L | L | L | L (accuracy fixes; additive; no behavioral change; token.rs is Cluster F surface but change is annotation-only) |
| H | L | L | L | L | L | L (mechanical won't-sync; leapfrog floor 0.66.1 for Phase 100) |

**Outcome:** Confirm drain-then-sync shape. No escalation threshold reached — no security
regression, no library-boundary violation, no new infrastructure required. The sole blocking
gate is **ADR-98** (Plan 03, Cluster A): the NetworkIntent adopt-vs-fork-diverge decision must
be settled before Phase 99 can begin absorbing Cluster A. All other clusters have determinate
dispositions and clear Phase 99 guidance. Continue.

**Downstream routing (final — confirmed by actual-diff):**
- Cluster A → **ADR-98 (Plan 03) PENDING**; Phase 99 after ADR settles; cross-target clippy MUST
- Cluster B → **won't-sync**; future phase if tool-sandbox/ adopted; 4-commit carry-forward
- Cluster C → **Phase 99 split** (skip tls_intercept/ hunks; apply pool.rs + shared surface +
  endpoint wiring; Cluster F carve-out review; CompiledEndpointPolicy compat check)
- Cluster D → **Phase 99 will-sync** (apply 5b8e94da; cross-target clippy gate; linux.rs)
- Cluster E → **Phase 99 will-sync** (apply c808f000; per-file review for OscarMackJr/nono URLs)
- Cluster F → **Phase 99 will-sync** (apply 2e64798d; sigstore cascade compat check)
- Cluster G → **Phase 99 will-sync** (apply a4d68189; clean apply; token.rs Cluster F carve-out safe)
- Cluster H → **Phase 100 won't-sync** (leapfrog floor 0.66.1; D-15 recorded)
