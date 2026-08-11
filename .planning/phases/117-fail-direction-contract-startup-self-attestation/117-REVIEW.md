---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-11T20:13:50Z
depth: deep
iteration: 6
files_reviewed: 15
files_reviewed_list:
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/telemetry/mod.rs
  - crates/nono-cli/tests/common/mod.rs
  - crates/nono-cli/tests/layer_registry_meta_test.rs
  - crates/nono-cli/tests/layer_registry_selfcheck.rs
  - crates/nono/src/sandbox/windows.rs
  - proj/SPEC-windows-fail-direction-contract.md
findings:
  critical: 3
  warning: 12
  info: 0
  total: 15
status: issues_found
---

# Phase 117: Code Review Report (Iteration 6)

**Reviewed:** 2026-08-11T20:13:50Z
**Depth:** deep (cross-file: import graph, call chains, sibling call sites, live test execution)
**Files Reviewed:** 15
**Status:** issues_found
**Numbering:** continues from iteration 5 (CR-03 / WR-21) → new criticals start at CR-04, new warnings at WR-22.

## Summary

Round 3 (plans 117-27..117-34) did partially break the "fixed at 1 of N sites" cycle: the
D-37 three-arm ancestor classification (`dacl_guard.rs`) is genuinely three-valued and its
`NotApplicable` arm is reachable and tested; `is_ident_boundary` really is a single shared
definition in `tests/common/mod.rs` consumed by both matchers; `advance_and_emit` really does
hold the chain mutex across the full build+advance+emit sequence and all three production
`advance_chain(` call sites (`telemetry/mod.rs:402`, `:494`, `:598`) now share that guarantee;
`low_integrity_label_rid` really does delegate to the CR-01-hardened `low_integrity_label_ace`.

It did **not** break the cycle in the two places that matter most, and it shipped a
build-failing test.

1. **The round's own new drift gate fails on HEAD.** `cargo test -p nono-sandbox-cli --test
   layer_registry_selfcheck` → `every_spec_symbol_citation_resolves_to_a_real_definition`
   FAILED (reproduced live, this review). Plan 117-32 built the gate; plan 117-34 (a later
   wave, same round) wrote a SPEC ledger row containing an unquoted illustrative
   `` `file.rs::Symbol` `` span that the gate then tries to resolve as a real citation.
   Read-before-write wave hazard, in the round that was planned to close exactly this class.

2. **CR-03's D-28 site enumeration stopped at the function boundary.** It enumerated the three
   emission sites *inside* `apply_startup_attestation_gate` and gated them on one shared
   predicate — correct as far as it goes — but never followed `emit_attestation_event` down
   its own call chain. `telemetry::windows::write_security_event_log`'s `RegisterEventSourceW
   == NULL` fallback raw-`eprintln!`s the entire serialized `SecurityEvent`, **including
   `downgraded_layers`**, to the supervisor's stderr: the exact channel D-28 exists to keep
   layer names off, completely outside the `log_target_is_private()` gate. A fourth site of the
   same class.

3. **WR-17's remediation prescribes a command that provably cannot clear the condition.** The
   `MandatoryIntegrityLabel` arm tells the operator to run `icacls <path> /setintegritylevel
   Medium`, which *writes a Medium mandatory-label ACE*. `AppliedLabelsGuard`'s D-02 rule
   treats **any** prior label that is not an exact Low+wanted-mask match as
   `SkipPreExistingLabel`, so after following the printed guidance the launch aborts
   identically. WR-04→WR-17 replaced one wrong command with a second wrong command.

Beyond those, the review found the private-channel discovery test lost its ability to
distinguish the site it names (WR-22), a fail-open in `log_target_is_private()`'s
granted-path check (WR-23/WR-24), a public label-reader API that still omits CR-01's
hardening with a production consumer (WR-25), two more "points at a channel that receives
nothing" instances (WR-26/WR-27), a coverage-arithmetic contradiction between the two sibling
`application()` implementations (WR-28), and a D-26 tighten-only regression introduced by
D-37's widened `NotApplicable` (WR-29).

Baseline note: `cargo test -p nono-sandbox-cli --bin nono` → 1624 passed / **12 failed**,
matching the documented 11 pre-existing baseline failures + WR-20's host-blocked test. No new
unit-test regressions in the `nono` binary.

---

## Critical Issues

### CR-04: The round's own SPEC drift gate fails the build on HEAD

**File:** `proj/SPEC-windows-fail-direction-contract.md:296` (offending span);
`crates/nono-cli/tests/layer_registry_selfcheck.rs:690-726` (the gate);
`crates/nono-cli/tests/layer_registry_selfcheck.rs:457-484` (`looks_like_a_spec_citation`)

**Issue:** `every_spec_symbol_citation_resolves_to_a_real_definition` (added by plan 117-32,
WR-19) FAILS on the shipped tree. Reproduced live during this review:

```
test every_spec_symbol_citation_resolves_to_a_real_definition ... FAILED
panicked at crates\nono-cli\tests\layer_registry_selfcheck.rs:726:5:
proj/SPEC-windows-fail-direction-contract.md cites a file.rs::Symbol that does not resolve ...
"file.rs::Symbol" -> resolved to ...\exec_strategy_windows\file.rs (unreadable: os error 2)
```

`looks_like_a_spec_citation` deliberately excludes the document's illustrative-example
convention — a *quoted* span, `` `"file.rs::Symbol"` `` (used at SPEC lines 274 and 284, both
correctly ignored). Plan 117-34's WR-19 ledger row (line 296) wrote the same illustrative
example **unquoted**: "content-verifies every `` `file.rs::Symbol` ``-shaped citation anywhere
in the whole SPEC document". The scanner accepts it, resolves `file.rs` against the default
`exec_strategy_windows/` root, and fails.

This is a read-before-write wave collision inside a single round: 117-32 produced the gate,
117-34 consumed the same document and re-introduced the input class the gate rejects. Neither
plan's self-check ran the other's test.

**Fix:** Quote the illustrative example on SPEC line 296 to match the document's own
established convention at lines 274/284:

```markdown
`every_spec_symbol_citation_resolves_to_a_real_definition` content-verifies every
`"file.rs::Symbol"`-shaped citation anywhere in the whole SPEC document, ...
```

And close the class rather than the instance — make the exclusion explicit instead of relying
on a quoting convention no writer is prompted about, e.g. skip any span whose surrounding
sentence is inside a ledger `| ... |` row that also contains the literal `-shaped`, or (better)
require citations to be introduced by a recognizable prefix. At minimum, add a regression
assertion that the extractor rejects the bare `file.rs::Symbol` form:

```rust
#[test]
fn illustrative_format_example_is_not_treated_as_a_citation() {
    assert!(!looks_like_a_spec_citation("file.rs::Symbol"));
}
```

---

### CR-05: D-28 violation — specific `LayerId` names reach the confined child's shared stderr through `emit_attestation_event`'s own call chain

**File:** `crates/nono-cli/src/telemetry/windows.rs:126-134` (the leak);
`crates/nono-cli/src/telemetry/windows.rs:74-80, 88-97` (payload/message builders);
`crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs:135-151` (caller);
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1489-1535` (CR-03's three-site gate)

**Issue:** CR-03 gated the three `tracing` emission sites *inside*
`apply_startup_attestation_gate` on one shared `log_target_is_private()` predicate. It did not
follow the audit-event call chain. On the `ProceedDowngraded` path:

`apply_startup_attestation_gate` → `emit_attestation_event` →
`emit_security_event(&security_event)` → `write_security_event_log(Warning, 10011, &payload)`,
where `payload = serde_json::to_string(event)` and `SecurityEvent.downgraded_layers`
(`telemetry/event.rs:308`) holds the comma-joined `LayerId` `Debug` names.

`write_security_event_log`'s documented D-03 fallback:

```rust
let handle = unsafe { RegisterEventSourceW(std::ptr::null(), source_wide.as_ptr()) };
if handle.is_null() {
    // Source not registered (development environment or broken MSI install).
    eprintln!(
        "nono: telemetry: RegisterEventSourceW returned NULL (source not registered) — {}",
        build_event_log_message(level, event_id, body)   // body == the full JSON
    );
    return;
}
```

That is a raw `eprintln!` to the supervisor's stderr — the same handle the confined child
shares on the non-detached-stdio path (`STARTF_USESTDHANDLES` unset, `bInheritHandles = 0`),
which is precisely the sharing CR-02's own ledger row established. It bypasses
`log_target_is_private()` entirely (it is not a `tracing` call at all), it is not deduplicated,
and it fires on exactly the hosts the comment names as common ("source not registered —
development environment or broken MSI install"), i.e. any install where `nono setup` has not
registered the `nono` Application-log source.

Net effect: on an unregistered-source host, D-28 is inverted — the layer names the gate
withholds from the console are dumped to the console verbatim, in JSON, with the session id and
chain head alongside.

**Fix:** Never render `downgraded_layers` on the stderr fallback. Scrub the field before the
fallback print, or make the fallback print only the non-layer envelope:

```rust
if handle.is_null() {
    // D-28: the fallback channel is the SHARED console — never render
    // layer-specific detail here.
    let redacted = SecurityEvent { downgraded_layers: None, ..event.clone() };
    eprintln!(
        "nono: telemetry: RegisterEventSourceW returned NULL (source not registered) — {}",
        build_event_log_message(level, event_id, &build_event_payload(&redacted))
    );
    return;
}
```

(Threading the event, not the pre-rendered `body`, into `write_security_event_log` is the
cleaner shape.) Then extend the CR-03 withholding test to drive the whole chain — construct a
`SecurityEvent` with `downgraded_layers: Some(..)`, call `build_event_log_message` on it, and
assert no `LayerId` `Debug` name appears — so the assertion is on the *rendered fallback text*,
not only on captured `tracing` events.

---

### CR-06: WR-17's `MandatoryIntegrityLabel` remediation cannot clear the condition it names — the operator stays locked out

**File:** `crates/nono-cli/src/main.rs:261-267` (the prescribed command);
`crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:284-334` (the D-02 rule);
`crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:144-156` (`LabelCoverage::application`)

**Issue:** Two independent defects in the same three lines.

**(a) The command does not clear the label.** `icacls <path> /setintegritylevel Medium` does not
remove the mandatory-label ACE; it *writes* one at `SECURITY_MANDATORY_MEDIUM_RID`. On the next
launch, `low_integrity_label_ace(&rule.path)` returns `Some((0x2000, mask, flags))`, which fails
the residue predicate (`prior_rid == SECURITY_MANDATORY_LOW_RID as u32 && prior_mask == wanted &&
(flags & INHERIT_ONLY_ACE) == 0`) and therefore records `SkipPreExistingLabel`
(`labels_guard.rs:332`). `coverage.applied` stays `0`, `LabelCoverage::application()` returns
`NotApplied`, `classify_row` maps that to `Unconfirmed`, and `decide_from_entries` returns
`Abort` again. The operator follows the only guidance nono prints and the launch fails
identically. nono's own revert path uses `clear_mandatory_label` (ACE removal), not a Medium
set — the guidance does not match the mechanism.

**(b) The stated cause is the one cause that can no longer produce this abort.** The message
says "A prior session that exited abnormally may have left mandatory-label ACEs on granted
paths." Plan 117-13 (NR3-01) specifically made nono's own residue non-aborting via
`AlreadyAtRequiredLevel`. The abort now fires only for a *foreign*/mask-mismatched label, an
`INHERIT_ONLY_ACE` (CR-01), or an all-`SkipNotOwned` policy (see WR-28) — none of which is
"nono's own residue from an abnormal exit".

This is WR-04's defect reproduced, not fixed: WR-04 printed `nono setup --check-only`, which
cannot diagnose the condition; WR-17 replaced it with a command that can diagnose but not
remedy it. The abort is a hard availability failure (the operator cannot run nono over their own
workspace), which is why this is Critical rather than a diagnostic-quality warning.

**Fix:** Name a command that actually removes the ACE, and describe the real condition:

```rust
"MandatoryIntegrityLabel" => format!(
    "nono:   {layer} could not be confirmed: one or more granted paths carry a \
     mandatory-label ACE nono did not write (a third-party label, a mask that does not \
     match this profile's access mode, or a structurally-inert INHERIT_ONLY ACE). nono \
     never mutates a pre-existing label. Inspect with `icacls <granted-path>` (look for \
     `Mandatory Label\\...`) and REMOVE the label — e.g. \
     `Set-Acl` after `$acl.SetSecurityDescriptorSddlForm(...)` with an empty `S:` — \
     or re-run against a path with no pre-existing label."
),
```

Whatever wording is chosen, add a test that the named remedy is one the guard's own predicate
accepts afterward — e.g. apply the remedy in a test, re-run `snapshot_and_apply`, and assert
`coverage().application() != NotApplied`. Without that, the next round replaces one wrong
command with a third.

---

## Warnings

### WR-22: CR-03's test widening destroyed the private-channel test's ability to observe the site it names; the `SECURITY_LAYER`-present arm has zero coverage

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:3735-3800`
(`proceed_downgraded_success_path_logs_downgraded_layers_field_on_private_log_channel`),
`:3830-3910` (`..._withholds_layer_names_on_the_shared_console_channel`)

**Issue:** Two problems, both of the phase's defining class.

(a) The private-channel test previously asserted a **structured field** literally named
`downgraded_layers` — a property only the third (unconditional operator warn) site could
satisfy, since sites 1 and 2 interpolate into message text. CR-03 widened it to
`fields.iter().any(|(_name, value)| value.contains("MandatoryIntegrityLabel"))`. In this test
binary `crate::telemetry::SECURITY_LAYER` is never `set` (verified: the only `set` call sites
are `cli_bootstrap.rs:338` and `agent_daemon/telemetry_init.rs:75`, neither reachable from a
unit test), so site 1 — `"...proceeding per AUD-04's non-fatal contract{layer_detail}"` — always
fires and its message already contains `MandatoryIntegrityLabel`. `CapturingSubscriber`'s
`FieldCapture::record_debug` captures the `message` field (`launch.rs:3678-3681`). The
assertion is therefore satisfied by site 1 alone: deleting `layer_detail` from site 3's
`operator_suffix` entirely would leave this test green.

(b) The sibling's doc comment asserts "This drives the SECURITY_LAYER-present arm (the
audit-emission success path, the third site)". It does not — `SECURITY_LAYER.get()` is `None`
in this binary too, so both shared-console tests exercise the identical arm. Site 2
(`emit_attestation_event` returning `Err`) has no coverage on any path.

**Fix:** Restore a site-3-specific assertion (a structured `downgraded_layers` field on the
unconditional warn, or a marker substring unique to site 3's message), and drive site 2/3 with a
real layer:

```rust
// site 2/3 coverage: install a real layer, then poison it so emit_attestation_event Errs.
let layer = SecurityEventLayer::new(TelemetryConfig::default(), "t".into());
layer.poison_for_test();
let _ = crate::telemetry::SECURITY_LAYER.set(layer); // OnceLock: one test only
```

If the process-global `OnceLock` makes that infeasible, factor the three-site emission block out
of `apply_startup_attestation_gate` into a pure function taking `Option<&SecurityEventLayer>`
and test it directly.

---

### WR-23: `log_target_is_private()` fails OPEN when no granted paths were recorded, contradicting its own documented fail-secure rule

**File:** `crates/nono-cli/src/cli_bootstrap.rs:48-49, 60-116`

**Issue:** The doc comment states the rule explicitly: *"Fail-secure on every unknown/error case
(WR-16's own stated rule): no `--log-file` arm selected, **no granted-path check has run**, or
the stored log path fails to canonicalize all return `false`"*. The code cannot implement that
rule, because `GRANTED_READ_PATHS` is a `Mutex<Vec<PathBuf>>` initialized to `Vec::new()` with
no `Option`/`bool` distinguishing "never populated" from "populated empty":

```rust
!granted_paths.iter().any(|granted| { ... })   // empty vec => `!false` => TRUE ("private")
```

So any path reaching the gate without `set_granted_read_paths` having run — or with an empty
`fs_capabilities()` — is reported **private**, and the D-28 gate opens. Today the only consumer
is inside `spawn_windows_child` (set at `launch.rs:1643-1649`, consumed at `:2475`), so the
ordering happens to hold; the safety therefore rests on an unstated single-call-site invariant
that the doc comment claims is enforced and is not.

**Fix:** Make "not yet populated" representable and fail secure on it:

```rust
static GRANTED_READ_PATHS: std::sync::Mutex<Option<Vec<PathBuf>>> = std::sync::Mutex::new(None);
...
let Some(granted_paths) = GRANTED_READ_PATHS.lock()... .clone() else { return false; };
```

Add the matching test (`no granted paths recorded => not private`) alongside the existing
`no_log_path_recorded_is_not_private`.

---

### WR-24: the "never skipped" raw-path fallback in the granted-path comparison is structurally always-false on Windows

**File:** `crates/nono-cli/src/cli_bootstrap.rs:107-115`

**Issue:**

```rust
let canonical_granted = std::fs::canonicalize(granted).unwrap_or_else(|_| granted.clone());
canonical_log_path.starts_with(&canonical_granted)
```

`canonical_log_path` came from `std::fs::canonicalize`, which on Windows yields a **verbatim**
path (`\\?\C:\...`, prefix component `Prefix(VerbatimDisk('C'))`). The fallback compares it
against a *non-canonical* `granted` (`Prefix(Disk('C'))`). `Path::starts_with` is
component-wise, and `VerbatimDisk` != `Disk`, so the fallback comparison can never match. The
comment claims "A not-yet-existing grant target must still be checked — never skipped — so fall
back to the raw (uncanonicalized) path rather than dropping the comparison"; the fallback drops
the comparison in effect.

Impact today is bounded (an existing log file cannot sit under a non-existent granted directory),
but the stated defense does not exist, and the same idiom will be wrong the moment
canonicalization fails for a reason other than non-existence (permissions, reparse-point depth).

**Fix:** Normalize both sides through one helper that strips/adds the verbatim prefix
consistently, and fail secure (treat the grant as covering) when a granted path cannot be
normalized:

```rust
let Ok(canonical_granted) = std::fs::canonicalize(granted) else {
    // Cannot resolve a granted path — assume it may cover the log file. Fail secure.
    return true;
};
canonical_log_path.starts_with(&canonical_granted)
```

---

### WR-25: `low_integrity_label_and_mask` — a `pub` library API with a production consumer — still drops `AceFlags` without CR-01's `INHERIT_ONLY_ACE` rejection

**File:** `crates/nono/src/sandbox/windows.rs:2102-2112`;
consumer `crates/nono-cli/src/hook_runtime_windows.rs:590`;
`crates/nono/src/lib.rs:97` (re-export)

**Issue:** WR-18's ledger row claims: *"Enumerated every `SYSTEM_MANDATORY_LABEL_ACE` reader in
the workspace; confirmed exactly these two exist and both now share one implementation."* The
independent enumeration performed for this review (`grep -n "SYSTEM_MANDATORY_LABEL_ACE\|GetAce\|
low_integrity_label"` across `crates/` + `bindings/`) confirms exactly one SACL *walk*
(`low_integrity_label_ace`) — but three *consumers* of the label read, and the CR-01 hardening
reaches only two of them:

| consumer | filters `INHERIT_ONLY_ACE`? |
|---|---|
| `low_integrity_label_rid` → `is_low_integrity_compatible_dir` → `Sandbox::windows_supports_direct_writable_dir` | yes (WR-18, `windows.rs:1996`) |
| `labels_guard.rs:302-305` residue predicate | yes (CR-01) |
| `low_integrity_label_and_mask` (`pub`, re-exported from `nono::`) → `hook_runtime_windows.rs:590` | **no** |

The third silently discards `flags`, so an inert inherit-only ACE is indistinguishable from an
effective one — CR-01's exact class, at the public API boundary, where external consumers
(bindings, `nono-py`, `nono-ts`) inherit it. The current in-tree consumer only emits a warning,
which is why this is a Warning and not a Critical, but the enumeration claim in the SPEC ledger
is inaccurate.

**Fix:** Either apply the same filter in the wrapper, or make the wrapper's un-hardened
semantics explicit in the name and doc:

```rust
#[must_use]
pub fn low_integrity_label_and_mask(path: &Path) -> Option<(u32, u32)> {
    low_integrity_label_ace(path)
        .filter(|(_, _, flags)| (u32::from(*flags) & INHERIT_ONLY_ACE) == 0)
        .map(|(rid, mask, _)| (rid, mask))
}
```

and add a mirror of `inherit_only_residue_is_not_treated_as_already_covered` asserting
`low_integrity_label_and_mask` rejects a planted inherit-only ACE. Correct the WR-18 ledger row's
enumeration claim from "two readers" to "one SACL walk, three consumers, all three hardened".

---

### WR-26: the banner and operator warn name the Windows Application event log unconditionally, but three common conditions leave nothing there

**File:** `crates/nono-cli/src/output.rs:115-125`;
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1556-1562`

**Issue:** WR-15 replaced the nonexistent "audit ledger" pointer with "layer detail is in the
Windows Application event log (source `nono`, event id 10011)". That destination receives the
record only when *all three* hold:

1. `crate::telemetry::SECURITY_LAYER.get()` is `Some(..)` — otherwise site 1 fires and no event
   is ever emitted (`launch.rs:1519-1526`);
2. `inner.config.enabled` is `true` — `emit_attestation_event`'s emit closure is `if enabled`
   (`attestation_downgrade_event.rs:136`), and `enabled` is driven by the HKLM
   `MachineEgressPolicy` telemetry control, which an administrator can turn off;
3. `RegisterEventSourceW` succeeds — otherwise the record goes to stderr instead (see CR-05).

In each of those cases the operator is sent to a destination that provably has no record — the
same defect class WR-15 was written to close, reintroduced by making the pointer
unconditional instead of conditional on the emission actually having succeeded.

**Fix:** Thread the emission outcome into the banner/warn text. `emit_attestation_event` already
returns `Result<String, &'static str>` and site 1 already knows `SECURITY_LAYER` was absent;
pass a `detail_channel: DowngradeDetailChannel` enum (`EventLog | PrivateLogFile | None`) into
`print_attestation_downgrade_banner` and render the pointer only for the channel that actually
received the record. When it is `None`, say so ("layer detail could not be recorded on any
channel — re-run with `--log-file <path>`").

---

### WR-27: `render_error_for_operator`'s non-label arm points at a channel that receives nothing on the abort path

**File:** `crates/nono-cli/src/main.rs:268-271`

**Issue:** The `_` arm prints "see the Windows Application event log (source `nono`) for the
per-layer attestation record." No such record exists for an abort. `emit_attestation_event` — the
only writer of a `LayerAttestationDowngraded` event — is called exclusively from
`apply_startup_attestation_gate`'s `ProceedDowngraded` arm (`launch.rs:1528`). The
`AttestationDecision::Abort` arm (`launch.rs:1474-1479`) constructs
`NonoError::LayerAttestationFailed` and returns; it emits no `SecurityEvent`, writes no Event
Log entry, and appends nothing to any chain. Confirmed by enumerating every
`LayerAttestationFailed` construction site in the workspace — none is paired with a telemetry
emit.

So the *only* guidance an operator gets for a `WfpEgressFilters` / `AppContainerProfile` /
`JobObjectContainment` abort directs them at an empty channel. Same class as WR-15 and CR-06.

**Fix:** Either emit an audit/Event-Log record on the abort path before returning `Err` (the
useful option — an abort is more security-relevant than a downgrade and currently leaves no
forensic trace), or point the operator at what genuinely exists: the `tracing` warn emitted by
the failing guard, i.e. "re-run with `-vv --log-file <path>` and search for the layer name". Add
a test asserting the rendered line names a channel the abort path actually writes to.

---

### WR-28: `LabelCoverage::application()` counts contract-exempt skips as coverage gaps — the opposite rule from its sibling `DaclGrantCoverage::application()`

**File:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:116-157`;
sibling `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:129-154`

**Issue:** The two coverage types implement contradictory rules for their own exempt category:

```rust
// dacl_guard.rs — EXCLUDES the exempt category from the denominator
fn writable_rules(self) -> usize { self.applied + self.skipped_writable_not_owned }  // read-only excluded
if self.writable_rules() == 0 { return NotApplicable; }

// labels_guard.rs — INCLUDES the exempt category in the denominator
policy_paths: self.entries.len()          // SkipNotOwned counted here
if self.policy_paths == 0 { return NotApplicable; }
if self.applied == 0     { return NotApplied; }   // => Unconfirmed => Abort
```

`SkipNotOwned`'s own doc (`labels_guard.rs:94-98`) says it is *"contract-exempt, not a coverage
gap"*, and the SPEC's standing rule says *"Contract-exempt skips are NOT downgrades"* — yet a
policy composed entirely of non-owned paths yields `policy_paths > 0, applied == 0` →
`NotApplied` → hard `Abort`. `guard_skips_path_not_owned_by_current_user`
(`labels_guard.rs:1108-1112`) pins exactly this behavior with the comment "a guard that wrote
zero ACEs must report NotApplied", so the contradiction is asserted, not accidental.

This is the same shape D-37 (WR-12) was convened to resolve for the ancestor guards — an exempt
outcome classified as a gap — left unfixed one file over, in a round that planned by class.

**Fix:** Exclude the exempt category from the denominator, mirroring the sibling:

```rust
fn non_exempt_paths(self) -> usize {
    self.applied
        .saturating_add(self.skipped_pre_existing_label)
}
pub(crate) fn application(self) -> LayerApplication {
    if self.non_exempt_paths() == 0 { return LayerApplication::NotApplicable; }
    if self.applied == 0 { return LayerApplication::NotApplied; }
    if self.skipped_pre_existing_label > 0 { return LayerApplication::PartiallyApplied; }
    LayerApplication::Applied
}
```

Update `guard_skips_path_not_owned_by_current_user`'s assertion in the same change, and add a
mixed case (1 owned+applied, 2 non-owned) asserting `Applied`, not `PartiallyApplied`.

---

### WR-29: D-37's widened `NotApplicable` silently defeats the D-26 tighten-only mechanism

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:488-499`;
`crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:470-481, 715-726`

**Issue:** `decide_from_entries` short-circuits `NotApplicable` **before** consulting
`required_names`:

```rust
let verdict = classify_row(entry, input);
let status = verdict.status;
if status == LayerAttestationStatus::NotApplicable
    || (status == LayerAttestationStatus::Confirmed && !verdict.partially_established)
{
    continue;                      // <-- tightening never evaluated
}
let tightened = required_names.contains(&layer_name);
```

Before plan 117-28, a walk that stopped at the first non-owned ancestor classified
`PartiallyApplied` → `EstablishedNotIndependentlyObservable` + `partially_established` → reached
the tighten check → `if tightened { Abort }`. After D-37 it classifies `NotApplicable` → the
`continue` fires → an operator (or `HKLM\...\RequiredLayers` fleet policy) that explicitly
*requires* `DaclAncestorTraverse` gets a silent pass instead of an abort. The SPEC's RF-16
paragraph still asserts "A D-26-tightened requirement on a partially established layer still
aborts"; that is now false for this condition.

Currently latent only because `apply_startup_attestation_gate` hardcodes
`required_layers_override: &[]` / `machine_required_layers: &[]` (`launch.rs:1469-1470`), so no
production path can tighten. It becomes live the moment RF-13's open enforcement decision lands.

**Fix:** Evaluate tightening before the `NotApplicable` short-circuit:

```rust
let layer_name = format!("{:?}", entry.id);
let tightened = required_names.contains(&layer_name);
if !tightened
    && (status == LayerAttestationStatus::NotApplicable
        || (status == LayerAttestationStatus::Confirmed && !verdict.partially_established))
{
    continue;
}
if tightened && status != LayerAttestationStatus::Confirmed {
    return AttestationDecision::Abort { layer: entry.id, status };
}
```

Add a test: `DaclAncestorTraverse` reported `NotApplicable` **and** named in
`required_layers_override` must `Abort`.

---

### WR-30: `advance_and_emit`'s "Deadlock analysis" states a false premise; safety rests on an incidental target-string mismatch

**File:** `crates/nono-cli/src/telemetry/mod.rs:460-469, 487-501`;
`crates/nono-cli/src/telemetry/windows.rs:198-209`;
`crates/nono-cli/src/telemetry/mod.rs:525-535`

**Issue:** The doc comment justifies holding the mutex across `emit` with:

> "the only caller ... passes an `emit` closure whose body calls
> `crate::telemetry::windows::emit_security_event` — a free function that writes to the Windows
> Application log / ETW and **touches no `SecurityEventLayer` state**, so it cannot re-enter
> this mutex."

`emit_security_event` does not merely write to the OS log; it emits a `tracing` event
(`windows.rs:198-209`), and `SecurityEventLayer` is a registered `Layer` on the global
subscriber. The only reasons this does not self-deadlock a `std::sync::Mutex` (non-reentrant,
same thread) are two mechanisms the comment never mentions:

1. `on_event` returns at `if !event.metadata().target().starts_with("nono_security::")` — and the
   emit target is exactly `"nono_security"`, with no trailing `::`, so the prefix test misses by
   two characters. Every doc comment in this module describes the target convention as
   `nono_security::*` (`mod.rs:13-15`, `windows.rs:170, 191`), so a future edit aligning the
   emit target with the documented convention flips this to a hang.
2. `tracing`'s per-thread dispatch re-entrancy guard (which does not apply here, since
   `advance_and_emit` is not itself inside a subscriber callback).

A hang here is worse than a crash: the supervisor deadlocks while holding a `CREATE_SUSPENDED`
child that will never be resumed or terminated.

**Fix:** Correct the analysis to state the real invariant, and enforce it mechanically:

```rust
/// # Deadlock analysis
///
/// `emit` MUST NOT emit any `tracing` event whose target begins with
/// `nono_security::` — `SecurityEventLayer::on_event` takes THIS mutex and
/// `std::sync::Mutex` is not reentrant. `emit_security_event` emits on the
/// bare target `"nono_security"` (no `::` suffix), which `on_event`'s prefix
/// test deliberately does not match. Pinned by
/// `emit_security_event_target_must_not_match_on_event_prefix`.
```

```rust
#[test]
fn emit_security_event_target_must_not_match_on_event_prefix() {
    // Source-text discovery: the emit target literal must not start with
    // the prefix on_event dispatches on.
    let src = include_str!("windows.rs");
    assert!(src.contains("target: \"nono_security\","));
    assert!(!src.contains("target: \"nono_security::"));
}
```

---

### WR-31: the daemon attests `DaclAncestorTraverse` from a boolean that observes only the workspace write grant

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:121-123, 1319-1322, 1399-1406`;
`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` (`DaclAncestorTraverse` expectancy
includes `Daemon`)

**Issue:** `daemon_attest_and_decide` attests **two** registry rows —
`DaclPackageSidGrant` *and* `DaclAncestorTraverse`, both `Abort`-outcome, both declared expected
at `(Daemon, None)` — from a single input:

```rust
pub(crate) fn granted_write_access(&self) -> bool { !self.write_applied.is_empty() }
...
if !dacl_guard_applied {
    return DaemonAttestationDecision::Abort { layer: "DaclPackageSidGrant", ... };
}
```

`write_applied` is populated only by pass 2 (the workspace write grant). `traverse_applied` —
the vector pass 1 and pass 3 populate, i.e. everything `DaclAncestorTraverse` actually describes
— is never read by the gate. The `DaclAncestorTraverse` row's negative is therefore
unrepresentable on the daemon arm, and when the predicate does fire the abort always names
`DaclPackageSidGrant`, so a `DaclAncestorTraverse` failure is unreportable even in principle.

WR-06 accepted the pass-1/pass-3 skip arms as "under-granting, never under-confining", which
justifies not *aborting* — it does not justify a distinct `Abort`-outcome registry row sharing an
unrelated row's input. This is the "green by absence" shape the SPEC's Structural constraints
section names as the failure mode this phase exists to close, one row over from where D-37
looked.

**Fix:** Give the row its own input:

```rust
pub(crate) fn granted_ancestor_traverse(&self) -> bool { !self.traverse_applied.is_empty() }
```

and pass it as a separate `ancestor_traverse_applied` parameter, with its own arm naming
`"DaclAncestorTraverse"` (non-aborting per D-37's contract-exempt rule if you keep WR-06's
disposition — but *classified separately*, so the two rows stop sharing one fact). Extend
`daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition` to require the two
inputs be distinct expressions, exactly as it already does for
`wfp_filters_installed`/`network_scoping_required`.

---

### WR-32: raw `file:line` citations survive inside `layer_registry.rs`'s own doc comments, outside WR-19's whole-document gate

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:536-540, 546-550`

**Issue:** WR-19 built `every_spec_symbol_citation_resolves_to_a_real_definition` for the SPEC
document and `spec_call_site_cells_match_registry_call_sites` for the registry table. Neither
covers the registry *source's* own prose, which still carries the pre-WR-08 form:

```rust
/// `"file:line"` citations of the enforcing call site(s). ...
pub call_sites: &'static [&'static str],
...
/// `token_arm` string constants naming `WindowsTokenArm` variants
/// (`launch.rs:1237-1278`) by name, ...
```

The first is now factually wrong (every entry uses `file.rs::Symbol`); the second is a raw line
citation of exactly the class NR3-08 → WR-08 → WR-19 have each closed only within the scope the
prior round checked — the fourth consecutive instance of that pattern.

**Fix:** Update the `call_sites` doc to describe the symbol form, replace `launch.rs:1237-1278`
with `launch.rs::select_windows_token_arm` (or whichever symbol declares the arms), and extend
`every_spec_symbol_citation_resolves_to_a_real_definition`'s scan to `layer_registry.rs`'s own
text — or add a companion test asserting no `\w+\.rs:\d+` pattern appears anywhere in
`layer_registry.rs`.

---

### WR-33: the new D-37 daemon test creates and DACL-mutates a directory under world-writable `%PUBLIC%` without exclusive-create semantics or panic-safe cleanup

**File:** `crates/nono-cli/src/agent_daemon/launch.rs` (`daemon_dacl_guard_apply_succeeds_when_immediate_ancestor_is_non_owned`, added by 117-28)

**Issue:** The test's chosen fixture is `%PUBLIC%\nono-test-d37-daemon-dacl-<pid>-<nanos>`,
created with `std::fs::create_dir_all`. `create_dir_all` succeeds on an **already-existing**
path, including a pre-planted directory junction or symlink. Any local user can write to
`C:\Users\Public`. The test then runs `DaemonDaclGuard::apply`, which calls
`grant_sid_write_on_path(workspace, TEST_PACKAGE_SID, /*inheritable=*/true)` — writing an
inheritable write-class ACE to whatever that path resolves to. Cleanup runs only on the
`Ok(guard)`/precondition-panic arms; a panic inside `apply` leaves the directory and its ACE
behind on a shared, world-readable location.

The `<pid>-<nanos>` name makes this hard to win in practice, which is why it is a Warning — but a
security-test suite should not choose a world-writable shared directory when the same D-37
condition is reachable with an exclusive create.

**Fix:** Use `std::fs::create_dir` (fails if the path exists) rather than `create_dir_all`, and
wrap the body in a cleanup guard so the directory is removed on any unwind:

```rust
std::fs::create_dir(&workspace).expect("exclusive create under %PUBLIC%");
struct Cleanup(PathBuf);
impl Drop for Cleanup { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
let _cleanup = Cleanup(workspace.clone());
```

Also verify the created directory is not a reparse point before applying
(`workspace.symlink_metadata()?.file_type().is_symlink()`).

---

### WR-34: `plant_mandatory_label_with_flags` is duplicated verbatim across two crates' test modules

**File:** `crates/nono/src/sandbox/windows.rs:3281` and
`crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:827`

**Issue:** Two byte-similar ~80-line `unsafe` Win32 helpers (SDDL construction,
`ConvertStringSecurityDescriptorToSecurityDescriptorW`, `GetSecurityDescriptorSacl`,
`SetNamedSecurityInfoW`, `LocalFree`) exist in separate crates because there is no shared
test-support crate. CR-01/WR-18's hardening depends on both planting the *same* ACE shape; a fix
or fixture change applied to one will not reach the other, and neither is covered by the
`is_ident_boundary` style of shared-definition discipline this round introduced for the test
matchers.

**Fix:** Promote the helper into the `nono` crate behind a `#[cfg(feature = "test-support")]`
`pub` module (mirroring the existing `layer-fault-injection` feature-gating precedent), and have
`labels_guard.rs`'s tests call it via `nono::test_support::plant_mandatory_label_with_flags`.
Failing that, add a source-text test asserting the two bodies stay in sync (the same
`include_str!` discovery idiom already used by `daemon_enum_segment`).

---

## Verification notes (what was independently re-derived, not taken from SUMMARYs)

- **Class enumeration, mandatory-label readers.** Independently enumerated via
  `grep -n "SYSTEM_MANDATORY_LABEL_ACE\|GetAce\|LABEL_SECURITY_INFORMATION\|low_integrity_label"`
  over `crates/` and `bindings/`. Exactly one SACL walk (`low_integrity_label_ace`); three
  consumers. WR-18's "exactly two, both hardened" is inaccurate — see WR-25.
- **Class enumeration, chain-advancing emitters.** `grep -rn "advance_chain("` → three
  production sites (`telemetry/mod.rs:402`, `:494`, `:598`). All three hold `inner`'s lock
  across their emit. WR-21's parity claim **holds**; the deadlock justification does not (WR-30).
- **Class enumeration, D-28 emission sites.** CR-03's three in-function sites are correctly
  gated on one shared `layer_detail`. A fourth site exists downstream (`write_security_event_log`
  stderr fallback) — CR-05.
- **Shared `is_ident_boundary`.** Single definition at `tests/common/mod.rs:41`; both
  `contains_fn_exact` (meta test) and `content_defines_symbol` (selfcheck) call it. Genuine, not
  a copy. `line_contains_word` re-implements only the *leading*-boundary half, with the asymmetry
  documented. Verified green by execution.
- **D-37 three-arm classification.** `(false,_,_)→NotApplicable`, `(true,true,true)→NotApplicable`,
  `(true,true,false)→NotApplied`, `(true,false,_)→Applied` present on **both** guards
  (`dacl_guard.rs:470-481` and `:715-726`). `stopped_at_non_owned` is set on the `Ok(false)` arm
  of both walks. The third arm is genuinely unreachable through `snapshot_and_apply*` today and
  the tests say so rather than faking reachability. `D37_CLASSIFICATION_TABLE` is non-vacuous
  (row 2 expects `Applied`, distinguishable from rows 0/1's `NotApplicable`). This part of round
  3 is sound; the residual issues are WR-28 (sibling file, opposite rule) and WR-29 (tighten
  path).
- **`NotApplied` reachability of the ancestor rows.** `mod.rs`'s `map_or(NotApplied, ..)`
  default arm still covers "the guard never ran", so a `None` guard does not vanish. Confirmed.
- **WR-20's host-blocked test read closely.** `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`
  is structurally correct and *would* pin the behavior if the host could run it: it plants a
  Low-RID label whose mask is asserted `!=` the rule's own wanted mask, so the residue predicate
  (`prior_rid == LOW && prior_mask == wanted && !INHERIT_ONLY`) cannot fire, and the only two
  outcomes available are `SkipNotOwned` (ownership gate first — the property under test) and
  `SkipPreExistingLabel` (ownership gate second — the perturbation). It confirms both variables
  independently before exercising the guard, and it fails loudly at setup rather than skipping.
  The one gap: it does not assert `coverage.application()`, so it pins the *entry* variant but
  not the downstream decision the reclassification actually changed — add
  `assert_eq!(coverage.application(), LayerApplication::NotApplied)` (per today's arithmetic) or
  `NotApplicable` (after WR-28's fix) so the reclassification's effect, not just its label, is
  pinned. Reproduced its documented loud failure on this host (failure 12 of 12).

---

_Reviewed: 2026-08-11T20:13:50Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
