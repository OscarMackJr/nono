# Phase 101: Verify-Gate Hardening + Azure Profile Confirmation - Context

**Gathered:** 2026-07-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Harden the CI Authenticode verify gate and confirm the Azure Trusted Signing profile
type so the **"Trusted Signing Smoke Test"** workflow runs GREEN on GitHub's clean
`windows-latest` runner — proving the publicly-trusted signing chain is live end-to-end
**before** any release is cut (Phase 104).

Deliverables (from SIGN-01/02/03):
- **SIGN-01 (operator-in-loop):** confirm `az trustedsigning certificate-profile show` →
  `profileType == PublicTrust`; if `PublicTrustTest` (the likely root cause of the
  `UnknownError` / `…Enterprise ID Verified Policy AOC CA…` issuer), operator creates a
  `PublicTrust` profile + updates `TRUSTED_SIGNING_PROFILE` and the FIC subject
  `repo:OscarMackJr/nono:environment:Development`. Finding (profile type + issuer chain)
  documented, not guessed.
- **SIGN-02 (autonomous):** extract a single shared `scripts/verify-authenticode.ps1`
  helper, dot-sourced by both fail-closed verify sites in `release.yml` and the smoke
  workflow; adds `signtool verify /pa /v` deep check, builds/repairs the cert chain,
  handles CRL/OCSP revocation transients, and classifies chain-build failure distinctly
  from a genuine untrusted root — **without ever loosening the fail-closed
  `Status -ne 'Valid'` contract.**
- **SIGN-03:** smoke workflow GREEN on `windows-latest` — a throwaway exe signs and
  verifies `Valid` with an issuer chaining to a public `Microsoft ID Verified CS EOC/AOC
  CA NN` root (not `PublicTrustTest`, not the POC root).

**NOT in this phase:** cutting the release (104), the package rename (102), Azure VM IaC /
clean-host gates (103), live publish (105). This phase only makes the verify path provably
correct and the smoke gate green.

</domain>

<decisions>
## Implementation Decisions

### signtool /pa role vs Get-AuthenticodeSignature (fail-closed contract)
- **D-01:** The helper runs **BOTH** `Get-AuthenticodeSignature` **AND** `signtool verify /pa /v`
  on every strict verify — a `Valid` verdict requires **both** to pass (AND gate). Two
  independent chain-build engines must agree. This is *additive corroboration*: the existing
  `Status -ne 'Valid'` fail-closed condition is unchanged; signtool is a second must-pass
  check layered on top, never a loosened/alternative pass path. Diff review must confirm the
  original condition is byte-for-byte preserved (SIGN-02 success criterion 3).

### CRL/OCSP revocation-check transient behavior
- **D-02:** On a **classified transient** (chain-build / revocation endpoint unreachable —
  the documented AOC/EOC CA rotation risk), the helper does **bounded retry-with-backoff**
  (sensible default ~3 attempts with increasing backoff — planner/researcher to set exact
  counts/timeouts), then **fail-closed** if still unresolved. Retries only ever *re-attempt*
  a verify; they never pass a non-`Valid` signature. A transient must never be silently
  swallowed — after exhausted retries it fails closed with the transient classification in
  the diagnostic.

### Helper interface / three-caller strictness (incl. .sys WHQL carve-out)
- **D-03:** One Verb-Noun function with an **explicit mode parameter**, e.g.
  `Assert-TrustedSignature -Path <file> -Mode Strict|Informational`.
  - `Strict` → full fail-closed AND-gate verify (loose `.exe` assets + MSI `.exe` payloads).
  - `Informational` → logs status **without gating**, preserving the existing
    `nono-wfp-driver.sys` WHQL/cross-sign carve-out (that driver returns `UnknownError` under
    Authenticode on CI and is a *separate signing regime* — gating it would break the release).
  - Mode is passed **explicitly by every call site**. **No extension-based auto-classification**
    — hiding the strict/informational (security-relevant) decision behind a filename check is a
    footgun per CLAUDE.md path-handling guidance.

### Failure diagnostics depth (root cause "documented, not guessed")
- **D-04:** On **any non-`Valid`** result, the helper dumps a **full chain introspection** to
  CI logs: every cert chain element (Subject / Issuer / Thumbprint), each element's
  chain-status flags, the verbatim `signtool /v` output, and which CRL/OCSP URLs were probed
  plus their reachability. Noise lands **only on the failure path** (happy path stays quiet).
  This must make the `PublicTrustTest`-issuer vs missing-intermediate vs revocation-transient
  distinction settleable from the CI log alone — directly serving SIGN-01's documented-not-guessed
  mandate.

### SIGN-01 operator flow + phase sequencing
- **D-05:** **Ship hardening first; operator profile-fix is a checkpoint.** The fork
  autonomously builds and lands the helper (SIGN-02), wires it into all three verify sites,
  and adds the diagnostics — independent of the Azure ops step. The operator's
  `az trustedsigning certificate-profile show` confirm/fix (SIGN-01) is a documented
  **checkpoint that must pass before SIGN-03 smoke can go green.** The fork does not block at
  phase start waiting on the Azure step; instead the hardened verify + diagnostics are what
  *let* the operator settle the root cause fast when they run the check.

### Claude's Discretion
- Exact retry count / backoff timings for D-02 (a small, documented default is fine).
- Precise PowerShell function name/signature and how dot-sourcing is wired at each of the
  three call sites, as long as D-01/D-03 semantics hold and no fail-closed condition is
  weakened.
- Whether to also add a small scripted `az` confirmation/record helper (an option the
  operator did not select but did not forbid) — optional nice-to-have, not required; the
  runbook step is sufficient for SIGN-01.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & roadmap (locked)
- `.planning/REQUIREMENTS.md` §SIGN-01/02/03 — the locked acceptance wording for this phase.
- `.planning/ROADMAP.md` → "Phase 101" Phase Details — goal + 4 success criteria (esp. criterion 3: fail-closed condition provably unweakened).

### Azure Trusted Signing go-live (root cause + operator runbook)
- `.planning/quick/260630-trusted-signing-golive/AZURE-TRUSTED-SIGNING-GOLIVE-COOKBOOK.md` — the go-live runbook; the `az trustedsigning certificate-profile show` step + FIC-subject correction live here. **Note:** the cookbook's informal "Test" profile = `PublicTrustTest` (the suspected root cause); accepted `--profile-type` values are `PrivateTrust, PrivateTrustCIPolicy, PublicTrust, PublicTrustTest, VBSEnclave`.
- Auto-memory `azure_trusted_signing_golive.md` — first smoke run `28467925298` evidence: Sign GREEN (signer `CN=TWGGLOBAL.onmicrosoft.com`) but Verify `UnknownError`, issuer `CN=Microsoft Enterprise ID Verified Policy AOC CA 01`; correct FIC subject `repo:OscarMackJr/nono:environment:Development`.

### Verify sites to refactor (single source of the two fail-closed checks)
- `.github/workflows/release.yml` ~line 259-273 — "Verify Authenticode signatures (Windows)" loop (loose `.exe` + broker + both MSIs), fail-closed on `Status -ne 'Valid'`. **Site 1.**
- `.github/workflows/release.yml` ~line 281-321 — "Verify MSI payload signatures (Windows)" (admin-extract `msiexec /a`, strict `.exe` payload gate, **informational `.sys` driver log** — the WHQL carve-out D-03 must preserve). **Site 2.**
- `.github/workflows/trusted-signing-smoke.yml` ~line 62-73 — "Verify the embedded signature is OUR Trusted Signing cert", fail-closed on `Status -ne 'Valid'`. **Smoke site.**

### Security guardrails (non-negotiable)
- `CLAUDE.md` → "Security Considerations" (Fail Secure; Path Handling footguns — string vs component comparison, extension-based classification). Reinforces D-03's no-auto-classify choice.
- `.planning/REQUIREMENTS.md` anti-goals table: "Loosening the fail-closed `-ne 'Valid'` verify to accept `UnknownError`" → explicitly forbidden.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- The two `release.yml` verify steps already implement the fail-closed `Status -ne 'Valid'`
  loop and the `.sys`-informational carve-out — the new helper should **preserve their exact
  gating semantics** while collapsing the duplicated logic into `scripts/verify-authenticode.ps1`.
- The smoke workflow's verify step is a near-duplicate of the release loop (minus MSI/broker) —
  it becomes the third dot-source call site.

### Established Patterns
- **Fail-closed / fail-secure is the house style** (CLAUDE.md): on any error, deny/abort;
  never silently degrade. The retry logic (D-02) must fit this — retries end in fail-closed,
  never in a pass.
- **`.sys` driver = separate signing regime.** `nono-wfp-driver.sys` is a checked-in
  WHQL/cross-signed copy; `Get-AuthenticodeSignature` returns `UnknownError` for it on CI by
  design. Any shared helper MUST route it through `-Mode Informational` (D-03), never strict.
- Prior dark-factory discipline: `verify-dark.ps1` gates emit machine-readable verdicts;
  Phase 103 will reuse this helper for a `trusted-signed-assertion` gate — design the helper
  so it's importable/dot-sourceable outside the CI YAML too.

### Integration Points
- `scripts/verify-authenticode.ps1` (new) dot-sourced by: `release.yml` (2 sites) +
  `trusted-signing-smoke.yml` (1 site). Phase 103 (`trusted-signed-assertion.ps1` gate) will
  also consume it — keep the interface caller-agnostic (no hard dependency on GitHub Actions
  env vars inside the core verify function).

</code_context>

<specifics>
## Specific Ideas

- The observed failure signature to disambiguate: `Get-AuthenticodeSignature` →
  `Status: UnknownError`, `Issuer: CN=Microsoft Enterprise ID Verified Policy AOC CA 01`.
  A public code-signing chain should read `Microsoft ID Verified **CS** EOC/AOC CA NN` — the
  `…Policy AOC CA…` naming is the `PublicTrustTest` tell. The full-chain dump (D-04) should
  make this distinction obvious in the log.
- `signtool verify /pa /v` (NOT `Get-AuthenticodeSignature`) is the recommended deep-check
  because it builds the AIA chain the way Windows does — key to distinguishing a *missing
  intermediate / revocation-transient* (`UnknownError`, chain couldn't be built) from a
  genuine `UntrustedRoot`.

</specifics>

<deferred>
## Deferred Ideas

- Retiring the POC signing path + deleting `trusted-signing-smoke.yml` + fixing the stale
  `docs/cli/development/windows-signing-guide.mdx` → **Phase 107 (Close-Out) / CLOSE-01**,
  gated on clean-host UAT PASS. Not this phase.
- Scripted `az` profile-confirmation helper (`scripts/azure/confirm-profile-type.ps1`) —
  optional; operator selected the runbook checkpoint. Can be added later if the manual step
  proves error-prone.
- Azure VM IaC + the `trusted-signed-assertion` / `broker-spawn-on-clean-host` gates that
  *reuse* this helper → **Phase 103**.

</deferred>

---

*Phase: 101-verify-gate-hardening-azure-profile-confirmation*
*Context gathered: 2026-07-02*
