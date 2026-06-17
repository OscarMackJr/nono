# Phase 77: Copilot CLI End-to-End Confinement - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-17
**Phase:** 77-copilot-cli-end-to-end-confinement
**Areas discussed:** Target binary, Setup command surface, Gate auth model, Grant posture

---

## Target binary

| Option | Description | Selected |
|--------|-------------|----------|
| Standalone @github/copilot (Node) | Newer npm standalone `copilot` CLI; Node-ESM realpathSync/lstat ancestor-walk case the goal describes; add node.exe interpreter coverage | ✓ |
| gh copilot extension (Node) | The gh-CLI extension `gh copilot suggest` (exact SC1 string); thinner/legacy surface | |
| copilot.exe native PE (keep current) | Trust existing D-06 native-PE finding; but then the Node-ESM fix has no target | |
| Re-verify on host first | Defer; let research empirically determine the installed binary | |

**User's choice:** Standalone @github/copilot (Node)
**Notes:** Drives D-01/D-02/D-03 — update `copilot-cli` profile with `node.exe` coverage; SC1's `gh copilot suggest` example string is superseded by the standalone `copilot` invocation. Profile's D-06 native-PE assumption is stale for this target.

---

## Setup command surface

| Option | Description | Selected |
|--------|-------------|----------|
| Generic --grant-ancestors --profile <p> | Reusable across any profile's derived SID; aligns with engine-agnostic milestone theme | ✓ |
| Copilot-specific --copilot-ancestors | Literal ROADMAP flag; simplest but not reusable | |
| Researcher decides | Lock only idempotent + one-time-admin + non-destructive | |

**User's choice:** Generic --grant-ancestors --profile <p>
**Notes:** D-06. Exact flag/subcommand spelling is planner discretion provided it stays generic, idempotent, one-time-admin, non-destructive.

---

## Gate auth model

| Option | Description | Selected |
|--------|-------------|----------|
| SKIP_HOST_UNAVAILABLE when unauthed | Gate checks Copilot install + auth as precondition; SKIP (not FAIL) if absent; PASS needs a real authed suggestion with zero ACCESS_DENIED/module-crash | ✓ |
| PASS on confinement-proof without auth | Define real task as module-resolution succeeding even if suggestion fails on auth | |
| Researcher decides minimal task | Lock only that the gate distinguishes confinement-failure from auth-failure | |

**User's choice:** SKIP_HOST_UNAVAILABLE when unauthed
**Notes:** D-07/D-08. Gate must distinguish a confinement failure (FAIL) from an unrelated Copilot/auth/network failure (SKIP).

---

## Grant posture (persistence / reversibility)

| Option | Description | Selected |
|--------|-------------|----------|
| Permanent + documented, no undo | RA is attribute-read only, scoped to one stable package SID; leave ACE permanently; no revert | ✓ |
| Provide a --revoke counterpart | Clean uninstall story; more work | |
| Researcher decides | Lock only non-destructive + idempotent + no deny-ACE changes | |

**User's choice:** Permanent + documented, no undo
**Notes:** D-09. `--revoke` counterpart noted as a deferred idea.

---

## Claude's Discretion

- Exact CLI flag/subcommand spelling for the generic grant command.
- Minimal scriptable "real task" command/args for the gate.
- The precondition probe the gate uses to detect Copilot install + auth.
- Whether `node.exe` coverage suffices or the standalone `copilot` shim needs additional path coverage (settle empirically in research).

## Deferred Ideas

- `--revoke` / uninstall counterpart for the ancestor grant — deferred (permanent posture chosen).
- Copilot-specific `--copilot-ancestors` flag — rejected in favor of the generic command.
- Reviewed-but-not-folded todos: MSI VC++ prereq (→ Phase 80), POC-cert broker (enterprise, out of scope), macOS rlimit defect (unrelated platform).
