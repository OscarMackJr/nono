# Project Research Summary

**Project:** nono v3.5 — Trusted Signing Go-Live + First Distributed Release
**Domain:** Windows Authenticode/Azure Trusted Signing CI hardening + ephemeral Azure Win11 VM clean-host UAT + live multi-registry package publish (crates.io / PyPI / npm)
**Researched:** 2026-07-02
**Confidence:** MEDIUM-HIGH (repo-internal evidence and live registry queries are HIGH; Azure Bicep VM specifics and exact CI-runner root-store state are MEDIUM/LOW pending phase-time verification)

## Executive Summary

This is not a greenfield build — it is a go-live milestone converting three already-built, already-dry-run-proven pipelines (Trusted Signing verify-gate, Azure clean-host UAT, multi-registry publish) from "prepared but never actually exercised live" into "actually executed, with real consequences." The single blocking defect discovered on the last live attempt (2026-06-30, run `28467925298`) was `Get-AuthenticodeSignature` returning `UnknownError` after a successful Trusted Signing sign step. Research resolves this into a disambiguable three-cause taxonomy — (1) wrong certificate profile type (observed issuer `CN=Microsoft Enterprise ID Verified Policy AOC CA 01` does not match the Public Trust naming pattern `Microsoft ID Verified CS EOC/AOC CA NN`, strongly suggesting the profile is Private/Enterprise/Test, not `PublicTrust`), (2) CI-runner root-store staleness on the March-2026 AOC/EOC intermediate rotation, or (3) CRL/OCSP revocation-check timeout — each with its own cheap, independent disambiguation probe (`az trustedsigning certificate-profile show`, `signtool verify /pa /v` on an independently-updated host, and a revocation-disabled diagnostic run, respectively). Fixing the wrong cause and re-running smoke will reproduce the identical opaque symptom, burning cycles; the verify-gate hardening phase must run all three probes, not stop at the first plausible one.

The second major finding — and the cheapest, earliest-value check in this entire milestone — is that crates.io (`nono`, `nono-proxy`), PyPI (`nono-py`), and npm (`nono-ts`) are all currently owned solely by the upstream maintainer (`lukehinds`/`lhinds`), verified live against each registry's public API. Every prior "PASS" on `cargo publish --dry-run` / `maturin build` / `npm publish --dry-run` never exercised registry authentication, so this failure mode is completely invisible to every gate run so far. A live `cargo publish`/`twine upload`/`npm publish` will 403 unless a co-owner grant exists — and this can be checked (via `cargo owner --list`, and out-of-band collaborator requests for PyPI/npm) before any signing work begins, at essentially zero cost. This should be the first go/no-go gate of the milestone, ahead of the Trusted Signing fix, because a NO-GO here reshapes FUT-01's entire scope (co-owner grant vs. renaming to fork-owned package identities) rather than being a late-discovered blocker.

The recommended approach: (1) stand up a shared `verify-authenticode.ps1` helper consolidating the three currently-duplicated `Get-AuthenticodeSignature -ne 'Valid'` fail-closed sites plus a `signtool verify /pa /v` diagnostic fallback — never loosen the fail-closed condition itself; (2) resolve registry ownership and the Azure profile-type misconfiguration as parallel, cheap preflights; (3) prove the fix on the disposable smoke workflow before ever cutting a real tag; (4) cut the real release and publish live, per-registry, using each registry's own correct idempotency/resume semantics (query-then-resume for crates.io, `--skip-existing` for PyPI, "already exists on retry = prior success" for npm — explicitly NOT a single uniform retry loop); (5) stand up an ephemeral, IaC-provisioned Azure Win11 VM (Gen2, Trusted Launch, genuinely never touched by the POC cert or corporate trust store) to close the two long-standing host-gated todos (`poc-cert-broker-clean-host`, `msi-vcredist-prereq`) via the existing `verify-dark.ps1` gate-discovery harness — no new harness code, just two new gate files; and (6) retire the POC signing secrets only after Gate 3 (clean-host UAT) passes, never merely after Gate 2 (release workflow green), so a fallback signing path survives until the new path is independently proven end-to-end on real hardware.

## Key Findings

### Recommended Stack

No new core language/runtime — this is additions-only to an already-Rust/PowerShell/Azure-CLI stack. The net-new tooling is entirely diagnostic and infrastructure: `signtool verify /pa /v /debug` as a fallback/diagnostic layer alongside the existing `Get-AuthenticodeSignature` gate (never a replacement), the `az trustedsigning` CLI extension (>= Azure CLI 2.57.0, Preview) for scriptable profile-type confirmation, and Bicep (via `az bicep install`, zero separate binary/state-file) for the ephemeral clean-host VM — chosen over Terraform specifically because it composes with the repo's existing `az`-native tooling and introduces no new state-management model for a single throwaway resource. The publish-leg tooling (`maturin`, `twine`, `@napi-rs/cli`, `cargo publish`) is already correctly pinned and dry-run-proven; v3.5 changes only the live-vs-dry-run posture, not the mechanism.

**Core technologies:**
- `signtool verify /pa /v /debug` — deep Authenticode chain-build diagnostic; use `/pa` (default WinVerifyTrust policy), not `/kp`, or you diagnose the wrong policy
- `az trustedsigning certificate-profile show` — authoritative, scriptable confirmation of `profileType == PublicTrust` (accepted values are exactly `{PrivateTrust, PrivateTrustCIPolicy, PublicTrust, PublicTrustTest, VBSEnclave}` — no plain "Test" value exists)
- Bicep + `MicrosoftWindowsDesktop:windows-11` Gen2, `--security-type TrustedLaunch --enable-secure-boot true --enable-vtpm true` — genuine client-OS trust-store/AppContainer semantics; Trusted Launch's vTPM is mandatory (not optional) for a Win11 marketplace image to boot
- `cargo owner --list`, PyPI/npm collaborator checks — the registry-identity preflight, net-new script-level check, zero build cost

### Expected Features

Full detail in FEATURES.md. This milestone's "features" are proof obligations, not new capabilities — code for all of them already ships; what's missing is live, host-verified evidence.

**Must have (table stakes):**
- Authenticode `Valid` on all signed binaries/MSIs chaining to a genuine Microsoft Public Trust root (not Test/Private/Enterprise)
- Broker self-trust gate (`verify_broker_authenticode`) passing with zero manual cert import on a clean host
- MSI installs cleanly on a fresh Win11 host with no VC++ redist preinstalled (`+crt-static` already ships)
- Live publish to crates.io in strict dependency order (`nono` -> `nono-proxy` -> `nono-cli`), PyPI, and npm actually succeeding (not just dry-run)
- No POC secrets/paths reachable after go-live

**Should have (differentiators):**
- `UnknownError`-vs-`UntrustedRoot` diagnostic distinction baked into the CI verify step itself
- Scripted, repeatable Azure-VM clean-host IaC (turns a one-time UAT into a re-runnable gate)
- Pre-flight "what's already published" registry-state check before any live push, enabling safe per-registry resume

**Defer (v2+ / beyond this milestone):**
- SmartScreen reputation monitoring/tracking over time — no lever to accelerate it, document and move on
- Re-runnable Azure-VM IaC as a *standing* per-release gate (this milestone proves it once; making it permanent is a v3.6+ candidate)

### Architecture Approach

Additions-only to existing CI/CD and gate-harness infrastructure: harden the three existing fail-closed verify sites in `release.yml`/`trusted-signing-smoke.yml` behind one new shared helper (`scripts/verify-authenticode.ps1`), stand up net-new `scripts/azure/clean-vm/` Bicep IaC (genuinely new territory — no prior `scripts/azure/` directory exists), and add exactly two new gate files under `scripts/gates/` that plug into the existing `verify-dark.ps1` auto-discovery with zero harness code changes.

**Major components:**
1. `scripts/verify-authenticode.ps1` — shared hardened-verify function (`Get-AuthenticodeSignature` + `signtool /pa /v` fallback + `UnknownError`-vs-`UntrustedRoot` classification), dot-sourced by `release.yml` (two call sites), `trusted-signing-smoke.yml`, and the new `trusted-signed-assertion.ps1` gate — prevents "CI thinks it's signed" and "operator's dark gate thinks it's signed" from ever silently diverging
2. `scripts/azure/clean-vm/` (main.bicep + deploy/teardown scripts) — ephemeral, tagged, torn-down-after-use Win11 VM; decoupled region from the Trusted Signing account's own region
3. `scripts/gates/trusted-signed-assertion.ps1` and `scripts/gates/broker-spawn-on-clean-host.ps1` — two new, self-contained gates (do not rely on `-All`'s alphabetical execution order, which would run `broker-spawn-on-clean-host` before `clean-host-install` — a confirmed ordering gotcha) following the existing `Test-Precondition`/`Invoke-Gate` two-function contract, reusing `clean-host-install.ps1` unmodified

### Critical Pitfalls

Full detail (11 pitfalls) in PITFALLS.md. Top 5, prioritized by leverage and blocking severity:

1. **Registry ownership not verified (crates.io/PyPI/npm all upstream-owned)** — live-queried and confirmed: `nono`/`nono-proxy` owned by `lukehinds`, `nono-py` by `lhinds`, `nono-ts` by `lukehinds`/`lhinds@protonmail.com`. A live publish will 403 without a co-owner grant. Run this check FIRST — before any signing work — via `cargo owner --list -p nono/-proxy/-cli` plus out-of-band PyPI/npm collaborator requests to the upstream maintainer. NO-GO here reshapes FUT-01 scope (co-owner grant vs. fork-owned package rename), so surface it as early and cheaply as possible.
2. **`UnknownError` conflated as one problem when it is three** — profile-type misconfiguration, CI-runner root-store staleness on the new AOC/EOC intermediate, and CRL/OCSP revocation timeout are independently plausible and non-mutually-exclusive. Disambiguate with three cheap probes before declaring any fix complete: `az trustedsigning certificate-profile show` (must read `PublicTrust`, not `Test`/`PrivateTrust`), `signtool verify /pa /v` on an independently-updated offline host (using the smoke workflow's `if: always()` artifact upload), and a revocation-disabled diagnostic run to isolate cause #3.
3. **Fail-closed gate weakened as a workaround under deadline pressure** — never change `-ne 'Valid'` to also accept `UnknownError`, wrap in `-ErrorAction SilentlyContinue`, or otherwise loosen the check. Fix the root cause (Pitfall 2), never the assertion. Any diff touching the verify condition without also touching root-cause remediation deserves extra review scrutiny.
4. **crates.io publish order is a strict, irreversible, one-way chain** — `nono` -> `nono-proxy` -> `nono-cli`; crates.io has no unpublish, only yank. A mid-sequence failure leaves a permanent half-published state. Prefer polling for index visibility over a blind `sleep 30`, and document the fix-forward (bump only the broken crate) recovery path explicitly before the operator's first live attempt.
5. **Premature secret retirement** — retiring `WINDOWS_SIGNING_CERT`/`_PASSWORD` and deleting the smoke workflow must happen strictly after Gate 3 (Azure VM clean-host UAT) passes, never merely after Gate 2 (release workflow green). If Gate 3 later reveals a problem invisible on `windows-latest`, there must still be a fallback signing path.

## Implications for Roadmap

Based on combined research, the critical path has a hard sequential dependency chain with 2 parallelizable authoring phases feeding into it. Registry-identity preflight is inserted as the cheapest, earliest gate — before the Trusted Signing fix — because a NO-GO reshapes downstream scope rather than being a late-discovered blocker.

### Phase 1: Registry-Identity + Verify-Gate Readiness Preflight
**Rationale:** Cheapest possible checks, zero build/signing cost, and both are pure go/no-go gates that reshape later scope if they fail. Registry ownership (Pitfall 1) can be checked before touching Azure at all. Confirming the Azure certificate profile's actual `profileType` is the highest-value diagnostic for the `UnknownError` symptom and should happen before any CI code changes are trusted to have "fixed" anything.
**Delivers:** `cargo owner --list` results for all 3 crates; PyPI/npm collaborator-grant status (or explicit rename decision); `az trustedsigning certificate-profile show` output confirming/denying `PublicTrust`
**Addresses:** FEATURES.md's P1 "Confirm/fix Azure Public Trust profile config" and the registry-ownership go/no-go
**Avoids:** Pitfall 1 (registry ownership), Pitfall 2 root-cause #1 (wrong profile type)

### Phase 2: CI Verify-Gate Hardening
**Rationale:** Cannot prove smoke-green without the hardened verify existing; must land before any tag push is meaningful (pushing against an unfixed `UnknownError` gate just reproduces the 2026-06-30 failure).
**Delivers:** `scripts/verify-authenticode.ps1` shared helper (Get-AuthenticodeSignature + signtool /pa /v fallback + UnknownError-vs-UntrustedRoot classification); modified `release.yml` (~line 259, ~281-321) and `trusted-signing-smoke.yml` (~line 62-73), fail-closed semantics preserved
**Uses:** `signtool verify /pa /v /debug`, `Get-AuthenticodeSignature`
**Implements:** Pattern 1 (Fail-closed verify with graduated fallback, not "or")
**Avoids:** Pitfall 3 (weakening the fail-closed gate), Pitfall 9 (AADSTS700213 recurrence — re-run smoke as canary after any workflow edit)

### Phase 3: Azure Clean-Host VM IaC + New Verify-Dark Gates (parallelizable with Phase 2)
**Rationale:** Independent code/infra work from Phase 2; gates can be authored against `SKIP_HOST_UNAVAILABLE` (same discipline as existing `clean-host-install.ps1`) even before the VM exists. Must exist before Phase 6 (clean-host UAT) can run.
**Delivers:** `scripts/azure/clean-vm/` (main.bicep, deploy/teardown scripts); `scripts/gates/trusted-signed-assertion.ps1` and `scripts/gates/broker-spawn-on-clean-host.ps1` (self-contained, not order-dependent on `-All`'s alphabetical sweep)
**Addresses:** FEATURES.md's "Scripted, repeatable Azure-VM clean-host IaC" differentiator
**Avoids:** Pitfall 5 (corporate-host confounds), Pitfall 11 (VM licensing/networking/egress gotchas — unfiltered AIA/CRL/OCSP/timestamp-authority egress, NSG scoped to operator IP, prompt teardown)

### Phase 4: Smoke Green + Cut the Trusted-Signed Release (Gates 1 and 2)
**Rationale:** Hard sequential dependency — smoke MUST be green before a real tag is pushed (per the cookbook's own "do not cut a release until smoke is green" directive). This is the actual go-live moment.
**Delivers:** Confirmed `Status: Valid` + issuer chaining to `Microsoft ID Verified CS` on the smoke workflow; then a real `v0.66.1` tag push producing a trusted-signed GitHub Release with MSI/exe/checksums
**Addresses:** FEATURES.md's P1 "Cut the trusted-signed 0.66.1 release"
**Avoids:** Pitfall 2 (must have disambiguated root cause first), Pitfall 9 (FIC subject regression)

### Phase 5: Live Multi-Registry Publish (FUT-01)
**Rationale:** Structurally requires Phase 4's real signed artifacts to exist first. Registry-specific resume semantics differ enough that a uniform retry loop is actively wrong (Pitfall 7, Pitfall 8) — this phase must encode per-registry logic, not copy one pattern three times.
**Delivers:** Live packages on crates.io (dependency-ordered, polling-based index-visibility wait instead of blind `sleep 30`), PyPI (`twine upload --skip-existing`, two-step form for safe resume), npm (`npm publish`, treating "already exists" on retry as confirmation of prior success, not failure); post-publish platform-coverage verification for PyPI/npm
**Addresses:** FEATURES.md's P1 "FUT-01 live publish to all 3 registries... with a pre-flight registry-state check"
**Avoids:** Pitfall 6 (fragile `cargo search` idempotency — match `cargo publish`'s actual "already exists" error text as a backstop), Pitfall 7 (irreversible mid-chain crates.io failure), Pitfall 8 (partial platform coverage — known upstream `nono-ts` Linux-package-missing bug must not be repeated)

### Phase 6: Azure VM Clean-Host UAT (Gate 3)
**Rationale:** Structurally cannot run before Phase 4/5 — needs the actual trusted-signed, published MSI artifact, not a smoke-test throwaway exe. This is the literal acceptance test both host-gated todos (`poc-cert-broker-clean-host`, `msi-vcredist-prereq`) have been waiting on.
**Delivers:** `.nono-runtime/verdicts/*.json` PASS evidence for `trusted-signed-assertion`, `broker-spawn-on-clean-host`, and reused `clean-host-install`, drained from a genuinely fresh Win11 VM never touched by the POC cert or corporate trust store
**Addresses:** FEATURES.md's FUT-03 clean-host UAT
**Avoids:** Pitfall 4 (SmartScreen cold-start misdiagnosed as a signing bug — document explicitly as expected/non-blocking in the gate output), Pitfall 5 (corporate-host contamination)

### Phase 7: Close-Out
**Rationale:** Must be strictly gated on Phase 6 PASS, not Phase 4 green — this is the single highest-severity sequencing risk in the whole milestone (Pitfall 10) because once POC secrets are deleted, there is no fallback signing path if Gate 3 reveals a problem invisible on CI runners.
**Delivers:** Retired `WINDOWS_SIGNING_CERT`/`_PASSWORD` GitHub secrets, deleted `trusted-signing-smoke.yml`, fixed `windows-signing-guide.mdx` (gitignored-but-tracked, needs `git add -f`), both todos moved `pending/` -> `resolved/`
**Addresses:** FEATURES.md's close-out target feature
**Avoids:** Pitfall 10 (premature secret retirement) — structurally require a link to a passed Gate 3 verdict before secret-retirement commands are permitted, not just a written reminder

### Phase Ordering Rationale

- Registry-identity preflight is placed FIRST, ahead of any Azure/signing work, because it is the cheapest possible check (no build cost) and a NO-GO result reshapes the entire FUT-01 scope — discovering this late (e.g., during Phase 5) would waste all prior signing-fix effort on a release that then can't actually publish.
- Phases 2 and 3 are explicitly parallelizable (independent code/infra), but both feed the same downstream critical path (2 -> 4; 3 -> 6) — a roadmap should not force them sequential.
- The hard sequential spine is 1 -> 2 -> 4 -> 5/6 -> 7, matching ARCHITECTURE.md's confirmed dependency chain; 3 feeds into 6 but does not block 4/5.
- Close-out is deliberately its own final phase, not folded into the release-cut phase, specifically to create a structural checkpoint against Pitfall 10.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 3 (Azure clean-host VM IaC):** ARCHITECTURE.md explicitly flags that no WebFetch/WebSearch was performed for Bicep syntax on `Microsoft.Compute/virtualMachines` + `Microsoft.Network/networkSecurityGroups` — do a fresh official-docs pass before authoring `main.bicep`. Also confirm current-region Windows 11 client SKU availability via `az vm image list-skus` at plan-authoring time, not from this research (Marketplace SKU availability is time/region-sensitive and has already shifted mid-2026).
- **Phase 2 (verify-gate hardening):** Confirm `signtool.exe` is actually present/resolvable on the current `windows-latest` runner image before relying on it (LOW confidence gap — typically preinstalled but not explicitly re-verified this pass); add an explicit path-resolve/fallback step.
- **Phase 1 (registry preflight):** No reliable read-only "am I a maintainer" API exists for PyPI — this leg may require direct outreach to the upstream maintainer and cannot be fully scripted; budget non-engineering lead time.

Phases with standard patterns (skip research-phase):
- **Phase 4 (smoke green + cut release):** Mechanism unchanged from prior milestones' dry-run-proven pipeline; only the live-vs-prepare-only posture changes.
- **Phase 5 (multi-registry publish):** Per-registry mechanics (crates.io dependency order, PyPI `--skip-existing`, npm atomic-publish) are HIGH confidence, verified against official docs/issue trackers; implementation is mechanical once Phase 1 clears ownership.
- **Phase 7 (close-out):** Well-scoped, low-complexity hygiene tasks with an explicit existing checklist (cookbook Section 8).

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM-HIGH | Azure Trusted Signing CLI surface and cert-chain naming verified against current Microsoft Learn docs; Azure region/Marketplace-SKU specifics are inherently drift-prone and explicitly flagged for re-verification at phase-authoring time |
| Features | HIGH for mechanics (Azure Trusted Signing, cargo/PyPI/npm publish semantics — verified against official docs); MEDIUM for the exact current chain-build failure cause on this specific `windows-latest` run (inferred from a documented AOC/EOC CA rotation pattern, not independently reproduced) | HIGH-confidence primary source: repo's own gate code and the 2026-06-30 field note |
| Architecture | HIGH for all integration points (cited directly against `release.yml`, `trusted-signing-smoke.yml`, `verify-dark.ps1`, `scripts/gates/*.ps1` as read in this repo); MEDIUM for Azure Bicep/VM specifics, genuinely new territory not yet grounded in a live Azure resource | No WebFetch/WebSearch performed for Bicep specifics in this research pass — explicitly flagged |
| Pitfalls | HIGH for pitfalls verified against this repo's actual code/config AND live registry queries (Pitfall 1's ownership finding is directly, freshly verified — not inferred); MEDIUM/LOW flagged inline for pitfalls based on general Trusted Signing/PKI domain knowledge not independently reproduced in this environment | Pitfall 1 (registry ownership) is the highest-confidence, highest-leverage finding in the entire research set |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **PyPI ownership resolution has no scriptable check:** unlike crates.io (`cargo owner --list`) and npm (`npm access ls-collaborators`), there is no reliable read-only "am I a maintainer" PyPI API call — Phase 1 will need direct, out-of-band communication with the upstream maintainer, which is a scheduling/lead-time risk not an engineering one. Handle by starting this outreach as early as possible, potentially before other Phase 1 work completes.
- **Bicep resource schema not verified against live docs:** Phase 3 planning must do a dedicated Context7/official-docs pass on `Microsoft.Compute/virtualMachines` (Gen2/Trusted Launch parameters) and `Microsoft.Network/networkSecurityGroups` before authoring `main.bicep` — this research explicitly did not perform that lookup to stay grounded in repo-internal evidence.
- **Exact root cause of the observed `Enterprise ID Verified Policy AOC CA 01` issuer string is unconfirmed:** no public documentation was found matching this exact issuer name pattern against either the Public Trust or Private Trust naming conventions — treat this as the actionable signal itself (re-verify the actual configured profile type in Azure directly, don't trust the cookbook's own prior guess) rather than a resolved fact.
- **Trusted Signing account-level rate/quota limits not researched:** flagged only as a scaling consideration (not a go-live blocker at this milestone's one-time cadence) — revisit if release frequency increases in a future milestone.
- **Azure VM subscription licensing eligibility (Dev/Test vs. Multitenant Hosting Rights) not confirmed for the actual target subscription:** verify at Phase 3 execution time, not assumed from this research.

## Sources

### Primary (HIGH confidence)
- Repo-internal, read directly: `.github/workflows/release.yml`, `.github/workflows/trusted-signing-smoke.yml`, `scripts/verify-dark.ps1`, `scripts/gates/clean-host-install.ps1`, `scripts/gates/release-readiness.ps1`, `crates/nono-cli/src/exec_strategy_windows/launch.rs:2236-2296`, `.planning/quick/260630-trusted-signing-golive/AZURE-TRUSTED-SIGNING-GOLIVE-COOKBOOK.md`, `.planning/todos/pending/20260611-poc-cert-broker-clean-host.md`, `.planning/todos/pending/20260611-msi-vcredist-prereq.md`, `.planning/milestones/v3.3-phases/97-.../RELEASE-RUNBOOK.md`, `.planning/PROJECT.md`
- Live registry API queries: `crates.io/api/v1/crates/nono` + `/owners`, `crates.io/api/v1/crates/nono-proxy/owners`, `pypi.org/pypi/nono-py/json`, `registry.npmjs.org/nono-ts`
- [az trustedsigning certificate-profile — Microsoft Learn CLI reference](https://learn.microsoft.com/en-us/cli/azure/trustedsigning/certificate-profile?view=azure-cli-latest)
- [Artifact Signing trust models — Microsoft Learn](https://learn.microsoft.com/en-us/azure/artifact-signing/concept-trust-models)
- [Azure Artifact Signing FAQ](https://learn.microsoft.com/en-us/azure/artifact-signing/faq)
- [Trusted Launch for Azure VMs — Microsoft Learn](https://learn.microsoft.com/en-us/azure/virtual-machines/trusted-launch)
- [How to deploy Windows 11 on Azure — Microsoft Learn](https://learn.microsoft.com/en-us/azure/virtual-machines/windows/windows-desktop-multitenant-hosting-deployment)
- [`cargo publish` / Publishing on crates.io — The Cargo Book](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [twine docs — `--skip-existing`](https://twine.readthedocs.io/)

### Secondary (MEDIUM confidence)
- [Root certificate trust problem after creating a Trusted Signing profile — Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/2140998/root-certificate-trust-problem-after-creating-an-a)
- [Trusted Signing new intermediate CAs causing SmartScreen warnings — Microsoft Q&A #5855442](https://learn.microsoft.com/en-ca/answers/questions/5855442/azure-trusted-signing-new-intermediate-cas-causing)
- [Azure Trusted Signing fails on Public Trust test certificates — dotnet/sign#908](https://github.com/dotnet/sign/issues/908)
- [`rust-lang/cargo#13397` — no native idempotent publish, confirmed open](https://github.com/rust-lang/cargo/issues/13397)
- [`pypa/twine#199` — HTTP 400 file-already-exists after retry-due-to-500](https://github.com/pypa/twine/issues/199)
- GitHub issue "Missing system-specific package on npm" (`always-further/nono-ts`) — corroborates the known upstream napi-rs multi-platform publish gap

### Tertiary (LOW confidence)
- `certutil -verify -URL` exact flag behavior — drawn from established tool knowledge, not re-verified against `certutil -?` output this pass
- Azure Bicep resource schema specifics for `Microsoft.Compute/virtualMachines` + NSG — not looked up this pass, flagged for Phase 3 dedicated research

---
*Research completed: 2026-07-02*
*Ready for roadmap: yes*
