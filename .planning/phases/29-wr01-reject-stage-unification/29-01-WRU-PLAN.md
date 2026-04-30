---
phase: 29-wr01-reject-stage-unification
plan: 01
type: execute
wave: 1
depends_on: []
requirements:
  - WRU-01
  - WRU-02
tags:
  - wr01
  - reject-stage
  - audit
  - design-decision
  - docs
tdd: false
risk: low
files_modified:
  - crates/nono-cli/src/exec_strategy_windows/supervisor.rs   # WR-01 module docstring (line ~2181) + 5 wr01_* test docstrings (lines 4320-4722)
  - crates/nono-cli/src/audit_integrity.rs                    # RejectStage enum docstrings + one-line back-reference
  - .planning/PROJECT.md                                      # 5 line updates (Active section, key-decisions table, scope-lock context)
autonomous: true
threat_model_summary:
  - "No threat surface — documentation-only plan; zero behavioral / wire / API delta. WR-01 verdict matrix preserved verbatim by Option (c)."

must_haves:
  truths:
    - "PROJECT.md Key Decisions table row for `WR-01 reject-stage asymmetry` no longer reads `⚠️ Revisit v2.2`; updated to `✓ Good — locked as permanent design property at v2.3 Phase 29 (Option c). Mask-gate is O(1) profile lookup; broker-failure flip is O(syscall) post-approval. Asymmetry is structural, not unifiable.`"
    - "PROJECT.md Active section flips REQ-WRU-01..02 from `(unplanned)` to `✓ closed via Phase 29 Plan 29-01 (locked design property)`; verified by `grep -n 'WRU-01' .planning/PROJECT.md` showing no `(unplanned)` adjacency."
    - "WR-01 module docstring in `supervisor.rs` (line ~2181) gains a sentence calling the asymmetry a permanent design property and back-references Phase 29 by name. Verified by `grep -c 'design property\\|Phase 29' crates/nono-cli/src/exec_strategy_windows/supervisor.rs` returning at least 2."
    - "All 5 `wr01_*` test docstrings updated: `grep -B 5 'fn wr01_' crates/nono-cli/src/exec_strategy_windows/supervisor.rs | grep -c 'design property\\|locked at Phase 29\\|Phase 29'` returns at least 5 (one matching note per test)."
    - "`RejectStage` enum docstring in `audit_integrity.rs` adds a one-line back-reference to Phase 29 and the design-property framing — verified by `grep -c 'Phase 29' crates/nono-cli/src/audit_integrity.rs` returning at least 1."
    - "All 5 `wr01_*` regression tests still pass with their assertions UNCHANGED (REQ-WRU-02 acceptance #1 — chosen verdict matrix is the existing matrix). Verified by `cargo test -p nono-cli --bin nono wr01_` exit 0 with all 5 tests reported as passed."
    - "Phase 23 audit-show counter wording (`Capability Decisions: N (M before-prompt, K after-prompt rejections)`) NOT changed — Option (c) preserves the existing wire shape exactly. Verified by `git diff --stat HEAD~2 HEAD -- crates/nono-cli/src/audit_commands.rs` showing zero deltas to the counter rendering code."
    - "Production behavior is byte-identical: `git diff --stat HEAD~2 HEAD -- crates/nono/` returns empty (D-19 preservation); on Windows, `cargo test -p nono-cli --bin nono` exits 0 with the same pass/fail count as the pre-Phase-29 baseline."
  artifacts:
    - path: "crates/nono-cli/src/exec_strategy_windows/supervisor.rs"
      provides: "WR-01 module docstring + 5 wr01_* test docstrings reframed: 'deferred to v2.2 as product decision' wording replaced by 'locked as permanent design property at Phase 29 — see PROJECT.md key-decisions table'."
      contains: "design property"
      contains: "Phase 29"
    - path: "crates/nono-cli/src/audit_integrity.rs"
      provides: "RejectStage enum docstring + AuditEventPayload::CapabilityDecision::reject_stage field doc gain a one-line back-reference to Phase 29 framing the asymmetry as structural (O(1) profile lookup vs O(syscall) post-approval)."
      contains: "Phase 29"
      contains: "design property"
    - path: ".planning/PROJECT.md"
      provides: "5 line updates: REQ-WRU-01..02 flipped from `(unplanned)` to closed (lines 23 + 109); v2.2-deferred-items list updated (line 115); scope-lock context updated (line 153); Key Decisions table row flipped from `⚠️ Revisit v2.2` to `✓ Good — locked as permanent design property at v2.3 Phase 29 (Option c)` (line 175)."
      contains: "Phase 29"
      contains: "design property"
      contains: "Option c"
  key_links:
    - from: "crates/nono-cli/src/exec_strategy_windows/supervisor.rs § WR-01 module docstring"
      to: ".planning/PROJECT.md § Key Decisions table"
      via: "back-reference in updated docstring text"
      pattern: "PROJECT\\.md|Phase 29"
    - from: "crates/nono-cli/src/audit_integrity.rs § RejectStage enum"
      to: ".planning/PROJECT.md § Key Decisions (Phase 29 entry)"
      via: "one-line back-reference framing the asymmetry as structural"
      pattern: "Phase 29"
    - from: ".planning/PROJECT.md § Active (REQ-WRU-01..02)"
      to: ".planning/phases/29-wr01-reject-stage-unification/29-01-WRU-PLAN.md"
      via: "closure marker referencing Phase 29 Plan 29-01"
      pattern: "29-01|Phase 29 Plan"
---

<objective>
Lock WR-01 reject-stage asymmetry as a permanent design property of the Windows-AIPC enforcement model (REQ-WRU-01) and refresh planning artifacts + source-code docstrings to reflect that closure (REQ-WRU-02).

Purpose: Phases 18 + 18.1 + 23 shipped Windows-AIPC capability brokering with two structurally distinct reject-stage paths: (1) a pre-broker O(1) profile-lookup mask gate at `supervisor.rs:1891` that denies Event/Mutex/JobObject mask-out-of-allowlist requests **before** the approval prompt, and (2) a G-04 broker-failure flip at `supervisor.rs:1997` that denies Pipe direction / Socket privileged-port + role-allowlist requests **after** approval, surfacing as `Approved → Denied { reason: "broker failed: <inner>" }`. Phase 18.1 documented this asymmetry as "deferred to v2.2 as a product decision" and locked the matrix into the 5 `wr01_*` regression tests so any future refactor breaks CI. v2.2 closed without revisiting; v2.3 Phase 29 finalizes the verdict.

The locked decision (Option c — see CONTEXT.md): **the asymmetry is a structural reflection of what is checkable upfront (O(1) profile lookup) vs only via OS interaction (O(syscall) post-approval), not a bug to unify**. Mask-gate kinds (Event/Mutex/JobObject) reject `BeforePrompt` because the supervisor's profile fully describes the policy ahead of any kernel call; broker-failure-flip kinds (Pipe/Socket) reject `AfterPrompt` because the failure mode (anonymous-pipe direction; SO_REUSEADDR + privileged-port + role allowlist) is only observable when the broker actually attempts the kernel operation. Forcing pre-prompt rejection for Pipe/Socket would require re-implementing kernel-side checks in supervisor space — a violation of the broker-pattern's defense-in-depth premise.

This plan is **documentation-only**. Zero source-code semantics change. Zero wire-protocol bytes change. Zero `crates/nono/` change (D-19 byte-identity preserved). The 5 `wr01_*` regression tests retain their exact assertions; only their docstrings get reframed from "deferred to v2.2" to "locked at Phase 29 as permanent design property". The Phase 23 audit-show counter wording (`Capability Decisions: N (M before-prompt, K after-prompt rejections)`) is preserved verbatim because Option (c) ratifies, not reshapes, the existing wire shape.

Output:
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — WR-01 module docstring (~line 2181) reframed; 5 `wr01_*` test docstrings (lines 4320, 4418, 4516, 4619, 4722) gain a single-line "Locked at Phase 29 as permanent design property — see PROJECT.md" trailing note.
- `crates/nono-cli/src/audit_integrity.rs` — `RejectStage` enum docstring (lines 30–46) gains a one-line back-reference to Phase 29 with the structural-rationale framing; `AuditEventPayload::CapabilityDecision::reject_stage` field doc (lines 69–79) gains a parallel one-line note.
- `.planning/PROJECT.md` — 5 line updates per the locations enumerated in CONTEXT.md (Active section flips at lines 23 + 109; v2.2-deferred-items list update at line 115; scope-lock context update at line 153; Key Decisions table row flip at line 175).

**ACCEPTANCE LOCK:** No change to `wr01_*` test assertions. No change to `RejectStage` enum shape. No change to Phase 23 audit-show counter rendering. No change to `crates/nono/`. No new tests, no new wire fields, no new `HandleKind` discriminants. Anything beyond docstrings + planning-artifact text is **out of scope**.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/29-wr01-reject-stage-unification/29-CONTEXT.md

<interfaces>
<!--
  WR-01 verdict matrix (verbatim from supervisor.rs capability_handler_tests module docstring,
  Phase 18.1). Option (c) ratifies this matrix unchanged. Discriminants reference the AIPC
  HandleKind table (Phase 18 wire protocol):

    HandleKind 0 = File         (not in WR-01 scope — never gated by mask in current codebase)
    HandleKind 1 = Socket       → AfterPrompt   (broker-failure flip; SO_REUSEADDR + privileged-port + role)
    HandleKind 2 = Pipe         → AfterPrompt   (broker-failure flip; direction allowlist)
    HandleKind 3 = JobObject    → BeforePrompt  (mask gate; terminate / set-info / kill-on-job-close)
    HandleKind 4 = Event        → BeforePrompt  (mask gate; modify-state / synchronize)
    HandleKind 5 = Mutex        → BeforePrompt  (mask gate; modify-state / synchronize)

  Reject-stage rationale (the structural-asymmetry argument that locks Option c):

    BeforePrompt path: O(1) profile lookup. The supervisor has the full mask allowlist for
    Event/Mutex/JobObject in the profile (loaded at session start). Comparing the requested
    mask bits against the allowlist is a constant-time bitwise AND; no kernel call is needed
    to know whether the request is in-policy. Therefore the mask gate runs BEFORE the approval
    prompt — denying out-of-policy requests silently with no UX dialog.

    AfterPrompt path: O(syscall) post-approval. The Pipe direction (read-end vs write-end) is
    set by the kernel at pipe creation; the supervisor cannot know it without calling
    `GetNamedPipeInfo`. Socket privileged-port + role allowlist requires `bind()` to attempt
    the requested port — the OS rejects privileged ports for non-elevated processes at the
    kernel level. The G-04 broker-failure flip catches the resulting Err and converts it to
    Denied { reason: "broker failed: <inner>" }, after the user has already seen and approved
    the prompt. The user UX shows: prompt → approve → "broker failed".

  This asymmetry is what makes the matrix non-unifiable without either (a) re-implementing
  kernel checks in supervisor space (security regression — violates defense-in-depth), or
  (b) deferring all approval prompts until after broker attempts (UX regression — breaks the
  approval-then-action contract Phase 18 shipped). Option (c) accepts the asymmetry as the
  design property it actually is.
-->

# Phase 23 wire-shape contract preserved verbatim by Option (c):
#   AuditEventPayload::CapabilityDecision { entry, reject_stage: Option<RejectStage> }
#   RejectStage = BeforePrompt | AfterPrompt
#   Audit-show counter: "Capability Decisions: N (M before-prompt, K after-prompt rejections)"
#
# These are FROZEN — Phase 29 modifies only the docstring text around them, never the bytes.
</interfaces>

<source_audit_summary>
- REQ-WRU-01 (close the WR-01 reject-stage question with a verdict locked into PROJECT.md key-decisions and source docstrings) → Tasks 1, 2
- REQ-WRU-02 acceptance #1 (chosen verdict matrix matches the existing matrix; all 5 wr01_* regression tests pass with assertions unchanged) → Task 3 verification
- REQ-WRU-02 acceptance #2 (Phase 23 wire shape preserved; audit-show counter unchanged) → Task 3 verification (git diff stat on audit_commands.rs)
- D-19 byte-identity preservation (no `crates/nono/` deltas) → Task 3 verification (git diff stat on crates/nono/)
- Pre-resolved facts from CONTEXT.md (line numbers + file locations for all edits) → Tasks 1, 2 (consumed inline; no re-investigation needed)
</source_audit_summary>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Update PROJECT.md (5 line edits)</name>
  <files>.planning/PROJECT.md</files>
  <read_first>
    - `.planning/PROJECT.md` lines 20–30 (Active section header + REQ-WRU-01..02 row at line 23)
    - `.planning/PROJECT.md` lines 105–120 (REQ-WRU-01..02 currently `(unplanned)` at line 109; v2.2-deferred-items list at line 115)
    - `.planning/PROJECT.md` lines 150–160 (scope-lock context paragraph at line 153)
    - `.planning/PROJECT.md` lines 170–185 (Key Decisions table row at line 175)
  </read_first>
  <action>
Make exactly 5 line edits to `.planning/PROJECT.md` per the locations enumerated in CONTEXT.md. Each edit is targeted by the EXACT existing line content (verified by reading the surrounding 5–10 lines first); do NOT use line numbers as the sole match key — the file may have shifted by 1–2 lines since the CONTEXT.md snapshot was taken on 2026-04-30.

**Edit 1 — Active section (around line 23):**
Find the line beginning with `REQ-WRU-01..02` (or `REQ-WRU-01, REQ-WRU-02`) that mentions "Phase 29: WR-01 Reject-Stage Unification". Replace the trailing `(unplanned)` marker with:
```
✓ closed via Phase 29 Plan 29-01 (locked design property — Option c)
```

**Edit 2 — Active section (around line 109):**
Find the second occurrence of `REQ-WRU-01..02` (likely in a status-table block listing requirement IDs with `(unplanned)`). Replace `(unplanned)` with:
```
✓ closed (Phase 29 Plan 29-01 — locked design property)
```

**Edit 3 — v2.2-deferred-items list (around line 115):**
Find the bullet listing "WR-01 reject-stage unification" as one of "five major v2.2-deferred items" (or similar phrasing). Update to:
```
- **WR-01 reject-stage asymmetry** — closed at v2.3 Phase 29 (Option c: locked as permanent design property; mask-gate vs broker-failure-flip is O(1) profile lookup vs O(syscall) post-approval, not a unifiable bug).
```

If the surrounding context says "five major v2.2-deferred items", decrement the count to "four" (one fewer deferred item now that WR-01 is closed). Verify no other WR-01-adjacent counts exist in the same paragraph.

**Edit 4 — scope-lock context paragraph (around line 153):**
Find the sentence containing "...unification deferred to v2.2" (or "...stage unification deferred to v2.2 as a product decision"). Replace with:
```
...stage unification decided at v2.3 Phase 29: locked as permanent design property (Option c) — the mask-gate vs broker-failure-flip distinction is structural (O(1) profile lookup vs O(syscall) post-approval), not unifiable without security or UX regression.
```

**Edit 5 — Key Decisions table row (around line 175):**
Find the row whose decision-name column reads "WR-01 reject-stage" (or "WR-01 stage unification" or close variant) and whose status column currently reads `⚠️ Revisit v2.2 — stage unification requires product decision, not bug fix`. Replace the entire status cell with:
```
✓ Good — locked as permanent design property at v2.3 Phase 29 (Option c). Mask-gate is O(1) profile lookup; broker-failure flip is O(syscall) post-approval. Asymmetry is structural, not unifiable without security or UX regression.
```

**General style:**
- Use markdown table-cell escape rules consistent with the surrounding rows (pipe-escaped `\|` if any cell contains a literal `|`; the replacement text above contains none).
- Preserve existing leading whitespace / table column alignment where the file uses fixed-width columns.
- Single atomic commit at end of task: `docs(29-01): close WR-01 reject-stage in PROJECT.md (Option c — locked design property)`.
- Sign-off: `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>`.

**PROJECT.md grep ambiguity guard (Risk 2):** before each edit, run a unique-substring check:
```bash
grep -nE 'REQ-WRU-01.*\(unplanned\)' .planning/PROJECT.md
grep -nE 'WR-01 reject-stage|WR-01 stage unification' .planning/PROJECT.md
grep -nE 'unification deferred to v2.2' .planning/PROJECT.md
```
If any pattern returns more than the expected number of matches (1 for Edit 1, 1 for Edit 4, 1 for Edit 5; 1–2 for Edits 2–3), pause and disambiguate using the surrounding 3 lines as additional match anchor before proceeding.
  </action>
  <verify>
    <automated>! grep -nE 'REQ-WRU-01.*\(unplanned\)|WRU-01.*\(unplanned\)' .planning/PROJECT.md && ! grep -qE 'unification deferred to v2.2' .planning/PROJECT.md && ! grep -qE '⚠️ Revisit v2.2.*WR-01|WR-01.*⚠️ Revisit v2.2' .planning/PROJECT.md && grep -qE 'Phase 29' .planning/PROJECT.md && grep -qE 'design property' .planning/PROJECT.md && grep -qE 'Option c' .planning/PROJECT.md</automated>
  </verify>
  <done>5 PROJECT.md edits applied: REQ-WRU-01..02 no longer marked `(unplanned)` anywhere; "deferred to v2.2" stale text removed; Key Decisions table row no longer reads `⚠️ Revisit v2.2`; new text references "Phase 29", "design property", and "Option c". Single atomic commit landed; `git log -1 --oneline` shows `docs(29-01): close WR-01 reject-stage in PROJECT.md`.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Update source-code docstrings (supervisor.rs WR-01 module + 5 wr01_* tests + audit_integrity.rs RejectStage)</name>
  <files>
    crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    crates/nono-cli/src/audit_integrity.rs
  </files>
  <read_first>
    - `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` lines 2170–2200 — WR-01 module docstring (already read at plan-time; the "deferred to v2.2 as a product decision" sentence is on line 2179)
    - `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` lines 4310–4330 — `wr01_event_rejects_before_prompt_on_out_of_allowlist_mask` docstring (line 4320 fn signature; docstring is the 5–10 lines preceding)
    - `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` lines 4408–4428 — `wr01_mutex_rejects_before_prompt_on_out_of_allowlist_mask` docstring (line 4418 fn signature)
    - `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` lines 4506–4526 — `wr01_job_object_rejects_before_prompt_on_terminate_mask` docstring (line 4516 fn signature)
    - `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` lines 4609–4629 — `wr01_pipe_rejects_after_prompt_on_readwrite_default_profile` docstring (line 4619 fn signature)
    - `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` lines 4712–4732 — `wr01_socket_privileged_port_rejects_after_prompt_empirical` docstring (line 4722 fn signature)
    - `crates/nono-cli/src/audit_integrity.rs` lines 25–95 — `RejectStage` enum docstring + `AuditEventPayload::CapabilityDecision::reject_stage` field doc (already read at plan-time)
  </read_first>
  <action>
Apply the following docstring updates. **Do NOT modify any test assertions, function bodies, enum variants, or trait implementations.** This is text-only inside `///` and `//!` comment blocks (and `//` line comments where noted).

---

**Update 2.1 — WR-01 module docstring in `supervisor.rs` (around line 2181):**

The current text (lines 2176–2183) reads:
```rust
//! Per CONTEXT.md D-14 (Phase 18.1) + 18-HUMAN-UAT Test 2 result,
//! the WR-01 stage-unification fix (routing Pipe direction + Socket
//! role/port through the same pre-broker gate as
//! Event/Mutex/JobObject) is explicitly **deferred to v2.2 as a
//! product decision**. Phase 18.1 only verifies + documents the
//! current behavior; the `wr01_*` tests below lock the matrix above
//! into regression guards so any future refactor that accidentally
//! moves a mask check pre/post-broker breaks CI.
```

Replace with:
```rust
//! **Phase 29 (v2.3) — locked as permanent design property (Option c).**
//! The mask-gate-before-prompt vs broker-failure-flip-after-prompt
//! distinction is a structural reflection of what is checkable
//! upfront (O(1) profile lookup against the supervisor's mask
//! allowlist) vs only via OS interaction (O(syscall) post-approval —
//! Pipe direction requires `GetNamedPipeInfo`; Socket privileged-port
//! + role allowlist requires `bind()` to attempt the kernel op). It
//! is NOT a bug to unify. Forcing pre-prompt rejection for Pipe/Socket
//! would require re-implementing kernel checks in supervisor space
//! (security regression — violates defense-in-depth) or deferring all
//! approval prompts until after broker attempts (UX regression —
//! breaks the approval-then-action contract Phase 18 shipped).
//!
//! See `.planning/PROJECT.md § Key Decisions — WR-01 reject-stage
//! asymmetry` for the locked verdict and `.planning/phases/29-wr01-
//! reject-stage-unification/` for the closure record. The `wr01_*`
//! tests below remain regression guards on the locked matrix; any
//! future refactor that accidentally moves a mask check pre/post-
//! broker still breaks CI.
```

This replacement preserves the regression-guard sentence at the end (compatible with the existing tests' rationale) but removes the "deferred to v2.2 as a product decision" stale text.

---

**Update 2.2 — 5 `wr01_*` test docstrings in `supervisor.rs`:**

For each of the 5 test functions (lines 4320, 4418, 4516, 4619, 4722), append a single trailing line to the existing docstring. Do NOT modify any other line of the docstring or function body.

**Risk 1 mitigation:** before each append, read the existing docstring (the `///` block immediately preceding the `fn wr01_*` line). If the docstring already says "deferred" or "to be unified" or "v2.2", REPLACE that wording with the new note. If the docstring is purely descriptive (no deferral language), simply APPEND the new note as a final `///` line.

The trailing line to add (verbatim):
```rust
/// Locked at Phase 29 as permanent design property (Option c) — see PROJECT.md § Key Decisions.
```

Apply this to:
- `wr01_event_rejects_before_prompt_on_out_of_allowlist_mask` (line 4320 fn signature; docstring above it)
- `wr01_mutex_rejects_before_prompt_on_out_of_allowlist_mask` (line 4418)
- `wr01_job_object_rejects_before_prompt_on_terminate_mask` (line 4516)
- `wr01_pipe_rejects_after_prompt_on_readwrite_default_profile` (line 4619)
- `wr01_socket_privileged_port_rejects_after_prompt_empirical` (line 4722)

After all 5 appends, verify:
```bash
grep -B 5 'fn wr01_' crates/nono-cli/src/exec_strategy_windows/supervisor.rs | grep -c 'Phase 29'
```
must return **at least 5**.

---

**Update 2.3 — `RejectStage` enum docstring in `audit_integrity.rs` (lines 30–46):**

The current text (lines 42–46) ends with:
```rust
/// Currently observable for exactly two HandleKinds: Pipe and Socket.
/// Future kinds may extend this; until then the matrix is locked by the
/// WR-01 verdict matrix in `exec_strategy_windows/supervisor.rs`'s
/// `capability_handler_tests` module docstring (lines 2034-2076).
```

Append the following single-paragraph addition immediately before the `#[derive(...)]` line on line 47, keeping the existing text intact:
```rust
///
/// **Phase 29 (v2.3) — locked as permanent design property.** Stage
/// classification is structural: mask-gate kinds (Event/Mutex/JobObject)
/// reject `BeforePrompt` because the supervisor's profile fully describes
/// the mask allowlist (O(1) lookup); broker-failure-flip kinds (Pipe/
/// Socket) reject `AfterPrompt` because the failure mode is only
/// observable when the broker attempts the kernel op (O(syscall) post-
/// approval). Future kinds inherit this taxonomy: if their checkability
/// is upfront, they reject `BeforePrompt`; if only OS-observable, they
/// reject `AfterPrompt`. This is not unifiable without security or UX
/// regression — see `.planning/PROJECT.md § Key Decisions` and Phase 29.
```

---

**Update 2.4 — `AuditEventPayload::CapabilityDecision::reject_stage` field doc (lines 69–79):**

The existing field doc references `supervisor.rs:1891` (mask gate) and `supervisor.rs:1997` (G-04 broker-failure flip). Append a single trailing `///` line immediately before the `#[serde(default, ...)]` attribute on line 78:
```rust
/// Stage asymmetry is locked as permanent design property at Phase 29 — see RejectStage docstring.
```

---

**Single atomic commit for all four updates:**
```
docs(29-01): reframe WR-01 reject-stage asymmetry as permanent design property

- supervisor.rs: WR-01 module docstring + 5 wr01_* test docstrings
- audit_integrity.rs: RejectStage enum + reject_stage field doc

No behavior change; no wire-shape change; no test-assertion change.
Tests remain regression guards on the locked matrix (REQ-WRU-02 #1).

Closes Phase 29 Plan 29-01 source-code surface.

Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
```
  </action>
  <verify>
    <automated>cargo build -p nono-cli --bin nono 2>&1 | tail -3 | grep -qE 'Compiling|Finished' && [ "$(grep -c 'Phase 29\|design property' crates/nono-cli/src/exec_strategy_windows/supervisor.rs)" -ge 7 ] && [ "$(grep -B 5 'fn wr01_' crates/nono-cli/src/exec_strategy_windows/supervisor.rs | grep -c 'Phase 29')" -ge 5 ] && grep -q 'Phase 29' crates/nono-cli/src/audit_integrity.rs && grep -q 'design property' crates/nono-cli/src/audit_integrity.rs && ! grep -q 'deferred to v2.2 as a product decision' crates/nono-cli/src/exec_strategy_windows/supervisor.rs</automated>
  </verify>
  <done>WR-01 module docstring updated (no longer says "deferred to v2.2"); all 5 `wr01_*` test docstrings reference Phase 29 / design property; `RejectStage` enum docstring + `reject_stage` field doc reference Phase 29; cargo build of `nono-cli` succeeds (docstring-only edits); no test assertions modified (verified by `git diff` showing only `///` / `//!` line changes inside the affected files).</done>
</task>

<task type="auto" tdd="false">
  <name>Task 3: Verification gate (no commit)</name>
  <files>(read-only verification — no files modified)</files>
  <action>
Final acceptance gate. This task does NOT modify files; it runs verification commands and asserts every must-have truth holds. If any check fails, do NOT proceed to SUMMARY; surface the failure to the maintainer with the specific command that failed and the actual vs expected output.

**Verification commands (all must pass):**

1. **Cargo build clean:**
   ```bash
   cargo build --workspace 2>&1 | tail -5
   ```
   Expect: ends with `Finished` line; no warnings, no errors.

2. **All 5 wr01_* tests pass with assertions unchanged (REQ-WRU-02 #1):**
   ```bash
   cargo test -p nono-cli --bin nono wr01_ 2>&1 | tail -10
   ```
   Expect: `test result: ok. 5 passed; 0 failed; 0 ignored`. If any test fails, STOP — Option (c) preservation is broken; do not proceed.

3. **Clippy clean (project-strict):**
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used 2>&1 | tail -3
   ```
   Expect: `Finished` with zero clippy warnings.

4. **Format clean:**
   ```bash
   cargo fmt --all -- --check
   ```
   Expect: exit 0, no output.

5. **D-19 byte-identity preservation — no `crates/nono/` deltas:**
   ```bash
   git diff --stat HEAD~2 HEAD -- crates/nono/
   ```
   Expect: empty output. If non-empty, STOP — D-19 is broken; revert the offending hunks before proceeding.

6. **Phase 23 audit-show counter rendering preserved:**
   ```bash
   git diff --stat HEAD~2 HEAD -- crates/nono-cli/src/audit_commands.rs
   ```
   Expect: empty output (Option c preserves the existing wire shape verbatim). If `audit_commands.rs` shows any delta, STOP and revert.

7. **No `#[ignore]` regression in wr01_* tests (Risk 3 mitigation):**
   ```bash
   awk '/fn wr01_/,/^}/' crates/nono-cli/src/exec_strategy_windows/supervisor.rs | grep -c '#\[ignore'
   ```
   Expect: `0`. If non-zero, STOP — a wr01_* test got accidentally ignored.

8. **PROJECT.md (unplanned) markers cleared for WRU-01..02:**
   ```bash
   grep -nE 'WRU-0[12].*\(unplanned\)' .planning/PROJECT.md
   ```
   Expect: no matches (exit 1).

9. **PROJECT.md no longer carries the `⚠️ Revisit v2.2` marker for WR-01:**
   ```bash
   grep -nE '⚠️ Revisit v2.2.*WR-01|WR-01.*⚠️ Revisit v2.2' .planning/PROJECT.md
   ```
   Expect: no matches (exit 1).

10. **Source-code docstring grep gates (must_haves.truths #3–#5):**
    ```bash
    [ "$(grep -c 'design property\|Phase 29' crates/nono-cli/src/exec_strategy_windows/supervisor.rs)" -ge 2 ]
    [ "$(grep -B 5 'fn wr01_' crates/nono-cli/src/exec_strategy_windows/supervisor.rs | grep -c 'design property\|locked at Phase 29\|Phase 29')" -ge 5 ]
    [ "$(grep -c 'Phase 29' crates/nono-cli/src/audit_integrity.rs)" -ge 1 ]
    ```
    Each must hold.

11. **Stale "deferred to v2.2 as a product decision" text removed from supervisor.rs:**
    ```bash
    grep -q 'deferred to v2.2 as a product decision' crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    ```
    Expect: exit 1 (no match).

12. **Diff scope sanity — only the 3 expected files in the last 2 commits:**
    ```bash
    git diff --name-only HEAD~2 HEAD | sort -u
    ```
    Expect exactly:
    ```
    .planning/PROJECT.md
    crates/nono-cli/src/audit_integrity.rs
    crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    ```
    No other files. If any other file shows up, STOP and investigate.

If all 12 checks pass, the plan is acceptance-ready. Proceed to SUMMARY.

If any check fails, surface the specific failure (command + actual output) to the maintainer; do NOT attempt auto-remediation beyond the documented Risk-1/2/3 mitigations already in Tasks 1 and 2.
  </action>
  <verify>
    <automated>cargo build --workspace 2>&1 | tail -1 | grep -q 'Finished' && cargo test -p nono-cli --bin nono wr01_ 2>&1 | tail -3 | grep -qE '5 passed' && cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used 2>&1 | tail -1 | grep -q 'Finished' && cargo fmt --all -- --check && [ -z "$(git diff --stat HEAD~2 HEAD -- crates/nono/)" ] && [ -z "$(git diff --stat HEAD~2 HEAD -- crates/nono-cli/src/audit_commands.rs)" ] && ! grep -nE 'WRU-0[12].*\(unplanned\)' .planning/PROJECT.md && ! grep -q 'deferred to v2.2 as a product decision' crates/nono-cli/src/exec_strategy_windows/supervisor.rs && [ "$(awk '/fn wr01_/,/^}/' crates/nono-cli/src/exec_strategy_windows/supervisor.rs | grep -c '#\[ignore')" = "0" ]</automated>
  </verify>
  <done>All 12 verification commands pass: cargo build/test/clippy/fmt clean; 5 wr01_* tests pass; D-19 preservation (zero `crates/nono/` deltas); Phase 23 wire-shape preservation (zero `audit_commands.rs` deltas); no `#[ignore]` regression on wr01_* tests; PROJECT.md `(unplanned)` and `⚠️ Revisit v2.2` markers cleared for WR-01; supervisor.rs docstrings reference Phase 29 + design property at the expected counts; audit_integrity.rs references Phase 29; diff scope limited to exactly the 3 expected files.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

This plan is **documentation-only** and produces zero runtime behavior delta. There are no new trust boundaries introduced, no new attack surface, no new data flow. The WR-01 verdict matrix is preserved verbatim — Option (c) ratifies the existing structural design.

| Boundary | Description |
|----------|-------------|
| (none)   | Modifies only `///` / `//!` doc comments inside source files plus a planning-artifact markdown file. No code paths, IPC channels, parsed inputs, or capability surfaces change. |

## STRIDE Threat Register

| Threat ID  | Category               | Component                                       | Disposition | Mitigation Plan |
|------------|------------------------|-------------------------------------------------|-------------|-----------------|
| T-29-01-01 | Information Disclosure | `supervisor.rs` WR-01 module docstring (public) | accept | The docstring already documents the matrix in detail (Phase 18.1 shipped this text). The Phase 29 reframe adds rationale ("structural", "O(1) vs O(syscall)") that is also a public design statement, not an exploit detail. No PII, no secrets, no kernel-call sequences are revealed beyond what AIPC's open-source `aipc_sdk.rs` already exposes. Accepting. |
| T-29-01-02 | Tampering              | `.planning/PROJECT.md` Key Decisions table      | mitigate | The 5 line edits are surgical and grep-pinned; Task 1 includes a uniqueness-check guard before each edit (Risk 2). Task 3 verification asserts the diff scope is limited to the 3 expected files. Reviewer signs off via standard PR flow. |
| T-29-01-03 | Repudiation            | `wr01_*` test docstrings                        | accept | Test docstrings only describe expected behavior; tampering with them does not affect test assertions. Task 3 verification re-runs all 5 tests and confirms `5 passed; 0 failed; 0 ignored` — the regression guards still hold. Accepting. |

(Threat register intentionally minimal — documentation-only plan with zero runtime delta has nearly nothing to threat-model.)
</threat_model>

<risks>
## Top Risks

1. **Stale text variation in `wr01_*` test docstrings.** The 5 tests may not currently say "to be unified" or "deferred to v2.2"; the docstrings could be purely descriptive (e.g., "Verifies that an Event handle with mask 0xFF is rejected before the approval prompt fires."). The risk is the executor blindly searches for "deferred"/"to be unified" wording, finds none, and skips the edit — leaving the docstring without a Phase 29 closure marker. Mitigated by: Task 2 action explicitly handles both cases — REPLACE if deferral language exists, APPEND if not. Task 3 verification asserts `grep -B 5 'fn wr01_' ... | grep -c 'Phase 29'` returns at least 5 (one match per test), catching any skipped append.

2. **PROJECT.md grep ambiguity (multiple "WR-01" / "unification" references).** Multiple PROJECT.md lines may mention "WR-01" or "unification"; the risk is editing the wrong line and leaving a stale `⚠️ Revisit v2.2` marker untouched, OR over-editing a line that should remain (e.g., a historical context paragraph that intentionally describes the original v2.2 deferral). Mitigated by: Task 1 action specifies the EXACT existing line content to match for each of the 5 edits, plus a uniqueness-check `grep -nE` guard before each edit. Task 3 verification asserts `grep -nE 'WRU-0[12].*\(unplanned\)'` returns no matches AND `grep -nE '⚠️ Revisit v2.2.*WR-01'` returns no matches, catching both under-edit and stale-marker survival.

3. **`#[ignore]` count regression in wr01_* test region.** None of the 5 wr01_* tests should be `#[ignore]`'d in any phase before or after this plan. The risk is a stray `#[ignore]` attribute getting introduced (e.g., from a stale rebase artifact, an editor auto-format mishap, or an accidental copy-paste). Mitigated by: Task 3 verification gate item 7 — `awk '/fn wr01_/,/^}/' supervisor.rs | grep -c '#\[ignore'` returns 0. If non-zero, STOP — a regression guard got disabled and the locked matrix is unprotected.

(Out of scope for this risk register: any change to `crates/nono/` byte-identity, any change to Phase 23 wire shape — these are explicitly verified as zero-delta in Task 3 items 5 and 6, and any non-zero delta is a hard STOP.)
</risks>

<verification>
## Phase-level Verification Gates

1. **Cargo build clean:** `cargo build --workspace` ends with `Finished`; zero warnings.
2. **All 5 `wr01_*` regression tests pass with assertions unchanged:** `cargo test -p nono-cli --bin nono wr01_` reports `5 passed; 0 failed; 0 ignored` (REQ-WRU-02 acceptance #1).
3. **Clippy clean (project-strict):** `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` exits 0.
4. **Format clean:** `cargo fmt --all -- --check` exits 0.
5. **D-19 byte-identity preserved:** `git diff --stat HEAD~2 HEAD -- crates/nono/` returns empty.
6. **Phase 23 wire-shape preserved:** `git diff --stat HEAD~2 HEAD -- crates/nono-cli/src/audit_commands.rs` returns empty (Option c preserves the existing audit-show counter wording verbatim — REQ-WRU-02 acceptance #2).
7. **No `#[ignore]` regression on `wr01_*` tests:** zero matches inside the wr01_* test region.
8. **PROJECT.md `(unplanned)` markers cleared for WRU-01..02:** `grep -nE 'WRU-0[12].*\(unplanned\)' .planning/PROJECT.md` returns no matches.
9. **PROJECT.md `⚠️ Revisit v2.2` marker cleared for WR-01:** no matches.
10. **Stale "deferred to v2.2 as a product decision" text removed from supervisor.rs:** no matches.
11. **Source-code docstring grep gates:** supervisor.rs has ≥2 occurrences of `Phase 29` / `design property`; the 5 `wr01_*` test docstrings collectively yield ≥5 `Phase 29` matches; audit_integrity.rs has ≥1 `Phase 29` match.
12. **Diff scope sanity:** `git diff --name-only HEAD~2 HEAD` returns exactly the 3 expected files (`PROJECT.md` + `supervisor.rs` + `audit_integrity.rs`); no other files.
</verification>

<success_criteria>
- [ ] PROJECT.md Key Decisions table row for WR-01 reads `✓ Good — locked as permanent design property at v2.3 Phase 29 (Option c)` with the structural-rationale paragraph.
- [ ] PROJECT.md Active section flips REQ-WRU-01..02 from `(unplanned)` to `✓ closed via Phase 29 Plan 29-01 (locked design property)`.
- [ ] PROJECT.md v2.2-deferred-items list and scope-lock context updated to reflect Phase 29 closure (no stale "deferred to v2.2" text).
- [ ] WR-01 module docstring in `supervisor.rs:~2181` reframes the asymmetry as a permanent design property and back-references Phase 29.
- [ ] All 5 `wr01_*` test docstrings carry a Phase 29 closure note; assertions UNCHANGED.
- [ ] `RejectStage` enum docstring + `reject_stage` field doc in `audit_integrity.rs` reference Phase 29 with the structural-rationale framing.
- [ ] `cargo test -p nono-cli --bin nono wr01_` exits 0 with 5 passed (REQ-WRU-02 #1).
- [ ] `git diff --stat HEAD~2 HEAD -- crates/nono/` returns empty (D-19 byte-identity).
- [ ] `git diff --stat HEAD~2 HEAD -- crates/nono-cli/src/audit_commands.rs` returns empty (Phase 23 wire-shape preservation, REQ-WRU-02 #2).
- [ ] `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`, and `cargo fmt --all -- --check` all clean.
- [ ] Diff scope of last 2 commits limited to exactly: `.planning/PROJECT.md`, `crates/nono-cli/src/audit_integrity.rs`, `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`.
</success_criteria>

<out_of_scope>
Explicit deferrals (do NOT include in this plan; do NOT let scope creep pull these in):

- **Any change to `wr01_*` test ASSERTIONS.** Option (c) keeps the existing matrix; only docstrings change. Modifying `assert_eq!` / `assert!` / mock-backend call counts is OUT — that would invalidate REQ-WRU-02 acceptance #1.
- **Any change to `RejectStage` enum SHAPE.** Phase 23 D-02 contract preserved verbatim; no new variants, no `#[serde(...)]` rename, no `#[repr(...)]` adjustment.
- **Any change to Phase 23 audit-show counter rendering.** `audit_commands.rs` must show zero deltas; Option (c) preserves the existing wire shape exactly. The counter wording `Capability Decisions: N (M before-prompt, K after-prompt rejections)` is FROZEN — REQ-WRU-02 acceptance #2.
- **Any change to `crates/nono/`.** D-19 byte-identity preserved; library bytes unchanged. Any `crates/nono/` delta is a hard STOP.
- **macOS / Linux equivalents.** WR-01 is Windows-AIPC-only by Phase 23 D-21; non-Windows behavior is unchanged. Cross-platform AIPC futures are documented in `docs/architecture/aipc-unix-futures.md` (Phase 25 Plan 25-02 ADR), not here.
- **STATE.md / ROADMAP.md updates.** STATE.md frontmatter + Session Continuity references and the ROADMAP progress-table flip from "Not started" to "Complete (option c locked)" are bookkeeping handled by the post-plan SUMMARY commit + milestone-close workflow, NOT by this plan. (If the executor finds the ROADMAP row is already correctly marked because scope-lock pre-flipped it, that is fine — leave alone.)
- **New tests.** No new test files, no new test functions; the existing 5 `wr01_*` tests are sufficient regression guards on the locked matrix.
- **Re-litigation of the decision.** If the AIPC subsystem grows new HandleKinds with different stage characteristics in v2.4+, a new ADR re-opens the question. Until then, Option (c) is permanent. This plan does not pre-design that future ADR.
- **Documentation diagrams, sequence diagrams, architecture diagrams.** Plain prose docstrings only; no ASCII art, no Mermaid, no embedded SVG.
</out_of_scope>

<output>
After completion, create `.planning/phases/29-wr01-reject-stage-unification/29-01-SUMMARY.md` documenting:
- The 5 PROJECT.md line edits (file path, before/after excerpt, verification grep output for each)
- The 4 source-code docstring updates (file path + line range, before/after excerpt for the WR-01 module docstring + RejectStage docstring; per-test note text for the 5 wr01_* test docstring appends)
- Verification command outputs (all 12 from Task 3)
- `git diff --stat HEAD~2 HEAD` output proving the diff scope is limited to the 3 expected files
- `cargo test -p nono-cli --bin nono wr01_` output proving 5/5 tests pass with assertions unchanged
- `git diff --stat HEAD~2 HEAD -- crates/nono/` and `... -- crates/nono-cli/src/audit_commands.rs` outputs proving D-19 + Phase 23 wire-shape preservation
- Closure statement: REQ-WRU-01 satisfied (verdict locked), REQ-WRU-02 satisfied (acceptance #1: 5 wr01_* tests pass with unchanged assertions; acceptance #2: Phase 23 wire shape preserved)
- Any deviations from the locked verdict text in PROJECT.md or the docstring closure-note wording (should be none — flag loudly if any)
</output>
