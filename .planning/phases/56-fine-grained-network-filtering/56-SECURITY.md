---
phase: 56
slug: fine-grained-network-filtering
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-05
---

# Phase 56 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register authored at plan time across 4 PLAN.md files. This audit VERIFIES each
> declared mitigation/acceptance exists in implementation; it does not scan for new threats.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CLI `--allow-domain` arg → `parse_allow_domain_arg` | User-controlled URL string; `url::Url::parse` normalizes percent-encoding + removes `..` at parse time (double-safe with `normalize_path` at match time) | Untrusted operator URL |
| Profile JSON → `AllowDomainEntry` | Untrusted, user-editable profile files parsed via `#[serde(untagged)]`; empty domain is a failure vector | Untrusted config |
| `endpoint_routes` → `resolved.routes` | Endpoint routes bypass CONNECT and enter the reverse-proxy path; must not expose credential injection (`credential_key: None` enforced) | Route config (no secrets) |
| `DomainEndpointState` → `query_ext` display | State file may be tampered; endpoint rules displayed are diagnostic only — no enforcement consequence | Operator config |
| `credential.rs` byte-identical gate | Any modification to `credential.rs` is an invariant violation; the SC4 verification sweep catches it | Credential injection logic |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation (evidence) | Status |
|-----------|----------|-----------|-------------|-----------------------|--------|
| T-56-01 | Tampering | `WithEndpoints` empty domain | mitigate | `network_policy.rs:381-386` — empty domain → `Err(ConfigParse)` (fail-secure, D-09) | closed |
| T-56-02 | Tampering | serde(untagged) misparse | accept | `profile/mod.rs:29-41` — `Plain` declared first; string→Plain, object→WithEndpoints; structurally unambiguous | closed |
| T-56-03 | Spoofing | D-12 bare entry widens scoped host | accept | WATCH-ITEM; `nono why --host` surfaces effective openness (`query_ext.rs`, `why_runtime.rs:192-198`) | closed |
| T-56-SC | Tampering | Supply chain (npm/pip/cargo) | mitigate | No new external packages; `globset` + `urlencoding` already in workspace (Approved) | closed |
| T-56-04 | Tampering | path traversal via `../` | mitigate | `proxy_runtime.rs:35-58` — `url::Url::parse` behind scheme guard (CR-02); proxy match uses `normalize_path` (`config.rs:218`) | closed |
| T-56-05 | Spoofing | `credential_key:None` on endpoint routes | mitigate | `network_policy.rs:396` sets `credential_key:None`; `reverse.rs:96` endpoint check before `:119` credential get (SC2 ordering) | closed |
| T-56-06 | Elevation | C5 rider adds host to allowlist | accept | `proxy_runtime.rs:206-218` strips only scheme prefixes from operator-supplied upstreams; no new surface | closed |
| T-56-07 | Info Disclosure | `domain_endpoints` persisted to state | accept | `sandbox_state.rs` persists config only; `credential_key:None` → no credential material | closed |
| T-56-08 | Tampering | Schema permits arbitrary method/path | accept | Schema advisory; enforcement in `config.rs:214-223` `is_allowed` (method + path) | closed |
| T-56-09 | Info Disclosure | `profile show` reveals endpoint rules | accept | Intentional user-facing output; no secrets | closed |
| T-56-10 | Tampering | manifest `try_into().ok()?` skips invalid | accept | `profile_cmd.rs:3258-3266`; manifest diagnostic, proxy enforces; omission is a visible gap | closed |
| T-56-11 | Tampering | `parse_host_input` display-only | accept | `why_runtime.rs` feeds display path only; enforcement only in proxy | closed |
| T-56-12 | Tampering | `path_matches_endpoint_rules` uses globset directly | mitigate | `query_ext.rs:400-422` doc'd "diagnostic only, not access control" (WR-02 advisory) | closed |
| T-56-13 | Spoofing | `credential.rs` modified by concurrent plan/merge | mitigate | Blob `8f7e78c5` byte-identical base `de2c6f8f` ↔ HEAD; untouched by phase 56 (last mod `5aef2f04`) | closed |
| T-56-14 | Repudiation | Missing `Upstream-commit` trailers | mitigate | Trailers present on `0ced085`, `75b2265`, `22e6c40` — all 3 declared SHAs found | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

### T-56-13 — SHA reconciliation (not a gap)

The plan declared the `credential.rs` invariant prefix as `c9f25164`; the actual
`git show HEAD:crates/nono-proxy/src/credential.rs | sha256sum` on this fork is
`5bfabf6f...`. This is **not** an implementation gap — `c9f25164` was the upstream
plan-author's machine value (documented in 56-04-SUMMARY:118-131). The property the
threat protects — `credential.rs` unmodified during Phase 56 — holds independently:
git blob `8f7e78c5` is byte-identical at base `de2c6f8f` and HEAD; the phase-56
commit range touches no credential.rs. Recommend future plan docs cite the fork SHA
`5bfabf6f`.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-56-02 | T-56-02 | Untagged serde: `Plain` first; string→Plain, object→WithEndpoints; structurally unambiguous. No enforcement consequence. | gsd-security-auditor | 2026-06-05 |
| AR-56-03 | T-56-03 | D-12 bare entry widens scoped host (WATCH-ITEM); `nono why --host` surfaces effective openness to operator. | gsd-security-auditor | 2026-06-05 |
| AR-56-06 | T-56-06 | C5 rider strips only scheme prefixes from operator-supplied upstreams; no new host surface. | gsd-security-auditor | 2026-06-05 |
| AR-56-07 | T-56-07 | `domain_endpoints` is operator-controlled config; `credential_key:None` → no credential material in state. | gsd-security-auditor | 2026-06-05 |
| AR-56-08 | T-56-08 | Schema is advisory; proxy `CompiledEndpointRules::is_allowed` enforces method + path. | gsd-security-auditor | 2026-06-05 |
| AR-56-09 | T-56-09 | `profile show` endpoint display is intentional user-facing output; no secrets emitted. | gsd-security-auditor | 2026-06-05 |
| AR-56-10 | T-56-10 | Manifest is diagnostic; proxy enforces; skipped invalid entry is a visible gap, not silent bypass. | gsd-security-auditor | 2026-06-05 |
| AR-56-11 | T-56-11 | `parse_host_input` feeds display path only; enforcement is exclusively in the proxy. | gsd-security-auditor | 2026-06-05 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-05 | 15 | 15 | 0 | gsd-security-auditor |

**Register origin:** authored at plan time (`register_authored_at_plan_time: true`) — auditor verified declared mitigations against implementation; did not scan for new threats.

### Advisory follow-ups (non-blocking, from 56-REVIEW.md)

Open advisory items, NOT declared-mitigation gaps — do not block under `block_on=critical/high`:

- **WR-02** — `nono why` endpoint display ignores rule `method` (path-only match); can over-report "allowed" vs proxy enforcement. Usability hazard; covered by T-56-12 disposition (display is non-access-control).
- **WR-03** — profile + CLI same-host `Plain`/`WithEndpoints` not merged at runtime; fail-secure on this fork via the `server.rs` CONNECT block. Fragile but safe.
- **IN-01/IN-02/IN-03** — dedup, schema `additionalProperties:false`, redundant lowercasing. Cosmetic / hardening; no security regression.

### Resolved blockers (verified present)

- **CR-01** (fail-open: profile endpoint rules dropped) — FIXED `05cd7580`; `PreparedSandbox.allow_domain: Vec<AllowDomainEntry>` threaded end-to-end (no lossy flatten).
- **CR-02** (host:port mangled into bogus endpoint routes) — FIXED `05cd7580`; `parse_allow_domain_arg` gated on explicit http(s):// scheme.
- **WR-01** (TLS downgrade via string-prefix loopback match) — FIXED `c4931750`; `is_loopback_domain` uses parsed `IpAddr` semantics (`network_policy.rs:344-352`).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-05
