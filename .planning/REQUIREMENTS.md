# Requirements: nono v3.5 — Trusted Signing Go-Live + First Distributed Release

**Defined:** 2026-07-02
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and, for v3.5, actually *distributable*: a publicly-trusted-signed release that runs out-of-the-box on a clean host, published live under fork-owned identities.

**Scope:** four pillars — (1) resolve the Azure Trusted Signing verify-gate `UnknownError` and harden the CI Authenticode verify; (2) cut the first publicly-trusted-signed `0.66.1` release; (3) publish `0.66.1` live to crates.io + PyPI + npm under **fork-owned `nono-sandbox` identities** (upstream owns the `nono*` names); (4) prove out-of-the-box clean-host behavior on a fresh Azure Win11 VM, draining both host-gated distribution todos, then close out the POC signing path. **Full go-live EXECUTE posture (operator-in-loop).**

> **Architecture invariants:**
> - **`UnknownError` is a disambiguable 3-cause taxonomy**, not a mystery: (a) the profile is `PublicTrustTest` not `PublicTrust` (Azure CLI has no plain "Test" type — the cookbook's "Test" = `PublicTrustTest`, whose issuer naming reproduces the exact 2026-06-30 symptom); (b) runner-side chain-build staleness from Azure's March-2026 AOC/EOC CA rotation; (c) CRL/OCSP revocation timeout. Probes: `az trustedsigning certificate-profile show` → confirm `profileType == PublicTrust`; `signtool verify /pa /v`; revocation-disabled diagnostic.
> - **Never loosen the fail-closed verify.** `release.yml`'s `Get-AuthenticodeSignature -ne 'Valid'` gate stays fail-closed — the fix is a robuster chain build + `signtool /pa` fallback + clearer diagnostics, NOT accepting `UnknownError`.
> - **Verify-gate debugging is CI-side** (GitHub `windows-latest`, a clean cloud host); only the behavioral install/broker-spawn/no-VC++ tests need the Azure VM. The operator's corporate host is NOT a valid clean host (POC cert previously imported + VC++ installed + corporate proxy/EDR/managed-trust-store confound the exact chain-build/revocation failure).
> - **Registry identities are upstream-owned.** crates.io `nono`/`nono-proxy`, PyPI `nono-py`, npm `nono-ts` belong to the upstream maintainer (lukehinds). The fork publishes under **`nono-sandbox`** family names it owns — the product/binary name `nono` (`[[bin]] name`), the `nono` lib name (`[lib] name`, so `use nono::` is unchanged internally), and the `OscarMackJr/nono` GitHub repo all stay as-is; only the `[package] name` registry identities change.
> - **Per-registry idempotency, not a uniform retry loop.** crates.io has no skip flag (dependency-order manual resume + index-visibility polling); PyPI uses `twine upload --skip-existing`; npm is atomic-per-version. A single uniform retry across all three is an anti-feature.
> - **Azure VM = Gen2 + Trusted Launch** (`--enable-secure-boot true --enable-vtpm true`); image `MicrosoftWindowsDesktop:windows-11`, SKU resolved live via `az vm image list-skus` (never hardcoded); ephemeral (create → UAT → teardown), never persistent. Do NOT pull the Trusted Signing cert as a PFX from Key Vault (unsupported; produces an untrusted chain that mimics this bug); no Terraform/Pulumi (repo is `az`-native).
> - **Secret-retirement ordering:** retire the POC signing secrets + delete the smoke workflow ONLY after clean-host UAT (Gate 3) passes — never merely after the Release workflow is green (Gate 2), so a recovery path survives if UAT reveals a problem.
> - **Repo stays PUBLIC** (minifilter altitude received; go-private retired) — no `build_notes/`/`.gsd/` staged before any push. Unlike prior prepare-only tags, v3.5's tag push is *intended* (go-live). All commits DCO-signed.

## v1 Requirements

### Trusted Signing Verify-Gate (SIGN)

- [x] **SIGN-01**: The Azure Trusted Signing certificate profile is confirmed to be type **`PublicTrust`** — the operator runs `az trustedsigning certificate-profile show` and verifies `profileType == PublicTrust`; if it is `PublicTrustTest` (the likely root cause — its issuer naming matches the observed `…Enterprise ID Verified Policy AOC CA…`), a `PublicTrust` profile is created and `TRUSTED_SIGNING_PROFILE` (+ the corrected FIC subject `repo:OscarMackJr/nono:environment:Development`) are updated. The finding is recorded (profile type + issuer chain) so the root cause is documented, not guessed. **CONFIRMED 2026-07-02** via Azure Portal (CLI extension unavailable — corporate-TLS SSL error): live account `ArtifactNono` / RG `RG_Nono` is already `PublicTrust`; no fix needed. The observed `UnknownError` is therefore NOT a profile-type issue — root cause remains open, routed to D-04 chain diagnostics on a future smoke run. See `101-SIGN01-FINDING.md`.
- [x] **SIGN-02**: The CI Authenticode verify is hardened into a single shared `scripts/verify-authenticode.ps1` helper, dot-sourced by both fail-closed verify sites in `release.yml` and the smoke workflow — it adds a `signtool verify /pa /v` deep check, builds/repairs the cert chain and handles CRL/OCSP-revocation transients, and reports chain-build failure distinctly from a genuine untrusted root — WITHOUT ever loosening the fail-closed `Status -ne 'Valid'` contract.
- [x] **SIGN-03**: The **"Trusted Signing Smoke Test"** workflow runs GREEN on GitHub's clean `windows-latest` runner (Gate 1) — a throwaway exe signs and verifies `Valid` with an issuer chaining to a public `Microsoft ID Verified CS EOC/AOC CA NN` root (not `PublicTrustTest`, not the POC root) — proving the signing path is live end-to-end before any release is cut. **FAILED 2026-07-03 (RED, diagnosed)**: operator authorized push + live dispatch; branch pushed, workflow dispatched twice. Run `28636000664` failed on a real wiring defect (missing `actions/checkout`, fixed in `83eefe11`). Run `28636133664` (authoritative) ran the real hardened verify path and still FAILED: `Status: UnknownError`, issuer `CN=Microsoft Enterprise ID Verified Policy AOC CA 02`, on a genuinely-signed `PublicTrust`-profile binary — the fail-closed gate correctly retried the transient case and then refused to pass (no loosening). This also **disproves** the research's issuer-naming heuristic (`Enterprise ID Verified Policy` is NOT a reliable `PublicTrustTest` tell — `PublicTrust` chains through it too). Root cause is a runner-side chain-build/revocation-validation issue on `windows-latest`, not a profile-type or gate defect. Full evidence + 3 findings + Phase 104 hand-off recorded in `101-SIGN03-SMOKE-VERDICT.md`. **PASSED 2026-08-16 (GREEN) — smoke run `31947685981`.** Azure Trusted Signing confirmed working end-to-end; the 2026-07-29 regression (HTTP 403 at Sign, suspected lapsed identity validation) is **RESOLVED** — the operator's Portal-side identity-validation fix took, and CI simply had not re-run since. Verbatim evidence: `Pre-sign status: NotSigned  (expected NotSigned)` / `Signing completed with status 'Succeeded' in 4.5054432s` / `Status: Valid` / `Issuer: CN=Microsoft ID Verified CS EOC CA 04, O=Microsoft Corporation, C=US`. OIDC subject `repo:OscarMackJr/nono:environment:Development`. This meets SIGN-03's literal bar on every clause: `Valid` (not `UnknownError`), a public `Microsoft ID Verified CS EOC CA NN` issuer (not `PublicTrustTest`, not the POC root), with the `NotSigned` pre-check proving the run actually performed the signing rather than re-verifying an already-signed artifact. The FAILED narrative above is retained deliberately: it records the two diagnosed root causes and Finding B (the disproved issuer-naming heuristic), which remain valid lessons.

### Trusted-Signed Release Go-Live (REL)

- [ ] **REL-01**: The first publicly-trusted-signed release is cut at **`0.66.1`** (Gate 2) — a tag push runs the `Release` workflow to green with all top-level `.exe` (`nono.exe`, `nono-shell-broker.exe`, `nono-wfp-service.exe`) and both MSIs Trusted-Signed and passing the hardened fail-closed verify; the published `OscarMackJr/nono` GitHub Release artifacts show **Verified publisher** — `Get-AuthenticodeSignature.Status -eq 'Valid'` plus a non-test signer (rejecting `CN=nono Test Signing`/`PublicTrustTest`), with the issuer captured informationally only, never gated on an issuer-substring match (per `101-SIGN03-SMOKE-VERDICT.md` Finding B, which disproved the prior issuer-naming heuristic: a genuine `PublicTrust` signature can chain through `Microsoft Enterprise ID Verified Policy AOC CA NN`).

### Fork-Owned Multi-Registry Publish (PUB — FUT-01)

- [x] **PUB-01**: The published package identities are renamed to fork-owned **`nono-sandbox`** family names — crates.io `nono-sandbox` / `nono-sandbox-proxy` / `nono-sandbox-cli`, PyPI `nono-sandbox`, npm `@oscarmackjr/nono-ts` — via `[package] name` (crates) + binding manifest changes only; the `[[bin]] name = "nono"`, `[lib] name = "nono"`, and the `OscarMackJr/nono` repo are unchanged, internal path-dep names/pins are reconciled, each new name's availability is confirmed on its registry, and the workspace + both binding builds are green. *(Plans 102-01/102-02/102-03/102-04 of 5 complete 2026-07-03: crates.io rename `[package] name` + `package=` reconciliation done, workspace `cargo build --workspace --all-targets` green, all 5 registry names live-confirmed available; Makefile + 4 permanent CI workflows reconciled to the renamed `-p nono-sandbox`/`-p nono-sandbox-cli` selectors, `make build`'s constituent commands + `cargo fmt --all -- --check` green; `../nono-py` Cargo.toml/pyproject.toml patched with `package=`/`[project] name = "nono-sandbox"`, `maturin build` green, `cargo tree` confirms resolution, DCO-signed commit `787e2dd` in the nono-py repo; `../nono-ts` Cargo.toml patched with `package=`, npm identity hand-rescoped to `@oscarmackjr/nono-ts` across package.json + all 4 platform subpackages + optionalDependencies (installed `napi rename` tool proved unusable — non-interactive-incapable and would have corrupted `nono-ts`'s own crate name), `napi build --platform --release` green, `cargo tree` confirms resolution, DCO-signed commit `c2f5aaa` in the nono-ts repo. Plan 102-05 (phase gate) closed 2026-07-03: fresh 5-way registry re-check (all 404, no same-window squat) + `cargo build --workspace --all-targets` + both `make build` constituent commands + `maturin build` + `napi build --platform --release` all green + all 4 SC1-SC4 confirmed against live state across all 3 repos. Phase 102 COMPLETE.)*
- [ ] **PUB-02**: `0.66.1` is published **LIVE** to all three registries under the fork-owned identities — crates.io in dependency order (`nono-sandbox` → `nono-sandbox-proxy` → `nono-sandbox-cli`) with index-visibility polling (not a fixed `sleep`), PyPI via maturin + `twine upload --skip-existing`, npm `@oscarmackjr/nono-ts` (all platform-specific packages present, avoiding the documented missing-platform-package failure) — each registry's own idempotency/resume semantics respected; `cargo install nono-sandbox-cli`, `pip install nono-sandbox`, and `npm i @oscarmackjr/nono-ts` all resolve post-publish.

### Clean-Host UAT — Azure VM (CHOST — FUT-03)

- [x] **CHOST-01**: A reproducible fresh-Win11 clean-host is stood up from fork-owned IaC (`az` CLI / Bicep under `scripts/azure/`) — a Gen2 + Trusted-Launch (vTPM) `windows-11` VM (SKU resolved live, not hardcoded), never-trusted-POC-cert, no VC++ runtime, RDP-reachable — with a documented create → use → teardown lifecycle (ephemeral, never persistent).
- [x] **CHOST-02**: Two new unattended `verify-dark.ps1` gates assert clean-host trust and plug into the existing gate-discovery harness (emitting `SKIP_HOST_UNAVAILABLE` when no clean host is present): a `trusted-signed-assertion` gate (Authenticode `Valid`, Issuer = `Microsoft ID Verified CS` root, reusing the SIGN-02 shared helper) and a **self-contained** `broker-spawn-on-clean-host` gate (install → `nono run --profile claude-code` spawns the broker with NO manual cert import → uninstall; ordered-safe under a `-All` sweep).
- [ ] **CHOST-03**: On the Azure VM, both host-gated distribution todos are drained to a genuine PASS (Gate 3) — `poc-cert-broker-clean-host` (the broker spawns out-of-box on the trusted-signed `0.66.1` release) via CHOST-02, and `msi-vcredist-prereq` (the machine MSI installs on fresh Win11 with no VC++ redist — no `1603`/rollback, `nono.exe` launches, no `0xC0000135`) via the existing `clean-host-install.ps1` gate — and both todos are moved `pending/` → `resolved/`.

### Close-Out (CLOSE)

- [ ] **CLOSE-01**: The POC signing path is retired **only after** clean-host UAT (Gate 3) passes — POC secrets (`WINDOWS_SIGNING_CERT` / `WINDOWS_SIGNING_CERT_PASSWORD`) deleted, `trusted-signing-smoke.yml` removed, the stale `docs/cli/development/windows-signing-guide.mdx` corrected to point at the go-live cookbook, and the runbook + STATE updated to reflect FUT-01/FUT-02/FUT-03 + DIST-SIGN-01 cleared.

## v2 / Future Requirements

Tracked, not in this roadmap.

- **FUT-04**: Native cross-target clippy run in a hosted CI matrix as the *enforcing* gate (complementary to the local toolchain).
- **FUT-05**: Next upstream-sync cadence past `nolabs-ai/nono` v0.66.0 (a future UPST cycle).
- **FUT-06**: SmartScreen reputation accrual for the new signing identity — time/volume-based, not gateable; monitor first-run "Windows protected your PC" behavior as it fades.
- **FUT-07**: Re-consolidate published package identity with upstream (co-owner grant) if/when the fork and upstream align — supersedes the `nono-sandbox` rename.

## Out of Scope

Explicit exclusions, documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Publishing under the upstream-owned `nono`/`nono-py`/`nono-ts` identities | Owned by the upstream maintainer (lukehinds); the fork publishes under fork-owned `nono-sandbox` names (PUB-01). |
| Pulling the Trusted Signing cert as a PFX from Key Vault | Microsoft-unsupported; produces an untrusted-root chain that mimics the exact `UnknownError` bug. |
| Terraform / Pulumi for the Azure VM | The repo is `az`-native; a second IaC toolchain is unjustified for one ephemeral VM. |
| A persistent Azure VM | The clean-host VM is ephemeral (create → UAT → teardown); a persistent VM re-contaminates the trust store and costs. |
| Loosening the fail-closed `-ne 'Valid'` verify to accept `UnknownError` | Security regression, not a fix — the gate stays fail-closed (SIGN-02). |
| Gating on SmartScreen reputation | Volume/time-based with no EV bypass (Artifact Signing issues no EV certs); documented as an accepted first-run cost (FUT-06). |
| Upstream sync past v0.66.0 | Out of this milestone's window (FUT-05). |
| Renaming the `nono` binary / product / GitHub repo | Only the registry `[package]` identities change; `[[bin]] name`, `[lib] name`, and `OscarMackJr/nono` stay as-is. |

## Traceability

Phase numbering continues from Phase 100 → Phase 101+.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SIGN-01 | Phase 101 | Complete |
| SIGN-02 | Phase 101 | Complete |
| SIGN-03 | Phase 101 | Complete (GREEN 2026-08-16, smoke run `31947685981`: `Status: Valid`, issuer `CN=Microsoft ID Verified CS EOC CA 04`; supersedes the FAILED 2026-07-03 verdict, whose diagnosis is retained above) |
| REL-01 | Phase 104 | In Progress (Plans 01-02/3 done: D-04 flush fix + NoCheck-mode classification landed; publish-crates neutralized (Pitfall 104-A closed) + regression guard + local root pre-check added + REL-01/SC3 wording corrected; SC1-SC3 pending operator-in-loop Plan 03) |
| PUB-01 | Phase 102 | Complete (5/5 plans done, phase gate PASSED 2026-07-03) |
| PUB-02 | Phase 105 | Pending |
| CHOST-01 | Phase 103 | Complete |
| CHOST-02 | Phase 103 | Complete |
| CHOST-03 | Phase 106 | Pending |
| CLOSE-01 | Phase 107 | Pending |

---

> **v3.6 (UPST12: Upstream Sync v0.66.0 → v0.69.0, Phases 108–114) SHIPPED 2026-08-08.**
> Its 23 requirements are archived at [`milestones/v3.6-REQUIREMENTS.md`](milestones/v3.6-REQUIREMENTS.md);
> the milestone audit is at [`milestones/v3.6-MILESTONE-AUDIT.md`](milestones/v3.6-MILESTONE-AUDIT.md).
> This file remains live because **v3.5 (Phases 101–107) is still open** — the v3.5 requirements
> above are active tracking, not history.

---

# Requirements: nono v3.7 — Composite Integrity + Tool-Sandbox Disposition

**Defined:** 2026-08-08
**Core Value:** Observation of enforcement must be as trustworthy as the enforcement. The fork's Windows guarantee is deny-by-*composition* — the intersection of integrity level, AppContainer profile, DACL, and WFP — not upstream's single deny-by-construction kernel gate. v3.7 makes that composite **prove** it is enforcing rather than adding another layer.

**Scope:** three workstreams — (1) **Composite Integrity**: a system-wide fail-direction contract + startup self-attestation, per-session enforcement receipts, an explicit security-model boundary statement, and a written state-of-the-art decision log; (2) **Tool-Sandbox Disposition**: an ADR-gated, genuinely-open verdict on upstream's never-absorbed `tool-sandbox/` subsystem, then executing that verdict; (3) **v3.6 carry-forward drain**: the six findings the v3.6 milestone audit carried forward. **Milestone-marker only** — no crate leapfrog, no publish.

> **Architecture invariants:**
> - **ADR-65 stands.** The v2.10 go/no-go verdict on a production minifilter (**No-go/Conditional-go**) is not overturned. Design-review items G-WIN-3 (minifilter Phase 64 to production) and G-WIN-4 (driver lifecycle / fleet story) are OUT of scope. The load-bearing consequence is itself a deliverable (BOUND-02): integrity levels and DACLs cannot express "read `C:\project\src` but not `C:\project\.env`" — same directory, no integrity distinction — so **per-file read policy is explicitly NOT claimed as a shipped guarantee.**
> - **A composite fails at its seams, has ambiguous fail-direction, and is harder to attest.** Those three properties are the source of every CINT/RCPT/BOUND requirement below. Upstream never has to answer them because a single kernel gate makes them unaskable.
> - **The failure mode being closed is "reports enforcing while one layer is silently inert."** This is not hypothetical — the fork already closed one instance (the "honesty gap" where `Sandbox::apply()` returned `UnsupportedPlatform` while the CLI enforced via WFP). CINT-02 exists so the next instance cannot be silent.
> - **Receipts are content-free.** An enforcement receipt records the containment a process ran under, never what the process did. No paths, no arguments, no payloads — same discipline as the existing redacted security events.
> - **Symbol-level verification, not file presence.** TSBX-02's confidence ratings must be grounded in greps for the actual types/functions/fields, not in "the target files exist." Phase 112's disposition table was wrong in 3 of ~6 re-checked dispositions, every time for exactly this reason.
> - **Executor self-check is not security evidence.** Phase 112's code-review gate caught 4 Critical fail-open defects that all 8 executor self-checks passed over; Phase 114 repeated it twice. Every code-touching phase in v3.7 runs `/gsd:code-review`.
> - **Structural fixes over spot fixes for DRAIN-02/DRAIN-03.** Both findings exist *because* a hand-maintained list drifted from its source of truth. Patching the missing arms reproduces the class.
> - **Milestone-marker only** — the tree stays at `0.70.0`; no crate leapfrog, no registry publish. Tag `v3.7` local. This keeps v3.5's paused go-live as the single release decision point.
> - **Two milestones are open.** v3.5 owns Phases 101–107 and remains live; v3.7 owns 115+. `phases.clear` must not run; this file and `ROADMAP.md` are **appended to, never overwritten**; SDK STATE writers stay banned. All commits DCO-signed; repo stays PUBLIC (no `build_notes/`/`.gsd/` staged).
> - **Cross-target clippy remains MUST** for any cfg-gated Unix edit — both local gates (`cross` linux-gnu, `cargo-zigbuild` apple-darwin) GREEN, no PARTIAL→CI.

## v1 Requirements

### Fail-Direction & Self-Attestation (CINT)

- [x] **CINT-01**: A single fail-direction contract states, for **every** layer the Windows backend composes — restricted token, mandatory integrity label, AppContainer profile + package SID, DACL grants, WFP egress filters, and the minifilter's *absence* — what happens when that layer cannot be established (fail closed / fail open / continue in reduced mode). Each entry is derived from the code and cites the enforcing call site, so the document is a description of behaviour rather than an assertion about it.
- [x] **CINT-02**: nono refuses to report "enforcing" when any expected layer is unconfirmed. A startup self-attestation pass checks each layer named in CINT-01 and either aborts or visibly downgrades its claim — it never proceeds while presenting a confinement guarantee it cannot substantiate.
- [x] **CINT-03**: Each layer's actual runtime fail-direction matches what CINT-01 claims, proven by a per-layer test that forces that layer unavailable and asserts the contracted outcome. A contract entry with no such test is not satisfied.

### Enforcement Receipts (RCPT)

- [x] **RCPT-01**: Every confined session emits a per-session enforcement receipt naming which layers were confirmed active for that process. The receipt is content-free — no paths, no arguments, no payloads — recording the containment the process ran under, not what it did.
- [x] **RCPT-02**: The receipt is tamper-evident on the same terms as the core audit chain — a **keyless SHA-256 hash chain** (D-25) — so a downstream governance consumer can verify a receipt was not edited or reordered after the fact, including after the writing process has exited. *(Text amended 2026-09-06; original read "(HMAC-chained `SecurityEventLayer`)" — see status table for why.)*
- [x] **RCPT-03**: A receipt distinguishes "layer confirmed active" from "layer not expected in this configuration" from "layer expected but unconfirmed". An unattested layer can never be read as an attested one, and a reader can tell the three cases apart without out-of-band knowledge.

### Security-Model Boundary & Decision Log (BOUND)

- [ ] **BOUND-01**: The security model documents what nono governs — destination, credential, containment — and what it does not: payload contents. It states explicitly that because the fork tunnels TLS transparently and filters at the host level, a prompt exfiltrating a secret to an **allowlisted** host is invisible to nono, so no downstream consumer can over-claim content control.
- [ ] **BOUND-02**: The filesystem guarantee is stated in terms of the shipped mechanism. Per-file read policy within a single directory (`src/` readable, `.env` not) is explicitly named as **not enforced**, citing ADR-65's standing No-go/Conditional-go verdict as the reason — so the 1.0 guarantee describes what ships, not what the design intends.
- [ ] **BOUND-03**: Each of the six state-of-the-art techniques is ruled in or out **in writing**, with reasoning: Windows Sandbox / Server Containers (HCS) as a strong-isolation tier; PPL / Restricted User Mode for protecting nono's **own supervisor** from the process it contains; WFP ALE layers beyond connect-time; ETW as an enforcement-adjacent escape-attempt signal; AppContainer capability profiles as declarative positive grants; and WDAC for exec gating. The output is a set of decisions, not a set of omissions.

### Tool-Sandbox Disposition (TSBX)

- [ ] **TSBX-01**: A per-commit divergence ledger covers upstream's `tool-sandbox/` subsystem (PR #1105, introduced v0.65.0, never absorbed) and the 7 refinement PRs Phase 108 fenced (#1280 / #1322 / #1325 / #1384 / #1394 / #1413 / #1417), with `windows-touch` flags and per-cluster dispositions — the Phase 108 / 98 / 94 / 85 ledger shape.
- [ ] **TSBX-02**: An ADR records the adopt-vs-formalize verdict, evaluating the fork's own independently-built answer (PreToolUse hook → `nono run` + Low-IL primary-token broker, PR #4) as a genuine alternative rather than a fallback. Every confidence rating is grounded in symbol-level verification of what exists in the fork — the types, functions and fields actually greppable — not file presence.
- [ ] **TSBX-03**: The verdict is executed. Either the subsystem is absorbed with a `platform/windows.rs` driver, or fork-native is formalized as a permanent, named scope boundary that a future absorb has something to reconcile against (ADR-111 / ADR-113 shape). Either way the divergence is closed by recorded decision and stops being a silent standing gap.

### v3.6 Carry-Forward Drain (DRAIN)

- [x] **DRAIN-01** *(NEW-05)*: A `platform_overrides.<os>` block that redefines a custom credential and omits `inject_mode` / `inject_header` inherits the base values instead of silently resetting the route to header-mode `Authorization`. `merge_custom_credential_def` (`crates/nono-cli/src/profile/mod.rs:3564-3567`) merges them with `.or(base)`, pinned by a test; and the NEW-02 regression test's missing `spiffe` assertion (ACC-04, `mod.rs:10188`/`:10238-10252`) is added.
- [x] **DRAIN-02** *(NEW-06, blocker-class)*: `nono-py` round-trips every denial category its own encoder emits — `capture_unsupported_path` and `capture_buffer_or_rewrite_failed` no longer raise `ValueError` from `../nono-py/src/undo.rs`. Pinned by an exhaustiveness test over the encoder's own output, not by adding two arms to a hand-maintained match that will drift again.
- [x] **DRAIN-03** *(NEW-01)*: The three denial sites that pass `&audit::EventContext::default()` — `connect.rs:86` (which is `deny_domain`'s HTTPS enforcement point), `external.rs:136`, `external.rs:200` — emit a real denial category; and the two variants with zero production constructors (`InterceptHandshakeFailed`, `ExternalProxyRejected`, `crates/nono/src/undo/types.rs:249`/`:252`) are either wired to a real site or removed.
- [x] **DRAIN-04** *(NEW-03)*: A route declaring both `aws_auth` and `capture` either works or is rejected at config-validation time. It no longer validates successfully and then returns 501 at runtime (`reverse.rs:328-331` returning before the capture branch at `:427`).
- [x] **DRAIN-05** *(NEW-07)*: A Python embedder can configure `capture` and `spiffe` on a `RouteConfig` — `../nono-py/src/proxy.rs`'s constructor exposes both instead of hardcoding `None`, so a Python-configured OAuth token-endpoint route no longer silently gets pre-113 / pre-114 behaviour.
- [x] **DRAIN-06** *(NEW-08)*: A SPIFFE route blocked by `deny_domain` is denied **before** a JWT-SVID is minted — `managed_auth.acquire()` (`reverse.rs:634`) no longer runs ahead of the filter host check (`:684`), matching the ordering every other dispatch path uses.

## v2 / Future Requirements

Tracked, not in this milestone.

- **FUT-08**: **UPST13** — the next upstream-sync cadence past `nolabs-ai/nono` v0.69.0. Offered as a fourth v3.7 workstream and deliberately declined to keep an already-three-workstream milestone bounded. Leading candidate for the milestone after v3.7.
- **FUT-09**: `platform/windows.rs` tool-sandbox driver build-out — **conditional**: only exists as future work if TSBX-02 returns *adopt* and TSBX-03 cannot fully land inside v3.7. Sized from the verdict, never pre-committed.
- **FUT-10**: Payload inspection / TLS interception. A large, CA-distribution-shaped project. BOUND-01 states the boundary rather than building it; if payload inspection is ever needed it is scoped as its own milestone, never retrofitted.

## Out of Scope

Explicit exclusions, documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Minifilter Phase 64 to production (G-WIN-3) | ADR-65 stands: No-go/Conditional-go. WFP + AppContainer/Low-IL already gives kernel-enforced isolation; the driver is incremental gain at high cert/maintenance cost. Its consequence is *documented* by BOUND-02, not built around. |
| Driver lifecycle / fleet rollout story (G-WIN-4) | Follows the minifilter. With no production driver in scope, a signing/altitude/staged-rollout/kernel-crash-rollback document has no subject. |
| Per-file read policy inside a single directory | Structurally unavailable without the minifilter — integrity levels and DACLs cannot distinguish `src/` from `.env` in the same directory. Explicitly not claimed (BOUND-02). |
| TLS interception / payload inspection | The fork dropped it deliberately (no CA to distribute, no interception liability). BOUND-01 states the resulting boundary; building it is FUT-10. |
| Crate version leapfrog and registry publish | Milestone-marker only. Stacking a second version bump behind v3.5's paused go-live would create two release decision points instead of one. |
| Upstream sync past v0.69.0 | Out of this milestone's window (FUT-08). Offered and declined at milestone open. |
| Overwriting `REQUIREMENTS.md` / `ROADMAP.md` wholesale | v3.5 is still open and its tracking lives in the same files; the stock archive/reset flow destroys it (caught and worked around at the v3.6 close, `1fd559bf`). |

## Traceability

Phase numbering continues from Phase 114 → Phase 115+ (v3.5 owns 101–107; no `--reset-phase-numbers`).

| Requirement | Phase | Status |
|-------------|-------|--------|
| DRAIN-01 | Phase 115 | Complete |
| DRAIN-02 | Phase 115 | Complete |
| DRAIN-03 | Phase 115 | Complete |
| DRAIN-04 | Phase 115 | Complete |
| DRAIN-05 | Phase 115 | Complete |
| DRAIN-06 | Phase 115 | Complete |
| TSBX-01 | Phase 116 | Pending |
| TSBX-02 | Phase 116 | Pending |
| CINT-01 | Phase 117 | Complete |
| CINT-02 | Phase 117 | Complete (two operator deferrals recorded: CR-02 daemon wiring, RF-13 fleet-control RequiredLayers) |
| CINT-03 | Phase 117 | Complete via operator override 2026-08-15 — 10/13 rows automated; 3 host/structurally-gated rows (DaclSessionSidGrant, MinifilterAbsence, BrokerAuthenticodeTrustGate) accepted, not met |
| RCPT-01 | Phase 118 | Complete 2026-09-06 — verifier enumerated all 16 post-spawn terminal exits across the 3 producers; all emit or are justified. NOTE: this requirement was violated in production for 3 weeks (broker receipts misdirected, CR-06) and 4 separate unenumerated-exit defects were found and fixed during close-out |
| RCPT-02 | Phase 118 | Complete 2026-09-06. **Requirement text AMENDED the same day, by operator decision, to name the mechanism that actually shipped.** It previously read "HMAC-chained `SecurityEventLayer`", which was wrong twice over: D-25 chose a KEYLESS SHA-256 chain because `SecurityEventLayer`'s key is per-process OsRng zeroized on `Drop` (an HMAC-chained receipt would be unverifiable once the process exited), and `crates/nono/src/audit.rs` — the actual pre-existing audit chain — is itself keyless. Intent unchanged: a downstream consumer can detect an edit. Claim boundary: tamper-EVIDENCE and ordering, **never authorship** (D-25 forbids "only the key holder could have produced this"). ⚠ Accepted limitation, NOT closed: a clean tail-record deletion is undetectable by a keyless chain with no external anchor, and `nono receipt verify` does not disclose that to an operator — the limit currently lives only in planning docs |
| RCPT-03 | Phase 118 | Complete 2026-09-06 — three-state rendering is compile-time exhaustive. WR-02 fixed during close-out (06393e57): an overlap between the two broker wire channels resolved permissively, letting a required layer read as "not expected in this configuration" — the exact misreading this requirement forbids |
| BOUND-01 | Phase 119 | Pending |
| BOUND-02 | Phase 119 | Pending |
| BOUND-03 | Phase 119 | Pending |
| TSBX-03 | Phase 120 | Pending |
