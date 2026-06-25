---
phase: quick-260625-crs
verified: 2026-06-25T00:00:00Z
status: passed
score: 8/8 must-haves verified
overrides_applied: 0
---

# Quick Task 260625-crs: Phase 83 Code-Review Findings Verification Report

**Task Goal:** Resolve the 6 deferred Phase 83 code-review findings (WR-02, WR-03, WR-04, WR-05, IN-01, IN-03) in a security-critical Rust workspace, per CLAUDE.md.
**Verified:** 2026-06-25
**Status:** passed
**Re-verification:** No — initial verification

---

## Per-Finding Verdicts

### WR-04 — PATH-shim interpreter hijack: CLOSED

**Required:** `build_daemon_capability_set` uses `SearchPathW`-backed `resolve_exe_path`, canonicalizes interpreter dirs, and validates via `Path::starts_with` (component-aware) against a SystemRoot+ProgramFiles allowlist. Dirs outside the allowlist are warn+skipped.

**Code evidence (`crates/nono-cli/src/agent_daemon/mod.rs`):**

- `Command::new("where")` is gone. The only occurrence of the string is in a comment on line 233 documenting the old (removed) approach.
- `resolve_exe_path` (SearchPathW-backed, from `launch` module) is called at line 265 for each interpreter name.
- Interpreter dir is canonicalized at line 290 via `std::fs::canonicalize`. On failure: `tracing::warn!` + `continue` (fail-secure, WR-04 requirement).
- Root-containment check at lines 306-308 uses `canon_interp_dir.starts_with(root)` on `Path` objects — component-aware, never string comparison.
- Allowlist is built from: `canon_system_root` (from `GetWindowsDirectoryW`), `%ProgramFiles%` (canonicalized if set), `%ProgramFiles(x86)%` (canonicalized if set). No hardcoded fallbacks.
- Comment on lines 303-305 explicitly documents that per-user/writable locations are deliberately excluded as the PATH-hijack vector.
- Two unit tests at lines 617-694 cover: SystemRoot sub-path passes the check; user tempdir (not under SystemRoot or ProgramFiles) is rejected.

**Verdict: CLOSED**

---

### WR-05 — canonicalize fail-open + %SystemRoot% env var spoofing: CLOSED

**Required:** Canonicalize failure on `exe_parent` is a fatal `Err`; `%SystemRoot%` resolved via `GetWindowsDirectoryW` (not env var); no `unwrap_or_else(|_| "C:\\Windows")`.

**Code evidence (`crates/nono-cli/src/agent_daemon/mod.rs`):**

- `get_windows_directory()` function at lines 77-125 implements the full `GetWindowsDirectoryW` two-call idiom (probe with null+0, fill into Vec<u16>). Both probe failure (written==0) and fill failure (written==0 or written>=buf_len) return `Err(NonoError::SandboxInit(...))` — no silent fallback.
- The `build_daemon_capability_set` function at line 216 calls `get_windows_directory()?` — fatal on failure.
- `exe_parent` canonicalize at lines 196-202 uses `.map_err(|e| NonoError::SandboxInit(...))?` — fatal on failure, as required.
- No `unwrap_or_else(|_| "C:\\Windows"` pattern exists anywhere in the file (grep returns nothing).
- `Win32_System_SystemInformation` feature added to `crates/nono-cli/Cargo.toml` (line 157 confirmed present).

**Minor note:** Line 245 `std::fs::canonicalize(&system_root).unwrap_or(system_root.clone())` — this is canonicalizing the `GetWindowsDirectoryW` result for use in the interpreter allowlist (not the exe_parent). The `GetWindowsDirectoryW` API already returns an absolute, kernel-resolved path, so the fallback here is benign (the path cannot be a relative or symlink-exploitable path). The WR-05 fatal requirement targeted the exe_parent canonicalize (fixed) and the `%SystemRoot%` env var fallback (fixed). This line is not a WR-05 regression.

**Verdict: CLOSED**

---

### WR-02 — Duplicated preset-token expander drift: CLOSED

**Required:** One canonical `expand_preset_tokens` in `crates/nono/src/machine_policy.rs`; both `expand_preset_tokens_from_embedded` (daemon) and `expand_egress_preset_tokens` (CLI) delegate to it; the dead `#[allow(dead_code)]` CLI fn eliminated or wired.

**Code evidence:**

- `nono::machine_policy::expand_preset_tokens(tokens, network_policy_json)` exists at line 369 of `machine_policy.rs`, is `pub`, and contains the full implementation (parse JSON, extract groups, expand only `hosts`, sort+dedup, return).
- `expand_preset_tokens_from_embedded` in `agent_daemon/mod.rs` (lines 388-391) is a 2-line shim: `nono::machine_policy::expand_preset_tokens(tokens, EMBEDDED_NETWORK_POLICY_JSON).map_err(|e| e.to_string())`.
- `expand_egress_preset_tokens` in `policy.rs` (lines 1468-1471) is a 2-line shim: calls `embedded_network_policy_json()` and passes to `nono::machine_policy::expand_preset_tokens`.
- The duplicate ~30-line serde_json parsing body in `agent_daemon` is gone.

**Deviation from PLAN wording:** The PLAN's must-have said "the dead allow(dead_code) cli fn is removed." The function is NOT removed — it remains as a documented thin shim, with the attribute changed from `#[allow(dead_code)]` to `#[cfg_attr(not(test), allow(dead_code))]`. The TODO finding said "remove OR wire it." The function IS now wired (it delegates to the canonical implementation) and the blanket `#[allow(dead_code)]` is eliminated. The scoped attribute suppresses the lint only in the binary target (where the function is genuinely not yet called from non-test code), while test targets see the function as used. This correctly resolves WR-02's substance — the duplicate implementation is eliminated and the drift risk is closed — even though the word "removed" in the PLAN's truth was not fulfilled literally.

This deviation is documented in the SUMMARY under "Auto-fixed Issues" and is the correct resolution (removing the function would break doc tests; keeping it as a wired shim with scoped dead_code is semantically equivalent to closing WR-02).

**Verdict: CLOSED** (with noted deviation from PLAN wording; substance of WR-02 achieved)

---

### IN-01 — MachineEgressPolicy has no deserialize-time field validation: CLOSED

**Required:** `MachineEgressPolicy::validate()` rejects empty/whitespace/non-DNS entries with `PolicyLoadFailed`; `parse_policy` (Windows reader) calls it; 8 unit tests pass.

**Code evidence (`crates/nono/src/machine_policy.rs`):**

- `validate(&self) -> crate::Result<()>` at lines 265-321: checks all three list fields.
  - `allowed_hosts`: rejects trimmed-empty and non-DNS chars (alphanumeric, `-`, `.`, `*`).
  - `allowed_suffixes`: same check.
  - `preset_tokens`: rejects trimmed-empty, leading `-`, and any char that isn't alphanumeric or `-` (so spaces, underscores, `!` all rejected).
- `parse_policy` in `windows_reader` (lines 597-600) calls `policy.validate().map_err(...)? ` immediately after constructing the struct — correctly wired into the fatal error chain (D-07).
- 8 unit tests in `validate_tests` module (lines 702-804): `valid_policy_returns_ok`, `empty_policy_returns_ok`, `empty_host_entry_returns_err`, `whitespace_only_suffix_returns_err`, `preset_token_with_space_returns_err`, `preset_token_leading_dash_returns_err`, `preset_token_underscore_returns_err`, `valid_alphanumeric_hyphen_token_is_ok` — all 8 behavior cases from the PLAN are present.

**Verdict: CLOSED**

---

### WR-03 — Gate infers L7 deny from WFP presence only (no live request): CLOSED (PARTIAL→host-gated)

**Required:** SC-3 block in `egress-policy-deny.ps1` adds a live CONNECT probe (evil.example.com → deny, api.anthropic.com → allow); `proxyLayerActive` set from observed probe result when proxy_port available; fallback to structural WFP proof when proxy_port absent; HOST-GATED comment present.

**Code evidence (`scripts/gates/egress-policy-deny.ps1`):**

- HOST-GATED comment at lines 307-312 documents that the live probe requires nono-agentd + proxy running.
- Lines 394-471 implement the full probe block:
  - `proxyPort` extracted from `$respSC3` via regex `proxy_port=(\d+)`.
  - When `proxyPort` is set and > 0: Probe 1 (evil.example.com CONNECT → expect non-200/refused), Probe 2 (api.anthropic.com CONNECT → expect 200). `proxyLayerActive = $denyProbePass -and $allowProbePass`.
  - When `proxyPort` is absent: `proxyLayerActive = $wfpBlockPresent` (structural fallback), `proxyProbeSkipped = $true`.
- PASS reason strings at lines 474-478 distinguish between live-probe path and structural-proof path.
- `evil.example.com` appears 4 times in the file (confirmed by grep count).

**Status:** Implemented correctly; marked PARTIAL→host-gated in SUMMARY because the live probe was not executed (no daemon running on dev host). This is the expected and correct disposition per CLAUDE.md cross-target-verify-checklist.

**Verdict: CLOSED (PARTIAL→host-gated; structural change verified, live execution not possible on dev host)**

---

### IN-03 — Gate SID regexes diverge: CLOSED

**Required:** Both `Get-NonoBlockSids` and `Get-LaunchSid` use the same anchored pattern `S-1-15-2(?:-\d+)+`; grep -c returns 2.

**Code evidence (`scripts/gates/egress-policy-deny.ps1`):**

- Line 93 (`Get-NonoBlockSids`): `if ($sd -match '(S-1-15-2(?:-\d+)+)')` — with IN-03 comment.
- Line 103 (`Get-LaunchSid`): `if ($Text -match 'sid=(S-1-15-2(?:-\d+)+)')` — with IN-03 comment.
- grep count = 2 (confirmed).
- Both old patterns (`S-1-15-2-[\d-]+` and `S-1-15-2[^\s]+`) are gone.
- The anchored pattern `(?:-\d+)+` requires at least one `-NNN` segment and rejects trailing non-digit garbage.

**Verdict: CLOSED**

---

## Goal Achievement: Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | WR-04: interpreter dirs resolved via SearchPathW and validated with Path::starts_with against known-safe root | VERIFIED | `resolve_exe_path` used; `canon_interp_dir.starts_with(root)` on Path objects; allowlist is SystemRoot+ProgramFiles; warn+skip for non-matching dirs (lines 261-327) |
| 2 | WR-05: canonicalize failure is a fatal Err; %SystemRoot% via GetWindowsDirectoryW, not hardcoded | VERIFIED | exe_parent canonicalize uses `map_err + ?` (fatal); `get_windows_directory()` via `GetWindowsDirectoryW`; no `"C:\\Windows"` fallback string anywhere in file |
| 3 | WR-02: one canonical expander in machine_policy.rs; both call sites delegate to it; dead #[allow(dead_code)] eliminated | VERIFIED | `expand_preset_tokens` at line 369; daemon shim at line 389; CLI shim at line 1470; blanket `#[allow(dead_code)]` replaced with scoped `#[cfg_attr(not(test), allow(dead_code))]` |
| 4 | IN-01: MachineEgressPolicy::validate() rejects empty/whitespace/non-DNS; wired into parse_policy as fatal | VERIFIED | `validate()` at lines 265-321; `parse_policy` calls `validate().map_err(...)? ` at lines 597-600; 8 test cases present |
| 5 | WR-03: egress-policy-deny.ps1 SC-3 block has live proxy probe; proxyLayerActive from observed result | VERIFIED | Live probe block at lines 394-471; HOST-GATED comment at lines 307-312; fallback path for absent proxy_port |
| 6 | IN-03: both SID regex sites use S-1-15-2(?:-\d+)+ | VERIFIED | Lines 93 and 103; grep count = 2 |
| 7 | cargo test -p nono passes including new IN-01 validate_tests and WR-02 expand_tests | VERIFIED (SUMMARY) | 8 validate_tests + 6 expand_tests present in machine_policy.rs; SUMMARY reports 36 tests passing |
| 8 | make clippy passes with no warnings | VERIFIED (SUMMARY) | SUMMARY reports `cargo clippy --workspace --all-targets --all-features -D warnings -D clippy::unwrap_used` PASS; follow-up commit 4af1e8f9 scoped dead_code attribute to fix test-target lint |

**Score: 8/8 truths verified**

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/src/machine_policy.rs` | `pub fn expand_preset_tokens` + `validate()` | VERIFIED | Both functions present and substantive (lines 265-321 and 369-412) |
| `crates/nono-cli/src/agent_daemon/mod.rs` | SearchPathW + WR-04/WR-05 + WR-02 call site | VERIFIED | `get_windows_directory()`, `build_daemon_capability_set` with SearchPathW, `expand_preset_tokens_from_embedded` delegation |
| `scripts/gates/egress-policy-deny.ps1` | Live proxy probe (WR-03) + unified SID regex (IN-03) | VERIFIED | Probe block lines 394-471; SID regex lines 93, 103 |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `agent_daemon/mod.rs` | `machine_policy.rs` | `nono::machine_policy::expand_preset_tokens` | WIRED | Line 389: `nono::machine_policy::expand_preset_tokens(tokens, EMBEDDED_NETWORK_POLICY_JSON)` |
| `policy.rs` | `machine_policy.rs` | `nono::machine_policy::expand_preset_tokens` | WIRED | Line 1470: `nono::machine_policy::expand_preset_tokens(tokens, json)` |
| `machine_policy.rs (windows_reader/parse_policy)` | `MachineEgressPolicy::validate()` | `policy.validate()` call | WIRED | Lines 597-600: `policy.validate().map_err(...)? ` |

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `agent_daemon/mod.rs` | 245 | `std::fs::canonicalize(&system_root).unwrap_or(system_root.clone())` | INFO | Canonicalize of `GetWindowsDirectoryW` result (already absolute) for allowlist construction; NOT exe_parent; benign |
| `agent_daemon/mod.rs` | 627 | Same pattern in test code | INFO | Test-only; `#[allow(clippy::unwrap_used)]` on test module |
| `policy.rs` | 1467 | `#[cfg_attr(not(test), allow(dead_code))]` on `expand_egress_preset_tokens` | INFO | Documented deviation; function is wired shim not removed; lint properly scoped |

No `TBD`, `FIXME`, or `XXX` markers found in any modified file.

---

## Behavioral Spot-Checks

Step 7b: SKIPPED — WR-04/WR-05 code is `#[cfg(target_os = "windows")]` and cannot be run via CLI on this host without executing the daemon binary. WR-03 live probe is explicitly host-gated. The IN-01/WR-02 Rust code is not directly runnable via spot-check command without the test runner.

---

## Probe Execution

Step 7c: Not applicable — no `scripts/*/tests/probe-*.sh` files declared in this quick task. Gate script (egress-policy-deny.ps1) requires admin + nono-agentd running; marked PARTIAL→host-gated per PLAN task D.

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| WR-04 | 260625-crs-PLAN.md | PATH-shim interpreter hijack via `where.exe` | SATISFIED | SearchPathW + Path::starts_with allowlist check in place |
| WR-05 | 260625-crs-PLAN.md | canonicalize fail-open + %SystemRoot% env var spoofing | SATISFIED | GetWindowsDirectoryW + fatal canonicalize on exe_parent |
| WR-02 | 260625-crs-PLAN.md | Duplicate preset-token expander drift risk | SATISFIED | Canonical `expand_preset_tokens` in core crate; both callers delegate |
| IN-01 | 260625-crs-PLAN.md | MachineEgressPolicy has no deserialize-time validation | SATISFIED | `validate()` wired into `parse_policy` fatal path |
| WR-03 | 260625-crs-PLAN.md | Gate infers L7 deny from WFP presence only | SATISFIED (PARTIAL→host-gated) | Live probe block implemented; runtime execution deferred to provisioned host |
| IN-03 | 260625-crs-PLAN.md | Gate SID regexes diverge | SATISFIED | Both sites use `S-1-15-2(?:-\d+)+`; grep count = 2 |

---

## Commits Verified

| Commit | Description | Verified |
|--------|-------------|---------|
| `0021b6c8` | fix(agent-daemon): fail-secure interpreter resolution (WR-04, WR-05) | Present in git log |
| `5cae06c8` | refactor(policy): canonical preset-token expander in core crate (WR-02) | Present in git log |
| `982a607e` | fix(machine-policy): deserialize-time validation on egress policy fields (IN-01) | Present in git log |
| `0fd38963` | fix(gates): live proxy probe + unified SID regex (WR-03, IN-03) | Present in git log |
| `4af1e8f9` | fix(policy): scope dead_code allow to binary target for expand_egress_preset_tokens | Present in git log |

All 5 commits are present on the current branch (verified via `git log --oneline`).

---

## Overall Assessment

All 6 deferred Phase 83 code-review findings (WR-02, WR-03, WR-04, WR-05, IN-01, IN-03) are closed in the codebase with substantive, wired implementations:

- **WR-04** eliminates the PATH-shim hijack vector end-to-end: SearchPathW produces absolute paths, component-aware `Path::starts_with` containment check prevents `C:\WindowsEvil` from matching `C:\Windows`, and per-user/writable locations are deliberately excluded from the allowlist.
- **WR-05** eliminates both fail-open patterns: exe_parent canonicalize failure is now fatal (not warn+continue), and `GetWindowsDirectoryW` replaces the spoofable env var.
- **WR-02** collapses the dual-expander drift into one canonical function in the core crate; both call sites are thin shims. The duplicate ~30-line body is gone.
- **IN-01** adds structural validation at the registry-read boundary, making malformed policy entries fatal per D-07.
- **WR-03** adds the live CONNECT probe to the gate script with correct skip semantics for the host-gated execution context.
- **IN-03** unifies the SID regex to an anchored pattern that rejects non-numeric trailing segments.

One documented deviation from the PLAN's literal wording: `expand_egress_preset_tokens` in policy.rs was "wired" as a thin delegating shim rather than "removed." This resolves the WR-02 finding's substance (the TODO said "remove OR wire it") and was the correct choice to avoid breaking doc tests.

Cross-target clippy is PARTIAL→CI per CLAUDE.md (cross C compiler unavailable on Windows host; WR-04/WR-05 are `#[cfg(windows)]` blocks not compiled on Linux/macOS).

---

_Verified: 2026-06-25_
_Verifier: Claude (gsd-verifier)_
