# Phase 59: Supervisor IPC Robustness - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-06
**Phase:** 59-supervisor-ipc-robustness
**Areas discussed:** Read-timeout value & config, Windows AIPC parity approach, Keep-alive / re-accept semantics, Verification strategy

---

## Read timeout value & config

| Option | Description | Selected |
|--------|-------------|----------|
| 5s constant + NONO_ env override | Add to timeouts.rs (matches upstream d1851c9 + 55-05 pattern), clamp via MAX_TIMEOUT; wire Unix set_read_timeout() | ✓ |
| Plain fixed 5s constant | Named constant, no env override; breaks 55-05 convention | |
| Per-run configurable flag | CLI flag; overkill for an internal supervisor knob | |

**User's choice:** 5s constant + NONO_ env override
**Notes:** Reuse the established Phase 55 timeouts.rs convention; wire the already-present unused `set_read_timeout()` on Unix; Windows reads the same constant.

---

## Windows AIPC parity approach

| Option | Description | Selected |
|--------|-------------|----------|
| PeekNamedPipe poll + deadline + re-accept | Lower-risk translate; meets SC4 bounded-timeout+keep-alive+robust-accept | ✓ |
| Full overlapped (async) I/O | Closest fidelity; larger rewrite / more unsafe FFI; rejected as too risky this phase | |
| Keep-alive + robust accept only | Document read-timeout as Unix-only partial; weakest parity | |

**User's choice:** PeekNamedPipe poll + deadline + re-accept
**Notes:** Conservative translation on the divergent named-pipe surface; document the translate-not-cherry-pick rationale in the plan SUMMARY (SC4). Overlapped-I/O rewrite deferred.

---

## Keep-alive / re-accept semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Re-accept loop on the URL-open/direct-IPC listener | Match upstream 51f56b8 scope; satisfies SC1 "closes and reconnects" | ✓ |
| Don't-crash-and-continue only | No re-accept; weaker than SC1 | |
| Harden ALL supervisor IPC reads | Broader than upstream; larger blast radius | |

**User's choice:** Re-accept loop on the URL-open/direct-IPC listener
**Notes:** Match upstream C2 scope; also absorb f956fb6 (blocking mode), 9820a2e (loop keep-alive conditions), 4a22e94 (child capability grant).

---

## Verification strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Integration tests primary + Windows live-repro note | CI-runnable disconnect/reconnect + bounded-timeout tests, plus documented Windows named-pipe repro | ✓ |
| Integration tests only | No manual Windows repro; named-pipe timing edge cases may slip | |
| Windows live repro primary | Manual UAT primary; not CI-gated | |

**User's choice:** Integration tests primary + Windows live-repro note
**Notes:** Existing IPC tests cover round-trip only; disconnect/timeout scenarios are net-new.

---

## Claude's Discretion

- Exact constant/env-var names (follow `NONO_*_TIMEOUT` convention).
- `PeekNamedPipe` poll interval / watchdog mechanics (within the bounded-deadline contract).
- Bounded-retry vs unbounded re-accept loop (per upstream behavior + fail-secure).
- Cross-target clippy for unix-gated `socket.rs` is PARTIAL/CI-deferred on the Windows host.

## Deferred Ideas

- Full overlapped/async-I/O Windows named-pipe rewrite (revisit only if PeekNamedPipe poll proves insufficient).
- Blanket hardening of all supervisor IPC read paths beyond the URL-open/direct-IPC listener.
