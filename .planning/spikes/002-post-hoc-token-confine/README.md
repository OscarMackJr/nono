---
spike: 002
name: post-hoc-token-confine
type: standard
validates: "Given an arbitrary process the daemon did NOT spawn, when it lowers that running process's primary-token integrity from outside, then NEW unauthorized writes are denied (i.e. post-hoc confinement is feasible)"
verdict: PENDING
related: [001]
tags: [windows, daemon, token, integrity, security, seed-004]
---

# Spike 002: post-hoc-token-confine (KILLER)

## What This Validates

**Given** a normal agent process the daemon did **not** launch (here: a non-cooperating `cmd.exe`),
**when** a separate "daemon" process opens that running process's primary token and lowers its integrity
to Low from outside (`OpenProcessToken` + `SetTokenInformation(TokenIntegrityLevel)`),
**then** the process's **new** writes to Medium-IL locations become denied (`NO_WRITE_UP`).

This is the **killer** for SEED-004's literal vision ("detect/label any `AI_AGENT` process token and confine
it"). If post-hoc lowering is impossible or ineffective, the model must pivot to **daemon-as-launcher**
(spike 003) — confine at spawn, before any agent code runs.

## Research

- **Windows constraint:** mandatory-label / restricting-SID confinement is normally established at process
  **creation** (that's how `nono run` and the broker work — set the token, then `CreateProcess`). MIC consults
  the token's integrity on each access check, so *lowering* a running token's IL via `SetTokenInformation`
  (lowering is permitted; raising is not, given `TOKEN_ADJUST_DEFAULT`) *might* affect subsequent checks — but
  this is genuinely uncertain and worth an empirical test.
- **Known caveats even if it "works":** (a) **handle leak** — handles the agent opened *before* lowering are
  access-checked at open time and keep their rights; (b) lowering IL adds **no restricting SID** and does
  **no CWD grant relabel**, and (c) **network is not covered**. So a "PARTIAL" here is still not a sound
  boundary versus spawn-time confinement.
- **Mirrors** `crates/nono/src/sandbox/windows.rs::apply_low_il_label` (same `CreateWellKnownSid(WinLowLabelSid)`
  + `SetTokenInformation(TokenIntegrityLevel)`), but targets *another* process's token instead of self.
- **Prior art:** spike 001 (AppContainer is itself Low-IL); broker / `windows_appcontainer_wfp_validated`
  (spawn-time confinement is proven — the fallback if this spike fails).

## How to Run

On a real Win11 box, from a **normal (non-elevated)** PowerShell console, as the same user:

```powershell
cd .planning\spikes\002-post-hoc-token-confine
cargo build            # already compiles clean on the dev host
.\target\debug\post-hoc-token-confine.exe
```

The binary spawns a `cmd.exe`, drives a pre-confinement write, lowers cmd's token IL from outside, drives a
post-confinement write, and prints a `[SPIKE-002] VERDICT:` line. No admin needed (same-user token access).

## What to Expect

`[SPIKE-002]` lines report: target IL before/after, whether `SetTokenInformation` succeeded, and
`pre_exists` / `post_exists`. Decision:

- **`SetTokenInformation` FAILS** → **INVALIDATED** — can't lower a running primary token; must confine at spawn.
- **lowered, `post_exists=false` (post write DENIED)** → **PARTIAL** — post-hoc lowering affects new opens, but
  leaked handles + no restricting-SID/grant/network make it unsound; daemon-as-launcher (003) is still required.
- **lowered, `post_exists=true` (post write SUCCEEDS)** → **INVALIDATED** — lowering had no enforcement effect.

Any non-INVALIDATED-with-clean-PARTIAL result still points to **spike 003 (daemon-as-launcher)** as the sound model.

## Investigation Trail

- 2026-06-13: Built the spike (standalone `windows-sys` crate, empty `[workspace]` to stay out of the root
  workspace). Token code mirrors `apply_low_il_label` but targets a `cmd.exe` we spawn-but-don't-confine and
  drive via stdin (a stand-in for "a process the daemon didn't launch"). Compiles clean (debug). **Verdict
  PENDING the operator's real-Win11 run** — the answer is an OS-behavior fact (does lowering a running token
  take effect?) that cannot be determined from the dev host / Bash tool.

## Results

_Pending operator run. Paste the `[SPIKE-002]` output here; the verdict + caveats will be recorded and the
MANIFEST row updated. Expected outcome (hypothesis): INVALIDATED or PARTIAL-but-unsound → proceed to spike 003
(daemon-as-launcher)._
