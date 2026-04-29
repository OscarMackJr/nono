# Phase 23: Windows Audit-Event Retrofit — Pattern Map

**Mapped:** 2026-04-28
**Files analyzed:** 5 (1 modified, 4 reference analogs)
**Analogs found:** 5 / 5 (every file has an in-fork analog from Phase 22-05a/b or Phase 18.1)

> **Cross-cuts every plan task:**
> - D-19 byte-identical preservation: pre/post structural-grep diffs on `crates/nono/src/`, `crates/nono-cli/src/terminal_approval.rs`, `crates/nono-cli/src/profile/`, `crates/nono-cli/data/` MUST be EMPTY across this phase's commits.
> - D-21 Windows-invariance: changes belong in `exec_strategy_windows/` and `audit_integrity.rs`. Cross-platform files (`exec_strategy.rs`, `supervised_runtime.rs`, `rollback_runtime.rs`) get the new parameter passed through with NO behavioral change for non-Windows.
> - DCO sign-off mandatory; clippy `-D warnings -D clippy::unwrap_used`; tests use `EnvVarGuard` save/restore.
> - The 5 push sites at `supervisor.rs:1795/1818/1849/1891/1997` are the funnel. Single-site discipline (G-04) means recorder calls piggyback on the existing `audit_log.push` shape — no per-kind helper signature changes.

---

## File Classification

| New / Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (modify) | service (dispatcher) | event-driven (pipe → decision → ledger) | Existing 5 `audit_log.push` sites at `supervisor.rs:1795–1997` + Phase 22-05a `record_session_started` wiring at `supervised_runtime.rs:362–367` | exact (self-analog: extends an already-discriminated 5-site funnel) |
| `crates/nono-cli/src/audit_integrity.rs` (modify — add `reject_stage` field on `AuditEventPayload::CapabilityDecision`) | model (serde struct) | transform (typed → NDJSON) | `AuditEventPayload` itself at `audit_integrity.rs:28–52` (the SessionStarted/SessionEnded variants are the field-shape analog) | exact (in-file analog) |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` (modify — thread `audit_recorder` into `handle_windows_supervisor_message` call site at `supervisor.rs:527`) | controller (entry point glue) | request-response (parameter threading) | `mod.rs:695, 811` already threads `audit_recorder` into `finalize_supervised_exit`; same shape extends to dispatcher | exact |
| `crates/nono-cli/src/audit_commands.rs` (modify — surface `reject_stage` in `cmd_show` rendering + JSON) | controller (CLI render) | transform (struct → text/JSON) | `audit_commands.rs:361–374` (audit-integrity rendering block) + `:480–514` (JSON merge) | exact |
| `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` (extend — add `--audit-integrity` + ledger parsing) | test (integration) | request-response (broker round-trip) + file-I/O (ledger parse) | `aipc_handle_brokering_integration.rs:1–134` (5 round-trip tests) + `audit_integrity.rs:395–442` (ledger replay/tamper test) | exact |

**No greenfield files.** Every change extends an existing analog within the same file or sibling file.

---

## Pattern Assignments

### 1. `supervisor.rs::handle_windows_supervisor_message` — thread `audit_recorder` + emit to ledger at all 5 push sites

**Role:** service (dispatcher) · **Data flow:** event-driven
**Analog:** the function itself + Phase 22-05a `Mutex<AuditRecorder>` lock-and-record idiom at `supervised_runtime.rs:362–367`

#### 1a. Existing function signature to extend (supervisor.rs:1776–1787)

```rust
pub(super) fn handle_windows_supervisor_message(
    sock: &mut nono::SupervisorSocket,
    msg: nono::supervisor::SupervisorMessage,
    approval_backend: &dyn ApprovalBackend,
    target_process: nono::BrokerTargetProcess,
    seen_request_ids: &mut HashSet<String>,
    audit_log: &mut Vec<AuditEntry>,
    expected_session_token: &str,
    user_session_id: &str,
    runtime_containment_job: HANDLE,
    resolved_allowlist: &AipcResolvedAllowlist,
) -> Result<()> {
```

**Add ONE parameter (D-01) — same idiom Phase 22-05a uses on `mod.rs:695`:**
```rust
audit_recorder: Option<&std::sync::Mutex<crate::audit_integrity::AuditRecorder>>,
```

#### 1b. The 5 existing audit_log.push sites (verbatim, supervisor.rs:1795–1997)

**Site 1 — duplicate replay (supervisor.rs:1791–1806):**
```rust
if seen_request_ids.contains(&request.request_id) {
    let decision = nono::ApprovalDecision::Denied {
        reason: "Duplicate request_id rejected (replay detected)".to_string(),
    };
    audit_log.push(audit_entry_with_redacted_token(
        &request,
        &decision,
        approval_backend.backend_name(),
        started_at,
    ));
    return sock.send_response(&nono::supervisor::SupervisorResponse::Decision {
        request_id: request.request_id,
        decision,
        grant: None,
    });
}
```
**Site 2 — invalid token (supervisor.rs:1814–1828):** identical shape, `reason: "Invalid session token".to_string()`.
**Site 3 — unknown HandleKind (supervisor.rs:1845–1859):** identical shape, `reason: "unknown handle type".to_string()`.
**Site 4 — mask gate deny for Event/Mutex/JobObject (supervisor.rs:1872–1904):**
```rust
if matches!(request.kind, HandleKind::Event | HandleKind::Mutex | HandleKind::JobObject) {
    let resolved = match request.kind { /* ... */ };
    if !policy::mask_is_allowed(request.kind, request.access_mask, resolved) {
        let decision = nono::ApprovalDecision::Denied {
            reason: format!("access mask 0x{:08x} not in allowlist for {:?} (resolved: 0x{:08x})", /* ... */),
        };
        audit_log.push(audit_entry_with_redacted_token(&request, &decision, approval_backend.backend_name(), started_at));
        return sock.send_response(/* ... */);
    }
}
```
**Site 5 — final decision after G-04 flip (supervisor.rs:1925–2008):**
```rust
let (decision, grant) = if decision.is_granted() {
    let result: Result<Option<nono::supervisor::ResourceGrant>> = match request.kind {
        HandleKind::File   => /* ... open_windows_supervisor_path + broker_file_handle_to_process ... */,
        HandleKind::Event  => handle_event_request(&request, target_process, user_session_id, resolved_allowlist),
        HandleKind::Mutex  => handle_mutex_request(/* ... */),
        HandleKind::Pipe   => handle_pipe_request(/* ... */),
        HandleKind::Socket => handle_socket_request(/* ... */),
        HandleKind::JobObject => handle_job_object_request(/* ... */),
    };
    match result {
        Ok(g) => (decision, g),
        Err(e) => {
            tracing::warn!("AIPC broker failure for kind {:?}: {}", request.kind, e);
            // G-04: flip decision so audit + wire both record Denied.
            let denied = nono::ApprovalDecision::Denied { reason: format!("broker failed: {e}") };
            (denied, None)
        }
    }
} else {
    (decision, None)
};

audit_log.push(audit_entry_with_redacted_token(
    &request,
    &decision,
    approval_backend.backend_name(),
    started_at,
));
```

#### 1c. Recorder lock-and-record idiom to copy (supervised_runtime.rs:362–367)

```rust
if let Some(recorder_mutex) = audit_recorder.as_ref() {
    let mut recorder = recorder_mutex
        .lock()
        .map_err(|_| nono::NonoError::Snapshot("Audit recorder lock poisoned".to_string()))?;
    recorder.record_session_started(started.clone(), command.to_vec())?;
}
```

**Critical: D-04 / Discretion #3 says recorder errors must NOT abort the supervisor.** Use this shape instead at the 5 dispatcher sites (NOT the `?` propagation above):

```rust
if let Some(recorder_mutex) = audit_recorder {
    match recorder_mutex.lock() {
        Ok(mut recorder) => {
            if let Err(e) = recorder.record_capability_decision(entry.clone()) {
                tracing::warn!(
                    request_id = %request.request_id,
                    error = %e,
                    "Failed to append capability-decision event to audit ledger; \
                     wire response continues normally",
                );
            }
        }
        Err(_) => {
            tracing::warn!(
                request_id = %request.request_id,
                "Audit recorder lock poisoned; skipping ledger emission",
            );
        }
    }
}
```

The `entry` here is the same `AuditEntry` value pushed into `audit_log` immediately above (clone it once; the `audit_log.push` consumes the original by-value, the recorder gets the clone — or vice-versa, planner picks). Prefer to reuse the `audit_entry_with_redacted_token(...)` return value: bind it to a local `let entry = audit_entry_with_redacted_token(...);`, push `entry.clone()` into `audit_log`, hand `entry` to the recorder.

#### 1d. The `record_capability_decision` API to call (audit_integrity.rs:144–147)

```rust
#[allow(dead_code)]
pub(crate) fn record_capability_decision(&mut self, entry: AuditEntry) -> Result<()> {
    self.append_event(AuditEventPayload::CapabilityDecision { entry })
}
```

**Phase 23 removes the `#[allow(dead_code)]` here** — it becomes live as soon as the dispatcher calls it. Per CLAUDE.md "Lazy use of dead code: Avoid `#[allow(dead_code)]`."

#### 1e. The `audit_entry_with_redacted_token` helper to reuse unchanged (supervisor.rs:1279–1294)

```rust
fn audit_entry_with_redacted_token(
    request: &nono::CapabilityRequest,
    decision: &nono::ApprovalDecision,
    backend_name: &str,
    started_at: Instant,
) -> AuditEntry {
    let mut redacted = request.clone();
    redacted.session_token.clear();
    AuditEntry {
        timestamp: SystemTime::now(),
        request: redacted,
        decision: decision.clone(),
        backend: backend_name.to_string(),
        duration_ms: started_at.elapsed().as_millis() as u64,
    }
}
```

**Token-redaction is load-bearing for D-04 sanitization regression test.** Do NOT add a parallel entry-builder; route every recorder call through this single helper.

#### 1f. Ordering choice (Discretion #2)

Lock-once-per-site is the recommended shape because:
- The dispatcher is invoked once per child request from a single pipe-server thread (`supervisor.rs:524–558`); concurrent re-entry within the same `handle_windows_supervisor_message` call is structurally impossible.
- Holding the lock across `sock.send_response(...)` (the I/O call that follows each push site) needlessly extends contention with the labels-guard Drop on shutdown.
- Per-site lock-and-drop matches the existing `supervised_runtime.rs:362` shape (lock, record, drop scope-end).

The `audit_flush_before_drop` invariant (`labels_guard.rs:511–591`) is naturally preserved because `audit_log` is drained into the recorder via `audit_tx.send(local_audit)` at `supervisor.rs:545–547`, which is upstream of the labels-guard Drop site.

---

### 2. `audit_integrity.rs::AuditEventPayload` — add `reject_stage` field to `CapabilityDecision` variant

**Role:** model (serde struct) · **Data flow:** transform
**Analog:** `AuditEventPayload` itself at `audit_integrity.rs:28–52`

#### 2a. Existing variant shape (audit_integrity.rs:28–52)

```rust
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)] // CapabilityDecision/UrlOpen/Network variants and their constructors land in
                    // follow-up cherry-picks 4ec61c29..9db06336 per Plan 22-05a Decision 5.
enum AuditEventPayload {
    SessionStarted {
        started: String,
        command: Vec<String>,
    },
    SessionEnded {
        ended: String,
        exit_code: i32,
    },
    CapabilityDecision {
        entry: AuditEntry,
    },
    UrlOpen {
        request: UrlOpenRequest,
        success: bool,
        error: Option<String>,
    },
    Network {
        event: NetworkAuditEvent,
    },
}
```

#### 2b. D-02 minimal shape change

```rust
CapabilityDecision {
    entry: AuditEntry,
    /// Windows-AIPC-specific reject-stage marker. `None` for Approved
    /// decisions, for non-Windows ledger entries, and for the three
    /// pre-stage rejections (duplicate replay, invalid token, unknown
    /// HandleKind). `Some(BeforePrompt)` when the mask gate at
    /// `supervisor.rs:1884` denies before the approval backend is
    /// consulted (Event/Mutex/JobObject). `Some(AfterPrompt)` when the
    /// G-04 broker-failure flip at `supervisor.rs:1987` denies after
    /// approval (Pipe direction allowlist + Socket privileged port /
    /// role allowlist).
    ///
    /// Currently observable for exactly two HandleKinds: Pipe and
    /// Socket. Future kinds may extend this; until then the matrix is
    /// locked by the WR-01 verdict matrix in
    /// `supervisor.rs::capability_handler_tests` module docstring
    /// (lines 2034–2076).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reject_stage: Option<RejectStage>,
},
```

**`#[serde(default, skip_serializing_if = "Option::is_none")]`** is the canonical backward-compat pattern (Phase 22-01 PROF-01 used `#[serde(default)]` for the same reason — see `crates/nono-cli/src/profile/mod.rs:1180–1222`). It guarantees:
- Old `audit-events.ndjson` files written by Phase 22 deserialize without the field (treated as `None`).
- New events written by Phase 23 omit the field on the `None` path, keeping non-Windows entries byte-identical.

#### 2c. New `RejectStage` enum to add (Discretion #1)

```rust
/// Windows-AIPC reject-stage discriminator (Phase 23 D-02).
///
/// Encodes whether a Denied capability decision was rejected BEFORE the
/// approval backend was consulted (`BeforePrompt` — Event/Mutex/JobObject
/// pre-broker mask gate) or AFTER approval was granted but the per-kind
/// broker helper failed (`AfterPrompt` — Pipe direction, Socket
/// privileged-port + role allowlist; surfaced via the G-04 broker-failure
/// flip at supervisor.rs:1987).
///
/// `None` (the absent-field case) covers: Approved decisions, the three
/// pre-stage early rejections (duplicate replay, invalid token, unknown
/// HandleKind), and all non-Windows entries.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RejectStage {
    BeforePrompt,
    AfterPrompt,
}
```

**Why `kebab-case`:** matches `serde(tag = "type", rename_all = "snake_case")` on the parent enum's tag rendering — but kebab-case is the closer match to existing ledger-payload tokens like `session-started` (Phase 22-05a's `rename_all = "snake_case"` actually emits `session_started`; verify which the planner picks). CONTEXT.md Discretion #1 calls out kebab-case as suggested; planner should grep `audit-events.ndjson` fixtures from existing recorder tests to lock the convention.

#### 2d. The 5 stage-assignment rules (D-02, copy verbatim into plan task)

| Site | Line | Decision shape | `reject_stage` value |
|------|------|----------------|----------------------|
| 1 (duplicate replay) | supervisor.rs:1795 | Denied | `None` (pre-stage) |
| 2 (invalid token) | supervisor.rs:1818 | Denied | `None` (pre-stage) |
| 3 (unknown HandleKind) | supervisor.rs:1849 | Denied | `None` (pre-stage) |
| 4 (mask gate deny — Event/Mutex/JobObject) | supervisor.rs:1891 | Denied | `Some(BeforePrompt)` |
| 5a (final, Approved) | supervisor.rs:1997 | Approved | `None` |
| 5b (final, File / Event/Mutex/JobObject after gate already passed → Denied is unreachable here) | supervisor.rs:1997 | (unreachable) | n/a |
| 5c (final, Pipe / Socket Denied via G-04 `broker failed:`) | supervisor.rs:1997 | Denied | `Some(AfterPrompt)` |

**Site 5 detection:** the planner needs to discriminate between Approved and the G-04-flipped Denied at site 5. Read the local `decision` variable AFTER the `(decision, grant)` rebind (supervisor.rs:1995). If it's `Denied { reason }` AND `reason.starts_with("broker failed:")` AND `request.kind` is `HandleKind::Pipe | HandleKind::Socket`, set `Some(AfterPrompt)`. Otherwise `None`.

A cleaner alternative: track a local `let mut reject_stage: Option<RejectStage> = None;` mutable binding and set it inline at the G-04 flip arm:
```rust
Err(e) => {
    tracing::warn!("AIPC broker failure for kind {:?}: {}", request.kind, e);
    let denied = nono::ApprovalDecision::Denied { reason: format!("broker failed: {e}") };
    if matches!(request.kind, HandleKind::Pipe | HandleKind::Socket) {
        reject_stage = Some(RejectStage::AfterPrompt);
    }
    (denied, None)
}
```

This keeps the stage-derivation co-located with the flip site, avoiding a downstream string-prefix re-parse.

---

### 3. `exec_strategy_windows/mod.rs` — thread `audit_recorder` into the dispatcher call site

**Role:** controller (entry point glue) · **Data flow:** request-response (parameter threading)
**Analog:** the same parameter at the same function, already threaded through `finalize_supervised_exit` at `mod.rs:811`

#### 3a. Existing parameter on `execute_supervised` (mod.rs:684–705)

```rust
#[allow(clippy::too_many_arguments)]
pub fn execute_supervised(
    config: &ExecConfig<'_>,
    supervisor: Option<&SupervisorConfig<'_>>,
    _trust_interceptor: Option<crate::trust_intercept::TrustInterceptor>,
    _on_fork: Option<&mut dyn FnMut(u32)>,
    pty_pair: Option<pty_proxy::PtyPair>,
    session_id: Option<&str>,
    audit_state: Option<AuditState>,
    rollback_state: Option<RollbackRuntimeState>,
    rollback_status: nono::undo::RollbackStatus,
    audit_recorder: Option<&std::sync::Mutex<crate::audit_integrity::AuditRecorder>>,
    /* ... */
) -> Result<i32> {
```

#### 3b. The actual capability-pipe-thread spawn site to extend (supervisor.rs:519–558)

```rust
let mut seen_request_ids = HashSet::new();
loop {
    if terminate_requested.load(Ordering::SeqCst) {
        break;
    }
    match sock.recv_message() {
        Ok(msg) => {
            let mut local_audit: Vec<AuditEntry> = Vec::new();
            if let Err(e) = handle_windows_supervisor_message(
                &mut sock,
                msg,
                backend.as_ref(),
                broker_target,
                &mut seen_request_ids,
                &mut local_audit,
                &session_token,
                &user_session_id,
                runtime_containment_job_local.0,
                resolved_aipc_allowlist.as_ref(),
            ) {
                tracing::warn!(
                    session_id = %session_id,
                    error = %e,
                    "Capability pipe handler returned an error",
                );
            }
            if !local_audit.is_empty() {
                let _ = audit_tx.send(local_audit);
            }
        }
        Err(e) => { /* ... */ break; }
    }
}
```

**Threading task for the planner:**
1. The capability-pipe thread is spawned somewhere upstream of line 519 (search for `start_streaming` and `set_child_broker_target` in `mod.rs:794–800`). Trace where the closure that owns this loop is constructed — `audit_recorder: Option<&Mutex<AuditRecorder>>` cannot cross thread boundaries by reference (lifetime constraint). Either:
   - **(a)** Clone the `Arc<Mutex<AuditRecorder>>`-style ownership at supervised_runtime construction time and `move` an `Option<Arc<Mutex<AuditRecorder>>>` into the closure; OR
   - **(b)** Wrap the recorder in `Arc<Mutex<AuditRecorder>>` (the existing `supervised_runtime.rs:235–242` builds a bare `Mutex<AuditRecorder>`; promoting to `Arc<Mutex<...>>` is the canonical Rust idiom for cross-thread shared mutable state).
2. **Recommended:** option (b) — change `supervised_runtime.rs:235` from `Mutex::new` to `Arc::new(Mutex::new(...))` and propagate the `Arc` type through `mod.rs:695`, `exec_strategy.rs:486`, and `rollback_runtime.rs:46`. The `as_ref()` / `Option<&Arc<Mutex<...>>>` pattern still works for passing to `finalize_supervised_exit`. Cross-platform `exec_strategy.rs` accepts the new type without behavioral change.

#### 3c. Non-Windows silencer pattern (D-21 reminder)

Cross-platform files (`exec_strategy.rs`, `supervised_runtime.rs`, `rollback_runtime.rs`) already accept the parameter — no new silencer needed. If any new helper extends the parameter list and the binding ends up unused on non-Windows, use the Phase 18.1 idiom (planner verifies; only add if a clippy `unused_variables` warning surfaces):

```rust
#[cfg(not(target_os = "windows"))]
let _ = &audit_recorder;
```

---

### 4. `audit_commands.rs::cmd_show` + `print_show_json` — surface `reject_stage`

**Role:** controller (CLI render) · **Data flow:** transform
**Analog:** the audit-integrity rendering block at `audit_commands.rs:361–374` + JSON merge at `:480–514`

#### 4a. Existing human-readable rendering pattern (audit_commands.rs:357–374)

```rust
// Plan 22-05a Decision 5 minimal scope: surface the audit-integrity
// summary (chain_head + merkle_root + event_count) when the session was
// recorded with `--audit-integrity`. Fields are absent for sessions
// recorded before this commit landed.
if let Some(integrity) = session.metadata.audit_integrity.as_ref() {
    eprintln!();
    eprintln!("  Audit Integrity:");
    eprintln!("    Algorithm:     {}", integrity.hash_algorithm);
    eprintln!("    Event count:   {}", integrity.event_count);
    eprintln!("    Chain head:    {}", integrity.chain_head);
    eprintln!("    Merkle root:   {}", integrity.merkle_root);
} else if session.metadata.audit_event_count > 0 {
    eprintln!();
    eprintln!(
        "  Audit Events:   {} (no integrity summary)",
        session.metadata.audit_event_count
    );
}
```

**Apply for Phase 23:** the `cmd_show` function does NOT currently iterate per-event NDJSON records — it surfaces the summary only. To render `reject_stage` per event, the planner must either:
- **(a)** Add a "Capability Decisions" block that re-reads `<session_dir>/audit-events.ndjson` (the file path is `audit_integrity::AUDIT_EVENTS_FILENAME = "audit-events.ndjson"`, exposed via `audit_integrity.rs:10`), filters records where `event.type == "capability_decision"`, and prints decision + reject_stage. The pattern to copy is the existing network-events block at `audit_commands.rs:408–451`:

```rust
if !session.metadata.network_events.is_empty() {
    eprintln!();
    eprintln!("  Network Events: {}", session.metadata.network_events.len());
    for event in &session.metadata.network_events {
        let decision = match event.decision {
            nono::undo::NetworkAuditDecision::Allow => "allow".green(),
            nono::undo::NetworkAuditDecision::Deny => "deny".red(),
        };
        let mode = network_mode_label(&event.mode);
        let mut target = sanitize_for_terminal(&event.target);
        // ... details vec ...
        if details.is_empty() {
            eprintln!("    {} {} {}", decision, mode, target);
        } else {
            eprintln!("    {} {} {} ({})", decision, mode, target, details.join(", "));
        }
    }
}
```

- **(b)** Defer per-event rendering: emit a counter line `Capability Decisions: N (M before-prompt, K after-prompt rejections)` aggregated from a one-pass NDJSON read.

**Recommendation:** option (b) for v2.2 (matches AUD-05's "ledger reflects each kind's reject stage" — the field's PRESENCE in NDJSON is the load-bearing requirement; per-event terminal rendering is a UX add). Per-event browsing can use `nono audit show <id> --json` and external tooling (jq).

#### 4b. JSON merge pattern to extend (audit_commands.rs:480–514)

```rust
// Plan 22-05a Decision 5 minimal scope: include audit-integrity summary
// in JSON output when present. Absent for sessions recorded before this
// commit landed; null is the canonical absence marker.
let audit_integrity_json = session.metadata.audit_integrity.as_ref().map(|s| {
    serde_json::json!({
        "hash_algorithm": s.hash_algorithm,
        "event_count": s.event_count,
        "chain_head": s.chain_head.to_string(),
        "merkle_root": s.merkle_root.to_string(),
    })
});

// ... output struct merges audit_integrity_json + executable_identity_json ...

let output = serde_json::json!({
    "session_id": session.metadata.session_id,
    /* ... */
    "audit_event_count": session.metadata.audit_event_count,
    "audit_integrity": audit_integrity_json,
    "snapshots": snapshots,
});
```

**Apply for Phase 23:** add a `capability_decisions` field that re-reads the ledger and emits one JSON object per event:

```rust
let capability_decisions_json: Option<Vec<serde_json::Value>> = read_capability_decisions_from_ledger(&session.dir).ok();
// where read_capability_decisions_from_ledger is a new helper in this file
// that mirrors verify_audit_log's reader pattern (audit_integrity.rs:280–327)
// but only collects CapabilityDecision events.

let output = serde_json::json!({
    /* ... existing fields ... */
    "audit_integrity": audit_integrity_json,
    "capability_decisions": capability_decisions_json,
    "snapshots": snapshots,
});
```

The reader helper should reuse the BufReader+lines pattern from `verify_audit_log` (audit_integrity.rs:280–327) — DO NOT introduce a parallel NDJSON parser.

---

### 5. `aipc_handle_brokering_integration.rs` — extend with `--audit-integrity` + ledger parsing

**Role:** test (integration) · **Data flow:** request-response + file-I/O
**Analog:** the file itself (`aipc_handle_brokering_integration.rs:1–134`) for round-trip shape + `audit_integrity.rs:395–442` for ledger replay/verification

#### 5a. Existing test gating (aipc_handle_brokering_integration.rs:28–35)

```rust
#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use nono::supervisor::policy;
use nono::supervisor::{GrantedResourceKind, PipeDirection, ResourceTransferKind, SocketRole};
use nono::BrokerTargetProcess;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
```

**The existing 5 round-trip tests (Event/Mutex/Pipe/Socket/JobObject) call the lower-level `nono::supervisor::socket::broker_*_to_process` directly** — they do NOT exercise `handle_windows_supervisor_message`. Phase 23's E2E test (D-04 layer 2) extends the file with NEW tests that:
1. Construct an `AuditRecorder` over a `tempfile::TempDir` session dir.
2. Build the same `make_request_aipc`-style requests as the dispatcher unit tests (planner can copy the helper from `supervisor.rs:2147–2185` into a shared test util OR duplicate it inline — Discretion #4).
3. Call `handle_windows_supervisor_message` with `Some(&Mutex::new(recorder))`.
4. After the dispatcher returns, drop the recorder, then read `<temp_dir>/audit-events.ndjson` line-by-line (or call `verify_audit_log` from `audit_integrity.rs:269–360`).
5. Assert exactly N records of type `capability_decision`, with the WR-01-matching `reject_stage` per request.

#### 5b. Ledger parsing pattern to copy (audit_integrity.rs:395–414)

```rust
#[test]
fn verify_audit_log_accepts_untampered_session() {
    let dir = tempfile::tempdir().unwrap();
    let mut recorder = AuditRecorder::new(dir.path().to_path_buf()).unwrap();
    recorder
        .record_session_started("2026-04-21T00:00:00Z".to_string(), vec!["pwd".to_string()])
        .unwrap();
    recorder
        .record_session_ended("2026-04-21T00:00:01Z".to_string(), 0)
        .unwrap();
    let summary = recorder.finalize().unwrap();

    let result = verify_audit_log(dir.path(), Some(&summary)).unwrap();
    assert!(result.is_valid(), "untampered session must verify");
    assert_eq!(result.event_count, 2);
    /* ... */
}
```

For the integration extension, the assertion shape is closer to the `audit_flush_before_drop` pattern at `labels_guard.rs:544–561`:

```rust
let pre_drop_ledger = std::fs::read_to_string(&ledger_path)
    .expect("ledger file must exist after AuditRecorder lifecycle");
let pre_drop_lines: Vec<&str> = pre_drop_ledger.lines().filter(|l| !l.is_empty()).collect();
assert_eq!(
    pre_drop_lines.len(),
    2,
    "ledger must contain session_started + session_ended BEFORE guard drop; got {} lines:\n{}",
    pre_drop_lines.len(),
    pre_drop_ledger
);
assert!(
    pre_drop_ledger.contains("session_started"),
    "session_started must be flushed before guard drop; ledger:\n{pre_drop_ledger}"
);
```

**For Phase 23 layer-2:**
```rust
let ledger = std::fs::read_to_string(&ledger_path).expect("ledger");
let lines: Vec<&str> = ledger.lines().filter(|l| !l.is_empty()).collect();
assert_eq!(lines.len(), 5, "expected 5 capability_decision events (one per HandleKind), got: {ledger}");
let kinds_seen: Vec<&str> = lines.iter()
    .filter(|l| l.contains("\"capability_decision\""))
    .map(|l| /* extract "kind" field */)
    .collect();
assert!(kinds_seen.contains(&"event"));
assert!(kinds_seen.contains(&"mutex"));
assert!(kinds_seen.contains(&"pipe"));
assert!(kinds_seen.contains(&"socket"));
assert!(kinds_seen.contains(&"job_object"));
```

Or — more robustly — deserialize each line into `AuditEventRecord` (currently `pub(crate)` in `audit_integrity.rs:54–61`; planner may need to expose a test-only deserializer or expose `AuditEventPayload::CapabilityDecision`'s shape via a test helper).

#### 5c. WR-01 dispatcher unit test extension (D-04 layer 1)

The 5 `wr01_*` tests at `supervisor.rs:4180–4534` are the per-kind regression guards. Phase 23's layer-1 extension takes EACH existing test and adds:

1. Replace `let mut audit_log = Vec::new();` with `let dir = tempfile::tempdir().unwrap(); let recorder = std::sync::Mutex::new(AuditRecorder::new(dir.path().to_path_buf()).unwrap());` (plus the existing audit_log).
2. Pass `Some(&recorder)` as the new last parameter to `handle_windows_supervisor_message`.
3. After the call, `drop(recorder);` (forces flush via Drop), then read `dir.path().join("audit-events.ndjson")` and assert the `reject_stage` field on the (single) `capability_decision` event matches the WR-01 verdict-matrix row.

**Pre-existing WR-01 test shape to copy** (supervisor.rs:4180–4238 — `wr01_event_rejects_before_prompt_on_out_of_allowlist_mask`):

```rust
#[test]
fn wr01_event_rejects_before_prompt_on_out_of_allowlist_mask() {
    let backend = CountingGrantBackend::new();
    let (mut supervisor, mut child) = new_pair();
    let mut seen = HashSet::new();
    let mut audit_log = Vec::new();

    let token = "testtoken12345678";
    let out_of_mask: u32 = 0x1000_0000;
    let req = make_request_aipc(token, "wr01-event-001", HandleKind::Event,
        Some(HandleTarget::EventName { name: "wr01-event".to_string() }),
        out_of_mask);
    handle_windows_supervisor_message(
        &mut supervisor,
        nono::supervisor::SupervisorMessage::Request(req),
        &backend,
        nono::BrokerTargetProcess::current(),
        &mut seen,
        &mut audit_log,
        token,
        "testaipc12345678",
        std::ptr::null_mut(),
        &AipcResolvedAllowlist::default(),
    )
    .expect("dispatch");

    assert_eq!(backend.calls(), 0,
        "WR-01 invariant: Event out-of-allowlist mask gate is PRE-prompt — backend MUST NOT be consulted");
    assert_eq!(audit_log.len(), 1);
    match &audit_log[0].decision {
        nono::ApprovalDecision::Denied { reason } => {
            assert!(reason.contains("access mask"), /* ... */);
            assert!(!reason.contains("broker failed:"),
                "WR-01 invariant violated: pre-prompt rejection MUST NOT be G-04-wrapped");
        }
        other => panic!("expected Denied, got {other:?}"),
    }
    let _ = child.recv_response().expect("drain");
}
```

**Phase 23 reject-stage assertion to add (matches site 4 → BeforePrompt):**

```rust
// Phase 23 D-04 ledger assertion — append to every wr01_* test.
let ledger = std::fs::read_to_string(dir.path().join("audit-events.ndjson"))
    .expect("ledger file");
let record: serde_json::Value = serde_json::from_str(ledger.trim().lines().last().unwrap())
    .expect("parse final NDJSON record");
let event = &record["event"];
assert_eq!(event["type"], "capability_decision");
assert_eq!(event["reject_stage"], serde_json::Value::String("before-prompt".to_string()),
    "Event mask-gate rejection MUST carry reject_stage=BeforePrompt");
```

For the Pipe/Socket WR-01 tests (`wr01_pipe_rejects_after_prompt_on_readwrite_default_profile` at supervisor.rs:4398, `wr01_socket_privileged_port_rejects_after_prompt_empirical` at supervisor.rs:4476), the assertion flips to `"after-prompt"`.

#### 5d. Privileged-port socket reject test (D-04 acceptance criterion 2)

The existing `wr01_socket_privileged_port_rejects_after_prompt_empirical` (supervisor.rs:4476–4534) already constructs port-80 + asserts `reason.contains("broker failed:") && reason.contains("privileged port")`. Phase 23 just appends the ledger-side `reject_stage == AfterPrompt` assertion shown above — it's NOT a greenfield test.

The privileged-port reason text is generated at `supervisor.rs:1505–1508`:
```rust
if port <= policy::PRIVILEGED_PORT_MAX {
    return Err(NonoError::SandboxInit(format!(
        "privileged port {port} not allowed (port must be > {})",
        policy::PRIVILEGED_PORT_MAX
    )));
}
```
The G-04 wrapper at `supervisor.rs:1987–1989` prefixes `"broker failed: "` to this — together they form the `"broker failed: privileged port {port} not allowed (port must be > 1023)"` substring the test asserts on.

---

## Shared Patterns

### `Mutex<AuditRecorder>` lock-and-record idiom

**Source:** `crates/nono-cli/src/supervised_runtime.rs:362–367` (`record_session_started` wiring)
**Apply to:** every new dispatcher recorder call site (5× in supervisor.rs)

```rust
if let Some(recorder_mutex) = audit_recorder {
    match recorder_mutex.lock() {
        Ok(mut recorder) => {
            if let Err(e) = recorder.record_capability_decision(entry.clone()) {
                tracing::warn!(/* … */, "audit ledger append failed");
            }
        }
        Err(_) => {
            tracing::warn!(/* … */, "audit recorder lock poisoned");
        }
    }
}
```

**Discretion #3 deviation from supervised_runtime.rs:** the dispatcher MUST NOT propagate the recorder error via `?` (that would abort the supervisor mid-IPC and break the wire-response contract). Use `tracing::warn!` and continue. The wire response goes out regardless.

### Token redaction via `audit_entry_with_redacted_token`

**Source:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:1279–1294`
**Apply to:** every recorder call (do NOT build a fresh `AuditEntry` — reuse the helper's output)

```rust
fn audit_entry_with_redacted_token(
    request: &nono::CapabilityRequest,
    decision: &nono::ApprovalDecision,
    backend_name: &str,
    started_at: Instant,
) -> AuditEntry {
    let mut redacted = request.clone();
    redacted.session_token.clear();
    AuditEntry { /* ... */ }
}
```

**Sanitization regression guard (D-04):** the test
```rust
let json = serde_json::to_string(&audit_log[0]).expect("serialize");
assert!(!json.contains(sensitive_token), "audit JSON must not contain the raw session token");
```
at `supervisor.rs:2298–2329` (`handle_redacts_token_in_audit_entry_json`) is the existing regression. Phase 23 must add an analogous assertion that reads the NDJSON ledger and asserts the same: `assert!(!ledger_string.contains(sensitive_token));`.

### `tempfile::TempDir` audit fixture

**Source:** `crates/nono-cli/src/audit_integrity.rs:369, 386, 397, 419` (existing recorder unit tests) and `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:515` (audit_flush_before_drop test)
**Apply to:** every Phase 23 dispatcher unit test that constructs an AuditRecorder

```rust
let dir = tempfile::tempdir().expect("tempdir");
let session_dir = dir.path().to_path_buf();
let recorder = AuditRecorder::new(session_dir).expect("audit recorder construction");
let recorder_mutex = std::sync::Mutex::new(recorder);
// ... pass &recorder_mutex into handle_windows_supervisor_message ...
// drop(recorder_mutex); // forces flush — the recorder's File handle closes
let ledger = std::fs::read_to_string(dir.path().join("audit-events.ndjson")).expect("ledger");
```

`tempfile::TempDir`'s Drop removes the directory, so no manual cleanup is needed. `tempfile` is already a dev-dep (CONTEXT.md confirms).

### NDJSON record deserialization for assertions

**Source:** `crates/nono-cli/src/audit_integrity.rs:287–327` (verify_audit_log's BufReader+lines pattern)
**Apply to:** every Phase 23 test that asserts on per-event ledger content

```rust
let file = File::open(&events_path).map_err(/* ... */)?;
let reader = BufReader::new(file);
for (line_idx, line_result) in reader.lines().enumerate() {
    let line = line_result.map_err(/* ... */)?;
    if line.trim().is_empty() { continue; }
    let record: AuditEventRecord = serde_json::from_str(&line).map_err(/* ... */)?;
    /* ... assert on record.event, record.sequence, etc. ... */
}
```

For tests inside `capability_handler_tests` mod (which is already `#[allow(clippy::unwrap_used)]` per `supervisor.rs:2032`), `.unwrap()` is acceptable on the deserialization step.

### Boundary deny-list grep gates (D-19)

**Source:** Phase 22-05a §11 commits' end-of-task gates
**Apply to:** every Phase 23 plan task's verification block

```bash
# Pre/post structural-grep diffs MUST be empty across every Phase 23 commit
git diff --stat HEAD~1 HEAD -- crates/nono/src/ crates/nono-cli/src/terminal_approval.rs crates/nono-cli/src/profile/ crates/nono-cli/data/ | grep -v '^$' && echo "FAIL: D-19 invariant violated" || echo "PASS: D-19 invariant preserved"
# Forbidden-pattern grep
grep -nE 'AuditEntry.*reject_stage' crates/nono/src/ && echo "FAIL: D-19 cross-platform AuditEntry mutated" || echo "PASS"
```

The `reject_stage` field MUST appear ONLY on `AuditEventPayload::CapabilityDecision` in `crates/nono-cli/src/audit_integrity.rs`. Any cross-platform `AuditEntry` field addition is a D-19 violation.

### EnvVarGuard save/restore

**Source:** `crates/nono-cli/src/test_env.rs::EnvVarGuard` (per Phase 22 PATTERNS.md § 14)
**Apply to:** any Phase 23 test that mutates env vars (none expected; the dispatcher tests are env-var-clean by construction, but flag if `HOME`/`TMPDIR`/`APPDATA` leakage surfaces).

`tempfile::tempdir()` does NOT mutate `TMPDIR` — it reads it. So Phase 23's tempdir usage does not need EnvVarGuard. Confirm by inspection.

---

## No Analog Found

Every Phase 23 file has an in-fork analog. **Zero greenfield surfaces.** The closest thing to a novel pattern is the `RejectStage` enum itself, but its serde shape mirrors `nono::undo::NetworkAuditDecision` (used at `audit_commands.rs:415–417`) and `nono::supervisor::PipeDirection` — both kebab/snake-cased serde enums already in fork.

---

## Contradictions Surfaced

None. Phase 23 is a clean extension of Phase 22-05a's deferred capability-decision hook. The verdict matrix (WR-01) was empirically locked by Phase 18.1 G-05 and is now mechanically reflected in the ledger via `reject_stage`. No fork-vs-upstream divergence to reconcile (Phase 23 is fork-only — REQ-AUD-05 is not in upstream's audit story).

---

## Metadata

**Analog search scope:**
- `crates/nono-cli/src/exec_strategy_windows/` (supervisor.rs, mod.rs, labels_guard.rs)
- `crates/nono-cli/src/audit_integrity.rs`
- `crates/nono-cli/src/audit_commands.rs`
- `crates/nono-cli/src/supervised_runtime.rs`
- `crates/nono-cli/src/rollback_runtime.rs`
- `crates/nono-cli/src/exec_strategy.rs`
- `crates/nono-cli/tests/aipc_handle_brokering_integration.rs`
- `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md` (cross-reference)

**Files scanned (read or grepped, with concrete excerpts extracted):**
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (4702 lines — targeted reads at 1–60, 510–558, 1270–1360, 1776–2110, 2180–2330, 4170–4310, 4470–4535)
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` (831 lines — targeted reads at 680–831)
- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` (592 lines — targeted read 480–591)
- `crates/nono-cli/src/audit_integrity.rs` (443 lines — full read)
- `crates/nono-cli/src/audit_commands.rs` (756 lines — targeted read 280–520)
- `crates/nono-cli/src/supervised_runtime.rs` (targeted read 220–420)
- `crates/nono-cli/src/rollback_runtime.rs` (targeted reads at 30–75, 530–590)
- `crates/nono-cli/src/exec_strategy.rs` (targeted reads at 475–510, 1220–1245)
- `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` (read 1–150)
- `.planning/phases/23-windows-audit-event-retrofit/23-CONTEXT.md` (full read)
- `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md` (full read for D-19/D-21 invariant cross-reference)

**Pattern extraction date:** 2026-04-28
