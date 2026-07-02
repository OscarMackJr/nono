# Feature Research — Trusted Signing Go-Live + First Distributed Release (v3.5)

**Domain:** Windows Authenticode code-signing (Azure Trusted Signing / Artifact Signing), SmartScreen
reputation, clean-host software distribution, multi-registry package publishing (crates.io / PyPI / npm)
**Researched:** 2026-07-02
**Confidence:** HIGH (Azure Trusted Signing / Artifact Signing mechanics, cargo/PyPI/npm publish
semantics — verified against current Microsoft Learn docs + Microsoft Q&A + tool docs) /
MEDIUM (exact current chain-build failure cause on GitHub's `windows-latest` runner — inferred from
documented AOC/EOC CA rotation pattern, not confirmed against this specific run) / HIGH (nono's own
gate code — read directly)

**What the fork controls vs external:** every behavior below is tagged **[FORK]** (CI/gates/binaries
this milestone builds), **[AZURE]** (Azure Trusted Signing service behavior, outside fork control),
or **[HOST]** (depends on the state of the target Windows host, only provable by the Azure-VM clean
host).

---

## Feature Landscape

### Table Stakes (Users/Operators Expect These)

These are the non-negotiable, testable behaviors a "go-live" milestone must prove. Missing any one
means the release is not actually publicly trusted / not actually distributable.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Authenticode `Valid` on `nono.exe`/broker/WFP service/both MSIs, chain to Microsoft Trusted Root | This is the entire point of migrating off the POC cert — "Verified publisher" with zero manual trust steps | MEDIUM | **[AZURE]/[FORK]** — Azure issues+signs; fork's fail-closed verify step (`release.yml:259`, `Get-AuthenticodeSignature -ne 'Valid'`) gates the release on it. See "Behavior 1" below. |
| Certificate profile is **Public Trust**, not Test/Private/Enterprise | Only Public Trust chains to the Microsoft Root Certificate Program; Test/Private profiles are explicitly documented as "not publicly trusted" | LOW | **[AZURE]** operator-config, but **[FORK]**-testable: assert issuer CN matches `Microsoft ID Verified CS (EOC\|AOC) CA \d+`, not `Microsoft Enterprise ID Verified…` or `nono Test Signing`. |
| Chain-build succeeds (AIA fetch of intermediate) + revocation reachable, on the CI runner used for the fail-closed gate | `Valid` requires the full chain build to complete; a stale/incomplete local intermediate cache on a "clean" machine (incl. a fresh `windows-latest` GH runner) produces `UnknownError`, which is a chain-build/revocation failure, **not** proof of an untrusted root | HIGH | **[AZURE]/[HOST]** — see "Behavior 1" for the specific new-CA-rollout failure mode already observed on this project's first live run. |
| Broker self-trust gate (`verify_broker_authenticode`) passes with **zero** manual cert import on a clean host | This is the acceptance criterion of the `poc-cert-broker-clean-host` todo and the reason this milestone exists | LOW (code already ships) | **[FORK]** code, **[HOST]**-provable only. Gate requires BOTH binaries independently `AuthenticodeStatus::Valid` AND identical `signer_subject`+`thumbprint` (`exec_strategy_windows/launch.rs:2236-2296`). Uses `WTD_REVOKE_NONE` (no online revocation check at gate time — see Behavior 1 nuance). |
| MSI installs cleanly on a fresh Win11 host with no VC++ redist preinstalled | `+crt-static` already ships (Phase 80/INST-01) specifically to eliminate this dependency; this milestone's job is proof, not new code | LOW (code already ships) | **[FORK]** code (done), **[HOST]**-provable only via Azure VM. |
| Live publish to crates.io succeeds in strict dependency order (`nono` → `nono-proxy` → `nono-cli`) | `nono-proxy`/`nono-cli` `Cargo.toml` path-deps resolve against the crates.io index, not local paths, at publish-verify time — publishing out of order fails resolution (`PRE_PUBLISH_REGISTRY_BLOCKED`-equivalent) | LOW (mechanical) | **[FORK]** operator action. Already dry-run-proven in v3.4 (`release-dry-run.ps1`, `RELEASE-RUNBOOK.md` Step 4). Needs the ~30s post-publish indexing gap between crates. |
| Live publish to PyPI (`nono-py` wheel via maturin/twine) and npm (`nono-ts`) succeed | FUT-01 requirement — "prepare-only" becomes "actually live" | LOW (mechanical) | **[FORK]** operator action; `maturin publish` or `maturin build` + `twine upload`; `npm publish`. Both already dry-run PASS per v3.4. |
| Idempotent/resumable multi-registry publish on partial failure | A 3-registry, multi-artifact publish sequence WILL sometimes partially fail (network blip, registry indexing lag); operators must be able to re-run without manual "did X already publish?" archaeology | MEDIUM | **[FORK]** — needs an explicit resume protocol; none of the 3 registries is naturally idempotent the same way, each needs a different resume strategy (see "Behavior 4"). |
| No POC secrets/paths reachable after go-live | Prevents any code path (accidental or malicious) from ever falling back to a self-signed cert | LOW | **[FORK]** close-out step, already scoped in the cookbook §8 and `poc-cert-broker-clean-host.md`. |

### Differentiators (What Sets This Milestone's Verification Apart)

Not required by Azure/registries themselves, but valuable given this project's Dark Factory /
scripted-gate posture and its history of "verify_gate says PASS but the underlying capability was
never actually exercised" bugs (e.g., the v3.2 `verify_override_production` gap, the v3.4
release-readiness false-positive twine detection).

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| `UnknownError`-vs-`UntrustedRoot` diagnostic distinction in the CI verify step | Prevents the team from mis-diagnosing a transient chain-build/revocation failure as "the signing setup is broken" (already nearly happened on 2026-06-30) and burning cycles re-doing Azure config that was already correct | MEDIUM | **[FORK]**. `Get-AuthenticodeSignature` alone collapses many distinct failure causes into one `Status` enum value; add a `signtool verify /pa /v` fallback + parse for `CERT_E_UNTRUSTEDROOT` (0x800B0109, genuine untrusted root) vs chain-build/revocation timeout signals vs profile-type mismatch (wrong issuer CN pattern). |
| Scripted, repeatable Azure-VM clean-host IaC (fresh Win11, `az`/Bicep) | Every prior "clean host" UAT in this project's history (v2.11, v2.13, v3.0, v3.1 Phase 90, Phase 100) has been `SKIP_HOST_UNAVAILABLE` or a manual one-off; a reproducible IaC template turns clean-host UAT from a one-time event into a re-runnable gate for every future release | HIGH | **[FORK]**. Directly closes the `clean-host-install` `SKIP_HOST_UNAVAILABLE` verdict from Phase 100 (`.nono-runtime/verdicts/clean-host-install.json`). |
| Machine-readable verdicts for all 3 Gate-3 UAT assertions (trusted-signed, broker-spawn, MSI-clean-install) via `verify-dark.ps1` | Keeps this milestone consistent with the project's established Dark Factory pattern rather than relying on a human screenshot | LOW (pattern reused) | **[FORK]**, reuses `Test-Precondition`/`Invoke-Gate` contract from Phase 76. |
| Pre-flight "what's already published" check before any live registry push | Directly prevents the partial-publish confusion described in Behavior 4; a one-shot check that queries crates.io index + PyPI JSON API + npm registry for `0.66.1` before running any publish command | MEDIUM | **[FORK]**, new — no equivalent exists in `release-dry-run.ps1` today (that script only dry-runs, it does not query live registry state). |

### Anti-Features (Tempting But Wrong For This Milestone)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|------------------|-------------|
| Retry-publish-everything-from-scratch on partial failure | Feels simplest — "just re-run the whole sequence" | crates.io/PyPI/npm are NOT idempotent on re-publish of an already-uploaded artifact: `cargo publish` of an existing version fails ("crate version X.Y.Z already exists"); `twine upload` fails 400 "File already exists" (immutable releases) unless `--skip-existing`; `npm publish` of an existing version fails 403/`EPUBLISHCONFLICT` (immutable). A naive "just re-run" script will error out on step 1 of the *second* attempt even though steps 2-3 still need to run. | Registry-aware resume: check what's live first, skip already-published legs, continue with the next un-published one (see Behavior 4). |
| EV/OV cert or requesting Extended Validation from Azure Trusted Signing | "EV certs skip SmartScreen faster" is common lore | Artifact Signing explicitly **does not issue EV certificates and has no plan to** (confirmed in the official FAQ) — this isn't an available lever regardless of preference | Accept that Public Trust (non-EV) reputation builds via file-hash download volume over time; document it, don't chase it. |
| Trying to force/pre-seed SmartScreen reputation via a support ticket before go-live | Feels proactive — "let's not have a scary warning on day one" | Microsoft's own guidance is that reputation "builds up automatically" from download history; submitting to Microsoft Security Intelligence is a *last resort* for a specific persistent false-positive, not a pre-emptive step, and won't complete before this milestone's timeline | Document the expected "Windows protected your PC" first-impression cost explicitly (Behavior 2) as an accepted, time-bound cost — not a blocker to resolve. |
| Manually importing a cert or pre-trusting *anything* on the Azure clean-host VM before running Gate 3 | Faster path to a green UAT run | Defeats the entire purpose of the clean-host test — the acceptance criterion is explicitly **zero manual trust steps** | The VM must be provisioned once, tested, and (ideally) discarded/re-imaged for the next release's UAT, not "prepared." |
| Skipping the `--skip-existing`/registry-check logic and instead hand-tracking "what did I already publish" in a chat/notepad | Seems fine for a one-person operator flow | This project's own history shows manual state-tracking across a multi-step operator runbook is exactly where drift creeps in (see multiple `feedback_stale_todos_verify_against_code` / `feedback_sdk_*` memory entries) | Scripted registry-state pre-check + `--skip-existing` (PyPI) / dependency-order skip-if-live (crates.io) / version-bump-required halt (npm) baked into the runbook, not tribal memory. |

---

## Behavior 1 — Publicly-Trusted Authenticode Signature: What "Valid" Requires

**Chain structure (Public Trust, verified against Microsoft Learn `azure/artifact-signing/concept-trust-models`, current as of 2026-01-06):**

```
Microsoft Identity Verification Root Certificate Authority 2020   (root — in Microsoft Root Cert Program)
  └── Microsoft ID Verified Code Signing PCA 2021                 (policy CA)
        └── Microsoft ID Verified CS EOC CA NN  |  Microsoft ID Verified CS AOC CA NN   (issuing intermediate; NN rotates — AOC CA 03 / EOC CA 04 as of 2026-03-26)
              └── <leaf cert, CN = validated legal entity name — no custom CN/O allowed>
```

- **EOC** vs **AOC** are both valid Public Trust issuing-intermediate families; the exact NN suffix
  rotates over time as Microsoft rolls new intermediates. A correct Public Trust chain's issuer CN
  will always read `Microsoft ID Verified CS EOC CA <N>` or `Microsoft ID Verified CS AOC CA <N>` —
  note the **`CS`** (Code Signing) token specifically.
- **A Test certificate profile is explicitly documented as "not publicly trusted"** even though it
  lives under the "Public Trust" *collection* in the portal — this is the most likely single
  misconfiguration to check first if verify keeps failing.
- **Private Trust** (a wholly separate trust model, for App Control/WDAC) uses an entirely different,
  non-default-trusted CA hierarchy — never the right choice for a public release binary.

**This project's own field note (2026-06-30, run `28467925298`) reported the issuer as `CN=Microsoft
Enterprise ID Verified Policy AOC CA 01`.** This string does **not** match the documented Public Trust
naming pattern (`Microsoft ID Verified CS EOC/AOC CA NN`) — the presence of `Enterprise ID Verified
Policy` rather than `ID Verified CS` is itself evidence worth re-checking against the actual configured
certificate-profile *type* in Azure, independent of the `UnknownError` chain-build symptom. **Confidence:
MEDIUM** — could not find public documentation of an "Enterprise ID Verified Policy" issuer name
specifically; it does not match any Public Trust or Private Trust naming pattern found in official docs,
which is itself the actionable signal (verify the profile's actual `--profile-type` in Azure, don't
assume the cookbook's own guess is the root cause).

**`UnknownError` ≠ `UntrustedRoot` — confirmed as a distinct, documented failure class:**

- `Get-AuthenticodeSignature` collapses many chain-verification outcomes into one enum; `UnknownError`
  specifically indicates the trust provider could not complete chain validation, not that it completed
  and found an untrusted root (that would surface as a different status / `CERT_E_UNTRUSTEDROOT`,
  `0x800B0109`).
- Two independently plausible root causes fit the symptom, per Microsoft's own guidance and observed
  community reports:
  1. **Revocation-check failure** — with online revocation mode, CryptoAPI must reach CDP/AIA-referenced
     CRL/OCSP endpoints for every cert in the chain; if any is unreachable or stale, chain build reports
     failure (`RevocationOffline`/similar), which downstream tooling can surface as `UnknownError`.
  2. **New intermediate CA not yet locally cached/trusted** — Microsoft Q&A (`5855442`, 2026) documents
     exactly this pattern for the March 2026 AOC CA 03 / EOC CA 04 rollout: apps signed by the new
     intermediates were flagged by consumers whose local machine/AIA cache hadn't yet picked up the new
     CA, even with a valid, unchanged Public Trust profile. Microsoft's own remediation was time-based
     (cache/reputation propagation), not a config change on the signer's side.
- **Practical implication for the fork's verify-gate hardening:** `signtool verify /pa /v <file>`
  (documented by Microsoft as the canonical deep-diagnostic command, distinct from
  `Get-AuthenticodeSignature`) plus explicit AIA-chain-build + revocation-mode logging will distinguish
  "genuinely untrusted/misconfigured" from "transient chain-build/revocation timeout on this specific
  runner" — the two need different remediations (fix the profile vs. retry/allow grace).

**Why the broker self-trust gate then passes on a clean host (once `Valid`):** the gate
(`crates/nono-cli/src/exec_strategy_windows/launch.rs:2236`, code confirmed) requires (a) `nono.exe`
Authenticode status `Valid`, (b) `nono-shell-broker.exe` Authenticode status `Valid`, AND (c) identical
`signer_subject` + `thumbprint` between the two. `release.yml` already signs all top-level `.exe`
(`nono.exe`, `nono-shell-broker.exe`, `nono-wfp-service.exe`) with the **same** Trusted Signing profile
in one step (`files-folder-filter: exe`, no recurse), so (c) is structurally satisfied whenever (a)/(b)
hold. The gate itself uses `WTD_REVOKE_NONE` (no online revocation check at gate-evaluation time) — it
only needs the chain to build to a trusted root, which for Public Trust means the Microsoft Root Program
root cert being present in the host's trust store. Since the July 2021 Windows CTL update
(KB5022661), that root is fetched automatically by Windows the first time it's encountered — **no
manual `LocalMachine\Root` import needed**, unlike the old self-signed POC cert. **Confidence: HIGH**
(gate code read directly; CTL auto-fetch documented in the official FAQ).

---

## Behavior 2 — First-Run SmartScreen Reputation for a Brand-New Signing Identity

**Expected, testable behavior:** the first time a fresh Public Trust signing identity's binaries hit a
consumer Windows machine, SmartScreen **can** show "Windows protected your PC" / "unrecognized
publisher," *even though the Authenticode signature itself is fully `Valid`*. Validity and reputation
are separate axes.

- Per Microsoft's own FAQ (`azure/artifact-signing/faq`): "**SmartScreen reputation builds up
  automatically. The prompt stops appearing once the file hash has sufficient download history.**" This
  is volume/time-based, not something the signing profile itself confers on day one.
- Confirmed pattern (Microsoft Q&A `5863283`, `5855442`, 2026): reputation does **not** automatically
  carry over when Microsoft rotates the intermediate CA (e.g., the March 2026 AOC CA 03/EOC CA 04
  rotation caused a *fresh* reputation-cold-start for files signed by the new intermediates, even for
  already-established publishers) — so a brand-new nono signing identity is doubly cold-start: new
  publisher AND (if Azure has rotated intermediates again by go-live) potentially a new-intermediate
  reset too.
- Last-resort mitigation if prompts persist unexpectedly long: submit the signed file to Microsoft
  Security Intelligence for review (`https://www.microsoft.com/wdsi`) — explicitly documented as a
  fallback, not a pre-emptive or fast-path step.
- **No EV path exists to bypass this** — Artifact Signing does not issue EV certificates (confirmed,
  official FAQ) and has no plan to.

**Testable acceptance framing for this milestone:** do NOT gate release success on "no SmartScreen
prompt" — that is an accepted, time-bound cost outside the fork's control. DO gate on: Authenticode
`Valid` + correct Public Trust issuer chain (Behavior 1), independent of whether SmartScreen still shows
a first-run prompt. Document the expected prompt explicitly in the cookbook/close-out docs so an
operator doesn't mistake it for a signing failure. **Confidence: HIGH** (directly sourced from official
FAQ + the cookbook's own troubleshooting table already anticipates this exact outcome).

---

## Behavior 3 — Clean-Host UAT Expectations

Two independent claims, both already coded, both requiring **[HOST]** (Azure VM) proof, not more code:

1. **MSI installs with no VC++ redist.** `+crt-static` (Phase 80, commit `a517284b`) statically links the
   CRT into `nono.exe`/`nono-wfp-service.exe` for the `x86_64-pc-windows-msvc` target — confirmed via
   binary-import inspection (no `vcruntime140.dll` import). The machine MSI's `<ServiceInstall>` uses
   `Vital="no"`/`ErrorControl="ignore"` so even a service-start hiccup doesn't roll back the whole
   product. **Expected correct behavior on a genuinely clean Win11 host:** MSI install completes with
   no `1603` and no rollback, `nono.exe` launches without `0xC0000135`
   (`STATUS_DLL_NOT_FOUND`) — with **zero** `vc_redist.x64.exe` step. This was previously blocked from
   verification purely by host availability (`clean-host-install` gate returned
   `SKIP_HOST_UNAVAILABLE`/exit 3 at Phase 100 because the dev host test requires elevation the runner
   didn't have) — the Azure VM removes that blocker by providing an actual disposable elevated clean
   host.
2. **Broker spawns with no manual cert import.** Covered fully in Behavior 1's last paragraph —
   contingent entirely on the Trusted Signing chain resolving to `Valid`/trusted-root first. This is the
   `poc-cert-broker-clean-host` todo's literal acceptance criterion:
   `nono run --profile claude-code -- <cmd>` spawns the broker on a fresh Win11 host with **zero**
   manual trust steps.

**Why the Azure VM specifically (not the operator's corporate dev host):** the corporate host already
has the POC cert previously imported, has VC++ preinstalled, and sits behind corporate
proxy/EDR/managed-trust-store — any of these three confounds the exact signal this milestone needs to
prove (a genuinely virgin trust state + genuinely clean CRT dependency + unfiltered AIA/CRL/OCSP
egress). This constraint is already captured correctly in `PROJECT.md`'s v3.5 "Key context" section;
this research confirms it against the mechanics above rather than just restating it.

**Confidence: HIGH** for the code-side claims (read directly / already verified in Phase 80/100);
**untestable without the VM** for the host-side claims by design — this is exactly what Gate 3 exists to
close.

---

## Behavior 4 — Live Multi-Registry Publish Mechanics

### crates.io — dependency-order, per-artifact-atomic, workspace-sequence-fragile

- **Dependency order is load-bearing, not cosmetic.** `nono-proxy` and `nono-cli` both path-depend on
  `nono`; crates.io resolves and verifies against the **published index**, not local paths, at
  publish-verify time. Publishing `nono-proxy` before `nono` is live fails resolution — this project's
  own dry-run tooling already models this exact failure as `PRE_PUBLISH_REGISTRY_BLOCKED`.
  Confirmed publish order: **`nono` → `nono-proxy` → `nono-cli`** (matches `RELEASE-RUNBOOK.md` Step 4
  and the workspace's actual dependency graph — `nono-shell-broker`/`nono-ffi`/`nono-fltmgr-client` are
  `publish = false`).
- **Indexing lag requires a gap between publishes.** crates.io's index needs a short window (the
  runbook uses `sleep 30`) to reflect a just-published crate before the next dependent crate's
  `cargo publish` can resolve it. This is a documented community pattern (per `cargo-publish-workspace`
  tooling, which built the same sleep-between-publishes behavior for exactly this reason), not
  nono-specific.
- **Per-crate publish is atomic, but the *sequence* is not.** A single `cargo publish -p X` either fully
  succeeds or fully fails (no partial-file state on crates.io per crate) — but across 3 sequential
  publishes, a mid-sequence failure (e.g., `nono` succeeds, `nono-proxy` publish then fails on a
  transient network error) leaves the workspace in a partially-published state.
- **Re-publish is NOT idempotent — cargo itself has no `--idempotent`/`--skip-existing` flag** (this is
  an open, unresolved cargo feature request, `rust-lang/cargo#13397`, confirmed still open). Re-running
  `cargo publish -p nono` after it already succeeded errors ("crate version already exists" /
  equivalent 4xx from crates.io). **Correct resume protocol:** before any retry, check which of the 3
  crates are actually live at `0.66.1` (crates.io API/index query), then resume the sequence starting
  from the first crate NOT yet published — do not blindly re-run the full 3-crate loop.

### PyPI (`nono-py` via maturin/twine) — file-level immutable, has a built-in resume flag

- `maturin publish` (single-step) or `maturin build` + `twine upload` (two-step) both push immutable,
  versioned wheel files.
- **PyPI enforces per-file immutability**: uploading a filename that already exists for that release
  fails with HTTP 400 "File already exists" — confirmed current behavior, and explicitly NOT a
  transient/retryable error; PyPI's own guidance is "create a new release" for actual content fixes,
  not re-upload.
- **`twine upload --skip-existing`** is the documented, purpose-built resume mechanism: it detects
  already-uploaded files and skips them, uploading only what's still missing — the correct way to
  resume a partially-completed multi-wheel PyPI publish (e.g., if this project publishes wheels for
  multiple platform tags) without manual bookkeeping.
- `maturin publish` (single combined command) does not expose the same `--skip-existing`
  granularity directly per Twine's docs — the safer resumable path for THIS project, given it explicitly
  documents the two-step fallback already (`RELEASE-RUNBOOK.md` Step 5: `maturin build --release` +
  `twine upload target/wheels/*.whl`), is to always use the two-step form with `--skip-existing` so a
  retry after partial failure is safe by construction.

### npm (`nono-ts`) — single-tarball, fully atomic per version, NOT resumable by retry

- `npm publish` uploads one tarball per version; there's no PyPI-style multi-file-per-release state to
  partially fail. It either fully succeeds or fully fails.
- **Publishing over an existing version fails** (403/`EPUBLISHCONFLICT`, "You cannot publish over the
  previously published version") — immutable per version, same posture as PyPI, but with no
  `--skip-existing` equivalent because there's nothing to skip: it's one atomic operation.
- **Correct resume protocol:** if `npm publish` reports "already exists" on retry, that IS confirmation
  the earlier attempt actually succeeded (npm doesn't leave visible partial state) — the correct action
  is "move on," not "investigate a failure." The existing runbook's `npm publish --dry-run` pre-check
  (Step 6) is the right pre-flight habit; extending it to check the live registry (not just dry-run
  manifest shape) closes the remaining gap.

### What "idempotent / resumable publish" concretely means for THIS milestone

Given the three registries have three *different* resume semantics (crates.io: no native skip, must
query-then-resume-from-N; PyPI: native `--skip-existing`; npm: atomic, "already exists" on retry = prior
success, not failure), a single uniform retry loop is wrong. **The differentiator feature ("pre-flight
'what's already published' check," above) is the correct fix**: before executing (or re-executing) the
live push sequence, query all three registries' actual current state for `0.66.1`
(`cargo search`/crates.io index API, `https://pypi.org/pypi/<pkg>/<version>/json`, `npm view <pkg>@<version>`),
and have the runbook/script skip legs that are already live rather than re-attempting them blind.
**Confidence: HIGH** for crates.io/PyPI/npm mechanics (verified against `rust-lang/cargo` issue tracker,
Cargo Book, `pypa/twine` docs/issues, and general npm publish semantics which are stable, long-documented
registry behavior); MEDIUM on the exact recommended query commands (not independently re-verified against
current live API responses in this research pass).

---

## Feature Dependencies

```
Verify-gate hardening (signtool /pa fallback + AIA/revocation diagnostics)  [FORK]
    └──requires──> Azure profile confirmed Public Trust (correct issuer CN)  [AZURE, operator-gated]
                       └──requires──> Identity validation Approved (already done, 2026-06-30)

Cut trusted-signed 0.66.1 release (Gate 2)
    └──requires──> Verify-gate hardening GREEN on GitHub windows-latest  [FORK]
    └──requires──> Azure profile + FIC subject + role assignment correct  [AZURE, operator-gated]

FUT-01 live multi-registry publish
    └──requires──> Cut trusted-signed 0.66.1 release (binaries/artifacts must exist first)
    └──enhanced-by──> Pre-flight "what's already published" check (safe to retry)

FUT-03 clean-host UAT (Gate 3: broker-spawn-no-cert-import, MSI-clean-install)
    └──requires──> Cut trusted-signed 0.66.1 release (needs the actual signed MSI artifact)
    └──requires──> Azure VM clean-host IaC (fresh Win11, no POC cert, no VC++)  [FORK, new]
    └──requires──> Behavior 1 chain resolves to Valid (broker gate is Behavior-1-downstream)

Close-out (retire POC secrets, delete smoke workflow, fix stale docs)
    └──requires──> Gate 3 PASS (only retire the fallback once the real path is proven, not just green-CI)

SmartScreen first-run prompt (Behavior 2)
    └──conflicts-with──> "no manual/no visible friction" framing — must be documented as an ACCEPTED
                          cost, not gated on / not blocking Gate 2 or Gate 3 pass criteria
```

### Dependency Notes

- **Verify-gate hardening must land before Gate 2's tag push is meaningful** — pushing `v0.66.1` against
  an unfixed `UnknownError` verify step just reproduces the 2026-06-30 failure (fail-closed abort,
  nothing published). This is why `release.yml:259`'s fail-closed check is a hard prerequisite, not
  optional polish.
- **Gate 3 (clean-host UAT) structurally cannot run before Gate 2 succeeds** — it needs the actual
  trusted-signed MSI artifact from a real release, not a smoke-test throwaway exe.
- **SmartScreen reputation (Behavior 2) is explicitly NOT a dependency of any gate** — conflating "chain
  is Valid" with "no first-run warning" would either block an otherwise-correct release indefinitely
  (reputation takes real-world time/volume this project cannot manufacture) or tempt an anti-feature
  (chasing EV/pre-seeding, both unavailable/ineffective per Behavior 2's sourcing).
- **The pre-flight registry-state check enhances but does not gate FUT-01** — publish can proceed
  without it (as v3.4 already scoped, manual runbook), but its absence is exactly what turns a partial
  failure into operator confusion, per Behavior 4.

---

## MVP Definition (for THIS milestone's scope)

### Launch With (v3.5 core)

- [ ] Verify-gate hardening: `signtool verify /pa /v` fallback + chain-build/revocation diagnostics
      distinguishing `UnknownError`-transient from genuine untrusted-root, on GitHub `windows-latest` —
      essential because the current fail-closed check cannot tell these apart and already blocked a
      real release attempt.
- [ ] Azure Public Trust profile confirmed correct (issuer CN pattern check) — essential precondition,
      cheapest possible check, directly targets the one open field-note anomaly.
- [ ] Cut the trusted-signed `0.66.1` release (Gate 2) — the literal deliverable.
- [ ] FUT-01 live publish to all 3 registries, in dependency order, with a pre-flight registry-state
      check for safe resume — essential because this milestone's stated goal is "genuinely distributed,"
      not "prepared."
- [ ] Azure VM clean-host IaC + Gate 3 scripted verdicts (trusted-signed, broker-spawn, MSI-install) —
      essential, this is the actual acceptance test both host-gated todos have been waiting years for.

### Add After Validation (v3.5.x / same milestone, later phase)

- [ ] Close-out: retire POC secrets, delete `trusted-signing-smoke.yml`, fix
      `windows-signing-guide.mdx` — correctly sequenced AFTER Gate 3 passes (don't remove the fallback
      before the real path is proven).
- [ ] Move both host-gated todos `pending/` → `resolved/`.

### Future Consideration (beyond v3.5)

- [ ] SmartScreen reputation monitoring/tracking over time — not blocking, no lever to pull faster; note
      it and move on.
- [ ] Re-runnable Azure-VM IaC as a *standing* per-release gate (not just this milestone's one-time
      proof) — valuable, but the immediate ask is proving it once; making it a permanent recurring CI
      leg is a natural v3.6+ candidate (ties to FUT-04 native cross-target clippy hosted-CI gate already
      flagged as a next-milestone candidate in `PROJECT.md`).

---

## Feature Prioritization Matrix

| Feature | User/Operator Value | Implementation Cost | Priority |
|---------|----------------------|----------------------|----------|
| Verify-gate `UnknownError` diagnostic hardening | HIGH (unblocks everything downstream) | MEDIUM | P1 |
| Confirm/fix Azure Public Trust profile config | HIGH | LOW (once operator has RBAC access) | P1 |
| Cut trusted-signed 0.66.1 release | HIGH (the milestone's core deliverable) | LOW (pipeline pre-built) | P1 |
| Live crates.io/PyPI/npm publish with dependency order | HIGH (FUT-01) | LOW (mechanical, dry-run-proven) | P1 |
| Pre-flight "what's already published" resume check | MEDIUM (prevents operator confusion on partial failure) | MEDIUM | P2 |
| Azure VM clean-host IaC | HIGH (closes 2 long-standing host-gated todos) | HIGH (net-new IaC) | P1 |
| Gate 3 scripted verdicts (verify-dark.ps1 additions) | HIGH (Dark Factory consistency) | MEDIUM (pattern reused) | P1 |
| SmartScreen reputation documentation (accept, don't chase) | LOW-MEDIUM (prevents future mis-diagnosis) | LOW | P2 |
| Close-out (retire POC secrets, delete smoke workflow, fix docs) | MEDIUM (hygiene/security posture) | LOW | P2 |

**Priority key:** P1 = must have for this milestone's goal; P2 = should have, sequenced after P1 gates
are green; P3 = nice to have, future consideration (none identified as P3 in this scope).

---

## Sources

- [Azure Artifact Signing (Trusted Signing) trust models — Public Trust / Private Trust / Test](https://learn.microsoft.com/en-us/azure/artifact-signing/concept-trust-models) — HIGH confidence, official, current (2026-01-06)
- [Azure Artifact Signing FAQ — SmartScreen reputation, error codes, VC++ Redistributable requirement for the *signing client*](https://learn.microsoft.com/en-us/azure/artifact-signing/faq) — HIGH confidence, official, updated 2026-06-22
- [Trusted Signing: new intermediate CAs (AOC CA 03/EOC CA 04) causing SmartScreen warnings — Microsoft Q&A](https://learn.microsoft.com/en-ca/answers/questions/5855442/azure-trusted-signing-new-intermediate-cas-causing) — MEDIUM confidence (community + MS employee reply, not a formal doc)
- [Trusted Signing profile pinned to AOC CA 03 — SmartScreen warnings — Microsoft Q&A](https://learn.microsoft.com/en-ca/answers/questions/5863283/trusted-signing-profile-makologics-pinned-to-aoc-c) — MEDIUM confidence
- [Root certificate trust problem after creating a Trusted Signing profile — Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/2140998/root-certificate-trust-problem-after-creating-an-a) — MEDIUM confidence
- [X509Chain CRL online check — chain build / revocation mechanics](https://learn.microsoft.com/en-us/answers/questions/362684/x509chain-crl-online-check-in-azure-functions) — MEDIUM confidence, general .NET/CryptoAPI chain-build behavior, not Trusted-Signing-specific but directly applicable
- [`cargo publish` — The Cargo Book](https://doc.rust-lang.org/cargo/commands/cargo-publish.html) / [Publishing on crates.io — The Cargo Book](https://doc.rust-lang.org/cargo/reference/publishing.html) — HIGH confidence, official
- [`rust-lang/cargo#13397` — "Want cargo publish --idempotent" (open, unresolved)](https://github.com/rust-lang/cargo/issues/13397) — HIGH confidence, confirms no native idempotent-publish exists
- [`rust-lang/cargo#9507` — publish multiple crates, races with dependencies](https://github.com/rust-lang/cargo/issues/9507) — HIGH confidence
- [twine — `--skip-existing` and PyPI immutable-release behavior, twine docs](https://twine.readthedocs.io/) — HIGH confidence, official
- [`pypa/twine#199` — HTTP 400 "file already exists" after retry due to HTTP 500](https://github.com/pypa/twine/issues/199) — HIGH confidence, direct evidence of the partial-failure-then-retry failure mode this milestone must handle
- [`pypa/twine#63` — "Do something more intelligent when the file already exists on pypi"](https://github.com/pypa/twine/issues/63) — MEDIUM confidence, historical context for `--skip-existing`'s existence
- Internal: `.planning/quick/260630-trusted-signing-golive/AZURE-TRUSTED-SIGNING-GOLIVE-COOKBOOK.md` (this project's own field note, run `28467925298`, 2026-06-30) — HIGH confidence, primary source for the exact observed `UnknownError`/issuer-CN anomaly
- Internal: `crates/nono-cli/src/exec_strategy_windows/launch.rs:2236-2296` (`verify_broker_authenticode`) — HIGH confidence, read directly
- Internal: `.planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md`, `scripts/release-dry-run.ps1` — HIGH confidence, read directly, defines the already-proven dependency order and dry-run behavior this milestone makes live
- Internal: `.planning/todos/pending/20260611-poc-cert-broker-clean-host.md`, `.planning/todos/pending/20260611-msi-vcredist-prereq.md` — HIGH confidence, defines the exact acceptance criteria this milestone must satisfy

---
*Feature research for: nono v3.5 — Trusted Signing Go-Live + First Distributed Release*
*Researched: 2026-07-02*
