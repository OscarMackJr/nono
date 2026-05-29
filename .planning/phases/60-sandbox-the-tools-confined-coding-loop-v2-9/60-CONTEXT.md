# Phase 60: Sandbox-the-Tools — Confined Coding Loop - Context

**Gathered:** 2026-05-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the merged PR #4 tool-mediation slice a **usable coding agent** for Windows POC
users: the Medium-IL `claude` TUI can **read, run commands, and edit files**, with each
side-effecting operation confined to a Low-IL `nono` jail. This phase turns the
currently-**denied** in-process file tools (`Write`/`Edit`/`MultiEdit`/`NotebookEdit`)
into **confined** file operations, and makes the PowerShell-backed shell usable.

This is defense-in-depth tool mediation, **not** agent isolation (the Medium-IL `claude`
process itself stays unconfined — see Read posture). Scope clarifies HOW to confine the
edit loop and shell; it does not add new capabilities.

</domain>

<decisions>
## Implementation Decisions

### File-edit confinement scope
- **D-01:** Confined file edits (`Write`/`Edit`/`MultiEdit`/`NotebookEdit`) are granted
  the **whole project CWD (`--allow-cwd`), consistent with the Bash runner** — not
  per-file. Rationale: the Bash runner already makes the entire CWD writable, so scoping
  Write tighter than Bash adds per-call friction (multi-file edits, new files in subdirs,
  Edit's read-before-write) without real isolation gain. The meaningful, OS-enforced
  boundary is **in-project vs. outside-project** (the merged UAT proved an outside-CWD
  write is denied at the OS boundary). Per-file least-privilege is deferred as largely
  theater while Bash is CWD-wide.

### Shell story (Bash vs PowerShell)
- **D-02:** Ship the Windows shell as a **PowerShell tool runner** and **steer Claude to
  emit PowerShell syntax** on Windows (via the profile tool-description / a system-prompt
  note — exact wiring is a planning detail). Do NOT attempt a confined real-bash path for
  the POC: MSYS2 cannot initialize as a Low-IL child (`0xC0000022` / `NtCreateDirectoryObject(\BaseNamedObjects\msys-2.0…)`).

### Agent read posture (SPEC Q5)
- **D-03:** **Accept unconfined Medium-IL agent reads** (`Read`/`Glob`/`Grep` can read
  anything the user can, incl. secrets/creds) and **document it clearly**. This matches
  the defense-in-depth framing: writes/exec/network are the confinement target, reads are
  not. Read-confinement is deferred (in-process `Read` can't be subprocess-wrapped without
  denying it and forcing read-via-Bash, which breaks the agent).

### Denied-tool UX
- **D-04:** Keep the **fail-closed deny with a clear reason** for unwrappable tools
  (`WebFetch`/`WebSearch`/`Task`/`mcp__*`), and ship a short **"what works / what's denied"
  tool-support matrix** for POC users so expectations are set upfront. Whether tools can be
  disabled/hidden via Claude Code settings (so Claude never attempts them) is a research
  question — pursue only if cheap.

### Self-disable invariant (folded todo)
- **D-05:** The per-call CWD grant for confined edits **must never make `~/.claude` hook
  state writable**. The merged hook CWD guard (`claude_code_hook.rs::cwd_self_disable_risk_reason`)
  already refuses to wrap when the resolved CWD covers `~/.claude` / `~/.claude.json[.lock]`
  / a project `.claude/`; the file-op confinement runs under the **same** guard (it gates
  before any rewrite). Do **NOT** rely on profile `add_deny_access` to carve `~/.claude` out
  of a CWD grant — it is a **Windows no-op for the allow-overlap case** (see canonical refs).

### Claude's Discretion (left to research + planning)
- The file-op **mechanism**: rewrite `Write`/`Edit` into a `nono run --allow-cwd -- <PowerShell write op>` subprocess (turning the in-process tool into a wrapped subprocess) **vs.** a `nono`-native Low-IL file broker. Both honor D-01 (CWD scope) and D-05 (guard); pick per research.
- The exact **PowerShell-steering wiring** (profile tool-description field vs. appended system-prompt note vs. a `CLAUDE.md` note).
- `Edit`'s read-before-write and new-file-in-subdir cases — covered by the CWD r+w grant (D-01); confirm no extra handling needed.

### Folded Todos
- **`2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`** — folded as
  **D-05**. Original problem: under `--allow-cwd`, launching from `~/.claude` re-grants
  write to the hook config, and profile `add_deny_access` doesn't stop it on Windows. Fits
  this phase because the new file-op confinement also runs under `--allow-cwd` and must
  preserve the hook-level CWD self-disable guard rather than depending on profile deny.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spec & rationale (read first)
- `.planning/quick/260528-sch-spec-the-sandbox-the-tools-windows-tool-/260528-sch-SPEC.md` — the de-facto spec; §7 Q1–Q6 are answered (enforceability verdict, tool coverage, capability mapping, Windows hook mechanics, security posture, sizing). MUST read.
- `.planning/research/windows-low-il-tui-blocked.md` — dead-end matrix + why the TUI is OS-blocked and why "sandbox the tools" is the pivot.
- `.planning/ROADMAP.md` (Phase 60 entry) — goal, success criteria, out-of-scope.

### The merged code being extended (PR #4, squash `7488dbba`)
- `crates/nono-cli/src/claude_code_hook.rs` — the PreToolUse handler: allow `Read`/`Glob`/`Grep`, rewrite `Bash`, **deny everything else** (the arms to change to "confine" for file tools), and the `cwd_self_disable_risk_reason` guard (D-05).
- `crates/nono-cli/src/hooks.rs` — hook install/reconcile + matcher (`*`).
- `packages/claude-code/claude-code-tools-windows.profile.json` — the experimental hook-carrier profile (matcher `*`).
- `packages/claude-code/claude-code-tools-windows-runner.profile.json` — the Low-IL Bash runner (CWD r+w, network blocked, no `~/.claude`); the file-op confinement will reuse/extend this.

### Capability + Windows enforcement mechanics
- `crates/nono-cli/src/capability_ext.rs` — per-path grant construction (`try_new_file`/`try_new_dir`, `--read`/`--write`/`--allow`, `FsCapability::new_file`); how a per-target grant would be built.
- `crates/nono-cli/src/policy.rs` — `add_deny_access_rules` (~605, macOS-gated emit) and `validate_deny_overlaps` (~1044, corrected comment) — **why `add_deny_access` is a Windows no-op for the allow-overlap case** (D-05).
- `crates/nono/src/sandbox/windows.rs` — the mandatory-label backend (grants → `SECURITY_MANDATORY_LOW_RID` labels; deny-within-allow is **not** wired).

### Folded todo
- `.planning/todos/pending/2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md` — folded as D-05.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`claude_code_hook.rs::pre_tool_use_response`** — the match arms that currently `deny`
  `Write`/`Edit`/`MultiEdit`/`NotebookEdit` are the exact extension point: change them to
  emit a confined file operation (per D-01/D-05) instead of `deny`.
- **`wrapped_bash_command` / `windows_bash_wrapper_script`** — the `nono run --profile
  claude-code-tools-windows-runner --allow-cwd -- powershell.exe -EncodedCommand …` +
  double-base64 trampoline pattern; reusable for a "confined write" subprocess if that
  mechanism is chosen.
- **`cwd_self_disable_risk_reason` / `path_covers` / `canonical_existing_path`** — the
  canonicalized, component-aware CWD guard; the file-op path must run under it (D-05).
- **`capability_ext.rs` per-path constructors** — `FsCapability::new_file`/`new_dir` with
  `AccessMode::{Read,Write,ReadWrite}` for building grants from `tool_input.file_path`.
- **`claude-code-tools-windows-runner` profile** — the Low-IL, network-blocked, no-`~/.claude`
  runner; the file-op confinement reuses it (CWD scope per D-01).

### Established Patterns
- Hook returns `permissionDecision: allow` + `updatedInput` (rewrite) or `deny` (fail-closed).
- **Deny-by-default** for unwrappable surfaces; matcher `*` routes all tools to the handler.
- Double base64 `-EncodedCommand` layering avoids nested-quoting fragility.
- Windows grants apply per-path `SECURITY_MANDATORY_LOW_RID` labels; deny is **not** a Windows
  enforcement primitive (rely on the hook guard / scoping, not `add_deny_access`).

### Integration Points
- The file tool arms in `pre_tool_use_response` (deny → confine).
- The profile tool-description / system-prompt for PowerShell steering (D-02).
- The runner profile (shared by Bash and the file-op path).

</code_context>

<specifics>
## Specific Ideas

- Driving goal: "get this working for POC users who need the **full coding loop (edit files)**" — read + run + edit, all confined.
- PowerShell-syntax steering is required: the PR #4 UAT showed the operator had to explicitly tell Claude "using PowerShell syntax" for Bash calls to work. D-02 removes that manual step.
- The merged build alone **denies** file edits, so this phase is the gating work before file-editing POC users are pointed at the experimental profile.

</specifics>

<deferred>
## Deferred Ideas

- **Network / `allow_domain` per-call grants** (confined `WebFetch`/`WebSearch`) — separate, bigger slice; out of POC scope.
- **MCP-under-`nono` or MCP-deny policy** + audit of MCP launch paths — separate slice.
- **`Task`/subagent confinement** — deferred; denied for the POC.
- **Read-confinement** — would need the same broker approach as writes; deferred (D-03).
- **Agent-process FS confinement** — the agent stays Medium-IL with unconfined reads (D-03 / SPEC Q5); a real isolation story is a separate initiative.
- **Wire deny→label in the Windows backend** so `add_deny_access` becomes a real Windows deny-within-allow primitive (`policy.rs:1038` follow-up) — would let profile-level carve-outs work; not needed once the hook guard handles self-disable.

</deferred>

---

*Phase: 60-sandbox-the-tools-confined-coding-loop-v2-9*
*Context gathered: 2026-05-29*
