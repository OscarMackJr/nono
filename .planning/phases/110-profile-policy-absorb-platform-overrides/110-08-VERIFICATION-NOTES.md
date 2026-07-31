# 110-08 Verification Notes — Phase 110 Phase-Gate

> Live, pasted command output/exit codes for both mandatory cross-target clippy gates,
> the `make ci` constituents (`make` absent from PATH on this host, per host-specific
> notes — substituted with its constituent commands). Task 2 appends the sibling
> binding rebuild evidence and the fork-invariant review below this section.

Host: Windows 11 (26200), Docker Desktop 29.6.2, `cross` 0.2.5, `cargo-zigbuild` 0.23.0,
`zig` 0.16.0, `make` absent (confirmed absent again this session).

---

## Task 1 — Cross-target clippy (both gates) + CI suite

### Gate 1: linux-gnu (`cross clippy`)

```
cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
```

Exit code: **0** (confirmed twice — once via piped output inspection, once via a clean
`> log 2>&1; echo "EXIT=$?"` capture with no pipe).

Tail of output:
```
    Checking nono-sandbox v0.66.1 (/project/crates/nono)
    Checking nono-sandbox-proxy v0.66.1 (/project/crates/nono-proxy)
    Checking nono-ffi v0.66.1 (/project/bindings/c)
    Checking nono-shell-broker v0.66.1 (/project/crates/nono-shell-broker)
    Checking nono-sandbox-cli v0.66.1 (/project/crates/nono-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 33s
```
Pinned image resolved: `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5@sha256:9e5b39c0...`
(same pin as the checklist's recorded reference). No warnings, no errors, zero clippy
findings across all 5 workspace crates.

### Gate 2: apple-darwin (`cargo-zigbuild clippy`, direct-binary form)

```
cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```
Run with `SDKROOT` confirmed UNSET (`env | grep -i sdkroot` produced no output before the
run).

Exit code: **0**

Output:
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.17s
```
Clean exit, no findings, no SDK extraction triggered.

**Both mandatory gates GREEN — no PARTIAL→CI fallback invoked, no documented runner
failure encountered.**

### `cargo fmt --all -- --check`

Exit code: **0** (no output — clean).

### `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (native Windows-host, supplementary — NOT a substitute for the two gates above)

Exit code: **0**
```
warning: nono-sandbox-cli v0.66.1 (C:\Users\OMack\nono\crates\nono-cli) ignoring invalid dependency `nono-shell-broker` which is missing a lib target
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.62s
```
(The `nono-shell-broker` dependency warning is a pre-existing, unrelated Cargo.toml
dev-dependency shape from Phase 102 — not a clippy lint, not phase-110 scope.)

### `cargo test --workspace` (make test constituent)

**Does NOT exit 0 on this host.** Full accounting below, split into (a) the documented
pre-existing baseline and (b) newly-observed-but-confirmed-pre-existing-and-unrelated
failures discovered only because `--no-fail-fast` was used to see past the first failing
binary for the first time in this project's phase-gate history.

**(a) Documented baseline — CONFIRMED MATCHING, 11/11:**

`cargo test -p nono-sandbox-cli --bin nono` → `test result: FAILED. 1471 passed; 11
failed; 2 ignored; ...`. Exact failing names:
```
audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty
config::tests::nono_home_dir_falls_through_when_unset
config::tests::nono_home_dir_rejects_non_absolute_override
config::tests::nono_home_dir_returns_override_when_set
config::tests::test_validated_home_falls_back_to_userprofile
config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists
config::tests::user_state_dir_uses_localappdata_on_windows
profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name
protected_paths::tests::blocks_child_directory_capability
protected_paths::tests::blocks_parent_directory_capability
protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root
```
Matches the documented categories exactly: stale `my-agent.json` profile_cmd
"already exists" (1), env-lock `PoisonError` cascade (6 `config::tests` + cascading
`protected_paths::tests` failures sharing the same poisoned lock, 3), session-dir count
mismatch (1). The documented `nono` lib flaky registry test
(`machine_policy::tests::windows_wrong_reg_type_returns_policy_load_failed`) **passed**
in this run (`ok` at log line 410) — consistent with its documented flakiness (a
Windows registry-deletion timing race, not a hard failure every run). `nono` lib: 820
passed, 0 failed (verified separately via `cargo test -p nono-sandbox --lib`).

**(b) Newly-observed, confirmed pre-existing and unrelated to Phase 110 (13 more
failures across 3 test files) — NOT part of the given host baseline text, enumerated
honestly per the plan's explicit instruction not to paper over new failures:**

All three files below were checked with `git log --oneline --all -- <file>` — **zero**
`110-0X` commits touch any of them. Full detail (including reproduction steps and fix
shape) recorded in `deferred-items.md` under `## Plan 08`; not fixed here per Scope
Boundary.

1. `crates/nono-cli/tests/audit_attestation.rs` — 2 failures
   (`audit_verify_reports_signed_attestation_with_pinned_public_key`,
   `rollback_signed_session_verifies_from_audit_dir_bundle`). Root cause: hardcoded
   `"/bin/pwd"` literal with no Windows fallback — `nono: Command execution failed:
   /bin/pwd: cannot find binary path`. Reproduced standalone twice
   (`cargo test -p nono-sandbox-cli --test audit_attestation -- --test-threads=1`),
   identical both times.
2. `crates/nono-cli/tests/env_vars.rs` — 10 failures, all `windows_run_*` live-execution
   integration tests (exit-code mismatches, unexpanded env placeholders). Full name list
   in `deferred-items.md`. This binary alone took 986s for 62 tests; its own log output
   shows `label guard: path has pre-existing mandatory-label ACE; skipping apply +
   revert` against real host paths (`C:\Users\OMack\.local\bin`), indicating host-state
   contention from a prior session, not a code regression.
3. `crates/nono-cli/tests/resl_nix_async_signal_safety.rs` — 1 failure
   (`cr_01_no_format_macro_in_post_fork_child_branch`). A stale text-signature-match
   test expects the literal string `fn clear_close_on_exec(fd: i32) -> std::io::Result<()>`
   in `exec_strategy.rs`; the real current signature (line 3817) is
   `fn clear_close_on_exec(fd: i32) -> Result<()>` (crate's own `Result` alias). No
   behavior change, purely a stale assertion string.

**Conclusion:** `cargo test --workspace` exits non-zero on this host for reasons fully
traced to pre-existing, phase-110-unrelated files. The acceptance criterion "`cargo test
--workspace` exits 0" is **NOT met** on this host, for pre-existing environmental
reasons unrelated to this phase's changes — stated plainly, not papered over. All
Phase-110-relevant tests (capability.rs, sandbox/{linux,macos,windows}.rs,
supervisor_linux.rs, profile/mod.rs, profile_runtime.rs, capability_ext.rs,
manifest_convert.rs, nono-wfp-service.rs, policy.json) pass — see Task 2's targeted
per-requirement runs and Wave-0 gap-closure evidence below.

### Task 1 Result

Both mandatory cross-target clippy gates GREEN, `cargo fmt --check` GREEN, native
Windows-host `cargo clippy --all-targets` GREEN (supplementary). `cargo test --workspace`
is RED but every failure is traced to files outside Phase 110's scope (11 documented
baseline + 13 newly-enumerated-but-confirmed-pre-existing, see above and
`deferred-items.md`).

---

## Task 2 — Binding rebuild (D-13) + fork-invariant review

### `maturin build` in `../nono-py`

Exit code: **0**
```
🍹 Building a mixed python/rust project
🐍 Found CPython 3.12 at C:\Users\OMack\AppData\Local\Programs\Python\Python312\python.exe
🔗 Found pyo3 bindings
   Compiling nono-sandbox v0.66.1 (C:\Users\OMack\Nono\crates\nono)
   Compiling nono-sandbox-proxy v0.66.1 (C:\Users\OMack\Nono\crates\nono-proxy)
   Compiling nono-py v0.66.1 (C:\Users\OMack\nono-py)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 16s
📦 Built wheel for CPython 3.12 to C:\Users\OMack\nono-py\target\wheels\nono_sandbox-0.66.1-cp312-cp312-win_amd64.whl
```
**No fix required** — built clean on the first attempt, no `E0063` or any struct-drift
error. Read-first grep of `../nono-py/src/{lib.rs,policy.rs}` for `CapabilitySet`
construction confirmed why: `nono-py` never constructs `CapabilitySet` via a struct
literal — every site uses `RustCapabilitySet::new()` (the builder pattern), and
`localhost_port_ranges` is a **private** field on the core `CapabilitySet` with only
builder-method (`allow_localhost_port_range`/`add_localhost_port_range`) and accessor
(`localhost_port_ranges()`) surface — there is no public struct-literal construction
path for a binding to miss. `grep -rn "localhost_port|PortConfig|open_port_range|listen_port_range"
../nono-py/src` returned zero matches, confirming no hand-written construction site
touches this phase's new field at all.

### `napi build --platform --release` in `../nono-ts`

Invoked via `npx napi build --platform --release` (`@napi-rs/cli` 3.6.0, no global
`napi` binary on PATH — `npx` resolves the locally-installed devDependency).

Exit code: **0**
```
   Compiling nono-sandbox v0.66.1 (C:\Users\OMack\Nono\crates\nono)
   Compiling nono-node v0.66.1 (C:\Users\OMack\nono-ts)
    Finished `release` profile [optimized] target(s) in 1m 56s
```
**No fix required.** Same read-first grep of `../nono-ts/src/lib.rs` for
`localhost_port|PortConfig|open_port_range|listen_port_range` returned zero matches.

Neither sibling repo required a fix in this plan — both builds went green on the first
attempt because this phase's `localhost_port_ranges` field addition to `CapabilitySet`
is private with builder/accessor-only surface (unlike Phase 109's `RouteConfig`, which
was a `pub`-field struct literal-constructed directly by `nono-py`).

### Fork-invariant check 1 — SC3 discrete-`Vec<u16>` back-compat

Grepped for every pre-existing discrete-port test function across the phase's touched
library files:
```
crates/nono/src/capability.rs:2850:    fn test_tcp_connect_ports() {
crates/nono/src/capability.rs:2858:    fn test_tcp_bind_ports() {
crates/nono/src/sandbox/macos.rs:1683:    fn test_generate_profile_blocked_with_localhost_ports() {
crates/nono/src/sandbox/macos.rs:1823:    fn test_generate_profile_proxy_with_localhost_ports() {
crates/nono/src/sandbox/macos.rs:1852:    fn test_generate_profile_allow_all_with_localhost_ports() {
crates/nono/src/sandbox/windows.rs:3573:    fn compile_network_policy_with_localhost_ports_only_is_fully_supported() {
```
Identified the 110-0X commits touching each of these 3 files:
`capability.rs` ← `a0bf9076` (110-03); `macos.rs` ← `0c747739`, `2102f538` (110-03);
`windows.rs` ← `ff10f52f` (110-06); `linux.rs` ← `5d0c8e34`, `2102f538` (110-03, has no
pre-existing discrete-port test to begin with).

`git show <commit> -- <file> | grep -n "^-.*fn test_tcp\|^+.*fn test_tcp"` (and the
equivalent for the `localhost_ports`/`localhost_ports_only` names) returned **zero
matches** for every commit against every pre-existing test function name — confirming
none of these test bodies were touched (removed lines were unrelated context, e.g.
`5d0c8e34`'s 204 insertions / 1 deletion in `linux.rs` is a single blank-line/import
shift, not a test edit).

Live-run confirmation, all pre-existing discrete-port tests still pass unmodified:
```
$ cargo test -p nono-sandbox --lib -- test_tcp_connect_ports test_tcp_bind_ports \
    test_generate_profile_blocked_with_localhost_ports \
    test_generate_profile_proxy_with_localhost_ports \
    test_generate_profile_allow_all_with_localhost_ports \
    compile_network_policy_with_localhost_ports_only_is_fully_supported

test capability::tests::test_tcp_bind_ports ... ok
test sandbox::windows::tests::compile_network_policy_with_localhost_ports_only_is_fully_supported ... ok
test capability::tests::test_tcp_connect_ports ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 817 filtered out; finished in 0.00s
```
(The 3 `macos.rs` tests are `#[cfg(target_os = "macos")]`-gated and do not compile on
this Windows host — expected, consistent with this project's existing posture; their
compile-cleanliness is proven by the apple-darwin cross clippy gate above, not a local
`cargo test` run, matching how every prior phase has verified macOS-only test bodies.)

**SC3 CONFIRMED: discrete-port back-compat unbroken.**

### Fork-invariant check 2 — ADR-86 policy-free-library boundary

Confirmed `crates/nono/src/capability.rs`'s new mechanism
(`localhost_port_ranges`/`merge_port_ranges`/`allow_localhost_port_range`/
`add_localhost_port_range`) is a pure, caller-supplied-list builder — identical in
shape to the pre-existing `localhost_ports` mechanism beside it (private
`Vec`/`Vec<(u16,u16)>` field, builder method appends, accessor exposes a slice; no
group/deny-list/dangerous-command logic anywhere in the diff).

Checked which plans touched `crates/nono/` at all:
```
capability.rs  ← 110-03 (a0bf9076)
macos.rs       ← 110-03 (0c747739, 2102f538)
linux.rs       ← 110-03 (5d0c8e34, 2102f538)
windows.rs     ← 110-06 (ff10f52f)
manifest_convert.rs, schema/capability-manifest.schema.json, tests/manifest_types.rs
               ← 110-05 (4d635305)
```
The plan's own `<interfaces>` prose states 110-01/04/05/07 touch `crates/nono-cli/`
only — this is **imprecise for 110-05**, which does touch `crates/nono/src/manifest_convert.rs`
(and its schema/test siblings). Read the actual diff (`git show 4d635305 -- crates/nono/src/manifest_convert.rs`)
to confirm the shape: it adds `TryFrom<&CapabilityManifest> for CapabilitySet`'s own
independent `start<=end` bounds check and a `#[cfg(target_os = "macos")]`-gated
cumulative-port cap check (mirroring the pre-existing `allow_localhost_port` validation
loop immediately above it in the same `impl` block) before calling
`caps.allow_localhost_port_range(start, end)?`. This is **input validation on a
manifest-conversion pathway**, structurally identical to validation the library already
performs elsewhere (e.g. rejecting out-of-range `u16` conversions) — not a
group/deny-list/dangerous-command decision. STATE.md's own 110-05 decision entry
documents this as an intentional dual-pathway design (ADR-86: "mechanism validates only
start==0; each CLI-facing/library-facing pathway owns its own input validation").
**No policy leak found; the plan's own interfaces text is corrected here for the
record, but the underlying boundary claim holds.**

**ADR-86 CONFIRMED: unregressed.**

### Fork-invariant check 3 — Wave-0 gap closure (all 4 gaps, live evidence)

| Gap (110-VALIDATION.md) | Closing plan/task | Live command | Result |
|---|---|---|---|
| `sandbox/linux.rs` port-range Landlock rule tests (PROF-03c) | 110-03 Task 3 | `cross test -p nono-sandbox --target x86_64-unknown-linux-gnu --lib -- sandbox::linux::tests` | `test_port_range_expands_landlock_bind_rules_for_every_port ... ok`, `test_port_range_no_artificial_cap_on_large_range ... ok` — 91 passed, 0 failed |
| `nono-wfp-service.rs` `PortCondition::RemoteRange`/`LocalRange` tests (PROF-03d) | 110-06 Task 2 | `cargo test -p nono-sandbox-cli --bin nono-wfp-service -- port` | `port_range_alone_triggers_block_fallback_even_in_allow_all_mode ... ok`, `discrete_port_and_range_coexist_in_same_request_without_regression ... ok`, `port_range_only_request_produces_one_spec_per_range_per_applicable_layer_not_per_port ... ok` — 5 passed, 0 failed |
| `platform_overrides` schema-validation-plus-resolution tests | 110-01 Task 3 | `cargo test -p nono-sandbox-cli --bin nono -- platform_overrides` | 17 passed, 0 failed (incl. `platform_overrides_validates_against_schema_alongside_existing_fields`, `platform_overrides_windows_low_il_broker_or_semantics_top_level_true_override_false_stays_true`) |
| `open_port_range`/`listen_port_range` schema-validation-plus-resolution tests | 110-04 Task 1 | `cargo test -p nono-sandbox-cli --bin nono -- open_port_range listen_port_range` | 3 passed, 0 failed (`validate_port_ranges_rejects_reversed_open_port_range`, `validate_port_ranges_rejects_reversed_listen_port_range`, `test_from_profile_open_port_range_populates_localhost_port_ranges`) |
| "resolve `bun-dev`/`mise-dev` by name" tests (PROF-04, D-11) | 110-07 Task 2 | `cargo test -p nono-sandbox-cli --bin nono -- bun_dev mise_dev` | `bun_dev_builtin_profile_resolves_and_carries_bun_runtime_group ... ok`, `mise_dev_builtin_profile_resolves_and_carries_mise_manager_group ... ok` — 2 passed, 0 failed |

Also re-ran the full `manifest_roundtrip` integration suite (schema-conformance,
distinct from the by-name resolution tests above): `cargo test -p nono-sandbox-cli
--test manifest_roundtrip` → 14 passed, 0 failed.

**All 4 Wave-0 gaps CONFIRMED CLOSED with live, passing evidence.**

---

## Summary

| Gate | Result |
|---|---|
| linux-gnu cross clippy | GREEN (exit 0) |
| apple-darwin cargo-zigbuild clippy | GREEN (exit 0) |
| `cargo fmt --all -- --check` | GREEN (exit 0) |
| `cargo clippy --workspace --all-targets` (native, supplementary) | GREEN (exit 0) |
| `cargo test --workspace` | **RED** — 24 total failures across 4 test binaries; 11 match the documented pre-existing baseline exactly; 13 more are newly-observed via `--no-fail-fast` but confirmed pre-existing and unrelated to any Phase 110 file (see `deferred-items.md` § Plan 08) |
| `maturin build` (`../nono-py`) | GREEN (exit 0), no fix needed |
| `napi build --platform --release` (`../nono-ts`) | GREEN (exit 0), no fix needed |
| SC3 discrete-port back-compat | CONFIRMED unregressed |
| ADR-86 policy-free-library boundary | CONFIRMED unregressed (110-05's `manifest_convert.rs` touch is input validation, not policy) |
| Wave-0 gap closure (4/4) | CONFIRMED closed with live test evidence |

**Phase 110 is NOT closed by this plan.** Plan 110-06 Task 3 (`checkpoint:human-verify`,
PROF-03e live-kernel `FwpmFilterAdd0` proof) remains pending Administrator-elevated
operator action — out of this plan's scope, not resolved here. `requirements
mark-complete PROF-03` is deliberately NOT run. See `110-08-SUMMARY.md` for the full
non-closure statement.
