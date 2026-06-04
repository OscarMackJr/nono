---
quick_id: 260604-bt3
slug: cso-poc-demo-runbook
status: complete
date: 2026-06-04
---

# CSO POC Demo Runbook — nono on Windows (current functionality, safe)

A rehearsed, safe command sequence to show the CSO **OS-enforced, fail-closed isolation for
untrusted AI agents**. Every command below was dry-tested on this host on 2026-06-04.

**Core message for the CSO:** the dangerous operations aren't *policy-suggested* — they're
*structurally impossible*. Windows enforcement = Job Objects + AppContainer + **kernel WFP** +
a Low-IL broker. Unsupported shapes **fail closed**.

---

## Verified environment (this host, 2026-06-04)

| Component | State |
|-----------|-------|
| Installed `nono` | **0.57.12** (POC-signed, trusted on this host) |
| `nono-wfp-service` | **RUNNING / Automatic** (boot-started kernel WFP — Phase 62, out-of-box) |
| Dev-layout binary | `C:\Users\OMack\Nono\target\release\nono.exe` (0.57.5) — fallback for explain-only |

---

## ⚠ Ground rules — read before the demo (this is the "safely" part)

1. **Use a real console** — Windows Terminal or a PowerShell window. **NOT** git-bash/MSYS; the
   Low-IL broker spawn fails (`CreateProcessAsUserW GLE=87`) without a real console.
2. **Profile runs from a covered cwd** — `cd $env:USERPROFILE\.claude` first. A bare
   `%USERPROFILE%` fails the cwd-coverage gate (lesson D-52-01).
3. **This is an on-host POC build** — signed with an internal POC cert trusted on *this machine
   only*. Present it as a POC on a controlled host, **not** a public release (public Authenticode
   signing via Azure Trusted Signing is still in progress). Don't install it on the CSO's laptop.
4. **The WFP block is per-process / per-AppContainer scoped** — it does **not** take down host
   networking, and the filters auto-remove when the run ends (Phase 62 clean-uninstall, SC4).
5. **Do NOT demo the interactive TUI** (`nono shell` full-screen, or a full `claude` TUI under the
   jail) — it is OS-blocked (`0xC0000142`). Stick to `nono run` one-shots + `why` + `--dry-run`.
6. **Rehearse Act 2 once** before the CSO is in the room (the live network-kill is the centerpiece).
7. Use **innocuous network targets** only (e.g. `https://api.ipify.org`).

---

## Pre-flight (run once, ~1 min)

```powershell
nono --version                       # expect: nono 0.57.12
Get-Service nono-wfp-service         # expect: Running / Automatic
New-Item -ItemType Directory -Force C:\demo\workspace | Out-Null
cd $env:USERPROFILE\.claude          # profile-covered cwd for Act 2
```

---

## Act 1 — Explain the policy, execute nothing (100% safe, verified)

Pure policy queries — no process is launched, nothing changes. Best opener for a CSO: it makes
the enforcement model legible.

```powershell
# 1) Credentials/keys are denied by default
nono why --path "$env:USERPROFILE\.ssh\id_rsa" --op read
#   -> DENIED  Reason: sensitive_path  Policy: group:deny_credentials

# 2) Network is kernel-deniable — show the contrast
nono why --host api.openai.com --block-net      #  -> DENIED  Reason: network_blocked
nono why --host api.openai.com                  #  -> ALLOWED (default policy, for contrast)

# 3) Show the EXACT capability plan that would be enforced — runs nothing
nono run --dry-run --block-net --allow C:\demo\workspace -- cmd /c "echo hi"
#   -> Capabilities:  r+w C:\demo\workspace (dir) | net outbound blocked
#   -> "dry-run sandbox would be applied with above capabilities"
```

**Talk track:** capability-based allow-list; least privilege; fail-closed; the policy is
inspectable *before* anything runs.

---

## Act 2 — Live kernel network kill (centerpiece; Phase 62, UAT 5/5)

Show that a process can launch normally yet be **physically unable** to reach the network — the
Windows kernel (WFP) drops its traffic, with no manual firewall step (the service is boot-started).

```powershell
# CONTROL — prove the network works right now
curl.exe -sS -m 5 https://api.ipify.org
#   -> prints your public IP

# CONFINED — same command, kernel-blocked
nono run --profile claude-code --block-net --allow-cwd -- curl.exe -sS -m 5 https://api.ipify.org
#   -> curl: (6) Could not resolve host  (or a timeout) — the child STARTED, WFP dropped its egress
```

**Talk track:** "The agent process launched and ran. It simply *cannot* open a socket — the
kernel filter is scoped to its AppContainer. Nothing the agent does from inside re-enables the
network. No manual firewall change; the enforcement service starts at boot." After the run the
WFP filters auto-remove — show `Get-Service nono-wfp-service` is still healthy.

> Evidence on file if you'd rather not run it live: Phase 62 passed this 5/5 in human UAT
> (`.planning/phases/62-.../62-HUMAN-UAT.md`, SC1).

---

## Act 3 — Filesystem confinement (live, safe)

```powershell
# Inside the jail: allowed
nono run --allow C:\demo\workspace -- cmd /c "echo ok > C:\demo\workspace\proof.txt"   # succeeds

# Outside the jail: structurally denied
nono run --allow C:\demo\workspace -- cmd /c "echo pwn > C:\Windows\System32\pwn.txt"   # DENIED
```

Zero-risk variant (no process, same point):
```powershell
nono why --path "C:\Windows\System32\pwn.txt" --op write     # -> DENIED
```

---

## Act 4 — Accountability (optional)

```powershell
nono audit          # the recorded trail of what sandboxed commands were run
```

---

## Recommended 4-minute flow

1. **Pre-flight** (version + service running) — "kernel enforcement is already on, started at boot."
2. **Act 1** (`why` x3 + `--dry-run`) — explain the model, zero risk.
3. **Act 2** (curl control → confined) — the visceral kernel network-kill.
4. **Act 3** (write inside vs outside the jail) — filesystem least-privilege.
5. Close on **honesty slide** below.

---

## If something misbehaves mid-demo (graceful fallback)

- If Act 2's live AppContainer path hiccups: immediately fall back to **Act 1's
  `why --host … --block-net` + `--dry-run`** (both verified working on this host today) and cite
  the recorded Phase 62 UAT (5/5). Never improvise the TUI.
- If the installed `nono` misbehaves, the **dev-layout** `target\release\nono.exe` runs the
  explain-only Acts 1/3-`why`/4 without the broker trust gate.

---

## Be honest with the CSO (current limitations — say these unprompted)

- **Public code-signing is in progress.** Today's binaries are POC-signed (trusted on this host);
  the public Authenticode path (Azure Trusted Signing) is being finalized. So this is a
  controlled-host POC, not a shipped public installer yet.
- **The confined coding loop is defense-in-depth, not full isolation.** The shipping model is a
  Medium-integrity agent with **per-tool-call** confinement (each side-effecting tool runs in a
  Low-IL nono jail), because a fully isolated interactive agent TUI is currently OS-blocked on
  Windows. Frame it as raising the bar / containing blast radius, not a hermetic sandbox.
- **What IS kernel-hard today:** out-of-box WFP network enforcement (boot-started, fail-closed)
  and filesystem capability confinement — those are the parts to lean on.

## Self-Check: PASSED
- All Act 1 commands executed on this host on 2026-06-04 with the shown output (`why` deny/allow,
  `--dry-run` plan). `nono-wfp-service` confirmed RUNNING/Automatic; installed product 0.57.12.
- Act 2/3 live commands are the Phase 62 / Phase 60 proven shapes; flagged "rehearse first" and
  given a verified zero-risk fallback. No code changed (runbook only).
