# Phase 82: Fleet Deployment Infrastructure - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-18
**Phase:** 82-fleet-deployment-infrastructure
**Areas discussed:** VC++ runtime strategy, Root-cert trust install, nono health schema, Scratch provisioning

---

## Todo folding

| Todo | Decision |
|------|----------|
| `msi-vcredist-prereq` | ✓ Folded |
| `poc-cert-broker-clean-host` | ✓ Folded |
| `macos-rlimit-as-setrlimit-fails` (0.2 score) | Reviewed, not folded (unrelated macOS defect) |

---

## VC++ runtime strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Static CRT (+crt-static) | RUSTFLAGS on all 3 Windows binaries; no redist ever; can't roll back on DLL-missing. CRT no longer Windows-Update-serviced; larger binaries. | ✓ |
| Bundle vc_redist chain | WiX Bundle/Burn chains vc_redist before nono.msi; CRT stays serviced. Changes artifact from .msi to .exe bundle (affects detection rule + signing). | |
| Non-fatal only (no CRT fix) | Keep dynamic CRT; Vital=no + documented prerequisite. nono.exe itself still won't load without redist — installed but broken. | |

**User's choice:** Static CRT (+crt-static)
**Notes:** Follow-up — flag location. Chose **`.cargo/config.toml` (all builds)** over release/CI-env-only, so dev == CI == shipped MSI binary (closes the dev-host-vs-shipped-artifact parity trap). CRT-CVE-rebuild tradeoff accepted.

---

## Root-cert trust install

### Machine store mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| Deferred custom action (certutil) | SYSTEM-context CA: `certutil -addstore Root` + `TrustedPublisher`; explicit, debuggable, non-fatal. | ✓ |
| WiX CertificateRef element | Declarative WiX util/iis Certificate into LocalMachine\Root. Finicky/IIS-bound; hard to also hit TrustedPublisher. | |
| nono setup --provision-fleet | MSI drops .cer; nono binary imports (Rust/testable, reused per-user). Runs freshly-installed exe during install. | |

**User's choice:** Deferred custom action (certutil) → `Root` + `TrustedPublisher`

### Per-user + Node reach

| Option | Description | Selected |
|--------|-------------|----------|
| First-run import + NODE_EXTRA_CA_CERTS | Reuse DEPLOY-03 first-run hook: idempotent CurrentUser\Root import + NODE_EXTRA_CA_CERTS → shipped PEM. One provisioning path. | ✓ |
| ActiveSetup per-user import | HKLM Active Setup stanza runs import at each user's first logon. Separate mechanism; Node still handled elsewhere. | |
| Machine-root only + document | Rely on LocalMachine\Root + set NODE_EXTRA_CA_CERTS; skip CurrentUser\Root. Contradicts success criterion 4. | |

**User's choice:** First-run import + NODE_EXTRA_CA_CERTS
**Notes:** Unifies with scratch provisioning into one first-run-in-user-context provisioner (D-09).

---

## nono health schema

### Exit-code contract

| Option | Description | Selected |
|--------|-------------|----------|
| 0 healthy / 1 degraded / 2 broken | Tri-state; JSON always on stdout. Lets scripts distinguish remediate-later from reinstall-now. | ✓ |
| 0 healthy / non-zero otherwise | Binary; detail in JSON. Simplest; matches success-criterion wording. | |
| Always exit 0 (JSON only) | Status only in JSON `status` field. Contradicts success criterion 5. | |

**User's choice:** 0 healthy / 1 degraded / 2 broken

### Inspected subsystems (multi-select)

| Option | Description | Selected |
|--------|-------------|----------|
| Install + version | INSTALLFOLDER, self-locate, version + ProductCode/UpgradeCode → broken/exit 2. | ✓ |
| WFP service state | SCM install + running state → degraded/exit 1. | ✓ |
| Machine policy state | HKLM\SOFTWARE\Policies\nono presence/readability (Phase 83 forward-looking). | ✓ |
| Scratch + cert + PATH | User-owned scratch, POC cert both stores, machine PATH entry. | ✓ |

**User's choice:** All four subsystem groups

---

## Scratch provisioning

| Option | Description | Selected |
|--------|-------------|----------|
| Auto on first run, %LOCALAPPDATA%\nono | First `nono run` auto-creates user-owned WRITE_OWNER scratch; idempotent; shares first-run hook with cert. | ✓ |
| Auto on first run, %USERPROFILE%\.nono | Same trigger, dotfolder root. Both support WRITE_OWNER; LOCALAPPDATA more conventional. | |
| Explicit nono setup | Predictable timing but reintroduces the manual step DEPLOY-03 eliminates. | |

**User's choice:** Auto on first run, %LOCALAPPDATA%\nono

---

## Claude's Discretion

- Exact `nono health` JSON field names/shape within the tri-state contract.
- MSI custom-action sequencing/conditions for cert import; PEM/CER staging into WiX harvest.
- Whether to register the `nono` CLI Event Log source now (Phase 84 prereq) or defer to Phase 84.
- First-run idempotency marker mechanism (registry sentinel vs filesystem marker).

## Deferred Ideas

- Real publicly-trusted code signing (Azure Trusted Signing / DIST-SIGN-01) — future distribution milestone.
- MSIX packaging — permanently out of scope.
- Live machine-policy reload (EGRESS-FU-01) — v3.x.
- `macos-rlimit-as-setrlimit-fails` todo — reviewed, not folded (unrelated macOS defect).
