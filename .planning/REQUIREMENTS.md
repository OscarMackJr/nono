# Requirements: nono — v2.13 Carry-Forward Closeout (Dark Factory)

**Defined:** 2026-06-16
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and that confinement must apply to *any* AI agent engine, not just Claude Code.

> **Milestone framing.** v2.13 closes the v2.12 confinement long-tail with **self-verifying automation**: every historically host-gated item must collapse to a single unattended scripted run ("Dark Factory" mandate). Building that harness is in-scope work, not nice-to-have. Phase numbering continues from Phase 75 (Phase 76+). No UPST9; enterprise seeds (001/002/003/005) deferred to a separate enterprise milestone.

## v1 Requirements

Requirements for this milestone. Each maps to exactly one roadmap phase.

### Copilot End-to-End Confinement

- [ ] **CPLT-01**: nono grants ancestor-chain `FILE_READ_ATTRIBUTES` (RA) up to the drive root for a confined target's package SID, so Node-ESM module resolution (`realpathSync`/`lstat` walking every path ancestor) succeeds under AppContainer instead of being denied.
- [ ] **CPLT-02**: An idempotent one-time-admin setup step grants the package-SID RA on the system ancestors (`C:\`, `C:\Users`) that nono cannot ACL at runtime — documented and verified non-destructive.
- [ ] **CPLT-03**: GitHub Copilot CLI completes a real task end-to-end under confinement (no longer confine-only), proven by an unattended scripted gate.

### Cross-Process Classification

- [x] **CLAS-01**: An operator can authoritatively classify any running PID as `AI_AGENT` (or not) via `nono classify <pid>`, answered cross-process by the daemon control-pipe `Classify` verb.
- [x] **CLAS-02**: The `Classify` verb is caller-gated and tenant-safe — same least-privilege/SDDL posture as the existing control pipe, with no cross-tenant disclosure.

### WFP Egress Isolation Proof

- [x] **WFP-01**: Per-agent WFP egress isolation is empirically proven by an automated test — one confined agent's allowed egress succeeds while a second agent (distinct package SID) is denied on the same host.

### nono-ts Ergonomics

- [x] **TSRG-01**: `confinedRun` in nono-ts defaults to the Low-IL broker arm and auto-covers the target executable's directory, so a caller gets a working confined run with no manual profile/coverage flags.

### Clean-Host Install UAT

- [ ] **INST-01**: The machine MSI installs and runs on a clean Win11 host with no manual steps, verified by an unattended clean-host harness.

### Self-Verifying Harness (Dark Factory)

- [x] **DARK-01**: Each host-gated verification (Copilot end-to-end, WFP isolation, clean-host install) ships as a single-invocation unattended script emitting a machine-readable pass/fail verdict — replacing interactive human UAT.
- [ ] **DARK-02**: A milestone-close aggregator collects the per-item verdicts so v2.13 completion is evaluable from harness output alone (no human interpretation step).

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Enterprise Hardening (seeds — separate milestone)

- **ENT-DEPLOY**: Silent/headless enterprise deployment — GPO/SCCM/Intune push, machine-wide service, invariant env vars, auto-provisioned scratch space (SEED-001).
- **ENT-EGRESS**: Machine-policy-managed deny-by-default corporate domain allowlist; reconcile `nono-proxy` + `nono-wfp-service` into one egress story (SEED-002).
- **ENT-SIEM**: Blocked actions → structured Windows Event Log / Syslog telemetry, tamper-evident chain (SEED-003).
- **ENT-ATTEST**: Signed policy overrides + ZT-Infra immutable-ledger attestation (SEED-005; external dependency).

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| UPST9 upstream divergence audit + cherry-pick sync | v2.13 is tightly scoped to v2.12 carry-forward debt; next upstream sync deferred to its own cadence. |
| Enterprise-hardening seeds (001/002/003/005) | A separate enterprise distribution milestone; not carry-forward debt. |
| Azure Trusted Signing / real publicly-trusted code signing | Cert-gated; anchors the future enterprise milestone. |
| WR-02 EDR HUMAN-UAT | Long-standing v3.0 deferral (re-affirmed every milestone since v2.1). |
| Gap 6b production EV/WHQL kernel minifilter (DRV-PROD-01) | Gated No-go/Conditional-go per ADR-65; future milestone. |
| Interactive human UAT as a verification mechanism | Replaced by the DARK self-verifying harness mandate; human touch reduced to a single unattended invocation. |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CPLT-01 | Phase 77 | Pending |
| CPLT-02 | Phase 77 | Pending |
| CPLT-03 | Phase 77 | Pending |
| CLAS-01 | Phase 78 | Complete |
| CLAS-02 | Phase 78 | Complete |
| WFP-01 | Phase 79 | Complete |
| TSRG-01 | Phase 79 | Complete |
| INST-01 | Phase 80 | Pending |
| DARK-01 | Phase 76 | Complete |
| DARK-02 | Phase 81 | Pending |

**Coverage:**
- v1 requirements: 10 total
- Mapped to phases: 10 (roadmap complete)
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-16*
*Last updated: 2026-06-17 — traceability filled by roadmapper (all 10 requirements mapped to Phases 76-81)*
