---
slug: sigstore-tuf-rotation
quick_id: 260511-fpn
created: 2026-05-11
completed: 2026-05-11
type: docs-workaround
status: complete
---

# Summary: Sigstore TUF root rotation — workaround documented + upgrade tracked

## What broke

POC user ran `nono setup --refresh-trust-root` and got:

```
[3/5] Refreshing Sigstore trusted root...
ERROR Setup error: Failed to fetch Sigstore trusted root from
  https://tuf-repo-cdn.sigstore.dev: TUF error: TUF repository load failed:
  Failed to verify trusted root metadata:
  Signature threshold of 3 not met for role root (0 valid signatures)
```

CDN fetch succeeded; signature verification failed at the TOFU step.

## Root cause

`sigstore-verify 0.6.5` (the fork's pinned dep in `crates/nono/Cargo.toml:38`) embeds a TUF
trust anchor that's now stale — Sigstore rotated their TUF root keys, and zero of the
embedded anchor's public keys remain valid against the currently-published root.json.
**Phase 32 D-32-01 / D-32-03 fail-closed-on-bad-chain is doing exactly what it should**;
the failure is structural to the upstream crate's staleness, not a fork bug.

`sigstore-verify 0.6.6` (released 2026-04-29, repo `prefix-dev/sigstore-rust`) ships PR #69
"API for fetching / using the trust root" which refreshes the embedded anchor. Upgrading is
mechanical but blast-radius-wide (touches keyless sign + verify in 6 fork source files +
~6 test files) — out of scope for a same-day POC unblock.

## Why a manual workaround works (no code change needed)

Per Phase 32 D-32-15 verify-is-offline design:
**`load_production_trusted_root()` at `crates/nono/src/trust/bundle.rs:147` reads the
cached `trusted_root.json` via plain JSON deserialization (`TrustedRoot::from_file()`),
NOT TUF re-verification.** Only an expiry gate (tlog keys' `valid_for.end` in the past)
fires. The CLI never re-fetches; it trusts the cached bytes on disk.

That means a manually-placed `trusted_root.json` in the user's cache path unblocks
`nono trust verify` immediately. The repo already ships such a capture at
`crates/nono/tests/fixtures/trust-root-frozen.json` (commit `d9969978`, 6787 bytes,
captured 2026-05-10 from `sigstore/root-signing@main` — the day before the POC failure).

## Changes

### 1. `docs/cli/development/windows-poc-handoff.mdx`

Added "Known issue: Sigstore TUF root rotation (sigstore-verify 0.6.5)" subsection inside
the existing `## Sigstore Trust Root Setup` section. ~45 lines covering: failure signature,
root cause one-liner, verify-is-offline rationale, and the PowerShell `Invoke-WebRequest`
workaround pinned to commit `281f71ab` of the fork's `oscarmackjr-twg/nono` repo.

### 2. `.planning/phases/32-sigstore-integration/deferred-items.md`

Appended `P32-DEFER-005: sigstore-verify 0.6.5 → 0.6.6 upgrade (TUF root rotation)` entry.
~75 lines documenting: what's deferred, the blast-radius surface (6 source files + ~6 test
files), the workaround that keeps the POC moving, and how to complete the upgrade later
(version bumps + Cargo.lock refresh + `make ci` on all 3 platforms + remove the Known Issue
callout from the POC doc after the upgrade lands).

## POC user workaround (verbatim — same block now in the doc)

```powershell
$cacheDir = "$env:USERPROFILE\.nono\trust-root"
New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null
Invoke-WebRequest -UseBasicParsing `
  -Uri "https://raw.githubusercontent.com/oscarmackjr-twg/nono/281f71ab/crates/nono/tests/fixtures/trust-root-frozen.json" `
  -OutFile "$cacheDir\trusted_root.json"

# Confirm:
nono setup --check-only
# Should now report: Trust root cache: OK

# Then verify works:
nono trust verify `
  --issuer https://token.actions.githubusercontent.com `
  --identity '<your-identity-regex>' `
  <bundle-file>
```

The pinned commit sha `281f71ab` is the fork's HEAD at the time this workaround was
written; the fixture file hasn't changed since `d9969978` (2026-05-10), and the URL form
above gives the user a reproducible fetch that survives future HEAD movement.

## Verification

| Check | Result |
|-------|--------|
| `docs/cli/development/windows-poc-handoff.mdx` has the Known Issue subsection inside `## Sigstore Trust Root Setup` | ✅ added before `**Check status:**` |
| Workaround command's cache path matches `crates/nono/src/trust/bundle.rs::nono_trust_root_cache_path()` | ✅ `%USERPROFILE%\.nono\trust-root\trusted_root.json` |
| `Invoke-WebRequest` URL pinned to specific commit sha (not `main`) | ✅ pinned to `281f71ab` |
| `.planning/phases/32-sigstore-integration/deferred-items.md` has P32-DEFER-005 entry | ✅ appended |
| P32-DEFER-005 entry lists the full sigstore-* dep set + the 6 source files + ~6 test files | ✅ enumerated |
| No code changes (D-32-01 verify-is-offline invariant preserved by relying on existing offline-cache load) | ✅ pure docs/tracking |

## What this does NOT do

- **No `sigstore-verify` upgrade.** That's P32-DEFER-005 for v2.4+.
- **No new CLI flag** like `nono setup --refresh-trust-root --from-file <PATH>`. The
  manual file drop covers POC need; flag addition is part of the eventual upgrade work
  if useful then.
- **No build-system bundling** of the fixture into the MSI. Future possible: ship
  `trusted_root.json` as a release asset alongside `nono.exe` so POC users don't need to
  fetch from GitHub. Not needed for this immediate unblock.

## Files touched

- `docs/cli/development/windows-poc-handoff.mdx` (1 edit hunk; net +45 lines for Known
  Issue + workaround)
- `.planning/phases/32-sigstore-integration/deferred-items.md` (1 appended P32-DEFER-005
  block; +75 lines)
- `.planning/quick/260511-fpn-sigstore-tuf-rotation/` (PLAN + SUMMARY)

## Open follow-ups

- **P32-DEFER-005** (this task created the tracking entry) — bump sigstore-verify to 0.6.6
  + sibling crates in `crates/nono/Cargo.toml` + `crates/nono-cli/Cargo.toml`; rerun
  `make ci` on Linux/macOS/Windows; remove the Known Issue subsection from the POC doc
  once `nono setup --refresh-trust-root` succeeds end-to-end against live
  `tuf-repo-cdn.sigstore.dev`.
- **Long-term:** consider whether the embedded-trust-anchor staleness pattern justifies
  bundling a fork-controlled trusted_root.json as a release asset, with a setup path that
  installs it without an external fetch. This is an architectural call for whenever the
  next Sigstore root rotation lands — sigstore-verify 0.6.6 will eventually be stale too.
