# Phase 87: Security Sync - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-20
**Phase:** 87-security-sync
**Areas discussed:** AF_UNIX enforcement mechanism, Cherry-pick vs manual port, Tests/verification, #1064 dedup guard approach, Scope of CR-01/CR-02 criticals

---

## AF_UNIX enforcement mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid: static deny unless grant, notify to validate dest | No grant → SECCOMP_RET_ERRNO(EPERM); grant present → USER_NOTIF validates per-call sockaddr_un | ✓ |
| Static gate only (closest to upstream) | Bake allow/deny at filter-build time from cap set; grant → allow all sends, no per-dest check | |
| Always route to USER_NOTIF supervisor | Every trapped send prompts/denies; max flexibility, prompt-flood + large new syscall surface | |

**User's choice:** Hybrid: static deny unless grant, notify to validate dest
**Notes:** Preserves upstream's connect-grant semantics on the fork's path-validation model. No-grant deny is fail-secure silent EPERM; remediation via Phase 86 diagnostic surface is opportunistic, not a blocker. Anchored on the existing `linux.rs:844-847` TODO comment.

---

## Cherry-pick vs manual port

| Option | Description | Selected |
|--------|-------------|----------|
| cherry-pick -x, port the conflict hunks | `git cherry-pick -x` per commit + DCO; resolve linux.rs/capability.rs conflicts by porting semantics onto fork structures | ✓ |
| Full manual port, reference commits in trailer | Hand-write fixes, cite SHAs; loses -x provenance, risks missing sub-changes | |
| Decide per-commit at execution | Try cherry-pick, fall back to manual; leaves disposition unpinned for planner | |

**User's choice:** cherry-pick -x, port the conflict hunks
**Notes:** One atomic commit per upstream SHA (e2086877, 6b3eb013). Mirrors Phase 86's 8-cherry-pick pattern.

---

## Tests / verification

| Option | Description | Selected |
|--------|-------------|----------|
| Port upstream + add fork hybrid test; Linux leg PARTIAL→CI | Adapt upstream matrix to fork module + add net-new hybrid-path test; Linux execution PARTIAL→CI | ✓ |
| Port upstream matrix verbatim only | As-is, no fork-specific additions; leaves hybrid notify behavior untested | |
| Gate the phase on green Linux CI before complete | Stricter; blocks closeout on CI turnaround + a push | |

**User's choice:** Port upstream + add fork hybrid test; Linux leg PARTIAL→CI
**Notes:** Same documented deferral category as cross-target clippy. Live GH Actions Linux lane is decisive.

---

## #1064 dedup guard approach

| Option | Description | Selected |
|--------|-------------|----------|
| Confirm-then-port fork-adapted guard | Regression test first; fork-adapted guard only if it fails; document if fork already safe | ✓ |
| Port upstream guard verbatim | Force-fit upstream logic onto divergent keying; risks double-guard/miss | |
| Test-only, defer code if fork is already safe | Add only the test; push the fix into execution if it fails | |

**User's choice:** Confirm-then-port fork-adapted guard
**Notes:** Evidence-first against the fork's divergent `deduplicate()` (platform-specific keying + deferred original/access updates).

---

## Scope of CR-01/CR-02 criticals

| Option | Description | Selected |
|--------|-------------|----------|
| Harden CR-02 here, defer CR-01 to Phase 88 | CR-02 (audit-integrity bypass) fixed now + documented as fork-divergence; CR-01 (FFI staleness) → Phase 88 | ✓ |
| Defer both, open a tracked fork-hardening item | Keep Phase 87 strictly SEC-01/SEC-02; avoid re-diverging mid-sync | |
| Harden both here | Security-hardening catch-all; widens scope, adds two divergence points | |

**User's choice:** Harden CR-02 here, defer CR-01 to Phase 88
**Notes:** CR-02 is security-relevant and fits "Security Sync"; record as deliberate fork-divergence in ledger/ADR. CR-01 is correctness, not a bypass — folds into Phase 88's open FFI surface.

---

## Claude's Discretion

- Exact seccomp BPF rule construction (AF_UNIX family detection, abstract-namespace handling, sendmmsg iovec walking).
- Whether connect-grant tracking needs supervisor-side state vs. a static filter decision.

## Deferred Ideas

- CR-01 (FFI stale `LAST_DIAGNOSTIC_CODE`, upstream-identical at `a6aa5995`) → Phase 88.
