# Requirements: nono v3.0 Enterprise Hardening I (Deploy · Control · Compliance)

**Defined:** 2026-06-18
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and that confinement must apply to *any* AI agent engine, deployable and governable across a corporate Windows fleet.

**Verification standard (Dark Factory, carried from v2.13):** every requirement below ships an unattended `scripts/verify-dark.ps1` scripted gate emitting a machine-readable verdict. True fleet/SIEM/EDR live UAT (domain-joined push, SIEM ingestion, cloud-EDR) is acknowledged host-gated tech-debt and validated by the scripted gate on a single dev host plus operator-gated live runs. The per-theme capstone gates are `deploy-silent-install`, `egress-policy-deny`, and `telemetry-event-emit`; the milestone closes on the no-flag `verify-dark.ps1` aggregator.

**Decided scope locks:**
- Stay on **WiX MSI**; MSIX is out of scope (cannot package the LocalSystem WFP service / kernel driver).
- Code-signing uses the **POC/test-cert path** (`nono setup --trust-broker`); real Azure Trusted Signing is deferred to a distribution milestone.
- Machine policy lives at **`HKLM\SOFTWARE\Policies\nono`**, read **at startup, restart-to-apply** (no live-reload this cycle), with per-user profile as fallback.
- Telemetry sink is **Windows Event Log only** this cycle (Syslog deferred).
- Tamper-evidence is an **in-session HMAC-SHA256 chain** honestly scoped to Windows Event Forwarding; cross-session / cryptographic-local anchoring is deferred to SEED-005.

---

## v1 Requirements

Requirements for this milestone. Each maps to exactly one roadmap phase.

### Deployment (SEED-001, P0)

- [ ] **DEPLOY-01**: An admin can install nono fleet-wide silently via `msiexec /i nono.msi /qn /norestart` with no interactive prompts, correct MSI exit codes (0 / 3010 reboot-required / 1603 failure), and SYSTEM-context safety (no dependence on a per-user profile path).
- [ ] **DEPLOY-02**: The machine MSI registers nono on the machine-wide `PATH` so any user can invoke `nono` with no per-user setup.
- [ ] **DEPLOY-03**: nono auto-provisions a user-owned `WRITE_OWNER` scratch workspace at first run in user context (MSI provisions only `%PROGRAMDATA%\nono\`), eliminating the manual profile-owned-CWD requirement and the SYSTEM-owned-scratch R-B3 failure.
- [ ] **DEPLOY-04**: An admin can push nono machine policy to a fleet via a shipped GPO **ADMX/.adml** template (and a documented Intune OMA-URI / `ADMXInstall` path) targeting `HKLM\SOFTWARE\Policies\nono`.
- [ ] **DEPLOY-05**: The machine MSI silently installs the POC root certificate into the machine trust store so the signed-broker / supervised path works on a clean host with no manual cert import.
- [ ] **DEPLOY-06**: Service install is atomic and non-fatal — a `nono-wfp-service` start failure does not roll back the whole product, and `nono health` reports install + service + policy state for fleet diagnostics.

### Machine Policy Spine (POLICY)

- [ ] **POLICY-01**: nono reads machine policy from `HKLM\SOFTWARE\Policies\nono` at process/daemon startup using the 64-bit view (`KEY_WOW64_64KEY`); when the policy key is present it takes precedence over the per-user profile.
- [ ] **POLICY-02**: A failure to read a *present* machine-policy key **fails secure** (deny / abort with a typed `NonoError`), never falling open to a less-restrictive state.
- [ ] **POLICY-03**: The egress allowlist and telemetry configuration are deserialized from this **single** policy source, so no two enforcement layers can read divergent config (drift is structurally prevented).

### Egress Control (SEED-002, P1)

- [ ] **EGRESS-01**: An admin can define a deny-by-default outbound egress allowlist in machine policy as wildcard FQDNs (`REG_MULTI_SZ`, e.g. `*.anthropic.com`); only listed domains are reachable and the policy's presence switches enforcement on.
- [ ] **EGRESS-02**: The machine-policy allowlist is enforced by **both** the `nono-proxy` domain filter and the kernel `nono-wfp-service` path from the same deserialized source, verified at both layers.
- [ ] **EGRESS-03**: Wildcard FQDN matching uses DNS-component comparison so `*.x.com` matches `api.x.com` but **not** `x.com` or `evilx.com`; any matching ambiguity fails secure (deny).
- [ ] **EGRESS-04**: nono ships AI-provider allowlist presets (e.g. `*.anthropic.com`, `*.openai.com`, `api.github.com`) so a default-deny posture does not brick the agent's required provider traffic.

### Compliance / Telemetry (SEED-003, P2)

- [ ] **TELEM-01**: Blocked/denied actions (path-deny, network-deny, label-violation, hook fail-closed) are emitted as structured security events to a custom **Application-tier Windows Event Log** channel with distinct EventIDs and named `EventData` fields that Splunk (`XmlWinEventLog`) and Microsoft Sentinel parse as columns.
- [ ] **TELEM-02**: Security events carry an in-session **HMAC-SHA256** chain (`ChainHead` exposed as a named field) for tamper-evidence; the tamper boundary is documented as Windows Event Forwarding, with cross-session/cryptographic-local anchoring explicitly deferred to SEED-005 (ADR recorded).
- [ ] **TELEM-03**: Telemetry emission redacts secrets, tokens, and sensitive payload content from event fields — a blocked-action event never leaks credentials or full secret values into the log.
- [ ] **TELEM-04**: The emitter is implemented in `nono-cli` as a `tracing::Layer` (not in the `nono` library's `DiagnosticFormatter`), preserving the library/CLI boundary; its enable/channel/level config is read from machine policy.

---

## v2 Requirements

Deferred to future release. Tracked, not in this roadmap.

### Egress Control

- **EGRESS-FU-01**: Live machine-policy reload mid-session via `RegNotifyChangeKeyValue` (this cycle is read-at-startup, restart-to-apply).

### Compliance / Telemetry

- **TELEM-FU-01**: RFC 5424 **Syslog** emission for non-Windows / network SIEM ingestion (this cycle is Windows Event Log only).
- **TELEM-FU-02**: Real-time direct SIEM API push (this cycle relies on Windows Event Forwarding / agent collection).

### Attestation (SEED-005 — its own later milestone)

- **ATTEST-01**: Cryptographically signed, revocable policy-exception overrides verified against the ZT-Infra v2 ledger, mutating the runtime ruleset for a bound repo context.
- **ATTEST-02**: Cross-session / cryptographic-local tamper-evident audit ledger anchoring (extends TELEM-02 beyond the WEF boundary).

---

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| MSIX packaging | Cannot package the LocalSystem `nono-wfp-service` / kernel driver; conflicts with the existing WiX MSI service-registration boundary. Stay on WiX MSI. |
| Real publicly-trusted code signing (Azure Trusted Signing) | Cert-gated; the anchor of a separate distribution milestone. POC/test-cert path used this cycle. |
| Syslog / RFC 5424 emission | Deferred (TELEM-FU-01); the two named SIEMs (Splunk, Sentinel) ingest the Windows Event Log channel directly. |
| Live machine-policy reload | Deferred (EGRESS-FU-01); read-at-startup/restart-to-apply is sufficient and avoids TOCTOU/reload-race surface this cycle. |
| ZT-Infra signed policy overrides (SEED-005) | X-Large with an external ledger dependency; its own later standalone milestone, sequenced after the audit pipeline ships. |
| UPST9 upstream sync | Separate upstream-sync cadence; not bundled into the enterprise arc. |
| DRV-PROD-01 production kernel minifilter / WR-02 cloud-EDR re-run | v3.0+ deferrals re-affirmed (ADR-65 No-go/Conditional-go). |
| IP-based egress allowlisting | Anti-feature per research; FQDN wildcard allowlisting is the convention. |

---

## Traceability

Which phases cover which requirements.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DEPLOY-01 | Phase 82 | Pending |
| DEPLOY-02 | Phase 82 | Pending |
| DEPLOY-03 | Phase 82 | Pending |
| DEPLOY-04 | Phase 82 (template) / Phase 83 (reader) | Pending |
| DEPLOY-05 | Phase 82 | Pending |
| DEPLOY-06 | Phase 82 | Pending |
| POLICY-01 | Phase 83 | Pending |
| POLICY-02 | Phase 83 | Pending |
| POLICY-03 | Phase 83 | Pending |
| EGRESS-01 | Phase 83 | Pending |
| EGRESS-02 | Phase 83 | Pending |
| EGRESS-03 | Phase 83 | Pending |
| EGRESS-04 | Phase 83 | Pending |
| TELEM-01 | Phase 84 | Pending |
| TELEM-02 | Phase 84 | Pending |
| TELEM-03 | Phase 84 | Pending |
| TELEM-04 | Phase 84 | Pending |
