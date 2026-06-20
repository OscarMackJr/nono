---
milestone: v3.1
milestone_name: UPST9 Upstream Sync (v0.62-v0.64) + v3.0 Drain
status: planning
created: 2026-06-19
---

# Roadmap: nono — v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain

## Overview

A drain-then-sync upstream-sync milestone (mirrors v2.5/v2.6/v2.8). Audit and *fully absorb* the upstream `always-further/nono` `v0.62.0..v0.64.0` window — 90 commits / 140 files — converging toward upstream's layout (audit-stack + structured-diagnostics relocated into the core `nono` crate, adopt-upstream) while preserving the fork's Windows security model; then drain v3.0's host-gated UAT debt. The audit (`DIVERGENCE-LEDGER.md`) comes first and gates every cherry-pick disposition. The heavy, highest-risk library-boundary convergence lands next (it touches FFI + diagnostics that downstream sync work depends on), followed by the security fix, the additive feature/dependency wave, and the proxy hardening cluster against the fork-divergent TLS-interception surface. The v3.0 host-gated UAT drain is independent and runs last. Milestone-marker only — no crate publish; a future release leapfrogs the crate version to ≥ `0.65.0`.

## Phases

**Phase Numbering:**
- Integer phases (85, 86, 87): Planned milestone work — **continued from Phase 84** (v3.0). NOT reset.
- Decimal phases (86.1, 86.2): Urgent insertions (marked with INSERTED)

- [x] **Phase 85: UPST9 Divergence Audit** - Build the `v0.62.0..v0.64.0` DIVERGENCE-LEDGER with per-cluster dispositions + ADR risk verdicts (completed 2026-06-19)
- [x] **Phase 86: Library-Boundary Convergence** - Adopt upstream's core-crate audit stack + structured-diagnostics model (lib + FFI + proxy), reconcile Windows paths, update CLAUDE.md + ADR (completed 2026-06-20)
- [x] **Phase 87: Security Sync** - Close the Linux AF_UNIX datagram bypass (#1096) and the procfs-remap dedup leak (#1064) (completed 2026-06-20)
- [ ] **Phase 88: Feature + Dependency Cherry-Pick Wave** - Absorb the additive feature cherry-picks + PTY ctrl-z fix + 9 dependency bumps across all 5 crates
- [ ] **Phase 89: Proxy Hardening Sync** - Absorb the proxy route/403/TLS-CONNECT/reactive-auth/customCredentials cluster against the fork-divergent TLS-interception surface
- [ ] **Phase 90: v3.0 Host-Gated UAT Drain** - Land daemon-side telemetry emission + the verify-dark.ps1 scripted gates for clean-VM install, dual-layer WFP block, and live SIEM

## Phase Details

### Phase 85: UPST9 Divergence Audit
**Goal**: A complete, disposition-resolved DIVERGENCE-LEDGER for upstream `v0.62.0..v0.64.0` that gates every downstream cherry-pick decision.
**Depends on**: Nothing (first phase of v3.1; follows Phase 84)
**Requirements**: AUDIT-01, AUDIT-02
**Success Criteria** (what must be TRUE):
  1. `DIVERGENCE-LEDGER.md` exists for `v0.62.0..v0.64.0` (Phase 42/47/48 shape) classifying every substantive commit into themed clusters (A–M from SEED-006), each carrying a `will-sync` / `fork-preserve` / `split` / `won't-sync` disposition.
  2. Upstream was re-fetched at audit-open and the ledger reflects any `v0.65.0` cut landing before then (window extended if so).
  3. Each cluster disposition carries an ADR-style L/M/H risk verdict across the standard dimensions (security, windows, maintenance, divergence, contributor).
  4. Cross-cluster re-export dependencies are diff-inspected (actual diffs, not `git --name-only`), structurally closing the `feedback_cluster_isolation_invalid` hazard — with the two HIGH-conflict refactors (themes A & B) explicitly dispositioned `will-sync / adopt-upstream`.
**Plans**: 1 plan
Plans:
- [x] 85-01-PLAN.md — Enumerate commits (clusters A–M), actual-diff re-export inspection (A/B/F), ADR review, empirical cross-check, produce 85-DIVERGENCE-LEDGER.md

### Phase 86: Library-Boundary Convergence
**Goal**: The audit/attestation/ledger stack and the structured-diagnostics model live in the core `nono` crate matching upstream, with the fork's Windows diagnostic paths and FFI reconciled and the policy-free-library invariant re-decided via ADR.
**Depends on**: Phase 85 (audit dispositions for themes A & B must be resolved first)
**Requirements**: BND-01, BND-02, BND-03
**Success Criteria** (what must be TRUE):
  1. Audit/attestation/ledger logic resides in `crates/nono/src/audit.rs` (matching upstream), `nono-cli` is reduced to thin wrappers, and all existing audit behavior passes its tests (recorder lifecycle, merkle/inclusion-proof, ledger append+verify, attestation sign/verify).
  2. The structured-diagnostics model exists in `crates/nono/src/diagnostic/*` with `NonoError::{diagnostic_code, remediation}`, and is exposed via FFI (`bindings/c/src/diagnostic.rs`, `NonoDiagnosticCode`, `last_diagnostic_code`/`last_remediation_json`) with `nono-ffi` building clean (exhaustive-match arms covered).
  3. The new diagnostics surface is reconciled with the fork's Windows diagnostic paths and the proxy `ProxyDiagnostic` surface (no regression in Windows denial output).
  4. `CLAUDE.md` § Library vs CLI boundary is updated to document the new core-crate audit + diagnostics modules, and an ADR records the boundary-convergence decision (rationale + what stays CLI-side).
**Plans**: 3 plans
Plans:
**Wave 1**
- [x] 86-01-PLAN.md — Theme A: cherry-pick a5b2a516 → aed35bec → 0b27cfc2 → e9529312 (audit → core, thin CLI wrappers, tests relocated)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 86-02-PLAN.md — Theme B: cherry-pick 4ad8ba92 → f867aba2 → a6aa5995 → 7f319b9e (diagnostics → core + FFI, proxy ProxyDiagnostic, Windows bridge)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 86-03-PLAN.md — BND-03: write ADR-86 + update CLAUDE.md § Library vs CLI Boundary, final make ci gate

### Phase 87: Security Sync
**Goal**: The two upstream security fixes in the window are absorbed with cross-target clippy clean on the cfg-gated Unix edits.
**Depends on**: Phase 86 (diagnostics/error surface settled; `capability.rs` touched by both)
**Requirements**: SEC-01, SEC-02
**Success Criteria** (what must be TRUE):
  1. The Linux AF_UNIX datagram bypass is closed — `sendto`/`sendmsg`/`sendmmsg` are trapped by the seccomp filter and gated on a connect grant (upstream #1096), verified by the ported test matrix on Linux CI.
  2. `deduplicate()` no longer inherits procfs-remap originals (upstream #1064 guard) — `/dev/null` is preserved when deduped with `/dev/stdin`.
  3. The cfg-gated Unix edits (`crates/nono/src/sandbox/linux.rs`, `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`, `crates/nono/src/capability.rs`) clear `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin` (or are marked PARTIAL→deferred-to-CI per the cross-target-verify checklist) — Windows-host `cargo check` is not accepted as a substitute.
**Plans**: 3 plans
Plans:
**Wave 1**
- [x] 87-01-PLAN.md — SEC-01: AF_UNIX datagram bypass close (cherry-pick e2086877 — BPF filter expansion, SYS_SEND* consts, send-family supervisor dispatch)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 87-02-PLAN.md — SEC-02 procfs-remap dedup guard (cherry-pick 6b3eb013) + CR-02 audit-integrity fix + ADR + divergence ledger addendum

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 87-03-PLAN.md — Cross-target clippy verification + PARTIAL→CI deferral record + ROADMAP/STATE update

### Phase 88: Feature + Dependency Cherry-Pick Wave
**Goal**: The additive, low-conflict feature cherry-picks, the PTY ctrl-z fix, and all workspace dependency bumps from the window are absorbed across the 5-crate workspace.
**Depends on**: Phase 86 (FEAT-02 XDG state touches audit roots; several features ride the reconciled error surface)
**Requirements**: FEAT-01, FEAT-02, FEAT-03, FEAT-04, FEAT-05, FEAT-06, DEPS-01, DEPS-02
**Success Criteria** (what must be TRUE):
  1. `set_vars` static env injection is available (profile + `CapabilitySet`) with env-name validation rejecting `PATH` and the `NONO_` prefix (FEAT-01); keyring access honors `NONO_KEYRING_TIMEOUT_SECS` (default 120s, `0`=none) (FEAT-04); `$PACK_DIR` store-pack session hooks resolve with `source_pack` propagation (FEAT-05).
  2. Runtime state (audit, sessions, rollback) resolves under XDG state dirs with legacy `~/.nono` fallback + one-time migration, and Windows path resolution is verified against the v3.0 scratch-space provisioner (FEAT-02).
  3. AWS auth config (`AwsAuthConfig`) is accepted/validated in profiles + proxy route config, mutually exclusive with `credential_key`/`oauth2` (FEAT-03); update-check reports CI provider/environment, profile names are namespace-standardized, and bool CLI flags accept truthy env values (FEAT-06).
  4. PTY ctrl-z suspend/resume no longer hangs under a PTY (DEPS-01).
  5. All 9 dependency bumps (x509-parser 0.18.1, hyper 1.10.1, cbindgen 0.29.4, typify 0.7.0, zeroize 1.9.0, time 0.3.49, chrono 0.4.45, ignore 0.4.26, which 8.0.3) are absorbed with internal path-dep version pins synced across all 5 crate `Cargo.toml` files; `make ci` is green on Windows (DEPS-02).
**Plans**: 6 plans
Plans:
**Wave 1**
- [x] 88-01-PLAN.md — FEAT-01 set_vars env injection + FEAT-04 keyring timeout (d48aeb7b, c6b13345)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 88-02-PLAN.md — FEAT-02 XDG state dirs + D-01/D-02/D-03 fork reconciliation (e8293b36, 8e0d94f9)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 88-03-PLAN.md — FEAT-03 AWS auth partial + fork 501 stub + FEAT-05 $PACK_DIR (5bb098cd partial, 7d274cf7)

**Wave 4** *(blocked on Wave 3 completion)*
- [ ] 88-04-PLAN.md — DEPS-01 PTY ctrl-z + FEAT-06a CI provider + FEAT-06b profile namespace + D-07/D-08 aliases (4179ce03, cc11b389, 6d88638e)

**Wave 5** *(blocked on Wave 4 completion)*
- [ ] 88-05-PLAN.md — FEAT-06c truthy env flags + Cluster M misc + e54cf9cb env_clear Unix-only (42e5bf73, a0bba5eb, ee7a3bda, 7e076d2d, 9800f307, e54cf9cb)

**Wave 6** *(blocked on Wave 5 completion)*
- [ ] 88-06-PLAN.md — CR-01 FFI clear-on-entry fork fix + DEPS-02 dep bumps + D-06 gate + PARTIAL→CI closeout

### Phase 89: Proxy Hardening Sync
**Goal**: The proxy hardening cluster is absorbed and reconciled against the fork-divergent TLS-interception surface without regressing fork TLS-intercept behavior.
**Depends on**: Phase 86 (proxy `ProxyDiagnostic` surface) and Phase 88 (AWS auth config / `customCredentials` credential path)
**Requirements**: PROXY-01, PROXY-02
**Success Criteria** (what must be TRUE):
  1. `allow_domain` endpoint routes no longer shadow the credential catch-all (#1132), and denied non-CONNECT requests return `403` plus an audit record (#1077).
  2. The TLS CONNECT intercept respects `upstream_proxy` (#1048/#1091), reactive proxy auth keeps the connection open on CONNECT (#1151), and the proxy activates when `customCredentials` is set (#1197).
  3. All of the above are reconciled by diff-inspection against the fork-divergent TLS-interception surface (Phase 34 C11 `fork-preserve`) — proxy tests pass and the fork's TLS-intercept invariants are preserved (no blind cherry-pick).
**Plans**: TBD

### Phase 90: v3.0 Host-Gated UAT Drain
**Goal**: The v3.0 host-gated UAT debt is drained — real daemon-side telemetry emission lands as code, and each host-gated item collapses to a single unattended `verify-dark.ps1` scripted gate with the residual live step explicitly host-gated.
**Depends on**: Nothing in this milestone (independent of the sync; can run last or in parallel). Builds on shipped v3.0 Phases 82–84.
**Requirements**: DRAIN-01, DRAIN-02, DRAIN-03, DRAIN-04
**Success Criteria** (what must be TRUE):
  1. `nono-agentd` registers the `SecurityEventLayer` so daemon-launched agent denials emit `nono_security::*` events — real wired code with a non-host-gated test (DRAIN-04).
  2. Clean-VM silent MSI install UAT (v3.0 DEPLOY-01/03/05) is executed on a fresh Win11 host with recorded verdicts, OR collapsed to a single unattended `verify-dark.ps1` gate with the residual live step explicitly host-gated (DRAIN-01).
  3. Dual-layer (proxy + kernel WFP) live egress-block proof (v3.0 EGRESS-02) is recorded via its scripted gate, with the live-host step operator-gated (DRAIN-02).
  4. The live SIEM telemetry gate plus admin opt-out / `min_severity` HKLM→emit (v3.0 TELEM-01/04) is verified via the `telemetry-event-emit` gate, with live SIEM ingestion host-gated (DRAIN-03).
**Plans**: TBD
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 85 → 86 → 87 → 88 → 89 → 90. Phase 90 (drain) is independent of 85–89 and may run in parallel or last.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 85. UPST9 Divergence Audit | 1/1 | Complete    | 2026-06-19 |
| 86. Library-Boundary Convergence | 3/3 | Complete    | 2026-06-20 |
| 87. Security Sync | 3/3 | Complete | 2026-06-20 |
| 88. Feature + Dependency Cherry-Pick Wave | 3/6 | In Progress|  |
| 89. Proxy Hardening Sync | 0/TBD | Not started | - |
| 90. v3.0 Host-Gated UAT Drain | 0/TBD | Not started | - |
