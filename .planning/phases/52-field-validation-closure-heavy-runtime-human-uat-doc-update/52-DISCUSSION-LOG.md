# Phase 52: Field validation closure — heavy-runtime HUMAN-UAT + doc update - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-26
**Phase:** 52-Field validation closure — heavy-runtime HUMAN-UAT + doc update
**Areas discussed:** Repro recipe & pass definition, Negative-result contingency, Doc claims & security envelope, UAT evidence capture

---

## Repro recipe & "pass" definition

**Q1 — How should the HUMAN-UAT construct the matrix commands, and what counts as "repro A passes"?**

| Option | Description | Selected |
|--------|-------------|----------|
| Proven recipe, true exit-0 for both | Doc's working shapes from %USERPROFILE%/%TEMP% cwd, claude-code profile; pass = exit 0 + expected stdout + no dialog for BOTH A and B; avoids the C:\-root --allow . gate | ✓ |
| Literal debug cmds; refusal OK for A | Run exact debug strings; accept fail-secure launch-path refusal as "pass" for A | |
| You decide / researcher determines | Leave exact cwd+grant recipe to planner | |

**User's choice:** Proven recipe, true exit-0 for both.
**Notes:** Under the claude-code profile both cmd.exe and claude.exe already clear the Phase 27 launch-path gate — that is why repro B reaches DLL init and fails at 0xC0000142 rather than being refused.

**Q2 — Should the UAT pin/record which claude.exe it validates against?**

| Option | Description | Selected |
|--------|-------------|----------|
| Pin to the heavy self-contained shape | Record claude --version + confirm 2026-05-24+ self-contained (~234 MB) build; validating the heavy shape is what proves the fix | ✓ |
| Whatever's on PATH; just record version | Run whatever's installed; record version but don't gate on shape | |
| You decide | Let planner specify | |

**User's choice:** Pin to the heavy self-contained shape.
**Notes:** An older/thin claude.exe wouldn't trigger the original failure → false positive.

---

## Negative-result contingency

**Q1 — If repro B still fails 0xC0000142 (or fails to exit 0), what does Phase 52 do?**

| Option | Description | Selected |
|--------|-------------|----------|
| Hard stop + diagnose, milestone blocked | FAIL verdict + full evidence; VERIFICATION non-pass; do not ship support claim; open /gsd:debug; milestone stays open until B passes | ✓ |
| Document as known limitation, close gap | Record FAIL, defer REQ-WSRH-04, doc says "attempted/not-yet-validated", allow milestone close | |
| Checkpoint to me before deciding | Pause and surface evidence for a go/no-go call | |

**User's choice:** Hard stop + diagnose, milestone blocked.
**Notes:** Corollary captured in CONTEXT — the doc support claim is gated on repro B passing; doc-update task sequences after the UAT verdict.

**Q2 — Exact pass boundary for repro B; does ANY deviation trigger the hard-stop?**

| Option | Description | Selected |
|--------|-------------|----------|
| Strict: all three, any deviation stops | Pass = no 0xC0000142 dialog AND version printed AND exit 0; any deviation → FAIL + hard-stop+diagnose | ✓ |
| DllMain-focused, triage unrelated exits | Load-bearing signal = no dialog + version; non-zero exit for unrelated reason → PASS-with-note, triage separately | |
| You decide | Let planner define VERIFICATION close criteria | |

**User's choice:** Strict: all three, any deviation stops.

---

## Doc claims & security envelope

*Factual basis established before asking:* `crates/nono-cli/src/exec_strategy_windows/launch.rs:1205,1214-1215` confirms the `BrokerLaunchNoPty` path waives per-session WFP differentiation (`FWPM_CONDITION_ALE_USER_ID`) and falls back to AppID filtering — so the doc's line-594 claim is now false on two counts for claude-code.

**Q1 — How accurately should the doc reflect claude-code's changed nono run security envelope?**

| Option | Description | Selected |
|--------|-------------|----------|
| Profile-conditional + explicit tradeoff | Document Low-IL broker token + NO_WRITE_UP (not WRITE_RESTRICTED) AND per-session WFP waived → AppID fallback for claude-code; other profiles unchanged | ✓ |
| Write-deny only, gloss WFP nuance | Note Low-IL broker + write-deny preserved; omit per-session-WFP-waiver detail | |
| You decide | Let planner set depth | |

**User's choice:** Profile-conditional + explicit tradeoff.

**Q2 — How wide should the doc edit go?**

| Option | Description | Selected |
|--------|-------------|----------|
| Full consistency sweep | Update line 594 (primary), line 573 (shell cross-ref), line 544 (claude --version framing), smoke-checklist table + add heavy-runtime subsection | ✓ |
| Focused edit only | Add heavy-runtime note + fix line 594 only; leave secondary mentions | |
| You decide | Let planner determine cross-references | |

**User's choice:** Full consistency sweep.
**Notes:** The `## Known limitation: nono run cannot host TUI agents` section (line 542) is ConPTY/TUI-specific, orthogonal to the heavy-runtime non-TUI fix — out of scope for the edit (cross-reference only; do not weaken).

---

## UAT evidence capture

**Q1 — What evidence should the 52-HUMAN-UAT artifact record per repro?**

| Option | Description | Selected |
|--------|-------------|----------|
| Rich audit trail | Per repro: command, full stdout, exit code, no-dialog confirmation + binary identity (~234 MB self-contained) + host Windows build number, all timestamped | ✓ |
| Minimal verdict | Pass/fail + exit code + timestamp only | |
| You decide | Let planner define schema | |

**User's choice:** Rich audit trail.

**Q2 — Where do the validation results live?**

| Option | Description | Selected |
|--------|-------------|----------|
| UAT is record; doc gets a dated note | UAT + VERIFICATION = system-of-record; doc makes support claim (gated on B pass) + short "validated <date>, build <N>" line per existing Phase 31 style | ✓ |
| UAT only; doc claim, no dated note | Raw evidence in UAT only; doc claim with no validation date/build line | |
| You decide | Let planner decide | |

**User's choice:** UAT is record; doc gets a dated note.

---

## Claude's Discretion

- Exact wording / section placement of the new heavy-runtime subsection in `windows-poc-handoff.mdx`.
- Exact `52-HUMAN-UAT.md` schema/layout within the D-52-07 evidence list.
- Whether the D-52-03 failure-path diagnostic capture uses ProcMon, Event Viewer, or both.

## Deferred Ideas

- CLI override flag (`--low-il-broker` / `--no-low-il-broker`) + profile-wide heavy-runtime audit — REQ-WSRH-AUDIT-01, deferred to a v2-follow-on.

## Reviewed Todos (not folded)

- `44-class-d-validator-preflight-investigation.md` — generic-keyword match only (Linux validator preflight); orthogonal. Not folded.
- `44-validate-restore-target-fd-relative-hardening.md` — generic-keyword match only (restore-target hardening); orthogonal. Not folded.
