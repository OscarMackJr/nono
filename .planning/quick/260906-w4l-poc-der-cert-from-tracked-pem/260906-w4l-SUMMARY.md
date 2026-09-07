---
quick_id: 260906-w4l
slug: poc-der-cert-from-tracked-pem
date: 2026-09-07
status: complete
commits:
  - 710cffdf
milestone: v3.7
unblocks_jobs:
  - Windows Build
  - Windows Packaging
---

# Quick Task 260906-w4l — Summary

**Status:** complete. Item 1 of 7 from the CI run `34068950597` triage.

## What was wrong

`build-windows-msi.ps1:239` threw `POC DER cert not found … Commit
dist/windows/nono-poc-signing.cer to the repo.` — instructing the operator to
commit a file `.gitignore` rejects twice (`nono-poc-signing.cer` and
`dist/windows/*.cer`). Both `Windows Build` and `Windows Packaging` died there,
and had for as long as the job has run.

The script's comment declared the DER the source of truth with the PEM derived
from it. The repo tracks the mirror image: PEM committed, `.cer` ignored.

## What was verified before touching anything

Both files are the **same** self-signed public certificate — identical SHA-1
`31:9E:50:7E:95:04:72:D4:90:F5:6F:7C:4C:D9:44:37:C0:13:CC:06`, subject and issuer
`CN=nono POC Signing`. The PEM holds one `CERTIFICATE` block and **no**
`PRIVATE KEY` block. No key material was added; the `.gitignore` entries that
guard private material (`*.p12`, `*.pfx.b64`, `*.cert.b64`) are untouched.

## Fix

Inverted the fallback so the tracked artifact is the source:

| Tree state | Behavior |
|---|---|
| DER present | use it (unchanged — preserves the local-dev flow) |
| DER absent, PEM present | **derive the DER** |
| both absent | throw, naming the PEM as the thing to restore |

The pre-existing PEM-from-DER fallback stays, so both directions work.

**Rejected alternative:** `git add -f` plus a `.gitignore` negation. It would
store the same public key twice in two encodings with nothing enforcing they
agree, and would punch a special case through a rule that also guards per-release
POC certs (`nono-v*-poc.cer`). Decoding is pure PowerShell base64, deliberately
not `certutil -decode`, since the script already cannot assume certutil is on
PATH — its own `-encode` fallback says so. The regex takes only the first
`CERTIFICATE` block so a multi-cert PEM cannot concatenate into an invalid blob.

## Verification

This host holds the genuine `.cer` that CI lacks, making it a true oracle rather
than a self-consistency check.

1. **Byte-identity:** DER derived from the tracked PEM vs the real `.cer` —
   778 bytes and `sha256 A9A95AC9…8FFF` on both. Parsed thumbprint
   `319E507E950472D490F56F7C4CD94437C013CC06`.
2. **Perturbation reproducing CI exactly:** moved the `.cer` aside, ran
   `build-windows-msi.ps1 -Scope machine`. It printed `Deriving DER cert from
   tracked PEM`, then **built the MSI end-to-end successfully**, and the `.cer`
   it regenerated was byte-identical to the stashed original. Restored after.
3. **Blast radius:** only this script reads the source `nono-poc-signing.cer`.
   Every other reference in the tree (`cert_trust.rs`, `cli.rs`,
   `provision_windows.rs`, the WiX `<File>` component) is to `nono-poc-root.cer`
   — the *installed* name the MSI emits from it.

## Finding worth carrying

**A build script can require what the repo forbids, indefinitely, without
anyone noticing.** The instruction "commit this file" and the `.gitignore` rule
rejecting it lived in the same tree and contradicted each other. Nothing checks
that a path a script demands is actually committable. Worth a cheap guard if this
recurs: any script that `throw`s "commit X" should have X verified against
`git check-ignore` in CI, or derive X instead of demanding it.
