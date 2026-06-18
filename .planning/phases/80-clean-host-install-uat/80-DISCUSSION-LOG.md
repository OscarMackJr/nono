# Phase 80: Clean-Host Install UAT - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-18
**Phase:** 80-clean-host-install-uat
**Areas discussed:** Clean-host acquisition, VC++/install-rollback fix, Cert/broker scope, PASS scope, Clean-host detection criteria, MSI artifact source

---

## Clean-host acquisition

| Option | Description | Selected |
|--------|-------------|----------|
| SKIP-on-dirty + fresh VM | Test-Precondition detects clean host; SKIP if dirty; PASS only on operator-provided fresh Win11 VM/snapshot | ✓ |
| Windows Sandbox auto-run | Gate auto-launches disposable Sandbox, installs + verifies inside, reads verdict from mapped folder | |
| Packaging-inspection proxy | No real install; inspect MSI + binaries (dumpbin imports, ServiceControl flags) | |

**User's choice:** SKIP-on-dirty + fresh VM
**Notes:** Windows Sandbox is `Disabled` on the dev host (needs admin + reboot). SKIP pattern matches the Phase 76 contract and keeps the gate host-agnostic; the dev (dirty) host emits SKIP_HOST_UNAVAILABLE, the fresh VM emits PASS.

---

## VC++ / install-rollback fix

| Option | Description | Selected |
|--------|-------------|----------|
| Static CRT (+crt-static) | Build all binaries with target-feature=+crt-static; no redist needed | |
| Bundle VC++ redist in MSI | Chain the redistributable install into the machine MSI | |
| Both: static CRT + non-fatal service | Static-CRT the binaries AND make nono-wfp-service start non-fatal to the install | ✓ |

**User's choice:** Both: static CRT + non-fatal service
**Notes:** Belt-and-suspenders — binaries always load (no vcruntime140 dependency) AND a service-start failure never rolls back the product (SC #2). Closes the `1603`/`0xC0000135`/SCM-7009 rollback finding.

---

## Cert / broker scope

| Option | Description | Selected |
|--------|-------------|----------|
| Install-level only | Assert install exit 0 + nono --version; broker untrusted-cert failure = known deferred limitation (DIST-SIGN-01) | ✓ |
| Also verify broker w/ trust helper | Exercise broker after a `nono setup --trust-broker` / cert-import step | |
| Include broker as a SKIP | Run broker path, classify untrusted-cert refusal as SKIP_HOST_UNAVAILABLE | |

**User's choice:** Install-level only
**Notes:** Public signing is deferred to the enterprise milestone; the broker/supervised path is out of scope for this gate. Matches the ROADMAP Phase 80 success criteria exactly.

---

## PASS scope + cleanup

| Option | Description | Selected |
|--------|-------------|----------|
| Install + run + non-fatal svc + cleanup | msiexec exit 0 + nono --version (new shell) + non-fatal service + loads w/o VC++ redist; uninstall after | ✓ |
| Install + run only | Just install exit 0 + nono --version | |
| Install + run + non-fatal svc, no cleanup | Assert install + run + non-fatal service but leave nono installed | |

**User's choice:** Install + run + non-fatal svc + cleanup
**Notes:** Full INST-01 coverage; uninstall makes a re-run on the same VM snapshot repeatable.

---

## Clean-host detection criteria

| Option | Description | Selected |
|--------|-------------|----------|
| No prior nono only | Clean = no nono.exe in Program Files\nono AND no nono-wfp-service/nono-agentd services | ✓ |
| No prior nono AND no VC++ runtime | Also require vcruntime140 absent | |
| No prior nono + explicit marker | Also require an operator-set marker (env var / sentinel file) | |

**User's choice:** No prior nono only
**Notes:** With static CRT the VC++ runtime isn't needed, and a fresh Win11 image may legitimately already have VC++. The condition that matters is "never had nono installed."

---

## MSI artifact source

| Option | Description | Selected |
|--------|-------------|----------|
| Path param, default dist location | -MsiPath param defaulting to dist\windows\nono-machine.msi (local unsigned build) | ✓ |
| Fixed known path on the VM | Gate expects MSI at a fixed convention path; no param | |
| CI-signed release MSI | Install the CI-signed release MSI | |

**User's choice:** Path param, default dist location
**Notes:** Unsigned local build is fine (cert/trust is out of scope per the install-level decision); CI-signed release uses the POC cert which only matters for the out-of-scope broker path.

---

## Claude's Discretion

- Exact `detail` JSON fields (install exit code, service state, `nono --version` output, optional dumpbin no-vcruntime import check).
- Precise WiX attribute for the non-fatal service (`vital="no"`) and the `.cargo/config.toml`/RUSTFLAGS stanza for `+crt-static` (verify against live build).
- Whether to add a dumpbin/`link /dump /imports` assertion reinforcing the static-CRT proof.

## Deferred Ideas

- Publicly-trusted code signing (DIST-SIGN-01) — enterprise milestone; broker path stays non-functional out-of-box on a clean host until then.
- Windows Sandbox auto-run of the gate (self-provisioning clean env) — future ergonomics.
- `nono setup --trust-broker` / POC-cert import helper — would make the broker path testable on a clean host; out for this phase.
