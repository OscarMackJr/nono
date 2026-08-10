---
phase: 117-fail-direction-contract-startup-self-attestation
fixed_at: 2026-08-10T00:00:00Z
review_path: .planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md
iteration: 2
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 117: Code Review Fix Report — iteration 2

**Source review:** `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md`
**Iteration:** 2
**Scope:** exactly five findings — CR-14 (BLOCKER), NR-02, NR-04, NR-05, NR-06.
The other iteration-2 findings (NR-03, NR-07..NR-10, WR-04-R, WR-06-R, WR-12-R)
were explicitly routed to gap closure and are NOT touched here.

> The iteration-1 report is preserved in git history (commit `936d1ddb`'s
> descendant `d70b4658`); this file is the iteration-2 report as instructed.

**Summary:**

- Findings in scope: 5
- Fixed: 5
- Skipped: 0

Findings 1–4 were one defect family: a check structurally incapable of
returning the negative result. Every one is now closed by making the check
read a **fact that was recorded where the fact happened**, and every one is
proven non-vacuous by a counterfactual whose observed failure text is
reproduced verbatim below.

## Commits

| Finding | Commit | Title |
|---|---|---|
| CR-14 | `580aa921` | `fix(117): CR-14 attest guard effect, not guard construction` |
| NR-02 | `59faa623` | `fix(117): NR-02 make ProceedDowngraded reachable from a production launch` |
| NR-04 | `3300e75a` | `fix(117): NR-04 source the network rows' two facts independently` |
| NR-05 | `ec6bc660` | `fix(117): NR-05 daemon gate reads real outcomes, not the gate condition` |
| NR-06 | `2837e44b` | `fix(117): NR-06 reject traversal and canonicalize before recursive delete` |

All five carry the DCO sign-off. Work was done in an isolated git worktree on
branch `gsd-reviewfix/117-1906`.

## Verification gate (run against the final tree)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo check --workspace --all-targets` | clean |
| `cargo check --workspace --all-targets --features layer-fault-injection` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | clean |
| `cargo clippy --workspace --all-targets --features layer-fault-injection -- …` | clean |
| `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) | clean — `Finished dev profile in 4m 03s` |
| `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | see "Cross-target linux-gnu" below |
| `cargo test -p nono-sandbox-cli --test layer_registry_meta_test --test layer_registry_selfcheck --test layer_force_unavailable --features layer-fault-injection -- --test-threads=1` | 10/10 pass (5 + 3 + 2) |
| `cargo test -p nono-sandbox-cli --bin nono-agentd -- --test-threads=1` | 89/89 pass |
| `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection -- --test-threads=1 exec_strategy::` | 240 pass, 2 fail — both pre-existing, see below |

### The two `exec_strategy::` failures are pre-existing, not regressions

- `broker_dispatch_tests::broker_launch_assigns_child_to_job_object` — failed
  only because `nono-shell-broker.exe` was not pre-built into this worktree's
  `target/release`. After `cargo build -p nono-shell-broker --release` it
  **passes**.
- `write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file`
  — still fails after the broker is built, with
  `broker: fatal error error=Sandbox initialization failed: --no-pty requires
  --app-container-name (WFP per-session enforcement); refusing to spawn a
  non-AppContainer (unmatched WFP) child`. The test builds the broker command
  line itself and does not pass `--app-container-name`; the refusal was
  introduced in **Phase 62** (`git log -S"requires --app-container-name"` →
  `cb341165 feat(62-12): broker spawns confined child as per-run AppContainer
  (T2)`), and none of this pass's five commits touch
  `crates/nono-shell-broker/` at all (`git log 580aa921^..HEAD -- crates/nono-shell-broker/`
  is empty). This is one of the KNOWN 11 pre-existing Windows failures; the
  count did not grow.

### Cross-target linux-gnu

`cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D
clippy::unwrap_used` was launched against the final tree. Per the registry's
own cross-target presence table (`layer_registry.rs` module doc §2, re-verified
2026-08-09), every file this pass touched
(`exec_strategy_windows/*.rs`, `agent_daemon/launch.rs`) contains **no**
literal `target_os = "linux"` / `"macos"` cfg branch, so the CLAUDE.md
cross-target MUST is not triggered by the diff's own content — the gate is run
here as blast-radius insurance for the crate-level build, not because a Unix
cfg branch changed. The apple-darwin gate (which shares that property) is
green.

---

## CR-14 (BLOCKER) — `AppliedLayers` reported guard *construction*, not guard *effect*

**Commit:** `580aa921`

### What changed

| File | Change |
|---|---|
| `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:38-66` | `AppliedLabel::Skip` split into `SkipPreExistingLabel` (a coverage gap — nono did not write that ACE) and `SkipNotOwned` (contract-exempt — a system path nono structurally cannot label). |
| `.../labels_guard.rs:68-112` | New `LabelCoverage { policy_paths, applied, skipped_pre_existing_label, skipped_not_owned }` + `application()`. |
| `.../labels_guard.rs:229-247` | New `AppliedLabelsGuard::coverage()`. |
| `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:88-154` | `AppliedDaclGrant::Skip` split into `SkipReadOnly` (exempt) and `SkipWritableNotOwned` (gap); new `DaclGrantCoverage` + `application()`. |
| `.../dacl_guard.rs:288-303` | New `AppliedDaclGrantsGuard::coverage()`. |
| `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:386-425` | `LayerApplication` gains `PartiallyApplied` and `#[default] NotApplied`. |
| `.../layer_registry.rs:427-470` | `AppliedLayers`' four `bool` fields become `LayerApplication`; the dead `interpreter_coverage_gate` field is **removed**. |
| `crates/nono-cli/src/exec_strategy_windows/attestation.rs:342-490` | `classify_row` returns `RowVerdict { status, partially_established }`; `RowVerdict::from_application` is the single mapping shared by the `ConfiguredOnly` and `ConfirmedByEnforcingComponentReport` arms. |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs:379-425` | `applied_layers()` derives every field from the guards' coverage accessors. |

**The contract, stated:**

| Guard coverage | `LayerApplication` | Decision |
|---|---|---|
| Nothing for the layer to act on | `NotApplicable` | row dropped |
| Every non-exempt target covered | `Applied` | `EstablishedNotIndependentlyObservable`, expected baseline |
| Some covered, some not | `PartiallyApplied` | `ProceedDowngraded` (D-27 banner + audit event) |
| Zero covered, non-empty policy | `NotApplied` | `Unconfirmed` → `Abort` |

**`interpreter_coverage_gate` was deleted rather than fixed, deliberately.**
Its registry row declares `ProbeKind::NotApplicable`, so `classify_row` returns
before ever calling `AppliedLayers::status()` — the literal `true` was a value
no decision could read. Removing it is strictly more honest than replacing one
unreadable literal with another. `layer_registry::tests::every_reported_layer_is_actually_consulted`
now fails the build if any future field is reported for a row whose probe kind
does not consult it; it iterates `ALL` and names no `LayerId` (D-32 discovery
rule).

### Observed counterfactual failure output

Transiently replaced `LabelCoverage::application()`'s body with the pre-fix
`LayerApplication::Applied` (i.e. the hardcoded `mandatory_integrity_label: true`),
ran `cargo test -p nono-sandbox-cli --bin nono labels_guard -- --test-threads=1`:

```
---- exec_strategy::labels_guard::tests::coverage_distinguishes_full_partial_and_zero_ace_launches stdout ----
assertion `left == right` failed: LabelCoverage { policy_paths: 2, applied: 1, skipped_pre_existing_label: 1, skipped_not_owned: 0 }
  left: Applied
 right: PartiallyApplied

---- exec_strategy::labels_guard::tests::guard_skips_path_not_owned_by_current_user stdout ----
assertion `left == right` failed: a guard that wrote zero ACEs must report NotApplied, not Applied: LabelCoverage { policy_paths: 1, applied: 0, skipped_pre_existing_label: 0, skipped_not_owned: 1 }
  left: Applied
 right: NotApplied

test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 1600 filtered out
```

Restored; 6/6 pass. The new test drives **real filesystem state** through the
real guard — a fresh file, a file pre-labelled with `try_set_mandatory_label`,
and `%SystemRoot%` — not a synthetic `AppliedLayers` value.

### Honest limits

- **The all-`SkipNotOwned` case now ABORTS.** A launch whose entire compiled
  policy consists of paths nono does not own writes zero ACEs and is refused.
  This is the review's demand taken literally ("a launch that wrote ZERO
  NO_WRITE_UP ACEs" must not be reported established). In practice the
  workspace/cwd is always a user-owned writable grant, so `applied >= 1`; but
  this is a real behaviour change with a real (small) availability edge, and it
  is recorded rather than hidden.
- **`SkipNotOwned` counts as covered, not as a gap.** Counting it would fire
  the D-27 banner on essentially every claude-code-profile launch (the
  `system_read_windows` group grants read on `C:\Windows`), recreating the
  permanently-on warning that RF-06 removed — and nono structurally *cannot*
  label a path it does not own, so the skip is a documented D-02 exemption, not
  a failure. Recorded in the SPEC (RF-15) so the choice is auditable.
- **`dacl_ancestor_traverse` / `dacl_ancestor_read_attrs` still report guard
  existence.** Unlike the label and package-SID guards, these two have **no
  skip arm**: the walk grants on every owned ancestor and stops at the first
  non-owned one, which is the documented contract outcome (traversal from there
  up relies on lowbox bypass-traverse), so an empty grant set is a legitimate
  full-coverage result. `is_some()` genuinely IS the effect fact for them. The
  reasoning is written into `mod.rs:404-412` next to the code so a future
  reviewer can challenge it directly.
- **Semantic direction caveat.** The mandatory-label ACE nono writes *lowers*
  granted paths to Low IL so the confined child can reach them; confinement
  comes from the child being Low-IL against a Medium-IL world. So "zero ACEs
  written" is closer to "the child may be unable to use its grants" than to
  "the child is unconfined". The registry nevertheless declares this row
  `outcome: Abort` on all five `DirectCli` arms, and the structural defect the
  review found — a field that could not report a negative — was real either
  way. Flagged for the operator because it affects how the abort should be
  *read*, not whether the fix is right.

---

## NR-02 — `ProceedDowngraded` (and the whole D-27 channel) was unreachable

**Commit:** `59faa623`

### What changed

| File | Change |
|---|---|
| `crates/nono-cli/src/exec_strategy_windows/attestation.rs:519-540` | The `ContractOutcome::Abort` arm pushes `entry.id` into `downgraded` when `verdict.partially_established`. |
| `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:320-345` | `DegradeWithVisibleClaim`'s doc comment now states that it is NOT the only route to a downgrade and names the live one. |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs:3583-3652` | `partially_applied_launch_is_downgraded_not_silently_passed` — REAL registry, real `CREATE_SUSPENDED` job-contained child. |
| `.../attestation.rs:881-956` | `partially_applied_configured_only_row_proceeds_downgraded` (decision core, plus the D-26-tightened-still-aborts direction) and `some_production_row_can_still_produce_a_downgrade` (discovery-style, names no `LayerId`). |
| `proj/SPEC-windows-fail-direction-contract.md` | New RF-15/RF-16 sections + two review-fix table rows. `spec_matches_registry`, `registry_call_sites_exist`, `host_gated_rows_are_loud`, `security_assumptions_are_loud` all still pass. |

The channel is live because `labels_guard` produces `PartiallyApplied` from a
condition that occurs on real workspaces: a compiled-policy path that already
carries a mandatory label somebody else wrote.

### Observed counterfactual failure output

(a) Removed the `downgraded.push(entry.id)`:

```
---- exec_strategy::attestation::tests::partially_applied_configured_only_row_proceeds_downgraded stdout ----
assertion `left == right` failed: a partially established layer must downgrade the visible claim, not silently pass as the full baseline (CR-14) and not abort (the layer IS partly in effect)
  left: Proceed
 right: ProceedDowngraded { downgraded: [MandatoryIntegrityLabel] }
```

(b) Collapsed `RowVerdict::from_application`'s `PartiallyApplied` arm into
`Self::plain(confirmed_status)` (the pre-fix two-state behaviour) — both the
decision-core test and the REAL-registry gate test failed:

```
---- exec_strategy::launch::attestation_gate_tests::partially_applied_launch_is_downgraded_not_silently_passed stdout ----
a partially established layer must reach ProceedDowngraded through the REAL registry — otherwise the D-27 banner, dedup marker and audit event are dead code; got Ok(Proceed)

test result: FAILED. 0 passed; 2 failed
```

Restored; both pass.

### Honest limits

- No registry row was given a `DegradeWithVisibleClaim` outcome. That value
  remains unused; the downgrade arrives through `PartiallyApplied` on an
  `Abort`-outcome row instead. If the operator prefers the outcome-based route,
  it is a one-line registry change — but it would then be the *row* that is
  permanently degraded rather than the *launch*, which is not what D-27
  describes.
- `some_production_row_can_still_produce_a_downgrade` is weaker than the
  review's suggested version: it accepts "some `ConfiguredOnly` row is expected
  somewhere" as evidence the channel is reachable. The strong evidence is the
  two behavioural tests; this one is a cheap structural tripwire.

---

## NR-04 — the network rows derived `applied` and `wfp_preconfirmed` from the same discriminant

**Commit:** `3300e75a`

### What changed

| File | Change |
|---|---|
| `crates/nono-cli/src/exec_strategy_windows/mod.rs:282-320` | `NetworkEnforcementGuard::WfpServiceManaged` gains `installed_filter_count: u32`; `FirewallRules` gains `installed_rule_count: u8`; new `FIREWALL_RULES_REQUIRED = 2`. |
| `.../mod.rs:454-490` | New `firewall_rules_report()` (installation evidence) and `wfp_composition_report()` (composition only). |
| `crates/nono-cli/src/exec_strategy_windows/network.rs:1774-1800` | The WFP guard records the count the elevated service actually reported, captured where `assert_wfp_activation_installed_filters` reads it. `unwrap_or(0)` is the restrictive default. |
| `.../network.rs:1595-1640` | `installed_rule_count` is incremented as each `netsh` block rule is accepted. |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs:1538-1560` | `derive_wfp_preconfirmed` evaluates the recorded count, not the discriminant. |
| `.../launch.rs:3641-3745` | `wfp_row_can_be_selected_and_unconfirmed_from_a_real_guard_value` and `firewall_rules_row_reports_installation_evidence_not_the_discriminant`. |

The two facts are now sourced independently: **composition** ("was this the
backend this launch selected") from the guard's variant, **enforcement
evidence** from the number the enforcing component reported. The deny input is
now a `NetworkEnforcementGuard` value — the type the production caller actually
passes — instead of a hand-set `AppliedLayers` field.

### Observed counterfactual failure output

Reverted `derive_wfp_preconfirmed` to `matches!(.., WfpServiceManaged { .. })`
and `firewall_rules_report` to `Some(true)`:

```
---- exec_strategy::launch::attestation_gate_tests::firewall_rules_row_reports_installation_evidence_not_the_discriminant stdout ----
assertion `left == right` failed: one of two block rules installed is not the FirewallRulesEgress claim
  left: Some(true)
 right: Some(false)

---- exec_strategy::launch::attestation_gate_tests::wfp_row_can_be_selected_and_unconfirmed_from_a_real_guard_value stdout ----
zero installed filters is not enforcement evidence

test result: FAILED. 8 passed; 2 failed
```

Restored; 10/10 gate tests pass, including the end-to-end direction where a
`(selected, zero filters)` guard aborts at the real gate with
`Err(LayerAttestationFailed { layer: "WfpEgressFilters", reason: "Unconfirmed" })`.

### Honest limits

- On the **shipped** code path the `(selected, unconfirmed)` state still cannot
  arise, because the guard is only constructed after
  `assert_wfp_activation_installed_filters` succeeded fail-closed — i.e. the
  launch already aborted *earlier and harder*. Deliberately **not** "fixed" by
  relaxing that assertion; that would trade a pre-spawn fail-closed gate for a
  later one, which is strictly worse.
- What the change buys is therefore defence in depth, but real defence in
  depth: the attestation gate now performs a **second, independent read of the
  enforcing component's own report**, so a future construction site — a new
  early return, a loosened assertion, a stale-service response the assertion is
  later taught to tolerate — is caught at the gate instead of silently claiming
  enforcement. That property is exactly what the pre-fix "both facts from one
  discriminant" shape could not provide, and the counterfactual above shows the
  test detects its loss.

---

## NR-05 — the daemon's abort predicate was a compile-time constant `false`

**Commit:** `ec6bc660`

### What changed

| File | Change |
|---|---|
| `crates/nono-cli/src/agent_daemon/launch.rs:110-124` | New `DaemonDaclGuard::granted_write_access()`. |
| `.../launch.rs:727-736` | `let mut wfp_filters_installed = false;` — initialised to the restrictive value. |
| `.../launch.rs:768` | Set to `true` only where `wfp_filter_add` returned `Ok`. |
| `.../launch.rs:800-804` | `dacl_guard_applied` captured from the guard before it moves into the tenant. |
| `.../launch.rs:895-906` | The gate call passes `dacl_guard_applied` and `wfp_filters_installed` instead of the literal `true` and a second `network_scoping_required`. |
| `.../launch.rs:2062-2116` | `daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition`. |

A behavioural test cannot catch this defect: `daemon_attest_and_decide` is
itself correct and its unit tests pass distinct values. The bug lived entirely
in the production call site's argument list, so the new test asserts on that
wiring — it parses its own source (`include_str!("launch.rs")`), extracts the
gate call's six arguments, and requires that neither predicate input is a bool
literal and that the two are distinct expressions.

### Observed counterfactual failure output

(a) Restored the literal `true` for `dacl_guard_applied`:

```
---- agent_daemon::launch::tests::daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition stdout ----
dacl_guard_applied must be the guard's own report, not a literal: true
```

(b) Restored `network_scoping_required` for both predicate inputs:

```
assertion `left != right` failed: wfp_filters_installed and network_scoping_required must be independent values — passing the same expression for both makes the row's abort predicate `x && !x`, a compile-time constant false (NR-05)
  left: "network_scoping_required"
 right: "network_scoping_required"
```

Restored; the test passes and the full `nono-agentd` suite is 89/89.

### Honest limits

- On today's control flow both values are still `true` whenever the gate is
  reached, because `wfp_filter_add`'s `Err` path terminates the suspended child
  and returns, and `DaemonDaclGuard::apply`'s pass 2 is fail-closed. The
  difference is that the gate now *reads* those facts, and the source-text test
  makes reverting to the constant form fail the build. As with NR-04, the fix
  is not "make the negative occur today" — it is "make the negative
  representable and make its erasure loud".
- A source-text test is a blunt instrument: reformatting the call site into one
  line would break the argument parse (it asserts `args.len() == 6` with a
  clear message, so it fails loudly rather than silently passing).

---

## NR-06 — the staging-root guard did not reject `..` or canonicalize

**Commit:** `2837e44b`

### What changed

`crates/nono-cli/src/exec_strategy_windows/network.rs:180-290`
(`cleanup_network_enforcement_staging`) — three ordered defences:

1. **Reject any `Component::ParentDir` / `Component::CurDir` outright.** This
   is what closes the class. Canonicalization alone would not: a non-existent
   path cannot be canonicalized, so a traversal pointing at a path that does
   not yet exist would fall through the canonicalization arm.
2. **Keep the existing literal `Path::starts_with` component test** — per the
   brief, no regression to string `starts_with` (CLAUDE.md footgun #1).
3. **Canonicalize BOTH sides and re-apply the strict-subdirectory test.** This
   resolves symlinks and Windows directory junctions, covering the
   `cleanup_stale_network_enforcement_artifacts` → `read_dir` → junction vector
   the review raised. A canonicalization failure is fail-secure (refuse). The
   delete then targets the canonical path.

`network.rs:1990-2065` — `cleanup_refuses_traversal_the_root_and_paths_outside_the_root`.
Every refusal is asserted by **survival of a real on-disk victim tree**, never
by the absence of a log line, and it includes the positive direction (a genuine
staging subdirectory IS still removed) so "refuse everything" cannot pass.

### Observed counterfactual failure output

Reverted the function to the pre-fix literal-only guard:

```
---- exec_strategy::network::tests::cleanup_refuses_traversal_the_root_and_paths_outside_the_root stdout ----
a `..` traversal out of the staging root must be refused; C:\Users\OMack\AppData\Local\Temp\nono-nr06-victim was deleted

test result: FAILED. 0 passed; 1 failed
```

The counterfactual did not merely fail an assertion — it **actually recursively
deleted the victim directory**, demonstrating the hole was live and exploitable
via a mis-constructed guard. Restored; the test passes and the working tree
stayed clean (`git status --porcelain` showed only the intentional
modification).

### Symlink / TOCTOU analysis, as requested

- **Symlinks and directory junctions: covered.** `Path::canonicalize` resolves
  reparse points on Windows, and both sides are canonicalized before the
  subdirectory test, so a junction planted under `%TEMP%\nono-net-block` by
  anything with write access to `%TEMP%` resolves to its real target and is
  refused if that target is outside the root.
- **TOCTOU: NOT closed, and documented in the function's doc comment rather
  than silently accepted.** This is a check-then-act sequence; an attacker able
  to write inside `%TEMP%` could swap a real directory for a junction between
  `canonicalize` and `remove_dir_all`. Closing it properly requires a
  handle-based delete (`CreateFileW` with `FILE_FLAG_OPEN_REPARSE_POINT` plus
  `FILE_DISPOSITION_INFO`, walking the tree by handle rather than by path),
  which is a materially larger change than this finding warrants. The exposure
  is bounded: `%TEMP%` is per-user, so an attacker who can write there as this
  user already holds this user's privileges, and the delete runs as that same
  user. Recorded here and in the source so it is a named residual, not an
  unexamined one.
- **`remove_dir_all` itself** follows the canonical path; it does not re-resolve
  the caller's original `staged_dir`, which removes one indirection from the
  race.

---

## Process note

During the verification sweep I ran a malformed shell command that included
`git checkout 936d1ddb -- .`, which reverted the worktree's tracked files to
that older commit. It was detected immediately from `git status` and repaired
with a scoped `git checkout HEAD -- .`. No `git reset --hard`, `git clean` or
`git stash` was used; all five commits were already in history and were
unaffected. Post-recovery the tree matched `HEAD` exactly
(`git status --porcelain` empty, `git diff HEAD --stat` empty,
`cargo fmt --all -- --check` clean) and the fix content was re-confirmed by
grep before continuing.

---

_Fixed: 2026-08-10_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
