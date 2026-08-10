---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-10T00:00:00Z
iteration: 4
depth: standard
files_reviewed: 15
files_reviewed_list:
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/telemetry/mod.rs
  - crates/nono-cli/src/query_ext.rs
  - crates/nono/src/diagnostic/codes.rs
  - crates/nono/src/error.rs
  - crates/nono-cli/tests/layer_registry_selfcheck.rs
  - crates/nono-cli/tests/layer_registry_meta_test.rs
  - .github/workflows/ci.yml
  - proj/SPEC-windows-fail-direction-contract.md
findings:
  critical: 2
  warning: 11
  info: 0
  total: 13
status: issues_found
---

# Phase 117: Code Review Report (iteration 4 — gap-closure delta `90cd3ef8..HEAD`, plans 117-13..117-19)

**Reviewed:** 2026-08-10
**Depth:** standard (delta-scoped, with cross-file tracing of every touched guard → report → decision path)
**Files Reviewed:** 15
**Status:** issues_found

## Summary

Scope was the gap-closure delta only (`git diff 90cd3ef8..HEAD`), verified against what
`117-REVIEW.iter3.md` actually asserted for NR3-01..NR3-05 and NR3-08. I compiled and ran the
affected code rather than reading it alone: `cargo clippy -p nono-sandbox-cli --all-targets
--features layer-fault-injection -- -D warnings -D clippy::unwrap_used` is clean, and the
`labels_guard`, `layer_registry_meta_test`, `layer_registry_selfcheck`,
`attestation_downgrade_event` and `daemon_attestation_*` suites all pass. The mechanical work is
sound; the defects below are in what the fixes *mean*, not in whether they build.

**Closure verdicts:**

| Finding | Claimed by | Verdict |
|---|---|---|
| NR3-01 (BLOCKER) | 117-13 | **PARTIAL — and it introduced a new fail-open.** The lockout is genuinely fixed (`stale_residue_from_an_abnormal_exit_does_not_self_lock_out_the_next_launch` drives real on-disk state and passes). But the equivalence predicate is narrower than the ACE it claims equivalence to (CR-01), has no provenance constraint (WR-01), silently reverses the concurrency contract it claims to preserve (WR-02), quietly retargeted a pre-existing test (WR-03), and the remediation half is unreachable from the CLI (WR-04). |
| NR3-02 | 117-14 | **PARTIAL.** `application_of` is gone and both guards now read their own `applied` vec. But `application()` is a two-valued function of `applied.is_empty()`; it can never return `NotApplied` or `PartiallyApplied`, so NR3-02's own question — *what input makes this row report NOT established?* — still answers "none", and the new `NotApplicable` branch now silently drops an `expected: true` / `outcome: Abort` row from the decision (WR-07). |
| NR3-03 | 117-15 | **RESOLVED.** `dacl_session_sid_grant_is_not_claimed_anywhere` now exists, and its converse `every_consulting_row_with_an_expectancy_has_a_reported_field` is genuinely non-vacuous: I traced 7 rows that actually reach its assertion, and it *would* fire if `DaclSessionSidGrant`'s expectancy were re-added. The citation lie is gone. |
| NR3-04 | 117-16 | **RESOLVED FUNCTIONALLY, BUT VIOLATES D-28 (CR-02).** The unconditional structured `downgraded_layers` field is real and the capturing-subscriber test is non-vacuous. It puts specific layer names on the confined child's shared console by default, justified by a comment that states a falsehood about where `tracing` routes. |
| NR3-05 | 117-17 | **PARTIAL.** Deletion is a defensible call, but its stated premise is false: `DaemonDaclGuard::apply` pass 1 *does* have a skip arm and pass 3 *does* stop early, both partial-coverage states `granted_write_access()` cannot express (WR-06). The cross-mirror conformance test NR3-05 asked for was not written. The telemetry relocation itself is correct — the daemon lost nothing it still emits — but it widened the HMAC chain's visibility (WR-09). |
| NR3-08 | 117-15 / 117-19 | **PARTIAL, honestly disclosed.** 4 of 13 rows converted. I sampled 15 of the remaining line-number citations across 9 rows: **15 of 15 point at unrelated code.** `restricted_token.rs:55` is not the only survivor — it is representative (WR-08). |
| SC3 (ci.yml) | 117-18 | **PARTIAL.** The broker pre-build is correct and load-bearing (`target/release/nono-shell-broker.exe` is exactly `candidate_default` in `broker_launch_assigns_child_to_job_object`). `NONO_CI_HAS_WFP` is dead config in that job (WR-05). `contains_fn_exact` is a real improvement with a doc/code mismatch (WR-10). |

---

## Critical Issues

### CR-01 (BLOCKER): the `AlreadyAtRequiredLevel` equivalence predicate ignores ACE flags, so an inherit-only (structurally ineffective) mandatory-label ACE now counts as an applied layer — converting a would-be `Abort` into a fully-attested `Applied`

**File:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:210-228`,
`crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:312-322`,
`crates/nono/src/sandbox/windows.rs:2114-2165` (the reader), `:1056-1059` (the writer)

**Issue:** 117-13's self-heal declares two ACEs equivalent on `(rid, mask)` alone:

```rust
let wanted = label_mask_for_access_mode(rule.access);
if prior_rid == SECURITY_MANDATORY_LOW_RID as u32 && prior_mask == wanted {
    guard.entries.push(AppliedLabel::AlreadyAtRequiredLevel);   // counts toward `applied`
    continue;
}
```

A `SYSTEM_MANDATORY_LABEL_ACE` is not `(rid, mask)`. It is `(AceType, **AceFlags**, Mask, Sid)`.
`low_integrity_label_and_mask` reads `header.AceType` to filter, then returns `(*rid, mask)` and
**never inspects `header.AceFlags`** (`windows.rs:2123-2163`). `try_set_mandatory_label` writes
`"S:(ML;;0x{mask:X};;;LW)"` — an empty ACE-flags field, i.e. `AceFlags == 0`.

So the two ACEs below are indistinguishable to this predicate:

| ACE | AceFlags | Effect on the object |
|---|---|---|
| what nono writes | `0` | mandatory label **applies to this object** |
| `S:(ML;OICIIO;NWNX;;;LW)` | `INHERIT_ONLY_ACE (0x08)` \| OI \| CI | **no effect on this object at all** — inherit-only ACEs are not evaluated for the object they sit on |

Both return `(0x1000, 0x5)` for an `AccessMode::Read` rule. The second one is *not* the contract
the layer requires; the object carries no effective mandatory label. Under this delta it records
`AlreadyAtRequiredLevel`, increments `LabelCoverage::applied` (`:314-315`), and — if every policy
path is in that state — `LabelCoverage::application()` returns `Applied` (`:129-135`), which
`RowVerdict::from_application` maps to `plain(confirmed_status)` (`attestation.rs:387`) and
`decide_from_entries` accepts. **A launch with zero effective mandatory-label ACEs reports the
`MandatoryIntegrityLabel` layer fully established.** That is verbatim the CR-14 defect this phase
exists to close, re-opened through a new door.

This is a **regression introduced by this delta**: before 117-13 the same ACE recorded
`SkipPreExistingLabel` → `NotApplied`/`PartiallyApplied` → `Abort` or a visible downgrade.

Trust boundary: setting a mandatory-label ACE requires `WRITE_OWNER` on the object, which the file
*owner* holds. For workspace files that is the user — so any Medium-IL code running as the user
(a prior escaped agent, a malicious installer, an earlier compromised session) can plant an
inherit-only ML ACE on every path in the compiled policy and permanently satisfy nono's own
attestation while nono's confinement layer is inert. The confined child itself cannot plant it
(Low-IL/AppContainer holds no `WRITE_OWNER`), but the phase's own threat framing is truthful
accounting, not just live escape.

**Fix:** widen the reader so the predicate can see the whole ACE, then require the flags to match
what nono writes:

```rust
// crates/nono/src/sandbox/windows.rs — return the flags too
pub fn low_integrity_label_ace(path: &Path) -> Option<(u32 /*rid*/, u32 /*mask*/, u8 /*flags*/)> {
    // ... same walk, but also read `header.AceFlags`
}

// labels_guard.rs
const NONO_APPLIED_ACE_FLAGS: u8 = 0; // try_set_mandatory_label writes "S:(ML;;0x..;;;LW)"
if prior_rid == SECURITY_MANDATORY_LOW_RID as u32
    && prior_mask == wanted
    // Reject INHERIT_ONLY_ACE (0x08) and anything else that is not the ACE we
    // would have written. INHERITED_ACE (0x10) is set by the OS on a propagated
    // ACE and IS effective, so mask it out before comparing.
    && (prior_flags & !INHERITED_ACE) == NONO_APPLIED_ACE_FLAGS
{
    guard.entries.push(AppliedLabel::AlreadyAtRequiredLevel);
    continue;
}
```

and add the negative-direction test alongside `mismatched_prior_mask_still_records_a_coverage_gap`:
plant `S:(ML;OICIIO;NWNX;;;LW)` via `ConvertStringSecurityDescriptorToSecurityDescriptorW` +
`SetNamedSecurityInfoW`, then assert `guard.entries[0]` is `SkipPreExistingLabel` and
`coverage().application() != LayerApplication::Applied`.

---

### CR-02 (BLOCKER): 117-16 emits specific downgraded layer names, unconditionally and undeduplicated, on the default `stderr`/console channel — the exact channel D-28 names as readable by the confined child — justified by a comment that states a falsehood about where `tracing` routes

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1516-1533`,
`crates/nono-cli/src/cli_bootstrap.rs:161-186`,
`crates/nono-cli/src/output.rs:114-124`,
`.planning/phases/117-fail-direction-contract-startup-self-attestation/117-CONTEXT.md:219-223`

**Issue:** The new warn's own justification reads:

```rust
// `tracing::warn!` at the CLI routes to the log file / Event Log subscriber,
// never to the confined child's stderr — D-28 is not implicated, this is not
// a channel the child shares.
tracing::warn!(
    downgraded_count = downgraded.len(),
    downgraded_layers = %dedup_key,     // <-- specific LayerId Debug names
    ...
);
```

`init_tracing` (`cli_bootstrap.rs:161-186`) has three arms. Only the `--log-file` arm writes to a
file. **Both other arms — the default and the file-open-failure fallback — build the fmt layer
with `.with_writer(std::io::stderr)`.** The overwhelmingly common invocation (`nono run ...`, the
per-tool-call hook path, no `--log-file`) therefore routes this warn to the supervisor's stderr.

D-28 is explicit about that channel:

> **D-28: Layer-specific downgrade detail stays off channels the confined process can read.**
> Naming which layer is not enforcing is reconnaissance for the process being confined, and **in
> supervised mode the child shares the console.**

The child does share it: on the non-detached-stdio path `spawn_windows_child` leaves
`STARTF_USESTDHANDLES` unset and `bInheritHandles = 0` (`launch.rs:2336-2355`), so the child
attaches to the parent's console and can read the screen buffer back via `CONOUT$` /
`ReadConsoleOutputCharacterW`.

The adjacent code is direct evidence of the intent this breaks: `print_attestation_downgrade_banner`
(`output.rs:114-124`) writes to the *same* `eprintln!` sink and deliberately carries **only a
count**, no layer names — that coarseness is D-28's whole point. The new warn sits two lines above
it and prints the names.

Two aggravating factors:

1. **Predicate width.** The pre-existing warns at `:1498`/`:1507` carried `downgraded_layers` only
   inside the audit-emission *failure* arms — a rare path. This change makes it fire on **every**
   downgraded launch.
2. **No dedup.** The banner dedups per session via a marker file; the warn does not. On the
   sandbox-the-tools hook path (one `nono run` per tool call) the confined agent gets the full
   downgraded-layer set re-printed to its shared console on every single tool invocation.

The daemon precedent cited in the comment does not transfer: `nono-agentd` runs as a
`SERVICE_USER_OWN_PROCESS` with no console at all, which is exactly why its own comment
(`agent_daemon/launch.rs`, pre-delta) argued the Event Log is "the equivalent surface *here*".

**Fix:** keep the field, move it off the shared sink. Either route it through the already-wired
ETW/Event Log layer only, or gate it on the log-file arm:

```rust
// Only emit layer-specific detail when tracing is NOT going to the shared console.
if log_target_is_private() {           // true for --log-file / ETW-only configurations
    tracing::warn!(downgraded_count = downgraded.len(), downgraded_layers = %dedup_key, "...");
} else {
    tracing::warn!(downgraded_count = downgraded.len(),
        "startup self-attestation: {} confinement layer(s) could not be fully confirmed \
         (layer detail withheld from the shared console per D-28; see the audit ledger)",
        downgraded.len());
}
```

and change the banner's "see diagnostic output for details" to name the audit ledger, which is
where NR3-04's detail can safely live. If instead the decision is that console read-back is an
accepted risk, D-28 must be amended in `117-CONTEXT.md` with that reasoning — not contradicted by
a code comment. Update the NR3-04 row in `proj/SPEC-windows-fail-direction-contract.md:272`, which
currently repeats the false premise.

---

## Warnings

### WR-01: the residue predicate has no provenance or ownership constraint, so it accepts label states nono itself could never have produced

**File:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:210-244` vs `:246-281`

**Issue:** The doc comment justifies `AlreadyAtRequiredLevel` entirely as *nono's own* residue —
"a PRIOR launch over the same workspace that never reached `Drop` … or an identical concurrent
session" (`:51-67`). But the predicate is evaluated **before** the ownership gate
(`path_is_owned_by_current_user`, `:258`), and tests nothing about who wrote the ACE.

Consequence: a path the current user does **not** own, carrying a Low-IL ACE whose mask happens to
equal this rule's `label_mask_for_access_mode`, now records `AlreadyAtRequiredLevel` and counts
toward `applied`. nono could never have produced that state — on a non-owned path it structurally
cannot write a label and would have recorded `SkipNotOwned` (contract-exempt, not counted). So the
predicate accepts a strictly larger set than the rationale it is written from, and the extra
members are precisely the ones whose provenance is unknown. Combined with CR-01 this is what turns
`applied == 0` (abort) into `applied == N` (fully attested).

**Fix:** move the residue check *after* the ownership gate, so it can only fire on a path nono
could itself have labelled:

```rust
// 1. ownership gate first (Ok(false) -> SkipNotOwned, Err -> propagate)
// 2. then: prior label present?
//      - matches wanted RID+mask+flags -> AlreadyAtRequiredLevel
//      - otherwise                     -> SkipPreExistingLabel
```

and state in the variant's doc comment that ownership is the provenance proxy (nono can only have
written an ACE on a path it owns).

---

### WR-02: `AlreadyAtRequiredLevel` is never reverted while the session that *did* apply reverts at its own `Drop` — this is first-session-out-restores, the inverse of the contract the code and the SPEC claim it preserves

**File:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:17-18`, `:58-67`, `:331-342`,
`proj/SPEC-windows-fail-direction-contract.md:269`

**Issue:** The new variant's doc comment asserts it "preserves the module doc's 'concurrent
sessions sharing the same path: last session out restores' contract rather than tearing down state
this guard does not own", and the SPEC's NR3-01 row repeats "a concurrent session may still depend
on it".

Trace two concurrent sessions over one workspace:

- **A** starts first, path unlabelled → `Applied { path }`, ACE written.
- **B** starts second, sees A's ACE → `AlreadyAtRequiredLevel` → counts toward `applied` → the
  `MandatoryIntegrityLabel` row reports `Applied` → B's startup attestation says the layer is
  established.
- **A** exits normally → `Drop` → `revert_all` → `best_effort_revert` → `clear_mandatory_label`.
  The ACE is gone.
- **B** is still running with a live confined child, and the layer it attested to no longer exists.

That is **first**-out-restores. `AlreadyAtRequiredLevel`'s no-op revert arm does not fix this — it
only guarantees B never removes it; nothing stops A. The module doc at `:17-18` has been factually
wrong since before this delta, but 117-13 is what makes the scenario *reachable* (pre-delta, B
aborted rather than proceeding with a claim it does not own) and what adds a new comment asserting
the false property.

D-20 ("mid-session layer removal is outside this claim") disclaims *external* teardown; it does not
contemplate nono itself removing another nono session's confinement.

**Fix:** at minimum, correct both comments and the SPEC row to state the actual property ("the
session that applied the ACE reverts it; a session that adopted an existing ACE does not, so a
concurrent session's exit can remove a label this session attested to at startup — accepted under
D-20"). If the property is wanted rather than merely documented, the cheapest real mechanism is an
adoption refcount: a per-path marker file under the session dir that `revert_all` checks before
clearing, so the last live holder clears.

---

### WR-03: 117-13 silently retargeted a pre-existing regression test — `guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label` no longer exercises the skip path its name and comments claim, and the D-02 "never revert a pre-existing label" property is now untested for `SkipPreExistingLabel`

**File:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:674-705`

**Issue:** The test pre-labels with the literal `0x5`:

```rust
try_set_mandatory_label(&file, 0x5) // NO_WRITE_UP | NO_EXECUTE_UP
```

and the rule is `AccessMode::Read`, whose `label_mask_for_access_mode` is
`SYSTEM_MANDATORY_LABEL_NO_WRITE_UP (1) | SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP (4)` = **`0x5`**.
After 117-13 this is an exact residue match, so the test now drives `AlreadyAtRequiredLevel`, not
`SkipPreExistingLabel`. It still passes because its only assertions are "label present during" and
"label present after drop" — both true of either arm.

The plan clearly saw this hazard elsewhere: the sibling case (b) in
`coverage_distinguishes_full_partial_and_zero_ace_launches` was explicitly rewritten to use
`label_mask_for_access_mode(AccessMode::Write)` with a `assert_ne!` precondition and a comment
about "a hand-picked constant that might accidentally equal Read's own wanted mask" (`:606-624`).
The same literal was left in place two functions later.

Residue: the `let _skip_variant_reference = AppliedLabel::SkipPreExistingLabel;` binding (`:696`)
is dead code that constructs a variant and asserts nothing, while its comment claims "the
pre-existing-label skip variant is exercised". And no test now covers "a genuine third-party
(mask-mismatched) label is not reverted at `Drop`" — `mismatched_prior_mask_still_records_a_coverage_gap`
never drops the guard and never re-reads the label.

**Fix:** change this test's pre-label to `label_mask_for_access_mode(AccessMode::Write)` with the
same `assert_ne!` precondition the sibling test uses, assert
`matches!(guard.entries[0], AppliedLabel::SkipPreExistingLabel)` (deleting the inert
`_skip_variant_reference`), and add a separate `residue_is_not_reverted_on_drop` test that pins the
`AlreadyAtRequiredLevel` non-revert property the old test was accidentally covering.

---

### WR-04: `NonoRemediation::ClearStaleLayerResidue` is unreachable on every CLI operator path and carries no actionable command, so NR3-01's remediation half produces zero operator-visible change

**File:** `crates/nono/src/diagnostic/codes.rs:71-81`, `crates/nono/src/error.rs:500-509`,
`crates/nono-cli/src/main.rs:204-223`, `crates/nono-cli/src/diagnostic/formatter.rs:1398-1419`,
`crates/nono-cli/src/query_ext.rs:652-657`

**Issue:** The new arm is correct and tested, but nothing on the CLI path reads it:

- `main.rs:221` prints `eprintln!("nono: {}", e)` — the `Display` string only
  (`"Startup self-attestation failed for layer {layer}: {reason}"`). `remediation()` is never
  called there; the only special-cased variant is `ActionRequired`.
- `format_follow_up_from_diagnostics` (`formatter.rs:1398-1419`) matches `RunDiscovery` and
  `CheckPolicy` and falls into `_ => {}` for everything else — including the new variant.
- Both copies of `suggested_flag_for_remediation` (core `codes.rs:119`, CLI `query_ext.rs:656`)
  explicitly return `None`.
- The only consumer that *would* see it is `bindings/c/src/lib.rs:106-110`'s
  `LAST_REMEDIATION_JSON`, which the `nono` binary does not use.

NR3-01's requested fix was a remediation "naming the stale-label cause and the concrete
`icacls`/`nono setup` command". The shipped variant carries only `layer: String` — no cause, no
command. The operator's experience is byte-identical to before the fix.

**Fix:** either add a rendering arm so the text actually reaches the operator —

```rust
NonoRemediation::ClearStaleLayerResidue { layer } => {
    lines.push(format!(
        "[nono]   Layer {layer} could not be confirmed. If a prior nono session exited \
         abnormally, clear its residual mandatory labels: \
         icacls <granted-path> /setintegritylevel Medium"
    ));
}
```

plus the `main.rs` hookup that calls `e.remediation()` for non-`ActionRequired` errors — or drop
the claim from the SPEC's NR3-01 row (`:269`) and the plan summary, which currently read as if the
operator gained guidance.

---

### WR-05: the `NONO_CI_HAS_WFP` addition to the fault-injection CI job is dead configuration — nothing that job runs reads it

**File:** `.github/workflows/ci.yml:316-317`

**Issue:** `NONO_CI_HAS_WFP` has exactly one consumer in the tree:

```
scripts/windows-test-harness.ps1:189:  if ($env:NONO_CI_HAS_WFP -eq 'true') {
scripts/windows-test-harness.ps1:192:      $msg = "SKIPPED: WFP tests require elevated runner (NONO_CI_HAS_WFP not set)"
```

No Rust source reads it (`grep -rn NONO_CI_HAS_WFP crates/ tests/` → zero hits). The
`windows-layer-fault-injection` job's only test steps are
`cargo test -p nono-sandbox-cli --features layer-fault-injection` and
`cargo test -p nono-shell-broker --features layer-fault-injection`; it never invokes
`windows-test-harness.ps1`. The env var therefore changes nothing.

On the narrow question asked: it cannot cause a silent skip-and-pass, precisely *because* it is
inert. But it also does not close NR-08's WFP half — it only makes the job look like it did. The
broker pre-build in the same commit **is** genuine: `cargo build -p nono-shell-broker --release`
lands the artifact at `target/release/nono-shell-broker.exe`, which is exactly
`candidate_default` in `broker_launch_assigns_child_to_job_object` (`launch.rs:4228-4247`), turning
that test's hard `panic!` into real coverage.

**Fix:** either delete the `env:` block from this job, or make it load-bearing by teaching the
Rust-side WFP tests to consult it (and then verify they are not currently passing by early-return).
Correct the 117-18 summary and the SPEC/ledger wording so it does not claim WFP gating was added
where only a broker pre-build was.

---

### WR-06: 117-17 deleted `ProceedDowngraded` on a stated premise that is false — `DaemonDaclGuard::apply` has two real partial-coverage arms, and `granted_write_access()` can express neither

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:1236-1253` (the new doc comment),
`:110-123` (`granted_write_access`), `:158-193` (pass 1), `:224-256` (pass 3),
`proj/SPEC-windows-fail-direction-contract.md:273`

**Issue:** The new enum doc comment and the SPEC's NR3-05 row both assert:

> `DaemonDaclGuard`'s three-pass apply has no partial-success return … so there is no
> partial-coverage state for a downgrade arm to represent.

Pass 1 contradicts that directly:

```rust
Ok(false) => {
    tracing::warn!(path = %rule.path.display(),
        "daemon dacl guard: read-only path not owned by current user; \
         skipping traverse grant (relying on lowbox bypass-traverse)");
    // no `continue`-with-record: the skip is recorded NOWHERE
}
```

That is a returned-`Ok` partial success, recorded in no field. Pass 3 has the same shape
(`break` at the first non-owned ancestor, `:245-252`). `granted_write_access()` inspects only
`write_applied`, which pass 2 populates, so it is `true` whenever pass 2 succeeded regardless of how
much of passes 1 and 3 was skipped. The CLI mirror classifies the exactly-analogous condition as
`SkipWritableNotOwned` → `PartiallyApplied` → `ProceedDowngraded`
(`dacl_guard.rs:149-151`). NR3-05's underlying complaint — the two mirrors disagree about the fail
direction of the same physical condition — is therefore still live; only the dead variant was
removed.

The fail *direction* here is safe (an ungranted traverse means less access, not less confinement),
which is a legitimate reason to prefer deletion. That reasoning is not what the comment says.

Secondly, NR3-05 asked for "a discovery-style conformance test that fails when one mirror gains a
decision state the other lacks." `daemon_attestation_decision_is_deliberately_two_state` only
inspects the daemon's own enum — adding a fourth variant to the CLI's `AttestationDecision` fails
nothing.

**Fix:** replace the false premise with the true one in both the enum doc comment and SPEC row
("passes 1 and 3 have skip arms, but their fail direction is under-granting — reduced access, never
reduced confinement — so a downgrade state would not carry security-relevant information; the CLI's
`SkipWritableNotOwned` differs because it gates a *write* the child was promised"). Then add the
cross-mirror test NR3-05 asked for, e.g. parse both enums' variant lists from source and assert the
daemon's is a documented subset with the divergence enumerated in one place.

---

### WR-07: NR3-02 is only half closed — both new `application()` accessors are two-valued (`Applied` / `NotApplicable`) and can never produce a negative, and the new `NotApplicable` branch now silently drops an `expected: true` / `outcome: Abort` row from the decision

**File:** `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:438-444`, `:647-653`,
`crates/nono-cli/src/exec_strategy_windows/mod.rs:443-457`,
`crates/nono-cli/src/exec_strategy_windows/attestation.rs:398-400`

**Issue:** Both accessors are:

```rust
if self.applied.is_empty() { NotApplicable } else { Applied }
```

Neither `NotApplied` nor `PartiallyApplied` is constructible from a guard that exists. Ask NR3-02's
own question of the new code — *what input makes this row report NOT established?* — and the answer
is still "none": `mod.rs`'s `map_or` default (`NotApplied`) only fires when the `Option` is `None`,
i.e. when `config.package_sid` is `None`, which `execution_runtime.rs` never produces. The literal
`is_some()` was replaced with a function whose reachable range is `{Applied, NotApplicable}` —
both of which pass the gate.

Worse, `NotApplicable` is not neutral here. `DaclAncestorTraverse` and `DaclAncestorReadAttrs` are
`expected: true` with `outcome: Abort` on all five `DirectCli` arms. `RowVerdict::from_application`
maps `NotApplicable` to `plain(LayerAttestationStatus::NotApplicable)` (`attestation.rs:398-400`),
and `decide_from_entries` `continue`s past it. So a walk that granted nothing now makes the row
**vanish from the attested claim entirely** — the "green by absence" shape the SPEC's own
"Structural constraints" section (`:135-149`) names as "the exact failure mode this phase exists to
close". Pre-fix the row at least made a (false) positive claim; post-fix it makes none, and no
consumer can tell the difference between "not part of this composition" and "the walk found nothing
to grant on a launch where the child genuinely needs traverse".

Also note the two new tests
(`ancestor_traverse_application_reports_not_applicable_when_nothing_owned_to_grant` and its RA
twin) assert only the accessor's return value. Nothing drives
`apply_startup_attestation_gate` with `dacl_ancestor_traverse: NotApplicable` to pin what the
*decision* does — unlike the `FirewallRulesEgress` test in the same delta, which correctly goes
through the real gate.

**Fix:** distinguish "walked and found nothing owned" from "walked and granted everything owned"
so the row can carry that fact, e.g. record `stopped_at: Option<PathBuf>` alongside `applied` and
return `PartiallyApplied` when the walk stopped before exhausting the chain with zero grants; and
add a gate-level test mirroring `firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate`
that drives `apply_startup_attestation_gate` with each reachable value and asserts the decision.

---

### WR-08: 9 of 13 registry rows still carry line-number citations, and **every one of the 15 I sampled points at unrelated code** — `registry_call_sites_exist` checks only file existence for those, and the SPEC's own "Enforcing call site(s)" column mirrors the same stale numbers

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:862`, `:873`, `:916-919`,
`:944-945`, `:976-980`, `:997`, `:1018-1022`, `:1031-1035`, `:1044-1047`;
`crates/nono-cli/tests/layer_registry_selfcheck.rs:187-245`;
`proj/SPEC-windows-fail-direction-contract.md:52`, `:54`, `:59-64`

**Issue:** 117-19 reported `restricted_token.rs:55` as one surviving stale citation. It is not one
— it is representative. I resolved 15 line citations against the current tree; **15 of 15 land on
unrelated code**:

| Citation | Actual content at that line |
|---|---|
| `restricted_token.rs:55` | `// D-29/D-30 (Phase 117-06): generalizes the shipped WFP force-unavailable` |
| `launch.rs:1403` | `/// abort (D-22). Factored out of `spawn_windows_child` so the decision logic` |
| `launch.rs:375` | `a non-success code. The child process has been terminated (fail-secure)."` |
| `launch.rs:2134-2137` | `let child_stdio: [HANDLE; 3] = [pipes.stdin_read, ...]` |
| `launch.rs:1505-1506` | `Some(security_layer) => {` |
| `launch.rs:1828-1829` | `}` |
| `launch.rs:2190` | `"InitializeProcThreadAttributeList failed (error={last})"` |
| `launch.rs:2236` | `broker_args.push(OsString::from("--inherit-handle"));` |
| `agent_daemon/launch.rs:133` | `///` |
| `agent_daemon/launch.rs:444` | a doc comment about the egress proxy port |
| `agent_daemon/launch.rs:636-650` | `// AppContainer token and Job Object apply to the resolved executable, not` |
| `agent_daemon/launch.rs:764` | `// SAFETY: both handles are valid; close to avoid leaks.` |
| `agent_daemon/launch.rs:959` | `));` |
| `agent_daemon/launch.rs:1049` | `/// that denies Low-IL and the agent's own package SID any job access.` |
| `mod.rs:357-362` | `// (0x80) on the USER-OWNED ancestors of the confined TARGET BINARY's...` |

`registry_call_sites_exist` deliberately only asserts file existence for these (its own
`// TODO(117-12)` says so, `:182-186`), so nothing fails. The SPEC's registry table repeats the same
numbers under a column literally headed "Enforcing call site(s)" while the document's header claims
it "describes shipped behaviour".

Two secondary observations on the 4 converted rows: `symbol_citation_extraction_finds_the_eight_converted_citations`
counts 8 citations of which 4 are the *identical* string `"mod.rs::prepare_live_windows_launch"`,
and the content check is `content.contains(symbol)` — satisfied by an occurrence in a comment. The
conversion also loses precision: `mod.rs:449-455` and `mod.rs:473-482` pointed at two different
guard constructions; both now cite the same 500-line function.

**Fix:** finish the conversion for the remaining 9 rows in one pass, using
`file.rs::Type::method`-precision symbols (not the enclosing function) so each row still identifies
*its own* call site, and tighten the content check to require the symbol to appear on a line
beginning with `fn `/`pub(crate) fn `/`impl ` rather than anywhere in the file. Regenerate the SPEC
table from `REGISTRY_ENTRIES` rather than transcribing it, or add a selfcheck test that asserts each
SPEC row's citation cell equals its registry row's `call_sites` joined — `spec_matches_registry`
currently compares only `LayerId` names, which is why three passes of drift went uncaught.

---

### WR-09: 117-17's relocation widened the tamper-evident HMAC chain's state to `pub(crate)`, giving every module in the binary direct mutable access to `chain`, `session_id` and `config` with no accessor discipline and no test guarding it

**File:** `crates/nono-cli/src/telemetry/mod.rs:190-207`, `:232-234`,
`crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs:44-51`, `:104`

**Issue:** To let a second `impl SecurityEventLayer` block live in `exec_strategy_windows/`, the
delta changed:

```rust
-struct SecurityEventLayerInner { chain: ChainState, session_id: String, ..., config: TelemetryConfig }
+pub(crate) struct SecurityEventLayerInner {
+    pub(crate) chain: ChainState,
+    pub(crate) session_id: String,
+    session_salt: [u8; 32],          // (correctly left private)
+    pub(crate) config: TelemetryConfig,
+}
-    inner: std::sync::Arc<Mutex<SecurityEventLayerInner>>,
+    pub(crate) inner: std::sync::Arc<Mutex<SecurityEventLayerInner>>,
```

The relocation itself is correct and I verified the daemon lost nothing: `nono-agentd.rs`
`#[path]`-includes only `agent_daemon/mod.rs`, `telemetry/mod.rs` and
`agent_daemon/telemetry_init.rs`, and the daemon's only `emit_attestation_event` call site was the
`ProceedDowngraded` arm deleted in the same plan. Both binaries still build clean under
`-D warnings`, and the three relocated tests pass in the `nono` binary.

The cost is that `chain` — the append-only, HMAC-chained audit state whose whole value is that it
can only advance through `advance_chain` — is now writable by any module in the crate. Nothing
enforces the discipline: a future edit anywhere in `nono-cli` can `SECURITY_LAYER.get().inner.lock()`
and rewrite `chain.head`/`chain.sequence` directly, silently forging or resetting the audit chain,
and no test would notice. Blast radius is bounded (nono-cli has no `[lib]` target, so `pub(crate)`
never crosses a published API boundary) — this is defence-in-depth erosion, not an exploitable
vulnerability today.

**Fix:** keep the fields private and expose the one operation the new module needs:

```rust
// telemetry/mod.rs
impl SecurityEventLayer {
    pub(crate) fn with_inner<R>(&self, f: impl FnOnce(&mut SecurityEventLayerInner) -> R)
        -> Result<R, &'static str> { ... }
    // and narrow field access to pub(in crate::telemetry) + accessor methods
}
```

so `attestation_downgrade_event.rs` composes through the same `advance_chain` discipline every other
emitter uses. Failing that, add a source-scan test asserting `.inner.lock()` appears in exactly the
files that are allowed to hold it.

---

### WR-10: `contains_fn_exact`'s documented `!` boundary rule is not implemented, and it matches `fn <name>` inside comments and string literals

**File:** `crates/nono-cli/tests/layer_registry_meta_test.rs:250-277`

**Issue:** The doc comment states the requirement as:

> Requires the character immediately after the name to be neither an identifier character nor `!`

The code implements only the first half:

```rust
Some(c) => !(c.is_ascii_alphanumeric() || c == '_'),
```

`'!'` is neither ASCII-alphanumeric nor `_`, so `boundary_ok` is `true` for it — the documented
exclusion does not exist.

On the syntax it will actually meet, the matcher is otherwise correct: `pub(crate) fn foo` and
`async fn foo` match (the needle is a suffix search, so preceding modifiers are irrelevant),
`fn foo<T>` matches (`<` is a boundary), `fn foo2` / `fn foobar` correctly do not, and attributes on
the preceding line are unaffected. The genuine residual weakness — unmentioned in the comment — is
that the scan is over raw text: a doc comment reading ``/// see `fn wfp_row_can_be_selected...` ``
or a `format!("fn {name}")` in the searched file would satisfy the check without a real test
existing. That matters because `ALSO_AUTOMATED` is the mechanism that lets 8 of 13 registry rows
count as covered.

**Fix:** implement the documented rule and require the match to start a definition:

```rust
let boundary_ok = matches!(src[after..].chars().next(), None | Some('(') | Some('<') | Some(' '));
// and require the match to be at a line start modulo leading whitespace + modifiers
let line_start = src[..match_start].rfind('\n').map_or(0, |i| i + 1);
let prefix = src[line_start..match_start].trim_start();
let is_definition = matches!(prefix, "" | "pub " | "pub(crate) " | "unsafe " | "async " | "const ")
    || prefix.ends_with("fn ") == false && !prefix.trim_start().starts_with("//");
```

---

### WR-11: `daemon_attestation_decision_is_deliberately_two_state` parses the first textual occurrence of a string that occurs twice in the file it reads — the same fragility class iteration 3 raised as NR3-07, reproduced in a brand-new test

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:2085-2091`

**Issue:**

```rust
let src = include_str!("launch.rs");
let enum_block = src.split("enum DaemonAttestationDecision {").nth(1)...
```

That literal appears twice in `launch.rs`: at the declaration (`:1244`) and inside this test's own
body (`:2085`). It works only because the declaration happens to come first. Moving `mod tests`
above `mod windows_impl`, or adding a doc-comment example containing the phrase, silently
re-targets the parse. NR3-07 raised exactly this for
`daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition`; this plan added a second
instance rather than fixing the pattern.

The failure mode is at least loud today (a mis-targeted parse would produce a variant list that
fails the `assert_eq!`), so this is fragility rather than a live hole. The other structural
weakness is that the whole test is lexical: it pins the *names* of the variants, not that
`daemon_attest_and_decide` has no third return shape.

**Fix:** anchor on a marker the test cannot itself contain, and add the behavioural half:

```rust
// at the declaration site:
// DAEMON-DECISION-ENUM (117-17 anchor — do not remove)
let enum_block = src.split("DAEMON-DECISION-ENUM").nth(2).expect("anchor + declaration");
```

plus a test that drives `daemon_attest_and_decide` across its input space and asserts every result
matches `Proceed | Abort { .. }` exhaustively (which the compiler already guarantees, but which
would then fail to *compile* rather than fail lexically if a variant were added).

---

_Reviewed: 2026-08-10_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard — iteration 4 (gap-closure delta verification)_
