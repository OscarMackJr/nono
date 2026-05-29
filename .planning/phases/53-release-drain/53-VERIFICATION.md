---
phase: 53-release-drain
verified: 2026-05-29T00:00:00Z
status: passed
score: 5/5
overrides_applied: 0
human_verification_resolved:
  - test: "no-PTY supervised path on the released v0.57.5 binary (SC-2)"
    result: "PASS (2026-05-29) — operator ran `cd %USERPROFILE%\\.claude; nono run --profile claude-code -- claude --version` on the installed v0.57.5 MSI from a real PowerShell console. The signed broker constructed a Low-IL primary token, spawned the Low-IL child (pid 19112), the child printed `2.1.156 (Claude Code)`, and exited code 0. No Self-trust-anchor error. The doubly-broken v2.7 path (d8b7ce00 + 005b4c9e) is confirmed working on the released signed binary."
---

# Phase 53: Release & Drain — Verification Report

**Phase Goal:** The post-v2.7 fixes ship as a real signed release and the carry-forward debt is cleared
**Verified:** 2026-05-29
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A v2.8 git tag exists; signed MSIs (machine + user) are produced off the post-`005b4c9e` nono.exe and the installed binary reports the v2.8 fork version | VERIFIED | Tags `v0.57.5` and `v2.8` both exist at `a3927be0`; GitHub release `v0.57.5` has `nono-v0.57.5-x86_64-pc-windows-msvc-machine.msi` and `nono-v0.57.5-x86_64-pc-windows-msvc-user.msi` uploaded. Operator confirmed on live install: `nono.exe` Authenticode `Valid`, `nono --version` = `0.57.5`. REQUIREMENTS.md REQ-RLS-01 `[x]` with SATISFIED note. |
| 2 | `nono run --profile claude-code -- <binary> --version` on the real-console no-PTY supervised path exits 0 and prints the version | VERIFIED | Operator ran `cd %USERPROFILE%\.claude; nono run --profile claude-code -- claude --version` on the installed v0.57.5 MSI from a real console (2026-05-29). Signed broker constructed Low-IL primary token, spawned Low-IL child (pid 19112), child printed `2.1.156 (Claude Code)`, exited code 0. No Self-trust-anchor error. The first attempt used `nono.exe` (in `C:\Program Files\nono`, not profile-covered) and was correctly refused at the executable-coverage gate; using a profile-covered child (`claude.exe` under `%USERPROFILE%\.claude`) exercises the broker path and passes. The doubly-broken v2.7 path (`d8b7ce00` + `005b4c9e`) is confirmed working on the released signed binary. |
| 3 | release.yml completes without startup_failure on a live v* tag push and produces signed artifacts | VERIFIED | CI run `26639177584` (tag `v0.57.5`): all 5 build jobs `success`, `Create Release` `success`. The two failures are cosmetic fork jobs (`Publish to crates.io` HTTP 303, `Bump Homebrew Core Formula`) that are explicitly accepted. No `startup_failure`. New "Sign Windows binaries (pre-package)" step and "Verify MSI payload signatures (Windows)" gate both pass. `v2.8` (two-segment) did not fire a CI run, confirming the `v*.*.*` trigger fix. |
| 4 | Elevated `sc stop` + `msiexec /x` leaves no WFP service, driver, or residual filters behind | VERIFIED | 53-04-SUMMARY documents all 5 UAT-C steps PASS on Windows 11 build 26200: Fix #1 (`sc.exe stop` accepted), Fix #2a (`nono setup --uninstall-wfp` removes both services), Fix #2b (WiX CA — `msiexec /x` leaves no service/driver/install-dir/filters), upgrade guard (`NOT UPGRADINGPRODUCTCODE`) preserved services. REQUIREMENTS.md REQ-DRN-01 `[x]`. Todo 1 in `todos/done/` (commit `bbcb5f97`). |
| 5 | The 3 pending todos are resolved or explicitly re-dispositioned with committed rationale | VERIFIED | All 3 todos closed: Todo 1 (`2026-05-27-wix-auto-uninstall-wfp-custom-action-plus-live-uat.md`) moved to `done/` with PASS disposition after UAT-C (commit `bbcb5f97`). Todos 2+3 (`44-class-d-validator-preflight-investigation.md`, `44-validate-restore-target-fd-relative-hardening.md`) moved to `done/` with D-53-08 rationale (commit `5b6fb2f2`). `.planning/todos/pending/` is empty. REQUIREMENTS.md has REQ-DENY-PREFLIGHT-01 and REQ-UNDO-TOCTOU-01 in v2 Deferred (4 matches across body + traceability table). |

**Score:** 5/5 truths verified (SC-2 closed by operator no-PTY test on v0.57.5, 2026-05-29)

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/Cargo.toml` | version = "0.57.5" | VERIFIED | Line 3: `version = "0.57.5"` (includes v0.57.4 → v0.57.5 re-bump via `ae5c3358`) |
| `crates/nono-cli/Cargo.toml` | version = "0.57.5" + 3 path-dep pins at 0.57.5 | VERIFIED | All path-dep pins (nono, nono-proxy, nono-shell-broker) read `"0.57.5"` |
| `crates/nono-proxy/Cargo.toml` | version = "0.57.5" + nono path-dep at 0.57.5 | VERIFIED | Confirmed |
| `crates/nono-shell-broker/Cargo.toml` | version = "0.57.5" + nono path-dep at 0.57.5 | VERIFIED | Confirmed |
| `bindings/c/Cargo.toml` | version = "0.57.5" + nono path-dep at 0.57.5 | VERIFIED | Confirmed |
| `.github/workflows/release.yml` | `on.push.tags: - 'v*.*.*'` + signing-order fix | VERIFIED | Trigger is `'v*.*.*'` (line confirmed); "Sign Windows binaries (pre-package)" step at line 146 runs before "Package (Windows)" at line 166; "Verify MSI payload signatures (Windows)" gate at line 258; signing gate (WINDOWS_SIGNING_CERT fail-closed) preserved |
| `scripts/sign-poc-local.ps1` | VersionTag default reflects release version | WARNING | Default is `"v0.57.4"` (not bumped to `v0.57.5` after the re-release). Plan 03 must_have required the default "matches the new release version". After v0.57.5 was cut, this was not re-updated. Minor drift only — the script is a local POC helper, not part of CI. |
| `docs/cli/development/windows-signing-guide.mdx` | CA-ready + fresh-cert sections | VERIFIED | Both sections present (lines 240+ "Production signing path (CA-ready)", line 261 "Fresh CI cert: generation, encoding, and trust redistribution"); commit `eceac886` (+110 lines) |
| `.planning/todos/done/2026-05-27-wix-auto-uninstall-wfp-custom-action-plus-live-uat.md` | Disposition footer with PASS, REQ-DRN-01 | VERIFIED | File exists in `done/`; "Disposition (Phase 53) — CLOSED" section present; references REQ-DRN-01 and D-53-07 |
| `.planning/todos/done/44-class-d-validator-preflight-investigation.md` | Disposition footer, pointer to REQ-DENY-PREFLIGHT-01 | VERIFIED | File exists; "Disposition (Phase 53)" present; references REQ-DENY-PREFLIGHT-01 |
| `.planning/todos/done/44-validate-restore-target-fd-relative-hardening.md` | Disposition footer, pointer to REQ-UNDO-TOCTOU-01 | VERIFIED | File exists; "Disposition (Phase 53)" present; references REQ-UNDO-TOCTOU-01 |
| `.planning/REQUIREMENTS.md` | REQ-DENY-PREFLIGHT-01 + REQ-UNDO-TOCTOU-01 in v2 Deferred | VERIFIED | 4 matches in grep (2 body entries + 2 traceability rows); all 4 Phase 53 REQs marked `[x]` Complete |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `release.yml` on.push.tags | CI build trigger | `'v*.*.*'` glob | WIRED | Confirmed line: `- 'v*.*.*'`; v2.8 (two-segment) confirmed not firing |
| "Sign Windows binaries (pre-package)" step | MSI harvest step | Runs before `build-windows-msi.ps1` | WIRED | Step at line 146 precedes "Package (Windows)" at line 166 per `release.yml` job sequence |
| "Verify MSI payload signatures" step | CI gate | Admin-extracts MSI, checks .exe Authenticode | WIRED | Step at line 258; gates only `.exe` files (nono.exe, broker, wfp-service), logs driver .sys informationally |
| `crates/nono-cli/Cargo.toml` path-dep | `crates/nono/Cargo.toml` | `nono = { version = "0.57.5", path = "../nono" }` | WIRED | Confirmed |
| `.planning/todos/done/44-class-d-*.md` | `.planning/REQUIREMENTS.md` | Pointer to REQ-DENY-PREFLIGHT-01 | WIRED | Cross-reference confirmed |
| `.planning/todos/done/44-validate-*.md` | `.planning/REQUIREMENTS.md` | Pointer to REQ-UNDO-TOCTOU-01 | WIRED | Cross-reference confirmed |
| GitHub Release `v0.57.5` | Tags `v0.57.5` + `v2.8` at `a3927be0` | `git tag v0.57.5; git push origin v0.57.5` | WIRED | Both tags exist at `a3927be0` on origin |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REQ-RLS-01 | 53-01, 53-03, 53-04 | v2.8 tag + signed MSIs off post-`005b4c9e` nono.exe; installed binary reports v2.8 fork version; no-PTY supervised path works | VERIFIED | v0.57.5 tag + signed MSIs + installed-binary version + payload Authenticode all confirmed. No-PTY path now VERIFIED on the released binary (operator ran `nono run --profile claude-code -- claude --version` with the profile-covered child; child printed `2.1.156`, exit 0). REQUIREMENTS.md `[x]`. |
| REQ-RLS-02 | 53-03, 53-04 | release.yml runs to completion on v* tag push, no startup_failure | VERIFIED | CI run `26639177584` green on all non-cosmetic jobs; no startup_failure |
| REQ-DRN-01 | 53-04 | WFP elevated live-uninstall leaves nothing behind | VERIFIED | All 5 UAT-C steps PASS per 53-04-SUMMARY |
| REQ-DRN-02 | 53-02 | 3 pending todos resolved/re-dispositioned with committed rationale | VERIFIED | All 3 todos in `done/` with disposition sections; REQUIREMENTS.md backlog updated |

---

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `scripts/sign-poc-local.ps1` | `$VersionTag = "v0.57.4"` not updated to v0.57.5 after re-release | INFO | Script default is stale by one version patch. Not a blocker — the script is a local POC signing helper; CI does not use this default. No security impact. Would mislead an operator signing a local POC build without overriding the tag. |

No TBD/FIXME/XXX/TODO markers found in any file modified by this phase.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All 5 crates at version 0.57.5 | `grep "^version = " crates/*/Cargo.toml bindings/c/Cargo.toml` | All 5 lines read `version = "0.57.5"` | PASS |
| No 0.57.3 strings remain | `grep -c "0\.57\.3" crates/*/Cargo.toml bindings/c/Cargo.toml` | All 0 | PASS |
| release.yml trigger is semver-only | `grep "tags:" -A3 .github/workflows/release.yml` | `- 'v*.*.*'` | PASS |
| Tags v0.57.5 and v2.8 exist | `git tag --list v0.57.5 v2.8` | Both present | PASS |
| Both tags point to same commit | `git rev-parse v0.57.5; git rev-parse v2.8` | Both `a3927be0` | PASS |
| GitHub release v0.57.5 has Windows MSIs | `gh release view v0.57.5 --json assets` | machine.msi + user.msi + SHA256SUMS.txt present | PASS |
| CI run conclusion for v0.57.5 | `gh run view 26639177584` | `Build x86_64-pc-windows-msvc: success; Create Release: success`; only cosmetic fork jobs failed | PASS |
| pending/ is empty | `ls .planning/todos/pending/` | Empty (no output) | PASS |
| done/ contains all 3 disposed todos | `ls .planning/todos/done/` | All 3 files present | PASS |

---

### Human Verification Required

### 1. No-PTY supervised path on installed MSI (ROADMAP SC-2, REQ-RLS-01 partial)

**Test:** On a Windows 11 machine with the v0.57.5 machine MSI installed and the POC cert in `LocalMachine\Root` + `TrustedPublisher`, from a real PowerShell console, run a command that exercises the no-PTY broker path where the child binary's path is covered by the profile. Options:
- Adjust the `claude-code` profile to cover `C:\Program Files\nono`, then: `nono run --profile claude-code -- nono --version`
- Or: identify a profile-covered binary path (e.g., `%USERPROFILE%\.claude`) and run a test binary from there: `nono run --profile claude-code -- <profile-covered-binary> --version`
- Or: explicitly accept via override that the dev-layout validation from v2.7 close is sufficient and document this as a test-design limitation.

**Expected:** Command exits 0 AND prints the version string to stdout. No empty output. No exit 1.

**Why human:** Requires a live Windows 11 install with the signed MSI, a real console (not MSYS/git-bash), and a working profile/child combination that exercises the broker. The underlying relay fix code (`d8b7ce00` + `005b4c9e`) is in the codebase — this is purely a test-execution gap, not a code gap. Can be closed by (a) running the test with a suitable child binary, or (b) adding an override entry to this VERIFICATION.md frontmatter documenting that dev-layout validation is accepted as sufficient.

---

### Gaps Summary

**No gaps remain.** All 5 success criteria VERIFIED. SC-2 (the sole prior uncertainty)
was closed on 2026-05-29: the operator ran `nono run --profile claude-code -- claude
--version` on the installed v0.57.5 MSI from a real console with the profile-covered
child `claude.exe` under `%USERPROFILE%\.claude`; the signed broker spawned the Low-IL
child, which printed `2.1.156 (Claude Code)` and exited code 0. The earlier failed
attempt used `nono.exe` in `C:\Program Files\nono` (not profile-covered) and was
correctly refused at the executable-coverage gate — that was a test-design issue, not a
code defect. **Phase 53: passed.**

---

### Additional Observations

**sign-poc-local.ps1 version drift (INFO):** The script's `$VersionTag` default reads `"v0.57.4"` because Plan 03 set it to match the then-current release target, and the subsequent v0.57.4→v0.57.5 re-bump (commit `ae5c3358`) did not update it. This is a local POC helper script — CI does not use its default value. Not a blocker. A follow-up patch bump to `"v0.57.5"` would keep it consistent.

**ROADMAP phase status not updated (INFO):** ROADMAP.md still shows `[ ] 53-04-PLAN.md` and `[ ] Phase 53: Release & Drain` as incomplete. REQUIREMENTS.md correctly shows all 4 Phase 53 REQs as `[x]` Complete. This is a doc-sync artifact — no functional impact.

**v0.57.4 release still has unsigned-payload MSIs (INFO):** The v0.57.4 GitHub Release is still present with MSIs containing unsigned payload binaries (the bug that triggered the v0.57.5 re-release). 53-04-SUMMARY flags this as a non-blocking follow-up: delete or annotate the v0.57.4 release to prevent inadvertent distribution of the unsigned build.

---

_Verified: 2026-05-29_
_Verifier: Claude (gsd-verifier)_
