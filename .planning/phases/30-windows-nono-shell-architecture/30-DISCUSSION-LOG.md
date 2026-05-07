# Phase 30: Windows nono shell Interactive Enforcement Architecture - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-07
**Phase:** 30-windows-nono-shell-architecture
**Areas discussed:** POC ship gating, TUI rendering requirement, Security envelope acceptance, Investigation depth (ProcMon first?)
**Driver:** Live debug session `.planning/debug/nono-shell-status-dll-init-failed.md` surfaced that today's POC field validation (`.\nono.exe shell --profile claude-code --allow-cwd`) fails immediately with `STATUS_DLL_INIT_FAILED (0xC0000142)`. Two token-shape patches (Option A: null token; Option D: Low-IL primary token) were applied and reverted in-session. User chose to step back from token-tweak iteration and bring the architectural decision to a planning checkpoint instead of continuing to thrash in field-test mode.

---

## POC ship gating

Question: How time-critical is the v2.3 POC ship to the 3–5 user group?

| Option | Description | Selected |
|--------|-------------|----------|
| Ship POC NOW; defer `nono shell` to v2.4 | Ship the per-user MSI for the POC group this week. Document `nono shell` as Windows-unsupported until v2.4. POC users use `nono run -- claude` or `nono run -- claude --output text` (non-PTY). Lowest risk to the 2-week milestone target. | |
| Delay POC ~1 week to try option 3 (Low-IL primary token) | Accept ~1 week slip to field-test Option D (already drafted on disk, untested). If it launches AND mandatory-label write-deny works, ship POC with `nono shell` working. If it fails, fall through to defer-to-v2.4. | |
| Delay POC ~2 weeks for ProcMon investigation | Invest in deep Win32 investigation before committing. Only if you'd rather slow down than ship a half-answer. | |
| Time pressure not binding — pick on technical merit | POC timeline is flexible. Pick whichever option produces the best long-term security/UX outcome regardless of whether it lands in v2.3 or v2.4. | ✓ |

**User's choice:** Time pressure not binding — pick on technical merit.
**Notes:** Frees the decision from "we must ship in v2.3" framing. Phase 30 can land in v2.3 if it ships in time, or slip to v2.4 if Wave 2 (ProcMon) is needed. POC users get `nono run -- claude` in the meantime.

---

## TUI rendering requirement

Question: Is interactive TUI rendering inside the sandbox a must-have for the POC?

| Option | Description | Selected |
|--------|-------------|----------|
| Required — Claude TUI is the demo-quality experience | POC users will evaluate Claude Code's full-screen TUI. Anything less feels like a regression vs running Claude unsandboxed. Rules out options 1 + 2 (no ConPTY). | ✓ |
| Not required — non-PTY Claude is acceptable | POC users can live with text-mode Claude output for the initial pilot. Upgrade to TUI in v2.4 when we have a real fix. Keeps options 1 + 2 on the table. | |
| Don't know — verify Claude non-PTY behavior first | Need to test how Claude Code behaves without a TTY on Windows before deciding. | |

**User's choice:** Required — Claude TUI is the demo-quality experience.
**Notes:** Locks ConPTY allocation as a phase requirement. Rules out option 1 (defer entirely) and option 2 (anonymous-pipe stdio, no ConPTY). Acceptance criterion #2 in CONTEXT.md.

---

## Security envelope acceptance

Question: Is Phase 15's waiver (null token + AppID WFP, drop WRITE_RESTRICTED + per-session WFP) acceptable for a LONG-LIVED INTERACTIVE shell?

| Option | Description | Selected |
|--------|-------------|----------|
| Strict — write protection must be preserved | Long-lived interactive shells must keep write-deny. Rules out option 4 (null token). Option 2 (anonymous-pipe + WRITE_RESTRICTED) and option 3 (Low-IL primary token) are the candidates. | ✓ |
| Try option 3 first; fall back to waiver only if it fails | Prefer the Low-IL primary token approach (preserves mandatory-label write-deny). If it has the same brittleness as WRITE_RESTRICTED+ConPTY, then — only then — take the waiver. | |
| Waiver acceptable — same precedent as Phase 15 | Detached path took this waiver and shipped. Long-lived shell takes it too. Document the security envelope, move on. Unblocks option 4 (null token + ConPTY) immediately. | |
| Need more analysis on actual threat surface | The detached vs interactive threat-model difference deserves a written assessment before deciding. Adds a security-review substep. | |

**User's choice:** Strict — write protection must be preserved.
**Notes:** Combined with TUI=required, this rules out null token (Option 4). Phase 15 detached waiver does NOT extend to long-lived interactive shells. Acceptance criterion #3 in CONTEXT.md: OS-level write-deny is required, not hook-level only.

---

## Investigation depth (ProcMon first?)

Question: Should we invest in ProcMon-driven Win32 investigation (option 5) before committing?

| Option | Description | Selected |
|--------|-------------|----------|
| No — pick from 1–4 now, ship | Pragmatic. Time-box this debug session; ProcMon work belongs in v3.0 or whenever a kernel driver lands. Pick the best fit from the four known options. | |
| Yes — timebox 3–5 days; pick from 1–4 if no sixth option surfaces | Worth the investment if the user has time. ProcMon may localize the actual Win32 mechanism (ALPC port DACL, named-section access, conhost handshake) and surface a clean fix none of us has thought of. | |
| Defer — ProcMon depends on whether we have a viable known option | If options 2 or 3 work, ProcMon's marginal value is low. If none of 1–4 is acceptable, ProcMon becomes urgent. Decide AFTER picking from the other three questions. | ✓ |

**User's choice:** Defer — ProcMon depends on whether we have a viable known option.
**Notes:** Sets up a two-wave phase structure: Wave 1 = Option 3 field-test; Wave 2 = ProcMon investigation iff Wave 1 fails. Wave 2 is conditional, not unconditional.

---

## Cross-area resolution

The four orthogonal answers (time-flexible / TUI-required / strict-write / defer-procmon) eliminate options 1, 2, and 4 cleanly:

- Option 1 (defer to v3.0) — eliminated by TUI=required
- Option 2 (anonymous-pipe stdio) — eliminated by TUI=required
- Option 3 (Low-IL primary token + ConPTY) — **selected as Wave 1 of Phase 30**
- Option 4 (null token + ConPTY) — eliminated by strict-write
- Option 5 (ProcMon investigation) — **conditional Wave 2, fires only if Wave 1 fails**

---

## Claude's Discretion

- Wave structure: planner determines exact task breakdown for Wave 1 (rebuild, field-test, write-deny verification, cookbook update, bookkeeping correction) and Wave 2 (ProcMon traces, hypothesis generation, sixth-option synthesis, defer-to-v3.0 fallback).
- Whether Wave 1 implementation reuses the exact Option D edit drafted earlier today (reverted from working tree but reconstructable from the debug session) or refines it. Helper-vs-inline gate, naming, comment shape — planner discretion.

## Deferred Ideas

- **AppContainer-based isolation for `nono shell`** — v3.0 candidate; out of scope for this phase.
- **AppContainer profile for the Claude Code child specifically** — v3.0; same deferral.
- **Kernel mini-filter driver for FS deny enforcement** — Phase 6b territory; long-deferred to v3.0.
- **`nono shell --integrity <Untrusted|Low|Medium>` user-controlled IL** — v2.4+ ergonomic improvement once Wave 1 establishes the Low-IL default works.
- **`nono shell` on Linux/macOS** — not in this phase's scope; separate work if/when needed.
- **`claude-code-hook-not-firing` debug session** — separate concern surfaced during today's field test (Claude Code PreToolUse hook didn't fire when Claude read a path outside the grant set). Out of scope for Phase 30; tracked separately.
- **`nono-labels-guard-leak` debug session** — 9 leaked Low-IL labels on user-home paths from a prior nono crash bypassing `AppliedLabelsGuard` Drop. Out of scope for Phase 30; tracked separately.
