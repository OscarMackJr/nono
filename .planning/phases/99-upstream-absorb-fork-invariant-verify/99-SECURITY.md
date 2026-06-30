---
phase: 99
slug: upstream-absorb-fork-invariant-verify
status: verified
threats_open: 0
asvs_level: 2
created: 2026-06-30
register_authored_at_plan_time: true
verdict: SECURED
---

# Phase 99 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> **Verdict:** SECURED — 24/24 threats CLOSED (0 OPEN, 0 ESCALATE).
> Register authored at plan time across 7 plans (99-01..99-07); auditor verified mitigations exist (no new-threat scan). All implementation files were read-only.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CLI proxy intent (NetworkIntent) → sandbox enforcement | NetworkIntent is CLI-only (`pub(crate)`); NetworkMode::ProxyOnly in crates/nono/ remains the enforcement type | Routing intent → enforcement decision |
| proxy_runtime.rs → CompiledEndpointPolicy evaluate() | CompiledEndpointPolicy chain must not be bypassed by NetworkIntent / endpoint_restrictions adoption | Endpoint allow/deny decision |
| CLI arg validation → sandbox/proxy setup | validate_block_net_conflicts() must run before any security-sensitive setup | Contradictory --block-net config |
| linux.rs 9P check → Landlock capability-add loop | 9P diagnostic must be additive only; must not affect AF_UNIX EPERM filter or cgroup enforcement | Filesystem-type metadata (read-only statfs) |
| ADR-86 policy-free-library boundary | NetworkIntent / CLI policy must not leak into crates/nono/ library | Type/symbol visibility |
| Cross-target clippy gate → cfg(linux)/cfg(macos) branches | Only cross-target gates detect drift in cfg-gated Unix branches; Windows-host clippy is structurally blind | Lint coverage |
| Upstream replay commits → fork git history | Each absorbed commit must retain DCO + upstream SHA trailer for auditability | Provenance metadata |
| sigstore-trust-root supply chain | New pin version must not introduce RUSTSEC advisories | Dependency resolution |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation / Evidence | Status |
|-----------|----------|-----------|-------------|-----------------------|--------|
| T-99-01-doc | Tampering | ROADMAP.md Phase 99 SC text | mitigate | SC reconciled to ledger (Cluster A/NetworkIntent/tool-sandbox refs present) | closed |
| T-99-02-doc | Repudiation | PR→cluster map (REQUIREMENTS.md) | mitigate | PR→cluster traceability table (25 refs incl. SHAs) | closed |
| T-99-SC (01) | Tampering | installs (Plan 01) | accept | Docs-only plan; no installs | closed |
| T-99-01 | Tampering | proxy_runtime.rs CompiledEndpointPolicy chain | mitigate | proxy_runtime.rs:681 `test_compiled_endpoint_policy_compat_deviation_preserved`; reverse.rs:1350 `denied_endpoint_returns_403_and_audit` | closed |
| T-99-02 | Tampering | ADR-86 policy-free-library boundary | mitigate | `grep -rn NetworkIntent crates/nono/src/` = 0; `pub(crate) enum NetworkIntent` launch_runtime.rs:147 | closed |
| T-99-03 | Elevation of Privilege | WSL2ProxyFallback bypass | mitigate | profile/mod.rs:1989 `Wsl2ProxyPolicy`; test :8621 `test_wsl2_proxy_policy_deviation_preserved` | closed |
| T-99-04 | Tampering | cfg(windows) drift | defer-to-P07 | Covered by T-99-17 (cross-target clippy gates, Plan 07) | closed |
| T-99-SC (02) | Tampering | installs (Plan 02) | accept | CLI-only refactor; Cargo.toml untouched | closed |
| T-99-05 | Tampering | validate_block_net_conflicts() call ordering | mitigate | launch_runtime.rs:361 call precedes prepare_proxy_launch_options :406 | closed |
| T-99-06 | Tampering | CompiledEndpointPolicy evaluate() bypass | mitigate | reverse.rs:1350 `denied_endpoint_returns_403_and_audit` present | closed |
| T-99-07 | Tampering | linux.rs seccomp/AF_UNIX invariants | mitigate | linux.rs:4546 `test_proxy_only_with_landlock_v4_returns_no_fallback`; :3955 `test_seccomp_network_fallback_mode_proxy_only` | closed |
| T-99-SC (03) | Tampering | installs (Plan 03) | accept | Function code only; no Cargo.toml change | closed |
| T-99-08 | Tampering | linux.rs AF_UNIX EPERM filter weakened by 9P insertion | mitigate | AF_UNIX EPERM filter intact (linux.rs:1478 `SECCOMP_RET_ERRNO\|EPERM`, :1527 `AF_UNIX`); 9P call :936 warning-only/additive | closed |
| T-99-09 | Information Disclosure | OscarMackJr/nono fork URL overwritten | mitigate | update_check.rs:34 `update.nono.sh/v1/check` (fork endpoint unchanged); github fixture :605 `nolabs-ai/nono`. See Stale-Mitigation note. | closed |
| T-99-10 | Tampering | GPU `test_is_nvidia_compute_device_*` deleted | mitigate | linux.rs:4859 / :4880 GPU tests present | closed |
| T-99-SC (04) | Tampering | installs (Plan 04) | accept | String replacements + code additions; no new deps | closed |
| T-99-11 | Tampering (supply chain) | sigstore-trust-root 0.9.0 | mitigate | Cargo.toml:58 `=0.9.0`; 6 Cargo.lock refs; cargo audit clean (attested). WR-01 dual-version pin human-accepted | closed |
| T-99-12 | Tampering (API break) | sigstore-verify API break | mitigate | `make build` GREEN (VERIFICATION truth 13; 99-05 SUMMARY) — closed-by-attestation | closed |
| T-99-13 | Information Disclosure | EffectiveProxySettings / `_ep_` namespace | mitigate | `EffectiveProxySettings` intact (proxy_runtime.rs:16 + main.rs); `_ep_` route.rs (5 matches). token.rs change did not disrupt. See file-location note. | closed |
| T-99-SC (05) | Tampering | cargo update scope (Plan 05) | mitigate | Scoped `cargo update -p sigstore-trust-root`; cargo audit clean | closed |
| T-99-14 | Elevation of Privilege | tls_intercept/ accidentally created | mitigate | `ls crates/nono-proxy/src/` no tls_intercept; `grep -rn h2_forward` = 0 | closed |
| T-99-15 | Tampering | CompiledEndpointPolicy bypassed by endpoint_restrictions | mitigate | proxy_runtime.rs: `endpoint_restrictions` (7) AND `CompiledEndpointPolicy` (8) both present; denied_endpoint guard passes | closed |
| T-99-16 | Tampering | `_ep_` credential route namespace disrupted by wildcard fix | mitigate | route.rs `_ep_` (5, incl. `_ep_api.openai.com`); route.rs:681 `allow_domain_endpoint_route_does_not_shadow_credential_route` | closed |
| T-99-SC (06) | Tampering | new deps hyper-util/h2 (Plan 06) | mitigate | Trusted upstream / hyperium crates; tls_intercept absent, namespace preserved | closed |
| T-99-17 | Tampering | cfg(linux)/cfg(macos) drift | mitigate | linux-gnu + apple-darwin cross clippy GREEN (99-07 SUMMARY, human-approved); let_chains edition-2021 catch proves gate ran — closed-by-attestation | closed |
| T-99-18 | Tampering | AppContainer/WFP/broker Windows backends regressed | mitigate | `git diff <phase> -- exec_strategy_windows/` empty; `WindowsNetworkPolicyMode::ProxyOnly` intact (network.rs:466+) | closed |
| T-99-19 | Tampering | ADR-86 boundary breached (NetworkIntent in library) | mitigate | `grep -rn NetworkIntent crates/nono/src/` = 0 | closed |
| T-99-20 | Repudiation | Missing upstream SHA trailer on replay commit | mitigate | 9 `(cherry picked from commit ...)` trailers; SHAs match VERIFICATION truth 1; Signed-off-by present | closed |
| T-99-SC (07) | Tampering | installs (Plan 07) | accept | Verify-only plan; no new packages | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party) · defer (covered by later plan)*

**Totals:** 24 register entries (20 numbered + 7 per-plan T-99-SC; T-99-04 folded into T-99-17). All CLOSED. 0 OPEN. 0 ESCALATE.

---

## Stale / Divergent Mitigation-Plan Notes (semantically satisfied, not gaps)

- **T-99-09** — The plan-time grep (`grep -n OscarMackJr update_check.rs`) is stale relative to the actual absorb. Cluster G commit c808f000 intentionally migrated org references to `nolabs-ai` (the fork's documented current home; upstream relocated nolabs-ai/nono in v3.3). The Information-Disclosure goal holds: the live update endpoint is the fork-controlled `update.nono.sh`, and the only github.com string is a test fixture pointing at `nolabs-ai/nono`. OscarMackJr identity still appears in `.github/` + scripts (VERIFICATION truth 7).
- **T-99-13** — The plan cited `token.rs` for the `EffectiveProxySettings` / `_ep_` grep. Those symbols actually live in `proxy_runtime.rs` (struct, pub(crate)) + `main.rs`, and `_ep_` in `route.rs`. The token.rs change (X-Nono-Token → Proxy-Authorization, a4d68189) is a different concern and did not remove or disrupt the namespace. Namespace count unchanged.

---

## Unregistered Flags (new attack surface with no threat mapping)

**None.** All 7 plan summaries' Threat Flags / Threat Surface Scan sections report no new network endpoints, auth paths, file-access patterns, or schema changes:
- 99-01: docs-only — none
- 99-02: NetworkIntent internal routing enum, no external surface
- 99-03: validate_block_net_conflicts is a pure error-on-contradiction validator
- 99-04: statfs(2) read-only metadata probe under /mnt only
- 99-05: Cluster F adds a dep version but no callable production surface
- 99-06: UpstreamPool reachable only via trusted internal ReverseProxyCtx
- 99-07: verify-only; gate fixes are structural cleanups

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| WR-01 | T-99-11 | `sigstore-trust-root = "=0.9.0"` is a direct dep unused at runtime (effective TUF root is transitive 0.8.0 via sigstore-verify). Intentional deferred-cascade marker; misleading Cargo.toml comment corrected inline (commit 6cdfe149). Dep removal deferred to Phase 100. No runtime attestation impact. | Human (Oscar Mack Jr) | 2026-06-30 |
| WR-02 | (quality, no threat-register entry) | `--allow-http2` / `enable_h2` is a runtime no-op (UpstreamPool::send never called from handle_reverse_proxy; tls_connector path used). Capability-claims gap only — fails to HTTP/1.1, no security exposure. Phase 100 release-notes gate must guard help text. | Human (Oscar Mack Jr) | 2026-06-30 |
| WR-03 | (quality, no threat-register entry) | `validate_block_net_conflicts` folds profile_network_block into its check and runs before strict_filter, so profile `block:true + allow_domain` errors instead of entering strict-filter mode. Fails closed (error, not bypass). Semantic decision deferred to Phase 100. | Human (Oscar Mack Jr) | 2026-06-30 |
| WR-04 | (scope limitation) | 9P warning only fires for paths under /mnt; 9P mounts elsewhere get no warning. Diagnostic-only — no enforcement decision depends on it. Not a regression. | Human (Oscar Mack Jr) | 2026-06-30 |

*Accepted risks do not resurface in future audit runs. WR-01..04 are quality/correctness items from 99-REVIEW.md; none is a security regression and all relevant paths fail closed.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-30 | 24 | 24 | 0 | gsd-security-auditor (verify-mitigations mode) |

---

## Audit Method

For each `mitigate` threat the declared grep/test/git check was run against the cited implementation file and matched to a concrete file:line or git result (table above). `accept` / `defer` dispositions were confirmed plausible and, where applicable, mapped to their covering verification. T-99-12, T-99-17, and the 11 pre-existing test-cli failures were taken as SUMMARY-attested + human-approved per VERIFICATION.md frontmatter (human_resolution 2026-06-30) and marked CLOSED-by-attestation rather than re-run, per audit scope. No implementation file was modified.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer / defer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-30
