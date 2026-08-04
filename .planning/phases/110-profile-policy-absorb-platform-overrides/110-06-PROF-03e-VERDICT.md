# Phase 110 Plan 06 Task 3 — PROF-03e Live-Kernel Verdict

**Verdict: PASS** on the criterion the checkpoint specifies (live WFP filter-table proof).
**Date:** 2026-08-04 · **Host:** Windows 11 Enterprise 26200 · **Operator-run, elevated capture**

---

## Verdict summary

| Criterion (110-06 Task 3) | Result |
|---|---|
| Range reaches the kernel as a native `FWP_MATCH_RANGE` filter | **PASS** |
| One filter object per range **per layer**, never unrolled per-port (D-06) | **PASS** |
| Filter scoped to the child, not machine-wide | **PASS** |
| Behavioural inside/outside connect probe | **NOT RUN** — see § Limitations |

`requirements.mark-complete PROF-03` is justified on this evidence *if* the operator accepts the
filter-table proof alone. The behavioural probe from the original 4-step checkpoint text could not
be executed on this host; it is **not** recorded as passing.

---

## Evidence

Binary under test: `target\release\nono.exe` + `target\release\nono-wfp-service.exe`, both built
2026-08-04 (14:17/14:25) and containing commits `7c7a189c`, `ea26b5b2`, `6d7ef719`.

Profile (`%APPDATA%\nono\profiles\portrange-probe.json`) — the allow-all shape the checkpoint
originally specified, no `block`:

```json
{ "meta": { "name": "portrange-probe" },
  "network": { "open_port_range": [[49200, 49210]] } }
```

Run: `nono run --profile portrange-probe -- cmd /c pause` (non-elevated), captured live via
`netsh wfp show filters` from an elevated window.

### Filter counts (diffed against a pre-run baseline)

| Measurement | Count |
|---|---|
| Baseline, no run active | 4 *(pre-existing orphans — see Defect 4)* |
| During run | **12** |
| **Installed by this run** | **8** |
| A per-port unroll would have installed | 44 permits + 4 blocks = **48** |

### The 8 filters

```
id=368224 PERMIT ALE_AUTH_CONNECT_V4      ALE_USER_ID/EQUAL, IP_REMOTE_PORT/FWP_MATCH_RANGE, FLAGS/ALL_SET
id=368225 BLOCK  ALE_AUTH_CONNECT_V4      ALE_USER_ID/EQUAL
id=368226 PERMIT ALE_AUTH_CONNECT_V6      ALE_USER_ID/EQUAL, IP_REMOTE_PORT/FWP_MATCH_RANGE, FLAGS/ALL_SET
id=368227 BLOCK  ALE_AUTH_CONNECT_V6      ALE_USER_ID/EQUAL
id=368228 PERMIT ALE_AUTH_RECV_ACCEPT_V4  ALE_USER_ID/EQUAL, IP_LOCAL_PORT/FWP_MATCH_RANGE,  FLAGS/ALL_SET
id=368229 BLOCK  ALE_AUTH_RECV_ACCEPT_V4  ALE_USER_ID/EQUAL
id=368230 PERMIT ALE_AUTH_RECV_ACCEPT_V6  ALE_USER_ID/EQUAL, IP_LOCAL_PORT/FWP_MATCH_RANGE,  FLAGS/ALL_SET
id=368231 BLOCK  ALE_AUTH_RECV_ACCEPT_V6  ALE_USER_ID/EQUAL
```

### Range bounds — all four conditions

```
id=368224 FWPM_CONDITION_IP_REMOTE_PORT low=49200 high=49210
id=368226 FWPM_CONDITION_IP_REMOTE_PORT low=49200 high=49210
id=368228 FWPM_CONDITION_IP_LOCAL_PORT  low=49200 high=49210
id=368230 FWPM_CONDITION_IP_LOCAL_PORT  low=49200 high=49210
```

Exactly 4 `FWP_MATCH_RANGE` conditions — one per layer, bounds correct, D-06 satisfied at the
kernel rather than only in the unit-tested spec-construction function.

---

## Limitations — read before treating this as complete

1. **Behavioural probe NOT RUN.** Neither `powershell` nor `curl.exe` starts inside the
   AppContainer under this minimal profile — both return `0xC0000142`
   (`STATUS_DLL_INIT_FAILED`), the known CLR/DLL-init constraint recorded in project memory. A
   valid behavioural test needs (a) a profile with real interpreter/binary coverage and (b) a
   listener that actually responds, since a bare `TcpListener` accepts without speaking HTTP and
   an unconfined control returned `28`/`28` — indistinguishable from a block. **Do not record
   the behavioural half as passing.**

2. **Baseline was 4, not 0.** Four orphaned block filters pre-existed and survived both
   `--purge-wfp-objects` and the service startup sweep (Defect 4 below). The verdict is a *diff*
   against that baseline, which is why § 3 of the runbook now mandates a pre-run capture — an
   absolute count cannot distinguish a fresh filter from a leaked one, and reading one absolute
   count is exactly what produced three misreadings earlier in the session.

3. ~~**Cleanup not re-measured after the fix.**~~ **CLOSED 2026-08-04.** After Defect 4's fix and a
   rebuilt service, `--purge-wfp-objects` returned exit 0 and the filter count went to **0** — the
   four orphans (ids 211023-211026) that had survived every prior purge attempt were removed,
   confirming Defect 4's diagnosis live. A subsequent run's filters were also gone at idle, so
   per-run teardown works. The full lifecycle is therefore **0 → 8 → 0**, and limitation 2's
   non-zero baseline no longer applies to any future run.

---

## Defects found while running this checkpoint

Four defects surfaced. The checkpoint could not have passed with any of the first three unfixed.

| # | Defect | Commit |
|---|---|---|
| 1 | `has_port_rules()` omitted `localhost_port_ranges`, so `select_network_backend` fell through to its catch-all and a range-only profile failed closed before reaching the service | `7c7a189c` |
| 2 | `cmd_show`'s `has_net` gate omitted the range fields, so `profile show` emitted no `Network:` section at all for a range-only profile | `ea26b5b2` |
| 3 | **Version-skew fail-open.** A `nono-wfp-service` older than the CLI silently dropped `localhost_port_ranges`, built zero filter specs, installed nothing, and returned `enforced-pending-cleanup`; the run exited 0 unenforced. Protocol bumped to 2 + `installed_filter_count` validation | `6d7ef719` |
| 4 | **Filter-sweep never enumerated anything.** `FWPM_FILTER_ENUM_TEMPLATE0.layerKey` is a `GUID` by value, so the zeroed template enumerated the all-zero GUID rather than "all layers"; `FWP_FILTER_ENUM_FULLY_CONTAINED` with zero conditions compounds it. Neither `--purge-wfp-objects` nor the startup sweep could remove a real nono filter. The sweep also short-circuited entirely when the sublayer was absent, making any survivor permanently unreachable | see § Defect 4 |

Defect 3 was proven live: with the stale service still registered, the new CLI produced
`Sandbox initialization failed: Windows WFP activation protocol mismatch: unsupported WFP runtime
activation protocol version 2; expected 1` and exited 1 — where an hour earlier the same profile
had exited 0 with no enforcement.

### Defect 4 — fix status

Fixed by enumerating per-layer with an explicit `layerKey` and `FWP_FILTER_ENUM_OVERLAPPING`,
driven from `build_wfp_layer_specs()` so the sweep list and the enforcement list cannot diverge,
plus removal of the sublayer-absent early return. Guarded by
`purge_covers_every_layer_enforcement_installs_into`.

**VERIFIED LIVE 2026-08-04.** With the rebuilt service, `--purge-wfp-objects` exited 0 and the
nono filter count dropped from 4 to **0** — the same four orphans (ids 211023-211026) that had
survived every prior purge attempt, including one run minutes earlier against the pre-fix binary.
The diagnosis (zeroed `layerKey` enumerating the all-zero GUID rather than "all layers") is
therefore confirmed by behaviour, not only by reading the bindings.

---

## Reproduction

Full procedure, including the service-identity check (§ 1.1) and baseline capture (§ 3) whose
absence caused the early misreadings: `110-06-CHECKPOINT-RUNBOOK.md`.
