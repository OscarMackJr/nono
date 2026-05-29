# Phase 60: Sandbox-the-Tools — Confined Coding Loop - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-29
**Phase:** 60-sandbox-the-tools-confined-coding-loop-v2-9
**Areas discussed:** File-edit grant scope, Shell story (Bash vs PowerShell), Agent read posture, Denied-tool UX

---

## File-edit grant scope

| Option | Description | Selected |
|--------|-------------|----------|
| Whole project CWD (consistent w/ Bash) | `--allow-cwd`, same as the Bash runner; smoothest for real coding; Bash already makes CWD writable so tighter Write scoping adds friction without isolation gain | ✓ |
| Target file + its directory | read+write on target file and parent dir; tighter; more per-call churn | |
| Exact target file only | strictest least-privilege; each file its own grant; largely theater while Bash is CWD-wide | |

**User's choice:** Whole project CWD (consistent with Bash) — D-01
**Notes:** The meaningful, OS-enforced boundary is in-project vs. outside-project (UAT proved outside-CWD writes are denied).

---

## Shell story (Bash vs PowerShell)

| Option | Description | Selected |
|--------|-------------|----------|
| PowerShell runner + steer Claude to emit PowerShell | honest PowerShell tool runner + tool-description/system-prompt steering; works now | ✓ |
| Build a confined real-bash path | Low-IL-capable bash; heavy/uncertain (MSYS2 fails Low-IL init) | |
| Ship as-is, document the caveat | no steering; users ask for PowerShell manually | |

**User's choice:** PowerShell runner + steer Claude to emit PowerShell — D-02
**Notes:** MSYS2 can't be a Low-IL child (`0xC0000022`); no real-bash for the POC.

---

## Agent read posture (SPEC Q5)

| Option | Description | Selected |
|--------|-------------|----------|
| Accept unconfined reads, document it | matches defense-in-depth framing; simplest | ✓ |
| Defer read-confinement to a later phase (note as gap) | accept now, track as follow-up | |
| Attempt read-confinement now | high friction; in-process Read can't be wrapped; would break the agent | |

**User's choice:** Accept unconfined reads, document it — D-03
**Notes:** Writes/exec/network are the confinement target; reads are not. Read-confinement deferred.

---

## Denied-tool UX

| Option | Description | Selected |
|--------|-------------|----------|
| Clear deny message + documented tool-support matrix | fail-closed deny + a "what works / what's denied" note for POC users | ✓ |
| Disable/hide tools in the profile | if settings support it, Claude never attempts them; feasibility is a research question | |
| Hard-deny only (current), no extra docs | minimal; risks confusion | |

**User's choice:** Clear deny message + documented tool-support matrix — D-04
**Notes:** Investigate settings-based tool disable only if cheap.

---

## Claude's Discretion

- File-op **mechanism**: rewrite-to-subprocess vs. nono-native Low-IL file broker (both honor D-01/D-05).
- Exact PowerShell-steering wiring (profile tool-description vs. system-prompt vs. CLAUDE.md note).
- `Edit` read-before-write / new-file-in-subdir handling (expected covered by the CWD grant).

## Deferred Ideas

- Network/`allow_domain` per-call grants (confined WebFetch/WebSearch).
- MCP-under-nono or MCP-deny policy + MCP launch-path audit.
- `Task`/subagent confinement.
- Read-confinement (broker, like writes).
- Agent-process FS confinement (real isolation — separate initiative).
- Wire deny→label in the Windows backend so `add_deny_access` becomes a real Windows carve-out primitive (`policy.rs:1038` follow-up).
