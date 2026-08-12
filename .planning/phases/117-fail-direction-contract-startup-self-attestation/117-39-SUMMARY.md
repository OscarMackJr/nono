---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 39
subsystem: windows-attestation
tags: [windows, d-02, d-37, operator-remediation, gap-closure, round-4, cr-06, wr-27, wr-28, wr-20, cint-02, cint-03]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-35's --no-fail-fast baseline, which records WR-20's test as pre-existing failure #12"
provides:
  - "A MandatoryIntegrityLabel remediation that names a command verified on this host to clear the condition"
  - "A non-label remediation arm pointing at a channel that exists on the abort path"
  - "LabelCoverage::application() excluding its contract-exempt category, mirroring DaclGrantCoverage"
  - "cr_06_remediation_command_actually_clears_the_condition — the remedy proven mechanically, not by prose"
affects: [117-43, 117-44]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Operator remediation text is verified by EXECUTING the prescribed command against a real
      instance of the condition, not by reading the API docs"
    - "A perturbation proof is also a test of the TEST: reverting the fix is how you discover a new
      test does not actually discriminate the change"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs

key-decisions:
  - "The plan's proposed PowerShell snippet (SetSecurityDescriptorSddlForm('S:', 'Sacl')) is invalid
    — 'Sacl' is not a member of AccessControlSections. Its nearest valid form (Get-Acl -Audit) fails
    without SeSecurityPrivilege, which a non-elevated operator does not hold. Prescribed instead the
    P/Invoke of SetNamedSecurityInfoW with LABEL_SECURITY_INFORMATION, verified on this host."
  - "Kept the mixed-policy test even though the perturbation proved it does NOT discriminate this
    fix. It guards against a different wrong fix (counting skipped_not_owned as applied), and its
    non-discriminating status is recorded rather than presented as proof."
  - "Kept four literal mentions of `/setintegritylevel Medium` in main.rs against the plan's
    zero-count criterion. All four either explain the defect or warn the operator OFF the command
    the previous version told them to run; omitting it would be weaker guidance."
---

# Plan 117-39: CR-06 + WR-27 + WR-28 + WR-20 — Execution Summary

**Executed:** 2026-08-12 · **Wave:** 16 · **Requirements:** CINT-02, CINT-03

## Task 1 — `render_error_for_operator` (CR-06, WR-27)

### WR-27 enumeration, re-derived independently

`grep -rn "LayerAttestationFailed" crates/ bindings/` → ~16 production construction sites
(`nono/src/attestation.rs` ×9, `agent_daemon/launch.rs:941,:1582`,
`exec_strategy_windows/attestation.rs:618`, `dacl_guard.rs:194,:387,:625`, `labels_guard.rs:233`,
`launch.rs:418`, `restricted_token.rs:86`, `nono-shell-broker/src/main.rs:616`). **Every one is a
plain `return Err(..)`; none is paired with a telemetry or event-log write.**

`grep -rn "emit_attestation_event("` → exactly ONE production caller, `launch.rs:1528`, inside
`apply_startup_attestation_gate`'s **`ProceedDowngraded`** arm.

**WR-27 confirmed: the Application Event Log receives nothing on the abort path.** The `_` arm now
points at `-vv --log-file <path>` plus the layer name — the failing guard's own `tracing`
diagnostic, which does exist there — and explicitly says why the event log is empty.

### CR-06: the remedy was researched by execution, not by reading docs

The plan prescribed a PowerShell snippet using
`SetSecurityDescriptorSddlForm("S:", "Sacl")`. **That is not valid**:
`[Enum]::GetNames([System.Security.AccessControl.AccessControlSections])` →
`None, Audit, Access, Owner, Group, All`. There is no `Sacl` member; the call would throw.

Tested every candidate against a real file carrying a planted Low label:

| Candidate | Observed result |
|---|---|
| `icacls <f> /setintegritylevel Medium` (**the shipped advice**) | `Mandatory Label\Medium Mandatory Level:(NW)` still present → D-02 reads `SkipPreExistingLabel` → **aborts identically. CR-06 confirmed empirically.** |
| `Get-Acl -Audit` + `SetSecurityDescriptorSddlForm('S:','Audit')` + `Set-Acl` | `FAILED: The process does not possess the 'SeSecurityPrivilege' privilege` — requests the whole SACL; wrong for a non-elevated operator |
| `icacls <f> /setintegritylevel Low` | sets `(NW)` only; `label_mask_for_access_mode(Read)` wants `NO_WRITE_UP\|NO_EXECUTE_UP` → still a mask mismatch |
| P/Invoke `SetNamedSecurityInfoW(path, SE_FILE_OBJECT, LABEL_SECURITY_INFORMATION, .., <empty ACL>)` | **`rc=0` (ERROR_SUCCESS), label REMOVED, from a NON-elevated shell** ✓ |

The last is exactly what `labels_guard::clear_mandatory_label` does internally. Verified twice,
including in the compact one-liner form now embedded in the remediation string. The empty ACL is
the 8-byte header `02 00 08 00 00 00 00 00` (revision 2, size 8, 0 ACEs).

The arm also drops the "a prior session that exited abnormally" cause (CR-06b): Plan 117-13
(NR3-01) made that residue **self-healing and non-aborting**, so it can no longer be the reason.
It now names the causes that CAN still abort — a third-party label, a mask mismatch, or an
inert `INHERIT_ONLY_ACE`.

Both existing tests updated (not deleted); the second renamed to
`render_error_for_operator_names_a_reachable_channel_for_non_label_layers`.
**3 passed / 0 failed.**

## Task 2 — `LabelCoverage::application()` (WR-28), WR-20 pin, CR-06 proof

Added `non_exempt_paths() = applied + skipped_pre_existing_label`, and gated `NotApplicable` on it
instead of `policy_paths == 0`. Mirrors `DaclGrantCoverage::writable_rules()`, which already omits
its own exempt category. The doc comment records that what is mirrored is the **RULE**, not the
variant names — the two guards classify "not owned" differently on purpose (a gap for DACLs,
because confined writes there are denied; exempt for labels, because nono cannot label what it
does not own).

Swept every `application()`/`NotApplied`/`NotApplicable` site in the file rather than only the two
the plan named. Four cases in `coverage_distinguishes_full_partial_and_zero_ace_launches`, the
NR3-01 residue test, and the CR-01 inherit-only test are all unaffected (verified by re-derivation:
`AlreadyAtRequiredLevel` counts as `applied`). One assertion changed:
`guard_skips_path_not_owned_by_current_user`, `NotApplied` → `NotApplicable`.

### Perturbation proof — and what it revealed about the plan's own criterion

The plan required BOTH the new mixed-policy test AND the WR-20 pin to fail when the fix is
reverted. Reverting the gate to `policy_paths == 0`:

```
test ...guard_skips_path_not_owned_by_current_user ... FAILED
test ...labels_guard_mixed_owned_and_non_owned_reports_applied_not_partially_applied ... ok

panicked: WR-28: an all-SkipNotOwned policy is contract-exempt, so it must classify
NotApplicable — never NotApplied ...:
LabelCoverage { policy_paths: 1, applied: 0, skipped_pre_existing_label: 0, skipped_not_owned: 1 }
```

**The mixed test passes under BOTH rules.** `SkipNotOwned` never contributed to
`PartiallyApplied` — that branch is gated solely on `skipped_pre_existing_label` — so the plan's
`<behavior>` bullet ("a mixed policy classifies `Applied`, not `PartiallyApplied`") described a
property that was **already true**, and its acceptance criterion was unsatisfiable as written.

The rules differ in exactly one case: `applied == 0 && skipped_pre_existing_label == 0 &&
skipped_not_owned > 0`. The discriminating tests are therefore the two all-exempt ones
(`guard_skips_path_not_owned_by_current_user`, and WR-20's — host-blocked).

The mixed test is retained as a regression guard against a *different* wrong fix (counting
`skipped_not_owned` as applied, which would make it report `Applied` with zero owned paths), but
it is **not** evidence for this change and is not claimed as such.

### CR-06 proven mechanically

`cr_06_remediation_command_actually_clears_the_condition` **passes**: plants a foreign-masked
label → `snapshot_and_apply` → asserts `skipped_pre_existing_label == 1` and `NotApplied` (the
state the operator is aborted out of) → calls `clear_mandatory_label` (the mechanism the printed
snippet drives) → asserts `low_integrity_label_and_mask` is now `None` → re-runs the SAME policy →
asserts `Applied`.

## WR-20 disposition — host-blocked, recorded, NOT weakened

`non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` **fails at setup** on this host:

```
test setup: could not reassign ownership of ...foreign-labeled.txt away from the current user —
icacls /setowner "NT AUTHORITY\SYSTEM" failed (This security ID may not be assigned as the owner
of this object), and the "BUILTIN\Administrators" fallback also failed. This host's session almost
certainly lacks SeRestorePrivilege (non-elevated / non-admin token) ...
```

Status: **authored, structurally reasoned correct, UNVERIFIED on this host.** The
`coverage.application()` assertion this plan added sits *after* the failing setup, so **it has
never executed here**. It needs an elevated or CI Windows runner before it may be called proven.
Not weakened to a skip; it fails loudly by design (D-31).

This is pre-existing baseline failure #12 from `117-BASELINE-ROUND4.md`, so `labels_guard::` at
**13 passed / 1 failed** reconciles exactly — no regression introduced.

## Deviations

1. **The plan's PowerShell snippet was invalid** (`Sacl` is not an `AccessControlSections` member)
   and its nearest valid form needs a privilege the operator lacks. Replaced with a
   host-verified P/Invoke. This is the plan's own instruction ("do not invent a plausible-sounding
   but unverified snippet") applied to the plan's own suggestion.
2. **`grep -c "/setintegritylevel Medium" main.rs` returns 4, not the required 0.** Two in the doc
   comment explaining the defect, one in the remediation warning the operator off it, one in a test
   asserting it is not prescribed. The criterion's intent (never prescribed as the remedy) is
   satisfied and pinned by a test.
3. **The mixed-policy test does not discriminate the fix** (see above). Retained with its status
   recorded.

## Verification

| Gate | Command | Result |
|---|---|---|
| Task 1 | `cargo test ... --no-fail-fast -- --test-threads=1 render_error_for_operator` | **3 passed / 0 failed** |
| Task 2 | `cargo test ... --features layer-fault-injection --no-fail-fast -- --test-threads=1 labels_guard::` | **13 passed / 1 failed** — the 1 is WR-20's host-blocked setup, baseline #12 |
| Format | `cargo fmt --all -- --check` | clean |
| Cross-target clippy (linux-gnu) | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |
| Cross-target clippy (apple-darwin) | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |

Both files trigger CLAUDE.md's cross-target MUST (`main.rs` carries Unix `cfg` blocks;
`labels_guard.rs` is under `exec_strategy_windows/`); both gates were run locally, neither deferred.
