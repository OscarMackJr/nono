---
phase: 118-per-session-enforcement-receipts
reviewed: 2026-09-05T22:10:56Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/bin/nono-agentd.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/receipt_commands.rs
  - crates/nono-cli/src/receipt_sink.rs
  - crates/nono-cli/tests/layer_registry_meta_test.rs
  - crates/nono-cli/tests/layer_registry_selfcheck.rs
  - crates/nono-cli/tests/receipt_content_free_scan.rs
  - crates/nono-cli/tests/receipt_sentinel_roundtrip.rs
  - crates/nono-shell-broker/Cargo.toml
  - crates/nono-shell-broker/src/main.rs
  - crates/nono-shell-broker/tests/broker_wire_contract_names.rs
  - crates/nono/src/attestation.rs
  - crates/nono/src/lib.rs
  - crates/nono/src/machine_policy.rs
  - crates/nono/src/receipt.rs
  - crates/nono/src/receipt_chain.rs
  - crates/nono/src/sandbox/windows.rs
findings:
  critical: 5
  warning: 7
  info: 4
  total: 16
status: issues_found
---

# Phase 118: Code Review Report

**Reviewed:** 2026-09-05T22:10:56Z
**Depth:** standard
**Files Reviewed:** 24
**Status:** issues_found

## Summary

Reviewed the Phase 118 diff (`2359c841..HEAD`, 8,136 insertions) implementing per-session
enforcement receipts on Windows across three producers (`nono.exe`, `nono-agentd.exe`,
`nono-shell-broker.exe`) and one reader (`nono receipt list|show|verify`).

The core vocabulary (`crates/nono/src/receipt.rs`), the keyless chain primitive
(`receipt_chain.rs`), the `RequireReceipts` machine-policy read (`machine_policy.rs`, correctly on
the abort-on-malformed EGRESS lifecycle), the four-state rendering exhaustiveness guard, and the
`deny_sid_on_path`/`revoke_sid_on_path` ACE-type fix are all sound and well-tested. D-25's
"keyless, not HMAC" discipline is honoured: no `key` field exists anywhere, and every user-facing
verify string is worded to the narrower claim.

The defects are concentrated in the three places the phase's own priority classes predicted:

1. **`nono.exe` — the primary producer — still has three unenumerated post-spawn terminal exits
   that emit no receipt** (CR-01). Plans 118-06 and 118-08 closed exactly this class for the broker
   and the daemon; `spawn_windows_child` was never given the same treatment, and one of the three
   leaves a *false* `Ran` receipt on disk for a session that never ran.
2. **The chain always restarts at `sequence: 0`** because both writers are constructed fresh per
   emission and never read the existing segment (CR-02). Four independent, reachable code paths
   produce a segment whose second record is `sequence: 0` — which `verify_records` reports as
   `"a record was reordered or deleted"`, i.e. an honest record indistinguishable from tampering.
3. **Two evidence-path inputs are trusted from the environment**: `%PROGRAMDATA%` (CR-04) and
   `NONO_DETACHED_SESSION_ID` (WR-01), both of which let the party being recorded silently defeat
   the fleet-wide `RequireReceipts=1` control without producing any error.

Plus one operational time bomb: the sink's DENY ACE is applied per session and **never revoked**
(CR-05), so the sink directory's DACL grows one ACE per confined launch until it hits the Win32 ACL
size ceiling.

Finally, Task 3's D-08 scope correction was applied to the module docs but **not** to three
constant/function doc comments in the same files, which still assert the read protection Task 3
falsified (WR-04, WR-05) — the exact "overstated coverage claim" class this phase already tripped
on once.

---

## Critical Issues

### CR-01: `nono.exe` has three post-spawn terminal exits that emit no receipt — and one writes a false `Ran`

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:2903-2936`

**Issue:** RCPT-01 requires *every* confined session to emit a receipt. Plan 118-06 closed this gap
for `nono-shell-broker.exe` (three `record_broker_receipt` calls added at its early-refuse
branches) and Plan 118-08 closed it for `nono-agentd.exe` (five
`write_daemon_refuse_receipt_best_effort` call sites at steps 6/6.5/6.6/7a/7b). **The equivalent
gap in `nono.exe` was never closed.** After `CreateProcess{AsUser}W` returns a real suspended
confined child, three terminal paths run before/around the attestation gate:

```rust
// launch.rs:2903 — confined child EXISTS and is terminated; NO receipt.
if let Err(err) = apply_process_handle_to_containment(containment, process.raw()) {
    terminate_suspended_process(process.raw(), "AssignProcessToJobObject failed");
    return Err(err);
}
// launch.rs:2910 — same shape; NO receipt.
if let Err(err) = apply_resource_limits(containment, limits) {
    terminate_suspended_process(process.raw(), "apply_resource_limits failed");
    return Err(err);
}
// ... attestation gate (writes the receipt) ...
// launch.rs:2936 — gate already wrote `Ran`; ResumeThread can still fail here.
resume_contained_process(process.raw(), thread.raw())?;
```

`resume_contained_process` (launch.rs:491-497) terminates the child on `ResumeThread` failure and
returns `Err`. The `Ran` receipt written moments earlier is **never corrected**. Both the broker
(broker `main.rs`, the `resumed == u32::MAX` branch) and the daemon (`agent_daemon/launch.rs`, the
`resume_result == u32::MAX` branch) explicitly append a corrective `Refused` record for exactly
this case; `nono.exe` does not.

**Failure scenario:** a `nono run` on a host where `AssignProcessToJobObject` fails (job already at
its nested-job depth limit, or the job handle was closed) spawns a confined child, terminates it,
and exits with an error — and `nono receipt show <session-id>` reports *"no receipt segment found"*.
Under `RequireReceipts=1` an operator auditing the fleet sees a confined session that produced no
record at all, which is precisely the green-by-absence gap D-01 exists to close. In the
`ResumeThread` case it is worse than absence: the sink asserts `outcome: Ran` for a process that
executed zero instructions.

**Fix:** mirror the broker/daemon gap closure. Add a best-effort refusal emitter alongside
`terminate_suspended_process` at both pre-gate sites, and a corrective record at the resume site:

```rust
// Before each `terminate_suspended_process(...); return Err(err);` pair:
let entries = layer_registry::all_entries();
let census = attestation::census_from_entries(entries, &input_for_this_launch);
let _ = emit_enforcement_receipt(
    &crate::receipt_sink::resolve_sink_dir(), census, session_id,
    config.session_sid.as_deref(), unsafe { GetProcessId(process.raw()) },
    layer_registry::EntryPath::DirectCli, Some(arm),
    nono::SessionOutcome::Refused, None,
);

// And at the resume site, matching the broker's own comment verbatim:
if let Err(err) = resume_contained_process(process.raw(), thread.raw()) {
    let _ = emit_enforcement_receipt(/* ..., SessionOutcome::Refused, ... */);
    return Err(err);
}
```

Note `resume_contained_process` currently uses `?`, which structurally prevents inserting the
correction — it must become an explicit `if let Err`.

---

### CR-02: every writer restarts the chain at `sequence: 0`, so any second record on an existing segment fails `verify` as if tampered

**File:** `crates/nono-cli/src/receipt_sink.rs:441-462` and `crates/nono-shell-broker/src/main.rs`
(`BrokerReceiptWriter::new`)

**Issue:** `ReceiptWriter::new` opens the segment file in append mode but **never reads it**, and
unconditionally initialises `ReceiptChainState { head: [0u8; 32], sequence: 0 }`. The broker's
`BrokerReceiptWriter::new` does the same. Neither writer is retained across emissions — every
emission site constructs a fresh one:

- `emit_enforcement_receipt` (launch.rs:1938) — `ReceiptWriter::new(...)` per call
- `daemon_emit_enforcement_receipt` (agent_daemon/launch.rs) — `ReceiptWriter::new(...)` per call
- `record_broker_receipt` (broker main.rs) — `BrokerReceiptWriter::new(...)` per call

So the *second* record appended to any segment carries `sequence: 0, prev_head: null` again.
`verify_records` (receipt_commands.rs:679-685) then fails on that record with:

> `receipt verify: sequence out of order — expected 1, found 0 (a record was reordered or deleted)`

An honest, system-generated record is reported with a **tamper-shaped error**, which destroys the
signal RCPT-02 exists to provide: on an affected segment, real tampering is indistinguishable from
normal operation.

**Four reachable paths produce this today:**

1. **Broker, `Ran` then `ResumeThread` fails** — the code comment claims the correcting record goes
   "to the SAME per-session chain (D-15's append-only ... never a replaced record, a truthful
   second one)". It does not: a new writer is built, so both records are `sequence: 0`. The
   corrective record makes the whole segment unverifiable.
2. **Daemon, `Ran` then `ResumeThread` fails** — identical shape, identical outcome.
3. **All broker receipts on a host** — see CR-03; every broker session shares
   `unknown-session.broker.jsonl`, so record 2 onward is permanently broken.
4. **Any user, deliberately** — `session_id` is sourced from `NONO_DETACHED_SESSION_ID`
   (`launch_runtime.rs:480`). Re-running `nono run` with
   `NONO_DETACHED_SESSION_ID=<an-existing-session-id>` makes the supervisor append a second
   `sequence: 0` record to that segment. **Any local user can permanently break the verification of
   any previously-written receipt segment without ever needing write access to the sink** — the
   supervisor performs the write on their behalf, past the D-08 DENY ACE.

This is beyond the single limitation the module doc names (receipt_sink.rs:160-168), which is about
*concurrent* same-`session_id` writes from two processes and `OpenOptions::append` atomicity. The
defect here is fully sequential and needs no race. The existing test
(`write_receipt_chains_two_records_and_recomputes_from_disk`) uses **one** writer for two writes, so
it cannot catch this.

**Fix:** make `ReceiptWriter::new` resume the existing chain instead of assuming genesis, and add
the perturbation test that two independently-constructed writers on the same segment still verify:

```rust
pub fn new(session_id: String, sink_dir: &Path) -> Result<Self> {
    let file_path = session_file_path(&session_id, sink_dir)?;
    // Resume, never assume genesis: read the tail record's sequence/chain_head.
    let (head, sequence) = match std::fs::read_to_string(&file_path) {
        Ok(existing) => match existing.lines().filter(|l| !l.trim().is_empty()).next_back() {
            Some(last) => {
                let rec: ReceiptRecord = serde_json::from_str(last).map_err(|e| {
                    NonoError::Snapshot(format!(
                        "receipt_sink: refusing to append to an unparseable segment {}: {e}",
                        file_path.display()))
                })?;
                (*rec.chain_head.as_bytes(), rec.sequence.saturating_add(1))
            }
            None => ([0u8; 32], 0),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ([0u8; 32], 0),
        Err(e) => return Err(NonoError::Snapshot(format!(
            "receipt_sink: failed to read existing segment {}: {e}", file_path.display()))),
    };
    // ... open for append (create) as today ...
    Ok(Self { inner: Mutex::new(ReceiptChainState { head, sequence }), file_path })
}
```

Apply the identical change to `BrokerReceiptWriter::new`. Fail closed (as above) when the existing
tail cannot be parsed — silently starting a new genesis on a corrupt tail would re-introduce the
same ambiguity.

---

### CR-03: `NONO_SESSION_ID` is never passed to the broker, so every broker receipt on the host lands in one shared `unknown-session.broker.jsonl`

**File:** `crates/nono-shell-broker/src/main.rs` (`run`, `session_id` read;
`BrokerReceiptWriter::new` degrade branch) and
`crates/nono-cli/src/exec_strategy_windows/launch.rs:2113-2148` (`broker_env_pairs`)

**Issue:** the broker reads its correlation id with
`std::env::var("NONO_SESSION_ID").unwrap_or_default()`, and `BrokerReceiptWriter::new` degrades an
empty/unsafe value to the fixed literal `"unknown-session"`. **Nothing on the `nono.exe` side ever
sets `NONO_SESSION_ID` in the broker's environment.** `broker_env_pairs` (launch.rs:2133-2148) is
`env_pairs.clone()` plus exactly two pushes — `BROKER_REQUIRED_LAYERS_ENV_VAR` and
`BROKER_NOT_APPLICABLE_LAYERS_ENV_VAR`. `env_pairs` comes from `build_child_env`, which copies
`nono.exe`'s own inherited process environment; `nono.exe` never sets `NONO_SESSION_ID` on itself
(a repo-wide grep finds it set only in `hook_runtime.rs:196` / `hook_runtime_windows.rs:105,171`,
which configure *hook* subprocesses, and in `aipc_sdk.rs` tests). `flags.session.session_id` is
threaded to the gate as a function parameter, never as an env var.

**Failure scenario (the normal case, not an edge case):**

- Every `nono shell` / broker-arm `nono run` on the host writes its `EntryPath::Broker` receipt to
  the single file `%PROGRAMDATA%\nono\receipts\unknown-session.broker.jsonl`.
- D-15's entire correlation story is dead: `nono receipt show <session-id>` finds only the
  `<session-id>.jsonl` primary segment and never the broker segment for the *actual confined
  grandchild* — the process that RCPT-01 most cares about.
- Compounded with CR-02, every broker receipt after the very first one on the host is
  `sequence: 0` in a file that already has records, so `nono receipt verify unknown-session`
  fails permanently, and each session's census is interleaved with every other session's in one
  unattributable file.

**Fix:** set the correlation id explicitly on the broker's environment at the producer, next to the
two wire-contract channels it already sets:

```rust
// launch.rs, inside the `BrokerLaunch | BrokerLaunchNoPty` block:
if let Some(id) = session_id {
    broker_env_pairs.push(("NONO_SESSION_ID".to_string(), id.to_string()));
}
```

Separately, reconsider the `"unknown-session"` degrade in `BrokerReceiptWriter::new`: collapsing
distinct sessions onto one shared segment file is worse than failing the write, and it diverges
from `ReceiptWriter::new`, which fails closed on the same input class (see IN-01).

---

### CR-04: the entire receipt sink location is taken from the `%PROGRAMDATA%` environment variable, silently defeating `RequireReceipts=1`

**File:** `crates/nono-cli/src/receipt_sink.rs:284-287` and the byte-identical copy in
`crates/nono-shell-broker/src/main.rs` (`broker_receipt_sink_dir`)

```rust
pub fn resolve_sink_dir() -> PathBuf {
    let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    PathBuf::from(base).join("nono").join(RECEIPT_SINK_DIRNAME)
}
```

**Issue:** CLAUDE.md § Path Handling (CRITICAL) states: *"Validate environment variables before use.
Never assume `HOME`, `TMPDIR`, etc. are trustworthy."* `PROGRAMDATA` is read raw, unvalidated, and
uncanonicalized, and it decides where the phase's entire body of security evidence is written. It
is not compared against the expected machine-wide root, not checked for ownership
(`path_is_owned_by_current_user` exists in core and is used for exactly this class of check
elsewhere), and not checked for being an absolute path.

**Failure scenario:** `set PROGRAMDATA=C:\Users\me\fake && nono run ...`. `ensure_sink_guarded`
succeeds (the user owns the directory, so the DACL and label both apply), `ReceiptWriter::new`
succeeds, `write_receipt` succeeds, so `record_receipt_write_outcome` is **never reached** and
`require_receipts` is never even consulted. The launch proceeds; the operator's governance
consumer, reading the real `C:\ProgramData\nono\receipts`, sees nothing. **The one fleet-wide knob
CONTEXT.md gives an operator to force honesty (`HKLM\SOFTWARE\Policies\nono\RequireReceipts`, an
admin-only key) is defeated by a user-settable environment variable, with no error, no warning, and
no log line.** Note the supervisor's own env is at issue here, not the child's — `build_child_env`'s
skip list normalises `PROGRAMDATA` for the *child* but has no effect on the supervisor.

Confidence note: I verified the read is unvalidated and that the `RequireReceipts` gate is only
consulted on the write-failure path (`record_receipt_write_outcome` is called *only* from
`emit_enforcement_receipt`'s `Err` arm and the `session_id: None` arm). I did not execute the
attack on this host.

**Fix:** resolve the sink from a trusted source and validate it, rather than trusting the variable:

```rust
pub fn resolve_sink_dir() -> Result<PathBuf> {
    // Prefer SHGetKnownFolderPath(FOLDERID_ProgramData) / the system drive,
    // never a bare env read. If %PROGRAMDATA% is retained as a fallback,
    // reject it unless it is absolute AND owned by an administrative principal:
    let base = PathBuf::from(std::env::var("PROGRAMDATA")
        .unwrap_or_else(|_| r"C:\ProgramData".to_string()));
    if !base.is_absolute() || nono::path_is_owned_by_current_user(&base)? {
        return Err(NonoError::Snapshot(
            "receipt_sink: %PROGRAMDATA% does not resolve to a machine-wide, \
             non-user-owned root — refusing to write receipts to a redirectable sink".into()));
    }
    Ok(base.join("nono").join(RECEIPT_SINK_DIRNAME))
}
```

The `Err` then flows into `record_receipt_write_outcome`, which already implements the correct
degrade-vs-abort posture. The broker's duplicated `broker_receipt_sink_dir` must change identically.

---

### CR-05: the sink's DENY ACE is added per session and never revoked — the DACL grows without bound until launches start failing

**File:** `crates/nono-cli/src/receipt_sink.rs:308-330` (`ensure_sink_guarded`) and
`crates/nono/src/sandbox/windows.rs` (`deny_sid_on_path` / `revoke_sid_on_path`)

**Issue:** `ensure_sink_guarded` adds a DENY ACE for this launch's SID to the **shared, permanent**
sink directory on every emission. The SIDs are unique per launch —
`generate_session_sid()` (`restricted_token.rs:21`) is UUID-derived, and the daemon's package SID is
derived from a per-run AppContainer moniker. `deny_sid_on_path` explicitly *preserves* the existing
DACL. **No caller ever calls `revoke_sid_on_path` on the sink directory** (verified: the only
`revoke_sid_on_path` call sites are `dacl_guard.rs:303,487,732` and `agent_daemon/launch.rs:293,303`,
all on *workspace* paths, all in `Drop` impls; the receipt sink has no such guard).

Ironically, `revoke_sid_on_path`'s own newly-written doc comment in this diff names this exact
consequence — *"every per-session synthetic SID's deny ACE would accumulate on the sink directory's
DACL forever, one per session, never revoked"* — and describes it as the bug being fixed. The
`DeleteAce` mechanism was fixed; **the sink never invokes it.**

**Failure scenario:** a Win32 ACL's `AclSize` field is a `u16`, capping the ACL at 65,535 bytes. An
`ACCESS_DENIED_ACE` for an `S-1-5-117-a-b-c-d` SID is ~36 bytes; a package SID
(`S-1-15-2-...`, 11 sub-authorities) is ~60. On a host running the per-tool-call hook path — where
*every tool call* is a fresh `nono run`, hence a fresh session SID, hence one more permanent ACE —
the ceiling is reached in the low thousands of launches, easily within days of normal agent use.
Past that point `SetEntriesInAclW`/`SetNamedSecurityInfoW` fail, `ensure_sink_guarded` returns
`Err`, and:

- with `RequireReceipts=1`: **every confined launch on the host aborts**, permanently, until an
  administrator manually resets the directory ACL — a self-inflicted host-wide denial of service;
- without it: every launch degrades and **no receipts are written at all** from that moment on,
  which is the silent evidence loss D-01 exists to prevent.

The `#[cfg(test)]` coverage (`ensure_sink_guarded_applies_both_deny_ace_and_no_read_up_label`) uses
a fresh `tempdir()` per test, so the accumulation is invisible to it.

**Fix:** two options, either sufficient.

1. Stop adding a per-session ACE. The confined children this guard targets share one property that
   *is* stable: they are the only subjects that carry a nono-minted restricting/package SID. Deny
   the stable well-known principal instead (e.g. `ALL APPLICATION PACKAGES` `S-1-15-2-1` for the
   AppContainer arms) plus one long-lived nono service SID, applied once at sink creation.
2. Keep the per-session ACE but tie it to a guard object that revokes it, exactly like
   `dacl_guard.rs`'s `Drop` impls:

```rust
pub struct SinkSidGuard { dir: PathBuf, sid: String }
impl Drop for SinkSidGuard {
    fn drop(&mut self) {
        if let Err(e) = nono::revoke_sid_on_path(&self.dir, &self.sid) {
            tracing::warn!(error = %e, "receipt_sink: failed to revoke the per-session sink DENY ACE");
        }
    }
}
```

Whichever route is taken, add a test that calls `ensure_sink_guarded` N times with N distinct SIDs
against one directory and asserts the ACE count does not grow linearly.

---

## Warnings

### WR-01: `validate_session_id_for_filename` has no length cap and does not reject Windows reserved device names

**File:** `crates/nono-cli/src/receipt_sink.rs:399-410` (and the non-Windows mirror at
`receipt_commands.rs:191-202`)

**Issue:** the check is character-class only (`is_ascii_alphanumeric() || '-' || '_'`, non-empty).
Its sibling for the same value, `session::validate_session_id` (`session.rs:814-828`), applies the
identical character class **plus a 64-byte length cap**. Two consequences:

1. **No length cap.** `session_id` is user-supplied via `NONO_DETACHED_SESSION_ID`
   (`launch_runtime.rs:480`). A 400-character value passes this validator and produces a path that
   may exceed `MAX_PATH`, so `OpenOptions::open` fails, the receipt is not written, and the launch
   degrades (or, under `RequireReceipts=1`, aborts) for a reason unrelated to confinement.
2. **Reserved DOS device names pass.** `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9` are
   all pure ASCII alphanumerics. Win32 path parsing resolves these legacy device names in any
   directory and regardless of extension, so `…\receipts\CON.jsonl` opens the console device. The
   write then *succeeds* — `write_receipt` returns `Ok(())`, `record_receipt_write_outcome` is never
   reached, `require_receipts` is never consulted — while nothing durable is created. `nono receipt
   verify CON` subsequently reports "no receipt segment found". This is a silent RCPT-01 violation
   that a user can trigger with one environment variable.

I confirmed the missing cap and missing reserved-name check by reading both validators; I did **not**
empirically confirm the `CON.jsonl` device-resolution behaviour on this host — that part rests on
documented Win32 path-parsing semantics.

**Fix:**

```rust
const RESERVED_DEVICE_NAMES: &[&str] = &[
    "CON","PRN","AUX","NUL","COM1","COM2","COM3","COM4","COM5","COM6","COM7","COM8","COM9",
    "LPT1","LPT2","LPT3","LPT4","LPT5","LPT6","LPT7","LPT8","LPT9",
];

pub(crate) fn validate_session_id_for_filename(session_id: &str) -> Result<()> {
    let ok = !session_id.is_empty()
        && session_id.len() <= 64                       // match session::validate_session_id
        && session_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !RESERVED_DEVICE_NAMES
            .iter()
            .any(|r| session_id.eq_ignore_ascii_case(r));
    if ok { Ok(()) } else { Err(NonoError::Snapshot(format!(
        "receipt_sink: session_id is not safe for use as a filename: {session_id:?}"))) }
}
```

Apply the same three additions to the `#[cfg(not(target_os = "windows"))]` mirror in
`receipt_commands.rs`, which the module doc already commits to keeping identical.

---

### WR-02: `broker_census` prefers `NotApplicable` over `Required`, so a required layer can be reported as "not expected in this configuration"

**File:** `crates/nono-shell-broker/src/main.rs` (`broker_census`)

```rust
let status = if not_applicable.contains(&name.as_str()) {
    nono::LayerAttestationStatus::NotApplicable        // checked FIRST
} else if required.contains(&name.as_str()) {
    broker_required_row_status(..)
} else {
    nono::LayerAttestationStatus::Unconfirmed
};
```

**Issue:** the two channels arrive as separate, unvalidated environment strings
(`NONO_BROKER_REQUIRED_LAYERS`, `NONO_BROKER_NOT_APPLICABLE_LAYERS`). `broker_census` never checks
that they are disjoint, and resolves an overlap in the *permissive* direction. If a `LayerId` name
appears on both channels, the receipt reports it `NotApplicable` — RCPT-03's "layer not expected in
this configuration" — for a layer the broker's own gate treats as must-be-`Confirmed`-or-terminate.

The disjointness is guaranteed only on the *producer* side (`broker_role`'s partition, tested in
`attestation.rs::broker_wire_contract_partitions_all_13_layer_ids_with_zero_overlap_or_gap`). The
consumer takes that on faith. Any process that can spawn `nono-shell-broker.exe` with a chosen
environment — same-user, same integrity level as the supervisor — can make the census present a
required layer as structurally out of scope. The `Required` branch is fail-closed
(`broker_required_row_status`'s `_ => Unconfirmed`); the `NotApplicable` branch is not.

**Fix:** check `required` first, and treat an overlap as a wire-contract violation:

```rust
let status = if required.contains(&name.as_str()) {
    if not_applicable.contains(&name.as_str()) {
        // A row on BOTH channels is a producer bug or a tampered environment —
        // never resolve it in the permissive direction.
        nono::LayerAttestationStatus::Unconfirmed
    } else {
        broker_required_row_status(&name, app_container_probe, expected_app_container_sid, integrity_probe)
    }
} else if not_applicable.contains(&name.as_str()) {
    nono::LayerAttestationStatus::NotApplicable
} else {
    nono::LayerAttestationStatus::Unconfirmed
};
```

---

### WR-03: unchecked `sequence + 1` on untrusted on-disk data in `nono receipt list`

**File:** `crates/nono-cli/src/receipt_commands.rs:359`

```rust
let record_count = format!("{} record(s)", last.sequence + 1);
```

**Issue:** `last.sequence` is a `u64` deserialized straight from the segment file with no range
validation (`read_segment` only checks JSON syntax; `verify_records` is not called on this path). A
corrupt or hand-edited record carrying `"sequence": 18446744073709551615` makes this add overflow:
**panic in a debug build, silent wrap to `0 record(s)` in release**. CLAUDE.md § Coding Standards
requires `checked_`/`saturating_`/`overflowing_` for security-critical math, and this is display of
security evidence.

It is also inconsistent with `list_entry_json` (line 399), which reports `records.len()` for the
same concept — so the human and `--json` outputs of the same command can disagree on the record
count for any segment whose sequences are not contiguous (which, per CR-02, is a state the system
produces itself).

**Fix:** use the actual parsed count, matching the JSON path:

```rust
let record_count = format!("{} record(s)", records.len());
```

---

### WR-04: three doc comments in the diff still assert the read protection Task 3 falsified

**File:** `crates/nono-cli/src/receipt_sink.rs:260-275`; `crates/nono/src/sandbox/windows.rs`
(`deny_sid_on_path` doc, "Why BOTH a DENY ACE and a mandatory label are required")

**Issue:** commit `050d0710` amended the *module-level* docs with the D-08 SCOPE CORRECTION
("a confined child CAN read every receipt file in the sink"). Three item-level doc comments in the
same files were not amended and still claim read coverage. A reader hovering the constant, or
reading the function's own rustdoc, sees only the falsified claim:

- `RECEIPT_SINK_LABEL_MASK` (receipt_sink.rs:269-275): *"`NO_READ_UP` (**the guard's actual job** —
  see module doc)"*. Per the correction, `NO_READ_UP` on this object does nothing at all: the label
  is pinned to a LOW RID (so a Low-IL subject is equal, not below) and carries no `(OI)(CI)` (so it
  never attaches to the receipt files).
- `RECEIPT_SINK_DENY_MASK` (receipt_sink.rs:260-267): *"Comprehensive — **read (reconnaissance,
  T-118-14)**, write/delete (tampering, T-118-15)"*. The read half was measured not to hold for the
  files.
- `deny_sid_on_path` (windows.rs, the bullet list): *"A DENY ACE naming the per-session synthetic
  SID (or the package SID on the AppContainer arm) **closes exactly this read-bypass**"*. The Task 3
  correction appears above this text but does not retract this sentence, which reads as a live claim.

This is the same defect class Task 3 itself found. A future caller reading only the const or the
function doc will believe receipts are confidential.

**Fix:** amend all three in place, e.g.:

```rust
/// Mandatory-label mask applied to the sink directory (D-08's label half).
/// **This mask provides no read protection for the receipt FILES** — the
/// label is pinned to a LOW RID and carries no `(OI)(CI)`, so it neither
/// blocks a Low-IL subject nor propagates to files (Plan 118-10 Task 3
/// measurement; see this module's "D-08 SCOPE CORRECTION"). It is retained
/// for directory-object hygiene only; `NO_WRITE_UP` is deliberately absent
/// because the supervisor must still be able to WRITE receipts.
const RECEIPT_SINK_LABEL_MASK: u32 = ...;
```

and strike/qualify the "closes exactly this read-bypass" sentence in `deny_sid_on_path`.

---

### WR-05: the "MIRROR ASYMMETRY" comment claims a guard that is inert on four of five CLI token arms

**File:** `crates/nono-cli/src/agent_daemon/launch.rs` (`daemon_emit_enforcement_receipt`, the
"MIRROR ASYMMETRY" block) and `crates/nono-cli/src/exec_strategy_windows/launch.rs:1935-1936`

**Issue:** the comment states the asymmetry is *"Harmless under the narrowed write-integrity claim,
**because each arm denies the SID its own confined child actually carries**."* That is true for the
daemon and for the CLI's `WriteRestricted` arm. It is **false for the other four CLI arms.**

`execution_runtime.rs:606` sets `session_sid: Some(exec_strategy::generate_session_sid())`
unconditionally, so `expected_session_sid` is always `Some(...)` at
`ensure_sink_guarded(sink_dir, expected_session_sid, None)`. But the synthetic session SID is only
*placed in a child token* on the `WriteRestricted` arm. On `Null` and `LowIlPrimary` the child gets
no restricting SID; on `BrokerLaunch`/`BrokerLaunchNoPty` the child is `nono-shell-broker.exe`
spawned via plain `CreateProcessW` *at the caller's identity* (launch.rs:2413, 2763 — "broker runs
at caller's identity"). On those four arms the DENY ACE names a SID **no subject holds**, so the
write-integrity guard `nono.exe` applies is inert for its own child, and each launch merely
contributes one more permanent ACE (see CR-05).

Practical coverage on the broker arms is restored only because the broker separately calls
`ensure_broker_receipt_sink_guarded(dir, package_sid)` for its own grandchild — a different process,
a different SID, and only when it gets far enough to write a receipt.

**Fix:** correct the comment to state which arms it actually covers, and — better — thread
`config.package_sid` through `apply_startup_attestation_gate` so the CLI call site can pass both
halves:

```rust
crate::receipt_sink::ensure_sink_guarded(sink_dir, expected_session_sid, package_sid)
```

The comment's own closing sentence already anticipates this ("any future widening must fix BOTH
sites together").

---

### WR-06: the D-14 content-free scan covers only `EnforcementReceipt`, not `LayerReceiptRow`, which is serialized inside every receipt

**File:** `crates/nono-cli/tests/receipt_content_free_scan.rs:85-130, 182-195`

**Issue:** the scan enforces the type allowlist on `pub struct EnforcementReceipt`'s own fields
only. `LayerReceiptRow` is a `pub struct` with `Serialize`/`Deserialize` that is embedded in every
receipt via `layers: Vec<LayerReceiptRow>` and therefore lands verbatim in the JSONL sink. A future
`pub detail: String` or `pub path: PathBuf` on `LayerReceiptRow` would serialize into every receipt
and **pass this scan unchanged** — the mechanical D-14 gate would report green while the
content-free property was violated. This is a class-coverage gap of exactly the kind
`<priority_checks>` #3 describes: the guard covers the instance someone noticed, not the class.

Secondary: `field_types_in_struct` uses `source.find(&marker)` — the *first* occurrence. If a second
`pub struct EnforcementReceipt {` ever appears earlier in the file (a doc-comment example, a
`#[cfg(test)]` fixture), the scan silently audits the wrong body. The sibling scan in
`receipt_commands.rs` guards against exactly this with a
`..._marker_is_present_exactly_once_as_a_real_definition` test; this file has no equivalent.

**Fix:** iterate the set of serialized receipt types rather than naming one, and add a
single-definition guard:

```rust
const SCANNED_STRUCTS: &[&str] = &["EnforcementReceipt", "LayerReceiptRow"];

#[test]
fn every_serialized_receipt_struct_passes_the_type_allowlist_scan() {
    let source = read_receipt_source();
    for name in SCANNED_STRUCTS {
        let marker = format!("pub struct {name} {{");
        assert_eq!(source.matches(&marker).count(), 1,
            "expected exactly one definition of {name}");
        let fields = field_types_in_struct(&source, name);
        assert!(!fields.is_empty(), "discovery failure on {name}");
        assert!(classify_fields(&fields).is_empty(),
            "{name} has non-allowlisted field types");
    }
}
```

(`ALLOWED_TYPES` needs `LayerId` and `LayerAttestationStatus` added for `LayerReceiptRow`.)

---

### WR-07: `attest_and_decide(input)?` is a fourth receipt-less terminal exit — latent today, live the moment RF-13 is wired

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1723`

**Issue:** the gate's census is built before the decision, but the decision is consumed with `?`:

```rust
let census = attestation::census_from_entries(entries, &input);
let pid = unsafe { GetProcessId(process) };
match attestation::attest_and_decide(input)? {   // <-- Err here skips all three receipt arms
```

`attest_and_decide` returns `Err(NonoError::LayerAttestationFailed)` when
`required_layers_override` or `machine_required_layers` names an unrecognised `LayerId`
(attestation.rs:757-764). Both slices are hard-coded empty today (the `RF-13 OPEN` comment at
launch.rs:1692-1702), so the branch is currently unreachable — but RF-13's stated resolution is
precisely to thread the already-read machine policy into this call site, at which point a single
admin typo in `HKLM\...\RequiredLayers` becomes a confined session that terminates its child and
emits **no receipt at all**. The census is already computed and sitting in scope one line above.

**Fix:** handle the error explicitly instead of `?`, reusing the census that already exists:

```rust
let decision = match attestation::attest_and_decide(input) {
    Ok(d) => d,
    Err(e) => {
        let _ = emit_enforcement_receipt(
            sink_dir, census, session_id, expected_session_sid, pid,
            entry_path, token_arm, nono::SessionOutcome::Refused,
            require_receipts_override,
        );
        return Err(e);
    }
};
match decision { /* ... */ }
```

---

## Info

### IN-01: the two writers handle an invalid `session_id` in opposite directions

**File:** `crates/nono-cli/src/receipt_sink.rs:399-427` vs `crates/nono-shell-broker/src/main.rs`
(`BrokerReceiptWriter::new`)

`ReceiptWriter::new` **fails closed** on an empty or non-filename-safe `session_id` (its own test
`write_receipt_rejects_unsafe_session_id_for_filename` pins this). `BrokerReceiptWriter::new`
**degrades** the identical input to the literal `"unknown-session"`. Two producers writing into one
shared sink, read by one deserializer, should not disagree about whether the same value class is
fatal. The broker's justification cites `NONO_SESSION_ID`'s "accept empty" house convention, but
that convention is about *audit correlation*, not about choosing a filename. See CR-03 for the
consequence.

**Fix:** make the broker fail the receipt write (visibly, via its existing D-04 `tracing::warn!`
degrade) rather than collapsing distinct sessions onto one segment file.

### IN-02: side effect inside a `map_err` closure

**File:** `crates/nono-cli/src/agent_daemon/launch.rs` (step 7b, `daemon_state.tenants.lock()`)

`write_daemon_refuse_receipt_best_effort(...)` is invoked inside the `.map_err(|_| { ... })` closure
whose declared job is constructing a `NonoError`. It works (the closure runs only on the poisoned-
lock path), but it hides a file write and two OS probes inside what reads as pure error mapping.
Hoisting it to an explicit `if let Err(_) = ... { write...; return Err(...); }` matches the idiom
every neighbouring refuse site uses.

### IN-03: `!body.contains("return")` is a substring match, not a token match

**File:** `crates/nono-cli/tests/layer_registry_meta_test.rs`
(`census_from_entries_has_no_early_return_and_is_registry_driven`)

The assertion matches the substring `return`, so an innocuous comment containing "returns" inside
`census_from_entries`'s body would fail the test. The failure direction is safe (a false alarm, not
a false pass), but it will surprise a future editor. Matching `"return "`/`"return;"` at a trimmed
line start would be equivalent in strength and less brittle.

### IN-04: `resolve_sink_dir` / `broker_receipt_sink_dir` are a hand-duplicated constant pair

**File:** `crates/nono-cli/src/receipt_sink.rs:284-287` and `crates/nono-shell-broker/src/main.rs`

The two functions must agree byte-for-byte (the broker's own doc says so) but are maintained
independently in two crates, with the `"receipts"` component spelled as a named const on one side
(`RECEIPT_SINK_DIRNAME`) and a bare literal on the other. Since `nono-shell-broker` already depends
on `nono` core, promoting the resolver into `crates/nono` (alongside `receipt_chain.rs`) would
remove the drift surface and give CR-04's validation one place to live rather than two.

---

_Reviewed: 2026-09-05T22:10:56Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

---

# Orchestrator verification pass (2026-09-05, post-review)

Every Critical was independently re-checked against the code before being reported to the operator,
per this project's standing rule that a reviewer's confidence is not evidence. Results below. One
finding's stated consequence is **corrected**, and one **new** finding was discovered empirically
during verification.

## Criticals: 5 of 5 stand

| ID | Verified how | Verdict |
|---|---|---|
| CR-01 | Read `exec_strategy_windows/launch.rs:2903-2936` | **CONFIRMED.** Both pre-gate exits (`apply_process_handle_to_containment`, `apply_resource_limits`) `terminate + return Err` with no emission. Worse, the gate writes its `Ran` receipt BEFORE `resume_contained_process(...)?` (receipt.rs documents `Ran` as "was resumed"), so a failed resume leaves a `Ran` receipt for a process that executed zero instructions. |
| CR-02 | Read `ReceiptWriter::new` | **CONFIRMED.** Opens `create(true).append(true)` and initialises `head: [0u8;32], sequence: 0` without ever reading existing records. A second writer on an existing segment re-emits `sequence: 0, prev_head: null`, which `verify_records` reports as reorder/deletion. |
| CR-03 | Grepped every `NONO_SESSION_ID` writer in `nono-cli` | **MECHANISM CONFIRMED, CONSEQUENCE CORRECTED — see below.** |
| CR-04 | Read `resolve_sink_dir` (`receipt_sink.rs:284-287`) | **CONFIRMED VERBATIM.** `std::env::var("PROGRAMDATA").unwrap_or_else(...)` — raw, unvalidated, uncanonicalised. Redirecting it relocates guard + writer + write together so all three SUCCEED, meaning `record_receipt_write_outcome` never runs and the admin-only `RequireReceipts` HKLM control is never consulted. Directly violates CLAUDE.md's "Validate environment variables before use." |
| CR-05 | **Independently corroborated by live measurement during Task 3** | **CONFIRMED.** `icacls` on the sink showed the DENY ACE count grow 5 → 7 across two broker sessions, one per session, never revoked. |

## CR-03 correction: the consequence is worse than reported

The review states broker receipts "collapse into one shared `unknown-session.broker.jsonl`". The
mechanism is right — `broker_env_pairs` (`launch.rs:2133`) pushes only the two wire-contract vars,
and no code in `nono-cli` sets `NONO_SESSION_ID` outside the hook paths
(`hook_runtime.rs:196`, `hook_runtime_windows.rs:105,171`) — and `BrokerReceiptWriter::new`
degrades an empty id to the literal `"unknown-session"`.

**But that file does not exist, and no broker receipt exists at all.** Measured on this host after
THREE broker-arm sessions run during Task 3:

```
find /c/ProgramData/nono /c/Users/OMack/AppData -name "*.broker.jsonl"   → (no results)
```

The three sessions each produced exactly ONE receipt, all from `nono.exe`:

```
b20cf38788b791ba.jsonl | entry_path=DirectCli token_arm=BrokerLaunchNoPty outcome=Ran
dbedffbeb593558d.jsonl | entry_path=DirectCli token_arm=BrokerLaunchNoPty outcome=Ran
efe66db47bf1b65f.jsonl | entry_path=DirectCli token_arm=BrokerLaunchNoPty outcome=Ran
```

D-15 specifies TWO receipts per broker session. One is present.

## CR-06 — RESOLVED 2026-09-06 (commit `4d0c5ded`), both arms live-verified

**Root cause (found via `/gsd:debug`, session `broker-receipt-not-written`): the receipts were
being written all along, to a sandbox-local path nobody searched.** `broker_env_pairs` cloned the
CONFINED CHILD's sanitized environment, in which `append_windows_runtime_env` had already rewritten
`PROGRAMDATA` to the per-workdir runtime redirect. The broker faithfully resolved its sink to
`<workdir>\.nono-runtime\programdata\nono\receipts`. That is why the failure produced no error, no
warning, and no missing file at the expected location — nothing failed.

**The hypothesis I eliminated below as #2 was WRONG and is retracted.** "The broker resolves a
different sink directory" was correct; I retired it on faulty evidence, having assumed the child
environment was either inherited unchanged or cleared. It was *rewritten* — a third possibility I
never considered — and the confirming filesystem sweep covered `%PROGRAMDATA%` and `%LOCALAPPDATA%`
but not the workdir. Of the three assumptions in "the contradiction to resolve", **#3 was the false
one**: the file existed.

The smoking gun, recovered intact:
`C:\Users\OMack\nono-probe\.nono-runtime\programdata\nono\receipts\unknown-session.broker.jsonl`
— exactly 4 records, one per reported session, and the `unknown-session` filename independently
corroborates CR-03.

**Fix:** the sink base moves off the environment onto argv (`--receipt-sink-base`), pushed on BOTH
arms through one shared helper; `broker_receipt_sink_dir()` can no longer read the environment and
fails closed with no fallback. Note this is the CR-04 defect class (an env-derived sink location)
being triggered by nono against itself — and CR-04's own fix is what would have surfaced it, since
the user-owned redirect base now fails validation loudly instead of misdirecting silently.

**Live verification on BOTH arms — measured, not by construction:**

| Check | `BrokerLaunchNoPty` | `BrokerLaunch` (PTY) |
|---|---|---|
| Arm confirmed via supervisor receipt `token_arm` | ✅ | ✅ `BrokerLaunch` |
| Broker receipt in the real `%PROGRAMDATA%` sink | ✅ `5754259428069866` | ✅ `60990aea032cfab8` |
| D-15 pair (`<sid>.jsonl` + `<sid>.broker.jsonl`) | ✅ | ✅ |
| Negative control: nothing in `.nono-runtime` redirect | ✅ | ✅ |
| `nono receipt verify <sid>` | ✅ | ✅ exit 0, **both segments chain-intact** |

D-15's two-receipts-per-broker-session contract now holds in practice for the first time.

**Impact while it was live (2026-08-16 → 2026-09-06):** RCPT-01 was violated on both broker arms,
and D-15 cross-binary correlation never worked — the receipts existed but were unfindable and named
`unknown-session`. Four pre-fix receipts remain stranded at the path above; no migration attempted.

**A related exposure, recorded as plausible and NOT measured:** the misdirected sink sat inside the
confined child's writable workspace. Its directory object WAS guarded (four AppContainer package-SID
DENY ACEs plus the Low mandatory label — verified with `icacls`), so the receipts were not left bare.
But per Task 3's established finding the DENY is directory-scoped and does not reach files, and
files there inherit Full Control for the invoking user, so a confined child could plausibly have
modified or deleted its own broker receipts. Moot now that the sink is off-env. Logged because it is
a second instance of the same directory-scoped-guard class as the D-08 narrowing, which strengthens
the case for revisiting that decision rather than leaving it merely narrowed.

---

### Original finding as written (superseded by the resolution above)

## NEW — CR-06: the broker emits no receipt on the successful path (RCPT-01)

`record_broker_receipt`'s own doc comment (`nono-shell-broker/src/main.rs:746-750`) claims it is
"Called at EVERY point [`run`] decides `Ran` or `Refused`, so a receipt exists before the child
could ever execute an instruction." A `SessionOutcome::Ran` call site does exist at line 1448, on
the main path, unconditionally after the attestation gate and immediately before `ResumeThread`.

It did not produce a file on three consecutive successful sessions. **This is an RCPT-01 violation
on the broker arm** — "every confined session emits a receipt" — and RCPT-01 is one of the three
requirements this phase exists to deliver.

**Root cause NOT established.** Candidates ruled out during verification:

- *Stale binary* (the trap from quick task `260815-gfd` Cluster C): ruled out —
  `target/debug/nono-shell-broker.exe` is dated Sep 4 23:56, while the broker source last changed
  2026-08-16 (`c7a237ef`).
- *Different sink path*: ruled out — `broker_receipt_sink_dir()` (`main.rs:566-569`) resolves
  `PROGRAMDATA` identically to `resolve_sink_dir()`, and the env-clear fallback lands on the same
  literal `C:\ProgramData`.
- *Path never reached*: unlikely — the child ran and exited 0, so `ResumeThread` executed, so line
  1448 was passed.
- *Silent guard failure*: plausible but unconfirmed — `record_broker_receipt` early-returns on
  `ensure_broker_receipt_sink_guarded` failure with only a `tracing::warn!`, and no such warning
  appeared in console output that was otherwise carrying broker `INFO` lines.

Reproduce with: `nono run -p claude-code --allow-cwd -- cmd /c echo hi` from a `%USERPROFILE%`
subdirectory, then check for `*.broker.jsonl` in `%PROGRAMDATA%\nono\receipts`.

## Note on CR-05 and the phase's own records

CR-05 duplicates a finding already logged independently in this phase's `deferred-items.md`
("Sink DENY ACEs accumulate without bound"), reached there by live `icacls` measurement rather than
code reading. Two independent routes to the same defect raises confidence; it is one defect, not two.
