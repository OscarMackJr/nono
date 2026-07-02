# Pitfalls Research

**Domain:** Code-signing pipeline go-live (Azure Trusted Signing) + first live multi-registry release (crates.io / PyPI / npm) for a Rust/Windows sandboxing tool
**Researched:** 2026-07-02
**Confidence:** HIGH for pitfalls verified against this repo's actual code/config and live registry queries; MEDIUM/LOW flagged inline where based on general Trusted Signing/PKI knowledge not directly reproduced in this environment

## Critical Pitfalls

### Pitfall 1: The fork does not own the crates.io / PyPI / npm package names it is about to publish to — VERIFIED, likely-blocking

**What goes wrong:**
The milestone's FUT-01 goal is "publish `0.66.1` live to crates.io + PyPI + npm." Live registry queries performed during this research show all three target package identities are **already registered and solely owned by the upstream maintainer**, not by this fork's operator:

| Registry | Package | Owner (verified via public API) | Upstream's current version |
|----------|---------|----------------------------------|------------------------------|
| crates.io | `nono` | `lukehinds` (sole owner, `/api/v1/crates/nono/owners`) | `0.66.0`, repo `nolabs-ai/nono` |
| crates.io | `nono-proxy` | `lukehinds` (sole owner) | — |
| crates.io | `nono-cli` | (not independently queried but same account family expected; treat as owned) | — |
| PyPI | `nono-py` | `lhinds` (sole "Owner" role, `pypi.org/pypi/nono-py/json`) | `0.11.0`, repo `nolabs-ai/nono-py` |
| npm | `nono-ts` | maintainer `lukehinds <lhinds@protonmail.com>` | `0.3.0` |

`cargo publish -p nono` (and `-proxy`/`-cli`), `maturin publish` / `twine upload` for `nono-py`, and `npm publish` for `nono-ts` **will all be rejected outright (403 / "not a package owner")** unless the operator's registry credentials (`CARGO_REGISTRY_TOKEN`, the PyPI token used by `twine`/`maturin publish`, and the npm token) belong to an account the upstream maintainer has explicitly added as a co-owner/maintainer on each of these five package identities. Nothing in `release.yml`, `RELEASE-RUNBOOK.md`, or the v3.3/v3.4 dry-run history establishes that this co-owner grant exists — the dry-runs (`cargo publish --dry-run`, `maturin build`, `npm publish --dry-run`) **do not exercise registry authentication/ownership at all**, so this failure mode is invisible to every gate that has been run so far.

**Why it happens:**
The fork is a downstream continuation of upstream's own crate/package names (by design — it is meant to converge with upstream, not rename). All prior "PASS" dry-run results check packaging/build correctness only, not registry-side authorization. The team has been repeatedly told (Phase 96/97/100) that the dry-run/readiness gates were green, which creates false confidence that "publish" is a formality — but auth/ownership is a distinct, unverified axis.

**How to avoid:**
Before any Step-3+ push in `RELEASE-RUNBOOK.md`, add an explicit **registry-identity preflight** that authenticates with the real (but non-mutating) credentials and checks ownership:
- `cargo owner --list -p nono` / `-p nono-proxy` / `-p nono-cli` with `CARGO_REGISTRY_TOKEN` set — confirms the token's account is a listed owner before any `cargo publish`.
- PyPI: there is no read-only "am I a maintainer" API call via twine; use `curl -H "Authorization: token $TOKEN" https://pypi.org/manage/project/nono-py/` behavior is not reliably scriptable — instead, resolve this **out-of-band with the upstream maintainer** (request a PyPI collaborator invite for the fork's account) before go-live, since PyPI has no dry-run-auth check.
- npm: `npm access ls-collaborators nono-ts` (read-only, requires being a team member to even list) or attempt `npm publish --dry-run` **with real auth** — note `--dry-run` still performs authentication and will surface `402`/`403` failures without publishing.
- If no co-owner grant exists and cannot be obtained quickly, the milestone must pivot: either request a name grant/transfer from upstream, or publish this fork under **new, fork-owned package names** (e.g. `nono-fork`, scoped npm `@oscarmackjr/nono-ts`, a differently-prefixed PyPI name) — a scope decision that changes `RELEASE-RUNBOOK.md` and every place these names are referenced (binding install docs, README badges). Note crate name is independent of the crates.io package registered under `Cargo.toml`'s `name` field — renaming there is what would be needed, not directory renaming.

**Warning signs:**
- No documented co-owner/collaborator invite email or crates.io "Pending owner invite" acceptance anywhere in project history.
- `RELEASE-RUNBOOK.md` and `release-dry-run.ps1` never mention `cargo owner --list` or any ownership check.
- The publish-crates job's idempotency check (`cargo search ... | grep -q "\"$VERSION\""`, see Pitfall 6) only ever branches on **version presence**, never on **authorization** — a 403 will surface as a raw `cargo publish` failure mid-workflow, after the Windows leg has already spent CI minutes signing and after `Create GitHub Release` has already run (since `publish-crates` runs `needs: release`, i.e. after the GitHub Release with real artifacts is already public).

**Phase to address:**
Registry-identity preflight belongs in the **verify-gate hardening / go-live-readiness phase**, run and resolved BEFORE the operator's Gate 2 (cut the release) — ideally even before Gate 1 smoke, since it's a pure identity check with no build cost. This is the single highest-leverage new check to add to this milestone; treat it as a go/no-go gate for the entire FUT-01 target feature, separate from and prior to the Trusted Signing verify-gate work.

---

### Pitfall 2: `UnknownError` on Authenticode verify is being treated as one problem — it is actually two (or three) independent failure modes that need separate disambiguation

**What goes wrong:**
The live field note (2026-06-30, run `28467925298`, recorded in the cookbook) shows: OIDC login succeeded, the Trusted Signing sign step succeeded (signer `CN=TWGGLOBAL.onmicrosoft.com`), but `Get-AuthenticodeSignature` on the runner reported `Status: UnknownError`, issuer `CN=Microsoft Enterprise ID Verified Policy AOC CA 01`. Three distinct, non-mutually-exclusive root causes can each independently produce this exact symptom, and fixing only one while assuming the others are ruled out wastes go-live cycles:

1. **Wrong profile type.** The observed issuer string `…Enterprise ID Verified Policy AOC CA…` does not match the publicly-trusted naming pattern `Microsoft ID Verified **CS** EOC/AOC CA NN`. "Enterprise" in the CA name strongly suggests the certificate profile in Azure is type **Private Trust / Test / Enterprise**, not **Public Trust** — an Enterprise/Private-Trust-issued cert is cryptographically valid but chains to a root that is **not** in the Windows/Microsoft public trusted-root program, so any host that hasn't separately trusted that private root will report an unresolvable/unknown chain.
2. **Chain-build failure (new intermediate not yet distributed).** Even a genuine Public Trust AOC/EOC intermediate can be "too new" for a given machine's local CryptoAPI/AuthRoot cache — Windows fetches root/intermediate updates lazily via Windows Update's AutoUpdate root program, and a `windows-latest` GitHub-hosted runner image (baked at a point in time, potentially with cached/pinned root store state) may not yet have fetched the newly-issued AOC intermediate. `Get-AuthenticodeSignature`/`WinVerifyTrust` cannot complete the chain and reports `UnknownError` even though the leaf certificate and signature bytes are perfectly valid.
3. **Revocation check timeout (CRL/OCSP).** Authenticode chain validation by default performs an online revocation check against the issuing CA's CRL/OCSP endpoint. On a network-constrained runner (proxy, egress filtering, transient outage) this check can time out; depending on flags, a timeout can surface as `UnknownError` rather than a clean revocation-specific error.

Treating "the profile is wrong" and "the runner can't build/verify the chain" as the same bug means fixing the Azure profile type (root cause #1) without addressing #2/#3 can still leave the exact same `UnknownError` on the next run, and the team will incorrectly conclude the profile fix didn't work.

**Why it happens:**
`UnknownError` is a generic .NET/CryptoAPI catch-all for "the trust engine could not resolve a definitive Trusted/Untrusted verdict" — it collapses several distinguishable low-level `CERT_TRUST_STATUS` bitflags (`CERT_TRUST_IS_PARTIAL_CHAIN`, `CERT_TRUST_REVOCATION_STATUS_UNKNOWN`, `CERT_TRUST_IS_UNTRUSTED_ROOT`) into one PowerShell-visible string. `Get-AuthenticodeSignature` does not expose the underlying `CERT_CHAIN_POLICY_STATUS` detail by default.

**How to avoid:**
Disambiguate with three independent, cheap probes before assuming a fix worked:
1. **Confirm profile type in Azure directly** (portal or `az trustedsigning certificate-profile show`) — must read `PublicTrust`, not `Test`/`PrivateTrust`. This is the authoritative source, not the issuer string alone (though the issuer naming — `CS` vs `Enterprise ID Verified Policy` — is a strong tell and matches what was actually observed).
2. **Re-verify the exact CI-signed binary offline on a separately-updated, internet-connected Win11 host** with `signtool verify /pa /v <file>` (not `/pa` alone — `/v` prints the full chain-build trace, including which intermediate/root it resolved and any revocation-check outcome). The smoke workflow already uploads the CI-signed exe as an artifact (`if: always()`) specifically for this offline re-check — use it. If `signtool /pa /v` says `Successfully verified` on an updated host, root causes #2/#3 (runner-side chain-build/revocation staleness) are confirmed and #1 (wrong profile) is ruled out.
3. **Force a fresh root-store fetch on the runner** as a targeted experiment: `certutil -generateSSTFromWU` or simply re-checking after an explicit Windows Update root-program sync, to see if the same runner resolves cleanly after an explicit refresh — isolates #2 from #3.
4. **Explicitly test with revocation checking disabled** as a diagnostic (not a shipped configuration) — if disabling revocation makes it `Valid`, root cause is #3 (CRL/OCSP timeout), not #1/#2.

Only after confirming the profile IS `PublicTrust` (step 1) should the CI verify step itself be hardened with a `signtool verify /pa /v` fallback (per the milestone's stated CI-hardening target feature) plus a documented, intentional decision on revocation-check behavior for CI (e.g., increase timeout, or accept `CERT_TRUST_REVOCATION_STATUS_UNKNOWN` as non-fatal if the rest of the chain resolves — this is a security-relevant tradeoff and should be an explicit, reviewed decision, not a silent workaround).

**Warning signs:**
- Fixing the Azure profile type and immediately re-running smoke without ALSO re-verifying with `/v` verbose output — if it fails again with the same generic `UnknownError`, you cannot tell from the PowerShell-only signal whether the profile fix "didn't work" or whether a *different* root cause (#2/#3) is now the blocker.
- Treating "Sign succeeded" as proof the profile is correct — it is not; Trusted Signing will happily sign with a Test/Enterprise profile too. Only Verify (and ultimately the issuer chain) proves profile type.

**Phase to address:**
Verify-gate hardening phase (CI, autonomous) — this is exactly the "diagnostics distinguishing `UnknownError`-chain-build from a genuine untrusted root" target feature already scoped. Make the `/v` verbose disambiguation step and the profile-type confirmation explicit, ordered gates in that phase, not an afterthought after the release tag is already pushed.

---

### Pitfall 3: `Get-AuthenticodeSignature` failing on the CI runner does not mean the artifact is actually untrustworthy — but the fail-closed gate cannot tell the difference, and must not be loosened carelessly

**What goes wrong:**
`release.yml:259` and the two other `Get-AuthenticodeSignature -ne 'Valid'` gates (`:311` MSI-wrapper check reused pattern, and the admin-extract MSI-payload check) are intentionally fail-closed (D-13) — any non-`Valid` status aborts the release and uploads nothing. This is the *correct* posture for a security-critical tool. But it creates a specific operational trap during go-live: if the true root cause is runner-side chain-build/revocation staleness (Pitfall 2, causes #2/#3) rather than a real signing defect, the team faces pressure to "just make the gate pass" — e.g., by weakening the check to accept `UnknownError`, or removing `/v`, or disabling revocation checking globally — which would silently reopen the door to shipping a genuinely untrusted-root artifact in some future scenario where `UnknownError` *does* mean a real problem.

**Why it happens:**
Under deadline pressure (this is explicitly a go-live milestone with an operator waiting), the fastest local "fix" for a CI gate turning red is to loosen the assertion rather than fix the underlying environment. The existing code comments (D-13, "fail closed if signing or verification fails") show the team already understands this principle for the PFX era; the temptation resurfaces anew with Trusted Signing's less-legible error surface.

**How to avoid:**
- Never change the `-ne "Valid"` fail condition to also accept `UnknownError`. Instead, resolve the *cause* of `UnknownError` (Pitfall 2) so the runner reliably produces `Valid`.
- If a runner-side chain-build staleness genuinely cannot be fixed (e.g., GitHub's `windows-latest` image lags Microsoft's root-program updates), the correct hardening is an explicit **pre-verify remediation step** — e.g., a root-store refresh or an updated runner image pin — run *before* the `Get-AuthenticodeSignature` gate, not a loosening of the gate itself. Add a `signtool verify /pa /v` step as a second, independent corroborating check (per the milestone's stated design) — but treat it as *additional evidence for diagnosis*, not as a replacement trust decision; the release gate should still require both checks to agree, and abort if either disagrees.
- Keep the offline-verification path (smoke workflow's `if: always()` artifact upload) as a permanent CI habit for any future signing regression, not a one-off debugging step.

**Warning signs:** A PR or commit that touches the verify step's condition (`-ne "Valid"`) without also touching root-cause remediation (profile type, root-store refresh, revocation handling) — review that diff with extra scrutiny; this is exactly the kind of "looks like a fix, is actually a security regression" change flagged as CRITICAL in CLAUDE.md's Security Considerations.

**Phase to address:** Verify-gate hardening phase. Explicitly write a design note/ADR for *why* the fail-closed condition stays as-is even after hardening, so future maintainers don't reopen this later.

---

### Pitfall 4: SmartScreen "Windows protected your PC" on first run is expected reputation-cold-start behavior, not a signing defect — but will be reported as a bug by the operator/UAT tester if not pre-briefed

**What goes wrong:**
A brand-new Trusted Signing Public Trust profile has **zero SmartScreen application reputation** the first time it's used — Microsoft's reputation system (distinct from Authenticode chain trust) builds up over time and download volume. The cookbook's own troubleshooting table already anticipates this: "Signature `Valid` but 'Windows protected your PC' on first run → new `AOC CA` intermediate has no SmartScreen reputation yet → Expected for new profiles; reputation accrues over time/volume. Validity is unaffected." The risk is not the phenomenon itself but that Gate 3 (clean-host UAT) will be run by someone unaware of this, who sees a scary SmartScreen interstitial on a "signed" release and (a) treats it as proof signing failed, re-opening Pitfall 2/3 debugging on a non-issue, or (b) blocks go-live waiting for a reputation threshold that has no fixed timeline.

**Why it happens:**
Authenticode validity (what `Get-AuthenticodeSignature`/`signtool` check) and SmartScreen application reputation (what causes the "Windows protected your PC" modal) are two separate Microsoft trust systems. A cryptographically `Valid` signature from a brand-new publisher identity can still trigger SmartScreen's reputation-based UX warning.

**How to avoid:**
- Document explicitly, in the Gate 3 UAT script/checklist, that a SmartScreen prompt on first run is an **expected, non-blocking** UX event for a new signing identity — the acceptance criterion for Gate 3 is Authenticode `Valid` + issuer chains to the public CS root + the MSI installs / broker spawns when the user clicks "More info → Run anyway", NOT the absence of the SmartScreen dialog.
- Separately note this in end-user-facing release notes/docs for the first trusted-signed release so downstream users aren't alarmed either.
- Do not attempt to "fix" this by requesting an EV cert or a reputation bypass — Trusted Signing's whole value proposition here is the publicly-trusted chain; reputation is a volume-based process, not a purchasable guarantee.

**Warning signs:** A UAT run report or issue that says "signing is broken, Windows blocks the app" without checking `Get-AuthenticodeSignature`/`signtool verify` status first.

**Phase to address:** Azure VM clean-host IaC + scripted gates phase — bake this expectation into the `verify-dark.ps1` clean-host gate's documentation/output (e.g., a note distinguishing "Authenticode: Valid" from "SmartScreen reputation: cold, expected"), and into the operator go-live runbook/checklist for Gate 3.

---

### Pitfall 5: The operator's corporate dev host cannot be used as a "clean host" stand-in — and the confounds are specific and stackable

**What goes wrong:**
PROJECT.md already correctly identifies that the corporate dev host is not a valid clean-host test environment, but the *reasons* are worth being explicit about because each one can independently mask or fake a result:
1. **The POC self-signed cert (`CN=nono Test Signing`) was previously imported into `LocalMachine\Root`/`TrustedPublisher`** on this host (per the `poc-cert-broker-clean-host` todo's own workaround). Any subsequent Authenticode check on this host is contaminated — it may show `Valid`/broker-spawns-fine even for artifacts that would fail on a genuinely clean host, because the host's trust store has been manually widened.
2. **VC++ redistributable is already installed** on the dev host (implied by normal dev tooling), which masks the exact `0xC0000135`/service-start failure class the `msi-vcredist-prereq` todo is trying to prove is fixed by `+crt-static`.
3. **Managed corporate trust store / Group Policy root-certificate distribution** may inject or block root CAs differently than a stock consumer/cloud Win11 image, changing chain-build outcomes independent of the Trusted Signing profile itself.
4. **Corporate proxy/firewall** may intercept or block outbound HTTP(S) calls needed for AIA (Authority Information Access, fetching intermediate certs) and CRL/OCSP revocation checks — this can *independently reproduce* the exact `UnknownError` symptom from Pitfall 2 cause #3, making it look like a signing pipeline defect when it is actually a network-egress artifact of the test host, not the artifact or the CI runner.
5. **EDR/antivirus** on a managed corporate endpoint may quarantine, delay-scan, or silently modify a freshly downloaded unfamiliar `.exe`/`.msi` before Authenticode verification runs, producing flaky or misleading results.

Because these confounds can each mask a real problem *or* fabricate a fake one, a "PASS" or a "FAIL" observed on the corporate host is not trustworthy evidence in either direction for this milestone's specific verify-gate and clean-install questions.

**Why it happens:** It's the fastest, most convenient host to test on, and results often "look right" — the trap is that a false PASS is just as damaging here as a false FAIL, because it would let an unresolved `UnknownError`/vcredist-dependency defect ship as "verified."

**How to avoid:**
- Use GitHub's `windows-latest` cloud runner for anything that only needs a *stock, non-manually-trusted* Windows environment (most of the Trusted Signing verify-gate debugging — per PROJECT.md's own framing, "Most verify-gate debugging needs no user host").
- Reserve the Azure VM specifically for the tests that need genuine out-of-box behavior: (a) MSI clean-install with no VC++ redist, (b) broker-spawn with no manual cert-trust step, (c) final Authenticode confirmation on a machine that has *never* seen the POC cert.
- The Azure VM build must start from a stock marketplace Win11 image (no dev tooling, no VC++, no manually imported certs) and must NOT reuse or clone from the operator's existing dev-host image/snapshot.
- If the corporate network path is used for the VM's egress (e.g., VM deployed inside the corporate vnet/ExpressRoute), explicitly verify AIA/CRL/OCSP outbound reachability separately — don't assume "it's an Azure VM" automatically means clean, unfiltered internet egress (see Pitfall 11).

**Warning signs:** Any UAT note that says "confirmed working on my machine" without specifying it was the *provisioned clean Azure VM*, not the operator's daily-driver Windows host.

**Phase to address:** Azure VM clean-host IaC + scripted gates phase — the IaC itself is the prevention mechanism (a reproducible, from-scratch VM removes the ambiguity); the phase's gates should assert/log "this host has never had `nono setup --trust-broker` or the POC cert run" as a precondition, failing closed (`SKIP_HOST_UNAVAILABLE` or similar) if that can't be established.

---

### Pitfall 6: The crates.io publish idempotency check (`cargo search`) is a fragile, historically-unreliable primitive for a step that must not double-publish

**What goes wrong:**
`release.yml`'s `publish-crates` job checks "already published?" via:
```bash
if cargo search nono --limit 1 | grep -q "\"$VERSION\""; then
  echo "already published, skipping"
else
  cargo publish -p nono --allow-dirty --token ...
fi
```
`cargo search` depends on crates.io's search-index backend and its own output formatting (a `name = "x.y.z"    # description` line), and its results/availability have been inconsistent across crates.io API changes over the past few years (rate limiting, backend migrations). If `cargo search` returns empty, errors, or a differently-formatted line for any reason at publish time, the `grep -q` check silently evaluates false and the script falls through to `cargo publish` regardless of whether the version already exists. This is not catastrophic on its own — crates.io publish is immutable and idempotent-safe in the sense that publishing an already-existing exact version simply fails with a clear error rather than corrupting anything — but it means the "skip" branch cannot be trusted as a real idempotency guarantee, and a retried/re-run workflow could hit a confusing `error: crate version already uploaded` on a step the log claims should have "skipped".

**Why it happens:** `cargo search` was written as a developer convenience command, not a machine-parseable API; using its stdout as a scripted precondition is inherently brittle, and this fork inherited the pattern without a live test against the real registry (dry-runs never reach a live registry).

**How to avoid:**
- Before relying on this check at go-live, manually validate `cargo search nono --limit 1` still returns parseable output in the CI environment (a cheap, non-mutating smoke check — run it standalone in a workflow_dispatch job, not embedded in the real publish job, before the actual tag push).
- Treat a `cargo publish` failure with `error: crate version ... already exists on crates.io index` as an *expected, non-fatal* "already done" signal in the publish script (specifically match that error text) rather than depending on the pre-check to prevent the call in the first place — this is the more robust idempotency strategy and doesn't depend on `cargo search` at all.
- Because of Pitfall 1 (ownership), this check is currently secondary to the bigger blocker — but once ownership is resolved, this fragility becomes the next thing that can turn a clean retry into a confusing partial-failure state.

**Warning signs:** A `publish-crates` job log showing "already published, skipping" for a version that in fact was never published (visible by checking crates.io directly) — silent false-skip.

**Phase to address:** Go-live execute / live multi-registry publish phase — harden this specific idempotency check as part of preparing the *live* (not dry-run) publish path, since the dry-run path never actually exercises `cargo search` against the real network with the real version string.

---

### Pitfall 7: crates.io publish order is a strict, irreversible, one-way dependency chain — a mid-sequence failure leaves the registry in a permanently half-published state

**What goes wrong:**
`RELEASE-RUNBOOK.md` and `release.yml` publish in fixed order `nono` → (sleep 30) → `nono-proxy` → (sleep 30) → `nono-cli`, because `nono-cli`/`nono-proxy` depend on `nono` and crates.io's publish-time dependency resolution requires the dependency to already be indexed. **crates.io does not support unpublishing** (only "yanking," which hides a version from *new* dependency resolution but does not remove it, does not free the version number for reuse, and does not fix a half-published release). If `nono` publishes successfully but `nono-proxy` or `nono-cli` then fails (build error, ownership problem discovered only at that crate, network blip, or the 30-second sleep being insufficient for crates.io's index to propagate under load), the workspace is left in an inconsistent state: `nono 0.66.1` is live and immutable, but the dependent crates are not, and there is no "undo" — the only path forward is fixing the failure and publishing the *same* version of the remaining crates (which still works, since only `nono` itself is version-locked), or bumping to `0.66.2` for the crates that need a code fix, creating an asymmetric version family that the `version-family` consistency check in the readiness gate does not currently anticipate.
The `sleep 30` between crates is a heuristic, not a guarantee — crates.io indexing/propagation time is not contractually bounded, and a `sleep 30` that was sufficient in past dry runs is not proof it will be sufficient under different registry load at go-live time.

**Why it happens:** Immutable, append-only package registries are a deliberate integrity design (prevents supply-chain tampering) — but that same property means every publish mistake is permanent, and staged multi-crate releases have an inherent window of partial-completion risk that cannot be made fully atomic.

**How to avoid:**
- Before Step 4 in `RELEASE-RUNBOOK.md`, re-run `cargo publish --dry-run -p nono-proxy` and `-p nono-cli` **immediately after** `nono` is confirmed live (not from the pre-push dry run) — this is the closest available preflight to catching a downstream build/manifest problem before it's irreversible.
- Treat the 30-second sleep as a minimum, not a fixed value — consider polling `https://crates.io/api/v1/crates/nono` for the new version to appear (bounded retry loop) instead of a blind sleep, so indexing delays under load don't silently produce a "not yet visible" dependency-resolution failure on the next crate.
- Document explicitly, in the runbook, what to do if the chain breaks mid-sequence (accept the partial-publish state, do NOT attempt to reuse the version number, and know that a fix-forward `0.66.2` on only the broken crates is the recovery path) — so the operator doesn't panic-improvise during a live go-live.
- This risk is compounded by, but independent of, Pitfall 1 (ownership) — even once ownership is resolved for `nono`, `nono-proxy`'s or `nono-cli`'s ownership could still independently fail at a different point in the chain if the co-owner grant wasn't applied uniformly to all crate names.

**Warning signs:** Any go-live retro/postmortem where "we just re-ran the whole publish job" is treated as safe — re-running is safe only for crates.io's own already-published-version guard (Pitfall 6), not for crates that legitimately failed to publish and need investigation before retry.

**Phase to address:** Go-live execute / live multi-registry publish phase. Update `RELEASE-RUNBOOK.md`'s Step 4 with the polling-instead-of-sleep approach and an explicit partial-failure recovery note before the operator's first live attempt.

---

### Pitfall 8: PyPI and npm publishes are similarly one-way (immutable versions, no re-upload), and the two registries fail differently — a partial 3-registry publish is a normal outcome, not a bug

**What goes wrong:**
Unlike crates.io's clear "already exists" error, PyPI rejects a re-upload of an existing version with `400 File already exists` and npm rejects with `403 You cannot publish over the previously published versions`. Both are equally immutable/irreversible per version, but the specific error text and retry semantics differ, so a single generic "retry the whole release" script/mental-model across all three registries will behave inconsistently. Additionally:
- **PyPI-specific:** `maturin publish` builds wheels for whatever platform the CI runner is (or the operator's local machine, per `RELEASE-RUNBOOK.md`'s manual fallback `maturin build --release` + `twine upload`) — if this is done from a single Windows/Linux dev machine rather than a proper multi-platform CI matrix, the published PyPI release may ship **only one platform's wheel** (or an sdist that requires a Rust toolchain to build from source on install), silently degrading `pip install nono-py` for other platforms even though the "publish succeeded." Verify what wheel tags actually land on PyPI after publish, not just that the command exited 0.
- **npm-specific:** `nono-ts` is a napi-rs binding (per its `.napi`/artifacts scripts) — same platform-wheel-equivalent risk: native `.node` bindings are typically split into per-platform optional-dependency packages (a documented failure mode already visible on the upstream project: a GitHub issue found during this research, "Missing system-specific package on npm" on `always-further/nono-ts`, describes `nono-ts` failing on Linux because platform-specific packages were never published). This is a **known, previously-observed failure mode on this exact package family** — confirm the npm publish step in this milestone's pipeline actually publishes every required platform-specific package (or explicitly scopes v3.5's npm publish to a single-platform release with that limitation documented), not just the top-level `nono-ts` package.

**Why it happens:** Multi-registry, multi-platform native-binding releases have more moving parts than a single-registry pure-source release, and "the publish command exited 0" is a much weaker signal of completeness than it is for a pure-Rust crate.

**How to avoid:**
- After each registry's live publish, verify the actual published artifact list (PyPI project files page / `pip download --no-deps` per platform; npm `npm view nono-ts` + check for platform-specific sibling packages) rather than trusting exit code alone.
- If the fork's napi-rs/maturin build matrix doesn't yet produce all platform artifacts in this milestone's CI, explicitly scope FUT-01's npm/PyPI publish to the platforms actually built and document the gap — don't silently ship a partial multi-platform release that repeats the exact bug already known to exist upstream.

**Warning signs:** `npm publish` or `maturin publish` succeeding with only a single `.whl`/native binary staged locally, with no CI matrix step that built the others.

**Phase to address:** Go-live execute / live multi-registry publish phase — add an explicit "verify published platform coverage" check as part of FUT-01's acceptance criteria, referencing the known upstream `nono-ts` Linux-package-missing issue as the concrete failure to avoid repeating.

---

### Pitfall 9: `AADSTS700213` (no matching federated identity record) is the most common OIDC failure and has multiple independent causes beyond the one already documented

**What goes wrong:**
The cookbook already fixed one instance of this (wrong repo casing / `ref:` vs `environment:` subject). But `AADSTS700213` recurs whenever *any* of the following drift: the GitHub environment name (`Development`) is renamed or the job's `environment:` key is removed/misspelled; the repo is renamed or transferred again; a workflow is invoked via a path that changes the OIDC subject claim (e.g., a reusable/called workflow, a fork-triggered PR run, or a `workflow_dispatch` from a branch vs the intended ref — subject claims can differ between `push`-triggered and `workflow_dispatch`-triggered runs depending on GitHub's OIDC claim construction); or a *second* federated credential with a stale subject is left registered and Azure's matching picks the wrong one if subjects overlap ambiguously (unlikely, but the cookbook's own §3b explicitly warns to delete old FICs, not just add new ones, "to avoid confusion").

**Why it happens:** Federated credential subjects are exact-string-match, case-sensitive, and tied to specific trigger/claim shapes; any pipeline edit that changes how the workflow triggers (including something as small as changing `workflow_dispatch` input handling or moving a job to a different `environment:`) silently invalidates the trust relationship.

**How to avoid:**
- Treat GATE 1 (smoke test) as the FIC canary on every future CI change that touches `release.yml`'s `environment:` key, trigger type, or the repo/org name — re-run smoke after any such edit, not just at initial go-live.
- Keep exactly one FIC per intended trigger shape; delete stale ones immediately (already correctly instructed in the cookbook — just don't skip it).
- If `AADSTS700213` recurs, the fastest diagnostic is `az ad app federated-credential list --id "$APP_ID" -o table` compared byte-for-byte against the actual subject GitHub's OIDC token presents (obtainable by decoding the JWT from a deliberately-failing run's debug logs) rather than guessing.

**Warning signs:** `azure/login` failing on a workflow edit that "shouldn't have touched auth" — e.g., renaming the `Development` environment, or converting `release.yml`'s Windows leg into a separate reusable workflow (which would change the subject's job/environment path).

**Phase to address:** Verify-gate hardening phase (guard against regressions) and go-live execute phase (final confirmation before Gate 2).

---

### Pitfall 10: Secret-retirement ordering — don't delete the POC fallback before the real thing is proven end-to-end

**What goes wrong:** The close-out checklist (cookbook §8, mirrored in PROJECT.md's "Close-out" target feature) retires `WINDOWS_SIGNING_CERT`/`_PASSWORD` and deletes `trusted-signing-smoke.yml`. If this happens as soon as Gate 2 (release workflow green) passes — rather than after Gate 3 (actual clean-host UAT on the Azure VM) also passes — the team has removed its only fallback signing path before confirming the new path produces a genuinely install-and-run-able artifact on a real clean host. If Gate 3 then reveals a problem (e.g., broker still won't spawn for some unrelated reason, or a chain-build issue that only manifests on a truly clean host and wasn't visible on `windows-latest`), there is no POC path left to fall back to for an interim patch release.

**Why it happens:** Close-out checklists get executed as a batch once "the release is out," and the distinction between "Gate 2 green" (CI-runner-verified) and "Gate 3 green" (independently, freshly-provisioned-host-verified) is easy to blur under the excitement of a first successful tag push.

**How to avoid:** Sequence close-out strictly as: Gate 1 (smoke) green → Gate 2 (release workflow, including all its Authenticode gates) green → Gate 3 (Azure VM clean-host UAT: MSI install + broker spawn with no manual cert-trust step) green → only then retire secrets and delete the smoke workflow. This matches the cookbook's own section ordering (§8 is explicitly "after Gate 3 passes") — the risk is process drift where an eager operator jumps to close-out right after Gate 2 because "the release published."

**Phase to address:** Close-out phase — but the *guard* belongs earlier: make the close-out checklist/PR itself structurally require a link to a passed Gate 3 verdict (e.g., the `verify-dark.ps1` clean-host verdict JSON) before secret-retirement commands are permitted to run, not just a written reminder.

---

### Pitfall 11: Azure Win11 VM provisioning gotchas specific to this project's needs

**What goes wrong, concretely:**
- **Image licensing:** Azure Marketplace Windows 11 images require either a pay-as-you-go Windows license baked into the VM's per-hour cost, or Azure Hybrid Benefit with an eligible on-prem license — using an unlicensed/eval image can trigger periodic license-nag behavior or reduced eval-period functionality that could confound "does the MSI install cleanly out of the box" testing (a licensing nag dialog is not the same as a `nono`-related failure, but could be misread as one during a UAT session).
- **Networking/egress for AIA, CRL, OCSP, and the timestamp authority:** the VM needs unfiltered outbound HTTPS/HTTP to Microsoft's certificate-chain infrastructure (AIA fetch for intermediates, CRL/OCSP responders) AND to `http://timestamp.acs.microsoft.com` (the RFC3161 timestamp authority `release.yml` already uses) for a fully-representative verify test — a default/locked-down Azure NSG or a corporate-peered vnet (see Pitfall 5 and Security Mistakes table) can silently block exactly the traffic needed to prove or disprove Pitfall 2's revocation-timeout hypothesis, again risking a false attribution of the failure to the signing pipeline itself rather than the VM's network posture.
- **RDP egress from a corporate tenant:** if the Azure subscription is associated with the operator's corporate tenant, conditional access policies (MFA, trusted-location, Intune-compliance requirements) may block or complicate RDP/Bastion access to the new VM in ways unrelated to the VM's own configuration — budget time for this, and prefer Azure Bastion (browser-based, no exposed RDP port) over raw RDP over the public internet both for security and to sidestep local-firewall/VPN routing surprises.
- **VM cleanup:** a Win11 Azure VM is billed hourly (and disk storage persists even when deallocated) — deallocate or delete immediately after Gate 3 to avoid ongoing cost, and prefer "delete" over "stop" once the UAT evidence (verdict JSON, screenshots) has been captured, since a lingering "clean" VM that later has other things installed on it for unrelated debugging stops being reusable as a clean baseline for a future release's Gate 3.

**How to avoid:** Scope the Bicep/IaC explicitly to: (a) a licensed Azure Marketplace Win11 Pro/Enterprise image (not a custom/manually-built one), (b) an NSG allowing only the operator's known egress IP for RDP/Bastion and unrestricted outbound 80/443 (default Azure NSGs already allow outbound internet by default — just don't add a restrictive outbound rule for "security" that then blocks AIA/CRL/timestamp traffic), (c) a teardown step (manual or scripted) as part of the Gate 3 runbook, not left as a "someday" cleanup task.

**Phase to address:** Azure VM clean-host IaC + scripted gates phase.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|-----------------|------------------|
| Loosening `Get-AuthenticodeSignature -ne 'Valid'` to also accept `UnknownError` | Unblocks the release TODAY | Reopens the exact untrusted-signature risk D-13 exists to prevent; a genuinely bad chain could ship silently | Never — fix root cause (Pitfall 2/3) instead |
| Relying on `cargo search` grep-based idempotency instead of matching `cargo publish`'s "already exists" error text | Simple, already-written | Silent false-skip risk under registry API drift (Pitfall 6) | Acceptable short-term IF paired with a manual `cargo search` sanity check immediately before the real go-live tag push; replace before making this a recurring/automated release cadence |
| `sleep 30` between dependent crates.io publishes instead of polling for index visibility | Simple, no extra code | Irreversible mid-chain failure if crates.io indexing is slower than 30s under load (Pitfall 7) | Acceptable for a low-frequency, human-supervised go-live where the operator is watching the log and can react; not acceptable for a fully unattended future release automation |
| Testing clean-host behavior on the corporate dev host "because it's faster" | Fast iteration | Both false-PASS (POC cert pre-trusted) and false-FAIL (VC++/proxy/EDR confounds) risk (Pitfall 5) | Never for the specific Gate 3 acceptance claims — only acceptable for unrelated, non-trust-sensitive dev iteration |
| Publishing to PyPI/npm from a single local/dev machine instead of a CI matrix | Gets *a* package live quickly | Silently ships partial platform coverage, repeating the known upstream `nono-ts` Linux-missing-package bug (Pitfall 8) | Acceptable only as an explicitly-scoped, documented interim (e.g., "Windows + macOS only for this release") — never silently |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|-----------------|-------------------|
| Azure Trusted Signing (`azure/trusted-signing-action`) | Assuming "Sign succeeded" implies "profile is Public Trust" | Sign and Verify are separate proofs; only Verify (with issuer inspection) confirms trust chain type — see Pitfall 2 |
| `azure/login` OIDC + federated credential | Editing `environment:`/trigger shape in `release.yml` without re-testing the FIC subject | Re-run the smoke workflow after ANY change to job `environment:`, trigger type, or repo identity — see Pitfall 9 |
| crates.io publish (multi-crate dependency chain) | Assuming a failed step can just be "retried" like any CI step | Understand crates.io's append-only/no-unpublish model before retrying; a failure mid-chain needs investigation, not blind retry — see Pitfall 7 |
| crates.io / PyPI / npm registry auth | Assuming dry-run success implies live-publish will succeed | Dry-runs never authenticate against the live registry with the real token — ownership/authorization is untested until the real push; verified in this research that all three package identities are currently owned by upstream (`lukehinds`/`lhinds`), not this fork — see Pitfall 1 |
| napi-rs (nono-ts) multi-platform native bindings | Publishing only the platform the build machine happens to be | Confirm the full platform-package matrix publishes; this exact gap is a documented, live upstream bug on this package family — see Pitfall 8 |
| Windows root-certificate distribution (AuthRoot) | Assuming a `windows-latest` GitHub runner has the latest Microsoft root-program state | New Public Trust intermediates can lag runner image root-store snapshots; corroborate with an independently-updated host — see Pitfall 2 |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Weakening the fail-closed Authenticode verify gate to work around `UnknownError` | Ships an artifact whose trust chain was never actually confirmed valid; defeats the entire purpose of D-13/D-02 | Fix the root cause (profile type / root-store staleness / revocation timeout), never the assertion — Pitfall 3 |
| Retiring the POC signing secrets (`WINDOWS_SIGNING_CERT`/`_PASSWORD`) before the trusted-signed release is confirmed green end-to-end (all gates + live clean-host UAT) | If the Trusted Signing path breaks after secret retirement, there is no fallback signing path at all — worse than the status quo, and could leave a broken release.yml unable to produce ANY signed artifact | Retire strictly in Close-Out, only after Gate 3 (clean-host UAT) has passed on the actual Azure VM with the real trusted-signed artifact — not merely after Gate 2 (release workflow green) — Pitfall 10 |
| Deploying the Azure clean-host VM inside the corporate network/vnet for "convenience" | Reintroduces the exact proxy/EDR/managed-trust-store confounds the VM exists to eliminate (Pitfall 5), and may also expose corporate credentials/network to an intentionally disposable, minimally-hardened test VM | Provision the VM on a standalone/isolated Azure subscription or vnet with direct internet egress, not peered into the corporate network |
| Leaving the RDP/management endpoint of the clean-host VM open to the internet ("0.0.0.0/0") for convenience during a one-off UAT session | Direct exposure of a freshly-provisioned Windows box to internet-wide RDP brute-force/exploit scanning | Scope RDP/WinRM access via Azure NSG to the operator's known egress IP only (or Azure Bastion), and destroy/deallocate the VM promptly after Gate 3 — Pitfall 11 |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-------------------|
| SmartScreen "Windows protected your PC" on first run of a brand-new signing identity | End users/UAT testers assume signing is broken and may abandon install | Document explicitly in release notes and Gate 3 script output that this is expected reputation-cold-start behavior, not a validity failure — Pitfall 4 |
| A `UnknownError` verify result with no further detail | Operator/engineer has no actionable next step from the raw PowerShell output alone | Always pair with `signtool verify /pa /v` for the verbose chain trace before concluding anything — Pitfall 2 |
| "Dry-run PASS" language across three different registries with three different actual guarantees (crates.io dry-run genuinely validates the package; PyPI/npm dry-runs vary in how much they check, and none check live auth) | False confidence that "all three registries are ready" | Report dry-run results per-registry with an explicit caveat on what was and wasn't checked (packaging vs. auth vs. platform coverage) |

## "Looks Done But Isn't" Checklist

- [ ] **Trusted Signing "live"**: Smoke workflow green is often read as "signing is live" — verify it also means Verify (not just Sign) returned `Valid` with an issuer chaining to the public `Microsoft ID Verified CS` root, on both the CI runner AND an independently-updated offline host (Pitfall 2).
- [ ] **"Publish dry-run PASS"**: Does not mean the live publish will succeed — verify registry ownership/authorization separately and explicitly before the real push (Pitfall 1).
- [ ] **"Release workflow green"**: Does not by itself mean the artifact is trustworthy on a genuinely clean host — SmartScreen cold-reputation and any remaining chain-build staleness on non-CI hosts are separate questions requiring Gate 3 (Pitfalls 2, 4, 5).
- [ ] **"npm/PyPI publish succeeded"**: Does not mean all target platforms are installable — verify the actual published artifact/platform-package list, especially for the known napi-rs multi-platform gap on `nono-ts` (Pitfall 8).
- [ ] **"Clean-host UAT passed"**: Verify it actually ran on the freshly-provisioned Azure VM (never touched by the POC cert / dev tooling), not the operator's corporate dev host (Pitfall 5).
- [ ] **"Secrets retired"**: Verify this happened strictly after Gate 3 passed on the live VM, not merely after the release workflow turned green (Pitfall 10).

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|----------------|------------------|
| Registry ownership blocks live publish (Pitfall 1) | MEDIUM | Request co-owner/collaborator invite from upstream maintainer for all 5 package identities before re-attempting; if declined/unresponsive, rename the fork's published package identities and update every downstream reference (docs, README badges, install instructions) — a scope change, not a quick patch |
| Mid-chain crates.io publish failure (Pitfall 7) | LOW–MEDIUM | Do not retry blindly; confirm exactly which crates published, fix the specific failure (build/manifest/ownership), and publish only the remaining crates at the SAME version if the failure was transient, or bump to a patch version for the broken crate(s) if a code fix was needed |
| Partial PyPI/npm platform coverage discovered post-publish (Pitfall 8) | MEDIUM | Cannot retract the published version; publish a follow-up patch version with the missing platform artifacts and document the gap in that version's release notes (mirrors how the known upstream `nono-ts` issue was eventually expected to be resolved) |
| `UnknownError` persists after profile-type fix (Pitfall 2) | LOW | Re-run the three disambiguation probes (verbose `signtool`, root-store refresh, revocation-disabled test) to isolate whether it's now cause #2 or #3, rather than re-guessing at the profile |
| Secrets retired prematurely and Trusted Signing breaks (Security Mistakes table / Pitfall 10) | HIGH | No fallback signing path exists once POC secrets are deleted — recovery requires either fixing Trusted Signing under time pressure with no safety net, or re-provisioning a temporary POC PFX (defeats the whole migration's purpose) — this is exactly why retirement ordering matters |
| Azure VM accidentally left running/exposed after UAT (Pitfall 11) | LOW | Deallocate/delete promptly; rotate any credentials that may have touched the VM if RDP was ever exposed broadly |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|-------------------|---------------|
| 1. Registry name ownership not verified (crates.io/PyPI/npm all upstream-owned) | Verify-gate hardening / go-live-readiness phase (run FIRST, before any signing work) | `cargo owner --list -p nono/-proxy/-cli` returns the fork's own token account; PyPI/npm collaborator status confirmed with upstream maintainer |
| 2. `UnknownError` conflated root causes (profile type vs chain-build vs revocation) | Verify-gate hardening phase | Azure profile confirmed `PublicTrust` via `az trustedsigning certificate-profile show`; `signtool verify /pa /v` on an independently-updated host returns `Successfully verified`; CI verify step distinguishes and logs which failure class occurred |
| 3. Fail-closed gate weakened as a workaround | Verify-gate hardening phase (code review gate) | Diff review confirms `-ne "Valid"` condition unchanged; any new logic is additive corroboration (e.g., `/v` verbose logging), not a loosened pass condition |
| 4. SmartScreen cold-start UX surprise | Azure VM clean-host IaC + scripted gates phase | Gate 3 script/checklist explicitly documents SmartScreen as expected and non-blocking; UAT report distinguishes it from Authenticode status |
| 5. Corporate-host confounds invalidate clean-host claims | Azure VM clean-host IaC + scripted gates phase | VM provisioning script starts from a stock marketplace image with no POC cert / no VC++ / no prior `nono setup` run; gate asserts this precondition and fails closed if unmet |
| 6. Fragile `cargo search` idempotency check | Go-live execute / live multi-registry publish phase | Manual `cargo search nono --limit 1` sanity-checked against live output immediately before the real tag push; publish script's error-handling also matches crates.io's actual "already exists" error text as a backstop |
| 7. Irreversible mid-chain crates.io publish failure | Go-live execute / live multi-registry publish phase | `RELEASE-RUNBOOK.md` updated with polling-based (not fixed-sleep) dependency-visibility wait and an explicit partial-failure recovery procedure |
| 8. Partial PyPI/npm platform coverage | FUT-01 live multi-registry publish phase | Post-publish check confirms every intended platform artifact/package is present on PyPI/npm, explicitly cross-checked against the known upstream `nono-ts` platform-package gap |
| 9. `AADSTS700213` recurrence on pipeline edits | Verify-gate hardening phase + go-live execute phase | Smoke workflow re-run (and green) after ANY edit to `release.yml`'s `environment:`/trigger shape, immediately before Gate 2 |
| 10. Premature secret retirement | Close-out phase | Explicit ordering enforced: retire `WINDOWS_SIGNING_CERT`/`_PASSWORD` ONLY after Gate 3 (Azure VM UAT) has passed, never merely after Gate 2 (release workflow green) |
| 11. Azure VM licensing/networking/egress gotchas | Azure VM clean-host IaC + scripted gates phase | IaC uses a properly licensed Azure Marketplace Win11 image (not a manually-activated/unlicensed build); NSG scoped to operator IP only; VM deallocated/destroyed promptly post-UAT; outbound 443/80 to the Trusted Signing endpoint, AIA/CRL/OCSP responders, and `timestamp.acs.microsoft.com` explicitly confirmed reachable before relying on the VM for verify testing |

## Sources

- This repository, read directly during research (HIGH confidence, primary evidence):
  - `.github/workflows/release.yml` (verify-gate logic lines 259–274, 281–321; publish-crates job lines 483–539; environment/OIDC wiring lines 24–32, 146–180)
  - `.github/workflows/trusted-signing-smoke.yml` (smoke test design, offline-verification artifact upload)
  - `.planning/quick/260630-trusted-signing-golive/AZURE-TRUSTED-SIGNING-GOLIVE-COOKBOOK.md` (field note on live run `28467925298`, Troubleshooting table)
  - `.planning/quick/260603-i31-cosign-sigstore-signing-authority/AZURE-TRUSTED-SIGNING-COOKBOOK.md` (Public Trust vs Test profile distinction)
  - `.planning/todos/pending/20260611-poc-cert-broker-clean-host.md` and `20260611-msi-vcredist-prereq.md` (corporate-host contamination history, VC++ dependency root cause)
  - `.planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md` (live push sequence, publish-order/sleep pattern)
  - `docs/cli/development/windows-signing-guide.mdx` (current signing architecture, POC-vs-CI split)
  - `.planning/PROJECT.md` (v3.5 milestone scope, target features, key decisions)
- Live registry queries performed during this research (HIGH confidence, directly verified):
  - `https://crates.io/api/v1/crates/nono` and `/owners` — owner `lukehinds`, max_version `0.66.0`, repo `nolabs-ai/nono`
  - `https://crates.io/api/v1/crates/nono-proxy/owners` — owner `lukehinds`
  - `https://pypi.org/pypi/nono-py/json` — owner `lhinds`, version `0.11.0`, repo `nolabs-ai/nono-py`
  - `https://registry.npmjs.org/nono-ts` — maintainer `lukehinds`/`lhinds@protonmail.com`, version `0.3.0`
  - Local `pyproject.toml` (`../nono-py`) and `package.json` (`../nono-ts`) — confirm unedited upstream author metadata (`Luke Hinds`) and package names (`nono-py`, `nono-ts`) still in place in the fork's own binding repos
- WebSearch (MEDIUM confidence, corroborating, general Trusted Signing/PKI domain knowledge not independently reproduced in this environment):
  - General Authenticode chain-build/revocation-check behavior distinguishing `UnknownError` from `UntrustedRoot`/explicit revocation failures (standard Windows CryptoAPI/WinVerifyTrust behavior, consistent with the observed field-note symptom)
  - GitHub issue "Missing system-specific package on npm" on `always-further/nono-ts` — corroborates Pitfall 8's napi-rs multi-platform publish gap as a real, previously-observed defect on this exact package family

---
*Pitfalls research for: Azure Trusted Signing go-live + first distributed multi-registry release (nono v3.5)*
*Researched: 2026-07-02*
