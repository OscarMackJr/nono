---
phase: 83
slug: machine-policy-spine-egress-control
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-18
---

# Phase 83 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) + PowerShell Dark Factory gate (`verify-dark.ps1`) |
| **Config file** | none — workspace `Cargo.toml` + existing `scripts/gates/` contract |
| **Quick run command** | `cargo test -p nono machine_policy && cargo test -p nono net_filter` |
| **Full suite command** | `make test` (lib + cli + doc) |
| **Estimated runtime** | ~30 s quick; ~3–5 min full |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono machine_policy && cargo test -p nono net_filter`
- **After every plan wave:** Run `make test` (plus `cargo clippy --workspace --target x86_64-unknown-linux-gnu` and `--target x86_64-apple-darwin` for any cfg-gated Unix-touching commit per CLAUDE.md cross-target rule)
- **Before `/gsd:verify-work`:** Full suite green; `verify-dark.ps1 --gate egress-policy-deny` run on a host-gated Windows box (admin + fresh nono-wfp-service + non-elevated daemon)
- **Max feedback latency:** 30 seconds (unit); host-gated WFP/gate proof deferred to UAT

---

## Per-Task Verification Map

> Mapped to the finalized plans (83-01..83-04). Task IDs are `{phase}-{plan}-{task}`.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 83-01-01 | 01 | 1 | POLICY-01, POLICY-02 | — | `winreg` dependency legitimacy verified before add (slopcheck) | checkpoint:human-verify | n/a (blocking-human gate) | n/a | ⬜ pending |
| 83-01-02 | 01 | 1 | POLICY-01, POLICY-02 | T-83-absent-fallthrough, T-83-malformed-failopen | Absent key (errno 2) → `Ok(None)` fall-through; unreadable/malformed → `Err(PolicyLoadFailed)`; 64-bit view; enumerate N×REG_SZ subkey | unit | `cargo test -p nono machine_policy` | ❌ W0 | ⬜ pending |
| 83-01-03 | 01 | 1 | EGRESS-03 | T-83-dns-component | `api.anthropic.com`✓ vs `anthropic.com`/`evilanthropic.com`/`anthropic.com.evil.com`✗ | unit | `cargo test -p nono net_filter::sc4_dns_component_matrix` | ✅ | ⬜ pending |
| 83-02-01 | 02 | 2 | POLICY-03, EGRESS-01 | T-83-layer-drift, T-83-deny-default | One startup HKLM read → `ProxyFilter::new_strict` deny-by-default; per-user ignored when machine policy present | unit | `cargo test -p nono-cli machine_policy_handoff` | ❌ W0 | ⬜ pending |
| 83-02-02 | 02 | 2 | EGRESS-02 | T-83-proxy-bypass | Per-SID WFP `proxy-only` permit=loopback-proxy-port only, block all else, from the same struct; WFP service never reads HKLM | unit | `cargo test -p nono-cli wfp_proxy_only` | ❌ W0 | ⬜ pending |
| 83-03-01 | 03 | 1 | EGRESS-04 | — | `network-policy.json` carries `anthropic`/`openai`/`github-api` groups; token→FQDN expands; unknown token → empty (least-priv) | unit | `cargo test -p nono-cli policy_egress_groups` | ❌ W0 | ⬜ pending |
| 83-03-02 | 03 | 1 | EGRESS-04 | — | ADMX here-string exposes three named preset toggles writing group TOKENS; `<list>` policies unchanged | script (dev-host source assert) | `grep -Eq 'Allow Anthropic\|Allow OpenAI\|Allow GitHub API' scripts/build-windows-msi.ps1` | ❌ W0 | ⬜ pending |
| 83-04-01 | 04 | 3 | POLICY-02, EGRESS-02 | T-83-malformed-failopen, T-83-proxy-bypass | SC-2 non-zero exit on corrupted key; SC-3 dual-layer deny | script (host-gated) | `pwsh scripts/verify-dark.ps1 --gate egress-policy-deny` | ❌ W0 | ⬜ pending |
| 83-04-02 | 04 | 3 | POLICY-02, EGRESS-02 | — | Cross-target clippy clean (Unix cfg-gated winreg/WFP code) | lint | `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` (+ darwin) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono/src/machine_policy.rs` — new module with unit tests for absent/unreadable/malformed taxonomy (POLICY-01, POLICY-02) and `MachineEgressPolicy` deserialization
- [ ] `crates/nono/src/net_filter.rs` — add `sc4_dns_component_matrix` test codifying EGRESS-03 reject set
- [ ] `crates/nono-cli/` — handoff + WFP `proxy-only` request test fixtures (POLICY-03, EGRESS-02)
- [ ] `crates/nono-cli/data/policy.json` — egress/domain preset group expansion test (EGRESS-04)
- [ ] `scripts/gates/egress-policy-deny.ps1` — new Dark Factory gate (clone `wfp-egress-isolation.ps1` two-function contract)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live dual-layer deny (proxy rejects out-of-list AND WFP blocks SID→out-of-list-IP) | EGRESS-02, POLICY-03 | Requires admin, fresh `nono-wfp-service`, non-elevated daemon, real AppContainer agent launch; WFP filter inspection via `netsh wfp show filters` | On host-gated Win11: install machine policy of only `*.anthropic.com`, launch confined agent via daemon, assert proxy denies out-of-list request and per-SID block filter is present |
| `verify-dark.ps1 --gate egress-policy-deny` non-zero exit on corrupted key | POLICY-02 | Gate seeds an ACCESS_DENIED/wrong-type key and runs the real startup path | On host-gated Win11: `pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny`; assert non-zero exit |
| GPO ADMX / Intune OMA-URI fleet push | POLICY-01, EGRESS-04 | Requires domain-joined / MDM-enrolled host | Import `nono.admx`/`nono.adml`, enable a preset toggle, confirm token lands in 64-bit `HKLM\SOFTWARE\Policies\nono` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (83-01-01 is a blocking-human checkpoint; 83-04-01 is host-gated with the corrupted-key gate)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s (unit tier)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-18 (plan-phase; plans 83-01..83-04 pass plan-checker Dimension 8)
