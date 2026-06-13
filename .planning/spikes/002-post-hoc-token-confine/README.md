---
spike: 002
name: post-hoc-token-confine
type: standard
validates: "Given an arbitrary process the daemon did NOT spawn, when it lowers that running process's primary-token integrity from outside, then NEW unauthorized writes are denied (i.e. post-hoc confinement is feasible)"
verdict: PARTIAL
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
  drive via stdin (a stand-in for "a process the daemon didn't launch"). Compiles clean (debug).
- 2026-06-13: Operator run (Win11 26200.8390, elevated console → cmd was High IL). `SetTokenInformation(Low)`
  on the running token SUCCEEDED (High→Low confirmed); new write to Medium-IL `%TEMP%` DENIED; pre write (before
  lowering) had succeeded. **PARTIAL** — post-hoc IL-drop works for new opens but is not sound (handle-leak
  window untested, no restricting SID retrofit, blunt vs authorized writes, no network). Surprising positive vs
  the going-in hypothesis; daemon-as-launcher (003) remains the sound primary.

## Results

**PARTIAL — feasible for new opens, but not a sound/complete boundary.** Operator run on Win11
build-26200.8390 (2026-06-13):

```
[SPIKE-002] spawned cmd.exe pid=26556
[SPIKE-002] pre-confinement write issued (exists=true)
[SPIKE-002] target IL before = High (0x3000)
[SPIKE-002] SetTokenInformation(Low) returned SUCCESS
[SPIKE-002] target IL after  = Low (0x1000)
   echo post> "...\spike002_post.txt"  →  Access is denied.
[SPIKE-002] result: pre_exists=true post_exists=false
[SPIKE-002] VERDICT: PARTIAL
```

**What was PROVEN (the surprise):** a separate same-user process CAN open an already-running process's
primary token (`OpenProcessToken`, `TOKEN_ADJUST_DEFAULT|TOKEN_QUERY`) and **lower its integrity from
outside** (`SetTokenInformation(TokenIntegrityLevel)` succeeded, IL High→Low confirmed by re-query), and the
lowered IL **takes effect for subsequent access checks** — a new write to a Medium-IL `%TEMP%` was denied by
`NO_WRITE_UP`. Contrary to the going-in hypothesis, post-hoc IL-drop is not impossible. (cmd was *High* here
only because the operator console was elevated; lowering Medium→Low is the same operation.)

**Why it's still PARTIAL / not a sound confinement (so daemon-as-launcher remains the primary model):**
1. **Handle-leak window (asserted, NOT tested here).** MIC checks handles at *open* time; any handle the agent
   opened *before* the daemon lowered its IL keeps its original access. The window between agent spawn and
   daemon detection+lowering is unbounded. Spike 002 only tested *new* opens — it did **not** empirically
   demonstrate the leak. → candidate follow-up spike.
2. **No restricting SID, and you can't retrofit one.** `CreateRestrictedToken` produces a *new* token; you
   cannot swap a running process's primary token. So the WRITE_RESTRICTED layer of the nono model is
   unavailable post-hoc.
3. **Blunt — breaks authorized writes too.** Lowering to Low without relabeling the agent's granted workdir as
   Low-writable denies *all* Medium-IL writes, including the agent's legitimate workspace. A usable post-hoc
   model would also have to relabel grant dirs (doable via `SetNamedSecurityInfo`, untested here).
4. **Network not covered.** WFP egress scoping is set up per-identity at/around spawn; IL-drop does nothing for it.

**Signal for the build:** post-hoc IL-drop is viable as a *supplementary* control (e.g. "demote a misbehaving
/ escaped agent on the fly"), but the **sound, complete** engine-agnostic model is still **daemon-as-launcher**
(spike 003) — confine at spawn so there is no leaked-handle window, a restricting SID can be applied, grant
dirs are relabeled, and WFP is scoped before any agent code runs. SEED-004's literal "detect-and-confine" is
feasible-but-leaky; "launch-and-confine" is the recommended primary.
