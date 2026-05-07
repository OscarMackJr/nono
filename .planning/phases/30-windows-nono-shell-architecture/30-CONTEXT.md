# Phase 30: Windows nono shell Interactive Enforcement Architecture - Context

**Gathered:** 2026-05-07
**Status:** Ready for planning
**Driver:** Debug session `.planning/debug/nono-shell-status-dll-init-failed.md` — `nono shell --profile claude-code` on Windows fails with `STATUS_DLL_INIT_FAILED (0xC0000142)` before this phase; SHELL-01's "validated" claim in PROJECT.md is wrong and must be reality-checked.
**Phase placement:** Not yet in ROADMAP.md. User decides v2.3 (in flight) vs v2.4 (next milestone) via `/gsd-phase add 30`.

<domain>
## Phase Boundary

Land OS-enforced filesystem write protection AND interactive TUI rendering for `nono shell --profile <name>` on Windows 10/11. The phase delivers either:

- A working `nono shell` Windows path that launches PowerShell 5.1 / cmd.exe under ConPTY with mandatory-label write enforcement intact, OR
- Documented evidence that no user-mode token shape can deliver both, plus a follow-up scope for kernel-driver work in v3.0.

**In scope:**
- Field-validate Option 3 (Low-IL primary token + ConPTY pseudoconsole) on the test box. Token shape: skip `create_restricted_token_with_sid`, call `create_low_integrity_primary_token` instead, drop session-SID restricting set entirely. Token IL is Low; ConPTY allocated; Job Object containment unchanged.
- If Option 3 launches AND mandatory-label NO_WRITE_UP fires for writes outside grant set: ship it. Includes test (mandatory-label write-deny verified inside the live shell), cookbook update (security envelope), bookkeeping (PROJECT.md SHELL-01 status corrected, debug session marked resolved), and field smoke against Claude Code TUI.
- If Option 3 fails (0xC0000142 OR write-deny doesn't fire): pivot to ProcMon-driven Win32 investigation (Option 5). Trace `\Device\ConDrv` ALPC interactions, named-section access, conhost handshake under restricted tokens. Goal: surface a sixth option (e.g., "ConPTY needs DACL ACE for restricting SID on `\BaseNamedObjects` per-session subdir") that none of the static-analysis options has identified.

**Out of scope:**
- Full v3.0 kernel mini-filter driver (Phase 6b territory; deferred long ago).
- Re-architecting the supervisor IPC model away from named pipes.
- Cross-platform RESL Unix backends (Phase 25).
- Authenticode chain-walker (Phase 28).
- AppContainer-based isolation (separate research; not on the Windows 10/11 user-mode menu).

**Acceptance:**
1. `.\nono.exe shell --profile claude-code --allow-cwd` on Windows 10/11 launches a sandboxed shell (no 0xC0000142, no silent exit). Verified on the test box.
2. `claude` runs inside the sandboxed shell with full TUI rendering (alternate screen buffer, cursor positioning, raw-mode input).
3. From inside the sandboxed shell, `Out-File` (or any direct write) to a path outside the grant set fails with "Access is denied" at OS level (mandatory-label NO_WRITE_UP enforcement, NOT just hook-level interception).
4. From inside the sandboxed shell, reads of granted paths (e.g. `~/.claude\claude.json`) still succeed.
5. PROJECT.md's SHELL-01 entry reflects current reality (validated/needs-rework/deferred — whichever this phase ships).
6. Cookbook (`docs/cli/development/windows-poc-handoff.mdx`) describes the security envelope honestly: which token shape, what's enforced at OS level, what relies on the Claude Code hook.

**Failure mode (explicit):** if Wave 2 (ProcMon) exhausts without surfacing a workable option, the phase ships with a documented finding that `nono shell` on Windows is structurally incompatible with simultaneous WRITE_RESTRICTED + ConPTY at user-mode and remains a v3.0 / kernel-driver concern. Cookbook reverts the `nono shell` recommendation; SHELL-01 status flips to "deferred to v3.0."

</domain>

<decisions>
## Implementation Decisions

### Token shape for ConPTY-allocating supervised path
- **D-01:** Wave 1 fix is **Low-IL primary token via `create_low_integrity_primary_token()`** — no WRITE_RESTRICTED, no session-SID. Rationale: WRITE_RESTRICTED+ConPTY combination triggers `STATUS_DLL_INIT_FAILED (0xC0000142)` (parallel to Phase 15's WRITE_RESTRICTED+DETACHED_PROCESS finding); Low-IL primary token avoids the brittleness while preserving mandatory-label write-deny because Low-IL subjects vs default Medium-IL files trigger NO_WRITE_UP.
- **D-02:** Null token (Option A — caller's identity Medium-IL) is **rejected**. Long-lived interactive shells warrant write protection at minimum; the Phase 15 detached waiver is a precedent for *short-lived* one-shot grandchildren, not for long-lived interactive subjects with full user typing.
- **D-03:** Anonymous-pipe stdio (Option 2 — Phase 17 pattern) is **rejected** because losing TUI rendering is worse than the security cost of dropping the per-session WFP differentiation.

### Investigation depth gating
- **D-04:** Wave 2 (ProcMon-driven Win32 investigation) is **conditional**, not unconditional. Spawn it only if Wave 1 field-test shows Option 3 also fails. Timebox: 3-5 working days. Goal: surface a sixth option from the actual Win32 mechanism (`\Device\ConDrv` ALPC, `\BaseNamedObjects` access, conhost handshake), not a token-shape iteration.

### TUI rendering as locked acceptance criterion
- **D-05:** `nono shell` on Windows MUST host Claude Code's full TUI (not text-mode fallback). Acceptance #2 will fail the phase if TUI rendering is degraded. This rules out any solution that drops ConPTY.

### Security envelope as locked acceptance criterion
- **D-06:** OS-level write-deny for paths outside the grant set is REQUIRED, not optional. The Claude Code PreToolUse hook is **defense-in-depth**, not the primary boundary. Acceptance #3 will fail the phase if writes succeed at OS level even if the hook would have blocked them. This rules out the Phase 15 detached waiver (null token) for the long-lived interactive path.

### POC ship gating
- **D-07:** v2.3 milestone delivery is NOT blocked on this phase. User chose "time pressure not binding — pick on technical merit." This phase can land in v2.3 if it ships in time, OR slip to v2.4 if Wave 2 is needed. POC users get `nono run -- claude` in the meantime (TUI rendering limitation documented; cookbook recommendation for `nono shell` stays put pending phase outcome).

### Hook-firing investigation is OUT OF SCOPE
- **D-08:** The separate concern that the Claude Code PreToolUse hook didn't fire when Claude read a path outside the grant set during today's field test is tracked as a follow-up debug session, NOT folded into this phase. Investigation reference: `crates/nono-cli/src/hooks.rs`, `crates/nono-cli/data/hooks/nono-hook.sh`, hook installation status surfacing in `nono setup --check-only` (currently absent). Suggested slug: `claude-code-hook-not-firing`.

### AppliedLabelsGuard leak is OUT OF SCOPE
- **D-09:** The 9 leaked Low-IL labels on user-home paths from a prior nono crash (observed in today's field run with `prior_rid="0x1000"`) are evidence of an `AppliedLabelsGuard` Drop lifecycle bug. Tracked as separate debug session. Suggested slug: `nono-labels-guard-leak`. Does not block this phase but should be opened for v2.4.

### SHELL-01 bookkeeping
- **D-10:** PROJECT.md's "✔ SHELL-01 — `nono shell` interactive ConPTY on Windows 10 17763+ — v2.0 Phase 08" entry is **wrong** (today's debug session invalidates it). Bookkeeping correction is in scope for this phase (Wave 1 first task) regardless of Wave 1's technical outcome — even if we ship a fix, the v2.0 "validated" claim was based on a smoke gate that didn't include `--profile claude-code` end-to-end with the full WRITE_RESTRICTED + ConPTY shape. Move SHELL-01 to "needs-rework" until phase completion, then update to validated/deferred per outcome.

### Claude's Discretion
- Wave structure: Wave 1 = Option 3 field-test; Wave 2 = ProcMon (conditional). Plan-phase will determine task breakdown.
- Whether Wave 1 implementation uses the exact Option D edit drafted earlier today (and reverted from working tree) or a refined version. Drafted edit is on disk in the debug session's "Files changed" notes.
- Whether to add a `should_use_low_il_for_pty` helper or inline the gate logic. Tests, naming, comment shape — planner discretion.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Today's debug session (primary input)
- `.planning/debug/nono-shell-status-dll-init-failed.md` — full investigation trail. Contains: H1-H8 hypotheses (only H7 still standing); 5 architectural options; static analysis confirming the WRITE_RESTRICTED+ConPTY = 0xC0000142 trigger; field-test data; reverted-from-working-tree diffs of Option A (null token) and Option D (Low-IL primary token). **Status: paused-pending-architecture-review** at the time of this CONTEXT.md; the planner should flip it to `architecture-decided-pending-implementation` and reference this CONTEXT.md as the resolution.

### Precedent debug session (parallel pattern)
- `.planning/debug/resolved/windows-supervised-exec-cascade.md` — Phase 15's resolution of the WRITE_RESTRICTED+DETACHED_PROCESS variant of the same 0xC0000142 bug class. Direction-b waiver (null token + AppID WFP). Documents the security trade-off precedent and why `should_allocate_pty` was introduced. § "Phase 15 Smoke Gate" + § "Resolution".

### Code under change
- `crates/nono-cli/src/exec_strategy_windows/launch.rs:1114-1349` — `spawn_windows_child`. The 5-arm token-selection cascade is the surgical edit point. Lines 1133-1160 hold the gate today (HEAD); Wave 1 adds a `pty.is_some()` branch that takes the Low-IL primary token.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs:1023-1077` — `create_low_integrity_primary_token`. Already implemented (dead code on supervised path per Phase 15 § "Eliminated hypotheses"). Wave 1 makes this code live for the ConPTY+supervised path.
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs:34-121` — `create_restricted_token_with_sid`. Comment block at lines 82-93 documents WRITE_RESTRICTED's actual semantics (writes-only blocking; reads pass through). Reference for the security-envelope cookbook update.
- `crates/nono-cli/src/supervised_runtime.rs:95-111` — `should_allocate_pty()` Windows arm. Phase 15's gate; unchanged in Wave 1.
- `crates/nono/src/sandbox/windows.rs:35-44, 56-73, 470-650` — Windows sandbox `apply()`, `try_set_mandatory_label`, mode → mask mapping. The mandatory-label write-deny mechanism that Wave 1 relies on for OS-level write enforcement.

### Bookkeeping under change
- `.planning/PROJECT.md` — "Validated" requirements list, specifically the `SHELL-01` entry. Wave 1 first task is bookkeeping correction.
- `.planning/STATE.md` — Session continuity; Stopped At line.

### Cookbook under update (post-Wave 1)
- `docs/cli/development/windows-poc-handoff.mdx` — POC cookbook. Wave 1 final task adds an honest security-envelope paragraph: which token shape, what's enforced at OS level, what relies on the Claude Code hook. Today's commit `0c69bd4b` recommended `nono shell --profile claude-code` as the TUI host on Windows; that recommendation should hold IF Wave 1 lands, and should be reverted IF Wave 2 also fails.

### Phase 15 implementation reference
- `.planning/phases/.archive/15-detached-console-conpty-architecture-investigation/` (if archived) or wherever Phase 15's plans live — Direction-b implementation pattern. The token-selection structure introduced in Phase 15 is the architecture this phase extends.

### External (Microsoft docs — researcher should pull)
- Microsoft `CreateRestrictedToken` MSDN — for the WRITE_RESTRICTED + restricting-SID semantics.
- Microsoft Mandatory Integrity Control overview — for NO_WRITE_UP / NO_READ_UP / NO_EXECUTE_UP rules and how subject vs object IL comparisons work.
- Microsoft `CreatePseudoConsole` / ConPTY documentation — for the `\Device\ConDrv` interaction model and what process attributes the pseudoconsole requires.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `create_low_integrity_primary_token()` at `launch.rs:1023-1077`: already implemented. Drops integrity to Low via `WinLowLabelSid`. Wave 1 lifts this from dead code to live code on the supervised+PTY path.
- `is_windows_detached_launch()` at `launch.rs:1402-1410`: gate helper for the Phase 15 detached path. Pattern to mirror — Wave 1 adds a parallel inline `pty.is_some()` check (or a helper, planner discretion).
- `pty_proxy::open_pty()` at `crates/nono-cli/src/pty_proxy/`: ConPTY allocation. Wave 1 doesn't change this; the failure was downstream in the token cascade.
- `try_set_mandatory_label` and `low_integrity_label_and_mask` in `crates/nono/src/sandbox/windows.rs`: per-path Low-IL labels with mode-derived NO_WRITE_UP / NO_READ_UP masks. Wave 1's write-deny acceptance (#3) verifies these fire correctly when the subject is at Low-IL.

### Established Patterns
- **Token selection cascade** (`launch.rs:1140-1160`): 4-arm `if/else if` over `is_windows_detached_launch`, `config.session_sid`, `should_use_low_integrity_windows_launch`. Wave 1 adds a fifth arm for `pty.is_some()` between detached and session_sid. Same shape; same RAII holder pattern.
- **Phase 15 direction-b waiver documentation**: STATE.md key-decisions block + cookbook section + commit body. Wave 1's security-envelope paragraph follows this template.
- **`AppliedLabelsGuard` RAII** (sandbox label apply + revert): Wave 1 doesn't touch this; just verifies it still fires correctly under the new token shape.

### Integration Points
- `exec_strategy_windows::execute_supervised` calls `spawn_windows_child(config, ..., pty=Some(pty_pair), ...)`. Token selection happens inside `spawn_windows_child`. Wave 1's edit is fully contained in that function.
- `WindowsSupervisorRuntime::initialize` calls `start_control_pipe_server` BEFORE the token-cascade-edit point. The capability-pipe SDDL fix (`938887f`) needs to remain compatible with Low-IL primary token children. Verify: the pipe DACL must admit Low-IL clients; the `(A;;0x0012019F;;;<logon_sid>)` ACE added in `938887f` for the WRITE_RESTRICTED case may or may not be sufficient when the client is Low-IL primary token. **Researcher MUST verify this** — it's the most likely lurking failure mode for Wave 1.

</code_context>

<specifics>
## Specific Ideas

- The Option D edit drafted earlier today (and reverted) is the closest-to-correct starting point for Wave 1. It restructured the token-cascade gate to add a `pty.is_some()` branch calling `create_low_integrity_primary_token`. Planner can re-derive the edit from the debug session's "Files changed" trail OR re-construct from scratch — both paths are fine.
- The user's test box has a release binary built from the reverted Option D code (`target/x86_64-pc-windows-msvc/release/nono.exe`, timestamp 16:26). It is NOT representative of HEAD. Wave 1 first build should be a fresh compile from HEAD with the new Wave 1 edit in place.
- 9 user-home paths on the test box already carry leaked Low-IL labels (`prior_rid="0x1000"`) from prior nono runs that bypassed `AppliedLabelsGuard` Drop. This is unrelated to Wave 1 but means Wave 1's field test will see "label guard: skipping apply + revert" warnings on those paths. Expected; not a Wave 1 failure indicator. Tracked separately as `nono-labels-guard-leak`.
- The Claude Code PreToolUse hook did NOT fire during today's field test for at least one tool call. This is a separate problem; tracked as `claude-code-hook-not-firing`. Wave 1 acceptance criteria explicitly require OS-level write-deny, not hook-level — so the hook concern doesn't gate Wave 1.

</specifics>

<deferred>
## Deferred Ideas

- **AppContainer-based isolation for `nono shell`** — strictly stronger than mandatory-label-only enforcement; available since Windows 8. Out of scope for this phase because AppContainer requires app capability sets, capability-aware ACLs on every accessed object, and likely breaks legacy shell apps (cmd, PowerShell 5.1) that don't enumerate capabilities. Strong v3.0 candidate.
- **AppContainer profile for the Claude Code child specifically** — same deferral; even narrower scope. v3.0.
- **Kernel mini-filter driver for FS deny enforcement** — Phase 6b territory; long-deferred to v3.0. Would unblock real read-deny on Windows (something nono can't deliver today even with WRITE_RESTRICTED).
- **`nono shell --integrity <Untrusted|Low|Medium>` user-controlled IL** — could be a v2.4 ergonomic improvement once Wave 1 establishes the Low-IL default works. Not needed to ship Wave 1.
- **`nono shell` on Linux/macOS** — not in this phase's scope. The Unix supervised path uses different mechanisms (Landlock for Linux, Seatbelt for macOS); separate work if needed.

### Reviewed Todos (not folded)
None — no todos matched this phase's scope per `gsd-sdk query todo.match-phase`.

</deferred>

---

*Phase: 30-Windows-nono-shell-Interactive-Enforcement-Architecture*
*Context gathered: 2026-05-07*
