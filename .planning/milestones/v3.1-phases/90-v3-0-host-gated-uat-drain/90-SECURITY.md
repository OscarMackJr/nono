---
phase: 90
slug: v3-0-host-gated-uat-drain
status: verified
threats_open: 0
asvs_level: 2
created: 2026-06-21
---

# Phase 90 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> v3.0 host-gated UAT drain — daemon telemetry wiring (90-01) + verify-dark gate closeout (90-02).

**Verdict:** SECURED · **Block-on:** HIGH · **Threats Closed:** 11/11 · **Threats Open:** 0

This audit verifies that every declared mitigation in the Phase 90 threat register (plans
90-01 and 90-02) is present in the implemented code. Implementation files were read-only;
no implementation file was modified during verification.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| HKLM machine policy → daemon | Admin-controlled `policy.telemetry` (`enabled`, `min_severity`) crosses into the daemon process; must be honored, not bypassed | Telemetry config (enabled flag, severity threshold) |
| in-process proxy → SecurityEventLayer | `nono_security::network_deny` events cross from the proxy into the registered telemetry layer | Security event records (host/port/agent_pid) |
| SecurityEventLayer → OS sink (Windows) | Scrubbed/hashed security events cross into ETW + Application Log | Hashed paths, scrubbed values, HMAC chain head |
| operator/dev host → gate scripts | `verify-dark.ps1` gate scripts evaluate security-control state (install integrity, egress block, telemetry emit) on the host | Gate verdicts (PASS/FAIL/SKIP) + exit codes |
| gate verdict → closeout record | Verdicts cross from `verify-dark.ps1` persistence into the human-UAT doc | Persisted verdict JSON references |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-90-01 | Tampering / Repudiation | Daemon telemetry event chain | mitigate | Per-session HMAC-SHA256 chain w/ domain separators (`telemetry/mod.rs:113-140`), `sequence` via `saturating_add(1)` (`:139`); D-01 test asserts `sequence == 1` after one event (`agent_daemon/telemetry_init.rs:174-210`) | closed |
| T-90-02 | Information disclosure | Event Log payload | mitigate | `path_hash_for` (`telemetry/mod.rs:299-303`) + `nono::scrub_value` (`:313`, impl `nono/src/scrub.rs:177`); proxy emit omits `reason`, adds only `host`/`port`/`agent_pid` (`nono-proxy/src/audit.rs:199,203-209`) | closed |
| T-90-03 | Elevation / policy bypass | Admin telemetry opt-out | mitigate | `config.enabled` opt-out (`telemetry/mod.rs:273-275`) + `min_severity` threshold (`:290-292`), both in-lock (no TOCTOU); opt-out test keeps `sequence == 0` (`telemetry_init.rs:214-247`) | closed |
| T-90-04 | DoS / fail-open | Policy load error | mitigate | `?` on `read_machine_egress_policy()` → Err aborts startup (`agent_daemon/mod.rs:363`); absent → `TelemetryConfig::default()` default-ON (`:406`), never silently off | closed |
| T-90-05 | Tampering (TOCTOU / drift) | Second HKLM read | mitigate | Exactly ONE `read_machine_egress_policy(` call (`agent_daemon/mod.rs:363`); config threaded via 3-tuple return (`:357-359,370,396`); no second read | closed |
| T-90-SC (90-01) | Tampering | npm/pip/cargo installs | accept | No package installs; phase-90 commits touch only `.rs` files — no manifest changes | closed |
| T-90-06 | Repudiation | Gate verdict provenance | mitigate | `Persist-Verdict` precedes `[Console]::Out.Write` (`verify-dark.ps1:239-242`); SKIP path persists before exit (`:205-209`); doc references persisted JSON | closed |
| T-90-07 | Spoofing / false-PASS | Swallowed exit code | mitigate | `-File`-only invocation contract + explicit `-Command "<bare path>"` prohibition (`90-HUMAN-UAT.md:16-17`); exit map preserved (`verify-dark.ps1:244-249`) | closed |
| T-90-08 | Tampering | Gate-script edits during closeout | accept | No `scripts/gates/*.ps1` or `verify-dark.ps1` changes in phase-90 git history; FAIL escalated as debug finding, not patched (D-04) | closed |
| T-90-09 | Denial / false-confidence | Treating SKIP as drained-complete | accept | Operator-gated residuals recorded per DRAIN-01/02/03; 4 SKIP_HOST_UNAVAILABLE + 1 env FAIL NOT treated as drained-complete (`90-HUMAN-UAT.md`) | closed |
| T-90-SC (90-02) | Tampering | npm/pip/cargo installs | accept | Record/closeout plan; only `90-HUMAN-UAT.md` (`d1dacba9`) + tracking docs created. No code, no installs | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-90-01 | T-90-SC (90-01) | No package installs in 90-01; all types reuse already-pinned workspace crates (RESEARCH: N/A Package Legitimacy Audit). Git stat shows only `.rs` changes. | gsd-security-auditor | 2026-06-21 |
| AR-90-02 | T-90-08 | D-04 scope rule: no gate-script code changes in this phase; a broken gate is escalated as a debug finding, not patched here. Verified absent from phase-90 git history. | gsd-security-auditor | 2026-06-21 |
| AR-90-03 | T-90-09 | Residual live host steps recorded explicitly as operator-gated host-gated tech-debt; closeout asserts the scripted-gate collapse, not live-host proof. SKIP/FAIL not rubber-stamped as complete. | gsd-security-auditor | 2026-06-21 |
| AR-90-04 | T-90-SC (90-02) | 90-02 is a record-only closeout plan; doc-only commit `d1dacba9`. No code, no installs, no Rust toolchain needed. | gsd-security-auditor | 2026-06-21 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-21 | 11 | 11 | 0 | gsd-security-auditor |

---

## Notes

- **WR-01 hardening (post-execution):** the fail-open observability gap fixed in commit
  `2c665299` (`report_try_init` in `telemetry_init.rs:34-41`) strengthens T-90-04's posture —
  a failed subscriber registration is logged loudly to stderr rather than silently dropped.
  The daemon deliberately degrades-not-dies on telemetry-wiring failure, consistent with the
  declared disposition: telemetry is observability, not the security boundary; OS-enforced
  sandboxing remains intact.
- T-90-03 / T-90-04 are verified at the library boundary: `TelemetryConfig::default()` is
  default-ON, so the absent-policy branch never silently disables telemetry.
- **Unregistered flags:** none. Both SUMMARY.md `## Threat Flags` sections declare "No new
  threat surface" — verified accurate. 90-01 wires the pre-existing audited
  `SecurityEventLayer` (chain / path-hashing / `scrub_value` / ETW+App-Log sinks unchanged,
  `Err → abort` preserved); 90-02 ran read-only gates + authored a record doc.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-21
