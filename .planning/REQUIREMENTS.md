# Requirements: nono — v2.11 Clean-Host Distribution Cleanup + UPST8

**Defined:** 2026-06-11
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms; every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees, or be explicitly documented as intentionally unsupported with a clear rationale.

**Trigger:** v2.10 shipped 2026-06-11. Its EDR HUMAN-UAT (Phase 66) and macOS live re-validation (gate-65-A) surfaced concrete defects that make the *public release* not work out-of-the-box on a clean host: the machine MSI rolls back on a host with no VC++ runtime, the supervised/broker path is refused because releases are signed with an untrusted self-signed POC cert, and macOS `--timeout`/`--max-processes` enforcement genuinely does not fire on a real host. The headline fix — **publicly-trusted code signing (Azure Trusted Signing)** — is **blocked on an incoming cert**, so v2.11 is scoped to the **cert-independent cleanup** done while the cert is in flight. The big enterprise distribution effort (trusted signing + silent fleet deployment, SEED-001…005) is the milestone after. UPST8 (the non-macOS upstream slice `v0.60.0..v0.61.2`, deferred at Phase 54) rides along.

## v1 Requirements (v2.11 Scope)

### Clean-Host Distribution (DIST)

- [ ] **DIST-01**: An operator can install the **machine MSI** to completion on a **fresh Windows 11 host with no VC++ x64 runtime pre-installed**, with no manual `vc_redist.x64.exe` step. The VC++ runtime dependency is satisfied structurally — by bundling the redistributable merge module / chaining the redist install, by building the Rust binaries with the static CRT (`+crt-static`), or by a declared-and-checked prerequisite — chosen during planning. Acceptance: a clean-host install does not fail `1603` for a missing CRT (`0xC0000135` STATUS_DLL_NOT_FOUND on `nono.exe`/`nono-wfp-service.exe`).
- [ ] **DIST-02**: A `nono-wfp-service` **start failure during MSI install does not roll back the entire product** — the install completes and leaves a usable `nono.exe`. The `ServiceControl` start is made non-fatal / deferred so an SCM start timeout (event 7009) cannot abort the whole install. The clean-uninstall invariant (no orphaned WFP filters / service registration) is preserved.

### Broker Trust on Clean Host (TRUST)

- [ ] **TRUST-01**: `nono setup --trust-broker` (or an equivalent helper) imports the **shipped code-signing cert** into the appropriate local trust stores (LocalMachine `Root` + `TrustedPublisher`) so the D-32-12 broker self-trust gate passes, letting `nono run --profile claude-code` spawn the broker on a clean host without manual `certmgr`/`Import-Certificate` steps. The helper is explicit and auditable (it states what it trusts and why); it never silently weakens the gate for an untrusted binary.
- [ ] **TRUST-02**: The clean-host trust limitation and the supported interim trust path are **clearly documented** (signing guide / POC runbook, e.g. `docs/cli/development/windows-signing-guide.mdx`), and the cert + import step ship with the release. The doc states plainly that public releases use a self-signed POC cert for the supervised path until publicly-trusted signing lands, and points at `TRUST-01` as the supported path.

### macOS Resource Limits (RESL-MAC)

- [ ] **RESL-MAC-01**: `nono run --timeout <D>` **SIGKILLs the child at the deadline on a real macOS host** — the supervisor wall-clock watchdog fires. Validated by `macos_timeout_kills_at_deadline` passing with `NONO_RESL_HOST_VALIDATED=1` on a real macOS host. (The watchdog is nono's own cross-platform supervisor code; its non-firing on macOS is a nono bug to fix, not an OS limitation.)
- [ ] **RESL-MAC-02**: `nono run --max-processes <N>` makes the child's `fork()` **fail (EAGAIN) past the cap on a real macOS host**. Validated by `macos_max_processes_blocks_on_rlimit_nproc` passing with `NONO_RESL_HOST_VALIDATED=1`. The macOS `setrlimit(RLIMIT_NPROC)` approach is applied to the child before `exec` (noting `RLIMIT_NPROC` on macOS counts all per-UID processes and may need a different bounding strategy than Linux `pids.max`).

### Upstream Sync (UPST8)

- [x] **UPST8-01**: A `DIVERGENCE-LEDGER.md` **audits upstream `always-further/nono` `v0.60.0..v0.61.2`** scoped to the **non-macOS** surface (the macOS slice was absorbed in v2.10), inventorying every relevant commit with per-commit dispositions (will-sync / fork-preserve / won't-sync / split), a `windows-touch` column, an ADR-cadence review (per the Phase 33 Option A `continue` rule), and a diff-inspect note per the `feedback_cluster_isolation_invalid` lesson. Re-fetch upstream at audit-open and record the head SHA.
- [ ] **UPST8-02**: The **will-sync upstream commits are cherry-picked** with verbatim D-19 `Upstream-commit:` trailer blocks, preserving the Windows-only-files invariant (no `*_windows.rs` / `exec_strategy_windows/` drift from upstream) and the fork-divergence catalog; D-20 manual replays are used where direct cherry-pick conflicts dominate. Cross-target clippy (Linux + macOS) verified per `.planning/templates/cross-target-verify-checklist.md`; the full workspace test suite passes post-sync.

## v2 Requirements (Deferred to the Enterprise Distribution Milestone)

- **DIST-SIGN-01** *(BLOCKED on incoming cert)*: Public releases are Authenticode-signed with a **publicly-trusted** code-signing cert (EV/OV, e.g. Azure Trusted Signing via the existing `trusted-signing-smoke.yml`) so `Get-AuthenticodeSignature` returns `Valid` on any clean host and the broker gate passes with **no manual cert trust** — the real fix that supersedes the `TRUST-01` interim helper.
- **DIST-SILENT-01** *(SEED-001, P0)*: Silent/headless/unattended install + GPO/SCCM/Intune packaging + machine-wide service / invariant env-var provisioning + auto-provisioned secure scratch space (eliminates the manual profile-owned-CWD requirement).
- **ENT-EGRESS-01** *(SEED-002, P1)*: Enterprise-policy-managed network egress allowlists (corporate domains + AI-provider wildcards) reconciling `nono-proxy` + `nono-wfp-service` into one default-deny egress story.
- **ENT-SIEM-01** *(SEED-003, P2)*: Structured security-event telemetry (blocked/denied actions) to Windows Event Log / Syslog for SIEM/EDR consumption.
- **ENT-MULTI-01** *(SEED-004, P3)*: Multi-engine pluggability — a security daemon confining any `AI_AGENT`-labeled token, not just Claude Code (likely warrants a `/gsd:spike`).
- **ENT-ATTEST-01** *(SEED-005, P3)*: Signed/attested policy overrides via the external ZT-Infra ledger (external dependency).
- **DRV-PROD-01** *(gated No-go/Conditional-go per ADR-65)*: Production EV/WHQL-signed Gap 6b minifilter.

## Out of Scope (Explicit Exclusions)

| Feature | Reason |
|---------|--------|
| Real publicly-trusted code signing (Azure Trusted Signing) | BLOCKED on an incoming cert; intentionally the anchor of the next (enterprise distribution) milestone, not v2.11. The `TRUST-01` interim helper is the stopgap. |
| Silent/headless fleet deployment (GPO/SCCM/Intune, MSIX) | Enterprise-distribution milestone (SEED-001); v2.11 only makes the *interactive* clean-host install work. |
| macOS UPST slice of `v0.60.0..v0.61.2` | Already absorbed in v2.10 (MACOS-01/02/03); UPST8 covers only the non-macOS remainder. |
| Production EV/WHQL minifilter (DRV-PROD-01) | Gated No-go/Conditional-go per ADR-65; revive only on a concrete capability gap AppContainer/WFP can't express. |
| Weakening the broker self-trust gate | `TRUST-01` imports a cert into trust stores (operator-auditable); it never bypasses the D-32-12 gate or trusts an unsigned/wrong-signer binary. |

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| DIST-01 | Phase 67 — Clean-Host Windows Install | Not started |
| DIST-02 | Phase 67 — Clean-Host Windows Install | Not started |
| TRUST-01 | Phase 67 — Clean-Host Windows Install | Not started |
| TRUST-02 | Phase 67 — Clean-Host Windows Install | Not started |
| RESL-MAC-01 | Phase 68 — macOS Resource-Limit Enforcement Fix | Not started |
| RESL-MAC-02 | Phase 68 — macOS Resource-Limit Enforcement Fix | Not started |
| UPST8-01 | Phase 69 — UPST8 Audit | Not started |
| UPST8-02 | Phase 70 — UPST8 Cherry-pick Sync | Not started |

**Coverage:** 8/8 v1 requirements mapped to exactly one phase. No orphans, no duplicates. Phases 67 and 68 are independent (parallel-safe, each host-gated); Phase 69 → 70 is the UPST8 audit-then-sync pair (linear, cadence-ordered after Phase 55).
