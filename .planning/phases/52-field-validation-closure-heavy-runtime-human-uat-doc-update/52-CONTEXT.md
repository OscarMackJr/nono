# Phase 52: Field validation closure — heavy-runtime HUMAN-UAT + doc update - Context

**Gathered:** 2026-05-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Field-validation + documentation **closure** for the v2.7 milestone. Phase 51 already shipped the implementation (no-PTY `BrokerLaunchNoPty` arm, `windows_low_il_broker` profile field on `claude-code`, no-PTY broker mode, write-deny regression test, CI + cross-target clippy green). Phase 52 has **no new product code to design**.

In scope (REQ-WSRH-04 / REQ-WSRH-06):
- Execute the two-command reproduction matrix on a **real Windows 11 host** and record timestamped verdicts in `52-HUMAN-UAT.md`:
  - **Repro A** (`nono run --profile claude-code -- cmd /c "echo hi"`) — plain console app, no regression.
  - **Repro B** (`nono run --profile claude-code -- claude --version`) — heavy self-contained `claude.exe`, no `0xC0000142`, version printed, exit 0.
- Close `52-VERIFICATION.md` with `status: pass` (closing the Phase 51 SC-4 positive-spawn deferral — see note below).
- Update `docs/cli/development/windows-poc-handoff.mdx` to reflect the supported heavy-runtime `nono run` behavior **for the `claude-code` profile** and its changed security envelope.

Out of scope (explicit):
- Any new code / cascade / profile changes (those were Phase 51).
- CLI override flag and profile-wide heavy-runtime audit (REQ-WSRH-AUDIT-01, deferred to v2-follow-on).
- Option 2 null-token fallback (explicitly rejected at milestone open).

**Sequencing note (load-bearing):** the doc's "heavy-runtime supported" claim is **gated on repro B actually passing** (D-52-03). The doc-update task runs *after* the UAT verdict, not in parallel. If B fails, no support claim is made.

**Phase 51 SC-4 linkage:** `51-HUMAN-UAT.md` deferred the "positive supervised-spawn exit-0 on a real Windows host" test (ROADMAP SC-4) to this phase. Phase 52's **repro B is that test.** The planner should reference and close that deferral here.

</domain>

<decisions>
## Implementation Decisions

### Reproduction recipe & "pass" definition
- **D-52-01:** The HUMAN-UAT prescribes the doc's **already-working recipe shapes**, run from a `%USERPROFILE%` (or `%TEMP%`) working directory under the `claude-code` profile, so **both A and B genuinely spawn and exit 0**. "Pass" = exit 0 + expected stdout + no `0xC0000142` dialog. This deliberately avoids the bare `C:\`-root + `--allow .` shape that tripped the Phase 27 launch-path policy gate in the Phase 51 dev-host note. (Under the `claude-code` profile both `cmd.exe` and `claude.exe` already clear the launch-path gate — that is *why* repro B reaches DLL init and fails at `0xC0000142` rather than being refused.)
  - Canonical command shapes:
    - `cd $env:USERPROFILE`
    - `nono run --profile claude-code -- cmd /c "echo hi"`  (repro A)
    - `nono run --profile claude-code -- claude --version`   (repro B)
- **D-52-02:** **Pin/record the heavy binary shape.** The UAT records `claude --version` output AND confirms the validated binary is the 2026-05-24-or-newer **self-contained (~234 MB single-exe) build**. Validating against the heavy shape is what actually proves the fix — an older/thin `claude.exe` would not trigger the original `0xC0000142` and would yield a false positive.

### Negative-result contingency & pass boundary
- **D-52-03:** **Hard stop + diagnose; milestone stays blocked.** If repro B fails, the phase records a FAIL verdict with full evidence, keeps VERIFICATION non-pass, does **not** ship the "heavy-runtime supported" doc claim, and opens `/gsd:debug` to investigate (Phase 51 fix insufficient). Consistent with the project fail-secure ethos — the milestone exists to make B pass. (Chosen over "document as known limitation" and over "checkpoint to user before deciding".)
- **D-52-04:** **Strict pass boundary.** Repro B passes only if **all three** hold: no `0xC0000142` dialog AND version string printed AND exit 0. Any deviation (including a non-zero exit for any reason) records FAIL and triggers the D-52-03 hard-stop + diagnose; the debug step then triages whether it is the same DllMain class or something new. (Chosen over the looser "DllMain-focused, triage unrelated exits" boundary.)

### Doc claims & security envelope
- **D-52-05:** **Profile-conditional + explicit tradeoff.** The doc documents that `nono run --profile claude-code` now routes through the **Low-IL broker**: write-deny is preserved via the `NO_WRITE_UP` mandatory label (**not** `WRITE_RESTRICTED`), AND **per-session WFP differentiation is waived** (falls back to AppID-based filtering — same characteristic the doc already describes for the `nono shell` broker path at line 573). Other profiles (codex / opencode / openclaw / swival) keep `WRITE_RESTRICTED` + per-session WFP. This is security-accurate and avoids overstating claude-code network isolation. **Verified factual basis:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1205,1214-1215` — the `BrokerLaunchNoPty` arm explicitly waives `FWPM_CONDITION_ALE_USER_ID` per-session filtering.
- **D-52-06:** **Full consistency sweep** of the doc's `nono run` / `WRITE_RESTRICTED` / `claude --version` claims so they are mutually consistent and profile-aware. Concretely touch: **line 594** (primary "non-TUI commands → WRITE_RESTRICTED + per-session SID + full WFP differentiation" claim), **line 573** (shell-path cross-ref that says "use `nono run` … retains WRITE_RESTRICTED + per-session SID"), **line 544** ("`claude --version` works fine through pipes" — pre-regression framing), and the **smoke-checklist table (lines ~615-616)** — plus add a focused **heavy-runtime children** subsection. (Chosen over a focused single-claim edit that would leave the doc self-contradictory.)
  - **Out of scope for the sweep:** the `## Known limitation: nono run cannot host TUI agents on Windows` section (line 542) is about ConPTY/TUI rendering — orthogonal to the heavy-runtime non-TUI fix. `claude --version` is non-TUI. Do not weaken or remove the TUI-limitation statement; a cross-reference is fine.

### UAT evidence & results location
- **D-52-07:** **Rich audit trail.** Per repro the UAT records: exact command, full stdout, exit code, explicit "no `0xC0000142` dialog" confirmation — plus binary identity (`claude --version` output + ~234 MB self-contained confirmation per D-52-02) and host Windows build number — all timestamped. If B fails, the D-52-03 hard-stop path already has its diagnostic context captured.
- **D-52-08:** **`52-HUMAN-UAT.md` + `52-VERIFICATION.md` are the system-of-record** for raw evidence. The doc makes the support claim (gated on B pass per D-52-03) AND carries a short **"validated `<date>`, build `<N>`"** line — consistent with the existing "Phase 31, validated on the user's Windows test box on 2026-05-09" style already in the doc. Keeps the doc clean but dated/traceable.

### Claude's Discretion
- Exact wording and section placement of the new heavy-runtime subsection within `windows-poc-handoff.mdx`, provided the D-52-05/06 accuracy + consistency bar is met.
- Exact `52-HUMAN-UAT.md` schema/layout within the D-52-07 evidence list and SC-3's timestamp requirement.
- Whether the diagnostic capture in D-52-03 uses ProcMon, Windows Event Viewer, or both — only relevant on the failure path.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements, roadmap & root cause
- `.planning/ROADMAP.md` § Phase 52 — Goal, the 4 success criteria (SC-1 repro B exit-0 + version; SC-2 repro A no-regression; SC-3 timestamped verdicts + VERIFICATION `status: pass`; SC-4 doc update) + § Cross-Phase Invariants (NO_WRITE_UP non-regression, fail-closed on error, Windows-only-files intentional).
- `.planning/REQUIREMENTS.md` — REQ-WSRH-04 (repro B passes) + REQ-WSRH-06 (HUMAN-UAT matrix + doc update) acceptance criteria + traceability table.
- `.planning/debug/claude-exe-dll-init-failed.md` — confirmed root cause (WriteRestricted restricting-SID double-gate vs heavy-runtime DllMain WRITE activity) + the falsification evidence (whoami.exe survives, claude.exe fails) the D-52-03 diagnose path would re-use.

### Phase 51 implementation (what is being validated)
- `.planning/phases/51-no-pty-low-il-broker-token-routing-write-deny-preservation/51-CONTEXT.md` — D-01..D-08 implementation decisions; D-03 (only `claude-code` opts in via `windows_low_il_broker`).
- `.planning/phases/51-no-pty-low-il-broker-token-routing-write-deny-preservation/51-HUMAN-UAT.md` — the deferred ROADMAP SC-4 positive-spawn test (closed by this phase's repro B) + the Phase 27 launch-path-gate note that motivates D-52-01.
- `.planning/phases/51-no-pty-low-il-broker-token-routing-write-deny-preservation/51-VERIFICATION.md` — Phase 51 close state (human_needed 9/10; SC-4 deferred here).
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `select_windows_token_arm` cascade + `WindowsTokenArm::BrokerLaunchNoPty` (arm at ~1113, cascade rule 3 at ~1124, spawn wiring at ~1631); **lines 1205, 1214-1215** = the per-session-WFP waiver that grounds D-52-05.

### Documentation target
- `docs/cli/development/windows-poc-handoff.mdx` — the doc to update (643 lines). Specific anchors for the D-52-06 sweep: line 594 (primary `nono run` envelope claim), line 573 (shell-path `nono run` cross-ref), line 544 (`claude --version works fine through pipes`), lines ~430-448 (working-directory choice + the Phase 27 launch-path gate that D-52-01 navigates), lines ~615-616 (smoke-checklist table). Line 542 (`## Known limitation: nono run cannot host TUI agents`) is **out of scope** for this edit.

### Reference pattern (security envelope language to mirror)
- `.planning/phases/31-broker-process-architecture-shell-01/31-05-SUMMARY.md` — Phase 31 broker production validation; the existing doc's "validated `<date>`" style (D-52-08) and the shell-path per-session-WFP waiver wording (line 573) that D-52-05 parallels.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Phase 51 `BrokerLaunchNoPty` arm + `windows_low_il_broker` profile field** — already shipped and CI-green; this phase only exercises and documents it, no edits.
- **Doc's existing working-directory + smoke recipes** (`windows-poc-handoff.mdx` §§ "Working directory choice", "Smoke checklist") — D-52-01 reuses these shapes directly rather than the bare debug-session strings.

### Established Patterns
- **`nono shell` broker-path security-envelope wording** (doc line 573 + §"Windows nono shell — security envelope") — the template for D-52-05's profile-conditional, per-session-WFP-waiver description. Mirror its phrasing for the `nono run --profile claude-code` case.
- **"validated `<date>`" doc note style** (Phase 31) — the template for D-52-08's dated validation line.

### Integration Points
- VERIFICATION close logic ← D-52-04 strict pass boundary + D-52-03 hard-stop policy.
- Doc support claim ← gated on repro B verdict (D-52-03 sequencing).

### Footgun for the executor
- **`docs/cli/development/` is gitignored-but-tracked** (per project memory `feedback_docs_cli_dev_gitignored`): editing the `.mdx` requires `git add -f` — a plain `git add` exits 1 and silently breaks `&& git commit` chains. The plan's doc-commit step must use `git add -f docs/cli/development/windows-poc-handoff.mdx`.

</code_context>

<specifics>
## Specific Ideas

- The confirmed bug command (validated fixed here): `nono run --profile claude-code -- claude --version` → previously `0xC0000142`.
- The confirmed non-regression command: `nono run --profile claude-code -- cmd /c "echo hi"`.
- The differentiator that makes B a valid test is heavy DllMain WRITE-type activity in the self-contained `claude.exe` (named-object create on `\BaseNamedObjects`, temp DLL extraction) — hence D-52-02's pin to the ~234 MB shape.
- The doc must NOT, after this edit, claim `nono run` is universally `WRITE_RESTRICTED` + full per-session WFP (it is profile-dependent post-Phase-51).

</specifics>

<deferred>
## Deferred Ideas

- **CLI override flag** (`--low-il-broker` / `--no-low-il-broker`) and **profile-wide heavy-runtime audit** — REQ-WSRH-AUDIT-01, explicitly deferred to a v2-follow-on. Not in v2.7.

### Reviewed Todos (not folded)
- `44-class-d-validator-preflight-investigation.md` — matched only on generic keywords (2026, phase, req, test); concerns a Linux deny-overlap validator preflight, orthogonal to a Windows field-validation + doc phase. Not folded.
- `44-validate-restore-target-fd-relative-hardening.md` — matched only on generic keywords (target, 2026, phase, req, docs); concerns restore-target fd-relative hardening, orthogonal to this phase. Not folded.

</deferred>

---

*Phase: 52-Field validation closure — heavy-runtime HUMAN-UAT + doc update*
*Context gathered: 2026-05-26*
