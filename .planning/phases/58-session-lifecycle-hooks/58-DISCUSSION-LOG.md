# Phase 58: Session Lifecycle Hooks - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-05
**Phase:** 58-session-lifecycle-hooks
**Areas discussed:** Fail-policy reconciliation, Windows trust level, Env-file export on Windows, Windows "vetted hook" bar

---

## Fail-policy reconciliation

| Option | Description | Selected |
|--------|-------------|----------|
| Fail-closed everywhere (override upstream) | nono policy = fail-closed both platforms; reinterpret SC2 as 'mechanism preserved', document divergence as fork invariant | ✓ |
| Profile toggle, default fail-closed | New `session_hooks.fail_open` field, default false | |
| Keep Unix fail-open, Windows fail-closed | Literal SC2; cross-platform behavioral split | |

**User's choice:** Fail-closed everywhere (override upstream).

### Follow-up: after-hook failure semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Non-zero exit code propagated, loud error | Loud log error AND nono exits non-zero so CI sees it | ✓ |
| Loud error only, preserve child exit code | Log loudly but keep child's exit code | |
| You decide | Defer to research | |

**User's choice:** Non-zero exit code propagated, loud error.
**Notes:** Before-hook fail → session does not start. SC2 reinterpreted as "runtime mechanism preserved, fail-policy hardened" — to be recorded as a fork invariant.

---

## Windows trust level (the ADR core)

| Option | Description | Selected |
|--------|-------------|----------|
| Confined Low-IL via broker arm | Hooks run Low-IL via existing broker (LowIlPrimary); confined not host-trusted; CLR needs primary token | ✓ |
| Host-trusted Medium-IL via broker | Unix parity but unconfined escape hatch | |
| Caller's IL, no broker | Simplest, no mandatory-label enforcement | |

**User's choice:** Confined Low-IL via broker arm.

### Follow-up: hook filesystem scope

| Option | Description | Selected |
|--------|-------------|----------|
| Session-dir + cwd write, nothing else | Write session dir + cwd, read script path; everything else denied at OS boundary | ✓ |
| Same capability set as the sandboxed child | Hook inherits profile's resolved CapabilitySet | |
| You decide | Defer to research | |

**User's choice:** Session-dir + cwd write, nothing else.
**Notes:** LowIlPrimary arm specifically (WriteRestricted can't start the CLR — Phase 60 finding). This scope is the ADR's core invariant.

---

## Env-file export on Windows

| Option | Description | Selected |
|--------|-------------|----------|
| Port it, ACL-locked + same dangerous-var filter | Windows env-export via session-dir env file (CREATE_NEW/ACL instead of O_EXCL+0o600); is_dangerous_env_var() extended for Windows | ✓ |
| Defer env-export, run hooks without it | Windows hooks run but no env push-back this phase | |
| You decide | Defer to research | |

**User's choice:** Port it, ACL-locked + same dangerous-var filter.
**Notes:** Low-IL-writer → Medium-IL-reader trust gap mitigated by ACL + dangerous-var filter; ADR must name the gap + mitigation. Windows danger set ≥ PATH/PATHEXT/COMSPEC (research to confirm full set).

---

## Windows "vetted hook" bar

| Option | Description | Selected |
|--------|-------------|----------|
| Parity port (owner + ACL + canonical) | Map upstream checks to Windows: canonical \\?\, regular file, owned by current user, no world/lower-IL-writable ACL, mandatory-label. No allowlist. | ✓ |
| Parity port + explicit profile allowlist | All parity checks PLUS path must match a profile allowlist | |
| You decide | Defer to research | |

**User's choice:** Parity port (owner + ACL + canonical).

### Follow-up: Windows interpreter / exec path

| Option | Description | Selected |
|--------|-------------|----------|
| Direct-exec only (.exe/.cmd/.bat/.ps1 by association) | Broker CreateProcess's the script directly | |
| PowerShell-runner (.ps1 via powershell.exe -File) | Standardize on PowerShell | |
| You decide | Defer to research given CreateProcess + IL constraints | ✓ |

**User's choice:** You decide (deferred to research).

---

## Claude's Discretion

- Windows interpreter / exec path under the Low-IL broker (constraint: script-file-references only, no inline scripts).
- `timeout_secs` enforcement on Windows (no `killpg` equivalent — Job Object / `TerminateJobObject` on the broker process tree). No hardcoded default.
- Session-id generation/reuse (follow existing `session::` helpers).
- ADR file location under `.planning/` (follow fork's existing ADR convention).

## Deferred Ideas

- Profile-level fail-open toggle — considered, rejected (D-01 chose unconditional fail-closed).
- Explicit hook allowlist in profile — considered, rejected (D-10 chose owner+ACL parity).
- Supervisor IPC robustness → Phase 59 (REQ-IPC-01).
