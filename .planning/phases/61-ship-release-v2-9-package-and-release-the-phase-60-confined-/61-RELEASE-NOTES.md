# v2.9 — Windows Confined Coding Loop + Out-of-Box WFP Enforcement

**Release:** v0.58.0 / v2.9 milestone  
**Date:** 2026-06-03  
**Artifacts:** CI-signed machine + user MSIs (wrapper AND embedded payloads Authenticode-valid, Phase 53 sign-before-harvest gate)

---

## What Ships in v2.9

v2.9 delivers two Windows-specific features validated on live Windows 11 (build 26200):

1. **Phase 60 — Confined coding-loop POC** (defense-in-depth, not full isolation)
2. **Phase 62 — Out-of-box WFP kernel network enforcement** for supervised runs

Both features are framed honestly below. The Windows OS-enforced boundary is real and
structurally unavoidable — once applied, the child cannot widen it — but the full
isolation story requires further work beyond v2.9.

Also bundles untagged post-v2.7 drain fixes: `d8b7ce00` (broker `CreateProcessAsUserW`
GLE=87 HANDLE_LIST dedup), `005b4c9e` (no-PTY relay stdout-echo — child stdout was
swallowed), `0cbeb3be` + `b852826b` (WFP service-stop and MSI uninstall).

---

## Feature 1: Windows Confined Coding Loop (Phase 60 POC)

**Verdict: defense-in-depth, NOT full isolation.** The Claude Code TUI runs at
Medium IL. Only side-effecting tool calls are individually jailed to Low-IL via the
`PreToolUse` hook. The agent process itself retains unconfined reads; network
(`WebFetch`, `WebSearch`) and orchestration (`Task`, MCP-under-nono) calls are denied
outright in this POC.

### What works (live UAT PASS 5/5 on real Win11, 2026-06-01)

- `Write`, `Edit`, `MultiEdit`, and `NotebookEdit` tool calls are confined to a
  Low-IL per-call `nono` jail scoped to the target path via AppContainer capability
  mapping. A write outside the granted scope is denied at the OS boundary.
- `Bash` tool calls are jailed Low-IL with a PowerShell-runner shell story for the
  common PS-scripting loop.
- Everything else (network, Task, MCP) is denied by default.

### Honest limits

- The agent process itself is Medium-IL with unconfined reads.
- A fully-confined interactive TUI at Low-IL is OS-blocked (`0xC0000142` STATUS_DLL_INIT_FAILED)
  — this is why the design pivoted to tool-wrapping rather than jailing the whole TUI.
- Heavy-runtime children (e.g. `claude.exe`, ~234 MB) need the no-PTY Low-IL broker
  path (shipped in v2.7); full read-grant model for `claude.exe` under AppContainer
  is deferred (the lowbox is a different security principal than the user).

---

## Security: Hook-Layer `~/.claude` Self-Disable Guard (Phase 60-03)

v2.9 closes a residual attack surface in the Windows confined tool-mediation loop:
the `PreToolUse` hook now refuses to wrap `Bash` whenever the launch CWD covers
`~/.claude`, `~/.claude.json[.lock]`, or a project-local `.claude/` directory. The
guard fires before any `nono run --allow-cwd` invocation is emitted, using
path-component comparison (not string prefix matching) to prevent false-negatives on
names like `.claudefoo`. This closes the self-disable vector where a confined `Bash`
tool call could rewrite `~/.claude/settings.json` to remove the `PreToolUse` hook.

**Scope note (D-09 hook-layer boundary):** this protection is a hook-layer
fail-closed guard that fires when Claude Code is the launcher. The deny fires at
`claude_code_hook.rs:204` before any `nono run --allow-cwd` command is emitted;
the guard is fail-closed (if home cannot be resolved, the guard denies). A direct
`nono run --profile claude-code-tools-windows-runner --allow-cwd ~/.claude`
invocation outside the hooked Claude Code loop bypasses the hook entirely. The
Windows OS label backend has no deny-within-allow primitive for the allow-overlap
case (`add_deny_access_rules` is a Windows no-op; there is no `Deny` variant in
`AccessMode`). This bare-CLI gap is a documented limitation consistent with the
Phase 60 "defense-in-depth, not full isolation" verdict — it is accepted (T-61-04)
and deferred to v3.0 (which would require a kernel-level minifilter, Gap 6b).

Verified: 16/16 hook unit tests pass on the v0.58.0 build (see
`.planning/phases/.../61-D09-VERIFICATION.md`).

---

## Feature 2: Out-of-Box WFP Kernel Network Enforcement (Phase 62)

`network.block:true` on a supervised `nono run` now enforces WFP kernel filtering
**out of the box** on a machine-MSI host — no manual `nono setup --start-wfp-service`
step required.

### What ships

- Machine-MSI installs `nono-wfp-service` with `start=auto` — the SCM boot-starts
  the WFP service as SYSTEM, so enforcement is available immediately after install.
- The control-pipe SDDL grants non-elevated Interactive Users read+write access, so
  `nono run --block-net` works from a standard user session without elevation.
- Per-run AppContainer profiles are registered before spawn and cleaned up on exit.
  WFP filters are scoped by the per-run package SID (`ALE_USER_ID` + package SID).
- **Fail-closed:** if the service is not running at enforcement time, nono attempts
  an auto-start; if that fails the run is aborted with a diagnostic naming the exact
  remediation command. Network is never passed through unenforced.
- Clean uninstall via `msiexec /x` leaves no service, driver, filters, or install
  directory behind (Phase 53 gate preserved).

Validated: **5/5 success criteria on live Windows 11 (build 26200)**: out-of-box
block (non-elevated, from `%USERPROFILE%\.claude`), boot-start survival, fail-closed
remediation, clean uninstall (leaves nothing), and control-pipe isolation.
Security review: **33/33 threats closed** (62-SECURITY.md, threats_open: 0).

Closes Phase 60's F-60-UAT-03 (WFP kernel enforcement gap).

### Known deferrals

- `nono-wfp-driver.sys` is a **placeholder** — all real WFP enforcement is done by
  the user-mode `LocalSystem` service. A signed kernel minifilter (Gap 6b) is
  **deferred to v3.0**.
- No crash-loop recovery policy for the WFP service yet (AR-62-10, LOW accepted
  risk). Residual is self-DoS only: a crashed service stays stopped and runs fail
  closed — no enforcement bypass.

---

## Version

All five workspace crates bumped to `0.58.0` in lockstep (5 `Cargo.toml` versions +
6 internal path-dep pins). The `v0.58.0` git tag is the `release.yml` build trigger;
`v2.9` is a non-building milestone marker on the same commit. CI-signed machine + user
MSIs (wrapper AND embedded payloads Authenticode-valid via Phase 53 sign-before-harvest
gate and the `Verify MSI payload signatures` admin-extract CI step).

---

## Full Changelog

See [CHANGELOG.md §0.58.0](../../../CHANGELOG.md) for the complete entry.
