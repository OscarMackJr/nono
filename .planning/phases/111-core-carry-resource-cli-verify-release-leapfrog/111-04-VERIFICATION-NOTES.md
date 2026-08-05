# 111-04 Verification Notes — Combined 108-111 Fork-Invariant Pass (VERIFY-01)

> Live, pasted command output/exit codes for both mandatory cross-target clippy gates,
> the `make ci` constituents (`make` absent from PATH on this host), the
> `cargo test --workspace --no-fail-fast` baseline diff, both sibling-binding rebuilds,
> and the D-09 grep/diff structural assertions (D-01, ADR-86 carve-out).
>
> **This record supersedes `.planning/phases/110-profile-policy-absorb-platform-overrides/110-08-VERIFICATION-NOTES.md`
> per D-09.** That record predates 4 defect-fix commits landed 2026-08-04
> (`7c7a189c`/`ea26b5b2`/`6d7ef719`/`4aec1944`, closing PROF-03e's live-kernel checkpoint)
> and Phase 111's own CORE-01/CORE-02 changes — `has_port_rules()` shipped broken straight
> through the 110-08 certification. This pass covers the combined 108-111 surface honestly,
> not merely Phase 111's own small diff.

Host: Windows 11 (26200), Docker Desktop 29.6.2, `cross` 0.2.5, `cargo-zigbuild` 0.23.0,
`zig` 0.16.0, `make` absent (confirmed absent again this session).

---

## Task 1 — Both cross-target clippy gates + D-01/ADR-86 grep-diff assertions

### D-01 assertion: no `pub mod resource;` in core library

```
$ grep -n "^pub mod resource" crates/nono/src/lib.rs
(zero matches, exit 1)

$ ls crates/nono/src | grep -i resource
(zero matches, exit 1)
```

Full `pub mod` list in `crates/nono/src/lib.rs` (18 modules, none named `resource*`):
`agent, audit, capability, diagnostic, error, keystore, machine_policy, manifest,
manifest_convert, net_filter, path, query, sandbox, scrub, state, supervisor, trust, undo`.

**D-01 CONFIRMED: still holds after CORE-01/CORE-02.** Resource limits remain CLI-side per
ADR-111's ADAPT-not-adopt disposition of upstream `e6d26871`/#1269.

### ADR-86 Windows carve-out assertion: `exec_strategy_windows/` unregressed

```
$ git diff c5c8c5fc6e6b146ba5f27e6cd1d427b2da750ba2..HEAD -- crates/nono-cli/src/exec_strategy_windows/
(empty output, confirmed via `wc -l` = 0)
```

Base SHA `c5c8c5fc` = "docs(111): add validation strategy" (pre-Wave-1). Confirms neither
`111-01` (CORE-01: `policy.json` + `exec_strategy.rs` MAX_CRYPTO_THREADS) nor `111-02`
(CORE-02: `cli.rs` + `docs/cli/usage/flags.mdx` help text) touched this directory.

**ADR-86 Windows carve-out CONFIRMED: structurally unregressed.**

### Gate 1: linux-gnu (`cross clippy`)

```
docker info | grep "Server Version"  ->  Server Version: 29.6.2  (engine up)

cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
```

Exit code: **0**

Output (34 lines total — build layer cached from Plan 111-01/111-03's own gate runs on this
same unchanged tree since Wave 2 modifies no source; `Finished` in 14.82s with zero
`Compiling`/error/warning lines is expected incremental-fresh behavior, not a skipped run —
the pinned image layer itself was rebuilt/verified (`#4`/`#5`/`#6` BuildKit steps present)
before cargo ran):
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.82s
EXIT=0
```
Pinned image: `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5@sha256:9e5b39c09874bc1816c675ed11afca2c2ed6cee0c4ed2b3c1d5763c346c9ae3f`
(matches the checklist's recorded pin). Zero clippy findings.

### Gate 2: apple-darwin (`cargo-zigbuild clippy`, direct-binary form)

```
$ echo "SDKROOT=[$SDKROOT]"
SDKROOT=[]   (confirmed unset before the run)

cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

Exit code: **0**
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.17s
EXIT=0
```
Clean exit, no findings, no SDK extraction.

**Both mandatory gates GREEN over the combined 108-111 surface — no PARTIAL→CI fallback
invoked, no documented runner failure encountered.**

### Task 1 Result

D-01 confirmed holding. ADR-86 Windows carve-out confirmed structurally unregressed. Both
cross-target clippy gates GREEN, zero findings, re-run fresh over the combined 108-111 tree
per D-09/D-10.

---

## Task 2 — `make ci` substitution + 24-name baseline diff + resource-flag regression guard

`make` confirmed absent from PATH again this session. Constituent commands run directly.

### `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` (native Windows host)

Exit code: **0**
```
warning: nono-sandbox-cli v0.66.1 (C:\Users\OMack\nono\crates\nono-cli) ignoring invalid dependency `nono-shell-broker` which is missing a lib target
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.56s
```
(Pre-existing, unrelated Cargo.toml dev-dep warning from Phase 102 — not a clippy lint.)

### `cargo fmt --all -- --check`

Exit code: **0** (no output — clean; confirms clippy-green does NOT by itself imply
rustfmt-clean, this was run and checked independently per host constraints).

### `cargo audit`

Exit code: **0**. 6 allowed advisory warnings (all `unmaintained`/`unsound` transitive-dep
notices: `async-std` RUSTSEC-2025-0052, `fxhash` RUSTSEC-2025-0057, `paste`
RUSTSEC-2024-0436, `rustls-pemfile` RUSTSEC-2025-0134, `anyhow` RUSTSEC-2026-0190,
`event-listener` RUSTSEC-2026-0221), zero vulnerabilities. `crossbeam-epoch`
RUSTSEC-2026-0204 does NOT appear — confirmed still closed (fixed pre-phase by quick task
`260729-nh4`, per STATE.md).

### `cargo test -p nono-sandbox` (make-test `test-lib` constituent)

Exit code: **0**. `9 passed; 0 failed` (doc-tests only ran; `--lib` unit tests already
covered by the full-suite run below at `820 passed; 0 failed`).

### `cargo test -p nono-sandbox-cli --bin nono` (make-test `test-cli` constituent)

Exit code: **101** (expected — reproduces the documented 11-name baseline exactly).
`1474 passed; 11 failed; 2 ignored`. Failing names, byte-for-byte identical to the 11-name
`--bin nono` baseline in `110-08-VERIFICATION-NOTES.md`:
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
(1474 passed here vs 1471 in 110-08 — the +3 delta is Wave 1's 2 new Wave-0 tests
(`111-01`'s `~/.cache` policy assertion + `MAX_CRYPTO_THREADS==12` assertion) plus one more
test added since, all passing; no baseline name lost or gained.)

### `cargo test -p nono-ffi` (make-test `test-ffi` constituent)

Exit code: **0**. `49 passed; 0 failed`.

### Full baseline sweep: `cargo test --workspace --no-fail-fast`

**First attempt genuinely stalled and had to be redone — recorded honestly, not hidden.**
The first invocation (foreground, auto-promoted to a background job by this harness after
its 600s wall-clock cap) reached `env_vars.rs` and then produced zero further log output for
~39 minutes with no `cargo`/`nono`/`rustc` process observable via `tasklist`/`ps` — the
harness's auto-promoted background job appears to have been silently killed rather than
continuing to run. This is a harness/environment behavior, not a code defect. The run was
restarted explicitly with `run_in_background: true` (not relying on timeout auto-promotion),
which completed successfully end-to-end.

Exit code: **101** (expected — `cargo test --workspace` does not exit 0 on this host, matching
110-08's own documented finding).

**Full observed failing-test-name set this run (17 names) — diffed against the documented
24-name baseline (11 + 13) from `110-08-VERIFICATION-NOTES.md`:**

| # | Test name | Binary | In 24-name baseline? |
|---|---|---|---|
| 1 | `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty` | `nono` --bin | yes |
| 2 | `config::tests::nono_home_dir_falls_through_when_unset` | `nono` --bin | yes |
| 3 | `config::tests::nono_home_dir_rejects_non_absolute_override` | `nono` --bin | yes |
| 4 | `config::tests::nono_home_dir_returns_override_when_set` | `nono` --bin | yes |
| 5 | `config::tests::test_validated_home_falls_back_to_userprofile` | `nono` --bin | yes |
| 6 | `config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists` | `nono` --bin | yes |
| 7 | `config::tests::user_state_dir_uses_localappdata_on_windows` | `nono` --bin | yes |
| 8 | `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` | `nono` --bin | yes |
| 9 | `protected_paths::tests::blocks_child_directory_capability` | `nono` --bin | yes |
| 10 | `protected_paths::tests::blocks_parent_directory_capability` | `nono` --bin | yes |
| 11 | `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root` | `nono` --bin | yes |
| 12 | `audit_verify_reports_signed_attestation_with_pinned_public_key` | `audit_attestation.rs` | yes |
| 13 | `rollback_signed_session_verifies_from_audit_dir_bundle` | `audit_attestation.rs` | yes |
| 14 | `windows_run_allow_all_network_probe_connects` | `env_vars.rs` | yes |
| 15 | `windows_run_blocks_live_block_net_without_enforcement` | `env_vars.rs` | yes |
| 16 | `windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified` | `env_vars.rs` | yes |
| 17 | `cr_01_no_format_macro_in_post_fork_child_branch` | `resl_nix_async_signal_safety.rs` | yes |

**Zero new failure names outside the documented 24-name list. This run's 17-name failing set
is a strict SUBSET of the 24-name baseline — not a superset, not a divergent set.**

`env_vars.rs` only failed 3 of its 10 previously-documented names this run (`59 passed; 3
failed; 14 ignored; finished in 2702.39s` — 45 min, vs 986s/16.4min in 110-08). The 7 names
that failed in 110-08 but PASSED here
(`windows_run_executes_basic_command`,
`windows_run_filters_dangerous_env_vars_and_keeps_safe_ones`,
`windows_run_filters_host_toolchain_home_vars_without_runtime_dir`,
`windows_run_live_default_profile_executes_command`,
`windows_run_propagates_child_exit_code`,
`windows_run_smoke_validates_stdout_stderr_and_exit_code`,
`windows_run_supervised_blocks_runtime_capability_elevation_with_actionable_diagnostic`)
are consistent with 110-08's own characterization of this binary as flaky/host-state-
contention-dependent (mandatory-label ACE contamination on real host paths), not a code
regression — a subset failing instead of the full set is the same category of flake, not a
new phenomenon. `audit_attestation.rs`'s 2 failures reproduce the exact same root cause
verbatim (`nono: Command execution failed: /bin/pwd: cannot find binary path`, hardcoded
Unix path literal with no Windows fallback). `resl_nix_async_signal_safety.rs`'s 1 failure
reproduces the exact same stale-text-signature-match panic message verbatim (expects
`std::io::Result<()>`, current signature spells `Result<()>` via the crate's own alias).
None of these three files, nor `protected_paths.rs`/`config.rs`/`profile_cmd.rs`/
`audit_session.rs`, are touched by any `111-0X` commit (`git log --oneline --all -- <file>`
for each returns no `111-0X` SHA, mirroring the same check 110-08 already performed for
`108-`/`110-0X`).

**Conclusion: `cargo test --workspace` does NOT exit 0 on this host, for the same
pre-existing, phase-111-unrelated reasons already documented in `deferred-items.md` §
Plan 08 of Phase 110. No new regression. No fabricated GREEN.**

### Resource-flag regression guard (D-09 assertion 3)

```
cargo test -p nono-sandbox-cli --bin nono -- cpu_percent_range_enforced_by_clap max_processes_range_enforced_by_clap memory_zero_rejected_by_parser
```
Exit code: **0**. `3 passed; 0 failed`.
```
test cli::parser_tests::memory_zero_rejected_by_parser ... ok
test cli::parser_tests::max_processes_range_enforced_by_clap ... ok
test cli::parser_tests::cpu_percent_range_enforced_by_clap ... ok
```

```
cargo test -p nono-sandbox-cli --bin nono -- env_filter_flags_do_not_collide_with_phase16_flags
```
Exit code: **0**. `1 passed; 0 failed` — the "Regression guard: Phase 16's --cpu-percent /
--memory / --timeout / --max-processes remain parseable" test (`cli.rs:4147`) passes
unmodified after Plan 111-02's doc-comment rewrite.

**All D-09 assertion-3 regression guards CONFIRMED passing.**

### Task 2 Result

`make ci` constituents (clippy, fmt-check, audit, test-lib, test-cli, test-ffi) all ran; only
`test-cli`'s `--bin nono` sub-run is RED, matching the documented baseline exactly. The full
`--workspace --no-fail-fast` sweep's failing-test-name set (17 names) is a confirmed strict
subset of the documented 24-name baseline — zero new failures, honestly diffed name-by-name.
Resource-flag regression guards all pass.

---

## Task 3 — Sibling binding rebuilds + this record

Both run at the CURRENT (pre-RLS-14) version, `0.66.1` — this task validates struct/API
compatibility, not version numbers. Plan 111-06 handles the version bump later.

### `maturin build` in `../nono-py`

Exit code: **0**
```
📦 Including license file `LICENSE`
🍹 Building a mixed python/rust project
🐍 Found CPython 3.12 at C:\Users\OMack\AppData\Local\Programs\Python\Python312\python.exe
🔗 Found pyo3 bindings
📡 Using build options features from pyproject.toml
   Compiling nono-sandbox v0.66.1 (C:\Users\OMack\Nono\crates\nono)
   Compiling nono-sandbox-proxy v0.66.1 (C:\Users\OMack\Nono\crates\nono-proxy)
   Compiling nono-py v0.66.1 (C:\Users\OMack\nono-py)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 34s
📦 Built wheel for CPython 3.12 to C:\Users\OMack\nono-py\target\wheels\nono_sandbox-0.66.1-cp312-cp312-win_amd64.whl
```
**No fix required** — clean on the first attempt, matching the 110-08 precedent. CORE-01
(`policy.json` grant + private `MAX_CRYPTO_THREADS` constant) and CORE-02 (CLI-only help
text) touch no `pub` struct field either binding constructs.

### `npx napi build --platform --release` in `../nono-ts`

Exit code: **0**
```
   Compiling nono-node v0.66.1 (C:\Users\OMack\nono-ts)
   Compiling nono-sandbox v0.66.1 (C:\Users\OMack\Nono\crates\nono)
    Finished `release` profile [optimized] target(s) in 2m 36s
```
**No fix required** — clean on the first attempt.

Neither sibling repo required a fix. Both binding rebuilds confirmed green at the pre-bump
version.

---

## Summary

| Gate | Result |
|---|---|
| linux-gnu cross clippy (combined 108-111 surface) | GREEN (exit 0) |
| apple-darwin cargo-zigbuild clippy (combined 108-111 surface) | GREEN (exit 0) |
| D-01 (`no pub mod resource;`) | CONFIRMED still holding |
| ADR-86 Windows carve-out (`exec_strategy_windows/`) | CONFIRMED structurally unregressed since `c5c8c5fc` |
| `cargo clippy --workspace --all-targets --all-features` (native, supplementary) | GREEN (exit 0) |
| `cargo fmt --all -- --check` | GREEN (exit 0) |
| `cargo audit` | GREEN (exit 0, 6 pre-existing allowed advisory warnings, 0 vulnerabilities) |
| `cargo test -p nono-sandbox` | GREEN (9/9) |
| `cargo test -p nono-sandbox-cli --bin nono` | RED — 11/11 matches documented baseline exactly |
| `cargo test -p nono-ffi` | GREEN (49/49) |
| `cargo test --workspace --no-fail-fast` | RED — 17-name failing set, confirmed strict SUBSET of documented 24-name baseline, zero new names |
| D-09 assertion-3 resource-flag regression guards | GREEN (4/4) |
| `maturin build` (`../nono-py`) | GREEN (exit 0), no fix needed |
| `napi build --platform --release` (`../nono-ts`) | GREEN (exit 0), no fix needed |

**This record supersedes `110-08-VERIFICATION-NOTES.md` per D-09.** That record predates
four defect-fix commits landed 2026-08-04 (`7c7a189c` `has_port_rules()` omission,
`ea26b5b2` `cmd_show`'s missing `Network:` section, `6d7ef719` version-skew fail-open,
`4aec1944` broken filter-sweep enumeration — closing PROF-03e's live-kernel checkpoint) and
Phase 111's own CORE-01/CORE-02 changes. `has_port_rules()` shipped broken straight through
the 110-08 certification, which is exactly why D-09 mandates this pass cover the combined
108-111 surface honestly rather than merely Phase 111's own small diff. Both cross-target
clippy gates and both binding rebuilds have now been re-run fresh against the tree
containing all of Phase 110's fixes plus Phase 111's Wave 1 changes, and are confirmed
GREEN. VERIFY-01 is satisfied: the Windows security model and the ADR-86 policy-free-library
boundary are both confirmed unregressed over the honest combined surface, and every
pre-existing test failure is traced to a name already present in the documented baseline —
no new regression, no fabricated GREEN.

---

## Addendum — Orchestrator-observed 20-name run (independent second sweep, same session)

While Plan 111-04's executor was running its sweep, the phase orchestrator ran an
**independent** `cargo test --workspace --no-fail-fast` on the same tree. That run produced
**20** failing names, not 17. The extra 3 are **outside the documented 24-name baseline**:

```
test_init_creates_valid_profile              crates/nono-cli/tests/profile_cmd.rs
test_init_rejects_existing_file_without_force  crates/nono-cli/tests/profile_cmd.rs
test_schema_output_to_file                   crates/nono-cli/tests/profile_cmd.rs
```

Recorded here because the Task 2 conclusion above ("zero new failure names outside the
documented 24-name list") was true **of that executor's run**, but is not stable run-to-run.
A future executor that observes these 3 names must not mistake them for a regression.

**Diagnosed, not hand-waved.** Observed failure text:

```
thread 'test_init_creates_valid_profile' panicked at crates\nono-cli\tests\profile_cmd.rs:39:5:
expected exit 0, stderr: nono: Profile parse error: Profile file already exists:
C:\Users\OMack\AppData\Local\Temp\nono-test-profile-init\test-agent.json
Use --force to overwrite
```

Re-run in isolation immediately afterwards:

```
$ cargo test -p nono-sandbox-cli --test profile_cmd
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.93s
```

**Root cause: pre-existing test-isolation defect, NOT a code regression.**
`crates/nono-cli/tests/profile_cmd.rs` scaffolds into a **fixed, shared** temp path
(`%TEMP%\nono-test-profile-init`) rather than a unique `tempfile::TempDir`. Under
`--workspace` parallel execution these tests race each other (and the `--bin nono`
`profile_cmd::tests` unit tests) over that one directory, so whichever loses the race sees a
leftover `test-agent.json` and fails "already exists". 10/10 pass when the binary runs alone.
This is the same root-cause family as the documented baseline's own
`profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` (stale `my-agent.json`)
— a shared-fixture flake, already present before Phase 108.

Not fixed here (out of scope for a verification-only plan). Follow-up shape if ever taken up:
switch `profile_cmd.rs` to `tempfile::TempDir` so each test gets a unique directory.

**Effect on the VERIFY-01 verdict: none.** Neither `profile_cmd.rs` nor the profile-init code
path is touched by Phase 108-111 (`git log --oneline -- crates/nono-cli/tests/profile_cmd.rs`
shows no 108/109/110/111 commit). The combined-surface certification above stands; this
addendum only widens the documented baseline from 24 to a 27-name known-flake universe so the
next sweep is diffed against reality.
