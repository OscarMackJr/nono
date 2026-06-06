---
phase: 61-ship-release-v2-9-package-and-release-the-phase-60-confined
plan: 04
status: complete
completed: 2026-06-06
requirements: [REQ-RLS-03, REQ-RLS-04]
requirements-completed: [REQ-RLS-03, REQ-RLS-04]
release_tag: v0.62.2
release_url: https://github.com/OscarMackJr/nono/releases/tag/v0.62.2
---

# 61-04 SUMMARY — Ship v2.9 (terminal release plan)

**One-liner:** v2.9 is published as a CI-signed public GitHub release — `v0.62.2`, all five
build legs green, signed machine + user MSIs (wrapper + embedded payloads Authenticode-valid),
release notes published.

## Outcome

**SHIPPED.** https://github.com/OscarMackJr/nono/releases/tag/v0.62.2 (Latest). `release.yml`
run `27074741774`: all 5 build legs `success`, `Create Release` `success`. The Windows job's
`Check signing secrets` → `Sign Windows binaries (pre-package)` → `Sign Windows MSIs` →
`Verify Authenticode signatures` → `Verify MSI payload signatures` steps all passed, so the
published machine + user MSIs are Authenticode-valid (wrapper AND embedded payloads). Cosmetic
`Publish to crates.io` (HTTP 303) + `Bump Homebrew Core Formula` jobs failed as expected on the
fork — non-blocking per the plan's `<interfaces>` note.

## Tasks

- **Task 1 (release notes):** `61-RELEASE-NOTES.md` written — Phase 60 confined tool-mediation
  POC + Phase 62 out-of-box WFP enforcement, framed honestly as defense-in-depth/POC, with the
  D-09 hook-layer `~/.claude` self-disable boundary note. Published to the release via
  `gh release edit v0.62.2 --notes-file`.
- **Task 2 (operator checkpoint — tag/release/sign-check):** done, with deviations (below).
- **Task 3 (release-verify record):** `61-RELEASE-VERIFY.md` updated to `status: shipped` /
  `release_status: SHIPPED` with the real root cause and run URL; the historical BLOCKED
  analysis is retained verbatim below a RESOLUTION header.

## Deviations from plan

1. **Release version is `v0.62.2`, not `v0.58.0`.** The plan was authored against `v0.58.0`. The
   crate version had since leapfrogged `0.58.0 → 0.62.0` (commit `145a58d3`, to clear upstream's
   `v0.58.0–v0.61.1` tag collision in this repo's namespace). The actual published tag is
   `v0.62.2` after the hotfix sequence below.
2. **Two hotfix iterations were required** because two latent cfg-gated cross-target compile
   errors — in code the Windows dev host never compiles — broke all four Linux/macOS build legs
   and gated `Create Release`, so `v0.62.0` and `v0.62.1` failed to publish:
   - `error[E0716]: temporary value dropped while borrowed` in `claude_code_hook.rs`
     `wrapped_bash_command` (`cfg(not(windows))`, Phase 60 / PR #4) — fixed `4de294e8`.
   - `error: let chains are only allowed in Rust 2024 or later` in `hook_runtime.rs`
     `EnvFileGuard::drop` (`cfg(unix)`, Phase 58) — fixed `7bb7c7e3` (masked behind the E0716, so
     it only surfaced on the `v0.62.1` attempt). Both fixed → `v0.62.2` published.
   - Diagnosed/fixed via quick task `260606-mp9` + follow-on commits. Root cause + durable lesson
     recorded in memory [[feedback_clippy_cross_target]]. Confirmed (by reading CI logs) that all
     *other* Linux/macOS failures are pre-existing `-Dwarnings` lint debt that `release.yml`'s
     plain `cargo build` does NOT gate on.
3. **D-02 signing was NOT the blocker** (contrary to the original `61-RELEASE-VERIFY.md` BLOCKED
   diagnosis). Windows always built + signed cleanly; signing material is production-working
   (Azure Trusted Signing, wired 2026-06-04).
4. **Tags annotated, not GPG-signed.** The plan said `git tag -s`; this repo has no GPG key, so
   annotated `git tag -a` was used (DCO text trailer is the project's signing convention;
   `release.yml` triggers on the `v*.*.*` glob regardless).
5. **`v2.9` milestone tag** already existed at `145a58d3` (pre-hotfix); left in place as an
   ancestor of the published commit rather than force-moved.

## Must-haves status

| Must-have | Status |
|-----------|--------|
| Tag push triggers release.yml on the bumped commit | ✅ `v0.62.2` → run `27074741774` |
| release.yml produces Authenticode-valid machine + user MSIs (wrapper + payloads) | ✅ all sign/verify steps green |
| CI signing material present; no POC-cert fallback | ✅ production signing; build signed clean |
| v2.9 milestone annotation tag exists | ✅ (`145a58d3`, pre-existing) |
| Release notes published with honest POC framing + D-09 boundary | ✅ published to v0.62.2 |
| Post-release MSI signature confirmation | ✅ via the CI `Verify MSI payload signatures` gate (green); operator host spot-check optional follow-up |

## Requirements

- **REQ-RLS-03** (CI-signed public release) — ✅ satisfied (v0.62.2 published, signed MSIs, notes).
- **REQ-RLS-04** (hook-layer `~/.claude` deny guard) — ✅ satisfied (verified in 61-02 /
  `61-D09-VERIFICATION.md`, 16/16 hook unit tests; documented in the release notes).

## Follow-ups (non-blocking)

- Pre-existing CI `-Dwarnings` lint debt (dead_code `timeouts.rs` PTY constants, unused
  `format_bytes_short` / import / `resource_session_id`) keeps CI red but does not affect
  `release.yml`. Cleanup is a separate hygiene task.
- Optional operator host-side `Get-AuthenticodeSignature` spot-check on the downloaded MSIs
  (the CI payload-verify gate already passed).
