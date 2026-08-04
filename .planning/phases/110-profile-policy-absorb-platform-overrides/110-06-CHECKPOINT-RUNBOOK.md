# Phase 110 Plan 06 — Task 3 Checkpoint Runbook (PROF-03e live-kernel proof)

**Purpose:** prove that `CapabilitySet.localhost_port_ranges()` reaches the Windows kernel as a
native `FWP_MATCH_RANGE` filter — one filter object per range per layer, never unrolled per-port.

**Status of this document:** authored 2026-08-04 from a live read of the production code paths
(`exec_strategy_windows/network.rs`, `bin/nono-wfp-service.rs`, `sandbox/windows.rs`,
`sandbox/mod.rs`, `capability_ext.rs`). Two corrections to the original 4-step checkpoint text
are recorded below — read § 0 before running anything.

---

## 0. Corrections to the original checkpoint steps

### 0.1 `runas /trustlevel:0x20000 nono-wfp-service.exe` is wrong — it inverts the privilege model

`runas /trustlevel:0x20000` **drops** privileges. `nono-wfp-service.exe` is the process that calls
`FwpmFilterAdd0`, which **requires Administrator**. Running it at trustlevel 0x20000 guarantees the
filter-add fails.

The non-elevated `runas` in project memory (`wfp_confined_egress_and_daemon_gate`) applies to the
**daemon/client** side (`nono-agentd` / `nono run`), not the service. Correct split:

| Process | Privilege | Why |
|---|---|---|
| `nono-wfp-service.exe` | **Elevated / LocalSystem** (via SCM) | calls `FwpmFilterAdd0`; needs BFE write access |
| `nono run` (the thing under test) | **Non-elevated** | it is the confined subject; it only writes a JSON request to `\\.\pipe\nono-wfp-control` |

The service is a registered Windows service (`nono-wfp-service`, started with `--service-mode`),
not a foreground exe you launch by hand. Use `nono setup` (§ 1).

### 0.2 A profile with *only* `open_port_range` failed closed — FIXED 2026-08-04 (`7c7a189c`)

> **Resolved.** `has_port_rules()` now counts `localhost_port_ranges`, so the allow-all +
> `open_port_range` profile the checkpoint originally specified works as written. The
> `"block": true` workaround below is **no longer required** — § 2 now offers both shapes.
> Both cross-target clippy gates were re-run green for the fix. The analysis is kept for the
> record.

Prior behaviour, `crates/nono/src/sandbox/mod.rs:450`:

```rust
pub fn has_port_rules(&self) -> bool {
    !self.tcp_connect_ports.is_empty()
        || !self.tcp_bind_ports.is_empty()
        || !self.localhost_ports.is_empty()
    //  ← localhost_port_ranges is NOT checked
}
```

Trace for `{"network": {"open_port_range": [[49200, 49210]]}}` with no `block`:

1. `capability_ext.rs:1054` — `profile.network.block` is false, so `set_network_blocked` is never
   called → `NetworkMode::AllowAll`.
2. `sandbox/windows.rs:352` — `requires_backend` **does** include `!localhost_port_ranges.is_empty()`
   → `preferred_backend = active_backend = Wfp`.
3. `network.rs:1501` — `matches!(mode, AllowAll) && policy.has_port_rules()` → `has_port_rules()`
   returns **false** (ranges not counted) → falls through.
4. `network.rs:1506` — the `AllowAll` match arm requires `active_backend == None`, but it is `Wfp`.
   No arm matches.
5. `network.rs:1527` — catch-all → `Err(UnsupportedPlatform: "Windows network enforcement does not
   have an applicable active backend for this policy (preferred backend: wfp, active backend: wfp)")`.

**The run aborts before any IPC to the service.** This is fail-closed (safe), but it means the
checkpoint's literal step 2 cannot prove anything.

The service side is *not* the problem — `bin/nono-wfp-service.rs:1254` already treats
`allow-all + non-empty localhost_port_ranges` as "permit the range, block everything else". Only the
CLI-side gate is missing the range check. The discrete-port equivalent (`open_port`) works today, so
the asymmetry is an omission, not a design choice.

**Fix applied** (`7c7a189c`): `localhost_port_ranges` added to the disjunction, plus a doc comment
on the method recording that it must stay in sync with `requires_backend`, plus a regression test
(`allow_all_policy_with_only_a_port_range_has_port_rules` in
`crates/nono-cli/tests/wfp_port_integration.rs`).

The same commit also aligns the second call site — `network.rs:1626`'s
`AllowAll && !has_port_rules()` early-returns `Ok(None)`, i.e. *no enforcement at all*. That path was
unreachable because `select_network_backend` errors first, but the two predicates must not disagree.

No binding rebuild was required: `WindowsNetworkPolicy` is `#[cfg(target_os = "windows")]` and the
change is to a method body, not a struct shape — neither `../nono-py` nor `../nono-ts` can observe it
(consistent with Plan 110-08's finding for `localhost_port_ranges` itself).

### 0.3 "exactly ONE filter object" is the wrong count

`build_wfp_layer_specs()` (`nono-wfp-service.rs:1114`) returns **4 layers**:
`ALE_AUTH_CONNECT_V4`, `ALE_AUTH_CONNECT_V6`, `ALE_AUTH_RECV_ACCEPT_V4`, `ALE_AUTH_RECV_ACCEPT_V6`.

One range therefore yields **4 range filters** (2 outbound `RemoteRange`, 2 inbound `LocalRange`),
plus 4 block-all filters = **8 total**. The claim being tested is *not* "one filter" — it is
**"one filter per range per layer, not one per port"**:

| | Filters |
|---|---|
| Correct (`FWP_MATCH_RANGE`) | **8** — 4 range permits + 4 block-alls |
| Regressed (per-port unroll) | **48** — 11 ports × 4 layers + 4 block-alls |

### 0.4 `cmd /c "echo test"` exits too fast to inspect

Filters are installed for the lifetime of the run and torn down by the
`NetworkEnforcementGuard::WfpServiceManaged` cleanup (`network.rs:1719`). A child that exits in
milliseconds leaves nothing for `netsh` to show. Use a **long-lived child** and inspect from a
second window (§ 3).

---

## 1. Start the WFP service (Administrator PowerShell, once)

```powershell
cd C:\Users\OMack\nono
cargo build --workspace --release

# Register + start. Idempotent; safe to re-run.
.\target\release\nono.exe setup --install-wfp-service --start-wfp-service

# Confirm it is RUNNING (not STOPPED / START_PENDING)
sc.exe query nono-wfp-service
```

If a stale `nono-agentd.exe` holds a file lock during the build, kill it first
(`Get-Process nono-agentd -ErrorAction SilentlyContinue | Stop-Process -Force`) — this bit Plan 06
already.

Confirm the control pipe exists (this is what `nono run` connects to,
`network.rs:851`):

```powershell
Test-Path \\.\pipe\nono-wfp-control   # expect True
```

> If `--start-wfp-service` fails, `nono run` will *not* silently degrade — it returns the
> fail-closed `"...could not be started automatically (elevation is required)..."` error from
> `network.rs:1662`. That error text is itself evidence the gate is intact.

---

## 2. Author the test profile (any shell)

```powershell
$profileDir = "$env:USERPROFILE\.nono\profiles"
New-Item -ItemType Directory -Force -Path $profileDir | Out-Null

# Shape A — allow-all + range. The originally-specified shape; works as of 7c7a189c.
@'
{
  "meta": { "name": "portrange-probe" },
  "network": {
    "open_port_range": [[49200, 49210]]
  }
}
'@ | Set-Content -Encoding utf8 "$profileDir\portrange-probe.json"

# Shape B — block-all except the range. Stronger confinement semantic.
@'
{
  "meta": { "name": "portrange-probe-blocked" },
  "network": {
    "block": true,
    "open_port_range": [[49200, 49210]]
  }
}
'@ | Set-Content -Encoding utf8 "$profileDir\portrange-probe-blocked.json"
```

Both shapes emit the same `FWP_MATCH_RANGE` permits — `build_policy_filter_specs`'s
`needs_outbound_block`/`needs_inbound_block` predicates (`nono-wfp-service.rs:1254`/`:1308`) already
add the block-all filter when ranges are present, even in allow-all mode. **Run Shape A** for the
literal PROF-03e proof; Shape B is a worthwhile second pass.

Sanity-check the profile parses and the range survives into the capability set — no elevation, no
service needed:

```powershell
.\target\release\nono.exe profile show portrange-probe
```

Expect the port range to appear in the rendered output (Plan 110-05 added display support; Plan
110-06 added it to the WFP diagnostic string too).

---

## 3. Run non-elevated, inspect from a second window

**Window A — non-elevated** (a normal user PowerShell; do *not* use the Administrator window):

```powershell
cd C:\Users\OMack\nono
.\target\release\nono.exe run --profile portrange-probe -- cmd /c pause
```

Leave it sitting at `Press any key to continue . . .`. The filters are live for exactly this long.

**Window B — Administrator** (`netsh wfp` requires elevation to enumerate):

```powershell
netsh wfp show filters file=C:\Users\OMack\AppData\Local\Temp\wfp-filters.xml
[xml]$x = Get-Content C:\Users\OMack\AppData\Local\Temp\wfp-filters.xml

$nono = $x.wfpdiag.filters.item | Where-Object { $_.displayData.name -eq 'nono Network Policy Filter' }

"total nono filters : {0}   (expect 8, NOT 48)" -f $nono.Count
"FWP_MATCH_RANGE    : {0}   (expect 4)" -f (
    ($nono.filterCondition.item | Where-Object { $_.matchType -eq 'FWP_MATCH_RANGE' }).Count
)
```

### Pass criteria

- [ ] Total `nono Network Policy Filter` objects = **8**, not 48 — proves no per-port unroll (D-06)
- [ ] `FWP_MATCH_RANGE` conditions = **4** (2 × `IP_REMOTE_PORT`, 2 × `IP_LOCAL_PORT`)
- [ ] Each range condition's `valueLow`/`valueHigh` read **49200** / **49210**
- [ ] Every filter's `subLayerKey` resolves to `nono Network Policy Sublayer`
- [ ] Each filter carries an `ALE_USER_ID` (package SID) **or** `ALE_APP_ID` condition — proving the
      filter is scoped to the child, not machine-wide

Dump one range filter in full to eyeball the bounds:

```powershell
$nono | Where-Object { $_.filterCondition.item.matchType -contains 'FWP_MATCH_RANGE' } |
    Select-Object -First 1 | Format-List *
```

---

## 4. Behavioural probe (optional but stronger)

With Window A still paused, from **Window B**, stand up two loopback listeners and confirm the
inside/outside split. Any listener works; `Test-NetConnection` is enough for the connect side.

```powershell
# inside the range
$in  = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 49205); $in.Start()
# outside the range
$out = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 49199); $out.Start()
```

Then from **inside the sandbox** — re-run with a shell instead of `pause` so you can issue probes as
the confined subject:

```powershell
.\target\release\nono.exe run --profile portrange-probe -- powershell -NoProfile -Command `
  "'49205:' + (Test-NetConnection 127.0.0.1 -Port 49205 -InformationLevel Quiet); `
   '49199:' + (Test-NetConnection 127.0.0.1 -Port 49199 -InformationLevel Quiet)"
```

Expect `49205: True` and `49199: False`. Stop the listeners afterwards (`$in.Stop(); $out.Stop()`).

> The probe must run **inside** the sandbox. A probe from Window B is unconfined and will connect to
> both ports — that would be a false negative, not a finding.

---

## 5. Teardown

```powershell
# Window A: press a key to release `pause` — the guard cleans up filters on exit.
# Then confirm zero residue (Administrator):
netsh wfp show filters file=C:\Users\OMack\AppData\Local\Temp\wfp-after.xml
[xml]$y = Get-Content C:\Users\OMack\AppData\Local\Temp\wfp-after.xml
($y.wfpdiag.filters.item | Where-Object { $_.displayData.name -eq 'nono Network Policy Filter' }).Count  # expect 0
```

Non-zero here is its own finding — a cleanup leak, worth recording even if §3 passed.

The service may be left registered; stop it with `sc.exe stop nono-wfp-service` if you want a clean
host. `nono-wfp-service.exe --purge-wfp-objects` exists for stubborn residue.

---

## 6. Recording the verdict

Resume signal for the checkpoint is **"approved"** only if §3 passes (and §4 if run). Then, and only
then:

```
requirements.mark-complete PROF-03
```

…and flip the Phase 110 checklist box in `ROADMAP.md` plus the `110-06-PLAN.md` line.

If §3 shows 48 filters, or §0.2's `UnsupportedPlatform` error reappears despite the `7c7a189c` fix,
record the observed output verbatim — a diagnosed RED closes the checkpoint honestly and is a
successful execution.
