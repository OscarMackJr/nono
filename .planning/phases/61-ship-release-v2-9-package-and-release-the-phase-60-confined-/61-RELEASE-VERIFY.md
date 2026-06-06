---
status: shipped
phase: 61-ship-release-v2-9-package-and-release-the-phase-60-confined
plan: 04
created: 2026-06-03
resolved: 2026-06-06
release_status: SHIPPED
release_tag: v0.62.2
release_url: https://github.com/OscarMackJr/nono/releases/tag/v0.62.2
---

# 61-RELEASE-VERIFY — v2.9 / v0.62.2 Release Outcome

> ## ✅ RESOLUTION (2026-06-06) — SHIPPED as v0.62.2
>
> **The release is published: https://github.com/OscarMackJr/nono/releases/tag/v0.62.2**
> (Latest; signed machine + user MSIs, all `Sign`/`Verify Authenticode`/`Verify MSI payload`
> steps green; release notes published; `release.yml` run `27074741774` — all 5 build legs
> success + `Create Release` success; only the cosmetic crates.io/Homebrew jobs "failed").
>
> **The original BLOCKED diagnosis below (D-02 signing cert) was superseded.** After it was
> written, signing was wired up (Azure Trusted Signing, 2026-06-04) and the `v0.62.0` tag was
> cut on 2026-06-05 — but `release.yml` still failed to **publish**, for a *different* reason
> than the cert: **two latent cross-target compile errors** in `cfg`-gated code the Windows
> dev host never compiles, which broke all four Linux/macOS build legs and gated `Create
> Release`:
> 1. `E0716` (temporary dropped while borrowed) in `claude_code_hook.rs` `wrapped_bash_command`
>    (`cfg(not(windows))`, Phase 60) — fixed in `4de294e8`.
> 2. `error: let chains are only allowed in Rust 2024 or later` in `hook_runtime.rs`
>    `EnvFileGuard::drop` (`cfg(unix)`, Phase 58) — fixed in `7bb7c7e3` (masked behind #1, so it
>    only surfaced on the `v0.62.1` attempt).
>
> Windows always built + signed cleanly, so the cert was **not** the publish blocker. Tag
> history: `v0.62.0` (compile-fail, unpublished) → `v0.62.1` (E0716 fixed, let-chain surfaced,
> unpublished) → **`v0.62.2` (both fixed, PUBLISHED)**. The historical BLOCKED analysis is
> retained verbatim below for the record.

---

# (HISTORICAL — superseded by the resolution above) v2.9 / v0.58.0 Release Outcome

**FINAL RELEASE STATUS (historical, 2026-06-03): 🔴 BLOCKED — not shipped.**

The release was stopped at the Plan 61-04 Task 2 human-verify checkpoint by the **D-02
signing-secret gate**. Nothing was published; no tag was pushed; `release.yml` never ran. The
state is fully recoverable — re-run the tag step once production signing material is configured.

## Blocker (D-02) — the reason the release did not ship

| Item | Result |
|------|--------|
| Operator confirmation of `WINDOWS_SIGNING_CERT` / `WINDOWS_SIGNING_CERT_PASSWORD` **values** | **POC cert / expired — NOT production material** |
| Decision | **BLOCK.** Per the plan (T-61-09) a public release MUST NOT be signed with the self-signed POC cert (`319E507E…`), and the CI fail-closed guard would otherwise sign with whatever is present. |
| Action taken | Release deliberately NOT triggered. `v0.58.0` tag NOT created, `main` NOT pushed, `release.yml` NOT run. |

The secret **names** are present (`gh secret list` shows both, dated 2026-05-28, per
61-PRETAG-READINESS.md), but the GitHub CLI cannot read secret **values** — only the operator
can. The operator confirmed the contents are the POC cert / expired, which is the D-02 release
blocker the pre-tag readiness gate was designed to surface.

## Release pipeline state (all NOT done — blocked upstream of these)

| Step | Status |
|------|--------|
| `git push origin main` (101 unpushed commits) | NOT done |
| `v0.58.0` build-trigger tag | NOT created / NOT pushed |
| `release.yml` run (release job + `Verify MSI payload signatures`) | NEVER triggered |
| `v2.9` milestone annotation tag | NOT created / NOT pushed |
| Post-release MSI signature spot-check (wrapper + payload Authenticode) | N/A (nothing published) |
| Publish 61-RELEASE-NOTES.md to the GitHub release | N/A (no release) |
| Latest GitHub release | still **v0.57.5** (unchanged) |

## Additional findings surfaced at the checkpoint (must be fixed before re-attempting)

1. **GPG vs DCO tag-signing mismatch.** The plan's operator instructions used `git tag -s`
   (GPG cryptographic signing). This repo has **no GPG signing key** (`git config user.signingkey`
   empty; `tag.gpgSign` unset), so `-s` fails: `gpg: signing failed: No secret key`. This
   project's signing requirement is the **DCO `Signed-off-by:` text trailer**, NOT GPG. The
   correct command is an **annotated** tag:
   ```
   git tag -a v0.58.0 -m "v2.9 — Windows confined coding loop + out-of-box WFP enforcement"
   ```
   (release.yml triggers on the `v*.*.*` tag glob regardless of GPG signing.)

2. **`main` is 101 commits ahead of `origin/main`.** Remote `main` is at `03c7b39a`
   (≈ v0.57.5 era); the entire Phase 60 / 62 / 61 history — including the 0.58.0 lockstep bump —
   is unpushed. The tagged commit's code must be on the remote. `git push origin main` is required
   before (or together with) the tag push.

## Remediation — exact sequence to ship once production signing material is in place

```
# 0. Configure PRODUCTION Authenticode signing material in repo secrets
#    WINDOWS_SIGNING_CERT (base64 PFX) + WINDOWS_SIGNING_CERT_PASSWORD
#    — NOT the self-signed POC cert 319E507E…

cd C:\Users\OMack\Nono                # confirm `git branch` = main, tip = the 0.58.0 commit
git push origin main                  # push the 101 commits (carries the 0.58.0 bump)

git tag -a v0.58.0 -m "v2.9 — Windows confined coding loop + out-of-box WFP enforcement"
git push origin v0.58.0               # triggers release.yml

gh run watch                          # gate on the `release` job + `Verify MSI payload signatures`
                                      # (crates.io HTTP 303 + homebrew-bump "failures" are cosmetic,
                                      #  non-blocking on the fork — do NOT treat as release failure)

# after the release job publishes:
git tag -a v2.9 -m "v2.9 milestone — confined coding loop + WFP enforcement"
git push origin v2.9                  # 2-component tag → no build, by design

# post-release MSI signature spot-check (Windows host):
#   Get-AuthenticodeSignature <machine.msi>, <user.msi>  → Status = Valid
#   msiexec /a <msi> /qn TARGETDIR=<dir>; Get-AuthenticodeSignature on extracted
#   nono.exe / broker / nono-wfp-service.exe → all Valid

gh release edit v0.58.0 --notes-file .planning/phases/61-ship-release-v2-9-package-and-release-the-phase-60-confined-/61-RELEASE-NOTES.md
```

## Notes for the next reviewer

- The cosmetic `crates.io HTTP 303` + homebrew-bump job failures on the fork are expected and
  non-fatal (per the release.yml signing-order work); gate success on the `release` job + the
  payload-signature step, never the whole-workflow-green state.
- 61-RELEASE-NOTES.md (Task 1, committed `7d5bef4f`) is written and ready to publish.
- Phase 61 stays **incomplete** until this release ships; re-running `/gsd:execute-phase 61`
  resumes at Plan 61-04.
