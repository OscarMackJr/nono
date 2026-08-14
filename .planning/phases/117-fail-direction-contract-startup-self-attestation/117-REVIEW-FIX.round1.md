---
phase: 117-fail-direction-contract-startup-self-attestation
fixed_at: 2026-08-14T04:05:00Z
review_path: .planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md
iteration: 6
findings_in_scope: 17
fixed: 13
partial: 1
skipped: 3
status: partial
---

# Phase 117: Code Review Fix Report — iteration 6

**Source review:** `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md`
(reviewed 2026-08-13T16:27:04Z; 3 Critical + 14 Warning = 17 findings, all in scope)
**Scope:** `critical_warning` — the review reported 0 Info findings, so this is every finding.

> The iteration-2 report previously at this path is preserved in git history (it covered a
> different review round: CR-14, NR-02, NR-04, NR-05, NR-06). This file replaces it, following
> the precedent that file itself set for the iteration-1 report.

**Summary:**

| | Count | Findings |
|---|---|---|
| Fixed | 13 | CR-01, CR-03, WR-01, WR-02, WR-03, WR-04, WR-05, WR-06, WR-07, WR-08, WR-09, WR-11, WR-13 |
| Partial | 1 | CR-02 |
| Skipped (recorded, not fixed) | 3 | WR-10, WR-12, WR-14 |

17 commits on `gsd-reviewfix/117-189`, fast-forwarded onto
`milestone/v2.13-carryforward-closeout`. All carry DCO sign-off.

---

## Verification posture

Every fix was verified by re-reading the change, running the affected tests, and — for every
guard, gate or predicate — **perturbing a production site to prove the guard actually denies**.
A guard that passed both before and after was not accepted as fixed. Perturbation evidence is
recorded per finding below and in each commit message.

**Whole-tree verification (final state):**

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| Windows host `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | PASS (exit 0) |
| `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS (exit 0)**, 29m02s cold container |
| `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (SDKROOT unset) | **PASS (exit 0)**, 4m53s |
| `cargo test --workspace` | see regression baseline below |

Both cross-target gates were run **locally and to completion** — no PARTIAL→CI fallback was
used. This was required: WR-02 touches `bindings/c/src/`, and WR-09 touches
`exec_strategy_windows/`.

### Regression baseline (no regressions)

`cargo test --workspace` ends with 14 failures, all in `-p nono-sandbox-cli --bin nono`. I ran
the **same target at the phase base commit `334530af`** in a throwaway worktree:

| | Passed | Failed |
|---|---|---|
| Phase base `334530af` | 1641 | 14 |
| This branch | 1646 | 14 |

The 14 failure **names are identical** in both runs (`config::tests::*` HOME/USERPROFILE env
races, `protected_paths::tests::*`, `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`,
`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`,
`exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`,
`exec_strategy::launch::broker_dispatch_tests::broker_launch_assigns_child_to_job_object`,
`exec_strategy::launch::write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file`).
These are the known Windows-host baseline failures. **Zero regressions; +5 net passing tests.**

Every other target is fully green: `nono-sandbox` lib 844, `nono-ffi` 51,
`layer_registry_selfcheck` 14, `layer_registry_meta_test` 10, plus 40/18 in the remaining
targets.

### Recommend human confirmation (runtime-behaviour changes)

Three fixes change runtime behaviour rather than only tests or docs. Tests and perturbations
pass, but semantics deserve a human read:

- **CR-03** — `MachineEgressPolicy.required_layers.required` is now populated from the registry.
  Confirmed `validate()` does not inspect the field (so a typo still cannot abort the policy
  read, preserving D-26 degrade-not-abort) and `is_unconfigured()` already ignores it (so a
  sentinel-only key cannot flip to "configured"). No consumer reads it yet, so shipped behaviour
  is unchanged — but that is the whole point of WR-12 remaining open.
- **WR-08** — `classify_row` now selects enforcing-component evidence by `LayerId`; a future row
  with `ConfirmedByEnforcingComponentReport` and no evidence channel classifies `Unconfirmed`
  (fail-closed) instead of inheriting WFP's answer. No behaviour change today (exactly one row
  has that probe kind, pinned by a new test).
- **WR-04** — the downgrade-banner dedup marker is now content-authoritative. The banner will
  print once more per session on a toolchain bump that changes `DefaultHasher`, and prints
  instead of suppressing on any marker mismatch. Fail direction is toward MORE visibility.

---

## Fixed

### CR-01: The WR-26 class gate matched zero sites in the file it primarily targets

**Files:** `crates/nono-cli/src/output.rs`
**Commits:** `5590a8fe`, plus a correction folded into `1400d77a` (see below)

Fixed both compounding causes. `production()` no longer ends at `l.trim() != "mod tests {"` (a
line `launch.rs` does not contain, so the scan silently ran the whole 5862-line file), and the
needle is matched against continuation-joined, whitespace-normalised lines instead of raw source.
Added a `hits >= 2` non-vacuity assertion.

**Perturbation (2 of 2 required, both fail only after the fix):**
- Hoisting an event-log pointer into the unconditional site-1 `warn!` in
  `emit_downgrade_diagnostics` — *the exact regression CR-01 states the old gate could not deny*
  — now FAILS: `launch.rs:1454 names the Windows Application event log but is not inside a
  DowngradeDetailChannel::EventLog arm ... Nearest arm found: ""`.
- Breaking the needle in both production sites now FAILS:
  `the WR-26 class gate matched 0 site(s)`.

**Residual defect in my own first fix, caught by WR-06's non-vacuity assertion and corrected in
`1400d77a`:** ending the scan at the first top-level `#[cfg(test)]` reduced the `main.rs` half to
NOTHING, because `main.rs:155` is a bare `#[cfg(test)] mod test_env;` *declaration* 130 lines
above the code the gate must see; and `output.rs`'s first test module is gated on
`#[cfg(all(test, target_os = "windows"))]`, which an exact match never saw. `production()` now
skips cfg-test-gated **inline** modules by brace region, which has neither failure mode. This is
worth recording: the first fix would have shipped a second silently-vacuous half.

### CR-03: `RequiredLayers` is parsed and then unconditionally discarded

**File:** `crates/nono/src/machine_policy.rs` · **Commit:** `ec78f8be`

`warn_if_required_layers_configured` → `read_required_layers`, now carrying the parsed names.
The control remains unenforced, but the gap moved to the single **consumer** (the launch gate),
one `grep machine_required_layers` away, instead of being hidden three files away in a parser
return. This removes the fail-open landmine the review identified: wiring
`machine_required_layers: policy.required_layers.required` would previously have compiled, passed
every test, looked wired, and still passed an empty slice.

**Perturbation:** restoring the `RequiredLayersPolicy::default()` discard makes
`windows_required_layers_round_trip_is_not_silently_dropped` FAIL; restoring the fix passes. All
38 `machine_policy` tests green.

### WR-01: Operator-facing literals contain 14-22-space runs from a bad automated edit

**Files:** `output.rs`, `exec_strategy_windows/{launch,attestation,attestation_downgrade_event}.rs`
**Commit:** `c34f569e`

Fixed by **class, not by the 5 cited sites**: a whole-workspace string-literal scan found 12
affected literals across 4 files on the D-27 surface (and correctly excluded look-alikes such as
`network.rs`'s `sc query` output fixtures, which are legitimately column-aligned). The `EventLog`
arm of `downgrade_detail_pointer` is deliberately re-wrapped so the needle stays intact on one
source line — the CR-01 gate scans line by line.

Added `no_downgrade_surface_literal_has_a_collapsed_continuation`, which rejects a run of 4+
spaces preceded by a non-space character (admitting the legitimate leading-indentation literals
`output.rs` uses by design) and asserts `checked >= 500` literals scanned.

**Perturbation:** re-collapsing `launch.rs:1505` FAILS with
`14 consecutive spaces mid-literal`; restoring passes.

### WR-02: `bindings/c` silently collapses `LayerAttestationFailed` to `Other`

**Files:** `bindings/c/src/types.rs`, `bindings/c/include/nono.h` · **Commit:** `8cfde970`

Added `LayerAttestationFailed = 15` plus its `From` arm; `nono.h` regenerated by
build.rs/cbindgen. Value appended after `Cancelled = 14`, so no existing ABI value moves. The
mandatory `_ => Self::Other` wildcard now carries a comment stating plainly that it swallows
every future addition (`nono::NonoDiagnosticCode` is `#[non_exhaustive]`, so no exhaustiveness
error is possible) and that a new code needs its own arm **and** its own round-trip assertion.

Two tests, so the positive cannot pass for the wrong reason:
`layer_attestation_failed_is_not_collapsed_to_other` (≠ `Other`, = the right variant, ABI value
15) and the control `other_still_maps_to_other`. All 51 `nono-ffi` tests green.

Cross-target verified (required — `bindings/c/src/`): linux-gnu PASS, apple-darwin PASS.

### WR-03: `ClearStaleLayerResidue`'s library doc prescribed the remedy `main.rs` proved wrong

**File:** `crates/nono/src/diagnostic/codes.rs` · **Commit:** `b6cf447f`

Removed the `icacls /setintegritylevel Medium` example (which writes a Medium label and
re-triggers the same abort) and pointed at `render_error_for_operator` as the single source, with
the reason recorded so it cannot be helpfully re-added. Swept the class: every remaining
`/setintegritylevel Medium` mention in the tree is corrective or negated, none prescriptive.

### WR-04: The "non-silenceable" D-27 banner is silenceable by a pre-planted marker

**File:** `crates/nono-cli/src/output.rs` · **Commit:** `e2cfe36e`

Took the review's option (b). The decision is extracted into
`marker_says_already_announced(path, key)` and now requires an exact stored-key match; absent,
zero-byte, different-content (a genuine 64-bit collision between two different downgraded-layer
sets) and unreadable all resolve to ANNOUNCE. The writer stores the key verbatim with
`create_new(true)`.

**Honest limit, stated in the function doc rather than papered over:** this does NOT stop a
same-user process from pre-planting a *correct* marker. Accepted, with reasoning — such a process
already holds strictly greater capability (it can edit nono's config or shadow `nono` on `PATH`),
and the audit event and `tracing::warn!` on the same path are independent channels it does not
control. "Unconditional" is redefined precisely: *not gated on `--silent`, not suppressed by any
nono code path* — **not** tamper-proof.

**Perturbation:** reverting the predicate to `path.exists()` FAILS on the zero-byte case. The
test also asserts the positive direction (an exact match must still suppress), or dedup would do
nothing and the hook path would re-print on every tool call.

### WR-05: `every_call_site_string_names_a_line_number` no longer checks what its name asserts

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` · **Commit:** `6b2b47e8`

Renamed to `every_call_site_string_is_symbol_form`, asserting `.rs::`, with a `checked >= 20`
non-vacuity bound covering the two rows that legitimately carry `call_sites: &[]`.

**Perturbation:** downgrading one real entry to `"restricted_token.rs:86"` (which the OLD
assertion accepted) now FAILS.

### WR-06: The WR-27 abort-path guard's predicate is narrower than the class it names

**Files:** `crates/nono-cli/src/output.rs`, `crates/nono-cli/src/main.rs` · **Commit:** `1400d77a`

Both mirrors now assert the CLASS ("mentions the event log") and exclude the legitimate mention
by requiring the negation, rather than narrowing to one verb. Both state the **same** rule — this
phase's memory records two plans shipping contradictory rules for one condition with every gate
green — and both carry non-vacuity assertions.

**Perturbation:** rewording `main.rs:305`'s negation to `check the Windows Application event log;
no record is written on this path` — a phrasing the OLD needle permitted — now FAILS **both**
guards (`output.rs`'s source-text mirror at `main.rs:303`, and
`render_error_for_operator_names_a_reachable_channel_for_non_label_layers` on the rendered
string).

### WR-07: `layer_force_unavailable.rs` claims three automated rows; two exist

**Files:** `crates/nono-cli/tests/layer_force_unavailable.rs`, `.../layer_registry_meta_test.rs`
**Commit:** `65691203`

Counts corrected to 2 + 8 `ALSO_AUTOMATED` + 3 `MANUALLY_VERIFIED` = 13 (the doc had also
omitted the `ALSO_AUTOMATED` bucket entirely). Added
`coverage_split_accounts_for_every_layer_id`, which counts **definition lines** (not mentions,
which would inflate the total exactly when the functions went missing) and additionally asserts
the two lists are **disjoint** — arithmetic alone would let a double-counted row hide a genuinely
uncovered one.

**Perturbation:** renaming `force_unavailable_dacl_package_sid_grant` FAILS with
`defines 1 fn force_unavailable_* test(s) ... LayerId::ALL has 13`.

### WR-08: `wfp_preconfirmed` is a WFP-specific fact consumed by a probe-kind-generic branch

**Files:** `exec_strategy_windows/attestation.rs`, `.../layer_registry.rs` · **Commit:** `786a7887`

Went beyond the review's stated minimum. The evidence is now selected by `match entry.id`, so any
other row with that probe kind fails **closed** (`Unconfirmed`) rather than inheriting WFP's
answer — fail-secure per CLAUDE.md in preference to the convenient wildcard. Because fail-closed
alone would be a silent trap, `exactly_one_row_is_confirmed_by_enforcing_component_report` makes
adding a second row a build failure whose message says exactly what to do (give it its own
evidence channel and arm; do NOT widen the WFP arm).

**Perturbation:** giving `FirewallRulesEgress` that probe kind FAILS with
`Rows carrying this probe kind today: [WfpEgressFilters, FirewallRulesEgress]`. All 74
attestation tests green.

### WR-09: No CI job runs clippy on Windows

**Files:** `.github/workflows/ci.yml`, `exec_strategy_windows/launch.rs` · **Commit:** `b8a4e5fc`

Added `windows-latest` to the clippy matrix. **The gate immediately caught a real, pre-existing
defect** — I ran the exact command locally before adding the leg and it FAILED:
`apply_startup_attestation_gate`'s doc comment had been orphaned ~130 lines above its function
(separated by a blank line and two helpers inserted between them), so it documented
`emit_downgrade_diagnostics` instead. Confirmed pre-existing by diffing against the phase base
`334530af`, not introduced by this pass. `clippy::empty_line_after_doc_comments` flagged it; the
block is moved back onto its function here, because the CI leg cannot be added while the gate is
red. Re-verified: exit 0.

### WR-11: `session_id_is_safe_path_component`'s doc overstates what it excludes

**File:** `crates/nono-cli/src/output.rs` · **Commit:** `634f2a9e`

Added the exclusion rather than weakening the claim — rejection is itself the safe direction here
(`None` means "no dedup marker", so the banner prints more, never less). ASCII-case-insensitive,
because the Win32 device namespace is; `COM0`/`LPT0` included.

The test covers the whole class (12 names × 3 casings) **plus a control set** (`CONSOLE`, `NULL`,
`COM10`, `con-1`, ...) that must still be accepted — an over-broad exclusion would silently
disable dedup for legitimate session ids, and nothing else would catch that.

**Perturbation:** replacing the `RESERVED` check with a tautology FAILS on `"CON"`.

### WR-13: `LayerApplication::NotApplied` is structurally unreachable for `WfpEgressFilters`

**Files:** `exec_strategy_windows/mod.rs`, `.../layer_registry.rs` · **Commit:** `4e4a7800`

This is a **doc-accuracy defect only** — the substantive deny direction is already exercised
end-to-end by `launch.rs::wfp_row_can_be_selected_and_unconfirmed_from_a_real_guard_value`
(composition `Some(true)`, evidence `false`, gate aborts against a guard reporting zero installed
filters). Both docs now state the two-valuedness explicitly and say why.

**Chose the doc-correction option deliberately over the review's first suggestion**
(`Some(*installed_filter_count > 0)`): that would re-collapse composition and enforcement
evidence into one signal, which is precisely the single-source-of-evidence defect NR-04 split
apart and the reason the deny direction was unreachable to begin with. Narrowing the field's type
was also rejected — it would delete the existing `Some(false)` negative-path coverage in
`attestation.rs` without adding a reachable state.

### (Not a review finding) Self-inflicted regression, found and fixed

**File:** `crates/nono-cli/tests/layer_registry_selfcheck.rs` · **Commit:** `7a8fcd36`

My WR-05 and WR-08 assertion messages legitimately name symbols in prose
(`attestation.rs::classify_row`, and a `"file.rs::Symbol"` shape example). The citation
extractors scanned **every** string literal in `layer_registry.rs`, so those were scraped as
call-site citations and "resolved" to paths like `exec_strategy_windows/ProbeKind`, failing
`registry_call_sites_exist` for citations nobody wrote. Caught by running the suite rather than
only the test I had touched.

Both extractors are now scoped via `call_sites_regions()`. Narrowing is the fail-OPEN direction
if the marker stops matching, so two floors were added (`>= 13` lists, `>= 20` citations).
**Perturbation:** breaking the region marker now FAILS the floor instead of extracting nothing.

---

## Partial

### CR-02: The `EntryPath::Daemon` half of the registry drives no decision

**Files:** `exec_strategy_windows/layer_registry.rs`, `proj/SPEC-windows-fail-direction-contract.md`
**Commit:** `9c2640c5`

The finding has three parts. **Two are closed; the third is an operator decision I deliberately
did not take.**

**Closed — defect 3 (no daemon analog of `broker_expected_rows_are_abort_only`).** Added
`daemon_expected_rows_are_all_named_by_the_daemon_gate`: a discovery-based gate naming no
`LayerId`, requiring every `(Daemon, expected: true)` row with an attestable probe to be named by
`agent_daemon/launch.rs`. Source-text by necessity — `nono-agentd` never declares
`exec_strategy_windows`, so it *cannot* link the registry.

**The perturbation mattered here.** My first version excluded comments only and stayed GREEN when
the production `layer = "DaclAncestorTraverse"` was renamed, because that file's own
`attestation_gate_tests` asserts on the same string. The gate now also excludes `#[cfg(test)]`
module bodies (indent-aware closer, since the daemon's test modules nest inside `mod
windows_impl`) and carries two non-vacuity assertions (`checked >= 5`,
`skipped_test_lines > 0`). With those, the same rename FAILS with *"the row goes completely
unattested on `nono agent launch`"*.

**Closed — defect 2 (the mirror contradicts the registry).** `DaclAncestorTraverse` declares
`outcome: Abort` at `(Daemon, None)` while the daemon warns and proceeds. Recorded in the SPEC's
D-15 "Contract vs. code discrepancies" ledger, which is where D-15 exists to hold it.

**NOT fixed — defect 1 (the `(Daemon, ..)` cells drive nothing).** This is the scope question.
The gate proves *"no daemon-expected row is unmentioned"*, **not** *"the registry drives the
daemon"*, and its own doc comment says so — I did not half-wire anything to make the registry
look connected when it is not.

**Open operator decision, with costs:**
- **Option A — make the daemon consume the registry.** Restructure `nono-agentd`'s `#[path]`
  includes so it links `layer_registry`/`attestation` and calls `attest_and_decide` with
  `EntryPath::Daemon`. Cost: pulls the CLI-side `exec_strategy_windows` tree into the daemon
  binary (that module's own doc states the independence is intentional), **and**
  `DaclAncestorTraverse` would then abort daemon launches that succeed today.
- **Option B — declare the `(Daemon, ..)` cells documentation-only**, give the row an explicit
  daemon-specific outcome, and keep the hand-written mirror with this new gate as the binding.
  Cost: the registry is authoritative on two of three entry paths, not three.

**Explicitly rejected third option:** marking the row `expected: false`. The daemon *does* apply
ancestor traverse (`ancestor_traverse_applied` is a real gate input), so that would be a lie in
the opposite direction.

---

## Skipped

### WR-10: `MandatoryIntegrityLabel` conflates two different kernel objects

**Commit:** `63e18a80` (recorded, not fixed)

Recorded in the SPEC's D-15 ledger with a greppable `WR-10 OPEN` marker on the registry row, so
code and SPEC now say the same thing — previously the conflation lived only in a parenthetical in
the registry table's Probe column, which is not where a standing discrepancy belongs. The review
explicitly sanctions this alternative ("If splitting is deferred, record the conflation
explicitly in the SPEC's ... table (D-15)").

**Why the split is deferred, not merely unfinished:** it renames a value carried in
`NONO_BROKER_REQUIRED_LAYERS`, a cross-binary wire contract whose consumer **fail-closed refuses
to resume on any unrecognised name** (RF-02, deliberate). It is a lockstep two-binary change in
which a mixed-version `nono-cli`/`nono-shell-broker` pair refuses **every** launch — an operator
decision about release ordering, not a registry edit.

**Fail direction meanwhile:** both objects *are* genuinely attested on their own arms. This is a
naming/claim-precision defect, not a layer going unattested.

### WR-12: The entire D-26 tighten path is unreachable in a shipped build

**Not fixed. No commit.**

The review's fix is "ship CR-03's plumbing (or a `--required-layers` CLI flag) so at least one
production path can populate a non-empty tighten set", plus a discovery test asserting the gate
call site does not pass a literal empty slice. The discovery test **cannot be added without the
plumbing** — it would fail on the current tree.

Carrying `MachineEgressPolicy` to `spawn_windows_child` is the step the SPEC's own RF-13 row
already records as an **open operator decision**, requiring "acceptance that a fleet registry key
can then refuse launches". Wiring it would take that decision on the operator's behalf and could
start aborting launches on any machine with the key configured. Adding a `--required-layers` flag
is a new feature, not a review fix.

**What CR-03 already removed:** the *dangerous* half. The parser no longer discards the value, so
the eventual plumbing cannot silently pass an empty slice while looking wired.

**Fail direction:** tightening only ever ADDS aborts, so an unreachable tighten path is dead code
plus unexercised-fix risk (WR-29's `NotApplicable`-abort fix is pinned only by unit tests against
synthetic rows) — not a fail-open.

### WR-14: `ClearStaleLayerResidue` fires for causes that are not residue

**Commit:** `730ddd2e` (hazard recorded, not fixed)

`remediation()` returns `ClearStaleLayerResidue` for every `LayerAttestationFailed`, including
three cause classes with nothing stale to clear. **All three are unreachable in a shipped build
today:** the `layer-fault-injection` seams are compiled out of default builds;
`probe_in_job`'s null-job refusal needs a handle no production caller passes; and the
operator-facing one (an unrecognized required-layer name, where `layer` is an admin's typo)
cannot fire while both tighten inputs at the launch gate are hardcoded empty slices — i.e. it
becomes reachable exactly when **WR-12**'s plumbing lands.

The real fix needs a discriminator the library can see (a `kind` field on the variant plus a
`NonoRemediation::CheckRequiredLayersPolicy`), because `crates/nono` is policy-free and must not
learn `LayerId` names. That is a public-API shape change rippling through ~20 construction sites,
~15 destructuring test patterns and the C FFI — too large to bundle into a review-fix pass
without its own review.

Recorded in `error.rs` as a greppable `WR-14 OPEN` block that enumerates the three classes, notes
their unreachability, states **"THIS ARM MUST BE FIXED IN THE SAME CHANGE"** as WR-12's plumbing,
and records that `render_error_for_operator`'s `_` arm must not be the place that decides.

---

## Commits

| # | Hash | Finding |
|---|---|---|
| 1 | `c34f569e` | WR-01 |
| 2 | `5590a8fe` | CR-01 |
| 3 | `1400d77a` | WR-06 (+ CR-01 `production()` correction) |
| 4 | `ec78f8be` | CR-03 |
| 5 | `9c2640c5` | CR-02 (partial) |
| 6 | `b6cf447f` | WR-03 |
| 7 | `6b2b47e8` | WR-05 |
| 8 | `65691203` | WR-07 |
| 9 | `634f2a9e` | WR-11 |
| 10 | `e2cfe36e` | WR-04 |
| 11 | `786a7887` | WR-08 |
| 12 | `4e4a7800` | WR-13 |
| 13 | `730ddd2e` | WR-14 (hazard recorded) |
| 14 | `63e18a80` | WR-10 (recorded) |
| 15 | `7a8fcd36` | self-inflicted regression repair |
| 16 | `b8a4e5fc` | WR-09 |
| 17 | `8cfde970` | WR-02 |

---

## Carry-forward for the next planning pass

1. **CR-02 defect 1** — operator decision: does the daemon arm consume the registry? (Options A/B
   costed above and in the SPEC ledger.)
2. **WR-12 + WR-14 are one change.** Landing the machine-policy plumbing makes WR-14's
   mis-targeted remediation reachable. Both markers (`WR-14 OPEN`, the SPEC's RF-13 row) say so.
3. **WR-10** — operator decision: accept the lockstep two-binary rename, or keep one `LayerId`
   covering two kernel objects with the divergence documented.

---

_Fixed: 2026-08-14_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 6_
