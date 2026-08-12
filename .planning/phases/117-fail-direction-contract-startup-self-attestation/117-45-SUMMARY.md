---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 45
subsystem: agent-daemon
tags: [windows, daemon, green-by-absence, test-hygiene, gap-closure, round-4, wr-31, wr-33, cint-02, cint-03]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-35's baseline; 117-36's layer_registry.rs as landed"
provides:
  - "DaclAncestorTraverse attested from traverse_applied — its negative is representable and its
    failure reportable under its own name (WR-31)"
  - "daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition extended to enforce
    input distinctness for the second row pair"
  - "The D-37 %PUBLIC% fixture: exclusive create, reparse-point check, unwind-safe cleanup (WR-33)"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Verify which BINARY a module compiles into before trusting a test command — agent_daemon is
      in nono-agentd, and `--bin nono ... agent_daemon::` matches 0 tests while reporting ok"
    - "On Windows a directory junction is a reparse point that is_symlink() does not reliably
      report; check FILE_ATTRIBUTE_REPARSE_POINT instead"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/agent_daemon/launch.rs

key-decisions:
  - "WR-31's fix is input-distinctness plus a named report, NOT a new decision variant.
    DaemonAttestationDecision has exactly two variants and daemon_decision_enum_variants pins that
    list; WR-06 forbids aborting. Adding a ProceedDowngraded variant is a design change no finding
    asked for — raised as an open operator question instead."
  - "DaclAncestorReadAttrs is NOT a third shared-input row. The doc passage naming it describes the
    OLD hardcoded list CR-06 removed, along with its (Daemon, None) expectancy cell."
  - "Used FILE_ATTRIBUTE_REPARSE_POINT rather than the plan's suggested is_symlink(), which does
    not reliably report directory junctions and would have left the stated defense non-existent."
---

# Plan 117-45: WR-31 + WR-33 — Execution Summary

**Executed:** 2026-08-12 · **Wave:** 17 · **Requirements:** CINT-02, CINT-03

## Task 1 — WR-31: give `DaclAncestorTraverse` its own input

`daemon_attest_and_decide` attested **two** distinct `Abort`-outcome rows, both declared expected at
`(Daemon, None)`, from **one** input. `granted_write_access()` reads `write_applied`, which only
pass 2 (the workspace write grant) populates; `traverse_applied` — the vector passes 1 and 3
populate, i.e. everything `DaclAncestorTraverse` actually describes — was never read by the gate.
So the row's negative was unrepresentable, and when the shared predicate fired the report always
named `DaclPackageSidGrant`, making an ancestor-traverse failure unreportable in principle.

Added `granted_ancestor_traverse()`, threaded `ancestor_traverse_applied` as a 7th parameter, and
gave the row its own arm. The stale doc comment — which documented the shared input as intended —
was corrected.

### The prescribed "non-aborting distinct classification" is not expressible

`DaemonAttestationDecision` has exactly two variants, `Proceed` and `Abort`, and
`daemon_decision_enum_variants` pins that list by parsing the enum's own source. WR-06's
disposition (an absent ancestor traverse under-grants reach, never widens confinement) forbids
aborting. So there is no outcome to classify a non-aborting failure into.

The review's actual complaint is narrower than its prescribed fix: *"its negative is
unrepresentable ... a failure is unreportable even in principle."* Both halves are now fixed —
the row reads its own fact, and when that fact is false it is **reported under its own name** via a
`tracing::warn!` naming `DaclAncestorTraverse`, while the launch proceeds per WR-06.

**Open operator question:** whether the daemon should gain a `ProceedDowngraded` variant so an
under-granted layer is a first-class decision rather than a log line. That is a design change no
finding asked for, and it would ripple into the caller and the variant-pinning test. Raised here
rather than decided.

### Per-call-site argument decisions (required by acceptance criteria)

| Site | New arg | Reason |
|---|---|---|
| production (`:941`) | `ancestor_traverse_applied` | the guard's own report, captured beside `dacl_guard_applied` before the guard moves into `tenant` |
| `null_handle_aborts_on_app_container_profile` | `true` | asserts an AppContainer abort upstream of both DACL rows; traverse is irrelevant |
| `dacl_guard_applied == false` case | `true` | exists to drive `DaclPackageSidGrant`'s abort; passing `false` too would confound which input caused it |
| `network_scoping_required` case | `true` | drives the WFP row |
| wrong-SID case | `true` | drives the AppContainer row |
| the two real-handle smoke tests | `true` | assert `Proceed`; traverse present keeps them testing that |
| `daemon_attest_and_decide_result_matches_exhaustively` | `true` | exhaustive-match shape only |

Literal `true` everywhere in tests is deliberate and distinct from the production expression, so it
cannot reintroduce the coupling.

`:1909-1911`'s `assert_eq!(layer, "DaclPackageSidGrant")` remains **correct** — that test drives
`dacl_guard_applied == false`, which is still exactly what names that layer.

### `DaclAncestorReadAttrs` is not a third instance

The plan pointed at it as a candidate sharing the input. It is not: the doc passage naming it
describes the **old hardcoded** `ProceedDowngraded` list that CR-06 **removed**, together with its
`(Daemon, None)` expectancy cell, because `DaemonDaclGuard::apply` never calls
`grant_sid_read_attributes_on_path`. The `Daemon`-expected DACL class is two rows, not three.

## Task 2 — input-distinctness enforced across the class

Extended the existing wiring test (rather than adding a parallel one): `args.len()` 6 → 7, plus a
not-a-literal check and `assert_ne!(ancestor_traverse_applied, dacl_guard_applied)` mirroring the
`wfp_filters_installed`/`network_scoping_required` rule already there.

**Perturbation:** recoupling the 7th argument to `dacl_guard_applied` fails it:

```
assertion `left != right` failed: ancestor_traverse_applied and dacl_guard_applied must be
independent expressions — DaclAncestorTraverse describes passes 1+3's traverse grants while
DaclPackageSidGrant describes pass 2's workspace write grant, and sharing one input is exactly WR-31
```

## Task 3 — WR-33 fixture hygiene

`create_dir_all` → `create_dir` (fails on an existing path, so a pre-planted junction fails setup
instead of being bound to), a `Drop`-based `WorkspaceCleanup` guard bound immediately after
creation, and a reparse-point check before any ACE is written.

**Reparse detection deviates from the plan.** It suggested
`symlink_metadata()?.file_type().is_symlink()`; on Windows a directory **junction** is a reparse
point that `is_symlink()` does not reliably report, so that check would have left the stated defense
non-existent — the WR-24/WR-27 shape. Used
`MetadataExt::file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT` instead, which covers junctions,
mount points and symlinks alike.

**The sibling site is NOT the same exposure.** `:2735` in
`daemon_dacl_guard_reap_revokes_traverse_paths` uses `tempfile::tempdir()` — a private per-process
directory with `TempDir`'s own Drop cleanup, not a world-writable shared location. Checked rather
than assumed; WR-33 is genuinely one site.

**Perturbation:** pre-planting the fixture path (simulated in-test, since the name embeds pid+nanos
and cannot be pre-created externally) makes the exclusive create fail loudly:

```
exclusively create a test-owned workspace under %PUBLIC% ... If this fails with AlreadyExists, the
path was pre-planted — do NOT relax this to create_dir_all (WR-33):
Os { code: 183, kind: AlreadyExists, message: "Cannot create a file when that file already exists." }
```

That perturbation also **reproduced WR-33's leak live**: it panicked between creation and the
guard's construction and left
`C:\Users\Public\nono-test-d37-daemon-dacl-86268-1786561544645598600` behind. `icacls` showed only
inherited ACEs (no package-SID ACE — the panic preceded `apply`). Removed, and a normal run
verified to leave nothing:

```
no leftover after a normal run — cleanup guard verified
```

## Deviation: the plan's verify command targets the wrong binary

The plan specifies `cargo test -p nono-sandbox-cli --bin nono ... agent_daemon::`. That matches
**0 tests** — `agent_daemon` compiles into `nono-agentd` (`src/bin/nono-agentd.rs` `#[path]`-includes
only `agent_daemon/mod.rs`, `telemetry/mod.rs`, `agent_daemon/telemetry_init.rs`), and the `nono`
binary does not rebuild when this file changes. Run as written it reports
`ok. 0 passed; ... 1651 filtered out` — a green that never executes a line of the changed code.
The correct target is `--bin nono-agentd`.

Relatedly, the new test first landed in the outer source-text `mod tests` rather than
`windows_impl::attestation_gate_tests` and failed to compile (`DaemonAttestationDecision`,
`CreateJobObjectW`, `CloseHandle` out of scope). Relocated.

## Verification

| Gate | Command | Result |
|---|---|---|
| Daemon binary | `cargo test -p nono-sandbox-cli --bin nono-agentd --no-fail-fast` | **97 passed / 0 failed** |
| WR-31 test | `... ancestor_traverse_and_package_sid_grant_classify_independently` | pass |
| Wiring test | `... daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition` | pass |
| WR-33 test | `... daemon_dacl_guard_apply_succeeds_when_immediate_ancestor_is_non_owned` | pass |
| Format | `cargo fmt --all -- --check` | clean |
| Cross-target clippy (linux-gnu) | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |
| Cross-target clippy (apple-darwin) | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |
