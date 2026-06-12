# Phase 67: Clean-Host Windows Install - Context

**Gathered:** 2026-06-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the **public machine MSI** install to completion on a **fresh Windows 11 host** (no VC++ x64 runtime, no pre-trusted nono cert) and let the **supervised/broker path** (`nono run --profile claude-code`) run there — the **cert-independent half** of "the public release works out-of-the-box."

In scope: VC++ runtime handling (DIST-01), non-fatal service install (DIST-02), an interim auditable broker-trust helper + cert shipping + docs (TRUST-01/02).

**Explicitly NOT in scope:** real publicly-trusted code signing (Azure Trusted Signing) — BLOCKED on an incoming cert, deferred to the next (enterprise distribution) milestone as `DIST-SIGN-01`. v2.11's TRUST work is the **interim** path only; it never weakens the D-32-12 broker self-trust gate. Silent/headless fleet deployment (GPO/SCCM/Intune) is the enterprise milestone (SEED-001).

</domain>

<decisions>
## Implementation Decisions

### VC++ Runtime Strategy (DIST-01)
- **D-01:** Satisfy the VC++ x64 runtime by building **both `nono.exe` and `nono-wfp-service.exe` with the static CRT (`+crt-static`)** — eliminate the runtime dependency entirely rather than bundling/declaring a redistributable. Rationale: works on any clean host with zero redist, no MSI-size or redist-version coupling, and is the ideal posture for the future enterprise silent-deploy milestone. Accept slightly larger binaries.
- **D-02:** The broker binary (`nono-shell-broker.exe`, `crates/nono-shell-broker/`) and any other shipped exe must link the **same way** — verify the *whole* shipped binary set is CRT-dependency-free, not just `nono.exe`.
- **D-03 (research-gated):** Static CRT must be proven not to break any workspace crate (notably `ring`/`aws-lc-sys`, `windows-sys`, the WFP service, sigstore deps) and must not disturb the Authenticode signing flow. If a hard conflict surfaces, the fallback is the **VC++ merge module in WiX** (not the "declared prereq" option, which was rejected). The planner should treat static CRT as the primary and carry the merge-module fallback as a documented contingency.

### Service-Start Posture (DIST-02)
- **D-04:** Keep `nono-wfp-service` **`Start="auto"`** (preserve Phase 62's out-of-box WFP kernel enforcement when the service can start) but make the **install-time start non-fatal** so a start failure can never roll back the whole product — the install completes and leaves a usable `nono.exe`.
- **D-05:** Preferred WiX mechanism (research to confirm): make the `<ServiceInstall>`/`<ServiceControl>` **`Vital="no"`** and/or drop the install-blocking `<ServiceControl Start="install">` so SCM start failure (event 7009) doesn't abort the sequence. The **clean-uninstall invariant is non-negotiable**: `<ServiceControl Remove="uninstall">` must still fire and leave no orphaned WFP filters / service registration / EventLog source.
- **D-06:** Note: with static CRT (D-01) the *missing-CRT* trigger for the rollback disappears; DIST-02 still ships as general robustness (no service hiccup of any cause should roll back the product).

### Broker Trust Helper (TRUST-01)
- **D-07:** Add `nono setup --trust-broker` (and a paired `nono setup --untrust-broker`) as new flags on the existing `SetupArgs` (`crates/nono-cli/src/cli.rs:2479`, handled in `setup.rs`). It imports **only the one shipped POC code-signing cert, pinned by thumbprint**, into **LocalMachine `Root` + `TrustedPublisher`** so the D-32-12 broker self-trust gate passes.
- **D-08:** The helper **self-elevates via UAC** (re-spawn elevated, e.g. `ShellExecute "runas"`) rather than erroring on a non-elevated shell — clean-host UX is the priority.
- **D-09:** **Reversible** — `--untrust-broker` removes exactly what `--trust-broker` added (pinned thumbprint, both stores).
- **D-10 (security invariant):** The helper is scoped, explicit, and auditable: it states *what* cert (subject + thumbprint) it trusts and *why*, trusts only that one POC cert, and **never** disables/bypasses the D-32-12 gate or trusts an unsigned / wrong-signer binary. It is an operator-auditable trust-store import, not a gate weakening.

### Cert Shipping & Discoverability (TRUST-02)
- **D-11:** **Ship the `.cer` inside the MSI payload** (alongside the binaries) so `--trust-broker` works fully offline with nothing to download.
- **D-12:** Make **`nono setup --check-only` warn when the broker cert is untrusted** on the current host (it already reports the broker Authenticode subject + thumbprint per the v2.10 work) — discoverability so a clean-host operator learns they need `--trust-broker`.
- **D-13:** Update the docs (`docs/cli/development/windows-signing-guide.mdx`) to **prominently** state: public releases use a self-signed **POC cert** for the supervised path; the supported clean-host path is `nono setup --trust-broker`; real publicly-trusted signing (`DIST-SIGN-01`) is the deferred replacement.

### Claude's Discretion
- Exact WiX attribute combination for non-fatal start (`Vital`/`ErrorControl`/dropping `Start="install"`) — pick the minimal change that satisfies D-04/D-05, validated against `validate-windows-msi-contract.ps1`.
- Static-CRT wiring mechanism (`.cargo/config.toml` `rustflags` vs per-target vs `RUSTFLAGS` in the build/release scripts) — pick what the release pipeline (`release.yml`) can carry cleanly across all build legs.
- The precise self-elevation + cert-import API (`certutil` shell-out vs `windows-sys` `CertAddCertificateContextToStore` to `LocalMachine`) — security-equivalent; pick the more testable/maintainable.

### Folded Todos
- **`msi-vcredist-prereq`** (`.planning/todos/pending/20260611-msi-vcredist-prereq.md`) — machine MSI 1603 on a clean host (no VC++ runtime: `nono.exe` `0xC0000135`, `nono-wfp-service.exe` can't load → SCM 7009 → full rollback). Resolved by D-01 (static CRT removes the dep) + D-04/D-05 (non-fatal start removes the rollback).
- **`poc-cert-broker-clean-host`** (`.planning/todos/pending/20260611-poc-cert-broker-clean-host.md`) — release signed with untrusted `CN=nono Test Signing` POC cert → D-32-12 gate refuses the broker → supervised path non-functional out-of-box. Resolved (interim) by D-07..D-13. The real fix (publicly-trusted signing) is `DIST-SIGN-01`, deferred to the enterprise milestone.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & milestone scope
- `.planning/REQUIREMENTS.md` — DIST-01, DIST-02, TRUST-01, TRUST-02 (acceptance criteria); v2/Out-of-Scope context (DIST-SIGN-01 deferral).
- `.planning/ROADMAP.md` § Phase 67 — goal, 4 success criteria, host gate.

### MSI build (DIST-01/02) — the .wxs is GENERATED, edit the script
- `scripts/build-windows-msi.ps1` — the machine + user MSI source. **The `dist/windows/nono-machine.wxs` is regenerated/overwritten from here-strings in this script on every build — edit the script, NOT the .wxs** (see memory `windows_msi_wxs_is_generated`). Current service surface at lines ~213-262: `<ServiceInstall Start="auto" ErrorControl="normal">` + `<ServiceControl Start="install" Remove="uninstall">` + EventLog source registration; driver is deliberately NOT a `<ServiceInstall>`.
- `scripts/validate-windows-msi-contract.ps1` — MSI contract validator; any service/CRT change must keep this green (it also references `-BrokerPath`).

### Broker trust gate (TRUST-01/02)
- `docs/architecture/broker-trust-anchor.md` — the D-32-11..14 broker self-trust-anchor design (`verify_broker_authenticode` at every `BrokerLaunch` dispatch; no cache/escape-hatch). The gate `--trust-broker` makes pass — MUST NOT be weakened.
- `crates/nono-cli/src/exec_identity_windows.rs` — `WinVerifyTrust` Authenticode query + chain walker (`Valid`/`Unsigned`/`InvalidSignature{hresult}`); the mechanism behind the gate and the `--check-only` thumbprint report.
- `crates/nono-cli/src/cli.rs:2479` — `SetupArgs` (`check_only`, `refresh_trust_root`, `from_file`); add `--trust-broker` / `--untrust-broker` here.
- `crates/nono-cli/src/setup.rs` — `SetupRunner`; `--check-only` already reports broker subject + thumbprint → wire the untrusted-warning + the trust/untrust handlers here.
- `docs/cli/development/windows-signing-guide.mdx` — TRUST-02 docs target (the POC-cert limitation + `--trust-broker` path).

### Deferred real-signing (context only — NOT this phase)
- `.github/workflows/trusted-signing-smoke.yml` — the Azure Trusted Signing surface for the deferred `DIST-SIGN-01`; referenced so the planner does NOT pull it into Phase 67.

### Cross-target / build discipline
- `.planning/templates/cross-target-verify-checklist.md` — static-CRT touches build config; any cfg-gated Unix code change needs the Linux+macOS clippy verification (CLAUDE.md MUST).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `SetupArgs` + `SetupRunner` (`cli.rs` / `setup.rs`): `--trust-broker`/`--untrust-broker` slot in next to the existing `--check-only`/`--refresh-trust-root`/`--from-file` flags; `--check-only` already surfaces the broker Authenticode subject + thumbprint, so the "untrusted" warning (D-12) is an extension, not new plumbing.
- `exec_identity_windows.rs` `WinVerifyTrust` query: reuse for "is the broker cert currently trusted on this host?" (drives D-12) — same call the gate uses.
- `scripts/build-windows-msi.ps1` here-string WiX generation: the single place to change service-install attributes (D-04/D-05) and add the `.cer` payload (D-11).

### Established Patterns
- **The .wxs is generated** (memory `windows_msi_wxs_is_generated`): all MSI edits go through `build-windows-msi.ps1` + `validate-windows-msi-contract.ps1`, never the emitted `.wxs`.
- **D-32-12 fail-secure broker gate**: the broker only spawns from a signed Program-Files install with a trusted cert. `--trust-broker` makes the host trust the cert; it does not change the gate. Dev-layout `target\release\nono.exe` skips the gate (don't UAT trust there).
- **Clean-uninstall invariant** (Phase 62): `<ServiceControl Remove="uninstall">` + orphan WFP-filter sweep must survive any DIST-02 change.

### Integration Points
- `.cargo/config.toml` (or release-script `RUSTFLAGS`): new `+crt-static` target rustflags (DIST-01) — currently absent (greenfield).
- `release.yml` build legs must carry the static-CRT flag consistently (cross-target drift hazard — CI is the signal).
- MSI payload list in `build-windows-msi.ps1`: add the `.cer` file (D-11).

</code_context>

<specifics>
## Specific Ideas

- Static CRT is preferred specifically because it's the cleanest base for the **next** milestone's silent/fleet deployment (no per-host redist step) — keep that forward-compatibility in mind.
- The trust helper must read as an **operator-auditable, reversible, single-cert** action — explicitly NOT a "disable security" switch. Wording in `--trust-broker` output and docs matters (it's a POC-trust convenience, retired by `DIST-SIGN-01`).

</specifics>

<deferred>
## Deferred Ideas

- **Real publicly-trusted code signing (Azure Trusted Signing) — `DIST-SIGN-01`**: the permanent replacement for the `--trust-broker` interim helper. BLOCKED on an incoming cert; anchor of the next (enterprise distribution) milestone. Do NOT implement in Phase 67.
- **Silent/headless fleet deployment** (GPO/SCCM/Intune, MSIX, machine-wide provisioning) — SEED-001, enterprise milestone.

### Reviewed Todos (not folded)
- **`macos-resl-enforcement-broken`** (`.planning/todos/pending/20260611-macos-resl-enforcement-broken.md`) — matched fuzzily by `todo.match-phase` but belongs to **Phase 68** (macOS resl fix), already linked there. Not folded into Phase 67.

</deferred>

---

*Phase: 67-clean-host-windows-install*
*Context gathered: 2026-06-12*
