---
spike: 003
name: daemon-as-launcher
type: standard
validates: "Given one persistent launcher process, when it launches multiple distinct engines (cmd, powershell, python) through the confined nono primitive, then each runs confined identically — a write to the granted workdir lands, a write outside it is denied"
verdict: VALIDATED
related: [001, 002]
tags: [windows, daemon, broker, launcher, engine-agnostic, seed-004]
---

# Spike 003: daemon-as-launcher

## What This Validates

**Given** one persistent launcher process ("daemon"),
**when** it launches several *distinct* agent engines — `cmd.exe`, `powershell.exe`, and `python.exe` (if
present) — each through the confined `nono run` primitive,
**then** every engine shows the **same** boundary: a write to the GRANTED workdir lands, a write OUTSIDE it
is denied (`NO_WRITE_UP`).

This is the sound primary model for SEED-004 (spike 002 showed post-hoc IL-drop is feasible-but-leaky).
Confine at *spawn*, engine-neutrally.

## Research

- **Already proven, not re-tested:** `nono run -- <exe>` confines arbitrary executables via the Low-IL
  primary-token broker arm; A6-3a (R-A6 verification) landed a confined write byte-faithfully. The Claude
  *specificity* lives only in the PreToolUse hook, not in `nono run`. So engine-neutral confinement exists at
  the primitive level — this spike confirms it concretely on a **second/third engine type** and wraps it in the
  persistent-launcher shape the seed envisions.
- **Deviation from the spike-001 convention (documented):** this spike is **std-only Rust**, not a `windows-sys`
  binary, because it delegates all token work to `nono run`. The novel question is the launcher/engine-variable
  shape, not raw Win32.
- **Scope boundary:** persistent *token/job reuse* and the multi-tenant `AI_AGENT` marker + IPC are spike 004;
  the engine abstraction boundary + Python-binding path is spike 005. This spike = launcher shape + engine
  neutrality only.

## How to Run

On the real Win11 box, from a **user-owned, non-elevated** dir (R-B3 — else the granted subdir is
Administrators-owned and confined writes are denied):

```powershell
cd C:\Users\OMack\Nono\.planning\spikes\003-daemon-as-launcher
cargo build
# Ensure the granted dir will be user-owned. If your console is elevated, after first run:
#   takeown /F .\daemon_grant | Out-Null
.\target\debug\daemon-as-launcher.exe
```

Knobs (env): `NONO_EXE` (default `C:\Users\OMack\Nono\target\debug\nono.exe`), `NONO_PROFILE`
(default `claude-code-tools-windows-runner` — must be the `windows_low_il_broker:true` +
`network.block:false` variant so no WFP service is needed).

## What to Expect

Per engine, a line:
```
[SPIKE-003] engine=cmd         nono_exit=Some(0) granted_write=true outside_write_blocked=true  => CONFINED ✓
```
- **All engines `CONFINED ✓`** (granted write lands, outside write blocked) → **VALIDATED** — one launcher,
  many engines, identical confinement.
- **Any `CHECK ✗`** → usually a setup issue, not a confinement bug: granted dir not user-owned (R-B3 — takeown
  it), profile `network.block:true` without the WFP service, or non-dev-layout nono (broker trust gate). The
  binary prints these hints.

## Investigation Trail

- 2026-06-13: Built std-only launcher (v1) that mediates cmd/powershell/(python) through `nono run --allow-cwd`.
  Compiles clean (debug).
- 2026-06-13: **Operator run v1** (Win11 26200.8390). Results:
  - **cmd → CONFINED ✓** — `granted_cmd.txt` landed in `daemon_grant`; outside write `Access is denied`. A
    real engine confined correctly via the persistent launcher. (The substantive pass.)
  - **powershell → CHECK ✗ (harness bug, not confinement):** the relative write resolved to
    `C:\granted_powershell.txt` — powershell did NOT inherit the launcher's CWD as `$PWD`, so the write went to
    `C:\` and was (correctly) denied. Confinement worked; the test wrote to the wrong place.
  - **python → CHECK ✗ (fail-secure + a real finding):** `nono: ... filesystem policy does not cover the
    executable path required for launch: ...\Python312\python.exe`. nono refused to launch python because the
    runner profile doesn't cover python's exe path. Not a confinement failure — and a **SEED-004 design
    requirement**: an engine-agnostic daemon must grant/cover each engine's executable path.
- 2026-06-13: **v2 fix** — switched all writes to ABSOLUTE paths (no CWD dependence), and added
  `--allow <exe-dir>` for engines outside the default-covered system paths (python). Recompiled clean.
- 2026-06-13: **Operator run v2 — VALIDATED.** All three engines `CONFINED ✓`:
  - cmd: `granted_cmd.txt` landed; outside `Access is denied`.
  - powershell: `granted_powershell.txt` landed (absolute path); outside `UnauthorizedAccessException`.
  - python: launched after `--allow ...\Python312` (banner shows `r+w ...\Python312`); `granted_python.txt`
    landed; outside `PermissionError [Errno 13]`. The strongest engine-variable proof (a real non-shell engine).

## Findings (so far)

1. **Engine-neutral confinement works** — cmd confined identically to the proven claude/cmd path, via one
   persistent launcher. The "engine as a variable" claim holds at the launch level.
2. **Executable-coverage requirement (design):** nono fail-secure refuses to launch an engine whose binary
   path isn't covered by the policy. The daemon/abstraction (005) must enumerate + cover each engine's exe path
   (and interpreter, e.g. python.exe). This is part of "what every engine must expose."
3. **CWD is per-engine** — engines don't uniformly inherit the launcher CWD as their working directory; grants
   should be expressed as absolute paths, not assumed-relative-to-CWD.

## Results

**VALIDATED (2026-06-13).** One persistent launcher confined **3 distinct engines** (cmd, powershell, python)
identically — granted write lands in the relabeled-Low workdir, outside write denied (`NO_WRITE_UP`). The
"engine as a variable" core of SEED-004 holds at the launch level; the Claude-specificity was only ever in the
hook, not in the confinement primitive.

**Banked findings for the build / spike 005 abstraction contract:**
1. **Engine neutrality confirmed** on a real non-shell engine (python), not just shells.
2. **Executable-coverage contract:** the launch policy MUST cover each engine's executable (and interpreter)
   path, or nono fail-secure refuses to launch it. The daemon/abstraction must enumerate + grant these.
3. **CWD is per-engine:** express grants as absolute paths; don't assume an engine inherits the launcher CWD.

**Not covered here (next spikes):** persistent token/job *reuse* and a multi-tenant `AI_AGENT` marker + IPC
(004); the formal abstraction boundary proven via the `nono-py` binding on a real Python/LangChain agent (005).
