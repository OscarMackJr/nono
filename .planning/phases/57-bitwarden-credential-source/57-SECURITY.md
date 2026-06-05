---
phase: 57
slug: bitwarden-credential-source
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-05
---

# Phase 57 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase 57 adds a `bw://` Bitwarden credential source to `crates/nono/src/keystore.rs`,
routing `bw://item/<id>/<selector>` to the Bitwarden `bw` CLI and `bw://secret/<uuid>`
to the Bitwarden Secrets Manager `bws` CLI. All threats from the plan-time STRIDE
register were verified against the implementation by `gsd-security-auditor` (2026-06-05).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Operator env → bw/bws subprocess | `BW_SESSION` / `BWS_ACCESS_TOKEN` travel from parent-process env to the child process environment. Token is not in argv; it is accessible via `/proc/<pid>/environ` on Linux — acknowledged, same class as `op://`. | Auth tokens (high sensitivity) |
| bw/bws CLI stdout → keystore.rs | Subprocess output parsed as JSON, field values extracted. A corrupted CLI could return adversarial JSON; mitigated by strict `serde_json` parse (non-UTF-8 / non-JSON is a hard error). | Secret values + structure (high sensitivity) |
| Operator-supplied URI → validate_bw_uri | URI is operator-supplied (profile or `--credential`). Attacker controlling the URI may attempt injection; mitigated by `FORBIDDEN_URI_CHARS` + `is_valid_bw_id` allowlist + `--` arg terminator before any spawn. | Untrusted control string |
| bw/bws stdout (Vec<u8>) → memory | The raw subprocess output `Vec<u8>` is not zeroized (T-57-05, accepted). Extracted values are wrapped in `Zeroizing<String>` immediately. | Secret values (transient) |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-57-01 | Tampering | validate_bw_uri / CLI args | mitigate | `FORBIDDEN_URI_CHARS` check on stripped path (`validate_bw_uri` keystore.rs:329); `is_valid_bw_id` allowlist (alphanumeric + hyphen, max 64) on item ID + secret UUID (keystore.rs:374, 445); `--` argument terminator at all three `Command::new` sites (keystore.rs:1673, 1740, 1838); `Command::new` (no shell). | closed |
| T-57-02 | Information Disclosure | BW_SESSION / BWS_ACCESS_TOKEN in process argv | mitigate | Tokens read from inherited env only; `--session` appears only in comments, never as `Command::arg`. Pre-flight non-empty validation before spawn (`BW_SESSION` keystore.rs:1657–1666; `BWS_ACCESS_TOKEN` keystore.rs:1815–1828). No secret value in argv at any spawn site. | closed |
| T-57-03 | Information Disclosure | Secret value in logs/diagnostics | mitigate | All `tracing::debug!` in bw paths use `redact_bw_uri(uri)` (keystore.rs:1668, 1830–1832); `classify_bw_error`/`classify_bws_error` log only redacted URI + stderr (keystore.rs:1589–1606, 1620–1635). Resolved secret value never logged. (See unregistered flag WR-01 — hardening-only, out of declared scope.) | closed |
| T-57-04 | DoS / Spoofing | Fail-open on missing token or locked vault | mitigate | `std::env::var` fails closed on `Err` (unset) + explicit empty-string check for both tokens (keystore.rs:1657–1666, 1815–1828); non-zero subprocess exit routed through `classify_*_error` returning `Err` — never `Ok(empty)`. Verified by `test_load_from_bw_no_session`, `test_load_from_bw_empty_session`, `test_load_from_bws_no_token`. | closed |
| T-57-05 | Information Disclosure | Secret in intermediate `Vec<u8>` (subprocess stdout) | accept | `output.stdout` (`Vec<u8>`) from `std::process::Output` is not zeroized — same class as `op://`. Accepted: `std::process::Output` provides no zeroize API; values promoted to `Zeroizing<String>` immediately on extraction. Documenting comments at keystore.rs:1648–1650, 1706–1708, 1799–1801, 1871–1873. See Accepted Risks Log AR-57-01. | closed |
| T-57-06 | Tampering | D-06 field selector on bw://secret/ | mitigate | `validate_bws_uri_segments` rejects any URI with `segments.len() != 2` (keystore.rs:431) with D-06 `ConfigParse` text. Verified by `test_validate_bw_uri_secret_no_field_selector`. | closed |
| T-57-SC | Tampering (supply chain) | No new package installs | accept | No new cargo/npm/pip packages; `serde_json` is a pre-existing workspace dependency (`crates/nono/Cargo.toml:24` `serde_json.workspace = true`). See Accepted Risks Log AR-57-02. | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-57-01 | T-57-05 | The raw subprocess stdout `Vec<u8>` cannot be zeroized — `std::process::Output` exposes no zeroize API. Exposure window is transient; extracted field values are promoted to `Zeroizing<String>` immediately on extraction and cleared on drop. Identical class to the existing `op://` backend. Documented in code at keystore.rs:1648–1650, 1706–1708, 1799–1801, 1871–1873. | Oscar Mack Jr | 2026-06-05 |
| AR-57-02 | T-57-SC | No new third-party dependency was added. `serde_json` is an existing direct workspace dependency. No new supply-chain attack surface; no slopcheck required. | Oscar Mack Jr | 2026-06-05 |

*Accepted risks do not resurface in future audit runs.*

---

## Unregistered Flags (non-blocking hardening debt)

| Flag | Source | Assessment |
|------|--------|------------|
| WR-01 | 57-REVIEW.md | `classify_bw_error`/`classify_bws_error` interpolate raw subprocess stderr into error messages. Pre-existing pattern inherited from `classify_op_error`; stderr from `bw`/`bws` is not the resolved secret value, so it is outside T-57-03's declared scope. Hardening-only, not a threat violation. |
| WR-02 | 57-REVIEW.md | `bw get totp` non-zero exit misclassified as "item not found" rather than "no TOTP configured". Diagnostic-accuracy / observability issue, not a security threat. |
| WR-03 | 57-REVIEW.md | `is_valid_bw_id` accepts leading-hyphen / all-hyphen IDs. The `--` argument terminator at all three spawn sites (keystore.rs:1673, 1740, 1838) structurally prevents flag injection, so there is no exploitable path today. Defense-in-depth hardening debt — non-blocking. |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-05 | 7 | 7 | 0 | gsd-security-auditor (sonnet) + orchestrator (accepted-risk log) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log (AR-57-01, AR-57-02)
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-05
