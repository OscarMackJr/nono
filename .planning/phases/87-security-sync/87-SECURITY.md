---
phase: 87
slug: security-sync
status: verified
threats_open: 0
asvs_level: balanced
created: 2026-06-21
---

# Phase 87 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Verified by gsd-security-auditor 2026-06-21 against the live source tree (Linux
> cfg-gated code read directly — Windows host cannot compile it; "can't compile
> on Windows" was NOT treated as a gap).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Sandboxed child → kernel | Syscalls from the untrusted child cross into the kernel seccomp-BPF filter | Syscall numbers + raw register args |
| Child address space → supervisor | Supervisor reads child memory via `/proc/PID/mem` to extract sockaddr/msghdr | Untrusted pointer-referenced sockaddr bytes |
| Seccomp USER_NOTIF → supervisor decision | Notification carries raw args the supervisor must validate before trusting | notif_id, syscall args, sockaddr |
| `CapabilitySet.deduplicate()` output → Landlock apply | Surviving FsCapability entries determine installed Landlock rules; a wrong `original` corrupts rule paths | Resolved/original path pairs |
| `verify_audit_log` result → caller `is_valid()` | Callers use `is_valid()` for audit-integrity decisions; a false-true misleads security decisions | Audit ledger verification verdict |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-87-01 | Tampering | AF_UNIX SOCK_DGRAM bypass via sendto/sendmsg/sendmmsg | mitigate | Hybrid gate: `af_unix_send_filter_action` (`exec_strategy.rs:480-498`) → Off=NoFilter / Pathname+grants=UserNotify / Pathname+no-grants=StaticEperm; same helper at child install (`:1339`) AND parent recv-fd (`:1643`), deadlock-safe. Static-EPERM filter `linux.rs:2337-2384`; USER_NOTIF 8-insn filter `linux.rs:2237-2280`. CR-01 fix `718fe59d` in tree. | closed |
| T-87-02 | Elevation of Privilege | Abstract-namespace AF_UNIX bypassing pathname allowlist | mitigate | `classify_af_unix` (`linux.rs:1880-1891`) `sun_path[0]==0`→Abstract, unreadable→Unnamed (fail-closed); `decide_network_notification` (`supervisor_linux.rs:592-606`) denies Abstract/Unnamed/None for all send ops | closed |
| T-87-03 | Denial of Service | sendmmsg large vlen → unbounded `/proc/PID/mem` reads | mitigate | `read_mmsghdr_dests` `MAX_MMSGHDRS=1024` cap (`linux.rs:2775`); `checked_mul`/`checked_add` fail-secure on overflow (`:2788`,`:2791`) | closed |
| T-87-04 | Information Disclosure | TOCTOU: child changes sockaddr between BPF trap and supervisor read | mitigate | `notif_id_valid` checked once before the per-sockaddr decision loop (`supervisor_linux.rs:977`); expiry/pid-change → deny | closed |
| T-87-05 | Tampering | Null/zero `msg_name` (connected socket) misclassified as bypass | accept | Fast-paths: sendto `args[4]==0`→continue (`:867-872`); sendmsg `msg_name==NULL`→continue (`:903-909`); sendmmsg all-NULL→continue (`:952-961`). Connected sends carry no per-call dest — no bypass possible | closed |
| T-87-06 | Tampering | procfs-remap dedup bug: `/dev/null` original overwritten by `/dev/stdin` alias → Landlock rule corrupted to `/proc/PID/fd/0` (SEC-02) | mitigate | `is_procfs_remap_original` guard (`capability.rs:1842-1844`) at both `original_updates.push` sites (`:1608`,`:1626`); regression test | closed |
| T-87-07 | Information Disclosure | audit-integrity bypass: `verify_audit_log` returns `is_valid()=true` for empty log with no stored metadata (CR-02) | mitigate | `records_verified = event_count > 0` (`audit.rs:1415`, not hardcoded); `is_valid()` (`:172-173`) ANDs it first → empty log false | closed |
| T-87-08 | Repudiation | Missing divergence record → future sync silently reverts hardening to upstream hardcoded-true | mitigate | `proj/ADR-87-cr02-audit-bypass.md` (references upstream `e9529312`, expect-conflict note, deliberate-divergence classification) | closed |
| T-87-09 | Tampering | Premature SEC-01/SEC-02 VERIFIED on Windows-only evidence | mitigate | `87-VERIFICATION.md` `status: human_needed`; SEC-01/SEC-02 explicitly PARTIAL→CI, not flipped to VERIFIED until GH Actions Linux Clippy green | closed |
| T-87-10 | Information Disclosure | Ledger CR-02 addendum missing | mitigate | `85-DIVERGENCE-LEDGER.md:807` "## Phase 87 CR-02 Addendum" with upstream ref + expected-conflict guidance | closed |
| T-87-SC | Tampering | npm/pip/cargo installs (Phase 87-01) | accept | No new packages; pure Rust edits to existing files (commit `6cf2645c` touches no Cargo.lock/toml) | closed |
| T-87-SC | Tampering | npm/pip/cargo installs (Phase 87-02) | accept | No new packages (commits `abeb2493`,`4a936f31` touch no Cargo.lock/toml) | closed |
| T-87-SC | Tampering | npm/pip/cargo installs (Phase 87-03 / CR-01 fix) | accept | No new packages (commit `718fe59d` touches no Cargo.lock/toml) | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-87-01 | T-87-05 | Null/zero `msg_name` connected-socket sends carry no per-call destination; the explicit continue fast-path is correct, not a bypass | Oscar Mack Jr | 2026-06-21 |
| AR-87-02 | T-87-SC | No package installs across any Phase 87 commit (pure Rust edits to existing files); supply-chain legitimacy gate N/A | Oscar Mack Jr | 2026-06-21 |
| AR-87-03 | T-87-01 / T-87-04 (WR-01) | TOCTOU on `msg_name` re-read under `SECCOMP_USER_NOTIF_FLAG_CONTINUE`, accepted on the single-threaded agent model (disclosed inline `supervisor_linux.rs:998-1016`) | Oscar Mack Jr | 2026-06-21 |
| AR-87-04 | T-87-03 (WR-03) | sendmmsg mediation is all-or-nothing (whole-call decision, not per-message), accepted seccomp-CONTINUE limitation (disclosed inline `supervisor_linux.rs:998-1016`) | Oscar Mack Jr | 2026-06-21 |
| AR-87-05 | T-87-01 (WR-02) | Kernels with Landlock V4 but no seccomp-BPF leave the SOCK_DGRAM bypass open; fails open by deliberate decision, logged to stderr (`exec_strategy.rs:1430-1437`) | Oscar Mack Jr | 2026-06-21 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-21 | 13 | 13 | 0 | gsd-security-auditor (verify-mitigations mode) |

---

## Residual / Deferred Verification (not open threats)

- SEC-01 / SEC-02 cross-target legs (Linux/macOS clippy + seccomp runtime tests) remain **PARTIAL→CI** pending GH Actions per T-87-09's own disposition — the correct documented state, not a gap. The mitigation *logic* is verified present in source; only live Linux execution is deferred.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-21
