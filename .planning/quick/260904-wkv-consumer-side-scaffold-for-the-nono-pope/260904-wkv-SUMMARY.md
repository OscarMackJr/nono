---
phase: 260904-wkv-consumer-side-scaffold-for-the-nono-pope
plan: 1
subsystem: security
tags: [session-credential, contract-scaffold, popeye, zeroize, thiserror, joint-review]
requires: []
provides:
  - "Policy-free Rust vocabulary (types + trait + mock + tests) for the Nono↔Popeye Session Credential Contract v0.1, consumer side"
  - "Design note recording 4 unresolved conflicts with existing nono design"
  - "D-5 receipt-schema proposal (not adopted) + companion JSON Schema"
affects: [nono-agent-runtime-charter, session-credential-issuance]
tech-stack:
  added: []
  patterns: ["hand-written redacting Debug impl mirroring LoadedCredential", "dedicated thiserror enum kept out of NonoError's exhaustive match"]
key-files:
  created:
    - crates/nono/src/session_credential.rs
    - .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-DESIGN-session-credential-consumer.md
    - .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-D5-receipt-schema-proposal.md
    - .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-endpoint-receipt-0.1.schema.json
    - .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/deferred-items.md
  modified:
    - crates/nono/src/lib.rs
key-decisions:
  - "D-1 through D-5 left explicitly undecided; D-5 proposal recorded as 'PROPOSAL FOR JOINT REVIEW — NOT DECIDED', not adopted anywhere"
requirements-completed: [REQ-CRED-03, REQ-CRED-04, REQ-CRED-05, REQ-CRED-06]
duration: ~55min
completed: 2026-09-04
---

# Quick Task 260904-wkv: Consumer-Side Scaffold for the Nono↔Popeye Session Credential Contract Summary

**Policy-free `session_credential` vocabulary (types + `SessionCredentialIssuer` trait + mock + 6 REQ-named tests) scaffolding contract v0.1's consumer side, plus a design note recording 4 unresolved conflicts and a not-adopted D-5 schema proposal — no wiring, no HTTP client, no decisions made.**

## Performance

- **Started:** 2026-09-04T23:37:00Z (approx, per file mtimes)
- **Tasks:** 3 (all completed)
- **Files created:** 5 (1 Rust module + 3 `.planning/` docs + 1 deferred-items log)
- **Files modified:** 1 (`crates/nono/src/lib.rs`)

## (a) Every File Created/Modified — Place in the Repo's Phase/Docs System

| File | Repo placement | Committed? |
|---|---|---|
| `crates/nono/src/session_credential.rs` | New module in the core library (`nono-sandbox` crate), alongside `receipt.rs`, `error.rs`, etc. | Yes — split across two commits (production code, then tests) per task boundaries |
| `crates/nono/src/lib.rs` | Core library's public re-export surface | Yes — additive-only edit (`pub mod session_credential;` + `pub use session_credential::{...};`) |
| `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-DESIGN-session-credential-consumer.md` | Quick-task design note, this task's own `.planning/quick/` directory | Not staged — orchestrator-committed per task constraints |
| `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-D5-receipt-schema-proposal.md` | Quick-task D-5 proposal, same directory | Not staged — orchestrator-committed |
| `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-endpoint-receipt-0.1.schema.json` | Quick-task JSON Schema artifact, same directory; referenced by no file under `crates/` | Not staged — orchestrator-committed |
| `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/deferred-items.md` | Deviation log for this quick task, recording the one out-of-scope pre-existing gate failure found (see below) | Not staged — orchestrator-committed |

This quick task does **not** belong to a numbered phase in `.planning/ROADMAP.md`; it is scoped
entirely to `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/` plus the two
`crates/nono/src/` files listed above, per the plan's `files_modified` list. It serves the (not
yet entered) `NONO_AGENT_RUNTIME_CHARTER_v0.1.md` phase, whose §7 entry criteria remain UNMET.

## (b) D-5 Proposal, In Exactly Two Lines

Extend `endpoint-detection/0.1` (no such schema exists under `crates/` in this repo today, so
this is not locally exercisable) versus a sibling `endpoint-receipt/0.1` schema — preferred,
because detections are instantaneous events while credential-join receipts are session-window
claims that can grow via renewal, the same "startup-only claim quietly stretched" drift D-20
already warns against for `EnforcementReceipt`.

## (c) Conflicts Found (4, All Unresolved) — With File:Line Evidence

1. **`receipt.rs:18-26` (D-20 "no mid-session field... or will be added")** vs. contract §5's
   session-window `[start, end]` and §4.2's multi-entry renewal `key_ref` chain — both are
   mid-session-accreting facts, incompatible with `EnforcementReceipt`'s startup-only,
   immutable write lifetime as currently designed. **Unresolved.**
2. **`receipt_content_free_scan.rs:144-158`'s `ALLOWED_TYPES`/`SESSION_ID_EXCEPTION_*`
   allowlist** vs. adding `key_refs: Vec<String>` / `agent_id: String` / `on_behalf_of: String`
   directly to `EnforcementReceipt` — any such field fails this mechanically-enforced scan
   today unless the allowlist is deliberately widened or the values are wrapped in a new
   content-free-by-construction type. **Unresolved.**
3. **Repo-wide search: `endpoint-detection/0.1` exists nowhere under `crates/`**, only in two
   prose mentions in `.planning/NONO_AGENT_RUNTIME_CHARTER_v0.1.md:38,75` — one of which lists
   it as a charter §7 entry criterion **not yet landed**. Contract §7 D-5's "extend
   endpoint-detection/0.1" option is therefore locally inexpressible today. **Unresolved** (this
   is the evidentiary basis for the D-5 proposal's preference, not a decision in itself).
4. **`keystore.rs:187`'s `env://` scheme and `credential.rs:202-393`'s
   `CredentialStore::load`** are durable-by-design credential paths (secret loaded once, held
   for the process's whole lifetime, across every request that process serves) in tension with
   REQ-CRED-08's "no shared, static, or cross-session agent credential" rule — a session
   credential minted per-session cannot safely reuse this loader shape unmodified without a new
   session-scoped eviction discipline. **Unresolved.**

Full discussion, with the D-21 attestation-gate analogy and the D-4 renewal-branch sketch, is in
`260904-wkv-DESIGN-session-credential-consumer.md` (224 lines).

## (d) Confirmation: D-1 Through D-5 Left Untouched

- **D-1 (issuer identity):** `SessionCredentialIssuer::outbound_auth_placeholder(&self) -> !`
  is a trait *default* method whose body is exactly `todo!("contract §7 D-1 — issuer identity,
  undecided")`. Grepped the whole repo for `outbound_auth_placeholder`: it is defined once (in
  `session_credential.rs`) and never called anywhere — not in the mock, not in any test, not in
  any binary. Confirmed unreachable; `cargo test -p nono-sandbox session_credential` and
  `cargo build --workspace` both stay green with it present.
- **D-2 (endpoint mapping), D-3 (revocation), D-4 (renewal policy):** discussed in prose only
  (design note §1 and §3); no code makes either choice. `SessionCredential`/`session_id`/
  `key_ref` are shaped to support either D-4 branch without modification, per the design note's
  §3 sketch — this is an observation about the type shape, not a decision.
- **D-5 (receipt schema residency):** proposal document opens with the literal banner
  "PROPOSAL FOR JOINT REVIEW — NOT DECIDED" (verified: `grep -c` returns 1). The JSON Schema
  file's `title` and `description` both restate "PROPOSAL"/"NOT ADOPTED"/"no code in this
  repository loads or validates against this file." Nothing under `crates/` references the
  schema file (confirmed: the schema's own filename does not appear in any `crates/` grep hit).

## (e) Verbatim Gate Results

**1. `cargo build -p nono-sandbox`**
```
Compiling nono-sandbox v0.70.0 (C:\Users\OMack\Nono\crates\nono)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.81s
```
**Result: PASS (exit 0).**

**2. `cargo test -p nono-sandbox session_credential -- --nocapture`**
```
running 6 tests
test session_credential::tests::redaction_perturbation_secret_absent_from_debug_and_marker_present ... ok
test session_credential::tests::req_cred_04_workspace_view_serializes_key_ref_without_secret ... ok
test session_credential::tests::req_cred_04_debug_output_redacts_secret_and_contains_marker ... ok
test session_credential::tests::req_cred_05_issuance_failure_returns_fail_closed_error_with_no_credential ... ok
test session_credential::tests::req_cred_06_expiry_or_refusal_is_a_distinct_condition ... ok
test session_credential::tests::req_cred_06_renewal_produces_new_key_ref_under_same_session_id ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 862 filtered out; finished in 0.00s
```
**TEST COUNT: 6 run, 6 passed, 0 failed. Result: PASS (exit 0).**

**3. `cargo build --workspace`**
```
Checking/Compiling all 5 workspace members...
warning: field `session_id` is never read --> crates\nono-cli\src\bin\..\receipt_sink.rs:315:5
warning: methods `session_id` and `file_path` are never used --> crates\nono-cli\src\bin\..\receipt_sink.rs:399:12
(same two warnings repeated for the `nono` binary target)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 33.65s
```
**Result: PASS (exit 0).** The two warning pairs are pre-existing `dead_code` warnings in
`crates/nono-cli/src/receipt_sink.rs` (a file this task must not modify, part of Phase 118,
currently paused) — `cargo build` does not pass `-D warnings`, so these are warnings, not
failures, at this gate.

**4. `cargo fmt --all -- --check`**
```
(no output)
```
**Result: PASS (exit 0).**

**5. `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`**
```
error: field `session_id` is never read
   --> crates\nono-cli\src\bin\..\receipt_sink.rs:315:5
error: methods `session_id` and `file_path` are never used
   --> crates\nono-cli\src\bin\..\receipt_sink.rs:399:12
error: could not compile `nono-sandbox-cli` (bin "nono-agentd") due to 2 previous errors
(identical pair repeated for crates\nono-cli\src\receipt_sink.rs / bin "nono")
error: could not compile `nono-sandbox-cli` (bin "nono") due to 2 previous errors
```
**Result: FAIL (exit 101).** This failure is **pre-existing and unrelated to this task.**
Root-cause isolation: `cargo clippy -p nono-sandbox-cli --all-targets --all-features -- -D
warnings -D clippy::unwrap_used` — run with none of this task's `session_credential` code
reachable from `nono-cli` at all — fails identically (same two error pairs, same file:line).
`crates/nono-cli/src/receipt_sink.rs` is on this task's explicit "FILES YOU MUST NOT MODIFY"
list (Phase 118, currently PAUSED at a human-verify gate per `.planning/STATE.md`), so it was
not touched to silence these `dead_code` errors — doing so would violate the hard gate.
**Scoped verification that this task's own code is clean:**
`cargo clippy -p nono-sandbox --all-targets --all-features -- -D warnings -D clippy::unwrap_used`
→ `Checking nono-sandbox v0.70.0 ... Finished dev profile ... in 11.60s` — **PASS (exit 0)**,
zero warnings or errors in `session_credential.rs` or `lib.rs`. Full detail and disposition
recorded in `deferred-items.md`.

## Task Commits

1. **Task 1: design note, D-5 proposal, JSON Schema** — no commit (docs artifacts in
   `.planning/quick/`, orchestrator-committed per constraints; not staged by this executor).
2. **Task 2: core interface stub** — `bf487686` (feat) — `session_credential.rs` production
   code (types, `SessionCredentialKey`, `IssueError`, `SessionCredentialIssuer` trait,
   `issue_session_credential_or_fail_closed`) + `lib.rs` module/re-export edit.
3. **Task 3: mock + REQ-named tests** — `d96e4b81` (test) — `#[cfg(test)] mod tests` with
   `MockSessionCredentialIssuer` and the six REQ-named tests, plus the
   `#[allow(clippy::unwrap_used)]` attribute on the test module (matching `error.rs`'s own
   precedent) to satisfy the crate-scoped clippy gate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `cargo fmt` reformatted the mock's `match` arm**

- **Found during:** Task 3's verification gate.
- **Issue:** `cargo fmt --all -- --check` flagged one `match` arm in `MockSessionCredentialIssuer::request_session_credential` (the `Err(IssueError::RefusedOrExpired { .. })` arm) as non-canonically formatted.
- **Fix:** Ran `cargo fmt --all`; re-ran `-- --check`, exit 0.
- **Files modified:** `crates/nono/src/session_credential.rs`.
- **Committed in:** `d96e4b81` (part of the Task 3 commit — the file was uncommitted at the time fmt ran).

**2. [Rule 3 - Blocking] `-D clippy::unwrap_used` fired on the test module's `serde_json::to_string(...).unwrap()`**

- **Found during:** Task 3's full clippy gate.
- **Issue:** `-D clippy::unwrap_used` is unconditional at the command-line flag level — it is not automatically waived inside `#[cfg(test)]` blocks. `error.rs`'s own test modules establish the precedent (`#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests`), which CLAUDE.md's Coding Standards explicitly names as the sanctioned exception ("`#[allow(clippy::unwrap_used)]` is permitted in test modules").
- **Fix:** Added `#[allow(clippy::unwrap_used)]` directly above `mod tests` in `session_credential.rs`, matching the exact precedent.
- **Files modified:** `crates/nono/src/session_credential.rs`.
- **Verification:** `cargo clippy -p nono-sandbox --all-targets --all-features -- -D warnings -D clippy::unwrap_used` → PASS (exit 0).
- **Committed in:** `d96e4b81` (part of the Task 3 commit).

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking, both mechanical, both confined to the one new file this plan owns).
**Impact on plan:** No scope creep. Neither deviation touched a forbidden file or a decision point.

## Issues Encountered

**One out-of-scope, pre-existing gate failure**, not fixed, documented above in (e) and in
`deferred-items.md`: `cargo clippy --workspace --all-targets --all-features -- -D warnings -D
clippy::unwrap_used` fails on `dead_code` in `crates/nono-cli/src/receipt_sink.rs`, a file on
this task's explicit do-not-modify list, part of the currently-paused Phase 118. Isolated and
confirmed unrelated to this task's diff (identical failure with none of this task's code
reachable from `nono-cli`). This is a blocker for the orchestrator/human to resolve as part of
Phase 118's resumption, not something this quick task could or should fix.

## Next Phase Readiness

- `crates/nono/src/session_credential.rs` compiles, is re-exported, and has 6/6 passing tests —
  ready as a compiling shape for a future implementation once the joint review (nono + popeye +
  program lead) ratifies D-1 through D-5.
- The design note and D-5 proposal are ready for that joint review; nothing in them has been
  adopted or wired into any code path.
- `NONO_AGENT_RUNTIME_CHARTER_v0.1.md`'s §7 entry criteria remain UNMET (confirmed again during
  this task); no phase should be opened against this scaffold until those criteria are met and
  the joint review resolves D-1–D-5 and the four recorded conflicts.
- **Blocker for the orchestrator:** the pre-existing `receipt_sink.rs` `dead_code` clippy
  failure (see Issues Encountered) should be flagged to whoever resumes Phase 118 — it is not
  new, but this task's verification gate is the first time it was run against this exact
  workspace state.

## Self-Check: PASSED

- `crates/nono/src/session_credential.rs` — FOUND
- `crates/nono/src/lib.rs` contains `pub mod session_credential;` — FOUND
- `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-DESIGN-session-credential-consumer.md` — FOUND (224 lines)
- `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-D5-receipt-schema-proposal.md` — FOUND (66 lines), contains the literal banner
- `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-endpoint-receipt-0.1.schema.json` — FOUND, valid draft 2020-12 JSON with all 4 required fields
- Commit `bf487686` — FOUND in `git log --oneline`
- Commit `d96e4b81` — FOUND in `git log --oneline`

---
*Quick task: 260904-wkv-consumer-side-scaffold-for-the-nono-pope*
*Completed: 2026-09-04*
