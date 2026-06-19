# Requirements: nono v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain

**Defined:** 2026-06-19
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — kept current with upstream `always-further/nono` without regressing the fork's Windows security model.

**Scope source:** `.planning/seeds/SEED-006-upst9-v0.62-v0.64-sync-window.md` (the function-level scoping of the 90-commit / 140-file `v0.62.0..v0.64.0` window).

**Verification standard:**
- **Sync requirements** are verified by `make ci` (clippy + fmt + tests) on Windows **plus** the mandatory cross-target clippy on Linux + macOS CI for any cfg-gated Unix edit (`feedback_clippy_cross_target`), and by the relevant upstream test matrix where one was ported.
- **Drain requirements** carry the Dark Factory mandate forward: each ships an unattended `scripts/verify-dark.ps1` scripted gate; true live UAT (clean-VM, dual-layer WFP, live SIEM) remains acknowledged **host-gated tech-debt** validated on a single dev host plus operator-gated live runs.

**Decided scope locks:**
- **Full-sync / adopt-upstream** for the two HIGH-conflict refactors — absorb upstream's relocation of the audit stack and the structured-diagnostics model **into the core `nono` crate**, converging toward upstream's layout (not fork-preserving CLI-side).
- **Drain-then-sync** shape (mirrors v2.5/v2.6/v2.8 UPST5/6/7): fold the v3.0 host-gated UAT debt into this milestone.
- **Milestone-marker only** — no crate publish this cycle (tag like `v2.13`/`v2.10`). A future release must still leapfrog the crate version to ≥ `0.65.0` (upstream is now `0.64.0`).
- **Window** = upstream `v0.62.0..v0.64.0`; **re-fetch upstream at audit-open** and extend to any `v0.65.0` cut before then.
- **Windows-only-files invariant (D-43-E1)** holds — none of the upstream-changed files are Windows backends, so the fork's Windows surface is largely untouched by the cherry-picks themselves.

---

## v1 Requirements

Requirements for this milestone. Each maps to exactly one roadmap phase.

### UPST9 Divergence Audit (AUDIT)

- [ ] **AUDIT-01**: A `DIVERGENCE-LEDGER.md` exists for upstream `v0.62.0..v0.64.0` (Phase 42/47/48 shape) classifying every substantive commit into themed clusters, each with a `will-sync` / `fork-preserve` / `split` / `won't-sync` disposition; re-fetched at audit-open to capture any `v0.65.0`.
- [ ] **AUDIT-02**: Each cluster disposition carries an ADR-style L/M/H risk verdict, and cross-cluster re-export dependencies are diff-inspected (not `--name-only`), structurally closing the `feedback_cluster_isolation_invalid` hazard.

### Library-Boundary Convergence (BND)

- [ ] **BND-01**: The audit/attestation/ledger logic is relocated into the core `nono` crate (`crates/nono/src/audit.rs`) matching upstream, with `nono-cli` reduced to thin wrappers; all existing audit behavior is preserved and tested (recorder lifecycle, merkle/inclusion-proof, ledger append+verify, attestation sign/verify).
- [ ] **BND-02**: The structured-diagnostics model is adopted into the core `nono` crate (`crates/nono/src/diagnostic/*`, `NonoError::{diagnostic_code, remediation}`) and exposed via FFI (`bindings/c/src/diagnostic.rs`, `NonoDiagnosticCode`, `last_diagnostic_code`/`last_remediation_json`), reconciled with the fork's Windows diagnostic paths and the proxy `ProxyDiagnostic` surface.
- [ ] **BND-03**: `CLAUDE.md` § Library vs CLI boundary is updated to reflect the new core-crate audit + diagnostics modules, and an ADR records the boundary-convergence decision, rationale, and what stays CLI-side.

### Security Sync (SEC)

- [ ] **SEC-01**: The Linux AF_UNIX datagram bypass is closed — `sendto`/`sendmsg`/`sendmmsg` are trapped by the seccomp filter and gated on a connect grant (upstream #1096), verified by the ported test matrix on Linux CI.
- [ ] **SEC-02**: `deduplicate()` no longer inherits procfs-remap originals (upstream #1064 guard), preserving `/dev/null` when deduped with `/dev/stdin`.

### Feature Sync (FEAT)

- [ ] **FEAT-01**: `set_vars` static env injection is available (profile + `CapabilitySet`), with env-name validation that rejects `PATH` and the `NONO_` prefix (upstream #1134).
- [ ] **FEAT-02**: Runtime state (audit, sessions, rollback) resolves under XDG state dirs with legacy `~/.nono` fallback + one-time migration (upstream #1152 / #1179); Windows path resolution verified against the v3.0 scratch-space provisioner.
- [ ] **FEAT-03**: AWS auth config (`AwsAuthConfig`) is accepted and validated in profiles + proxy route config, mutually exclusive with `credential_key` / `oauth2` (upstream #1166).
- [ ] **FEAT-04**: Keyring access honors `NONO_KEYRING_TIMEOUT_SECS` (default 120s, `0` = no timeout) (upstream #977).
- [ ] **FEAT-05**: `$PACK_DIR` store-pack session hooks resolve with `source_pack` propagation (upstream #1073).
- [ ] **FEAT-06**: update-check reports CI provider/environment discovery (upstream #1113), profile names are standardized to the namespace, and bool CLI flags accept truthy env values (upstream #1136).

### Proxy Sync (PROXY)

- [ ] **PROXY-01**: `allow_domain` endpoint routes no longer shadow the credential catch-all (upstream #1132), and denied non-CONNECT requests return `403` + an audit record (upstream #1077).
- [ ] **PROXY-02**: The TLS CONNECT intercept respects `upstream_proxy` (upstream #1048 / #1091), reactive proxy auth keeps the connection open on CONNECT (upstream #1151), and the proxy activates when `customCredentials` is set (upstream #1197) — all reconciled against the fork-divergent TLS-interception surface (Phase 34 C11 `fork-preserve`).

### PTY + Dependency Sync (DEPS)

- [ ] **DEPS-01**: PTY ctrl-z suspend/resume no longer hangs when running under a PTY (upstream #1135).
- [ ] **DEPS-02**: Workspace dependency bumps from the window are absorbed across all 5 crates with internal path-dep version pins synced (`project_workspace_crates`): x509-parser 0.18.1, hyper 1.10.1, cbindgen 0.29.4, typify 0.7.0, zeroize 1.9.0, time 0.3.49, chrono 0.4.45, ignore 0.4.26, which 8.0.3.

### v3.0 Host-Gated UAT Drain (DRAIN)

- [ ] **DRAIN-01**: Clean-VM silent MSI install UAT (v3.0 DEPLOY-01/03/05) is executed on a fresh Win11 host with recorded verdicts, OR collapsed to a single unattended `verify-dark.ps1` gate with the residual live step explicitly host-gated.
- [ ] **DRAIN-02**: Dual-layer (proxy + kernel WFP) live egress-block proof (v3.0 EGRESS-02) is recorded via its scripted gate; the live-host step is operator-gated.
- [ ] **DRAIN-03**: Live SIEM telemetry gate plus admin opt-out / `min_severity` HKLM→emit (v3.0 TELEM-01/04) is verified via the `telemetry-event-emit` gate; live SIEM ingestion remains host-gated.
- [ ] **DRAIN-04**: Daemon-side telemetry emission is wired — `nono-agentd` registers the `SecurityEventLayer` so daemon-launched agent denials emit `nono_security::*` events (real code, not host-gated).

---

## Future Requirements (deferred)

- **Tagged signed crate release** — leapfrog the crate version to ≥ `0.65.0` and exercise `release.yml` + signing. Deferred: this milestone is marker-only.
- **UPST10** — the next upstream-sync window (`v0.64.0`+). Separate cadence.

## Out of Scope (explicit exclusions)

- **SEED-005 ZT-Infra signed policy overrides / decentralized attestation** — X-Large with an external ledger dependency; its own later standalone milestone.
- **Distribution / real publicly-trusted code signing (Azure Trusted Signing)** — cert-gated; the anchor of a separate distribution milestone.
- **DRV-PROD-01 production kernel minifilter / WR-02 cloud-EDR re-run** — long-standing deferrals, re-affirmed (gated No-go/Conditional-go per ADR-65).
- **macOS-only upstream items beyond the `v0.62.0..v0.64.0` window** — v2.10 already brought Seatbelt to `v0.61.2`; further macOS-only drift stays on its own cadence.

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| _(filled by roadmap)_ | | |
