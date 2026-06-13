---
spike: 003
name: daemon-as-launcher
type: standard
validates: "Given one persistent launcher process, when it launches multiple distinct engines (cmd, powershell, python) through the confined nono primitive, then each runs confined identically — a write to the granted workdir lands, a write outside it is denied"
verdict: PENDING
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

- 2026-06-13: Built std-only launcher that mediates cmd/powershell/(python) through `nono run --allow-cwd`,
  each attempting a granted write + an outside write, and checks file outcomes. Compiles clean (debug).
  Verdict PENDING the operator's real-Win11 run (needs the broker + runner profile + a user-owned grant dir).

## Results

_Pending operator run. Paste the `[SPIKE-003]` lines. VALIDATED if every engine is `CONFINED ✓`; then reassess
004 (persistent token/job + multi-tenant AI_AGENT marker/IPC) and 005 (engine-agnostic abstraction via the
nono-py binding)._
