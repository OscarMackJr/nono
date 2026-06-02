# Requirements: nono — v2.8 UPST7 + v2.7 Drain & Release

**Defined:** 2026-05-28
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms; every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees, or be explicitly documented as intentionally unsupported with a clear rationale.

**Trigger:** v2.7 close (2026-05-28) surfaced genuine new carry-forwards — two untagged post-`637a426c` fixes (`d8b7ce00` broker GLE=87, `005b4c9e` no-PTY relay stdout-echo) that the tagged v2.7 build lacks, plus WFP service-stop/uninstall fixes (`0cbeb3be` / `b852826b`), the WFP elevated live-uninstall UAT, an MSI rebuild, and 3 pending todos. In parallel, the UPST7 cadence trigger is met: the fork's confirmed sync high-water mark is upstream `v0.57.0` (UPST6 / Phase 48), so the forward gap is upstream `v0.58.0` + `v0.59.0` (~19-commit backlog) per the `260527-sgo` gap analysis (gap matrix + 6 phase buckets). This is a **drain-then-sync** milestone mirroring the v2.5/v2.6 shape: ship the release the tagged build needs, clear the debt, then absorb UPST7 in full.

## v1 Requirements (v2.8 Scope)

### Release & Distribution (RLS)

- [x] **REQ-RLS-01**: A v2.8 git tag is cut and **signed MSIs (machine + user)** are produced off the post-`005b4c9e` `nono.exe`, containing the untagged v2.7 fixes (`d8b7ce00` broker `CreateProcessAsUserW` GLE=87 HANDLE_LIST dedup, `005b4c9e` no-PTY relay stdout-echo, `0cbeb3be` + `b852826b` WFP service-stop + MSI-uninstall). An operator can install the signed v2.8 MSI; the bundled `nono.exe` reports the v2.8 fork version and runs correctly on the real-console no-PTY supervised path (the tagged v2.7 build's doubly-broken path is gone).
  - **SATISFIED (Phase 53 UAT-B re-run on v0.57.5, 2026-05-29):** the first attempt (v0.57.4) failed UAT-B — the installed MSI payload binaries were Authenticode `NotSigned` (MSI *wrapper* `Valid`), caused by a `release.yml` signing-order defect: `build-windows-msi.ps1` harvested **unsigned** binaries because signing ran *after* packaging. Fixed in `release.yml` (sign `nono.exe`/broker/wfp-service **before** the MSI build + a new `Verify MSI payload signatures` admin-extract gate). Re-released as **v0.57.5** (tag `a3927be0`); CI confirmed every MSI-embedded executable verifies `MSI payload Authenticode OK`, and the operator confirmed on a live install: `(Get-AuthenticodeSignature 'C:\Program Files\nono\nono.exe').Status` = `Valid` (broker + wfp-service likewise), `nono --version` → `0.57.5`. **Known gap (non-blocking):** the no-PTY supervised path could not be exercised on the Program-Files install — the `claude-code` profile does not cover `C:\Program Files\nono`, so launch is refused at the executable-coverage gate before reaching the broker; the no-PTY relay fix (`d8b7ce00` + `005b4c9e`) remains validated at dev-layout (v2.7 close). See `.planning/phases/53-release-drain/53-04-SUMMARY.md`.
- [x] **REQ-RLS-02**: `.github/workflows/release.yml` runs to completion on a `v*` tag push and produces the signed release artifacts — the chronic 0s `startup_failure` (broken `docker` reusable-call job removed in `5c90c4cf`, never live-verified) is resolved and confirmed live on a tag push.

### Drain (DRN)

- [x] **REQ-DRN-01**: WFP elevated live-uninstall is verified (HUMAN-UAT) — an operator running elevated `sc stop` of the WFP service then `msiexec /x` confirms the service stops cleanly and uninstall removes the service/driver, leaving nothing behind (closes the `wfp-service-stop-uninstall` debug's remaining live-verify leg). **SATISFIED (Phase 53 UAT-C, 2026-05-29):** all 5 steps passed on Windows 11 build 26200 — `sc stop` accepted (Fix #1), `nono setup --uninstall-wfp` removed both services (Fix #2a), `msiexec /x` left no service/driver/install-dir/filters (Fix #2b WiX CA), and the major-upgrade guard preserved services. Todo 1 closed to `todos/done/`.
- [x] **REQ-DRN-02**: The 3 pending follow-up todos in `.planning/todos/pending/` are resolved or explicitly re-dispositioned (carried since v2.7 close).

### Upstream Sync — Audit (UPST7)

- [ ] **REQ-UPST7-01**: A `DIVERGENCE-LEDGER.md` for upstream `v0.57.0..v0.59.0` is produced (mirroring Phase 42/47 shape): per-cluster dispositions (will-sync / fork-preserve / won't-sync / split), a `windows-touch` column, an `## ADR review` confirming (or revising) the Phase 33 Option A `continue` strategy, an `## Empirical cross-check` of re-export surfaces on fork-shared files (per the `feedback_cluster_isolation_invalid` lesson — diff-inspect, not just `--name-only`), and a fresh upstream re-fetch at audit-open that captures any `v0.59.x` patch releases cut after 2026-05-27.

### Upstream Sync — Cherry-pick wave (UPST7)

- [ ] **REQ-UPST7-02**: The cross-platform straight ports are absorbed per audit dispositions with verbatim D-19 `Upstream-commit:` trailers (or `Upstream-replayed-from:` for D-20 replays): JSONC profile parsing, `target_binary` profile field, `opencode` pack relocation, configurable timeout constants, `java-dev` profile / `java_runtime` group (with Windows-conditional JDK paths via `platform.rs`), proxy 502 hardening, and denial/diagnostic polish (suppressed-denial annotations, canonical denial-path precompute, access-mode `rfind` split, overflow-check tightening). Schema-collision checks run against the fork's canonical-sections; the D-43-E1 Windows-only-files invariant is respected.

### Network Filtering (NET)

- [ ] **REQ-NET-01**: `--allow-domain` accepts a URL with **path scoping** and **fine-grained HTTP method + path restrictions**, enforced in `nono-proxy` (`route`/`filter`/`server`), and TLS-intercept endpoint rules are evaluated **before** credential selection. Cross-platform; the change is diff-inspected against the fork-divergent TLS-interception surface (Phase 34 C11 `fork-preserve`) rather than blind-cherry-picked. `nono why --host` awareness of the new scoping is preserved.

### Credentials (CRED)

- [ ] **REQ-CRED-01**: A `bw://` Bitwarden credential source resolves secrets through the keystore abstraction alongside the existing `keyring://` / `env://` / `file://` schemes, with secret fields held in `Zeroizing<String>` (aligned with the fork's `zeroize` standard) and in-place truncation. Cross-platform; isolated surface in `crates/nono/src/keystore.rs`.

### Session Hooks (HOOK)

- [ ] **REQ-HOOK-01**: A `session_hooks` profile field runs vetted hooks at session start/stop. On Unix the upstream `hook_runtime` behavior is preserved (gated unix-only as upstream ships it); on Windows the hooks execute via a Windows-safe design (broker-spawned, Low-IL, no `fork`/`sh` assumption) documented in an ADR. Hook resolution or execution failure is **fail-closed**, never silently skipped.

### Supervisor IPC (IPC)

- [ ] **REQ-IPC-01**: The supervisor survives a transient child IPC close (keep-alive instead of dropping the supervisor loop), enforces bounded read-timeouts, and accepts connections robustly. The Unix side absorbs upstream's named-socket hardening (cross-platform-core); the Windows side translates the robustness intent onto the fork's Named-Pipe AIPC path (Phase 18) — a translate-not-cherry-pick.

### Windows Network Enforcement (WFP)

- [ ] **REQ-WFP-01**: Out-of-box WFP operational enforcement for supervised runs on a
  machine-MSI-installed Windows host. A `network.block:true` supervised `nono run` enforces
  WFP kernel network filtering without any manual `nono setup --start-wfp-service` step.
  Specifically: (1) the machine MSI registers `nono-wfp-service` with `start=auto` so the
  SCM boot-starts it as SYSTEM; (2) the control-pipe SDDL grants non-elevated supervised-run
  sessions (Interactive Users) read+write access; (3) when the service is not running at
  enforcement time, nono attempts an auto-start and if that fails returns a fail-closed error
  naming the exact remediation command — never passes through unenforced; (4) clean
  uninstall via `msiexec /x` still leaves nothing behind. Closes Phase 60's F-60-UAT-03.
  (v2.9 track, Phase 62)

## v2 Requirements (Deferred)

Acknowledged but not in the v2.8 roadmap.

### Broader heavy-runtime audit

- **REQ-WSRH-AUDIT-01** *(deferred from v2.7)*: Systematic audit of which other built-in profiles / heavy-runtime binaries (Electron/Node/CLR-class) hit the same `WriteRestricted` gate under `nono run`. v2.7 fixed the confirmed `claude.exe` case; a profile-wide audit is a follow-on.

### Release-pipeline attestation

- **REQ-RLS-ATTEST-01** *(deferred)*: Evaluate whether upstream's `actions/attest-build-provenance` build-provenance attestation composes with the fork's existing sigstore/TUF + Authenticode + MSI signing pipeline, or is superseded by it. CI-only and fork-divergent; sequence after `release.yml` is healthy. May fold into REQ-RLS-02 if cheap.

### Residual v0.44–v0.57 deferrals

- **REQ-UPST-RESID-01** *(deferred)*: `b5f0a3ab` deep ExecConfig refactor + `bbdf7b85` escape-quote structured-property wiring + the full `wiring.rs` idempotent JSON-merge abstraction — macOS-learn-diagnostics-oriented residue from the v0.44–v0.57 window; out of forward UPST7 scope.

### Deny-overlap validator preflight (carried from v2.7)

- **REQ-DENY-PREFLIGHT-01** *(deferred from v2.7 carry-forward, Phase 44 Plan 44-02 D-44-C3 follow-up)*: Linux-host-gated investigation of why `validate_deny_overlaps` pre-flight does not fire on CI Linux (the either-or assertion already proves security equivalence — both code paths deny the read, neither leaks the secret). This is a latent-diagnostic investigation, not a security gap. Deferred until a dedicated Linux CI runner is available for the instrumented-trace investigation (RUST_LOG=trace + strace). Source: `.planning/todos/pending/44-class-d-validator-preflight-investigation.md`. Rationale per D-53-08: security equivalence is proven; investigation is low-priority and Linux-host-gated. Priority: low. Affects: `crates/nono-cli/src/policy.rs:1032-1088`, `crates/nono-cli/tests/deny_overlap_run.rs`.

### Snapshot restore TOCTOU hardening (carried from v2.7)

- **REQ-UNDO-TOCTOU-01** *(deferred from v2.7 carry-forward, Phase 44 Plan 44-01 WR-01 P43 follow-up)*: Full fd-relative TOCTOU hardening of `validate_restore_target` + downstream write sequence in `crates/nono/src/undo/snapshot.rs`. Requires O_NOFOLLOW + openat/mkdirat/renameat/fchmodat on Linux/macOS and NtCreateFile-with-OBJ_DONT_REPARSE or documented defense-in-depth on Windows. Estimated ~2-3 weeks of focused cross-platform work. Warrants a dedicated security-scoped phase. Source: `.planning/todos/pending/44-validate-restore-target-fd-relative-hardening.md`. Rationale per D-53-08: residual TOCTOU race is a known local-attacker-with-write-access scenario; doc note already shipped (Phase 44); full closure is a standalone security phase, not a drain item. Priority: medium. Affects: `crates/nono/src/undo/snapshot.rs`.

## Out of Scope (Explicit Exclusions)

| Feature | Reason |
|---------|--------|
| WR-02 EDR telemetry HUMAN-UAT | v3.0-deferred pending an EDR-instrumented runner (re-affirmed every milestone since v2.1). |
| Gap 6b — runtime trust interception via kernel minifilter | Requires a signed kernel driver; deferred to v3.0. |
| macOS-only v0.58/v0.59 items (`$PWD` symlink-CWD capture, platform-rules-after-user-write-allows ordering) | Not Windows-relevant; absorb only if/when the macOS Seatbelt layer is exercised. Tracked in the UPST7 ledger as `unix-only-N/A`. |
| CI repo-hygiene (PR size labels, PR-summary workflow, artifact-job reorder) | Low value, fork-divergent CI; not phase-worthy. |
| Routine dependency bumps (`shlex`, `serde_json`, `similar`, docker/* actions) | Absorbed on the fork's own cadence; only `landlock`/`sigstore` security-relevant bumps are considered, and not as standalone requirements. |

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| REQ-RLS-01 | Phase 53 | Complete (v0.57.5 re-release; MSI payloads signed + UAT-B PASS) |
| REQ-RLS-02 | Phase 53 | Complete |
| REQ-DRN-01 | Phase 53 | Complete |
| REQ-DRN-02 | Phase 53 | Complete |
| REQ-UPST7-01 | Phase 54 | Pending |
| REQ-UPST7-02 | Phase 55 | Pending |
| REQ-NET-01 | Phase 56 | Pending |
| REQ-CRED-01 | Phase 57 | Pending |
| REQ-HOOK-01 | Phase 58 | Pending |
| REQ-IPC-01 | Phase 59 | Pending |
| REQ-WFP-01 | Phase 62 | Complete |
| REQ-DENY-PREFLIGHT-01 | v2 Deferred | Deferred |
| REQ-UNDO-TOCTOU-01 | v2 Deferred | Deferred |

**Coverage:**
- v1 requirements: 11 total
- Mapped to phases: 11/11 ✓
- Unmapped: 0

---
*Requirements defined: 2026-05-28*
*Last updated: 2026-05-28 — traceability populated at roadmap creation*
