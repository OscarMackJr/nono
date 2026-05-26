# Phase 51: No-PTY Low-IL broker + token routing + write-deny preservation - Context

**Gathered:** 2026-05-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Re-route the non-PTY `nono run` supervised path off the restricting-SID `WindowsTokenArm::WriteRestricted` arm onto a **Low-IL broker arm with no synthetic restricting SID**, so heavy-runtime children (the confirmed case: the 234 MB self-contained `claude.exe`) survive DllMain/bootstrap (no `STATUS_DLL_INIT_FAILED 0xC0000142`) while mandatory-label `NO_WRITE_UP` write-deny is preserved at the OS level.

In scope (REQ-WSRH-01/02/03/05):
- A **no-PTY broker mode** in `crates/nono-shell-broker/` (anonymous-pipe stdio, no ConPTY).
- Extending the `select_windows_token_arm` cascade so a non-detached, non-PTY, session-SID, **profile-opted-in** launch dispatches to the Low-IL broker arm instead of `WriteRestricted`.
- A **real-spawn regression test** proving the Low-IL child's write to a Medium-IL-labeled path is kernel-denied (MIC pre-DACL).
- Full no-regression sweep: plain `cmd`/`echo` still passes, Phase 31 `nono shell` PTY path and the detached path unchanged, Windows CI green, cross-target Linux/macOS clippy clean.

Out of scope: Windows-host field validation of `claude --version` (REQ-WSRH-04/06 → Phase 52); profile-wide heavy-runtime audit (REQ-WSRH-AUDIT-01, deferred); CLI override flag; ConPTY resize on the no-PTY path; Option 2 null-token fallback (explicitly rejected).

</domain>

<decisions>
## Implementation Decisions

### Broker routing predicate
- **D-01:** The Low-IL broker route is a **profile-gated opt-in** via a new profile field (a boolean, Windows-only-meaningful, a no-op on Linux/macOS — cf. the existing `unsafe_macos_seatbelt_rules` precedent). Chosen over a blanket "all non-PTY run → broker" change (which would retire `WriteRestricted` and contradict REQ-WSRH-02's "still reachable, not a blanket removal") and over a binary-shape heuristic (which would make the security envelope depend on a non-deterministic, hard-to-audit guess).
- **D-02:** **Profile-only for v2.7** — no per-invocation CLI override flag (`--low-il-broker`/etc.). Keeps the surface minimal; a CLI override can be added later under the deferred REQ-WSRH-AUDIT-01 heavy-runtime audit. The field threads into `select_windows_token_arm` as a new input (e.g. `prefers_low_il_broker: bool`); when false, the existing `WriteRestricted` branch is taken unchanged.
- **D-03:** **Only the `claude-code` built-in profile** sets the field in v2.7 — the single confirmed `0xC0000142` case. codex / opencode / openclaw / swival stay on `WriteRestricted`. Matches the deferred REQ-WSRH-AUDIT-01 boundary and limits the blast radius to the one binary actually validated.

### No-PTY stdio mechanism
- **D-04:** The no-PTY broker uses **anonymous-pipe stdio, supervisor-relayed** — nono.exe creates the pipes, passes the ends to the broker via `--inherit-handle` as the child's std handles, and the supervisor relays child stdout/stderr to nono's own stdout. Reuses the Phase 17 attach machinery. Chosen over inherited-console (direct-to-terminal), which breaks `nono run … > file` redirection and prevents supervised output capture.
- **D-05:** Console-*presence* for the heavy-runtime child is handled by the broker's existing `AllocConsole` probe, independent of std-handle wiring. Researcher confirms the heavy-runtime child has a working console + valid std handles under this shape.

### Cascade arm structure
- **D-06:** Add a **distinct `WindowsTokenArm::BrokerLaunchNoPty` variant**. The pure `select_windows_token_arm` function returns it explicitly for the (non-detached, non-PTY, session-SID, profile-opt-in) case — a clean unit-test assertion target. Token *construction* is identical to `BrokerLaunch` (null `h_token`; broker self-degrades to Low-IL internally); the variant only signals the downstream spawn wiring (anonymous pipes, no ConPTY). Chosen over reusing `BrokerLaunch` + a `pty.is_none()` check so the existing PTY-path tests keep asserting `BrokerLaunch` — **structurally proving the Phase 31 PTY path is untouched**.

### Write-deny regression test shape
- **D-07:** **Real-spawn integration test** (REQ-WSRH-03): spawn an actual Low-IL child via the no-PTY broker; the child attempts to write a Medium-IL-labeled temp file; assert the write fails with access-denied (kernel MIC pre-DACL check). Highest fidelity, matches REQ-WSRH-03's wording and the Phase 31 `broker_dispatch_tests` runtime-assertion precedent (`IsProcessInJob`). Chosen over a construction-level unit test that only verifies the token/label pieces are set up correctly.
- **D-08:** **Hard-fail, no silent skip** — if the test can't set up the labeled fixture or spawn the Low-IL child, it FAILS loudly rather than `#[ignore]`-skipping. Consistent with the BROKER-CR-04 policy resolution (v2.5 Phase 41) that retired silent-SKIP for broker runtime tests. A non-running security regression test is treated as a real problem, not ignored. (Note for research: be mindful of the WRITE_OWNER drive-root label-apply limitation — use a `%USERPROFILE%`/`%TEMP%` fixture path, not a drive-root path.)

### Claude's Discretion
- Exact profile-field name (`windows_low_il_broker` is a suggestion, not locked) and its placement in the profile/policy schema.
- The exact name/signature of the new `select_windows_token_arm` input and how the profile field is resolved into it.
- stdin wiring on the no-PTY path (inherit nono's stdin vs a fourth pipe) — `claude --version` needs no stdin; keep it simple.
- Whether the broker needs an explicit `--no-pty` CLI signal or can infer the mode from the handle set / absence of a ConPTY attribute.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Root cause & requirements
- `.planning/debug/claude-exe-dll-init-failed.md` — confirmed root cause (WriteRestricted restricting-SID double-gate vs heavy-runtime DllMain writes) + the Option 1 fix decision + the orchestrator falsification test (whoami.exe survives, claude.exe fails).
- `.planning/REQUIREMENTS.md` — REQ-WSRH-01/02/03/05 (Phase 51 scope) acceptance criteria + traceability; out-of-scope exclusions (Option 2 rejection, ConPTY resize).
- `.planning/ROADMAP.md` § Phase 51 + § Cross-Phase Invariants — success criteria + the five in-force invariants (Windows-only-files intentional, cross-target clippy required, NO_WRITE_UP non-regression, Phase 31 PTY path unchanged, fail-closed on error).

### Reference implementation (Phase 31 broker)
- `.planning/phases/31-broker-process-architecture-shell-01/31-05-SUMMARY.md` — Phase 31 broker production validation: Low-IL primary token + inherited console, NO_WRITE_UP enforced, PowerShell/CLR child ran clean. The reference pattern this phase extends.
- `crates/nono-shell-broker/src/main.rs` — current broker: `AllocConsole` probe, `--inherit-handle` HANDLE_LIST, `CreateProcessAsUserW` with `EXTENDED_STARTUPINFO_PRESENT` (no pseudoconsole attribute), rejects empty inherit-handle list (BROKER-CR-03).
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `select_windows_token_arm` cascade (lines ~1106) + `WindowsTokenArm` enum (~1073) + the `created = if let Some(pty_pair) = pty { … }` spawn branch + existing arm tests (`pty_some_no_detach_selects_broker_launch`).

### Policy & profiles
- `crates/nono-cli/data/policy.json` — `claude-code` profile (the profile that gets the opt-in field); profile schema for the new field.

### Verification protocol
- `.planning/templates/cross-target-verify-checklist.md` — mandatory cross-target clippy verification (Linux + macOS targets) per CLAUDE.md § Coding Standards MUST/NEVER bullet.

### Phase 17 stdio precedent
- `crates/nono-cli/src/exec_strategy_windows/` — Phase 17 anonymous-pipe attach machinery (the stdio pattern D-04 reuses); D-07 anonymous-pipe-vs-ConPTY exclusivity.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`nono-shell-broker` (Phase 31):** already stdio-agnostic — inherits whatever handles nono.exe passes via `--inherit-handle`, uses `EXTENDED_STARTUPINFO_PRESENT` with NO pseudoconsole attribute, self-degrades to Low-IL. The no-PTY mode is largely a caller-side change (cascade routing + anonymous-pipe handle set) plus possibly a mode signal to the broker.
- **Phase 17 anonymous-pipe attach stdio:** the supervisor-relay pattern D-04 reuses for child stdout/stderr.
- **`select_windows_token_arm` pure decision function:** already unit-tested per-arm; the new `BrokerLaunchNoPty` arm slots in as a new branch + new test, leaving PTY-path tests asserting `BrokerLaunch` (proves PTY path unchanged — D-06).
- **`broker_dispatch_tests` (Phase 31):** runtime-assertion harness precedent (`IsProcessInJob`) for the D-07 real-spawn write-deny test.
- **`AppliedLabelsGuard` / `try_set_mandatory_label`:** existing NO_WRITE_UP label apply + RAII revert (Phase 21 WSFG); the test fixture's Medium-IL label and the child's Low-IL token reuse this.

### Established Patterns
- **Cascade ordering is load-bearing** (`launch.rs:1060-1066`): detached → PTY → session-SID → caps. The new arm inserts as a guarded branch *before* the `has_session_sid → WriteRestricted` fall-through, gated on the profile opt-in, so WriteRestricted stays reachable (REQ-WSRH-02).
- **Fail-closed, no silent downgrade** (ROADMAP invariant): any broker/token/label failure on the no-PTY path produces a clean `NonoError` + diagnostic, never a fallback to WriteRestricted or null.
- **Profile fields that are Windows-only-meaningful** parse cross-platform and no-op elsewhere (cf. `unsafe_macos_seatbelt_rules`).
- **WRITE_OWNER drive-root limitation** (memory `feedback_windows_mandatory_label_write_owner`): label apply fails on `C:\poc\*` drive-root dirs but succeeds on `%USERPROFILE%`/`%TEMP%`; the D-07 fixture must use a profile/temp path.

### Integration Points
- `select_windows_token_arm` (new input) ← profile field resolution (where the profile's opt-in becomes the `prefers_low_il_broker` cascade input).
- Broker invocation construction in `launch.rs` (`created = …`) ← new no-PTY branch wiring anonymous-pipe handles instead of ConPTY pipes.
- Supervised runtime stdout capture ← anonymous-pipe relay (so `claude --version` output reaches nono's stdout / Phase 52 validation).

</code_context>

<specifics>
## Specific Ideas

- Confirmed failing command (the bug this phase fixes, validated in Phase 52): `nono run --profile claude-code -- claude --version` → currently `0xC0000142`.
- Confirmed non-regression command (must still pass): `nono run --profile claude-code -- cmd /c "echo hi"`.
- The differentiator is heavy DllMain WRITE-type activity (`NtCreateSection SECTION_MAP_WRITE` on `\BaseNamedObjects`, named-object create, temp DLL extraction) against the restricting SID `S-1-5-117-*` — not nono being broken for all children (whoami.exe survived the same token in the falsification test).

</specifics>

<deferred>
## Deferred Ideas

- **CLI override flag** (`--low-il-broker` / `--no-low-il-broker`) for ad-hoc per-invocation routing — deferred with REQ-WSRH-AUDIT-01 (v2 / follow-on).
- **Profile-wide heavy-runtime audit** — which other built-in / heavy-runtime (Electron/Node/CLR) profiles hit the same gate (REQ-WSRH-AUDIT-01, explicitly deferred). v2.7 fixes only the confirmed claude-code case.
- **Windows-host field validation** of `claude --version` + `windows-poc-handoff.mdx` doc update — REQ-WSRH-04/06, Phase 52 (requires the fixed binary on a real host).

</deferred>

---

*Phase: 51-No-PTY Low-IL broker + token routing + write-deny preservation*
*Context gathered: 2026-05-26*
