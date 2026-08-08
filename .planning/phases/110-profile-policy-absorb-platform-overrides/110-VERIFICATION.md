---
phase: 110-profile-policy-absorb-platform-overrides
verified: 2026-08-08T00:00:00Z
status: passed
score: 4/5 must-haves verified (1 partial)
overrides_applied: 0
gaps:
  - truth: "platform_overrides.windows can only tighten, never silently loosen, security-relevant profile config it patches in"
    status: partial
    reason: >
      merge_profiles's custom_credentials field (crates/nono-cli/src/profile/mod.rs:3683-3687)
      merges the HashMap<String, CustomCredentialDef> with plain .extend() — whole-value replace
      per key, not a field-level merge. apply_platform_overrides (mod.rs:3493-3504) merges the
      matching platform_overrides.<os> block in as an ordinary merge_profiles "child", so this
      same whole-value-replace behavior applies to platform_overrides.windows. A
      platform_overrides.windows block that redefines an existing custom_credentials route name
      WITHOUT its base definition's capture (nono_proxy::config::CaptureConfig) or spiffe
      (nono_proxy::config::SpiffeAuthConfig) field silently drops that field for the route on
      Windows only — the OAuth-capture rewrite or SPIFFE auth stops applying and the real token/
      unauthenticated request reaches the sandboxed agent. validate_profile_custom_credentials
      (mod.rs:1584-1589, invoked post-merge at mod.rs:3098) only checks each merged credential's
      own internal structural validity (e.g. "at least one of credential_key/auth/aws_auth/spiffe/
      capture present") — it does not compare merged output against the base definition, so an
      override entry carrying only credential_key satisfies validation while silently losing
      capture/spiffe. This directly contradicts the phase's own stated design principle at
      mod.rs:2687-2694 (D-08a: "an override may tighten (add) but never silently loosen") and is
      the odd one out among network.* merge fields, all of which are fail-secure-additive:
      block is OR (:3644), deny_domain/no_proxy are dedup_append (:3653-3654). Verified live at
      source on current HEAD; not fixed by any of the four post-phase WFP defect fixes
      (7c7a189c/ea26b5b2/6d7ef719/4aec1944), which are unrelated. Independently confirmed as
      finding NEW-02 (severity warning-security) in .planning/v3.6-MILESTONE-AUDIT.md, tied to
      PROF-01/NET-02/SEC-02 — not yet fixed by any later phase (111-114) and not named as
      in-scope for any of their goals, so it does not qualify as deferred-and-covered.
    artifacts:
      - path: "crates/nono-cli/src/profile/mod.rs"
        issue: "custom_credentials merge (:3683-3687) is whole-value-replace-per-key, reachable through platform_overrides.windows via apply_platform_overrides/merge_profiles"
    missing:
      - "A field-level (or collision-detecting) merge for custom_credentials — e.g. merge each CustomCredentialDef's capture/spiffe/endpoint_policy fields with base-wins-if-child-absent semantics, or reject/warn on a platform_overrides entry that redefines an existing route name without re-declaring every security-relevant field the base declared."
      - "A regression test asserting a platform_overrides.windows block cannot silently drop a base route's capture or spiffe field (the same class of test the D-08a windows_low_il_broker/windows_interpreters OR-semantics proof already has, applied to custom_credentials)."
human_verification: []
---

# Phase 110: Profile/Policy Absorb + platform_overrides Verification Report

**Phase Goal:** The fork gains upstream's per-OS profile-patch model and the v0.67–v0.68
profile/policy features, and retires its top-level `windows_*` flag sprawl into
`platform_overrides.windows`.
**Verified:** 2026-08-08 (retroactive — phase completed 2026-08-04 without a verification gate;
this closes that gap per the v3.6 milestone audit's blocker finding)
**Status:** gaps_found
**Re-verification:** No — initial verification (no prior `110-VERIFICATION.md` existed)

**Note on currency:** this phase's code has been touched by Phases 111-114 since it closed.
Every finding below is checked against current `HEAD` (branch `milestone/v2.13-carryforward-
closeout`), not against the tree as it stood on 2026-08-04. Nothing later reverted or altered
Phase 110's mechanism; the four WFP defect fixes (`7c7a189c`, `ea26b5b2`, `6d7ef719`, `4aec1944`)
that landed after 110-08's own phase-gate certification are present on this branch and were
independently re-verified below (110-08-VERIFICATION-NOTES.md, which recorded the *pre-fix* tree,
is therefore stale on those four points — noted, not treated as current evidence).

## Goal Achievement

### Observable Truths

Sourced from ROADMAP.md's 4 Success Criteria for Phase 110 (`roadmap_truths`), merged with the
per-plan `must_haves.truths` from all 8 PLAN.md frontmatter blocks (no reduction — every plan's
truths are additionally verified individually against the code, itemized in the Requirements
Coverage section below).

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | `platform_overrides` (#1371) absorbed, preserved through `extends` (#1380), `windows_low_il_broker`/`windows_interpreters` migrated into `platform_overrides.windows` with back-compat aliases | ⚠️ VERIFIED WITH CAVEAT | `PlatformOverrides`/`PlatformOverride` types + `apply_platform_overrides`/`merge_platform_overrides`/`merge_platform_override_slot` at `crates/nono-cli/src/profile/mod.rs:2698-3543`; wired as first statement of `finalize_profile` (`:3090`); `merge_platform_overrides` deep-merges (not nulls) during `extends` resolution (`:3548-3554`), closing the #1380 gap. 17/17 `platform_overrides_tests` pass live (see below), including the D-08a OR-semantics distinguishing test (`platform_overrides_windows_low_il_broker_or_semantics_top_level_true_override_false_stays_true`) and both back-compat tests. **Caveat: see Gap below** — the same merge path this SC relies on silently loosens `custom_credentials` on collision, contradicting the phase's own stated "tighten, never loosen" invariant for override merges. |
| 2 | `$VAR` (#1296) and `@git:*` (#1298) token expansion works in profile filesystem paths | ✓ VERIFIED (Windows scope note) | `crates/nono-cli/src/policy.rs::expand_env_vars` + `capability_ext.rs::expand_profile_path` wired at all 8 upstream-identified fs.* call sites (`allow/read/write/allow_file/read_file/write_file/deny(add_deny_access)/bypass_protection`, confirmed by grep). `dynamic_tokens.rs` (1145 lines) carries the ported `expand_dynamic_tokens` + git-scope-restricted dispatch, with `git_read_paths_excludes_per_repo_local_config_overrides` present and passing. On Windows, `@git:*` expansion is an intentional no-op passthrough (`capability_ext.rs:24-27`, documented D-01) matching upstream's own non-Unix fallback — this is a deliberate, documented scope limit (not a hidden gap): the literal token string passes through unexpanded and fails safe (a nonexistent path) rather than granting anything unintended. 82/82 targeted `capability_ext`/`dynamic_tokens` tests pass live. |
| 3 | Port-range profile schema (#1398) absorbed with WFP-native remote-port-range emitter on Windows and discrete-`Vec<u16>` back-compat | ✓ VERIFIED | `CapabilitySet::localhost_port_ranges`/`merge_port_ranges`/`MACOS_PORT_RANGE_LIMIT` at `crates/nono/src/capability.rs`; macOS unrolls with the 16,384 cumulative cap (`sandbox/macos.rs`); Linux unrolls per-port via `NetPort::new` with no cap (`sandbox/linux.rs`); Windows emits native `FWP_MATCH_RANGE` — one filter per range per layer, never unrolled (`nono-wfp-service.rs:1280-1360`, `1576-1600`). Profile schema (`network.open_port_range`/`listen_port_range`) and manifest schema (`PortConfig.localhost_range`) both present and wired independently. **Live-kernel proof PASSED 2026-08-04**: 8 filters installed from a verified-zero baseline, exactly 4 `FWP_MATCH_RANGE` conditions at the correct bounds, torn down to 0 on exit (`110-06-PROF-03e-VERDICT.md`). The 4 defects the live checkpoint surfaced (`has_port_rules` omission, `profile show` Network-section omission, version-skew fail-open, filter-sweep enumeration bug) are all fixed on current HEAD (`7c7a189c`/`ea26b5b2`/`6d7ef719`/`4aec1944`, confirmed present on this branch). Discrete-port back-compat: `test_tcp_connect_ports`/`test_tcp_bind_ports`/`compile_network_policy_with_localhost_ports_only_is_fully_supported` all pass unmodified. Behavioral inside/outside connect probe explicitly NOT run (`0xC0000142`, AppContainer + minimal profile constraint) — closure was accepted on the filter-table proof per the checkpoint's own stated acceptance criterion, not silently skipped. |
| 4 | `bun` (#1305) and `mise` (#1387) runtime presets present and resolvable | ✓ VERIFIED | `bun_runtime`/`mise_manager` groups and `bun-dev`/`mise-dev` profiles in `crates/nono-cli/data/policy.json:428-436,1084-1119`. Dedicated by-name resolution tests (D-11, distinct from schema-shape validation) at `profile/mod.rs`: `bun_dev_builtin_profile_resolves_and_carries_bun_runtime_group`, `mise_dev_builtin_profile_resolves_and_carries_mise_manager_group` — both pass live, confirming group membership, not merely JSON presence. |

**Score:** 4/5 (SC1 marked VERIFIED WITH CAVEAT rather than a clean pass — see Gap; the other
3 SCs are unqualified passes)

### Known Residual Defect Weighed Into the PROF-01 Verdict

The task brief asked this verifier to specifically weigh a known finding rather than rediscover
it. It was independently re-verified at source (not merely trusted from the brief):

- **`crates/nono-cli/src/profile/mod.rs:3683-3687`** — `custom_credentials` merges via
  `HashMap::extend()`, which is whole-value replacement per key, not a field-level merge.
- **`apply_platform_overrides` (`:3493-3504`)** feeds the current-OS `platform_overrides` block
  into `merge_profiles` as an ordinary "child" — so this replacement semantics is exactly what a
  `platform_overrides.windows` block gets for `custom_credentials`.
- **Consequence, confirmed by reading `CustomCredentialDef`'s field list** (`mod.rs:939-1050`,
  includes `capture: Option<nono_proxy::config::CaptureConfig>` and
  `spiffe: Option<nono_proxy::config::SpiffeAuthConfig>`): a `platform_overrides.windows` block
  that redeclares an existing route name without re-specifying `capture`/`spiffe` silently
  removes those fields for that route on Windows, while the same route keeps its full protection
  on macOS/Linux (or on Windows without the override).
- **Post-merge validation does not catch this.** `validate_profile_custom_credentials` /
  `validate_custom_credential` only checks structural self-consistency of the merged result
  (e.g., "credential_key and auth are mutually exclusive," "at least one of the injection
  mechanisms is present") — it has no cross-check against the base definition, so a
  minimal override (`credential_key` only) passes validation while dropping capture/spiffe.
- **This is the "odd one out."** Every other field in `NetworkConfig`'s merge (`block`, OR;
  `deny_domain`/`no_proxy`, dedup-append; `credentials` (the OAuth2-list field, distinct from
  `custom_credentials`), explicit-empty-vs-merge with a code comment explaining the semantics)
  is deliberately fail-secure-additive. `custom_credentials` is the exception, and it is the
  exception in exactly the field that carries this milestone's own OAuth-capture (Phase 114,
  SEC-02) and SPIFFE (Phase 113, NET-02) security features.
- **Contradicts the phase's own documented invariant.** The code comment directly above
  `PlatformOverrides` (`mod.rs:2687-2694`) states the general design intent: "an override may
  tighten (add) but never silently loosen." That comment's literal scope is the two boolean
  flags (`windows_low_il_broker`/`windows_interpreters`), which DO correctly keep OR/union
  semantics and DO have a dedicated distinguishing test. But the invariant it states as the
  *reason* those two fields get special handling — "never silently loosen" — is violated by an
  adjacent field reached through the exact same merge machinery, and nothing in the phase's test
  suite (`platform_overrides_tests`, 17 tests) exercises a `custom_credentials` collision case.

**Verdict on PROF-01:** the phase's literal, plan-level must-haves (parsing, extends-survival,
back-compat, and the two named flags' OR-semantics) are all satisfied and test-proven. The
requirement is **not therefore FAILED outright** — the core per-OS profile-patch mechanism works
exactly as designed and specified. But PROF-01 is **not a clean PASS** either: it delivers a
mechanism whose own stated fail-secure principle has a real, live, unaddressed hole one field
over, in a security-sensitive area this same milestone built (SPIFFE/OAuth-capture). This is
classified as a **gap (WARNING severity)**, not a BLOCKER, because (a) it does not undermine the
literal deliverable ROADMAP SC1 asks for, (b) it requires an operator/profile-author to already
be using `platform_overrides.windows` to redefine an existing route name in a specific way, not a
default-configuration exposure, and (c) it is already independently tracked as finding NEW-02 in
`.planning/v3.6-MILESTONE-AUDIT.md`. It is listed here formally, with fix guidance, rather than
left to be rediscovered a third time.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/nono-cli/src/profile/mod.rs` — `PlatformOverrides`/`PlatformOverride`/`apply_platform_overrides`/`merge_platform_overrides` | PROF-01 mechanism | ✓ VERIFIED | Present, >200 lines, 17 live-passing tests |
| `crates/nono-cli/data/nono-profile.schema.json` — `platform_overrides` property | PROF-01 schema | ✓ VERIFIED | Present at line 131, `additionalProperties: true` per-slot documented rationale |
| `crates/nono-cli/src/dynamic_tokens.rs` | PROF-02 ported `expand_dynamic_tokens` | ✓ VERIFIED | 1145 lines, ~50 tests, all pass; cfg-scoped dead_code allow for the non-Unix arm is documented, not silent |
| `crates/nono-cli/src/policy.rs` — `substitute_vars`/`expand_env_vars` | PROF-02 `$VAR` engine | ✓ VERIFIED | Wired into `capability_ext.rs::expand_profile_path` |
| `crates/nono/src/capability.rs` — `localhost_port_ranges`/`merge_port_ranges`/`MACOS_PORT_RANGE_LIMIT` | PROF-03 mechanism | ✓ VERIFIED | Private field + builder/accessor surface, no policy leak (ADR-86 confirmed unregressed) |
| `crates/nono/src/sandbox/macos.rs`, `linux.rs`, `windows.rs` | PROF-03 3-platform emitters | ✓ VERIFIED | macOS: per-port unroll + 16,384 cap. Linux: per-port unroll, no cap. Windows: native `FWP_MATCH_RANGE`, no unroll. All three reachable via `compile_network_policy`/`generate_profile`/Landlock ruleset build paths. |
| `crates/nono-cli/src/bin/nono-wfp-service.rs` — `PortCondition::RemoteRange`/`LocalRange` | PROF-03d Windows spec builder | ✓ VERIFIED | 25/25 unit tests pass; live-kernel proof independently confirms the same behavior at the `FwpmFilterAdd0` boundary |
| `crates/nono-cli/data/policy.json` — `bun_runtime`/`mise_manager`/`bun-dev`/`mise-dev` | PROF-04 | ✓ VERIFIED | Present, schema-valid, and resolvable by name (dedicated tests) |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `finalize_profile` | `apply_platform_overrides` | first statement, before `merge_implicit_default_groups` | ✓ WIRED | `mod.rs:3090` |
| `merge_profiles` | `merge_platform_overrides` | struct-literal field | ✓ WIRED | `mod.rs:3551-3554` |
| `capability_ext.rs` fs.* loops (8 sites) | `dynamic_tokens::expand_dynamic_tokens` | pre-`expand_profile_path` call | ✓ WIRED | Confirmed at all 8 call sites by grep |
| `expand_profile_path` | `policy::expand_env_vars` | direct call | ✓ WIRED | `capability_ext.rs:37` |
| `sandbox/macos.rs::generate_profile` | `capability.rs::merge_port_ranges` | called before cumulative-limit check | ✓ WIRED | `macos.rs:513` |
| `sandbox/linux.rs` | `landlock::NetPort::new` per merged range | unrolled loop | ✓ WIRED | `linux.rs:990-1005` |
| `sandbox/windows.rs::compile_network_policy` | `capability.rs::merge_port_ranges` | direct call | ✓ WIRED | `windows.rs:351` |
| `exec_strategy_windows/network.rs::build_wfp_runtime_activation_request` | `windows_wfp_contract.rs::WfpRuntimeActivationRequest.localhost_port_ranges` | field assignment | ✓ WIRED | `network.rs:533` |
| `nono-wfp-service.rs::add_policy_filter` | `FWP_MATCH_RANGE`/`FWP_RANGE0` | condition construction | ✓ WIRED | `nono-wfp-service.rs:1576-1600`; live-kernel-confirmed |
| `capability_ext.rs::CapabilitySet::from_profile` | `capability.rs::add_localhost_port_range` | profile-pathway loop | ✓ WIRED | Confirmed present |
| `manifest_convert.rs` | `capability.rs::allow_localhost_port_range` | manifest-pathway loop, independent validation | ✓ WIRED | `manifest_convert.rs:83-112` |
| `policy.json` `bun-dev`/`mise-dev` profiles | `bun_runtime`/`mise_manager` groups | `security.groups` array | ✓ WIRED | Resolution tests pass |
| `apply_platform_overrides`/`merge_profiles` | `custom_credentials` collision safety | HashMap `.extend()` | ✗ NOT_WIRED (fail-secure) | See Gap — whole-value replace, not field-merge; silently drops `capture`/`spiffe` on a redefinition collision |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| `platform_overrides` full test suite | `cargo test -p nono-sandbox-cli --bin nono -- platform_overrides_tests --test-threads=1` | 17 passed, 0 failed | ✓ PASS |
| `$VAR`/`@git:*`/dynamic-token suite | `cargo test -p nono-sandbox-cli --bin nono -- bun_dev_builtin mise_dev_builtin dynamic_tokens capability_ext --test-threads=1` | 82 passed, 0 failed | ✓ PASS |
| WFP port-range spec-builder suite | `cargo test -p nono-sandbox-cli --bin nono-wfp-service -- --test-threads=1` | 25 passed, 0 failed | ✓ PASS |
| `merge_port_ranges` + discrete-port back-compat (nono lib) | `cargo test -p nono-sandbox --lib -- merge_port_ranges test_tcp_connect_ports test_tcp_bind_ports --test-threads=1` | 9 passed, 0 failed | ✓ PASS |
| `cargo fmt --all -- --check` | (native Windows host) | exit 0, no output | ✓ PASS |
| macOS/Linux cfg-gated emitter tests (`sandbox::macos`, `sandbox::linux`, `supervisor_linux`) | not runnable on this Windows verification host | N/A | ? SKIP — relies on the recorded `cross`/`cargo-zigbuild` evidence in `110-08-VERIFICATION-NOTES.md` (both gates GREEN, and Linux port-range tests explicitly enumerated as passing via `cross test`) |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes are declared by this phase's PLAN/SUMMARY files, and none
exist under that convention for this phase's surface. Skipped — N/A, not a migration/CLI-tooling
phase in the probe-driven sense.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| PROF-01 | 110-01 | `platform_overrides` per-OS patching + `extends` preservation + flag migration/back-compat | ⚠️ SATISFIED WITH GAP | Mechanism verified live; `custom_credentials` collision-loosening gap documented above |
| PROF-02 | 110-02 | `$VAR` + `@git:*` token expansion | ✓ SATISFIED | Verified live, 82/82 tests; Windows `@git:*` no-op is a documented, deliberate, fail-safe scope limit |
| PROF-03 | 110-03, 110-04, 110-05, 110-06 | Port-range schema, mechanism, 3-platform emitters, WFP-native Windows emitter | ✓ SATISFIED | Verified live at every layer including a live-kernel `FwpmFilterAdd0` proof; all 4 post-checkpoint defects fixed on current HEAD |
| PROF-04 | 110-07 | `bun`/`mise` runtime presets | ✓ SATISFIED | Verified live, resolvable-by-name proof present |

No orphaned requirements: REQUIREMENTS.md's traceability table maps exactly PROF-01..04 to Phase
110, and all four appear in at least one plan's `requirements:` frontmatter field.

### Anti-Patterns Found

None. Scanned every file this phase's 8 plans modified (`profile/mod.rs`, `capability_ext.rs`,
`dynamic_tokens.rs`, `policy.rs`, `capability.rs`, `sandbox/{macos,linux,windows}.rs`,
`exec_strategy_windows/network.rs`, `windows_wfp_contract.rs`, `bin/nono-wfp-service.rs`,
`profile_runtime.rs`, `profile_cmd.rs`, `output.rs`, `manifest_convert.rs`, `policy.json`) for
`TBD`/`FIXME`/`XXX` (debt-marker gate — zero hits) and `TODO`/`HACK`/`PLACEHOLDER` (zero hits).
The `custom_credentials` finding above is a design/logic gap, not a code-smell marker — it was
found by reading the merge semantics, not by grepping for a marker comment.

### Human Verification Required

None outstanding. The one item that would have belonged here — PROF-03e's live-kernel WFP proof
— was already run by the operator on an Administrator-elevated session during Phase 110's own
execution (`110-06-PROF-03e-VERDICT.md`), with its accepted scope limitation (behavioral
inside/outside connect probe not run, filter-table proof accepted instead) already recorded and
already the closure basis ROADMAP.md cites. Nothing here requires a fresh human pass.

### Gaps Summary

Phase 110 delivers everything its four Success Criteria literally ask for, and every one of the
25+ automated tests this verifier ran live (platform_overrides, dynamic-token, WFP spec-builder,
capability/back-compat) passes on current HEAD. The Windows WFP-native port-range emitter — the
phase's largest and most novel piece of work — has a genuine live-kernel proof, not just unit
tests, and all four defects that live-kernel run surfaced are fixed and present on this branch.

The one open gap is narrower and more specific than a missing feature: the `platform_overrides`
merge machinery this phase built reuses `merge_profiles` for its `custom_credentials` field
exactly as designed (D-08's "one function, no new precedence code" simplicity goal), but
`custom_credentials`'s existing merge semantics (present since before this phase, `HashMap::
extend()`) is whole-value-replace, not the fail-secure additive semantics the phase's own D-08a
decision explicitly calls out as the general principle. Because `platform_overrides.windows` is
new surface this phase adds, and because `custom_credentials` is also where this milestone's
SPIFFE (Phase 113) and OAuth-capture (Phase 114) security features live, this is a real,
narrow, security-relevant hole — not a theoretical one — and it was missed by all 8 plans' test
suites, none of which exercise a `custom_credentials` collision under `platform_overrides`.

This does not block the phase's core deliverable and is classified WARNING, not BLOCKER, per the
same severity the independent v3.6 milestone audit assigned it (finding NEW-02). It is
recommended as a follow-up fix (field-level merge or collision-rejection for `custom_credentials`,
plus a regression test mirroring the existing OR-semantics proof) rather than a phase re-open.

---

*Verified: 2026-08-08*
*Verifier: Claude (gsd-verifier)*

---

## Gap Resolved 2026-08-08 — NEW-02 fixed

The single gap that held this verification at `gaps_found` is closed in commit `8b9fd74c`.

`merge_profiles` now merges colliding `custom_credentials` keys **field-by-field** via a new
`merge_custom_credential_def()`: every `Option` field falls back to the base when the child omits
it, and `endpoint_rules` inherits when the child states none. A `platform_overrides.<os>` block
can still change any field by stating it explicitly — it can no longer remove one by silence.
That restores the D-08a invariant this phase documented at `profile/mod.rs:2687-2694` but did not
hold.

Regression test `platform_overrides_custom_credential_collision_cannot_drop_capture_or_spiffe`
was verified **load-bearing and precisely scoped**: reverting to the old bare
`merged.extend(child…)` fails exactly that test while the other 17 `platform_overrides` tests
still pass, so pre-existing D-08a semantics are undisturbed. It also asserts the override's own
stated change still applies, so the fix cannot turn overrides into no-ops.

Gates after fix: build exit 0, `cargo fmt --all --check` clean, native clippy exit 0,
apple-darwin `cargo-zigbuild clippy` exit 0, `platform_overrides_` 18/18, `profile::` 325/325,
`nono-sandbox-proxy --lib` 301/301.

**PROF-01 is now fully satisfied; status raised to `passed`.**
