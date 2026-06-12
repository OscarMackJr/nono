# Phase 67: Clean-Host Windows Install - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-12
**Phase:** 67-clean-host-windows-install
**Areas discussed:** VC++ runtime strategy, Service-start posture, trust-broker helper scope & safety, Cert shipping & docs framing

---

## VC++ Runtime Strategy (DIST-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Static CRT (`+crt-static`) | Build nono.exe + nono-wfp-service.exe with the static CRT — eliminates the runtime dependency entirely (no redist, any clean host, ideal for future silent fleet deploy). Slightly larger binaries; must verify no workspace crate breaks. | ✓ |
| Bundle VC++ merge module | Add the Microsoft VC143 CRT merge module to the WiX MSI. Standard, but adds MSI size + pins a redist version. | (fallback) |
| Declared prereq + non-fatal only | Don't bundle anything; rely on DIST-02 + document the redist prereq. | |

**User's choice:** Static CRT (`+crt-static`).
**Notes:** Bundle-merge-module retained as the documented contingency if static CRT breaks a workspace crate (ring/aws-lc-sys etc.) or disturbs Authenticode signing. The "declared prereq" option was rejected.

---

## Service-Start Posture (DIST-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-start, but non-fatal | Keep Start=auto (out-of-box WFP enforcement) but make install-time start non-fatal (Vital=no / drop blocking ServiceControl Start) so a start hiccup leaves a usable nono.exe. | ✓ |
| Switch to demand-start | Don't start from installer; nono starts the service on first run. Install never blocks — but WFP inactive until first nono run. | |

**User's choice:** Auto-start, but non-fatal.
**Notes:** Preserves Phase 62's out-of-box WFP value. Clean-uninstall invariant must survive the change. With static CRT, the missing-CRT trigger for the rollback is already gone; DIST-02 ships as general robustness.

---

## Broker Trust Helper (TRUST-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Pin one cert, self-elevate, reversible | Import ONLY the shipped POC cert (pinned by thumbprint) into LocalMachine Root + TrustedPublisher, self-elevate via UAC, paired `--untrust-broker`. Scoped, auditable, reversible. | ✓ |
| Same, but require elevated shell | Identical scope, no UAC self-elevation; errors unless already elevated. | |
| Print manual instructions only | nono never touches trust stores; just prints certutil/Import-Certificate commands. | |

**User's choice:** Pin one cert, self-elevate, reversible.
**Notes:** Hard invariant — never weakens/bypasses the D-32-12 gate; trusts only the one POC cert; auditable + reversible.

---

## Cert Shipping & Discoverability (TRUST-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Cert in MSI + check-only warning | Ship the .cer inside the MSI payload (--trust-broker works offline), `setup --check-only` warns when the broker cert is untrusted, docs prominently flag the POC-cert limitation. | ✓ |
| Cert as release asset only | Ship the .cer as a GitHub release asset (not in MSI) + docs + check-only warning. | |
| Docs only, no shipped cert | Document how to extract the cert from the signed binary; ship nothing. | |

**User's choice:** Cert in MSI + check-only warning.
**Notes:** Best clean-host UX (offline-capable). Docs state plainly: POC cert for the supervised path until `DIST-SIGN-01` (real signing) lands.

---

## Claude's Discretion

- Exact WiX attribute combination for non-fatal start (`Vital`/`ErrorControl`/dropping `Start="install"`) — minimal change satisfying D-04/D-05, validated against `validate-windows-msi-contract.ps1`.
- Static-CRT wiring mechanism (`.cargo/config.toml` rustflags vs per-target vs release-script `RUSTFLAGS`) — pick what `release.yml` carries cleanly across all build legs.
- Self-elevation + cert-import API (`certutil` shell-out vs `windows-sys` `CertAddCertificateContextToStore`) — security-equivalent; pick the more testable.

## Deferred Ideas

- Real publicly-trusted code signing (Azure Trusted Signing) — `DIST-SIGN-01`, BLOCKED on incoming cert, next (enterprise) milestone.
- Silent/headless fleet deployment (GPO/SCCM/Intune, MSIX) — SEED-001, enterprise milestone.
- `macos-resl-enforcement-broken` todo — fuzzy-matched but belongs to Phase 68 (already linked there); not folded.
