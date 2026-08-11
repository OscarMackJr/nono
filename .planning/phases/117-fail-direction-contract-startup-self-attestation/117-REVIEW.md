---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-11T00:00:00Z
iteration: 5
depth: standard
files_reviewed: 33
files_reviewed_list:
  - .github/workflows/ci.yml
  - bindings/c/src/lib.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/network.rs
  - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/query_ext.rs
  - crates/nono-cli/src/telemetry/event.rs
  - crates/nono-cli/src/telemetry/mod.rs
  - crates/nono-cli/src/telemetry/windows.rs
  - crates/nono-cli/tests/layer_force_unavailable.rs
  - crates/nono-cli/tests/layer_registry_meta_test.rs
  - crates/nono-cli/tests/layer_registry_selfcheck.rs
  - crates/nono-shell-broker/Cargo.toml
  - crates/nono-shell-broker/src/main.rs
  - crates/nono/src/diagnostic/codes.rs
  - crates/nono/src/error.rs
  - crates/nono/src/lib.rs
  - crates/nono/src/machine_policy.rs
  - crates/nono/src/sandbox/windows.rs
  - proj/SPEC-windows-fail-direction-contract.md
findings:
  critical: 1
  warning: 10
  info: 0
  total: 11
status: issues_found
---

# Phase 117: Code Review Report (iteration 5 — gap-closure round 2 delta `97ff0962..HEAD`, plans 117-20..117-26)

**Reviewed:** 2026-08-11
**Depth:** standard (delta-scoped, with cross-file tracing of every guard → report → decision → operator-channel path touched by the round)
**Files Reviewed:** 33
**Status:** issues_found

## Summary

Scope was `git diff 97ff0962..HEAD` (plans 117-20..117-26), verified against what
`117-REVIEW.iter4.md` actually asserted for CR-01, CR-02 and WR-01..WR-11. Every claimed closure
was re-derived from current source rather than from the plan summaries or the SPEC's own
review-fix ledger.

**The mechanical quality of this round is materially better than iteration 4's.** CR-01 is
genuinely closed at the root (the reader was widened, the predicate reads `AceFlags`, and the
regression test plants a real inherit-only ACE through raw SDDL rather than asserting on a
synthetic value). WR-01, WR-03, WR-05, WR-09 and WR-10 are all genuinely closed. WR-02 and WR-06
are honest documentation corrections that I verified against the code they describe — in
particular `DaemonDaclGuard::apply` pass 2 really is fail-closed end-to-end (`Ok(false)` →
`revert_all()` + `Err`, `agent_daemon/launch.rs:212-220`), so this round's replacement premise is
true where iteration 4's was false.

The defects below are, again, in what the fixes *mean* rather than whether they build — and the
phase's established pattern holds: **two of this round's own fixes contradict each other.** Plan
117-22 (WR-06) wrote a rationale saying a skipped ancestor-traverse grant is under-granting and
therefore carries no security-relevant downgrade signal; Plan 117-23 (WR-07) landed in the same
round making the *identical physical condition* on the CLI mirror a visible downgrade. And CR-02's
D-28 fix was applied to 1 of the 3 sites in the same function that put layer names on the shared
console.

**Closure verdicts:**

| Finding | Claimed by | Verdict |
|---|---|---|
| CR-01 (BLOCKER) | 117-20 | **RESOLVED.** `low_integrity_label_ace` returns `header.AceFlags` (`windows.rs:2162-2169`); the predicate requires `(u32::from(prior_flags) & INHERIT_ONLY_ACE) == 0` (`labels_guard.rs:302-305`); `inherit_only_residue_is_not_treated_as_already_covered` plants a real `S:(ML;OICIIO;…;LW)` ACE via `ConvertStringSecurityDescriptorToSecurityDescriptorW`+`SetNamedSecurityInfoW` and asserts both `SkipPreExistingLabel` and `application() != Applied`. Rejecting only `INHERIT_ONLY_ACE` (rather than iter4's suggested exact-flag equality) is the *more* correct choice — OI/CI/NP/ID do not make an ML ACE inert. One sibling reader was left behind (WR-18). |
| CR-02 (BLOCKER) | 117-21 | **PARTIAL — class coverage failure.** The gate itself is correct and the sibling withholding test is real. But the two audit-emission warns 30 lines above it still interpolate the specific `LayerId` names into their message text, ungated (**CR-03**); the replacement pointer ("see the audit ledger") names a channel the event never reaches (**WR-15**); and `log_target_is_private()` equates "a `--log-file` opened" with "the child cannot read it" without checking (**WR-16**). |
| WR-01 | 117-20 | **RESOLVED.** `match path_is_owned_by_current_user(&rule.path)` at `:258` now precedes the residue push at `:314`; a non-owned path can no longer record `AlreadyAtRequiredLevel`. Side effect not recorded anywhere: WR-20. |
| WR-02 | 117-20 | **RESOLVED AS DOCUMENTED.** Module doc `:17-24` and the variant doc `:73-78` now state first-out-restores truthfully; `residue_is_not_reverted_on_drop` pins the non-revert property. The SPEC row says "accepted, not mechanically fixed" — honest. |
| WR-03 | 117-20 | **RESOLVED.** Pre-label mask is `label_mask_for_access_mode(AccessMode::Write)` behind an `assert_ne!` precondition (`:744-749`), the test asserts `matches!(guard.entries[0], AppliedLabel::SkipPreExistingLabel)`, and the inert `_skip_variant_reference` binding is gone. |
| WR-04 | 117-21 | **PARTIAL.** `render_error_for_operator` is genuinely reachable from `main.rs:221`. But it points the operator at `nono setup --check-only`, which has no mandatory-label-residue check at all (WR-17). |
| WR-05 | 117-25 | **RESOLVED.** Removed from `windows-layer-fault-injection` only; the `windows-security` job's copy (`ci.yml:243`) is load-bearing via `windows-test-harness.ps1:189` and was correctly left. |
| WR-06 | 117-22 | **RESOLVED (premise now true).** I verified pass 1 (`:177-183`) and pass 3 (`:242-251`) do have skip/`break` arms and pass 2 (`:212-220`) is genuinely fail-closed, so the corrected "under-granting, never under-confining" rationale holds. The cross-mirror test was added — but it is name-only, structurally cannot catch a *rationale* divergence, and is now contradicted by WR-07's own fix (WR-12), and it reproduces WR-11's parse fragility (WR-14). |
| WR-07 | 117-23 | **PARTIAL — the fix over-corrects.** `walked` makes `application()` genuinely three-valued and the gate-level test proves the row downgrades rather than vanishing. But the new `PartiallyApplied` arm reclassifies the D-04-documented "stop at the first non-owned ancestor" contract outcome as a coverage gap, contradicting the SPEC's own "Contract-exempt skips are NOT downgrades" rule, the surviving first half of its own comment in `mod.rs`, and the daemon-mirror rationale landed in the same round (WR-12). |
| WR-08 | 117-24 / 117-26 | **PARTIAL.** All 9 rows converted, every symbol verified present, and the SPEC's `## Layer registry` table now matches. But `content_defines_symbol` has no trailing word-boundary check (its sibling in the same plan does) and discards the type qualifier, so the "a renamed symbol fails the build" claim is false for prefix-preserving renames (WR-13); and the SPEC's *Manual verification* section still carries a stale `launch.rs:2190` line citation (WR-19). |
| WR-09 | 117-25 | **RESOLVED.** `chain`/`session_id`/`config` are private again; `advance_and_snapshot` is the single accessor and always routes through `advance_chain`. Residuals in WR-21. |
| WR-10 | 117-24 | **RESOLVED.** `c == '!'` is now in the boundary rejection set, the definition-line prefix check landed, a rejected match does not short-circuit the scan, and three unit tests pin all three behaviours. |
| WR-11 | 117-22 | **PARTIAL.** The named test is properly anchored (`DAEMON-DECISION-ENUM`, occurrence count asserted `== 2`) and `daemon_attest_and_decide_result_matches_exhaustively` is a real compile-time gate. The brand-new sibling test added by the *same plan* reintroduces the exact first-textual-occurrence parse the finding was about (WR-14). |

---

## Critical Issues

### CR-03 (BLOCKER): CR-02's D-28 gate was applied to 1 of the 3 sites in the same function that put specific `LayerId` names on the shared console — the two audit-emission warns still leak them as message text, and the new D-28 test is blind to them by construction

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1496-1514` (the two leaking warns),
`:1538-1555` (the site that *was* gated), `:3838-3853` (the test that cannot see the leak)

**Issue:** Plan 117-21 gated the unconditional downgrade warn on
`cli_bootstrap::log_target_is_private()`. Thirty lines above it, inside the same
`ProceedDowngraded` arm, two warns are untouched:

```rust
match crate::telemetry::SECURITY_LAYER.get() {
    None => {
        tracing::warn!(
            "attestation downgrade audit emission unavailable \
             (AUD-04: SecurityEventLayer not initialized) — \
             proceeding per AUD-04's non-fatal contract; \
             downgraded_layers={dedup_key}"          // <-- specific LayerId names
        );
    }
    Some(security_layer) => {
        if let Err(e) = security_layer.emit_attestation_event(&downgraded_refs) {
            tracing::warn!(
                "attestation downgrade audit emission failed (AUD-04) — \
                 proceeding per AUD-04's non-fatal contract: {e}; \
                 downgraded_layers={dedup_key}"      // <-- specific LayerId names
            );
        }
    }
}
```

`dedup_key` is `names.join(",")` over `format!("{id:?}")` of every downgraded `LayerId`
(`:1484-1486`) — the exact payload CR-02 was raised to keep off the shared channel. On the default
no-`--log-file` arm and the file-open-failure fallback arm, `init_tracing_with_security` builds the
fmt layer with `.with_writer(std::io::stderr)` (`cli_bootstrap.rs:241-251`), which the confined
child shares on the non-detached-stdio path. D-28 does not have an "only on the success path"
qualifier.

Both triggers are reachable in a shipped build: `SECURITY_LAYER.get()` is `None` whenever
`init_tracing` has not run (or lost the `OnceLock::set` race), and `emit_attestation_event`
returns `Err("mutex poisoned")` after any panic while the telemetry mutex is held — precisely the
degraded conditions during which an operator is *most* likely to be running a confined agent that
should not be handed a map of which layer is inert.

**This is not an oversight the tests could have caught, because the test was written to the
narrower shape.** `proceed_downgraded_success_path_withholds_layer_names_on_the_shared_console_channel`
asserts only that no captured event carries a *field* named `downgraded_layers` (`:3845-3852`),
and its sibling's doc comment states outright that the pre-existing warns "interpolate
`downgraded_layers={dedup_key}` as plain text inside their `message` field" (`:3701-3707`). The
author knew about both sites, gated one, and wrote an assertion that structurally cannot observe
the other two. This is the "tests that name their targets are blind by construction" class in its
purest form: the predicate was chosen to match the fix rather than the property.

**Fix:** gate all three sites on the same predicate, and widen the test to the property rather
than the mechanism.

```rust
// launch.rs — build the detail suffix once, per-channel.
let layer_detail = if crate::cli_bootstrap::log_target_is_private() {
    format!("; downgraded_layers={dedup_key}")
} else {
    String::new()   // D-28: names withheld from the shared console
};

match crate::telemetry::SECURITY_LAYER.get() {
    None => tracing::warn!(
        downgraded_count = downgraded.len(),
        "attestation downgrade audit emission unavailable (AUD-04) — proceeding{layer_detail}"
    ),
    Some(security_layer) => {
        if let Err(e) = security_layer.emit_attestation_event(&downgraded_refs) {
            tracing::warn!(
                downgraded_count = downgraded.len(),
                "attestation downgrade audit emission failed (AUD-04): {e}{layer_detail}"
            );
        }
    }
}
```

and change the withholding test to scan the rendered event **text** as well as the field names,
for every `LayerId` name in the downgrade set — so it fails for a leak in a message, a field, a
`target`, or anywhere else:

```rust
let leaked = events.iter().flat_map(|f| f.iter()).any(|(_, value)| {
    downgraded_ids.iter().any(|id| value.contains(&format!("{id:?}")))
});
assert!(!leaked, "no captured event may name a LayerId on the shared console (D-28): {events:?}");
```

Drive it once through the `SECURITY_LAYER == None` arm (which today is the arm the unit test
actually takes) so the assertion covers the audit-failure paths, not only the success path.

---

## Warnings

### WR-12: WR-07's fix reclassifies a documented, contract-exempt condition as a coverage gap — and directly contradicts the rationale plan 117-22 wrote for the identical physical condition in the daemon mirror, in the same round

**File:** `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:435-455`, `:655-676`,
`:519-525`; `crates/nono-cli/src/exec_strategy_windows/mod.rs:404-421`;
`crates/nono-cli/src/agent_daemon/launch.rs:1237-1266`;
`proj/SPEC-windows-fail-direction-contract.md:232-238`

**Issue:** Plan 117-23 made a walk that ran but granted zero owned ancestors report
`PartiallyApplied` → `ProceedDowngraded` → D-27 banner. The condition it now downgrades on is the
first non-owned ancestor stopping the walk — which every other artifact in the tree calls the
*contract outcome*, not a gap:

- The guard's own struct doc, left in place: "Non-owned ancestor (`Ok(false)`): STOP the walk (the
  D-04 split — proves the runtime guard structurally cannot touch `C:\Users`/`C:\`). Not an error."
  (`dacl_guard.rs:521-522`)
- The `mod.rs` comment contradicts itself inside one block: "Both walks are fail-closed with **no
  skip arm distinct from 'nothing owned to grant'** … STOP at the first non-owned one (reaching the
  cwd from there up relies on the lowbox's bypass-traverse). WR-07 corrected the coverage rule: an
  empty grant set from a walk that DID run … **is a real coverage gap**." (`mod.rs:404-418`)
- The SPEC's own standing rule: "**Contract-exempt skips are NOT downgrades.** … Counting it would
  fire the banner on essentially every profile, recreating the permanently-on warning RF-06
  removed." (`:232-238`)
- **Plan 117-22, landed in the same round, argues the opposite for the same condition**: the
  daemon's pass 3 (`workspace.ancestors().skip(1)`, `break` at the first non-owned ancestor,
  `agent_daemon/launch.rs:242-251`) is byte-for-byte the same walk, and its new doc comment says a
  skipped traverse grant "only narrows an ancillary discoverability path the daemon relies on the
  lowbox bypass-traverse privilege to reach anyway … so a downgrade state here would carry no
  security-relevant signal an operator needs surfaced."

Both cannot be right. Either the condition is security-relevant (and the daemon mirror is now
under-reporting) or it is not (and the CLI mirror now fires a permanent banner). Concretely, on a
workspace whose immediate parent is not user-owned — `C:\proj`, a repo under an admin-created
`C:\dev`, anything one level under a drive root — `AppliedAncestorTraverseGuard` grants nothing,
reports `PartiallyApplied`, and the non-silenceable D-27 banner fires on **every session**. RF-06
is explicit that this shape is the failure it exists to prevent ("a permanently-on warning is
functionally identical to no warning").

The cross-mirror test WR-06 added cannot catch this: it compares variant *name sets*, and both
enums' names are unchanged. The divergence is in the classification rule, which nothing compares.

**Fix:** decide the semantics once, in the SPEC, and make both mirrors implement it. The
defensible reading is 117-22's: a stop at the first non-owned ancestor is under-granting, so it is
contract-exempt. If so, distinguish the two states in the guard rather than collapsing them —

```rust
pub(crate) struct AppliedAncestorTraverseGuard {
    applied: Vec<PathBuf>,
    /// Set when the walk ended by hitting a non-owned ancestor (contract-exempt,
    /// D-04) rather than by exhausting an owned chain.
    stopped_at_non_owned: bool,
    walked: bool,
}

pub(crate) fn application(&self) -> LayerApplication {
    match (self.walked, self.applied.is_empty(), self.stopped_at_non_owned) {
        (false, _, _)        => LayerApplication::NotApplicable, // no ancestors at all
        (true, true, true)   => LayerApplication::NotApplicable, // every ancestor exempt (D-04)
        (true, true, false)  => LayerApplication::NotApplied,    // walked, nothing granted, not exempt
        (true, false, _)     => LayerApplication::Applied,
    }
}
```

— and then extend the cross-mirror test from variant names to the classification rule itself (e.g.
a shared table of `(condition, LayerApplication)` that both mirrors are asserted against), so the
next round cannot land two opposite rationales for one condition again. Reconcile the
self-contradicting `mod.rs` comment and `dacl_guard.rs:521-522` in the same change.

---

### WR-13: `content_defines_symbol` omits the trailing word-boundary check its own sibling in the same plan implements, and discards the type qualifier — so the SPEC's "a renamed enforcing function fails the build" claim is false

**File:** `crates/nono-cli/tests/layer_registry_selfcheck.rs:179-224`,
`crates/nono-cli/tests/layer_registry_meta_test.rs:279-299`,
`proj/SPEC-windows-fail-direction-contract.md:284`

**Issue:** Plan 117-24 wrote two source-scanning matchers in one pass.
`contains_fn_exact` (meta test) got both the prefix rule **and** a trailing boundary rule.
`content_defines_symbol` (selfcheck) got only the prefix rule:

```rust
let last = symbol.rsplit("::").next().unwrap_or(symbol);
let needles = [format!("fn {last}"), format!("impl {last}")];
// ... prefix check only; nothing inspects the character AFTER the needle
```

Two consequences, both live today:

1. **Prefix-preserving renames pass.** `restricted_token.rs::create_restricted_token_with_sid`
   would still be satisfied after a rename to `create_restricted_token_with_sid_v2`, because
   `fn create_restricted_token_with_sid` remains a substring. The SPEC's WR-08 row and the
   selfcheck's own module doc both claim "a renamed or removed enforcing function fails the
   build" — for the removal half that is true; for the rename half it is not. I confirmed the
   mechanism empirically against `nono-shell-broker/src/main.rs`, where the cited symbol `::run`
   is satisfied by `fn run_fails_when_app_container_forced_unavailable` (`:1902`) just as well as
   by the real `pub fn run` (`:459`).
2. **The type qualifier is thrown away**, so `Type::method` citations verify only the method name.
   `WfpEgressFilters`'s `network.rs::WfpNetworkBackend::install` and `FirewallRulesEgress`'s
   `network.rs::FirewallRulesNetworkBackend::install` both reduce to the needle `fn install` in the
   same file and are now indistinguishable — as are
   `agent_daemon/launch.rs::DaemonDaclGuard::apply` (needle `fn apply`) and
   `dacl_guard.rs::AppliedDaclGrantsGuard::snapshot_and_apply` (needle `fn snapshot_and_apply`,
   shared by three guards in that file). This was the only precision the citations had left after
   the line numbers were removed.

**Fix:** lift the boundary rule out of `contains_fn_exact` into a shared helper both files use
(they are in the same test crate), and resolve `Type::method` through the enclosing `impl`:

```rust
fn is_ident_boundary(next: Option<char>) -> bool {
    !matches!(next, Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '!')
}

// and for `Type::method`, require the nearest preceding `impl <Type>` before the match:
fn content_defines_symbol(content: &str, symbol: &str) -> bool {
    let mut parts = symbol.rsplit("::");
    let method = parts.next().unwrap_or(symbol);
    let ty = parts.next();                       // Some("DaemonDaclGuard") for Type::method
    // ... on each candidate match, additionally require that the last `impl ` line
    // above `match_pos` names `ty` when `ty.is_some()`.
}
```

---

### WR-14: WR-11's anchor hardening was applied to 1 of 2 tests — the brand-new cross-mirror test added by the same plan reintroduces the first-textual-occurrence parse the finding was about

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:2286-2292` (the new test),
`:2134-2178` (`parse_enum_variant_names`), `:2229-2251` (the correctly hardened test)

**Issue:** `daemon_attestation_decision_is_deliberately_two_state` was properly hardened: it
anchors on `DAEMON-DECISION-ENUM` and asserts the marker occurs exactly twice before trusting the
split. `every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence` — added in the
same commit — does not:

```rust
let daemon_src = include_str!("launch.rs");
let daemon_variants = parse_enum_variant_names(daemon_src, "enum DaemonAttestationDecision {");
```

`parse_enum_variant_names` does `src.split(enum_marker).nth(1)`, i.e. the first textual occurrence.
That literal now appears **three** times in `launch.rs`: the declaration (`:1281`), the hardened
test's call (`:2292`), and this test's call (`:2288`). It works only because the declaration
happens to come first — verbatim the property WR-11 said was unacceptable. Moving `mod tests`
above `mod windows_impl`, or a `#[cfg(test)]` reorder, silently re-targets the parse to a test's
own argument string and yields an empty/garbage variant list.

Secondary: `parse_enum_variant_names`'s candidate filter excludes `///`, `#`, `}` and blank lines
but **not** plain `//` comments. A `// note` line at variant indent inside either enum body is
parsed as a variant named `// note` and fails both tests with a confusing message.

**Fix:** route both tests through the anchor, and reject `//` in the filter:

```rust
fn daemon_enum_segment(src: &str) -> &str {
    let anchor = "DAEMON-DECISION-ENUM";
    assert_eq!(src.matches(anchor).count(), 3, "anchor duplicated or removed");
    src.split(anchor).nth(1).expect("anchor precedes the declaration")
}
// both tests: parse_enum_variant_names(daemon_enum_segment(daemon_src), "enum DaemonAttestationDecision {")

// and in candidate_indent:
|| trimmed.starts_with("//")
```

(Bumping the expected count to 3 as the anchor gains a third reference is itself the loud signal
the test is meant to produce.)

---

### WR-15: CR-02's replacement pointer names the wrong channel — `LayerAttestationDowngraded` never reaches the audit ledger, and with telemetry disabled the layer names reach no channel at all

**File:** `crates/nono-cli/src/output.rs:117-120`,
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1547-1554`,
`crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs:126-142`,
`crates/nono-cli/src/telemetry/windows.rs:181-214`

**Issue:** The fix replaced "see diagnostic output for details" with "see the audit ledger for
layer-level detail" (banner) and "see the audit ledger" (the withheld-detail warn). In this
codebase "audit ledger" means `<session_dir>/audit-events.ndjson`, written by `AuditRecorder` and
read by `nono audit show` — the same artifact `labels_guard.rs`'s own `audit_flush_before_drop`
test asserts against (`:1158`, `:1229`).

`emit_attestation_event` never touches it. It calls `emit_security_event`, which writes the
Windows **Application Event Log** (`write_security_event_log`) and re-emits a
`tracing::warn!(target: "nono_security", …)` whose field list is
`event_type/event_id/agent_pid/path_hash/host/session_id/chain_head/timestamp_unix_ms` — no
`downgraded_layers`. So an operator who follows the banner's instruction finds nothing, which is
NR3-04's original defect reproduced with a different wrong destination.

Two aggravating factors:

1. **Telemetry opt-out erases the detail entirely.** `emit_attestation_event` only emits when
   `enabled` (`attestation_downgrade_event.rs:126`), and `TelemetryConfig.enabled` is an HKLM
   `MachineEgressPolicy` control an admin can set to `0` (`main.rs:189-193`). On a
   telemetry-disabled host with no `--log-file`, a downgraded launch produces a count on stderr
   and the layer names nowhere at all.
2. **The one channel it does use rests on an explicitly unverified assumption.** The SPEC's own
   "Cross-cutting security assumptions" table lists `etw-applog-child-readability` as an open
   manual gap — whether an AppContainer/Low-IL child can read the Application log has never been
   checked. CR-02's fix now makes that unverified channel the *sole* carrier of the detail.

**Fix:** point the banner and the warn at a destination that actually holds the data. Either
append a `layer_attestation_downgraded` record to the session `AuditRecorder` ledger (which makes
both strings true and is the channel the phase's own tests already treat as the audit surface), or
name the real destination:

```rust
"{downgraded_count} confinement layer(s) could not be fully confirmed at startup — \
 layer detail is in the Windows Application event log (source `nono`, event id {id}); \
 re-run with --log-file <path> to capture it locally"
```

and, if telemetry is disabled, log the detail to the private log-file channel unconditionally
rather than dropping it — the D-14 degrade-not-abort policy is about *emission sinks*, not about
withholding a security-relevant fact from the operator who owns the machine.

---

### WR-16: `log_target_is_private()` treats "a `--log-file` was opened" as proof the confined child cannot read it, without validating the path against the child's own granted filesystem policy

**File:** `crates/nono-cli/src/cli_bootstrap.rs:13-40`, `:221-233`

**Issue:** The predicate's doc comment asserts the file-log arm "writes to a channel private to
the operator (a file the confined child does not share)". Nothing checks that. `--log-file` takes
an arbitrary operator-supplied path, and the overwhelmingly common convenience invocation puts it
inside the very directory the child is granted read+write on:

```
nono run --workspace . --log-file .\nono.log --profile claude-code -- claude
```

`prepare_live_windows_launch` grants the workspace to the package SID (write, `(OI)(CI)`), so the
child can `type nono.log` and read the full `downgraded_layers=` line the gate deliberately
withheld from its console. The gate is correctly *placed* and its predicate is correctly *wide*,
but the predicate tests the wrong proposition: "is tracing going to a file" rather than "is that
file outside everything this child can read".

**Fix:** make the predicate mean what its name claims, by validating the resolved log path against
the compiled policy at init time:

```rust
// cli_bootstrap.rs — set the flag only if the log file is outside every granted path.
let private = std::fs::canonicalize(path)
    .ok()
    .map(|log| !compiled_policy_paths().iter().any(|granted| log.starts_with(granted)))
    .unwrap_or(false);   // fail-secure: unknown -> not private
#[cfg(target_os = "windows")]
TRACING_LOG_TARGET_IS_PRIVATE.store(private, Ordering::Relaxed);
```

using `Path::starts_with` (component comparison, per CLAUDE.md), canonicalizing both sides, and
defaulting to `false` on any resolution error. If the policy is not yet compiled at
`init_tracing` time, defer the decision to a lazily-computed check at the gate instead of pinning
`true` at init.

---

### WR-17: WR-04's remediation names a command that cannot diagnose the cause it names, and fires for every `LayerAttestationFailed` regardless of layer

**File:** `crates/nono-cli/src/main.rs:228-256`, `crates/nono/src/error.rs:505-509`,
`crates/nono/src/diagnostic/codes.rs:71-82`, `crates/nono-cli/src/setup.rs:1315-1336`

**Issue:** The rendered second line is:

> `nono:   layer {layer} could not be confirmed. If a prior nono session exited abnormally, its
> residual confinement state may remain — run `nono setup --check-only` to diagnose.`

`nono setup --check-only` runs `print_check_only_summary`, which prints sandbox support status,
storage roots, the Windows foundation report, WFP readiness, trust-root status and nono.exe's own
Authenticode subject (`setup.rs:1315-1336`). It performs **no** mandatory-label inspection on any
policy path. An operator who hits the stale-residue abort and follows the instruction gets a clean
"Installation verified!" and no closer to a diagnosis.

The core crate already knows the right answer and it never reaches the operator — `codes.rs:74-79`
documents the remedy as "`icacls <path> /setintegritylevel Medium` for `MandatoryIntegrityLabel`".

Second problem: `NonoError::remediation()` maps **every** `LayerAttestationFailed` to
`ClearStaleLayerResidue` (`error.rs:505-509`, `_ => None` after it). A `WfpEgressFilters`,
`AppContainerProfile`, `JobObjectContainment` or forced-unavailable-seam abort now tells the
operator to look for residue from a prior abnormal exit — a wrong lead for four of the five layers
that can produce this error.

**Fix:** make the guidance layer-specific and actionable:

```rust
fn render_error_for_operator(e: &nono::NonoError) -> Vec<String> {
    let mut lines = vec![format!("nono: {e}")];
    if let Some(nono::NonoRemediation::ClearStaleLayerResidue { layer }) = e.remediation() {
        lines.push(match layer.as_str() {
            "MandatoryIntegrityLabel" => format!(
                "nono:   {layer} could not be confirmed. A prior session that exited abnormally \
                 may have left mandatory-label ACEs on granted paths. Inspect with \
                 `icacls <granted-path>` and clear with \
                 `icacls <granted-path> /setintegritylevel Medium`."),
            _ => format!(
                "nono:   {layer} could not be confirmed at startup; see the Windows Application \
                 event log (source `nono`) for the per-layer attestation record."),
        });
    }
    lines
}
```

and update the SPEC's WR-04 row, which currently reads as if the operator gained actionable
guidance.

---

### WR-18: CR-01's class-coverage gap — `low_integrity_label_rid`, the other mandatory-label reader in the same file, still ignores `AceFlags`, so `is_low_integrity_compatible_dir` accepts a structurally-inert inherit-only label

**File:** `crates/nono/src/sandbox/windows.rs:1982-2070`, `:2186-2205`,
`crates/nono/src/sandbox/mod.rs:958-967`

**Issue:** `low_integrity_label_ace` was widened; the function directly above it in the same file
was not. `low_integrity_label_rid` walks the same SACL, filters on the same
`SYSTEM_MANDATORY_LABEL_ACE_TYPE`, and returns the RID without ever reading `header.AceFlags`. Its
one production consumer is:

```rust
pub fn is_low_integrity_compatible_dir(path: &Path) -> bool {
    // ...
    canonical.as_deref()
        .and_then(low_integrity_label_rid)
        .is_some_and(|rid| rid <= SECURITY_MANDATORY_LOW_RID as u32)
}
```

exported as `Sandbox::windows_supports_direct_writable_dir` — "whether a Windows directory is
directly writable by the current low-integrity restricted-launch path **without mutating the
directory's label**". A directory carrying only an `S:(ML;OICIIO;…;LW)` ACE has no effective label
at all, and this function reports it compatible. That is CR-01's exact proposition ("an inert ACE
is indistinguishable from an effective one") in a second reader.

Exposure today is latent: `grep -rn windows_supports_direct_writable_dir` finds no in-tree caller
outside `sandbox/mod.rs`'s own declaration, so this is a public library API with no CLI consumer.
It is still the same defect one function above the one that was just fixed, in a crate whose
`#[must_use] pub fn` surface is consumed by the C/Python/TypeScript bindings.

**Fix:** collapse the duplication rather than patching it twice —

```rust
fn low_integrity_label_rid(path: &Path) -> Option<u32> {
    // CR-01: an INHERIT_ONLY_ACE is never evaluated against the object it sits on.
    low_integrity_label_ace(path)
        .filter(|(_, _, flags)| (u32::from(*flags) & INHERIT_ONLY_ACE) == 0)
        .map(|(rid, _, _)| rid)
}
```

and add the mirror of `inherit_only_residue_is_not_treated_as_already_covered` asserting
`!is_low_integrity_compatible_dir(dir)` for an inherit-only-labelled directory.

---

### WR-19: the SPEC's *Manual verification* table still carries a stale `launch.rs:2190`/`:2194` line citation — the same WR-08 class, in the document plan 117-26 claimed to re-sync, outside `spec_matches_registry`'s coverage

**File:** `proj/SPEC-windows-fail-direction-contract.md:315`,
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1800`, `:2129`, `:2515`

**Issue:** The `BrokerAuthenticodeTrustGate` manual-verification row reads:

> The gate is skipped entirely under `is_dev_build_layout()` (`launch.rs:2190`/`:2194`)

`launch.rs:2186-2196` is `InitializeProcThreadAttributeList` buffer sizing. The real call sites are
`:1800` and `:2129`; the declaration is `:2515`. Plans 117-24 and 117-26 converted every citation in
the registry table and re-synced the `## Layer registry` table, but the *Manual verification*
section was not in scope and was not swept, so the document that "describes shipped behaviour"
still points a reader at unrelated code.

`spec_matches_registry` cannot catch it — as iteration 4 noted and this round did not change, it
compares `LayerId` **names** only, so no citation in this document is drift-checked against
anything.

**Fix:** convert to symbol form (`launch.rs::is_dev_build_layout`) in the same pass, and close the
structural hole by making the drift gate cover citations, not just names:

```rust
// layer_registry_selfcheck.rs
#[test]
fn spec_call_site_cells_match_registry_call_sites() {
    // for each SPEC row, parse the "Enforcing call site(s)" cell and assert it
    // equals the registry entry's `call_sites` joined — so the SPEC cannot drift
    // from the registry silently, which is how three passes of drift went uncaught.
}
```

and extend `content_defines_symbol`'s scan to every `file.rs::Symbol` citation appearing anywhere
in the SPEC, not only those inside `REGISTRY_ENTRIES`.

---

### WR-20: the WR-01 reordering silently loosens the coverage claim for non-owned paths carrying a third-party mandatory label — a state that used to downgrade now classifies contract-exempt, with no test and no ledger entry

**File:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:240-334`,
`proj/SPEC-windows-fail-direction-contract.md:277`

**Issue:** Before this round, `snapshot_and_apply` read the prior label first and the ownership
gate second, so a path that was **both** non-owned **and** carrying a foreign mandatory label
recorded `SkipPreExistingLabel` → counted in `skipped_pre_existing_label` → `PartiallyApplied` →
visible downgrade. After the reorder, `Ok(false)` records `SkipNotOwned` and `continue`s before
`low_integrity_label_ace` is ever called (`:259-268`), so the same path is now contract-exempt and
uncounted.

The change is defensible on the merits — nono structurally cannot label a path it does not own, so
the skip is exempt under D-02 either way — and I am not arguing the new classification is wrong.
The problem is that it is an unrecorded, untested loosening of the layer's coverage arithmetic
landed as a side effect of a finding about *provenance*. The SPEC's WR-01 ledger row describes only
the ordering change and its provenance benefit; nothing states that a class of path moved from
"downgrade" to "exempt". `guard_skips_path_not_owned_by_current_user` uses `C:\Windows`, whose
label state on a given host is incidental, so it pins neither behaviour.

**Fix:** record the reclassification in the SPEC's WR-01 row and pin it with a test that controls
both variables:

```rust
#[test]
fn non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap() {
    // a non-owned path carrying a mandatory label must record SkipNotOwned
    // (contract-exempt), never SkipPreExistingLabel — the ownership gate runs first.
}
```

---

### WR-21: `emit_attestation_event` now emits its audit record outside the chain mutex while `emit_override_event` still emits inside it, and `SecurityEventLayer::inner` stayed `pub(crate)` behind a comment that no longer explains it

**File:** `crates/nono-cli/src/telemetry/mod.rs:236-239`, `:387-429`,
`crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs:113-142`

**Issue:** Two residuals from an otherwise correct WR-09 fix.

1. **Lock scope changed.** Pre-fix, `emit_attestation_event` held `inner` across both
   `advance_chain` and `emit_security_event`. `advance_and_snapshot` releases the lock on return,
   so chain advancement and record emission are no longer atomic for this event type — two
   concurrent emitters can advance the chain in order A,B and write the Event Log records in order
   B,A. `emit_override_event` (`:347-385`) still holds the lock across its own emit, so the two
   sibling emitters on the same chain now have different atomicity guarantees. Each record still
   carries its own correct `chain_head`, so this is an ordering/consistency erosion rather than a
   forgery risk, but it is an undocumented divergence in a tamper-evidence mechanism.
2. **`inner` is still `pub(crate)` and its justification is gone.** Line 237 reads
   `// `pub(crate)` — see the NR3-05 follow-up comment on `SecurityEventLayerInner`.` — but Plan 25
   rewrote that comment to explain why the *fields* are private, and it says nothing about `inner`.
   The only remaining cross-module use is
   `attestation_downgrade_event.rs:208`'s `layer_clone.inner.lock().unwrap()` inside a
   `#[cfg(test)]` poisoning helper. So production code has crate-wide `.lock()` access to the
   audit-chain mutex for a test-only reason, behind a comment that points at nothing.

**Fix:** hold the lock across the emit (restoring parity with `emit_override_event`) by passing the
emit closure into the accessor, and narrow `inner`:

```rust
pub(crate) fn advance_and_emit<R>(
    &self,
    build_event_bytes: impl FnOnce(&str) -> Vec<u8>,
    emit: impl FnOnce(&str /*session_id*/, &str /*chain_head*/, bool /*enabled*/) -> R,
) -> Result<R, &'static str> { /* single lock, advance, then emit under it */ }

// telemetry/mod.rs
#[cfg(test)]
pub(crate) fn poison_for_test(&self) { /* the one thing the test needed `inner` for */ }
inner: std::sync::Arc<Mutex<SecurityEventLayerInner>>,   // private again
```

and delete the stale comment on line 237.

---

_Reviewed: 2026-08-11_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard — iteration 5 (gap-closure round 2 delta verification)_
