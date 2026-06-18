# Phase 80: Clean-Host Install UAT - Context

**Gathered:** 2026-06-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Close the Phase 67 v2.11 carry-forward (**INST-01**): make the **machine** MSI actually install and run on a *fresh* Win11 host with no manual prerequisite steps, and prove it with an unattended `scripts/gates/clean-host-install.ps1` gate that plugs into the Phase 76 framework (`Test-Precondition`/`Invoke-Gate`, PASS/FAIL/SKIP_HOST_UNAVAILABLE).

This phase has TWO halves:
1. **The fix** — the binaries must load with no VC++ redist present, and a `nono-wfp-service` start failure must not roll back the install (today a clean host fails MSI `1603` and rolls back).
2. **The gate** — `scripts/gates/clean-host-install.ps1` that orchestrates an unattended install + verification on a clean host and emits the verdict.

**Out of scope:** publicly-trusted code signing (DIST-SIGN-01, enterprise milestone) and therefore the broker/supervised-path's untrusted-cert failure on a clean host — that is a *known, documented, deferred limitation*, not something this phase fixes.

</domain>

<decisions>
## Implementation Decisions

### Obtaining the clean host (D-01)
- **D-01:** **SKIP-on-dirty + operator-provided fresh VM.** The gate does NOT auto-spin a clean environment. `Test-Precondition` detects whether the current host is clean; if not, it returns `SKIP_HOST_UNAVAILABLE` with a reason. PASS is only achievable when the operator runs the gate on a deliberately-prepared fresh Win11 VM/snapshot. (Windows Sandbox auto-run was rejected — the feature is `Disabled` on the dev host and would need admin + a reboot to enable; the SKIP pattern matches Phase 76's contract and keeps the gate host-agnostic.)

### Clean-host detection criteria (D-02)
- **D-02:** **"Clean" = no prior nono only.** `Test-Precondition` returns a SKIP reason if EITHER `nono.exe` exists under `C:\Program Files\nono\` OR a `nono-wfp-service` / `nono-agentd` service is registered. It does **NOT** require the VC++ x64 runtime to be absent — with static CRT (D-03) the binaries don't need it, and a fresh Win11 image legitimately may already have VC++ from Windows/other software. The condition that actually matters is "this host has never had nono installed."

### The VC++ / install-rollback fix (D-03, D-04)
- **D-03:** **Static CRT (`+crt-static`).** Build all Rust binaries (nono, nono-shell-broker, nono-wfp-service, nono-agentd) with `target-feature=+crt-static` so `vcruntime140.dll` is not needed at all on the host — no redist payload, no chained installer. (Set via the Windows MSVC target only; Windows-only change, does not affect Linux/macOS linkage. Researcher/planner to confirm the exact `.cargo/config.toml` / RUSTFLAGS wiring and cross-target verification.)
- **D-04:** **Non-fatal `nono-wfp-service` start (belt-and-suspenders with D-03).** The MSI's service install/control for `nono-wfp-service` must be made non-fatal so a service-start failure does NOT roll back the whole product (SC #2 — the machine must be usable regardless). In WiX terms this is the `vital="no"` posture on the relevant `ServiceInstall`/`ServiceControl`. Both D-03 and D-04 ship: binaries always load AND a transient service hiccup never bricks the install.

### Cert / broker scope on the clean host (D-05)
- **D-05:** **Install-level only.** The gate asserts install exit 0 + `nono --version` runs (both cert-independent). It does NOT exercise the broker/supervised path. The broker's untrusted-POC-cert refusal on a clean host is recorded as a KNOWN deferred limitation (DIST-SIGN-01, enterprise milestone) — captured in the verdict/SUMMARY, not tested or fixed here. Matches the ROADMAP Phase 80 success criteria exactly.

### What PASS means + cleanup (D-06)
- **D-06:** **Install + run + non-fatal service + cleanup.** `Invoke-Gate` PASS requires ALL of:
  1. `msiexec /i <msi> /quiet` exits 0 (install completes, no rollback).
  2. `nono --version` runs from a NEW PowerShell session (PATH propagation).
  3. `nono-wfp-service` start is non-fatal — install succeeds even if the service fails to start (verify the install did not roll back; service state is reported in `detail`, a stopped/failed service is NOT a FAIL).
  4. The binaries load with no separately-installed VC++ redist (the static-CRT proof — this is implicit in #1/#2 succeeding on a host without VC++, and may be reinforced by a dumpbin/no-vcruntime-import check in `detail`).
  Then the gate **uninstalls** (`msiexec /x <msi> /quiet`) so a re-run on the same VM snapshot is repeatable.

### MSI artifact source (D-07)
- **D-07:** **`-MsiPath` param, default `dist\windows\nono-machine.msi`.** The gate takes an optional `-MsiPath` parameter defaulting to the locally-built machine MSI path; the operator stages that MSI on the fresh VM. The unsigned local build is acceptable (cert/trust is out of scope per D-05). Not the CI-signed release MSI (the POC cert is untrusted on a clean host anyway, and only matters for the out-of-scope broker path).

### Claude's Discretion
- Exact `detail` JSON fields beyond the verdict contract (install exit code, service state, `nono --version` output, optional dumpbin import check).
- The precise WiX attribute(s) that make the service non-fatal, and the exact `.cargo/config.toml` stanza for `+crt-static` (planner/researcher to verify against the live build).
- Whether to add an optional dumpbin/`link /dump /imports` assertion that no binary imports `vcruntime140.dll` (reinforces the static-CRT proof) vs trusting the clean-host install success as sufficient evidence.

### Folded Todos
- **`20260611-msi-vcredist-prereq.md`** (medium) — "machine MSI must handle the VC++ x64 runtime prerequisite." Clean Win11 host (no VC++) fails MSI `1603`: `nono.exe`/`nono-wfp-service.exe` hit `0xC0000135` STATUS_DLL_NOT_FOUND, the service start times out, and the whole install rolls back. **Folded as D-03 (static CRT) + D-04 (non-fatal service)** — this todo IS the core INST-01 fix this phase delivers.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone scope & requirements
- `.planning/ROADMAP.md` § Phase 80 — Goal, 4 Success Criteria, host gate ("clean Win11 host: no prior nono, no VC++ runtime, no pre-trusted cert"), unattended gate (`scripts/verify-dark.ps1 --gate clean-host-install`). Also § Phase 81 — the aggregator that consumes this gate's verdict.
- `.planning/REQUIREMENTS.md` — **INST-01** (this phase's requirement) and its origin DIST-01/DIST-02 (v2.11 Phase 67).

### Carry-forward todos (the actual clean-host findings)
- `.planning/todos/20260611-msi-vcredist-prereq.md` — the VC++ `1603`/`0xC0000135` rollback finding + fix options (folded — D-03/D-04).
- `.planning/todos/20260611-poc-cert-broker-clean-host.md` — the untrusted-POC-cert broker failure (reviewed, deferred — D-05). Origin: `.planning/phases/66-wr-02-edr-human-uat/66-HUMAN-UAT.md`.

### Gate framework (Phase 76) — the contract this gate plugs into
- `.planning/phases/76-self-verifying-harness-foundation/76-CONTEXT.md` — D-01..D-11: verdict JSON `{gate,verdict,reason,detail,timestamp}`; exit mapping 0/2/3; auto-discovered `scripts/gates/<name>.ps1`; `Test-Precondition`→`$null`/reason, `Invoke-Gate`→verdict (may shell out to msiexec); `.nono-runtime/verdicts/<gate>.json` persistence.
- `scripts/verify-dark.ps1` — the runner (dispatch + persist + exit mapping; gate must not call `exit` or `Persist-Verdict`).
- `scripts/gates/harness-self-check.ps1` and `scripts/gates/wfp-egress-isolation.ps1` — reference gate implementations of the contract.

### MSI build (the artifact under test + where the fix lands)
- `scripts/build-windows-msi.ps1` — generates the machine MSI from here-strings (the `.wxs` is GENERATED, not source — edit the script). Installs nono.exe, nono-shell-broker.exe, nono-wfp-service.exe (ServiceInstall/ServiceControl), nono-wfp-driver.sys, nono-agentd. D-04's non-fatal service change lands here.
- `scripts/validate-windows-msi-contract.ps1` — existing MSI-contract validation (style reference for MSI inspection).
- `docs/cli/development/windows-signing-guide.mdx` — referenced by the deferred cert todo (context only; signing is out of scope).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `scripts/gates/wfp-egress-isolation.ps1` / `harness-self-check.ps1` — copy the gate contract verbatim (`Test-Precondition`/`Invoke-Gate`, no bare `exit`, `[ordered]@{gate;verdict;reason;detail;timestamp}`).
- `scripts/windows-test-harness.ps1::Invoke-LoggedCargo` — child-process spawn + stdout/stderr capture + exit-code check idiom (mirror for the `msiexec` shell-out; do not extend the file).
- `.nono-runtime/verdicts/` — verdict persistence dir already in use by the runner (gitignored).

### Established Patterns
- PowerShell host-gated scripts use `$ErrorActionPreference="Stop"` + `$PSNativeCommandUseErrorActionPreference=$false` so native-tool stderr isn't promoted to a terminating error — apply for the `msiexec` shell-out.
- "Run from a NEW shell" for PATH propagation: launch `nono --version` via a fresh `pwsh -NoProfile -Command` (not the current session) so the verdict reflects what a real new session sees after install.

### Integration Points
- **Phase 81 aggregator** consumes this gate's `.nono-runtime/verdicts/clean-host-install.json` — the gate must conform to the shared verdict shape so the rollup works. A clean-host SKIP on the dev host must flow through as `SKIP_HOST_UNAVAILABLE` (→ `PASS_WITH_SKIPS` at the aggregator), never a FAIL.
- **`scripts/build-windows-msi.ps1`** — where D-04 (non-fatal service) lands; D-03 (static CRT) lands in the cargo/build config consumed before the MSI harvest.

</code_context>

<specifics>
## Specific Ideas

- The exact clean-host failure to prevent (from the todo): machine MSI `1603`; `nono.exe` `0xC0000135` (STATUS_DLL_NOT_FOUND, missing `vcruntime140.dll`); `nono-wfp-service` start → SCM event `7009` timeout → full rollback. The fix must make `nono --version` work AND keep the install from rolling back on a service hiccup.
- Gate invocation shape: `pwsh scripts/verify-dark.ps1 --gate clean-host-install` (optionally with the MSI staged at `dist\windows\nono-machine.msi` on the VM, or `-MsiPath`). On the dev (dirty) host this emits `SKIP_HOST_UNAVAILABLE`; on a fresh VM it emits `PASS`.

</specifics>

<deferred>
## Deferred Ideas

- **Publicly-trusted code signing** (DIST-SIGN-01) — enterprise milestone. Until then the broker/supervised path does not work out-of-box on a clean host (untrusted POC cert). Not fixed or tested here.
- **Windows Sandbox auto-run** of the clean-host gate — rejected for now (feature disabled on dev host; needs admin + reboot). Could be revisited as a future ergonomics improvement so the gate self-provisions a clean environment.
- **Optional `nono setup --trust-broker` helper / POC-cert import UX** — would make the broker path testable on a clean host, but pulls deferred cert-trust work into scope. Out for this phase.

### Reviewed Todos (not folded)
- **`20260611-poc-cert-broker-clean-host.md`** (high) — untrusted-POC-cert broker failure on clean host. Reviewed; NOT folded. Per D-05 the gate is install-level only; the real fix (public signing) is DIST-SIGN-01 (enterprise milestone). This phase records it as a known deferred limitation rather than solving it.

</deferred>

---

*Phase: 80-clean-host-install-uat*
*Context gathered: 2026-06-18*
