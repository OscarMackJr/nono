---
phase: 84-siem-edr-telemetry
plan: "03"
subsystem: telemetry
tags: [denial-wiring, tracing, dual-emit, adr, tamper-evidence, path-deny, network-deny, hook-fail-closed]
dependency_graph:
  requires: [84-01, 84-02]
  provides: [path_deny_wiring, network_deny_wiring, hook_fail_closed_wiring, tamper_evidence_adr]
  affects:
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-proxy/src/audit.rs
    - crates/nono-cli/src/hooks.rs
    - docs/adr/telemetry-tamper-evidence.md
tech_stack:
  added: []
  patterns: [additive-dual-emit, nono_security::* target routing, D-07 ADR discipline]
key_files:
  created:
    - docs/adr/telemetry-tamper-evidence.md
  modified:
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-proxy/src/audit.rs
    - crates/nono-cli/src/hooks.rs
decisions:
  - "OPTION B (label-violation): IL denials surface as path-deny DenialRecords on current Windows backend; EventID 10003 LabelViolation is RESERVED-but-unemitted in Phase 84; Plan 84-04 gate must not assert 10003"
  - "hook_fail_closed wired at hook script write failure in install_claude_code_hook (prevents PreToolUse security hook from running = fail-closed signal)"
  - "D-07 ADR completed: tamper boundary = WEF; in-session HMAC only; SEED-005 deferred; Pitfall 12 honesty recorded"
metrics:
  duration: ~30m
  completed: 2026-06-19
  tasks_completed: 2
  files_created: 1
  files_modified: 3
---

# Phase 84 Plan 03: Denial-Source Wiring + Tamper-Evidence ADR

Connected the three denial sources (path-deny, network-deny, hook fail-closed) to `nono_security::*`
tracing targets so SecurityEventLayer routes them to dual-emit (Application-log + ETW). Created
the D-07 mandatory tamper-evidence ADR.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Denial-source wiring — exec_strategy, audit.rs, hooks.rs | `6ddbe784` | exec_strategy.rs, audit.rs, hooks.rs |
| 2 | Tamper-evidence ADR (D-07 mandatory deliverable) | `dbb066f7` | docs/adr/telemetry-tamper-evidence.md |

## What Was Built

### Task 1: Denial-source wiring

**exec_strategy.rs — path-deny wiring (post-child-exit):**

After `print_diagnostic_footer`, iterates over `&denials` (the `Vec<DenialRecord>` already
in scope at that call site). For each denial, emits:

```rust
tracing::warn!(
    target: "nono_security::path_deny",
    path = %denial.path.display(),
    access = %denial.access,
    agent_pid = std::process::id(),
    "path deny"
);
```

The `path` field carries the display string to `SecurityEventLayer::on_event`, which hashes it
via `path_hash_for` before constructing `SecurityEvent`. The raw path NEVER reaches `ReportEventW`
(D-08 boundary). `access` is the `AccessMode` enum display (e.g. `Read`, `ReadWrite`), scrubbed
inside the Layer before emit.

**nono-proxy/audit.rs — network-deny additive wiring:**

Added `tracing::warn!(target: "nono_security::network_deny", ...)` AFTER the existing
`info!(target: "nono_proxy::audit", ...)` call in `log_denied()`. The existing `info!` call
is preserved (invariant 5: additive only, never replace). `host` stays cleartext (D-10 exception:
FQDN is what analysts need per SC-1). No full URLs, path components, or query params. `reason`
is NOT forwarded at the call site (scrubbing happens inside SecurityEventLayer).

**hooks.rs — hook fail-closed wiring:**

Added `tracing::warn!(target: "nono_security::hook_fail_closed", ...)` inside the `.map_err()`
closure on the `fs::write(&script_path, script_content)` call in `install_claude_code_hook`.
This is the fail-closed case: if the hook script cannot be written, the PreToolUse security
hook cannot run for agent tool calls. `hook_name = "preToolUse"` (static string, no PII).
No file paths or error detail strings in event fields.

### Task 2: Tamper-evidence ADR (D-07)

`docs/adr/telemetry-tamper-evidence.md` (114 lines) records:

1. **Tamper boundary = Windows Event Forwarding** — external SIEM copy is outside a
   locally-compromised host's reach.
2. **In-session HMAC chain only** — ephemeral `OsRng` key, zeroized on drop, never persisted.
   Proves intra-session ordering/continuity. No cross-session continuity.
3. **What is NOT guaranteed**: (a) local admin log-clear via `wevtutil cl Application`;
   (b) local admin process substitution with a fresh-keyed nono binary; (c) cross-session
   chain continuity; (d) cryptographic-local anchoring (SEED-005 deferred).
4. **Consequences**: fleet deployments SHOULD configure WEF; documentation MUST use accurate
   language ("tamper-evident via WEF", not "tamper-proof").

## Label-Violation Section (EventID 10003)

**Decision: OPTION B — RESERVED-but-unemitted in Phase 84.**

After reading all exec_strategy_windows/ files, no reachable code path surfaces a mandatory-label
(IL) denial as a distinct event type at the exec_strategy layer. Windows mandatory-label denials
(`STATUS_ACCESS_DENIED` / `NonoError::LabelApplyFailed`) surface either:
- As `NonoError::LabelApplyFailed` during sandbox setup (propagated as `Err`, not via DenialRecord)
- As ordinary `DenialRecord` entries in `denials` (where the IL source is indistinguishable from
  a policy-blocked path-deny at the exec_strategy caller level)

Neither path produces a distinct `nono_security::label_violation` event with the current backend.
The `labels_guard.rs` errors abort the session via `?` before the post-child-exit handler runs,
so there are no DenialRecords to iterate at that point for IL-specific cases.

**Plan 84-04 gate update required**: `scripts/gates/telemetry-event-emit.ps1` (created in
Plan 84-04) MUST NOT assert EventID 10003 in its `Invoke-Gate` function. The EventID range
should cover 10001 (PathDeny), 10002 (NetworkDeny), 10004 (HookFailClosed), and 10005
(TelemetryDegraded). EventID 10003 (LabelViolation) remains defined in
`telemetry/event.rs` (`event_id_for(SecurityEventType::LabelViolation) = 10003`) but is not
emitted by any source in Phase 84.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written.

### Scope Deviations

**1. hook_fail_closed site selection**

The plan said to find "all fail-closed return paths" in hooks.rs. The codebase has many
`return Err(NonoError::HookInstall(...))` paths, but only one is semantically fail-closed from
a security perspective: the hook script write failure in `install_claude_code_hook`. If the
script cannot be written, the PreToolUse security hook cannot execute for subsequent tool calls.
All other `HookInstall` errors (directory create failure, settings.json write failure) are also
logged but the agent session continues — they are not "fail-closed" in the security sense.
Added the `tracing::warn!` at the script write failure only, which is the most accurate
representation of a security-relevant fail-closed event.

## Cross-Target Verification

**Status: PARTIAL** (Windows dev host cannot run Linux/macOS clippy)

- `cargo build -p nono-cli` — PASS on Windows host
- `cargo build -p nono-proxy` — PASS on Windows host
- `cargo clippy --workspace --target x86_64-unknown-linux-gnu` — DEFERRED to CI
  (no cross-toolchain on Windows host; per CLAUDE.md MUST/NEVER rule)

**cfg-gated code touched:**
- `exec_strategy.rs`: The new `tracing::warn!` block is NOT cfg-gated itself, but `exec_strategy.rs`
  contains `#[cfg(target_os = "windows")]` and `#[cfg(not(target_os = "linux"))]` branches in
  the surrounding supervisor loop code. Risk: LOW (the new block is unconditional at the
  post-child-exit site, which runs on all platforms).
- `hooks.rs`: Contains `#[cfg(unix)]` and `#[cfg(windows)]` platform dispatch in
  `create_symlink_platform`. The new `tracing::warn!` is in `install_claude_code_hook` which
  is unconditional code (no cfg gate). Risk: LOW.
- `nono-proxy/audit.rs`: No cfg gates. Risk: NONE.

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| EventID 10003 LabelViolation | `telemetry/event.rs` | Defined in schema but unemitted — IL denials not distinguishable at exec_strategy layer. See §Label-Violation above. |

## Threat Surface Scan

No new network endpoints, auth paths, or trust boundaries. Changes are purely additive
`tracing::warn!` calls at existing denial exit paths. The `path` field in path_deny is
forwarded to SecurityEventLayer for hashing — this is the intended D-08 data flow.
T-84-12 (path flowing raw to Application-log) is mitigated: the raw path value reaches
SecurityEventLayer::on_event (by design) but is hashed before constructing SecurityEvent;
it never reaches ReportEventW.

## Self-Check: PASSED

- `docs/adr/telemetry-tamper-evidence.md` — EXISTS, contains "Windows Event Forwarding" (3x), "SEED-005" (3x), multiple "NOT"/"does not"/"cannot" honesty statements
- `crates/nono-cli/src/exec_strategy.rs` — EXISTS, contains "nono_security::path_deny"
- `crates/nono-proxy/src/audit.rs` — EXISTS, contains "nono_security::network_deny" AND "nono_proxy::audit" (both present — additive invariant confirmed)
- `crates/nono-cli/src/hooks.rs` — EXISTS, contains "nono_security::hook_fail_closed"
- Commits `6ddbe784` and `dbb066f7` present in git log
- `cargo build -p nono-cli` — PASS
- `cargo build -p nono-proxy` — PASS
