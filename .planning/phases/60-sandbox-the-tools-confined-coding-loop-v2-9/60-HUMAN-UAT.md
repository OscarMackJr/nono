---
phase: 60-sandbox-the-tools-confined-coding-loop-v2-9
doc: live-human-uat-results
host: Win11 (operator), Claude Code v2.1.159, nono 0.57.5 (unreleased Phase 60 build)
started: 2026-06-01
status: sc1-mechanism-FIXED (F-60-UAT-04 resolved — DACL grant for restricting SID, commits b5324b3c+aff506eb, live-verified at nono-run layer); UAT 1 hook E2E + UAT 2/4/5 pending
---

# Phase 60 — Live Human UAT Results

Tracks the 5 human-verification items from `60-VERIFICATION.md` (status `human_needed`).
Build under test: local `main` rebuild, `nono 0.57.5` (contains unreleased Phase 60 confined
Write/Edit/MultiEdit arms + CR-01 `path_covers` fix). Runbook: `C:\temp\nono-phase60-uat-cookbook.md`.

## Results matrix

| # | UAT item | SC | Result | Notes |
|---|----------|----|--------|-------|
| 1 | Confined edit lands | SC 1 | 🟡 write-capability FIXED; hook E2E blocked by gap C | F-60-UAT-04 (restricting-SID DACL gap) fixed (b5324b3c+aff506eb): `nono run ... cmd.exe echo>file` lands. But the HOOK routes the write through `powershell.exe` (.NET) whose CLR fails at Low IL (`80070005`) — F-60-UAT-05 / gap C. Edit doesn't land via the hook yet. |
| 2 | Out-of-scope write denied at OS boundary | SC 1 | ⏸ not reached | blocked by item 1 |
| 3 | deny+additionalContext → Bash retry (A1) | — | ⏸ not reached | blocked by item 1 |
| 4 | PowerShell steering unprompted | SC 2 | ⏸ not reached | blocked by item 1 |
| 5 | E2E read→edit→run | SC 4 | ⏸ not reached | blocked by item 1 |

## F-60-UAT-01 — Self-disable guard fires on the documented working dir (setup conflict)

**Observed (2026-06-01).** From `C:\Users\omack\nono-poc` (the cookbook's "supported" dir), the
first `Write test.txt` was denied, and the Bash fallback was denied with the **same** reason:

```
refusing to wrap Bash: CWD '\\?\C:\Users\omack\nono-poc' covers Claude Code hook settings
or agent state; would allow the tool jail to disable its own hooks
```

Claude correctly recognized this as a deliberate safety guard (not a path-permission denial),
stopped after the deny without retry-spamming, and asked the operator how to proceed.

**Root cause — `crates/nono-cli/src/claude_code_hook.rs` lines 292–295:**

```rust
let project_claude = cwd.join(".claude");
if project_claude.exists() {
    return Ok(Some(self_disable_reason(&cwd)));
}
```

The guard refuses to wrap any tool call when the CWD contains a `.claude` subdirectory. The
UAT setup (cookbook §0 / bundle README step 3) wires the hook via
`Copy-Item ... "$proj\.claude\settings.json"` — i.e. it creates `nono-poc\.claude` **inside the
CWD that the jail would be granted write to**. A confined write to the CWD could therefore
overwrite the hook config and disable the very confinement under test, so the guard fires. The
guard is **working as designed**; the *documented setup is self-defeating*.

This is NOT the home `~/.claude` branch (`nono-poc` is a sibling of `~/.claude`, so
`cwd_covers_home_claude_state()` returns false) — it is specifically the project-local `.claude`
branch at line 293.

**Why it matters (design finding, exceeds /gsd-fast).** The guard is *CWD-coarse*: it disables
confinement for the entire CWD whenever **any** `.claude` dir exists directly under it. But
project-scoped hook registration (`<project>/.claude/settings.json`) is the **normal** way to
install a Claude Code hook. As written, sandbox-the-tools confinement can never engage in a real
project that registers the hook project-locally. The blanket CWD-level refusal could plausibly be
narrowed to: refuse only if the *specific write target* is within `.claude`, or only if the
granted scope provably includes `.claude`. That is a security-sensitive decision (don't weaken the
self-disable invariant) and needs a proper plan/debug pass — captured here as a follow-on, not
fixed inline.

**Operator workarounds — tried (2026-06-01):**

- **(B) Launch from a subdirectory — FAILED.** Kept `nono-poc\.claude\settings.json`, launched
  `claude` from `nono-poc\work`. `/hooks` showed **no** nono PreToolUse `*` entry. Confirmed via
  claude-code-guide: **Claude Code does NOT walk up the directory tree for project settings** — it
  reads `.claude/settings.json` relative to the launch CWD / git root only. So the only place the
  hook reliably loads (`CWD\.claude`) is exactly the place the guard refuses. Project scope is a
  hard dead end with the current guard.
- **(A) User-scope in real `~\.claude\settings.json` — REJECTED (unsafe here).** That file is the
  operator's live GSD harness (applies to every Claude Code session for the user, incl. the
  assistant session helping run this UAT). A `matcher:"*"` deny-by-default nono hook there would
  break normal Claude Code usage. Not viable on this box.
- **(C) `CLAUDE_CONFIG_DIR` relocation — WORKING PATH.** Point `CLAUDE_CONFIG_DIR` at an isolated
  dir (e.g. `C:\temp\nono-uat-cfg`) holding a minimal `settings.json` with ONLY the nono PreToolUse
  hook; copy `~\.claude\.credentials.json` into it to skip re-login; remove `nono-poc\.claude`; and
  launch `claude` from `nono-poc`. The self-disable guard checks the *real* `~\.claude` (resolved
  via `home_dir()`, NOT `CLAUDE_CONFIG_DIR`), which is a sibling of `nono-poc`, so it passes; the
  hook loads from the relocated config; and the operator's real `~\.claude` is never modified, so
  the assistant session is unaffected. NOTE: `.claude/settings.local.json` does NOT work — it still
  creates `CWD\.claude`, re-tripping the guard.

The CLAUDE_CONFIG_DIR requirement reinforces F-60-UAT-01: with the guard as written, the only way
to exercise confinement is a relocated/isolated user config, never project-scoped hooks.

## F-60-UAT-02 — Runner profile not installed by the bundle (setup gap)

**Observed (2026-06-01, after the CLAUDE_CONFIG_DIR fix unblocked the guard).** The hook engaged
correctly and denied `Write test.txt` with the confinement message ("Write is an in-process tool
that cannot be confined directly; use the Bash tool with the PowerShell command in
additionalContext"). Claude **auto-retried as a Bash call** using the base64/PowerShell command
from `additionalContext`. But the confined Bash exited 1 with:

```
nono: Profile not found: claude-code-tools-windows-runner
```

**Root cause.** The hook rewrites confined Bash to `nono run --profile claude-code-tools-windows-runner
--allow-cwd -- ...` (`claude_code_hook.rs:18` `DEFAULT_TOOL_RUNNER_PROFILE`, used at lines 399/414;
name is hardcoded, no env override). The bundle README step 1 only installed the *mediation* profile
(`claude-code-tools-windows`), which the hook path does NOT use — `nono claude-code-hook` makes the
allow/deny/rewrite decision in-process. The *runner* profile is what the confined Bash consumes, and
it was never copied into `%APPDATA%\nono\profiles\`.

**Fix (applied 2026-06-01).** Installed the runner profile from the repo:
`packages\claude-code\claude-code-tools-windows-runner.profile.json` →
`%APPDATA%\nono\profiles\claude-code-tools-windows-runner.json`; also added it to the
`C:\temp\nono-uat\` bundle and corrected the bundle README step 1 to install both profiles
(runner REQUIRED, mediation optional).

## Positive signals already observed (pre-runner-profile)
Even before the runner profile was installed, two behaviors validated cleanly on the live host:
- **A1 / UAT 3 (deny+additionalContext → Bash retry): WORKING.** On both the in-CWD and the
  out-of-scope attempts, Claude auto-converted the blocked Write into a Bash call using the exact
  PowerShell command from `additionalContext`, with no manual nudge. (Confirm once more across 3–5
  varied edits after the runner profile is in place.)
- **Sandbox-citizen refusal behavior: correct.** On the (config) failure Claude did not retry
  variants or attempt alternative paths — it stopped and surfaced the error, per the CLAUDE.md
  steering note.

## F-60-UAT-03 — Runner profile network.block requires the WFP service (blocks file-confinement UAT)

**Observed (2026-06-01, after installing the runner profile).** The confined Bash got further —
nono initialized the jail with the correct capabilities (`r+w \\?\C:\Users\OMack\nono-poc`, net
outbound blocked, +6 system/group paths) — then failed closed with:

```
nono: Platform not supported: Windows WFP runtime activation is required for blocked Windows
network access but the WFP service `nono-wfp-service` is registered but not running.
```

**Root cause.** The runner profile sets `network.block: true`; on Windows that requires the
`nono-wfp-service` WFP kernel service to be running, and it is registered-but-stopped. Starting it
needs an elevated admin shell (`nono setup --start-wfp-service`) — the same WFP service stop/start
surface tracked as the separate WFP elevated live-UAT carry-forward. This is the **network** layer;
it is orthogonal to the **file** confinement Phase 60's SCs validate.

**UAT deviation (applied 2026-06-01).** Installed a UAT-only variant of
`claude-code-tools-windows-runner` with `network.block: false` (original saved alongside as
`...\claude-code-tools-windows-runner.json.netblock.bak`). The file-confinement boundary (Low-IL
workdir grant + mandatory label) is independent of WFP, so this does not weaken UAT 1/2/3/5. Network
blocking + WFP is validated separately. Restore the `.netblock.bak` profile to test network blocking.

## F-60-UAT-04 — HEADLINE: confined write fails — .NET CLR will not start at Low IL (SC 1 mechanism broken)

**Observed (2026-06-01, all config layers cleared).** With guard bypassed (CLAUDE_CONFIG_DIR),
runner profile installed, and network.block disabled, the confined Bash retry finally reached the
Low-IL shell launch and the jail initialized with the correct grant (`r+w \\?\C:\Users\OMack\nono-poc`,
net outbound allowed). It then failed closed with:

```
Starting the CLR failed with HRESULT 80070005 (access denied).
```

**Root cause (hypothesis, needs debug confirmation).** The hook rewrites Write/Edit to
`nono run --profile claude-code-tools-windows-runner --allow-cwd -- powershell.exe -NoProfile
-NonInteractive -EncodedCommand <b64>` (claude_code_hook.rs ~line 397). **Windows PowerShell
(.NET Framework) cannot start its CLR at Low integrity** — HRESULT 80070005 = E_ACCESSDENIED on
CLR init. The Low-IL mandatory label (NO_WRITE_UP) blocks writes to Medium-IL locations the CLR
needs (e.g. `%TEMP%`/fusion); the jail only relabels the CWD as Low-IL-writable, not a temp dir.
The .NET `WriteAllText` payload never executes, so **no confined edit ever lands**.

**Impact.** This is the core mechanism of Phase 60 SC 1 ("file edits work, confined"). On a real
Win11 host with the shipped code + profiles, confined edits do **not** land. UAT 1 = FAIL. UAT 2
(out-of-scope deny) and 5 (E2E) cannot be meaningfully validated until the write path works — UAT 2
"passed" only incidentally because the CLR failed before any path check.

**Positive signals that DO hold (independent of this failure):**
- A1 / UAT 3 (deny+additionalContext → Bash retry): WORKING — Claude auto-retries every time, no nudge.
- Self-disable guard + CLAUDE_CONFIG_DIR isolation: correct.
- Jail capability scoping: correct (`r+w` exactly the CWD, nothing broader).
- Sandbox-citizen refusal behavior: correct (no workaround-hunting on failure).

**Diagnostic handed to operator (run from `nono-poc`, real PowerShell):**
- A) `... -- cmd.exe /c "echo hi> test_cmd.txt"` — does the Low-IL FILE write work without a CLR?
- B) `... -- pwsh.exe -NoProfile -NonInteractive -Command "Set-Content test_pwsh.txt hi"` — does PS7/.NET Core start at Low IL?
- C) `... -- powershell.exe ...` — reproduce the failure in isolation.

Decision tree: A-pass/C-fail → fix = no-CLR write (cmd) or grant Low-IL `%TEMP%`; B-pass → fix =
switch hook runner shell to `pwsh.exe`; all-fail → Low-IL jail blocks basic writes (deeper).

**Recommendation: escalate to `/gsd:debug`.** This is a code/design fix (runner shell choice or
Low-IL temp provisioning), not a setup tweak — exceeds /gsd-fast scope.

> **UPDATE 2026-06-01 — RESOLVED.** The A/B/C diagnostic below disproved the .NET-CLR hypothesis;
> debug `/gsd:debug` (slug `lowil-cwd-write-denied`, now at `.planning/debug/resolved/`) found the
> real cause: the WriteRestricted token's synthetic restricting SID was never added to granted
> writable-path DACLs. Fixed in commits `b5324b3c`+`aff506eb` (new `grant_sid_write_on_path`/
> `revoke_sid_on_path` lib primitives + `AppliedDaclGrantsGuard`). Live-verified: the confined
> `nono run ... cmd.exe echo>file` write now lands. Re-run UAT 1 through the Claude Code hook to
> close SC 1 end-to-end.

### Diagnostic A/B/C results (2026-06-01) — root cause is DEEPER than .NET

Ran the three isolation commands from `C:\Users\OMack\nono-poc` with the 0.57.5 msvc binary:

- **A) `cmd.exe /c "echo hi> test_cmd.txt"` (no CLR):** process launched, jail showed `r+w nono-poc`,
  then the write → **`Access is denied.`** A *basic, no-.NET* Low-IL write to the granted CWD is
  denied. This **disproves the "it's only a .NET CLR quirk" hypothesis.**
- **B) `pwsh.exe ...` (PowerShell 7 / .NET Core):** never launched —
  `Windows filesystem policy does not cover the executable path required for launch:
  C:\Program Files\WindowsApps\Microsoft.PowerShell_7.6.1.0_x64__8wekyb3d8bbwe\pwsh.exe`.
  PS7 is a Store/MSIX app under `WindowsApps`, a path the runner profile's launch policy doesn't
  cover. Secondary issue (switching the hook to pwsh would also need this path allowed).
- **C) `powershell.exe ...`:** `Starting the CLR failed with HRESULT 80070005` — the same
  access-denied class as A, manifesting at the CLR's own (Medium-IL `%TEMP%`/fusion) writes.

**Corrected root cause (for debug to confirm).** The Low-IL child cannot write to the granted CWD.
A (CWD write) and C (CLR temp write) are the same failure. The `nono run` output carries the likely
mechanism — repeated `WARN label guard: path has pre-existing mandatory-label ACE; skipping apply +
revert (grant may have no observable enforcement effect ...)` for group paths, plus the general
pattern that the CWD grant may not be producing an *effective* Low mandatory label (so the Low-IL
subject is still blocked from a Medium-IL CWD by NO_WRITE_UP). If the CWD relabel is a no-op /
ineffective, confined writes fail everywhere.

**Net:** SC 1 ("edits work, confined") does not hold on real Win11 with shipped code — and the cause
is the Low-IL write path itself, not the shell. UAT 2/5 cannot be validated until A passes. Hand the
full `nono run` stderr (including the label-guard WARNs and the `Access is denied` line) to
`/gsd:debug` as the primary evidence.

## F-60-UAT-05 — Hook confined-write vehicle is .NET PowerShell; CLR fails at Low IL (gap C, now on the hook E2E)

**Observed (2026-06-01, after F-60-UAT-04 fix).** Re-ran UAT 1 through the Claude Code hook. Write
denied → Claude retried as Bash with the hook's additionalContext command
(`[System.IO.File]::WriteAllText(...)` — a .NET PowerShell command). The hook wraps it as
`nono run ... -- powershell.exe -EncodedCommand <b64>`. The Low-IL `powershell.exe` failed:
`Starting the CLR failed with HRESULT 80070005`. The file did not land. Meanwhile the SAME confined
write via `cmd.exe` (no CLR) DOES land (F-60-UAT-04 fix). So the write CAPABILITY is fixed; the
hook's .NET VEHICLE is the remaining blocker.

**Likely mechanism (same as F-60-UAT-04, different path):** .NET Framework CLR startup writes to a
Medium-IL location (almost certainly `%TEMP%`/fusion). That path is NOT in the runner grant, so the
session restricting SID is absent from its DACL → under WRITE_RESTRICTED the CLR's startup write is
denied → `80070005`. (Confirm via the %TEMP%-grant experiment below.)

**Fix options (decision needed — a follow-on, not done here):**
- (1) Change the hook's confined-write vehicle to NOT use .NET: emit a `cmd.exe`-based write, or
  better, a native `nono write-file`-style primitive run inside the jail (nono.exe is native, no
  CLR). Most robust; avoids granting %TEMP%. Touches `claude_code_hook.rs` confined-write rewrite.
- (2) Grant a Low-IL-writable temp to the runner profile so the CLR can start (the F-60-UAT-04 DACL
  fix would then cover it). Simpler but broadens the jail (shared %TEMP%); prefer a per-session temp
  subdir if taken.

## Other findings (lower priority, also need a decision — not fast edits)
- F-60-UAT-01: CWD-coarse self-disable guard makes project-scoped hooks unusable; CLAUDE_CONFIG_DIR
  isolation is the only working deployment. Decide: narrow the guard vs. document the constraint.
- F-60-UAT-03: runner `network.block:true` requires the WFP service (elevated, separately tracked);
  UAT runs a no-network variant for file-confinement items.
