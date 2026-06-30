# Phase 99: Upstream Absorb + Fork-Invariant Verify — Research

**Researched:** 2026-06-30
**Domain:** Rust upstream-sync / cherry-pick absorption + cross-target verification
**Confidence:** HIGH (all key commits reachable; cherry-pick feasibility empirically tested)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Phase 98 ledger is the binding scope for Phase 99 (not the ROADMAP SC PR list).
- **D-02:** Reconcile the stale SC text as a tracked Phase 99 task. Caveat: do not silently
  drop any SC-listed PR without confirming it is genuinely out-of-window or already synced.
- **D-03:** Hand-replay only applicable hunks for Cluster C. Never stage `tls_intercept/` or
  `h2_forward`/`h2_probe` hunks.
- **D-04:** Verify `CompiledEndpointPolicy` compatibility against Phase 95 fork divergence as
  part of Cluster C absorption.
- **D-05:** Cherry-pick with `-x` trailer where it applies cleanly; fall back to manual replay
  on conflict. (Research confirms: all clusters conflict — see Cherry-Pick Feasibility section.)
- **D-06:** One atomic DCO-signed commit per upstream commit. `Signed-off-by: Oscar Mack Jr
  <oscar.mack.jr@gmail.com>`. No squashing.
- **D-07:** Dependency order: Cluster A (`72bcfd66` then `d457ecc3`) first; then D/E/F/G;
  then Cluster C after A.
- **D-08:** Author focused regression tests for two ADR-98 Cluster A deviations: (a)
  `WSL2ProxyFallback` preservation in `profile/mod.rs`, (b) `CompiledEndpointPolicy`
  compatibility in `proxy_runtime.rs`.
- **D-09:** Run existing gates: Phase 89 proxy guard tests, linux.rs seccomp tests, both Unix
  cross-target clippy gates (no PARTIAL→CI), `make ci`, code-review + verifier pass.
- **D-10:** Produce an explicit fork-invariant carve-out checklist: AppContainer/WFP/broker,
  ADR-86 policy-free-library boundary, `exec_strategy_windows/` denial-rendering carve-out
  — none marked regressed.

### Claude's Discretion

Wave/plan breakdown is the planner's call. Suggested shape: (1) SC reconciliation + Cluster A
adopt with deviations + tests; (2) will-sync D/E/F/G; (3) Cluster C split hand-replay +
endpoint-policy compat; (4) full fork-invariant verify gate. Planner may merge/reorder so long
as D-07 dependency order and D-09 gates hold.

Exact test names/locations for the D-08 deviation tests are the executor's call.

### Deferred Ideas (OUT OF SCOPE)

- Cluster B (tool-sandbox) carry-forward — won't-sync, fork lacks `tool-sandbox/` dir.
- Cluster H release metadata / 0.66.1 leapfrog — Phase 100.
- TLS-interception capability — the dropped `tls_intercept/` + `h2_forward`/`h2_probe` hunks.
- MSI vcredist prereq, POC-cert/broker clean-host concerns (host-gated todos).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UPST11-02 | All will-sync feature/fix clusters absorbed without regressing Windows model or policy-free boundary | Covered by Cluster A/C/D/G absorb sections + guard test map; SC text needs D-02 reconciliation to match ledger (tool-sandbox → won't-sync; #1225 → key adoption) |
| UPST11-03 | Dependency, CI, and documentation clusters absorbed or reconciled; Cargo.lock regenerated; workspace builds clean | Covered by Cluster E/F/G absorb sections; Cluster F has sigstore-verify cascade complication (see Cluster F Cascade section — critical planning input) |
| UPST11-04 | Fork-divergent invariants explicitly preserved and verified; cross-target clippy GREEN on both Unix gates; `make ci` clean; code-review + verifier pass | Covered by Verification Gate Mechanics + Fork-Invariant Carve-out Checklist sections |
</phase_requirements>

---

## Summary

Phase 99 absorbs 9 upstream commits across 6 ledger clusters (A, C, D, E, F, G) from the
`nolabs-ai/nono` `v0.65.1..v0.66.0` window into the fork, in dependency order, then proves all
fork invariants are unregressed.

**The critical planning input from research:** Every cluster conflicts with the fork's HEAD when
`git cherry-pick --no-commit` is attempted. There are no clean `-x` cherry-picks available. All
clusters require the D-05 manual-replay fallback. The planner must structure each plan task as
"inspect the upstream diff, apply the applicable changes by hand, commit with upstream SHA in
trailer." This affects time estimation significantly — the `-x` fast-path is unavailable.

**Cluster F has a cascade complication:** The fork is at `sigstore-verify = "0.8.0"` while
upstream was already at `sigstore-verify = "0.9.0"` at the v0.65.1 baseline (the bump was in
commit `9e084cbb`, PR #1228, which predates the v0.65.1 release and was not absorbed in Phase
94). Cluster F (`2e64798d`) bumps `sigstore-trust-root` from `=0.8.0` to `=0.9.0` — but
upstream's context already had `sigstore-verify 0.9.0`. The Cluster F plan task must evaluate
whether to also bump `sigstore-verify` from 0.8.0 to 0.9.0 (and drop the `features = ["tuf"]`
the fork added in Phase 37) or confirm `sigstore-trust-root 0.9.0` is compatible with
`sigstore-verify 0.8.0`. [ASSUMED] compatibility needs explicit executor verification.

The `tls_intercept/` directory is CONFIRMED ABSENT from the fork (verified live). Cluster C
split is well-defined: apply `pool.rs` + shared proxy surfaces + CLI plumbing for `--allow-http2`;
skip the 5 `tls_intercept/` files entirely.

**Primary recommendation:** Plan in four waves: (1) SC reconciliation + Cluster A (NetworkIntent
full-sync-adopt + D-08 deviation tests); (2) will-sync clusters D/E/F/G one-per-plan; (3)
Cluster C split hand-replay with endpoint-policy compat verification; (4) fork-invariant verify
gate (both cross-target clippy gates + make ci + carve-out checklist + code-review/verifier).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| NetworkIntent enum (`pub(crate)`) | nono-cli | — | CLI owns all proxy intent policy; library stays policy-free per ADR-86 |
| `CapabilitySet` / `NetworkMode::ProxyOnly` | nono (library) | nono-cli (reads/writes) | Library type stable; CLI writes it at proxy-start with real port |
| `WindowsNetworkPolicyMode::ProxyOnly` | nono library (`sandbox/windows.rs`) | nono-cli exec_strategy_windows/ | Derived from library type at sandbox-apply time; not affected by NetworkIntent CLI refactor |
| `CompiledEndpointPolicy` / `evaluate()` | nono-proxy (`config.rs`, `route.rs`) | nono-cli (`network_policy.rs`, `proxy_runtime.rs`) | Proxy crate owns compiled policy; CLI builds it at launch time |
| `WSL2ProxyFallback` (fork-only) | nono-cli (`profile/mod.rs`) | — | Fork-specific WSL2 policy extension; must survive Cluster A adoption |
| Connection pooling (`pool.rs`) | nono-proxy | — | Intra-proxy; new module from Cluster C |
| Wildcard credential routing | nono-proxy (`route.rs`) | — | Intra-proxy fix from Cluster C `08ca19a8` |
| 9P filesystem warning | nono library (`sandbox/linux.rs`) | — | Additive diagnostic from Cluster D; cfg-gated linux |
| sigstore attestation trust root | nono library (`Cargo.toml`) | — | Direct dep in core library; Cluster F bump |

---

## Per-Cluster Absorption Surface (Standard Stack)

### Cluster A — NetworkIntent Full-Sync-Adopt

**Commits:** `72bcfd66` (#1225) then `d457ecc3` (#1263)
**All 11 files in `crates/nono-cli/src/` only.** [VERIFIED: git show 72bcfd66, d457ecc3]

| File | Change | Windows-touch |
|------|--------|---------------|
| `capability_ext.rs` | Remove `ProxyOnly { port: 0 }` placeholder; NetworkIntent-based path | YES (line 1027 placeholder + 1182 real port) |
| `command_runtime.rs` | Replace `ProxyOnly { .. }` match arm for dry-run path | YES (cfg(windows) blocks exist in file) |
| `execution_runtime.rs` | Replace proxy-activation detection with NetworkIntent checks | YES |
| `launch_runtime.rs` | Adapt port/bind_ports extraction; `validate_block_net_conflicts()` call added by d457ecc3 | YES |
| `main.rs` | Minor — NetworkIntent import/use | NO (auto-merges in test) |
| `output.rs` | `print_capabilities` gains `proxy_pending` parameter | YES |
| `profile/mod.rs` | NetworkIntent-related changes; **MUST preserve `Wsl2ProxyPolicy` at line 1983** | NO |
| `proxy_runtime.rs` | Replace placeholder-based proxy launch with NetworkIntent-based; **MUST verify `CompiledEndpointPolicy::compile() → evaluate()` chain** | NO |
| `sandbox_prepare.rs` | Rename field; `validate_block_net_conflicts()` added (d457ecc3, +256 lines) | NO, but gains `#[cfg(target_os = "linux")]` blocks (+2) |
| `supervised_runtime.rs` | Adapt port extraction to NetworkIntent-aware path | YES |
| `terminal_approval.rs` | NetworkIntent awareness for approval display | YES |

**Fork-specific deviations Phase 99 must track:**
1. `WSL2ProxyFallback` — `profile/mod.rs` line 1983: `pub enum Wsl2ProxyPolicy { Refuse, Allow }` + `wsl2_proxy_policy: Option<Wsl2ProxyPolicy>` field at line 2103. Upstream's `c808f000` diff also touches `profile/mod.rs`; both must be reconciled. [VERIFIED: grep confirmed Wsl2ProxyPolicy at line 1983]
2. `CompiledEndpointPolicy` compatibility — the type lives in `crates/nono-proxy/src/config.rs` lines 272–486 and is wired through `crates/nono-cli/src/network_policy.rs` (zero direct hits from `#1225` per ledger Check 9). The conflict is indirect: `proxy_runtime.rs` appears in BOTH the #1225 diff and the endpoint-policy surface. [VERIFIED: grep confirms CompiledEndpointPolicy at config.rs:272, route.rs:13, reverse.rs:121, network_policy.rs all]

**cherry-pick result (empirically tested):** CONFLICT on `command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `proxy_runtime.rs`, `sandbox_prepare.rs`. Auto-merges: `capability_ext.rs`, `output.rs`, `profile/mod.rs`. Manual replay required for conflicting files.

### Cluster C — Proxy HTTP/2 + Endpoint Routing (SPLIT)

**Commits:** `cdeeb5b9` (#983, split), `46bcfbb9` (#1127, apply), `08ca19a8` (#1243, apply)
**tls_intercept/ CONFIRMED ABSENT in fork.** [VERIFIED: ls crates/nono-proxy/src/ — no tls_intercept/]

#### cdeeb5b9 (#983) — Apply vs Skip

| File | Lines | Action |
|------|-------|--------|
| `crates/nono-proxy/src/pool.rs` | 365 (new) | **APPLY** — connection pooling; independent of tls_intercept/ |
| `crates/nono-proxy/src/lib.rs` | +1 | **APPLY** — `pub mod pool` addition |
| `crates/nono-proxy/src/reverse.rs` | 446 changed | **APPLY applicable hunks** — pool-based request construction; skip tls_intercept integration hunks (h2_forward, h2_probe references) |
| `crates/nono-proxy/src/route.rs` | 52 changed | **APPLY applicable hunks** — post Cluster F carve-out review; Phase 89 guard tests must pass |
| `crates/nono-proxy/src/server.rs` | 91 changed | **APPLY applicable hunks** — `--allow-http2` flag plumbing; skip tls_intercept/ wiring |
| `crates/nono-proxy/src/credential.rs` | 6 changed | **APPLY** — small improvements |
| `crates/nono-proxy/src/config.rs` | 7 changed | **APPLY** — small improvements |
| `crates/nono-proxy/Cargo.toml` | 7 changed | **APPLY applicable** — `hyper-util`, h2 dep additions for pool; skip tls_intercept-specific deps if any |
| `crates/nono-cli/src/cli.rs` | 18 changed | **APPLY** — `--allow-http2` CLI flag |
| `crates/nono-cli/src/launch_runtime.rs` | 2 changed | **APPLY** |
| `crates/nono-cli/src/main.rs` | 2 changed | **APPLY** |
| `crates/nono-cli/src/profile/mod.rs` | 8 changed | **APPLY** — `network.allow_http2` profile field |
| `crates/nono-cli/src/proxy_runtime.rs` | 3 changed | **APPLY** |
| `crates/nono-cli/src/sandbox_prepare.rs` | 12 changed | **APPLY** |
| `crates/nono-cli/data/nono-profile.schema.json` | 4 changed | **APPLY** — schema update for allow_http2 field |
| `crates/nono-proxy/src/tls_intercept/acceptor.rs` | 150 changed | **SKIP** — fork lacks tls_intercept/ |
| `crates/nono-proxy/src/tls_intercept/h2_forward.rs` | 2607 (new) | **SKIP** — fork lacks tls_intercept/ |
| `crates/nono-proxy/src/tls_intercept/h2_probe.rs` | 352 (new) | **SKIP** — fork lacks tls_intercept/ |
| `crates/nono-proxy/src/tls_intercept/handle.rs` | 679 changed | **SKIP** — fork lacks tls_intercept/ |
| `crates/nono-proxy/src/tls_intercept/mod.rs` | 16 changed | **SKIP** — fork lacks tls_intercept/ |

**Do NOT create `tls_intercept/`** as a side effect of the split.

#### 46bcfbb9 (#1127) — endpoint wiring
Adds `endpoint_restrictions: Vec<(String, EndpointRule)>` to `ProxyLaunchOptions` in `proxy_runtime.rs` (+225/-8). The `crates/nono-cli/src/` field references `nono_proxy::config::EndpointRule` (existing cross-crate dep pattern). Verify `CompiledEndpointPolicy` compatibility (D-04): the fork's `compile()` → `evaluate()` chain in `network_policy.rs` must not be duplicated or bypassed. [VERIFIED: confirmed 46bcfbb9 only touches launch_runtime.rs and proxy_runtime.rs per ledger]

#### 08ca19a8 (#1243) — wildcard route fix
Targeted additive fix in `crates/nono-proxy/src/route.rs` (+102/-4). Wildcard pattern matching for credential upstream routes. Confirm `_ep_` key namespace preserved (Phase 89 Reconciliation Addendum). [VERIFIED: ledger re-export scan found no new pub items]

### Cluster D — 9P Filesystem Warning

**Commit:** `5b8e94da` — additive `crates/nono/src/sandbox/linux.rs` (+128 lines)
**cherry-pick result (empirically tested):** CONFLICT in linux.rs test section. The fork adds GPU device tests (`test_is_nvidia_compute_device_*`) after the upstream test section that `5b8e94da` modifies, causing a 3-way merge conflict. Manual replay required: extract the `is_9p_path()` function and the 9P warning insertion point from the upstream diff, apply by hand without touching the fork's GPU test additions.

### Cluster E — Org Reference Migration

**Commit:** `c808f000` — 31 files total; 8 in drift-filter (src/ files)
**cherry-pick result (empirically tested):** CONFLICT in `profile/mod.rs`, `proxy_runtime.rs`, `setup.rs`, `route.rs` (content conflicts); modify-delete on `command_policy.rs` and `migration.rs` (deleted in fork, modified in upstream).

**Manual replay approach:** For each drift-filter-relevant file that exists in the fork, apply only the `always-further/nono` → `nolabs-ai/nono` string replacements where they appear. Files deleted in fork (`command_policy.rs`, `migration.rs`): no action needed — the string doesn't exist. **Preserve `OscarMackJr/nono` fork-identity URLs in all production code paths.** [VERIFIED: ledger confirms `update_check.rs` only changes a test fixture JSON value, not the production `NONO_UPDATE_URL` constant]

The `.github/workflows/` conflicts are out of the drift-filter scope; handle per the fork's own CI structure (the fork's workflows are already different from upstream's).

### Cluster F — Sigstore-Trust-Root Bump (CASCADE WARNING)

**Commit:** `2e64798d` — `crates/nono/Cargo.toml` 1 line change + Cargo.lock regen
**cherry-pick result (empirically tested):** CONFLICT in both `Cargo.toml` and `Cargo.lock`.

**CRITICAL FINDING — Sigstore-verify version gap:**
The fork currently has `sigstore-verify = { version = "0.8.0", features = ["tuf"] }`. The
upstream's `Cargo.toml` at the v0.65.1 window base ALREADY had `sigstore-verify = "0.9.0"`
(no "tuf" feature). The bump `9e084cbb` (PR #1228) moved sigstore-verify from 0.8.0 to 0.9.0
BEFORE the window base — this bump was in the Phase 94 UPST10 window but was NOT absorbed.
[VERIFIED: `git show 1d1c88c9:crates/nono/Cargo.toml | grep sigstore` confirms `sigstore-verify = "0.9.0"` at v0.65.1 tip]

**Cascade requirements for Cluster F task:**
1. Edit `crates/nono/Cargo.toml`: change `sigstore-trust-root = "=0.8.0"` → `"=0.9.0"`
2. Evaluate whether to ALSO bump `sigstore-verify` from 0.8.0 to 0.9.0 and drop `features = ["tuf"]`. Two sub-tasks for the executor:
   - Check if `sigstore-trust-root 0.9.0` is API-compatible with `sigstore-verify 0.8.0` at the Rust usage sites (search for `sigstore_trust_root::` usage in `crates/nono/src/`)
   - Check if `sigstore-verify 0.9.0` dropped the `"tuf"` feature or renamed it
3. Run `cargo update -p sigstore-trust-root` (and if also bumping verify: `cargo update -p sigstore-verify`) to regen Cargo.lock
4. Run `cargo audit` / check for RUSTSEC advisories on the new resolved versions
5. Confirm `make build` passes clean after the Cargo.lock regen

[ASSUMED] that `sigstore-trust-root 0.9.0` and `sigstore-verify 0.8.0` can resolve together without a hard dependency conflict — the upstream Cargo.lock after `2e64798d` shows both 0.8.0 and 0.9.0 sigstore-crypto packages coexisting, which is consistent with a dual-version resolution. The executor MUST verify this at apply time.

### Cluster G — Proxy Docs + Deprecated Flag Metadata Fix

**Commit:** `a4d68189` — `cli.rs` (+2/-2), `cli_bootstrap.rs` (+62/-30), `nono-proxy/README.md` (+4/-2), `token.rs` (+11/-7)
**cherry-pick result (empirically tested):** CONFLICT in `cli.rs` and `token.rs`.

The conflicts are because the fork's `cli.rs` and `token.rs` have diverged from upstream's. Manual replay: extract the specific `X-Nono-Token` accuracy fixes and deprecated flag display changes from the diff and apply them to the fork's current versions. The `cli_bootstrap.rs` change (formatting/restructuring) auto-merged — apply cleanly. `token.rs` change is annotation/metadata only (no behavioral change to `EffectiveProxySettings`, `_ep_` key namespace, or the activation predicate). [VERIFIED: ledger confirms token.rs change is metadata-only]

---

## Cherry-Pick Feasibility Summary (Empirically Tested)

**Method:** `git cherry-pick --no-commit <sha>` on current HEAD (`7e916672`), then `git cherry-pick --abort && git reset --hard HEAD`.

| Cluster | SHA | cherry-pick -x result | Reason | Action |
|---------|-----|----------------------|--------|--------|
| A #1225 | `72bcfd66` | CONFLICT | `command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `proxy_runtime.rs`, `sandbox_prepare.rs` diverged | Manual replay |
| A #1263 | `d457ecc3` | (untested; depends on A first commit being applied) | Uses NetworkIntent types from 72bcfd66 | Manual replay after A |
| D | `5b8e94da` | CONFLICT | linux.rs GPU test additions in fork conflict with upstream test section | Manual replay |
| E | `c808f000` | CONFLICT | `profile/mod.rs`, `proxy_runtime.rs`, `setup.rs`, `route.rs` content conflicts; `command_policy.rs`/`migration.rs` modify-delete | Manual replay |
| F | `2e64798d` | CONFLICT | `Cargo.toml` sigstore-verify version divergence; `Cargo.lock` diverged | Manual replay (edit + cargo update) |
| G | `a4d68189` | CONFLICT | `cli.rs`, `token.rs` diverged | Manual replay |
| C | `cdeeb5b9` | N/A (split) | tls_intercept/ SKIP required | Hand-replay applicable hunks only |
| C | `46bcfbb9` | N/A (split) | endpoint wiring after Cluster A adoption | Hand-replay |
| C | `08ca19a8` | N/A (split) | wildcard route fix | Hand-replay |

**Conclusion:** The D-05 fallback (manual replay) is the universal mechanic for Phase 99. Plan tasks must specify "extract the upstream diff, apply applicable changes by hand, commit with `(cherry picked from commit <sha>)` trailer and DCO sign-off."

---

## D-02 SC Reconciliation Map

The ROADMAP Phase 99 SC and REQUIREMENTS UPST11-02/03 list PRs from the preliminary
`260629-toe` guess. Per D-02, the planner must reconcile these against the ledger reality.

### UPST11-02 SC text — what needs changing

Current (stale) text includes: "tool-sandbox (#1268 self-invocation policy, #1271 @git:common-dir
token, #1253 skip-missing-dirs, #1249 TLS-trust-bundle env)"

Required reconciliation:
- **ADD** at the front: `network: #1225 NetworkIntent full-sync-adopt (ADR-98), #1263 contradictory-flags validation`
- **REPLACE** tool-sandbox list with: `tool-sandbox: won't-sync (Cluster B — fork lacks tool-sandbox/; carry-forward to future phase)`
- **NOTE** for #1213: `tests: 30cfee67 (#1213 e2e exec-strategy integration tests) — out of src/ filter (in tests/ subdir); evaluated per ledger noise section; adopt separately if desired after cfg-gated Unix absorbs`

### UPST11-03 SC text — what needs changing

Current text includes: `criterion 0.5.1→0.8.2 (#1232), CI compile-step mapping fix (#1251)` and
`proxy docs (#1247 activation, #1246 stale X-Nono-Token)`.

Required reconciliation:
- `criterion 0.5.1→0.8.2 (#1232)`: Out of filter — only `nono-cli/Cargo.toml`, not `nono/Cargo.toml` (drift filter). Mark as `N/A — out of drift-filter scope; nono-cli/Cargo.toml change; evaluate separately`.
- CI compile-step fix `#1251`: Out of filter — CI yaml only. Mark as `N/A (CI yaml); defer to Phase 100`.
- proxy docs `#1247`: Out of filter — `crates/nono-cli/data/` directory (not src/). Mark as `N/A (data dir, out of filter); reconcile with fork's proxy-divergence docs`.
- `#1246 stale X-Nono-Token`: IN SCOPE — Cluster G (`a4d68189`). Keep as-is.

### Full PR→Cluster mapping (for auditable drop/keep justification)

| PR | SHA | Cluster | Phase 99 action |
|----|-----|---------|-----------------|
| #1225 | `72bcfd66` | A | IN SCOPE — full-sync-adopt (ADR-98) |
| #1263 | `d457ecc3` | A | IN SCOPE — companion to #1225 |
| #983 | `cdeeb5b9` | C | IN SCOPE — split (skip tls_intercept/) |
| #1127 | `46bcfbb9` | C | IN SCOPE — apply |
| #1243 | `08ca19a8` | C | IN SCOPE — apply |
| #1207 | `5b8e94da` | D | IN SCOPE — will-sync |
| #1235 | `c808f000` | E | IN SCOPE — will-sync (per-file review) |
| #1229 | `2e64798d` | F | IN SCOPE — will-sync (cascade check) |
| #1246 | `a4d68189` | G | IN SCOPE — will-sync |
| #1268 | `691e0f4f` | B | WON'T-SYNC — fork lacks tool-sandbox/ |
| #1271 | `7011bc85` | B | WON'T-SYNC — fork lacks tool-sandbox/ |
| #1253 | `d2252225` | B | WON'T-SYNC — fork lacks tool-sandbox/ |
| #1249 | `853d5236` | B | WON'T-SYNC — fork lacks tool-sandbox/ |
| #1213 | `30cfee67` | Noise | OUT OF FILTER — tests/ not src/; evaluate separately |
| #1232 | `5441f4eb` | Noise | OUT OF FILTER — nono-cli/Cargo.toml (not nono/Cargo.toml) |
| #1251 | `84b5e7ce` | Noise | OUT OF FILTER — CI yaml |
| #1247 | `8aee0e77` | Noise | OUT OF FILTER — data/ dir |
| #1293 | `d817ed53` | H | WON'T-SYNC — release metadata; Phase 100 leapfrog 0.66.1 |

All PRs from ROADMAP SC are in the `v0.65.1..v0.66.0` window; none are out-of-window or
already-synced-in-v3.3. [VERIFIED: per-cluster commit tables in 98-DIVERGENCE-LEDGER.md]

---

## Guard Test Map (for D-09 verification)

All test names confirmed by `grep` against the live codebase.

### Phase 89 Proxy Guard Tests

| Test name | File | Guards |
|-----------|------|--------|
| `proxy_activates_with_custom_credentials_only` | `crates/nono-cli/src/proxy_runtime.rs:503` | Proxy activation predicate (survives NetworkIntent adoption) |
| `block_net_overrides_custom_credentials_activation` | `crates/nono-cli/src/proxy_runtime.rs:569` | block-net + proxy conflict handling |
| `build_proxy_config_maps_upstream_proxy_to_external_proxy` | `crates/nono-cli/src/proxy_runtime.rs:483` | ProxyConfig construction |
| `connect_keeps_open_on_missing_proxy_auth` | `crates/nono-proxy/src/connect.rs:432` | Credential route on proxy |
| `denied_endpoint_returns_403_and_audit` | `crates/nono-proxy/src/reverse.rs:1343` | CompiledEndpointPolicy evaluate() chain — PRIMARY Cluster A/C signal |
| `allow_domain_endpoint_route_does_not_shadow_credential_route` | `crates/nono-proxy/src/route.rs:602` | Route resolution order |

### Phase 95/96 Linux Seccomp Guard Tests

> Note: CONTEXT.md D-09 references `proxy_no_v4_seccomp` / `proxy_v4_no_seccomp`. The actual
> function names in the codebase are listed below. [VERIFIED: grep on crates/ --include="*.rs"]

| Test name | File | Guards |
|-----------|------|--------|
| `test_proxy_only_with_landlock_v4_returns_no_fallback` | `crates/nono/src/sandbox/linux.rs:4477` | ProxyOnly on Linux V4+ kernel (no seccomp fallback needed) |
| `test_seccomp_network_fallback_mode_proxy_only` | `crates/nono/src/sandbox/linux.rs:3886` | ProxyOnly on pre-V4 kernel (seccomp fallback activates) |
| `test_seccomp_network_fallback_mode_proxy_only_with_bind` | `crates/nono/src/sandbox/linux.rs:3898` | ProxyOnly with bind_ports on pre-V4 |

### D-08 New Deviation Tests (Executor's call on exact names)

| Target | Expected behavior to test | Suggested home |
|--------|--------------------------|----------------|
| WSL2ProxyFallback preservation | Post-#1225 adoption: `Wsl2ProxyPolicy::Refuse`/`Allow` still loads from profile; does not regress to None or wrong mode | `profile/mod.rs` test module or `proxy_runtime.rs` test module |
| CompiledEndpointPolicy compatibility | Post-#1225 `proxy_runtime.rs` rewrite: `CompiledEndpointPolicy::compile()` → `evaluate()` chain still produces 403 for denied endpoints and correct route for allowed | `proxy_runtime.rs` test module (or covered by Phase 89 `denied_endpoint_returns_403_and_audit` if that test passes) |

---

## Verification Gate Mechanics

Source: `.planning/templates/cross-target-verify-checklist.md` + `CLAUDE.md`. [VERIFIED: read directly]

### Cross-Target Clippy Gates (MANDATORY — no PARTIAL→CI)

```bash
# Gate 1: linux-gnu (Docker + cross 0.2.5)
# Prerequisite: docker info 2>&1 | grep "Server Version"  ← must show a version
cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used

# Gate 2: apple-darwin (zig 0.16.0 + cargo-zigbuild 0.23.0; SDKROOT MUST stay UNSET)
# Use the direct-binary form — NOT "cargo zigbuild clippy"
cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

**PARTIAL→CI is the fallback ONLY on a documented runner failure** (captured image-pull error
for linux-gnu, or captured macOS-SDK/framework link error for apple-darwin). A stopped Docker
daemon is NOT a documented failure — start the daemon. Missing-but-installable tools are NOT a
failure — install them.

### Full Suite

```bash
make ci    # clippy + fmt + tests (all three must pass)
```

### Windows Local Gate (still needed — does NOT substitute for cross-target)

```bash
cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used
```

Note: `--all-targets` is required (not `--bin nono`) to catch `nono-ffi` exhaustive-match
errors (ADR-86 D-04 lesson). Windows-host clippy covers Windows cfg branches; it is
STRUCTURALLY BLIND to `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` code.

### Cross-Target Triggers in This Phase

Clusters that REQUIRE cross-target clippy (CLAUDE.md MUST rule):

| Cluster | Files | Cfg-gate trigger |
|---------|-------|-----------------|
| A | `supervised_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `command_runtime.rs`, `terminal_approval.rs`, `sandbox_prepare.rs` | `cfg(target_os = "linux")` and `cfg(target_os = "macos")` blocks in these files |
| D | `crates/nono/src/sandbox/linux.rs` | `#[cfg(target_os = "linux")]` throughout |

---

## Fork-Invariant Carve-Out Checklist (for D-10)

The planner must produce a plan task that generates this checklist with explicit verdicts.
Each entry must be marked "Verified unregressed" or "Regressed: <what changed>".

| Invariant | Location | What to Check | Status after Phase 99 |
|-----------|----------|---------------|----------------------|
| AppContainer / WFP / broker Windows backends | `exec_strategy_windows/` (`launch.rs`, `network.rs`, `mod.rs`, `restricted_token.rs`, `dacl_guard.rs`) | None of these files are touched by any in-scope cluster commit. `exec_strategy_windows/` is excluded from drift-filter scope. [VERIFIED: DIVERGENCE-LEDGER confirms windows-touch:no for all clusters except A; exec_strategy_windows/ explicitly excluded from filter] | — |
| ADR-86 policy-free-library boundary | `crates/nono/src/` (core library) | NetworkIntent is `pub(crate)` within nono-cli only. `NetworkMode::ProxyOnly` in `crates/nono/src/capability.rs` (variant def + constructors + match arms) untouched by any in-scope commit. No audit or diagnostic logic moves into/out of library as a result of Phase 99 absorbs. [VERIFIED: ledger re-export scan + actual-diff checks 7–9] | — |
| `exec_strategy_windows/` denial-rendering carve-out (ADR-86 D-02) | `crates/nono-cli/src/exec_strategy_windows/network.rs` lines 464–514 | `WindowsNetworkPolicyMode::ProxyOnly` in `exec_strategy_windows/network.rs` reads an enforcement-time type derived from the library's `NetworkMode::ProxyOnly` at sandbox-apply time — NOT the CLI intent type `NetworkIntent`. No changes to `exec_strategy_windows/` are required under NetworkIntent adoption. [VERIFIED: ADR-98 Consequences + grep confirmed WindowsNetworkPolicyMode::ProxyOnly at lines 464/466/494/496/500/512/514 in network.rs] | — |

---

## Common Pitfalls

### Pitfall 1: Leaving linux.rs GPU tests unresolved during Cluster D replay

**What goes wrong:** Cluster D's `5b8e94da` conflicts in the `mod tests` section of `linux.rs`
because the fork has added `test_is_nvidia_compute_device_*` test functions that didn't exist in
upstream. A naive manual replay might discard the fork's GPU tests.

**How to avoid:** Apply the 9P warning addition (`is_9p_path()` function + the insertion before
the Landlock capability-add loop) without touching the fork's test section additions. The 9P
warning code is pure addition — insert it in the right place in the non-test source, then verify
the fork's GPU tests and existing seccomp tests still compile and pass.

### Pitfall 2: Deleting fork-identity URLs during Cluster E replay

**What goes wrong:** The `c808f000` org-ref migration replaces ALL `always-further` references
including some that should stay as `OscarMackJr/nono` in the fork.

**How to avoid:** For each file in the drift-filter set, manually inspect: only replace
`always-further/nono.git` → `nolabs-ai/nono.git` (upstream references), never
`OscarMackJr/nono` (fork identity). The `update_check.rs` test fixture JSON change (URL in a
test, not the production `NONO_UPDATE_URL` constant) is safe to apply. [VERIFIED: ledger
confirms the `update_check.rs` diff only changes a test fixture, not the production constant]

### Pitfall 3: Staging tls_intercept/ files during Cluster C split

**What goes wrong:** Using a broad `git apply` or manual copy-paste that includes `tls_intercept/` hunks, creating a `tls_intercept/` directory that doesn't exist in the fork and adding ~3,600 lines of TLS-interception code the fork deliberately doesn't have.

**How to avoid:** Extract the split via `git show cdeeb5b9 -- <file>` for EACH applicable file
individually. Never use `git apply` on the full `cdeeb5b9` diff. Verify after the commit that
`ls crates/nono-proxy/src/` contains no `tls_intercept/` directory.

### Pitfall 4: Missing the sigstore-verify version gap in Cluster F

**What goes wrong:** Applying only `sigstore-trust-root = "=0.9.0"` while leaving
`sigstore-verify = "0.8.0"` — then hitting a Cargo resolver conflict, API break, or
RUSTSEC advisory that `cargo build` surfaces.

**How to avoid:** Before applying `2e64798d`, check the Cargo resolver compatibility: run
`cargo update -p sigstore-trust-root` and inspect the full Cargo.lock diff. If the resolver
requires `sigstore-verify 0.9.0` to resolve, also bump `sigstore-verify`. Check if the `"tuf"`
feature still exists in `sigstore-verify 0.9.0` (upstream removed it in their 0.9.0 bump
based on the diff context showing no features). Run `cargo audit` after the regen.

### Pitfall 5: Breaking CompiledEndpointPolicy evaluate() chain during Cluster A

**What goes wrong:** Applying the `proxy_runtime.rs` NetworkIntent rewrite without verifying
that `CompiledEndpointPolicy::compile() → evaluate()` chain still runs. The conflict in
`proxy_runtime.rs` is deep (it's in BOTH the #1225 diff AND the Phase 95 endpoint-policy
surface), and a naive resolution might drop the fork's endpoint-policy wiring.

**How to avoid:** After applying Cluster A, run `denied_endpoint_returns_403_and_audit` before
proceeding to Cluster C. If it fails, the `proxy_runtime.rs` resolution is broken — fix it
before committing. This test is the primary verification signal for the CompiledEndpointPolicy
chain (D-04).

### Pitfall 6: Using `--bin nono` scope instead of `--workspace --all-targets`

**What goes wrong:** `--bin nono` scope hides `nono-ffi` exhaustive-match errors (ADR-86 D-04
lesson, Phase 84). A new NetworkIntent variant or a new sigstore type added to an `#[repr(C)]`
enum in `nono-ffi` won't be caught.

**How to avoid:** Always use `cargo clippy --workspace --all-targets` for Windows-host lint.
Always use `cross clippy --workspace` (not `--bin nono`) for linux-gnu gate. The `--all-targets`
flag ensures `cdylib`/`staticlib` targets in `bindings/c/` are included.

### Pitfall 7: Misidentifying `proxy_no_v4_seccomp` / `proxy_v4_no_seccomp` test names

**What goes wrong:** CONTEXT.md D-09 uses abbreviated test names that do not match the actual
function names in the codebase. If the executor searches for `proxy_no_v4_seccomp`, they won't
find it and may conclude the tests don't exist.

**How to avoid:** Use the actual function names confirmed by research:
`test_proxy_only_with_landlock_v4_returns_no_fallback`, `test_seccomp_network_fallback_mode_proxy_only`,
`test_seccomp_network_fallback_mode_proxy_only_with_bind`. Run them via
`cargo test -p nono --lib sandbox::linux::tests::test_proxy_only_with_landlock_v4_returns_no_fallback`.

---

## Code Examples (Verified Patterns)

### Upstream NetworkIntent shape (from git show 72bcfd66)

```rust
// crates/nono-cli/src/sandbox_prepare.rs (upstream post-#1225)
// Source: git show 72bcfd66 -- crates/nono-cli/src/sandbox_prepare.rs
pub(crate) enum NetworkIntent {
    Unrestricted,
    BlockAll,
    ProxyFiltered(BoxedProxyLaunchOptions),
}
// pub(crate) accessors added alongside:
// pub(crate) fn is_proxy_active(&self) -> bool
// pub(crate) fn proxy_options(&self) -> Option<&ProxyLaunchOptions>
```

### Current fork WSL2ProxyFallback location (must survive Cluster A)

```rust
// crates/nono-cli/src/profile/mod.rs:1983 (current fork)
// Source: grep verified live
pub enum Wsl2ProxyPolicy { /* Refuse, Allow */ }
// line 2103: wsl2_proxy_policy: Option<Wsl2ProxyPolicy>
```

### CompiledEndpointPolicy chain (must survive Cluster A + C)

```rust
// crates/nono-proxy/src/config.rs:272, 350  (current fork)
// Source: grep verified live
pub struct CompiledEndpointPolicy { default: EndpointPolicyDefault, /* ... */ }
impl CompiledEndpointPolicy {
    pub fn compile(policy: Option<&EndpointPolicyConfig>) -> Self { /* ... */ }
    // pub fn evaluate(...) at config.rs:~486
}
// Used in:  route.rs:13/41/91, reverse.rs:121, network_policy.rs (all)
```

### Manual replay commit trailer pattern (D-06)

```bash
# One commit per upstream commit; DCO sign-off required
git commit -m "$(cat <<'EOF'
feat(network): introduce NetworkIntent and remove ProxyOnly placeholders

Upstream refactor #1225 / 72bcfd66: replace ProxyOnly { port: 0 }
placeholder with explicit NetworkIntent enum in the CLI layer.
Fork-specific deviations: WSL2ProxyFallback preserved in profile/mod.rs;
CompiledEndpointPolicy wiring preserved in proxy_runtime.rs.

(cherry picked from commit 72bcfd66f98f0a1f3ff79cff1509713aaec7cdb0)

Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Project Constraints (from CLAUDE.md)

- **Policy-free-library:** Library applies ONLY what clients add to `CapabilitySet`. NetworkIntent MUST remain `pub(crate)` in nono-cli only. No intent types in `crates/nono/`.
- **Unwrap policy:** Strictly forbid `.unwrap()` and `.expect()` in new code; enforced by `clippy::unwrap_used`.
- **Cross-target clippy MUST:** Any commit touching cfg-gated Unix code MUST run both local cross-target gates (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`). PARTIAL→CI is retired (Phase 96 D-07).
- **`--workspace --all-targets` mandatory:** Not `--bin nono`. Catches nono-ffi exhaustive-match errors.
- **DCO sign-off:** Every commit: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
- **Five crates:** Workspace = nono, nono-cli, nono-proxy, nono-shell-broker, nono-ffi. Version/dep changes must consider all relevant `Cargo.toml` files.
- **Conventional Commit Title:** Subject lowercase after type prefix; standard types only (feat/fix/refactor/docs/chore/test). Verifier CI rejects uppercase-initial subjects.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `sigstore-trust-root 0.9.0` can resolve alongside `sigstore-verify 0.8.0` without a hard dependency conflict | Cluster F Cascade | If incompatible, Cluster F task must also bump `sigstore-verify` 0.8.0→0.9.0 and remove `features = ["tuf"]`; adds effort and API compat check |
| A2 | `sigstore-verify 0.9.0` (upstream) removed the `"tuf"` feature flag | Cluster F Cascade | If feature still exists and is needed, the fork can keep it; if renamed, need to update |
| A3 | The `proxy_runtime.rs` conflict from Cluster A can be resolved without losing the `CompiledEndpointPolicy` chain (i.e., `denied_endpoint_returns_403_and_audit` will pass after resolution) | Cluster A | If the conflict resolution is complex enough that the endpoint-policy chain is broken, it must be fixed inline before proceeding |
| A4 | The `Cargo.toml` comment in nono/Cargo.toml referencing "Phase 37 Plan 37-06" explains why sigstore-verify was at 0.8.0; no deliberate fork-preserve carve-out was recorded for staying at 0.8.0 | Cluster F Cascade | If there's a hidden reason the fork deliberately stayed at 0.8.0 (undocumented), bumping to 0.9.0 might regress something |

**If table is empty for any claim:** Only A1–A4 are assumed; all other claims were verified via
direct code inspection or git show against the live repository.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cross` | linux-gnu clippy gate | ✓ (confirmed by CLAUDE.md + Phase 96) | 0.2.5 | None — MANDATORY gate |
| Docker (linux engine) | `cross clippy` | Must be started | 29.5.3 (Phase 96 record) | None — start daemon |
| `zig` | apple-darwin clippy gate | ✓ (confirmed by Phase 96) | 0.16.0 | None — MANDATORY gate |
| `cargo-zigbuild` | apple-darwin clippy gate | ✓ (confirmed by Phase 96) | 0.23.0 | None — MANDATORY gate |
| `upstream` git remote | cherry-pick / manual replay | ✓ | nolabs-ai/nono configured | — |
| All 9 upstream commits | commit inspection | ✓ (all reachable: `git cat-file -t 72bcfd66` → commit) | — | — |

**Missing dependencies with no fallback:** None — all required tooling confirmed available.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner + `cargo test` |
| Config file | None (`Cargo.toml` workspace; `make test` / `make ci`) |
| Quick run command | `cargo test -p nono-proxy --lib reverse::tests::denied_endpoint_returns_403_and_audit` (single guard test) |
| Full suite command | `make ci` (clippy + fmt + tests) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Exists? |
|--------|----------|-----------|-------------------|---------|
| UPST11-02 | Cluster A: NetworkIntent adoption without endpoint-policy regression | unit | `cargo test -p nono-proxy --lib` (incl. `denied_endpoint_returns_403_and_audit`) | ✅ |
| UPST11-02 | Cluster A: WSL2ProxyFallback preserved | unit | New test (D-08); in `proxy_runtime.rs` or `profile/mod.rs` test module | ❌ Wave 0 (new test) |
| UPST11-02 | Cluster A: CompiledEndpointPolicy compat | unit | New test (D-08); or covered by `denied_endpoint_returns_403_and_audit` | ❌ Wave 0 (verify coverage) |
| UPST11-02 | Cluster D: linux.rs fork invariants (seccomp/cgroup) survive 9P addition | unit | `cargo test -p nono --lib sandbox::linux` | ✅ |
| UPST11-03 | Cluster F: workspace builds after sigstore bump | build | `make build` | ✅ (run after bump) |
| UPST11-04 | linux-gnu cross-target clippy GREEN | build | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | ✅ (gate) |
| UPST11-04 | apple-darwin cross-target clippy GREEN | build | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✅ (gate) |
| UPST11-04 | Fork-invariant carve-out checklist | manual | Code review + checklist completion (D-10) | ❌ Wave 0 (new checklist) |

### Sampling Rate

- **Per cluster commit:** `cargo test -p nono-proxy --lib` (Phase 89 proxy guard tests) after each Cluster A or C commit; `cargo test -p nono --lib sandbox::linux` after Cluster D commit
- **Per wave merge:** `make ci` (clippy + fmt + full test suite)
- **Phase gate:** Both cross-target clippy gates GREEN + `make ci` + code-review/verifier pass before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] D-08 new tests for `WSL2ProxyFallback` deviation (profile/mod.rs or proxy_runtime.rs test module)
- [ ] D-08 new tests for `CompiledEndpointPolicy` compat (or confirm coverage from `denied_endpoint_returns_403_and_audit`)
- [ ] D-10 fork-invariant carve-out checklist document (with verdict lines, not just template)

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (proxy credential injection) | Existing `CompiledEndpointPolicy::evaluate()` + Phase 89 guard tests |
| V3 Session Management | no | — |
| V4 Access Control | yes (NetworkIntent → sandbox enforcement path) | `NetworkMode::ProxyOnly` in library; fail-secure ProxyOnly logic |
| V5 Input Validation | yes (endpoint rule parsing, org-ref URL inputs) | Existing validation in `network_policy.rs` + route parsing |
| V6 Cryptography | yes (sigstore-trust-root bump) | sigstore-verify + sigstore-trust-root; never hand-roll |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Credential bypass via NetworkIntent adoption | Spoofing / Tampering | `proxy_activates_with_custom_credentials_only` guard test; `denied_endpoint_returns_403_and_audit` |
| EndpointPolicy bypass via Cluster C proxy surface changes | Tampering | `allow_domain_endpoint_route_does_not_shadow_credential_route`; CompiledEndpointPolicy compat check |
| Malicious sigstore-trust-root 0.9.0 (supply chain) | Tampering | Dependabot-generated commit; verify against RUSTSEC advisory database after lock regen |
| tls_intercept/ accidentally introduced by Cluster C split | Elevation of privilege | Verify `ls crates/nono-proxy/src/` post-commit: no `tls_intercept/` dir |

---

## Open Questions

1. **sigstore-verify 0.8.0 → 0.9.0 compatibility**
   - What we know: fork is at 0.8.0, upstream was at 0.9.0 at window base; Cluster F bumps trust-root to 0.9.0
   - What's unclear: whether `cargo update -p sigstore-trust-root` will ALSO pull sigstore-verify 0.9.0 due to transitive deps, and whether the `"tuf"` feature still exists in sigstore-verify 0.9.0
   - Recommendation: executor should run `cargo update -p sigstore-trust-root --dry-run` (if available) or inspect the Cargo.lock diff after `cargo update -p sigstore-trust-root` to check what gets pulled before committing. If `sigstore-verify` also gets bumped by the resolver, inspect the API diff.

2. **`command_policy.rs` and `migration.rs` deletion in fork**
   - What we know: Cluster E's `c808f000` tries to modify these files but they are deleted in the fork HEAD (modify-delete conflict). Upstream's changes are just string replacements.
   - What's unclear: whether there are any remaining references to these files in the fork that would need updating if the string content were still present.
   - Recommendation: mark as N/A for Cluster E — the files don't exist in the fork so there's nothing to update. Verify with `ls crates/nono-cli/src/command_policy.rs 2>/dev/null || echo absent`.

---

## Sources

### Primary (HIGH confidence)

- `git show <sha>` per-cluster actual diff inspection (72bcfd66, d457ecc3, 5b8e94da, c808f000, 2e64798d, a4d68189, cdeeb5b9, 46bcfbb9, 08ca19a8) — all commits reachable locally
- `.planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md` — per-cluster audit findings
- `proj/ADR-98-network-intent-disposition.md` — Cluster A full-sync-adopt decision with non-regression proofs
- `proj/ADR-86-library-boundary-convergence.md` — policy-free boundary + Windows carve-out
- `.planning/templates/cross-target-verify-checklist.md` — cross-target gate invocations
- `git show 1d1c88c9:crates/nono/Cargo.toml` — upstream sigstore-verify version at v0.65.1 baseline
- `git cherry-pick --no-commit <sha>` feasibility tests (empirical, 2026-06-30)
- `grep` against live codebase for guard test function names, WSL2ProxyFallback location, CompiledEndpointPolicy location

### Secondary (MEDIUM confidence)

- `git log --oneline upstream/main -- crates/nono/Cargo.toml` — confirmed sigstore-verify 0.8.0→0.9.0 bump (9e084cbb / PR #1228) predates v0.65.1
- `ls crates/nono-proxy/src/` — tls_intercept/ absence confirmed live

### Tertiary (LOW confidence — see Assumptions Log)

- sigstore-trust-root 0.9.0 / sigstore-verify 0.8.0 Cargo resolver compatibility — [ASSUMED]; Cargo.lock shows dual 0.8.0/0.9.0 versions coexisting in upstream post-bump, but fork's resolver behavior may differ

---

## Metadata

**Confidence breakdown:**
- Cherry-pick feasibility: HIGH — empirically tested against live HEAD
- Cluster A absorption surface (file list, conflict files): HIGH — from actual-diff (git show) + cherry-pick test
- Cluster C split (tls_intercept/ absence, applicable files): HIGH — verified live filesystem + actual-diff
- Cluster D linux.rs conflict type: HIGH — cherry-pick conflict text inspected
- Cluster E src/ conflict files: HIGH — cherry-pick conflict output inspected
- Cluster F sigstore cascade: MEDIUM (sigstore-verify gap VERIFIED; compat of mixing 0.8.0/0.9.0 ASSUMED)
- Guard test names: HIGH — grep against live codebase
- D-02 SC reconciliation map: HIGH — all PRs cross-referenced against ledger per-commit tables
- Cross-target gate invocations: HIGH — from cross-target-verify-checklist.md (authoritative)

**Research date:** 2026-06-30
**Valid until:** 2026-07-30 (stable ecosystem; upstream commits are pinned SHAs, fork HEAD stable)
