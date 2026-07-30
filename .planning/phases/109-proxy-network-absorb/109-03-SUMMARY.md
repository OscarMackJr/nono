---
phase: 109-proxy-network-absorb
plan: 03
subsystem: proxy-network
tags: [no-proxy, ssrf, host-filter, proxy-config, adr-108, fail-closed, profile-schema, cli-plumbing]

# Dependency graph
requires:
  - phase: 109-proxy-network-absorb
    plan: 02
    provides: "ProxyConfig.no_proxy field + the three D-06-named proxy-crate validators (validate_no_proxy_entry, no_proxy_entry_overlaps_host_pattern, bare_single_label_suffix_overlaps_host) this plan's CLI-crate validators call into"
provides:
  - "NetworkConfig.no_proxy profile field + validate_profile_no_proxy, wired into parse_profile_bytes, parse_profile_file, AND finalize_profile (direct + extends-inherited overlap detection)"
  - "no_proxy threaded end-to-end: PreparedProfile -> PreparedSandbox -> EffectiveProxySettings -> ProxyLaunchOptions -> ProxyConfig.no_proxy (profile-only, no CLI flag)"
  - "validate_proxy_launch_no_proxy_conflicts (literal allow_domain overlap) + validate_expanded_proxy_no_proxy_conflicts (group-expanded allow_domain overlap), both wired into build_proxy_config_from_flags"
  - "no_proxy visibility in `nono profile show`/`diff` (text + JSON), mirroring allow_domain/upstream_bypass"
affects: [109-04, 109-05, 112, 113]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Profile-level D-06 overlap check runs at THREE call sites (parse_profile_bytes, parse_profile_file, finalize_profile) so both the direct (same-file) and extends-inherited overlap cases are caught, without needing a fourth merged-profile-only entry point"
    - "CLI-flag-layer D-06 overlap check runs in TWO phases inside build_proxy_config_from_flags: pre-group-expansion (literal allow_domain strings) and post-group-expansion (network_policy::partition_allow_domain's plain_hosts) — closing the gap where a no_proxy entry names a group MEMBER host rather than the group name itself"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/profile_runtime.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/profile_cmd.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - crates/nono-cli/data/profile-authoring-guide.md

key-decisions:
  - "validate_profile_no_proxy is called from THREE sites, not the plan's minimum two: parse_profile_bytes and parse_profile_file catch the direct (same-file) overlap at raw-parse time; finalize_profile catches the extends-inherited case, since it always receives the fully resolve_extends-merged profile (per load_from_file's parse-then-resolve-then-finalize pipeline). A guard placed only at raw-parse time would miss a child's no_proxy overlapping a base's allow_domain, since each file is validated independently before merge."
  - "proxy_config.no_proxy = proxy.no_proxy.clone() was added in build_proxy_config_from_flags even though the plan's must_haves artifacts only name the two validators. Without this assignment, a no_proxy entry that passes both overlap checks would be silently discarded and never reach the running proxy's push_no_proxy_entry pipeline (Plan 109-02) — making the entire feature a stub that validates but never applies. Treated as Rule 2 (auto-add missing critical functionality)."
  - "no_proxy has NO CLI flag (--no-proxy does not exist) — confirmed by the plan's files_modified list omitting cli.rs. Threading is profile-only: PreparedProfile.no_proxy -> PreparedSandbox.no_proxy -> EffectiveProxySettings.no_proxy is a straight clone (no CLI-arg merge step), unlike deny_domain's profile+flag merge."
  - "profile_cmd.rs's `nono profile show`/`diff` gained no_proxy display support (text + JSON, both show and diff) even though the equivalent deny_domain display was never added in 109-01. Scoped narrowly to no_proxy only, per this plan's own files_modified list — deny_domain's missing display is a separate, out-of-scope gap not fixed here."

patterns-established:
  - "D-06: every layer a no_proxy entry can enter from (direct profile, extends-inherited, CLI-flag-merged ProxyLaunchOptions, group-expanded allow_domain) has its own dedicated overlap check, and all four reuse the same nono_proxy::config::no_proxy_entry_overlaps_host_pattern primitive so overlap semantics never drift between layers."

requirements-completed: [NET-03]

# Metrics
duration: ~70min
completed: 2026-07-29
---

# Phase 109 Plan 03: no_proxy CLI-crate absorb (profile schema + D-06 launch/expansion validators) Summary

**Closed the D-06 overlap-detection loop at every remaining layer a `no_proxy` entry can enter from — direct profile, `extends`-inherited base, and `allow_domain` group-name expansion — and wired the validated entries into `ProxyConfig.no_proxy` so they actually reach the running proxy's NO_PROXY pipeline instead of being silently discarded after validation.**

## Performance

- **Duration:** ~70 min
- **Completed:** 2026-07-29
- **Tasks:** 2/2 completed
- **Files modified:** 9 (`profile/mod.rs`, `proxy_runtime.rs`, `launch_runtime.rs`, `main.rs`, `profile_runtime.rs`, `sandbox_prepare.rs`, `profile_cmd.rs`, `nono-profile.schema.json`, `profile-authoring-guide.md`) + this SUMMARY

## Accomplishments

- `NetworkConfig.no_proxy: Vec<String>` profile field, merged via `dedup_append` in `merge_profiles()`, plus `validate_profile_no_proxy` which rejects a `no_proxy` entry overlapping `allow_domain` using Plan 109-02's `no_proxy_entry_overlaps_host_pattern`. Wired into THREE call sites — `parse_profile_bytes`, `parse_profile_file` (catch the direct, same-file overlap), and `finalize_profile` (catches the `extends`-inherited case on the fully-merged profile).
- Both D-06-named upstream regression tests exist verbatim and pass: `test_network_no_proxy_rejects_allow_domain_overlap` and `test_finalize_profile_rejects_inherited_no_proxy_allow_domain_overlap`.
- `no_proxy` threaded profile-only (no `--no-proxy` CLI flag) end-to-end: `PreparedProfile.no_proxy` -> `PreparedSandbox.no_proxy` -> `EffectiveProxySettings.no_proxy` -> `ProxyLaunchOptions.no_proxy`.
- `validate_proxy_launch_no_proxy_conflicts` (rejects overlap against literal, pre-group-expansion `allow_domain` entries) and `validate_expanded_proxy_no_proxy_conflicts` (rejects overlap against the group-expanded host list from `network_policy::partition_allow_domain`) both wired into `build_proxy_config_from_flags`, before and after expansion respectively.
- `test_build_proxy_config_rejects_group_expanded_no_proxy_overlap` proves the group-expansion gap is closed using the real `llm_apis` embedded-network-policy group (expands to `api.openai.com`, among others) — a case a pre-expansion-only check would silently miss.
- `proxy_config.no_proxy = proxy.no_proxy.clone()` wired after both validators pass, so a validated `no_proxy` entry actually reaches the running proxy's `push_no_proxy_entry` pipeline (Plan 109-02) instead of being discarded post-validation.
- `nono profile show`/`diff` (text and JSON output) extended with `no_proxy` visibility, mirroring the existing `allow_domain`/`upstream_bypass` display fields.

## Task Commits

Each task was committed atomically:

1. **Task 1: Profile-level no_proxy field, schema, and validate_profile_no_proxy (D-06)** - `71d24dea` (feat)
2. **Task 2: Launch-time + expanded-group no_proxy conflict validators, end-to-end plumbing** - `909fcb85` (feat)

_No plan-metadata-only commit yet; this SUMMARY + STATE/ROADMAP updates land in the orchestrator's final commit (STATE.md/ROADMAP.md/REQUIREMENTS.md are hand-tracked for this milestone per the execution prompt and are out of scope for this executor)._

## Files Created/Modified

- `crates/nono-cli/src/profile/mod.rs` — `NetworkConfig.no_proxy` field, `merge_profiles` wiring, `validate_profile_no_proxy` + 3 call sites (`parse_profile_bytes`, `parse_profile_file`, `finalize_profile`), 6 new tests including the two D-06-named regression tests.
- `crates/nono-cli/src/proxy_runtime.rs` — `EffectiveProxySettings.no_proxy`, `validate_proxy_launch_no_proxy_conflicts`, `validate_expanded_proxy_no_proxy_conflicts`, `ProxyConfig.no_proxy` wiring in `build_proxy_config_from_flags`, 4 new tests (literal overlap reject, group-expanded overlap reject — the named upstream regression test — non-overlapping propagation, struct-literal completions).
- `crates/nono-cli/src/launch_runtime.rs` — `ProxyLaunchOptions.no_proxy` field.
- `crates/nono-cli/src/profile_runtime.rs` — `PreparedProfile.no_proxy`, sourced from `profile.network.no_proxy`.
- `crates/nono-cli/src/sandbox_prepare.rs` — `PreparedSandbox.no_proxy` (manifest path: always empty since the manifest schema has no `no_proxy` key; profile path: forwarded from `PreparedProfile`).
- `crates/nono-cli/src/main.rs` — struct-literal completeness for `PreparedSandbox`/`EffectiveProxySettings` in 2 existing tests, extended to prove `--allow-net` clears profile `no_proxy` and that profile-only `no_proxy` threads through with no CLI-arg merge.
- `crates/nono-cli/src/profile_cmd.rs` — `no_proxy` added to `has_net` detection, text display, JSON `show` output, text diff, and JSON diff.
- `crates/nono-cli/data/nono-profile.schema.json` — `no_proxy` schema key (verbatim upstream shape from this plan's `<interfaces>` block).
- `crates/nono-cli/data/profile-authoring-guide.md` — `no_proxy` row in the network config table.

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. `validate_profile_no_proxy` runs at 3 call sites (not the plan's stated minimum of 2) because a merged-profile check is structurally required to catch `extends`-inherited overlaps, and `finalize_profile` is the only point in the pipeline that always sees the fully-merged profile.
2. `proxy_config.no_proxy = proxy.no_proxy.clone()` was added even though not explicitly named in the plan's `must_haves.artifacts` — without it the feature validates but never applies, which would be a stub blocking the plan's own `<verification>` goal ("`nono run`/`nono wrap` reject a conflicting `no_proxy` profile at load time, before any proxy activation" implies the accepted case must also *work*, not just not-error).
3. `no_proxy` has no CLI flag — confirmed by the plan's own `files_modified` list omitting `cli.rs`. All threading is profile-only.
4. `profile_cmd.rs` display additions scoped narrowly to `no_proxy` (per this plan's `files_modified` list), not retrofitted onto `deny_domain`'s pre-existing display gap from 109-01 (out of scope here).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `ProxyConfig.no_proxy` was never assigned from `ProxyLaunchOptions.no_proxy`**
- **Found during:** Task 2, while wiring `build_proxy_config_from_flags`
- **Issue:** The plan's `must_haves.artifacts` names only the two validators (`validate_proxy_launch_no_proxy_conflicts`, `validate_expanded_proxy_no_proxy_conflicts`) as `proxy_runtime.rs`'s deliverable. Neither validator, by itself, causes the accepted (non-overlapping) `no_proxy` entries to reach `ProxyConfig` — without an explicit assignment, `proxy_config.no_proxy` stays at its `Default::default()` empty value regardless of what the profile declared, so Plan 109-02's `push_no_proxy_entry`/`NONO_NO_PROXY` pipeline in `server.rs` would never see any profile-declared `no_proxy` entries. The feature would validate-and-discard rather than validate-and-apply.
- **Fix:** Added `proxy_config.no_proxy = proxy.no_proxy.clone();` immediately after the two D-06 validators pass and `ProxyConfig` is constructed.
- **Files modified:** `crates/nono-cli/src/proxy_runtime.rs`
- **Verification:** New test `build_proxy_config_propagates_non_overlapping_no_proxy` asserts `config.no_proxy == vec!["internal-only".to_string()]` after a successful `build_proxy_config_from_flags` call.
- **Commit:** `909fcb85`

---

**Total deviations:** 1 auto-fixed (1 missing-critical-functionality)
**Impact on plan:** The fix was necessary for the absorbed feature to actually function end-to-end, not just validate. No scope creep — no functionality was added beyond what `1619275c`'s CLI-crate half specifies (this is the CLI-side counterpart to Plan 109-02's proxy-crate `no_proxy` pipeline, which already expected `ProxyConfig.no_proxy` to be populated by a caller).

## Issues Encountered

None beyond the deviation above. The plan's `<interfaces>` block flagged that the two `proxy_runtime.rs`-level validator function bodies were "not yet read in this plan's research pass" and needed to be read from the live upstream diff before implementing; no upstream diff access was available in this environment, so both validators were designed fresh against the plan's stated behavior contracts (pre-expansion literal overlap; post-expansion group-member overlap) and Plan 109-02's existing `no_proxy_entry_overlaps_host_pattern` primitive, rather than ported verbatim from an upstream source. Their behavior is proven correct by the three named regression tests plus two additional coverage tests (literal-overlap-reject, non-overlapping-propagation).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The CLI-crate half of `#1415` (`no_proxy` profile schema, `extends` inheritance, group-expansion overlap detection, and end-to-end `ProxyConfig.no_proxy` wiring) is complete and tested. Combined with Plan 109-02's proxy-crate half, `no_proxy` is now a fully validated, fully applied bypass surface at every layer it can be declared from.
- `deny_domain` (Plan 109-01) plumbing is preserved and unmodified: `validate_deny_domain_requires_allow_domain` remains called from both `command_runtime.rs` and `launch_runtime.rs`.
- No blockers for Plan 109-04/109-05.

## Known Stubs

None. `no_proxy` is fully wired end-to-end: profile parse-time validation (direct + inherited) -> CLI-flag-layer validation (literal + group-expanded) -> `ProxyConfig.no_proxy` -> Plan 109-02's `push_no_proxy_entry` pipeline -> generated `NO_PROXY`/`NONO_NO_PROXY` env vars. No placeholder or empty-value stub was introduced.

## Threat Flags

None. The security-relevant surface introduced here (`NetworkConfig.no_proxy`, `validate_profile_no_proxy`, `validate_proxy_launch_no_proxy_conflicts`, `validate_expanded_proxy_no_proxy_conflicts`, `ProxyConfig.no_proxy` wiring) is explicitly covered by the plan's own `<threat_model>` (T-109-08, T-109-09, T-109-SC). No new network endpoints, auth paths, or schema changes at trust boundaries beyond what the threat model already names.

## Verification

- `cargo build --workspace --all-targets` — exits 0.
- `cargo fmt --all -- --check` — clean.
- `cargo test -p nono-sandbox-proxy` — 209 passed, 0 failed (unchanged from Plan 109-02's baseline).
- `cargo test -p nono-sandbox-cli --bin nono` — 1411 passed, 11 failed, 2 ignored (the 11 failures are the documented pre-existing baseline: `audit_session.rs`, `config/mod.rs`, `profile_cmd.rs` init, `protected_paths.rs` — unchanged count from Plan 109-01/02's baseline; this plan does not touch any of those failure sites' logic).
- Named-test targeted runs: `cargo test -p nono-sandbox-cli --bin nono rejects_allow_domain_overlap` (1 passed), `rejects_inherited_no_proxy_allow_domain_overlap` (1 passed), `rejects_group_expanded_no_proxy_overlap` (1 passed).
- Cross-target clippy (mandatory — this plan's Task 2 touches `proxy_runtime.rs`, `launch_runtime.rs`, `sandbox_prepare.rs`, `profile_runtime.rs`, `main.rs`, all of which contain `#[cfg(target_os = "linux"/"macos")]` blocks): `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` GREEN; `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` GREEN (`SDKROOT` unset, direct-binary invocation form).
- Acceptance-criteria greps: `validate_profile_no_proxy` in `profile/mod.rs` (1 definition + 3 call sites); `no_proxy` in `nono-profile.schema.json` (1 hit); `fn validate_proxy_launch_no_proxy_conflicts|fn validate_expanded_proxy_no_proxy_conflicts` in `proxy_runtime.rs` (2 definition hits, each with 1 call site in `build_proxy_config_from_flags`) — all confirmed.
- No `.unwrap()`/`.expect()` introduced in production (non-test) code — confirmed by direct diff review of every non-test hunk across both task commits.

## Self-Check: PASSED

- FOUND: `crates/nono-cli/src/profile/mod.rs` (NetworkConfig.no_proxy field + validate_profile_no_proxy + 3 call sites present)
- FOUND: `crates/nono-cli/src/proxy_runtime.rs` (both validators + ProxyConfig.no_proxy wiring present)
- FOUND: `crates/nono-cli/src/launch_runtime.rs` (ProxyLaunchOptions.no_proxy field present)
- FOUND: `crates/nono-cli/src/profile_runtime.rs` (PreparedProfile.no_proxy present)
- FOUND: `crates/nono-cli/src/sandbox_prepare.rs` (PreparedSandbox.no_proxy present)
- FOUND: `crates/nono-cli/src/profile_cmd.rs` (no_proxy in show/diff)
- FOUND: `crates/nono-cli/data/nono-profile.schema.json` (no_proxy schema key present)
- FOUND commit: `71d24dea` (Task 1)
- FOUND commit: `909fcb85` (Task 2)
- `cargo test -p nono-sandbox-proxy`: 209 passed, 0 failed
- `cargo test -p nono-sandbox-cli --bin nono`: 1411 passed, 11 failed (documented pre-existing baseline), 2 ignored
- `cargo build --workspace --all-targets`: exit 0
- `cargo fmt --all -- --check`: clean
- Cross-target clippy (linux-gnu via `cross`, apple-darwin via `cargo-zigbuild`): both GREEN

---
*Phase: 109-proxy-network-absorb*
*Completed: 2026-07-29*
