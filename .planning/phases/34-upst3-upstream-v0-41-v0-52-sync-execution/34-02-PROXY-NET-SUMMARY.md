---
phase: 34-upst3-upstream-v0-41-v0-52-sync-execution
plan: 34-02
slug: proxy-net
cluster_id: C4
type: execute
wave: 2
status: complete
date: 2026-05-12
commits:
  - 02626ebe   # fix(proxy): NO_PROXY hole closure (ad23d794, v0.42.0)
  - 108d1139   # feat(cli): --allow-connect-port (8c818f84, v0.43.0)
  - d2447525   # fix(cli): macOS fail-fast on --allow-connect-port (cba186f4, v0.43.0)
  - fd8ac66a   # feat(packages): native TLS root certificates (cb6b199c, v0.45.0)
  - 9e47c675   # style(34-02): rustfmt fixup for cherry-picked capability_ext.rs
upstream_commits_landed: 4
final_main_sha: 9e47c675f5898425feb4dc010c6c4804b9125757
base_sha: f32898b56f58287d5d3a32c2759d0daa0cd98162
dependency-graph:
  requires: [34-04, 34-04b, 34-01]
  provides: [C4-proxy-net-hardening, allow-connect-port-flag-proxy-only, native-tls-roots]
  affects: [crates/nono-proxy/, crates/nono-cli/src/capability_ext.rs, crates/nono-cli/src/cli.rs, crates/nono-cli/src/profile/mod.rs, crates/nono-cli/src/proxy_runtime.rs, crates/nono-cli/src/registry_client.rs]
  blocks: [34-10]
tech-stack:
  added:
    - ureq feature flag platform-verifier (delegate TLS trust to OS)
    - rustls-platform-verifier v0.7.0 (transitive, package-registry TLS path)
  patterns:
    - direct_connect_ports gate on NO_PROXY env-var population
    - --allow-connect-port repeatable clap flag; macOS fail-fast guard
    - native TLS roots replacing bundled webpki for registry handshakes
key-files:
  created: []
  modified:
    - crates/nono-cli/Cargo.toml             # ureq platform-verifier feature
    - crates/nono-cli/src/capability_ext.rs  # add_tcp_connect_port + macOS fail-fast guard
    - crates/nono-cli/src/cli.rs             # --allow-connect-port clap flag
    - crates/nono-cli/src/profile/mod.rs     # NetworkConfig::connect_port profile field
    - crates/nono-cli/src/proxy_runtime.rs   # populate direct_connect_ports from caps
    - crates/nono-cli/src/registry_client.rs # ureq tls_config with PlatformVerifier
    - crates/nono-proxy/src/config.rs        # ProxyConfig::direct_connect_ports field
    - crates/nono-proxy/src/server.rs        # NO_PROXY gate by direct_connect_ports + 2 new tests
    - Cargo.lock                             # regenerated for platform-verifier feature
decisions:
  - "D-34-B2 surgical posture preserved: --allow-connect-port flows through nono-proxy ONLY. Zero WFP retrofit; exec_strategy_windows/ last-touched SHA unchanged from baseline (2823ec29)."
  - "Cluster C3 (won't-sync) is the basis for resolving the ad23d794 capability_ext.rs conflict — drop upstream's warn!->info! adjustment in try_new_unix_socket_{file,dir} (those helpers don't exist in the fork)."
  - "cb6b199c registry_client.rs tests conflict resolved by keeping the fork's full Plan 26-02 streaming/size-cap suite and APPENDING upstream's registry_client_normalizes_base_url smoke test inside the same module."
  - "Cargo.lock regenerated from resolved Cargo.toml (clean build with platform-verifier feature on ureq v3)."
metrics:
  duration_minutes: ~25
  task_count: 4
  file_count: 9
  commits_count: 5  # 4 cherry-picks + 1 rustfmt fixup
tags: [upst3, c4, proxy, network, allow-connect-port, no-proxy, native-tls, wave-2, surgical-posture]
---

# Phase 34 Plan 34-02 (C4): Proxy/Network Hardening + --allow-connect-port Summary

Absorbed cluster C4 (4 upstream commits, v0.42.0..v0.45.0) covering three behavioral fixes — NO_PROXY hole closure, `--allow-connect-port` outbound TCP port allowlisting, native TLS roots for package downloads — plus a macOS fail-fast guard. D-34-B2 surgical posture preserved: `--allow-connect-port` is proxy-only, NO Phase 09 WFP retrofit.

## Commits Landed

| # | Order | SHA (fork) | Upstream SHA | Tag | Subject |
|---|-------|------------|--------------|-----|---------|
| 1 | 1/4 | `02626ebe` | `ad23d794` | v0.42.0 | fix(proxy): stop adding allow_domain hosts to NO_PROXY without direct TCP grants |
| 2 | 2/4 | `108d1139` | `8c818f84` | v0.43.0 | feat(cli): add --allow-connect-port for outbound TCP port allowlisting |
| 3 | 3/4 | `d2447525` | `cba186f4` | v0.43.0 | fix(cli): fail fast on --allow-connect-port on macOS |
| 4 | 4/4 | `fd8ac66a` | `cb6b199c` | v0.45.0 | feat(packages): use native tls root certificates |
| 5 | fmt | `9e47c675` | — (fork-side) | — | style(34-02): rustfmt fixup for cherry-picked capability_ext.rs |

## D-34-D2 Close-Gate Results

| # | Gate | Status | Detail |
|---|------|--------|--------|
| 1 | `cargo test --workspace --all-features` (Windows) | **PASS** | 918/919 tests pass. Single failure = `query_ext::test_query_path_denied` carry-forward (P34-DEFER-01-1, Windows UNC long-path canonicalization quirk — pre-existing, NOT introduced by this plan). `nono-proxy` 148/148 including new NO_PROXY tests. |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) | **PASS** | Clean. |
| 3 | Linux cross-target clippy | **DOCUMENT-SKIPPED** | Linux linker not installed on dev host; deferred to CI per user-accepted posture from Plan 34-04 close. |
| 4 | macOS cross-target clippy | **DOCUMENT-SKIPPED** | macOS linker not installed on dev host; deferred to CI per user-accepted posture from Plan 34-04 close. |
| 5 | `cargo fmt --all -- --check` | **PASS** (after fixup commit `9e47c675`) | Cherry-pick `cba186f4` introduced a line-wrap fmt drift; landed as standalone `style:` commit. |
| 6 | Phase 15 detached-console 5-row smoke | **ADMIN-SKIPPED** | Per executor prompt. |
| 7 | `wfp_port_integration` test suite | **ADMIN-SKIPPED** | Per executor prompt. |
| 8 | `learn_windows_integration` test suite | **ADMIN-SKIPPED** | Per executor prompt. |

**Mandatory gates (1, 2, 5):** ALL PASS. Plan close-gate cleared.

## D-19 Trailer Compliance

All 4 cherry-pick commits carry the verbatim 6-line trailer block (lowercase 'a' in `Upstream-author:`):

```
Upstream-commit: <8-char SHA>
Upstream-tag: <vX.Y.Z>
Upstream-author: <upstream name> <email>
Co-Authored-By: <upstream name> <email>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
```

Smoke check: `git log --format='%B' main~5..main | grep -c '^Upstream-commit: '` returns **4** (matches landed cherry-pick count). 4 `Upstream-author:` lines (all lowercase 'a'); 4 `Co-Authored-By:`; 8 `Signed-off-by:` (2 per cherry-pick commit). The `style:` fixup commit `9e47c675` carries DCO sign-off but no D-19 trailer (it is not a cherry-pick — it is a fork-side rustfmt adjustment of work landed by the cherry-pick chain).

## D-34-E1 Windows-Only Files Invariant

Per-commit `git diff --stat HEAD~1 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l`:

| Commit | Result | Verdict |
|--------|--------|---------|
| `02626ebe` (cherry-pick 1) | 0 | PASS |
| `108d1139` (cherry-pick 2) | 0 | PASS |
| `d2447525` (cherry-pick 3) | 0 | PASS |
| `fd8ac66a` (cherry-pick 4) | 0 | PASS |
| `9e47c675` (fmt fixup)     | 0 | PASS |

Cumulative `git diff --stat HEAD~5 HEAD` shows **zero** changes inside `*_windows.rs` files or `crates/nono-cli/src/exec_strategy_windows/`.

`exec_strategy_windows/` last-touched SHA: `2823ec29f29dc7d310f938f72688af60507ec37d` — **unchanged** from Plan 34-02 Task 1 baseline (captured before any cherry-pick). D-34-B2 surgical posture verified.

## D-34-B2 Surgical Posture Verification

The load-bearing C4 invariant: `--allow-connect-port` flows through `nono-proxy` ONLY; NO fork-side wiring into Phase 09 WFP port-level filter.

| Check | Command | Result |
|-------|---------|--------|
| No WFP wiring of allow_connect_port | `grep -rc 'allow_connect_port.*wfp\|wfp.*allow_connect_port' crates/nono-cli/src/` | 0 matches (clean) |
| Flag exists in clap surface | `grep -c 'allow_connect_port' crates/nono-cli/src/cli.rs` | 5 (clap flag + handler wiring) |
| Proxy layer carries enforcement | `grep -rc 'direct_connect_ports' crates/nono-proxy/` | 10 (config.rs + server.rs + tests) |
| Phase 09 WFP allowlist untouched | `git diff main~5 main -- crates/nono-cli/src/exec_strategy_windows/` | 0 lines |

`nono run --help` confirms the flag appears under `--allow-connect-port <PORT>` with description `"Allow outbound TCP connect to a specific port (repeatable; Linux Landlock V4+ only)"`. Behavior is proxy-only on Windows; Phase 09 `--allow-port` remains the kernel-enforced parallel layer.

## Fork-Defense Baselines (preserved)

| Pattern | File | Pre-plan | Post-plan | Status |
|---------|------|----------|-----------|--------|
| `never_grant\|apply_deny_overrides` | `crates/nono-cli/src/policy.rs` | 21 | **21** | Preserved |
| `validate_path_within` | `crates/nono-cli/src/package_cmd.rs` | 9 | **9** | Preserved |
| `capabilities.aipc\|loaded_profile` | `crates/nono-cli/src/profile/mod.rs` | 17 | **17** | Preserved |
| `find_denied_user_grants` | `crates/nono-cli/src/policy.rs` | 7 | **7** | Preserved |
| `bypass_protection` | `crates/nono-cli/src/profile/mod.rs` | 17 | **17** | Preserved |

Phase 22-04 OAuth2 + WSAStartup ordering in `crates/nono-proxy/src/server.rs` preserved (server.rs touched only inside the NO_PROXY computation block at line ~261; boot sequence at top of `start()` unchanged).

## Conflict Resolution Detail

### Cherry-pick 1 (`ad23d794`) — capability_ext.rs

**Conflict:** Upstream commit's side-fix `warn! -> info!` adjustment lives inside `try_new_unix_socket_{file,dir}` helpers — both of which were added by cluster C3 (`85708ca`, "feat(cli): add --allow-unix-socket flag family + profile schema", v0.42.0). C3 is dispositioned `won't-sync` per Phase 33 DIVERGENCE-LEDGER.md (UnixSocketCapability is Unix-only by construction; no Windows analog).

**Resolution:** Drop the entire conflicted region — keep HEAD (fork has no Unix-socket helpers to adjust). The core fix (NO_PROXY hole in `nono-proxy/src/server.rs` + `direct_connect_ports` field in `config.rs` + `proxy_runtime.rs` plumbing through `caps.tcp_connect_ports()`) applied cleanly. The fork's `CapabilitySet::tcp_connect_ports()` method already existed from a prior upstream sync, so commit 1's reference to it compiles before commit 2 lands.

Commit body documents the resolution choice.

### Cherry-pick 4 (`cb6b199c`) — registry_client.rs + Cargo.lock

**Conflict 1 (registry_client.rs):** Upstream adds a small `mod tests { fn registry_client_normalizes_base_url() }` block in the same location where the fork has the extensive Plan 26-02 streaming/size-cap REQ-PKGS-01/04 test suite (~290 lines).

**Resolution:** Keep the fork's full test module AND append upstream's `registry_client_normalizes_base_url` as an additional test inside the same module (with a leading comment crediting upstream `cb6b199c`). No fork test coverage lost; upstream's smoke test absorbed.

**Conflict 2 (Cargo.lock):** Multiple lockfile chunks (transitive dependency rewiring from the `ureq platform-verifier` feature flag).

**Resolution:** Took `--theirs` Cargo.lock (upstream's), then ran `cargo build --workspace` to regenerate from the resolved `Cargo.toml`. Result: clean lockfile with `platform-verifier` feature on ureq v3 + transitive `rustls-platform-verifier v0.7.0` (new) and `rustls-platform-verifier v0.6.2` (existing for nono-proxy path). No version-pin regressions on direct deps.

Commit body documents both resolutions.

### Cherry-picks 2 + 3 — clean

Both `8c818f84` and `cba186f4` applied without conflicts. Auto-merge in `capability_ext.rs`, `cli.rs`, and `profile/mod.rs` produced valid trees.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] rustfmt drift from cherry-pick 3**
- **Found during:** D-34-D2 Gate 5 (`cargo fmt --all -- --check`)
- **Issue:** Cherry-pick `cba186f4` brought in a line-wrap form that rustfmt rewrote to fit the fork's column budget (test function arrow-pattern in `capability_ext.rs::tests`).
- **Fix:** Ran `cargo fmt --all`; landed as a standalone `style:` commit (`9e47c675`) preserving cherry-pick atomicity. No behavior change; D-19 trailer not added because this is not itself a cherry-pick.
- **Files modified:** `crates/nono-cli/src/capability_ext.rs` (2 lines).
- **Commit:** `9e47c675`.

### Auth Gates

None — fully autonomous execution.

## Known Stubs

None.

## Threat Flags

No new security-relevant surface beyond what's in the plan's `<threat_model>`. The plan's STRIDE register (T-34-02-01 through T-34-02-07) covers all introduced surface:

- T-34-02-01 (Windows-only files invariant): mitigated — 0 hits per commit, 0 cumulative.
- T-34-02-02 (D-19 trailer missing): mitigated — 4/4 cherry-picks carry the trailer; lowercase 'a' verified.
- T-34-02-03 (WFP retrofit accident): mitigated — exec_strategy_windows/ untouched; grep for `allow_connect_port.*wfp` returns 0.
- T-34-02-04 (NO_PROXY hole reopens via malformed cherry-pick): mitigated — nono-proxy 148/148 tests pass including the 2 new NO_PROXY behavior tests added by `ad23d794` (`test_no_proxy_empty_without_direct_connect_ports`, `test_no_proxy_includes_hosts_with_matching_connect_port`).
- T-34-02-05 (compromised CA in OS trust-store): accepted per threat register — out of nono's scope.
- T-34-02-06 (native-TLS DoS on stripped Windows installs): low-severity mitigated — fork's MSI installer ships with explicit TLS-root config; users on stripped installs can override.
- T-34-02-07 (port number bounds): mitigated by clap u16 bounded type.

## Carry-Forward Deferrals

- **P34-DEFER-01-1** (test_query_path_denied Windows UNC long-path flake) — explicit carry-forward per executor prompt; pre-existing, NOT introduced by Plan 34-02.

No new deferrals from Plan 34-02.

## Push + PR

Per executor prompt: **push MANDATORY at plan close**. Performed after SUMMARY commit; PR creation per D-34-D1 happens in narration step (not part of the close-gate).

## Self-Check: PASSED

- All 5 commits present in `git log` (`02626ebe`, `108d1139`, `d2447525`, `fd8ac66a`, `9e47c675`).
- SUMMARY file exists at the documented path.
- All 8 key modified files exist on disk post-plan.
- D-19 trailer count = 4 (matches landed cherry-pick count).
- D-34-E1 cumulative invariant = 0 (no Windows-only file edits).
- D-34-B2 surgical posture: exec_strategy_windows last-touched SHA unchanged from baseline (`2823ec29`).
- Fork-defense baselines preserved (21/9/17/7/17).
- Mandatory close-gates (1/2/5) PASS; gates 3/4 deferred-to-CI; gates 6/7/8 admin-skipped.
