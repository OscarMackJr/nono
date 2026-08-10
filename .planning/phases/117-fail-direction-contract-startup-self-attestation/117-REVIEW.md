---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-10T00:00:00Z
iteration: 3
depth: deep
files_reviewed: 16
files_reviewed_list:
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/network.rs
  - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/tests/layer_force_unavailable.rs
  - crates/nono-cli/tests/layer_registry_meta_test.rs
  - crates/nono-cli/tests/layer_registry_selfcheck.rs
  - crates/nono-shell-broker/src/main.rs
  - crates/nono/src/error.rs
  - proj/SPEC-windows-fail-direction-contract.md
findings:
  critical: 1
  warning: 9
  info: 0
  total: 10
status: issues_found
---

# Phase 117: Code Review Report (iteration 3 — verification re-review of the 5-finding fix pass)

**Reviewed:** 2026-08-10
**Depth:** deep
**Files Reviewed:** 16 (the `d70b4658..HEAD` delta plus every call site and consumer it depends on)
**Status:** issues_found

## Summary

This pass is materially better than the previous one. The decisive claim of
CR-14 — *"a launch that wrote zero NO_WRITE_UP ACEs is reported fully
attested"* — is genuinely dead. I traced the whole data path
(`AppliedLabel::{SkipPreExistingLabel,SkipNotOwned,Applied}` →
`AppliedLabelsGuard::coverage()` → `LabelCoverage::application()` →
`AppliedLayers::mandatory_integrity_label` → `AppliedLayers::status()` →
`classify_row`'s `ConfiguredOnly` arm → `RowVerdict::from_application` →
`decide_from_entries`) and the all-skip case now lands on
`LayerApplication::NotApplied` → `LayerAttestationStatus::Unconfirmed` →
`AttestationDecision::Abort`. It is proven by a test that drives **real
filesystem state** (`labels_guard.rs:534`) rather than a synthetic
`AppliedLayers` value, and by a real-registry gate test against a live
`CREATE_SUSPENDED` job-contained child (`launch.rs:3530`). NR-02's
`ProceedDowngraded` is reachable from that same real production condition, and
it was made reachable by *detecting a real partial state*, not by lowering what
counts as confirmed and not by firing unconditionally. NR-06's staging guard
now rejects `..`/`.` **before** the literal test, keeps `Path::starts_with`
(never string `starts_with`), canonicalizes both sides, fails secure when
canonicalization fails, and is pinned by a test that asserts refusal via
survival of a real on-disk victim tree plus the positive direction.

The "guard construction relabelled as guard effect" move I was asked to watch
for **did not repeat wholesale — but it survives in two of four fields.**
`dacl_ancestor_traverse` and `dacl_ancestor_read_attrs` are still
`self._applied_ancestor_*.is_some()` (`mod.rs:413-414`), and
`execution_runtime.rs:608` sets `package_sid: Some(..)` unconditionally, so
both are the constant `Applied` on every shipped `DirectCli` launch and neither
row can ever deny. The reasoning is written next to the code and is defensible
on the merits; it is still the same shape CR-14 named, and it is recorded here
rather than accepted silently (NR3-02).

What the fix pass did *not* consider is the operational consequence of making
`applied == 0` a hard abort. `AppliedLabelsGuard` reverts labels in `Drop`, and
`Drop` does not run on a console Ctrl-C, a hard kill, or a crash. The labels
nono itself wrote then persist, so on the **next** launch every user-owned
policy path returns `Some(..)` from `low_integrity_label_and_mask`, records
`SkipPreExistingLabel`, and yields `applied == 0` → `NotApplied` → `Abort`. The
same thing happens to the second of two concurrent sessions over one workspace
(the per-tool-call hook path can run tools in parallel). There is no stale-label
sweep, no remediation on the error variant, and no CLI affordance to clear the
ACEs. nono locks itself out of its own workspace after its own abnormal exit.
That is NR3-01, and it is the one blocker.

Two of the five claimed fixes are **PARTIAL** for the same reason: NR-04 and
NR-05 both made the two facts independent *expressions*, but on every reachable
production control flow the negative still cannot occur, because an earlier and
harder gate has already aborted. The fix report says so plainly in its "honest
limits" sections and argues defence-in-depth, which is a reasonable position —
but the review's question was whether the deny direction is reachable in a
shipped build, and the answer is still no.

Separately, iteration 2 (and I, at first) accepted a claim from a source
comment that turns out to be fictional: `layer_registry.rs:686` says
`dacl_session_sid_grant_is_not_claimed_anywhere` "fails the build if anyone
re-adds an expectancy cell." That test does not exist anywhere in the tree, and
`AppliedLayers::status()` hardcodes `NotApplicable` for that `ConfiguredOnly`
row — so re-adding an expectancy cell would silently drop the row from every
decision, fail-open, with nothing to catch it (NR3-03).

I did not re-run the build or test suites; the verdicts below are from source
analysis of the delta and its consumers.

## Iteration 2 Fix Disposition

| ID | Verdict | Evidence |
|---|---|---|
| **CR-14** (BLOCKER) | **RESOLVED** (2 residuals → NR3-01, NR3-02) | The all-`Skip` path is now a hard deny: `labels_guard.rs:112-113` returns `NotApplied` when `applied == 0`, `attestation.rs:395-397` maps it to `Unconfirmed`, `attestation.rs:501-507` aborts. `guard_skips_path_not_owned_by_current_user` (`labels_guard.rs:735-742`) asserts the exact CR-14 scenario against `%SystemRoot%`. Residual: `mod.rs:413-414` still reports `is_some()` for the two ancestor-DACL rows. |
| **NR-02** | **RESOLVED** (residuals → NR3-04, NR3-06) | `attestation.rs:527-529` pushes a `partially_established` `Abort`-outcome row into `downgraded`; `LabelCoverage::application()` produces `PartiallyApplied` from a real, occurring condition (one policy path already carrying a third-party label), proven by `coverage_distinguishes_full_partial_and_zero_ace_launches` and by `partially_applied_launch_is_downgraded_not_silently_passed` against the REAL registry. The bar for `Confirmed`/`Applied` was not lowered and the banner is not unconditional. |
| **NR-04** | **PARTIAL** | The two facts *are* independent now — composition from the discriminant (`mod.rs:473-478`), evidence from `installed_filter_count` / `installed_rule_count` recorded where `netsh`/the WFP IPC returned (`network.rs:1696`, `:1714`, `:1880-1883`), consumed by `derive_wfp_preconfirmed` (`launch.rs:1548-1556`). But `assert_wfp_activation_installed_filters` (`network.rs:1859`) rejects `None`/`Some(0)` **before** the guard is constructed, and both `netsh` `Err` paths `return Err` before construction — so no production site can produce `(selected, unconfirmed)`. The deny input is a real guard *type*, but not a value any production caller emits. |
| **NR-05** | **PARTIAL** | `x && !x` is gone: `wfp_filters_installed` is a distinct local initialised to `false` and set only where `wfp_filter_add` returned `Ok` (`agent_daemon/launch.rs:728`, `:775`), and `dacl_guard_applied` reads `DaemonDaclGuard::granted_write_access()` (`:810`, `:122`). But `wfp_filter_add`'s `Err` path terminates and returns, and `DaemonDaclGuard::apply` pass 2 is fail-closed (`:212-225`), so both values remain dynamically identical to the gate condition on every reachable path — the daemon mirror's `WfpEgressFilters`/`DaclPackageSidGrant` deny direction is still production-unreachable. The new guard is a source-text test (see NR3-07). |
| **NR-06** | **RESOLVED** (residual → NR3-09) | `network.rs:240-252` rejects `Component::ParentDir`/`CurDir` first; `:255` keeps the literal `Path::starts_with` component test (no string ops — CLAUDE.md footgun #1 respected); `:268-276` canonicalizes BOTH sides and fails secure on error; `:277-286` re-applies the strict-subdirectory test to the canonical forms; `:288` deletes the canonical path. Symlinks/junctions are covered by canonicalization; the TOCTOU window is named in the doc comment (`:220-231`) rather than silently accepted. `cleanup_refuses_traversal_the_root_and_paths_outside_the_root` asserts every refusal by survival of a real victim tree **and** asserts the positive direction, so "refuse everything" cannot pass. |

## Still Open From Iteration 2 (routed to gap closure)

- **NR-03** — `AppliedLayers`'s three `Option<bool>` fields still default to `None` → `from_tristate(None)` → `NotApplicable` → the row is dropped from the decision. Not widened by this pass, and the type's doc comment was corrected to stop claiming otherwise (`layer_registry.rs:445-448`). The type-level fix (`NetworkBackendReport::{Unreported, NotSelected, Selected}`) is not done. Live consequence today: `broker_authenticode_trust_gate` stays `None` under `is_dev_build_layout()` (`launch.rs:1767`, `:2092`), which drops `BrokerAuthenticodeTrustGate` from the decision entirely on the two broker arms.
- **NR-07** — `RestrictedToken`'s `generate_session_sid()` → `ConvertStringSidToSidW` → token → `ConvertSidToStringSidW` round-trip still has **no** positive-direction test. `restricted_token_requires_this_launch_own_session_sid` (`attestation.rs:1275-1306`) asserts only the two negatives. A formatting divergence still refuses every default supervised Windows launch.
- **NR-08** — `.github/workflows/ci.yml` untouched. The `windows-layer-fault-injection` job still runs the whole `-p nono-sandbox-cli` suite with no `NONO_CI_HAS_WFP` and no pre-built `nono-shell-broker.exe`.
- **NR-09** — `crates/nono-shell-broker/src/main.rs` untouched. Both broker spawn shapes still pass `lpEnvironment: null` and the broker's own environment still carries `NONO_BROKER_REQUIRED_LAYERS`, so the confined grandchild can read the required-layer list (D-28 counterexample).
- **NR-10** — `manual_verification_section` (`layer_registry_meta_test.rs:337-341`) still builds its line-start iterator as `match_indices('\n').map(|(i,_)| i+1).chain(once(0))`, so index `0` is visited last and the iterator is not ascending.
- **WR-04-R** — `every_registry_row_has_a_test` still accepts a bare function name (`layer_registry_meta_test.rs:311-314`), and `MANUALLY_VERIFIED` is still 11 of 13 rows (`:151-273`). No cap was added.
- **WR-06-R** — **worsened again.** This pass added ~145 lines to `mod.rs` above the guard block. Registry citations vs. actual positions today: `labels_guard.rs:83` → `:176`; `mod.rs:425` → `:570`; `dacl_guard.rs:92` → `:182`; `mod.rs:436-440` → `:581-585`; `dacl_guard.rs:236` → `:364`; `mod.rs:449-455` → `:594-600`; `dacl_guard.rs:401` → `:540`; `mod.rs:473-482` → `:618-627`. `registry_call_sites_exist` only checks that the *file* exists. The SPEC repeats the stale numbers.
- **WR-12-R** — no test asserts that `nono-shell-broker`'s `"NONO_BROKER_REQUIRED_LAYERS"` literal (`main.rs:886`) equals `BROKER_REQUIRED_LAYERS_ENV_VAR` (`attestation.rs:117`). Verified absent from `layer_registry_selfcheck.rs`.

## Critical Issues

### NR3-01 (BLOCKER): stale mandatory labels from nono's own abnormal exit — or from a concurrent session — now hard-abort every subsequent launch, with no remediation and no way out

**File:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:106-118`,
`:190-206`, `:318-322`,
`crates/nono-cli/src/exec_strategy_windows/attestation.rs:395-397`, `:501-507`,
`crates/nono/src/error.rs:387-393`

**Issue:** CR-14's fix makes `applied == 0` on a non-empty policy a hard
`Abort`:

```rust
// labels_guard.rs:106
pub(crate) fn application(self) -> layer_registry::LayerApplication {
    if self.policy_paths == 0 { return LayerApplication::NotApplicable; }
    if self.applied == 0 { return LayerApplication::NotApplied; }   // <-- Abort
    ...
}
```

That is the right answer for the case the review demanded. But `applied == 0`
is not an exotic state — it is nono's own steady-state residue:

1. `snapshot_and_apply` records `SkipPreExistingLabel` for **any** path that
   already carries **any** mandatory-label ACE (`:193-206`), and
   `low_integrity_label_and_mask` returns `Some(..)` for exactly the Low-IL ACE
   nono itself writes via `try_set_mandatory_label` — this is asserted by
   `guard_apply_then_drop_reverts_label_for_fresh_file` (`:514`).
2. The only thing that clears those ACEs is `AppliedLabelsGuard::drop` →
   `revert_all` (`:318-322`). `Drop` does not run when the console delivers
   `CTRL_C_EVENT`, when the process is killed, or on a crash.
3. So after any abnormal exit, **every** user-owned path in the compiled policy
   carries a label nono wrote. On the next launch each of those records
   `SkipPreExistingLabel`; the remaining system paths record `SkipNotOwned`;
   `applied == 0`; `NotApplied` → `Unconfirmed` → `AttestationDecision::Abort`.
4. The same mechanism refuses the **second of two concurrent sessions** over one
   workspace. The module's own doc comment (`:17-18`) explicitly supports
   concurrent sessions ("last session out restores"); this change silently
   revokes that support. The Windows sandbox-the-tools hook path runs one
   `nono run` per tool call and Claude Code dispatches tool calls in parallel.

The operator sees `Startup self-attestation failed for layer
MandatoryIntegrityLabel: Unconfirmed` (`error.rs:387`). `NonoError::remediation()`
has **no arm** for `LayerAttestationFailed` (verified by grep), there is no
`nono` subcommand that clears mandatory labels (`clear_mandatory_label` is
private and reachable only from `best_effort_revert`), and there is no
stale-label sweep analogous to `cleanup_stale_network_enforcement_artifacts`.
The user's only recourse is to discover `icacls /setintegritylevel` themselves.

The fix report's "honest limits" section discloses the all-`SkipNotOwned`
variant of this and calls the availability edge "small". It does not consider
the pre-existing-label variant, which is the common one, is self-inflicted, and
is sticky.

**Fix:** three parts, all cheap.

1. Make nono's own residue self-healing rather than fatal. Record enough at
   apply time to recognise a label nono wrote (the mode-derived mask is already
   deterministic per `AccessMode`), and treat a matching pre-existing label as
   `Applied` — the ACE the layer's contract requires **is** present:

```rust
// labels_guard.rs, in the prior.is_some() arm
if let Some((prior_rid, prior_mask)) = prior {
    let wanted = label_mask_for_access_mode(rule.access);
    if prior_rid == LOW_INTEGRITY_RID && prior_mask == wanted {
        // The contracted ACE is already in place (our own residue, or an
        // identical concurrent session's). Covered, and NOT ours to revert.
        guard.entries.push(AppliedLabel::AlreadyAtRequiredLevel);
        continue;
    }
    guard.entries.push(AppliedLabel::SkipPreExistingLabel);
    continue;
}
```
   and count `AlreadyAtRequiredLevel` toward `applied` in `LabelCoverage`.

2. Add a `NonoError::remediation()` arm for
   `LayerAttestationFailed { layer: "MandatoryIntegrityLabel", .. }` naming the
   stale-label cause and the concrete `icacls`/`nono setup` command.

3. Add a regression test that pins the recovery: apply a guard, `mem::forget`
   it (simulating a killed process), then construct a second guard over the same
   policy and assert `application() != LayerApplication::NotApplied`.

## Warnings

### NR3-02 (WARNING): CR-14's defect survives for two of four `LayerApplication` fields — `dacl_ancestor_traverse` and `dacl_ancestor_read_attrs` are a constant `Applied` and can never deny

**File:** `crates/nono-cli/src/exec_strategy_windows/mod.rs:404-414`, `:439-445`,
`crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:385-421`, `:561-609`

**Issue:** `applied_layers()` derives these two from `is_some()`:

```rust
dacl_ancestor_traverse: application_of(self._applied_ancestor_traverse.is_some()),
dacl_ancestor_read_attrs: application_of(self._applied_ancestor_read_attrs.is_some()),
```

Both `Option`s are `config.package_sid.as_deref().map(..).transpose()?`
(`mod.rs:594-627`), and `execution_runtime.rs:608` sets
`package_sid: Some(windows_package_sid)` unconditionally — the same fact WR-09's
fix leans on. So on every shipped `DirectCli` launch both fields are the literal
`Applied`. Both rows are `expected: true` on all five `DirectCli` arms
(`DACL_PACKAGE_SID_SCOPED_EXPECTANCY` / `DACL_ANCESTOR_READ_ATTRS_EXPECTANCY`)
with `outcome: Abort`, and neither can ever produce a negative in a shipped
build. Ask the iteration-3 question directly — *what input makes this row report
NOT established?* — and the answer is "none".

The written justification (`mod.rs:404-412`) is that these guards have no skip
arm, so an empty grant set is legitimate full coverage. That is true of the walk
itself, but it is an argument about the guard, not about the report: the report
still reads object existence, and it would keep reading `Applied` if the walk
were later given a skip arm. The two guards *do* already distinguish outcomes
internally (`applied: Vec<PathBuf>` plus the "stopped at first non-owned
ancestor" break), so a real coverage accessor costs nothing.

**Fix:** give both guards the same accessor shape the other two now have, so the
report is a fact rather than an existence check:

```rust
// dacl_guard.rs
impl AppliedAncestorTraverseGuard {
    /// `NotApplicable` when the walk had nothing owned to grant (the
    /// documented bypass-traverse case), `Applied` otherwise.
    pub(crate) fn application(&self) -> layer_registry::LayerApplication {
        if self.applied.is_empty() {
            layer_registry::LayerApplication::NotApplicable
        } else {
            layer_registry::LayerApplication::Applied
        }
    }
}
// mod.rs
dacl_ancestor_traverse: self._applied_ancestor_traverse
    .as_ref()
    .map_or(layer_registry::LayerApplication::NotApplied,
            dacl_guard::AppliedAncestorTraverseGuard::application),
```

and delete `application_of`, whose only two callers are these lines.

---

### NR3-03 (WARNING): `layer_registry.rs` cites a build-failing guard test that does not exist, and the invariant it claims to protect is genuinely unguarded and fail-open

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:685-698`,
`:511-527`, `:1129-1161`

**Issue:** The `DACL_SESSION_SID_GRANT_EXPECTANCY` doc comment states:

> The `LayerId` variant is retained (Phase 118 receipts share this vocabulary)
> and `dacl_session_sid_grant_is_not_claimed_anywhere` below fails the build if
> anyone re-adds an expectancy cell without also making the grant real.

`grep -rn dacl_session_sid_grant_is_not_claimed_anywhere crates/` returns
exactly one hit: that comment. **The test does not exist.** Iteration 2's
disposition table cited this same non-existent test as part of accepting CR-05
as `ACCEPTED_OPEN`.

The gap is not cosmetic. `DaclSessionSidGrant`'s row carries
`probe: ProbeKind::ConfiguredOnly` and `outcome: Abort`, but
`AppliedLayers::status()` hardcodes it to `LayerApplication::NotApplicable`
(`:514-526`). `classify_row`'s `ConfiguredOnly` arm feeds that straight into
`RowVerdict::from_application`, which returns
`plain(LayerAttestationStatus::NotApplicable)` — and `decide_from_entries`
`continue`s past it (`attestation.rs:491-495`). So if anyone re-adds an
expectancy cell (the RF-03 open operator decision explicitly contemplates
restoring this grant), the row is silently **dropped from every decision**
rather than aborting — a fail-open, in the module whose purpose is truthful
layer accounting.

The new `every_reported_layer_is_actually_consulted` test only covers the
forward direction (reported ⇒ consulted). The dangerous direction is the
converse: a `ConfiguredOnly` / `ConfirmedByEnforcingComponentReport` row that
`status()` maps to a hardcoded `NotApplicable` while carrying a non-empty
expectancy. Nothing checks it.

**Fix:** write the test the comment already promises, plus its converse, both
discovery-style:

```rust
#[test]
fn dacl_session_sid_grant_is_not_claimed_anywhere() {
    let row = all_entries().iter().find(|e| e.id == LayerId::DaclSessionSidGrant)
        .expect("row exists");
    assert!(row.expectancy.is_empty(),
            "DaclSessionSidGrant expectancy was re-added, but AppliedLayers::status() still \
             hardcodes NotApplicable — the row would be silently dropped (fail-open). Add a \
             reported field in the same commit.");
}

#[test]
fn every_consulting_row_with_an_expectancy_has_a_reported_field() {
    let all_applied = AppliedLayers { /* everything positive */ };
    for entry in all_entries() {
        let consults = matches!(entry.probe,
            ProbeKind::ConfiguredOnly | ProbeKind::ConfirmedByEnforcingComponentReport);
        let expected_somewhere = entry.expectancy.iter().any(|a| a.expected);
        if consults && expected_somewhere {
            assert_ne!(all_applied.status(entry.id), LayerApplication::NotApplicable,
                "{:?} is expected somewhere and its probe consults AppliedLayers, but status() \
                 can only ever return NotApplicable — the row is unattestable by construction",
                entry.id);
        }
    }
}
```

---

### NR3-04 (WARNING): the now-live D-27 banner points the operator at diagnostic output that the CLI gate never produces

**File:** `crates/nono-cli/src/output.rs:115-124`,
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1480-1524`,
`crates/nono-cli/src/agent_daemon/launch.rs:965-972`

**Issue:** NR-02 made this channel fire for the first time, so its wording is
now load-bearing. The banner reads:

> `{n} confinement layer(s) could not be fully confirmed at startup — see diagnostic output for details`

but `apply_startup_attestation_gate`'s `ProceedDowngraded` arm emits **no**
operator-visible diagnostic naming the layers. It emits the audit event via
`SECURITY_LAYER.emit_attestation_event(&downgraded_refs)` and then the banner —
and the only `tracing::warn!` carrying `downgraded_layers={dedup_key}` fires on
the *failure* paths (`:1498`, `:1507`), i.e. the detail is visible only when the
audit emission breaks. An operator who follows the banner's instruction finds
nothing.

The daemon mirror does the opposite: `agent_daemon/launch.rs:965-972` emits
`tracing::warn!(downgraded_layers = %dedup_key, ...)` unconditionally. Two
mirrors, two different operator surfaces for the same state.

This is not a D-28 conflict: D-28 forbids layer detail on channels the confined
process can read (stderr, which the child shares), and `tracing` at the CLI
routes to the log file / Event Log, not to the child's stderr — which is exactly
why the daemon can do it.

**Fix:** emit the detail on the channel the banner points at, matching the
daemon:

```rust
tracing::warn!(
    downgraded_layers = %dedup_key,
    downgraded_count = downgraded.len(),
    "startup self-attestation: proceeding with a downgraded confinement claim"
);
crate::output::print_attestation_downgrade_banner(downgraded.len(), session_id, &dedup_key);
```

or, if that is deliberately withheld, change the banner text to name where the
detail actually lives (the audit ledger) instead of a channel that carries
nothing.

---

### NR3-05 (WARNING): the daemon mirror has no downgrade state at all — `DaemonAttestationDecision::ProceedDowngraded` is never constructed, and its DACL fact is a bool that cannot express partial coverage

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:1378-1432`, `:915-973`,
`:110-124`, `:158-195`

**Issue:** `daemon_attest_and_decide` returns only `Abort` (four sites) or
`Proceed` (one site). `DaemonAttestationDecision::ProceedDowngraded` has **no
constructor anywhere** (verified by grep), so the ~60-line arm at `:915-973` —
its audit emission, its dedup key, its Event Log warning — is dead code in the
shipped daemon binary. This is exactly the NR-02 defect, in the mirror NR-02 did
not touch: the CLI core now has three outcomes and the daemon has two.

It is also a coverage divergence, not just a plumbing one.
`DaemonDaclGuard::apply` pass 1 *skips* non-owned read-only paths with a warning
(`:177-183`) and pass 3 *stops* at the first non-owned ancestor (`:242-250`) —
the same conditions that CR-14 taught the CLI to report as `PartiallyApplied`.
The daemon collapses all of it into `granted_write_access() -> bool`
(`:120-123`), which only inspects `write_applied` and is `true` whenever pass 2
succeeded. Review priority #4 asks for the three mirrors' fail directions to
match; they do not.

**Fix:** either give `DaemonDaclGuard` a coverage accessor and route it into a
real `ProceedDowngraded`, mirroring `DaclGrantCoverage::application()`, or
delete the `ProceedDowngraded` variant and its arm and state in the daemon's doc
comment that the daemon path is abort-or-proceed by design. Add a discovery-style
conformance test that fails when one mirror gains a decision state the other
lacks.

---

### NR3-06 (WARNING): `some_production_row_can_still_produce_a_downgrade` cannot fail for the reason it names

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:936-950`

**Issue:** The test's message claims it guards against "no registry row can ever
reach `ProceedDowngraded`", but its predicate inspects only registry *data*:

```rust
matches!(e.outcome, DegradeWithVisibleClaim | FailOpenDefect { .. })
  || (matches!(e.probe, ProbeKind::ConfiguredOnly)
      && e.expectancy.iter().any(|arm| arm.expected))
```

Seven rows carry `ProbeKind::ConfiguredOnly` with a non-empty expectancy, so the
disjunct is satisfied regardless of the mapping under test. Collapse
`RowVerdict::from_application`'s `PartiallyApplied` arm into
`Self::plain(confirmed_status)` — the exact pre-fix behaviour the fix report
used as its own counterfactual — and this test still passes. It also cannot see
that two of those seven rows (`FirewallRulesEgress`,
`BrokerAuthenticodeTrustGate`) are `Option<bool>`-sourced and therefore
*structurally incapable* of reporting `PartiallyApplied`.

The two behavioural tests are genuine; this one is a tripwire that is not
connected to anything.

**Fix:** assert on the mapping, not on the data:

```rust
#[test]
fn some_production_row_can_still_produce_a_downgrade() {
    let downgradable = layer_registry::all_entries().iter().any(|e| {
        let arm = e.expectancy.iter().find(|a| a.expected);
        arm.is_some() && {
            let v = RowVerdict::from_application(
                layer_registry::LayerApplication::PartiallyApplied,
                LayerAttestationStatus::EstablishedNotIndependentlyObservable);
            v.partially_established
        }
    });
    assert!(downgradable, "…");
}
```

or, better, drive `decide_from_entries` over `all_entries()` with a
`PartiallyApplied` report and assert the result is `ProceedDowngraded`.

---

### NR3-07 (WARNING): the NR-05 wiring test parses the first textual occurrence of its own search string and pins a textual, not semantic, invariant

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:2074-2113`

**Issue:** The test does `include_str!("launch.rs").split("match daemon_attest_and_decide(").nth(1)`.
That literal occurs **twice** in the file: once at the real call site (`:899`)
and once inside the test itself (`:2078`). It works today only because the call
site happens to appear first. Move the test module above the `windows_impl`
module, add a doc-comment example containing the same phrase, or introduce a
second gate call site, and the parse silently re-targets — `args.len() == 6`
would then fail with "unexpected call shape", which is at least loud, but the
test is one edit away from testing the wrong text.

Second, the invariant is purely lexical. `assert_ne!(wfp_filters_installed,
network_scoping_required)` compares *identifier strings*. A future edit to
`let wfp_filters_installed = network_scoping_required;` restores the exact
`x && !x` defect and this test passes green — the two argument tokens differ.

**Fix:** anchor the split on a stable, unique marker the test itself cannot
contain, and add the semantic half as a behavioural assertion:

```rust
// at the call site
// ATTESTATION-GATE-CALL (NR-05 wiring anchor — do not remove)
let call = src.split("ATTESTATION-GATE-CALL").nth(2).expect("anchor comment + call");
```

plus a test that drives `daemon_attest_and_decide(.., true, true, false)` and
asserts `Abort { layer: "WfpEgressFilters", .. }`, so the predicate's deny
direction is pinned behaviourally rather than by token comparison.

---

### NR3-08 (WARNING): registry call-site citations drifted a further ~145 lines in this pass, and the SPEC — which declares itself the source of truth — was not updated for NR-04/NR-05/NR-06 and still describes the pre-NR-06 staging guard

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:866-870`,
`:937-942`, `:950`, `:958`, `:966-971`,
`proj/SPEC-windows-fail-direction-contract.md:263`

**Issue:** Two related drifts, both created by this pass.

1. `mod.rs` grew from ~380 to ~525 lines above the guard block, so every
   `mod.rs:NNN` citation moved again. Measured against the current tree:
   `labels_guard.rs:83` → actual `:176`; `mod.rs:425` → `:570`;
   `dacl_guard.rs:92` → `:182`; `mod.rs:436-440` → `:581-585`;
   `dacl_guard.rs:236` → `:364`; `mod.rs:449-455` → `:594-600`;
   `dacl_guard.rs:401` → `:540`; `mod.rs:473-482` → `:618-627`. This is the
   third consecutive pass in which WR-06 has been deferred and the numbers have
   moved further from truth.

2. The SPEC gained RF-15/RF-16 (CR-14, NR-02) but **no** row for NR-04, NR-05 or
   NR-06, and its existing RF-14 row still says the guard "refuses any path that
   is not a strict subdirectory of `%TEMP%/nono-net-block`, compared by path
   components" — which is the pre-NR-06 description. D-01 says the SPEC is
   drift-checked against the registry; `spec_matches_registry` only checks
   `LayerId` names, so none of this is caught.

**Fix:** cite stable symbols instead of lines
(`"mod.rs::prepare_live_windows_launch/AppliedLabelsGuard::snapshot_and_apply"`)
and make `registry_call_sites_exist` assert the file *contains the symbol*; if
that is deferred again, re-run the numbers before shipping. Add the three
missing SPEC rows and update RF-14 to describe the three-defence guard.

---

### NR3-09 (WARNING): the NR-06 test mutates shared, real `%TEMP%` state and depends on the process CWD, so it is parallel-unsafe and its strongest assertion is environment-dependent

**File:** `crates/nono-cli/src/exec_strategy_windows/network.rs:1993-2065`

**Issue:** The test creates and writes into the **real**
`network_enforcement_staging_root()` (`%TEMP%\nono-net-block`) and a real
`%TEMP%\nono-nr06-victim`, both shared process-wide. `cargo test` runs the
`nono` bin's unit tests in parallel threads, and
`cleanup_stale_network_enforcement_artifacts` (`:291-304`) enumerates that same
root and calls `cleanup_network_enforcement_staging` on every entry — so a
concurrent test or a concurrent real `nono` session on the same host can delete
`nono-nr06-legit-staging` before or after this test's own call, making the
positive-direction assertion (`!legit.exists()`) pass or fail for the wrong
reason. The test also leaves the staging root behind on exit.

`assert!(Path::new("Cargo.toml").exists())` (`:2011`) additionally depends on
cargo running the test with the package root as CWD; under a different runner it
asserts nothing.

Separately, the two new coverage accumulators use unchecked `+=`
(`labels_guard.rs:276-280`, `dacl_guard.rs:277-282`) while
`DaclGrantCoverage::writable_rules` correctly uses `saturating_add`
(`dacl_guard.rs:132`) — an inconsistency with CLAUDE.md's arithmetic rule inside
one struct.

**Fix:** point the test at an isolated root. Extract
`cleanup_network_enforcement_staging_under(root, staged_dir)` and have the
production wrapper pass `network_enforcement_staging_root()`, so the test can
drive a `tempdir()` root with no shared state and no CWD dependency. Use
`saturating_add(1)` for the two accumulators.

---

### NR3-10 (WARNING): `attestation.rs`'s module doc still asserts a claim that fix pass 1 invalidated

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:62-72`

**Issue:** Deviation 2's justification reads:

> a literal reading … would abort **every** ordinary Windows launch that expects
> any `ProbeKind::ConfiguredOnly` row with `outcome: Abort` (`DaclSessionSidGrant`,
> `DaclPackageSidGrant`, …) … That is not a per-row edge case;
> `DaclSessionSidGrant` alone is expected on every `WriteRestricted`-arm launch,
> nono-cli's default supervised path.

`DaclSessionSidGrant`'s expectancy has been empty since CR-05's fix
(`layer_registry.rs:698`), so it is expected on **no** arm. The paragraph is the
load-bearing rationale for the one place this module deliberately does *not*
abort, and its central example is now false — a reader auditing the
downgrade-instead-of-abort decision is handed a premise that no longer holds.

**Fix:** re-derive the example from the rows that are actually expected today
(`DaclPackageSidGrant` on all five `DirectCli` arms is the correct
substitute), and add a note that CR-14 narrowed the deviation: it now applies
only to `LayerApplication::Applied`, since `NotApplied` aborts and
`PartiallyApplied` downgrades.

---

_Reviewed: 2026-08-10_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep — iteration 3 (verification re-review)_
