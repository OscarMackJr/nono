---
phase: quick-260625-crs
plan: 01
subsystem: security
tags: [windows, egress-policy, machine-policy, sandbox, capability-set, powershell-gates]

requires:
  - phase: phase-83
    provides: "MachineEgressPolicy + build_daemon_capability_set + egress-policy-deny.ps1 (baseline implementations)"

provides:
  - "Fail-secure interpreter resolution in build_daemon_capability_set (WR-04, WR-05)"
  - "Canonical preset-token expander in nono::machine_policy::expand_preset_tokens (WR-02)"
  - "Deserialize-time structural validation on MachineEgressPolicy (IN-01)"
  - "Live proxy probe + unified SID regex in egress-policy-deny.ps1 (WR-03, IN-03)"

affects: [agent-daemon, machine-policy, egress-policy-gate]

tech-stack:
  added: ["Win32_System_SystemInformation feature for windows-sys (GetWindowsDirectoryW)"]
  patterns:
    - "Path::starts_with component-aware allowlist check (not string starts_with) for interpreter root-containment"
    - "GetWindowsDirectoryW probe idiom (null+0 call for required size, then fill call)"
    - "Canonical core-library function taking json-as-param to stay policy-free"
    - "#[cfg_attr(not(test), allow(dead_code))] scoped suppression for pending-wiring shims"

key-files:
  created: []
  modified:
    - "crates/nono-cli/src/agent_daemon/mod.rs"
    - "crates/nono/src/machine_policy.rs"
    - "crates/nono-cli/src/policy.rs"
    - "crates/nono-cli/Cargo.toml"
    - "scripts/gates/egress-policy-deny.ps1"

key-decisions:
  - "WR-04: expand interpreter allowlist to SystemRoot+ProgramFiles+ProgramFiles(x86); per-user locations (LOCALAPPDATA) deliberately excluded as PATH-hijack vector"
  - "WR-02: expand_preset_tokens expands only hosts (not suffixes) matching existing behavior; suffixes double-add when caller also calls raw_allowlist()"
  - "WR-02: #[cfg_attr(not(test), allow(dead_code))] used instead of #[expect(dead_code)] — #[expect] fires unfulfilled_lint_expectations in test targets where the function IS used"
  - "WR-03: structural proof fallback retained when proxy_port absent; live probe only fires when daemon IS running (non-optional when daemon present)"
  - "IN-01: validate() is structural-only (character-set sanity); no host-resolvability or token-existence check (library stays policy-free)"

requirements-completed: [WR-02, WR-03, WR-04, WR-05, IN-01, IN-03]

duration: 55min
completed: 2026-06-25
---

# Quick Task 260625-crs: Phase 83 Code-Review Deferred Findings Summary

**Six deferred Phase-83 security findings closed: PATH-shim interpreter hijack eliminated via SearchPathW+component-aware allowlist (WR-04), GetWindowsDirectoryW replaces spoofable %SystemRoot% env var (WR-05), dual-expander drift collapsed to canonical nono::machine_policy::expand_preset_tokens (WR-02), MachineEgressPolicy::validate() adds deserialize-time DNS-sanity checks (IN-01), SC-3 gate gains live proxy probe (WR-03), and both SID regex sites unified to anchored S-1-15-2(?:-\d+)+ (IN-03)**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-06-25T00:00:00Z
- **Completed:** 2026-06-25
- **Tasks:** 4 (A, B, C, D)
- **Files modified:** 5

## Accomplishments

- **WR-04 + WR-05 (Task A):** `build_daemon_capability_set` now uses `resolve_exe_path` (SearchPathW) for interpreter resolution, validates interpreter dirs via `Path::starts_with` against a canonicalized SystemRoot+ProgramFiles allowlist, makes `canonicalize(exe_parent)` fatal instead of warn+continue, and calls `GetWindowsDirectoryW` instead of `env::var("SystemRoot")`. Two unit tests added (SystemRoot sub-path passes; user tempdir rejected).
- **WR-02 (Task B):** `nono::machine_policy::expand_preset_tokens(tokens, json)` is the canonical implementation; `expand_preset_tokens_from_embedded` and `expand_egress_preset_tokens` are now thin shims. The duplicate ~30-line body in agent_daemon is replaced by a 2-line delegation. Six unit tests cover all behavior cases.
- **IN-01 (Task C):** `MachineEgressPolicy::validate()` added; `parse_policy` (Windows reader) calls it after constructing the struct with IN-01 error propagating as `PolicyLoadFailed` (D-07 abort chain). Eight unit tests cover empty, whitespace, invalid DNS chars, leading-dash tokens, and underscore tokens.
- **WR-03 + IN-03 (Task D):** egress-policy-deny.ps1 SC-3 block adds live CONNECT probe to evil.example.com (deny) and api.anthropic.com (allow); `proxyLayerActive` is set from observed probe results when `proxy_port` is in the daemon response. Both SID regex sites use `S-1-15-2(?:-\d+)+`. HOST-GATED comment added.

## Task Commits

1. **Task A: WR-04 + WR-05 — Fail-secure interpreter resolution** - `0021b6c8` (fix)
2. **Task B: WR-02 — Canonical preset-token expander** - `5cae06c8` (refactor)
3. **Task C: IN-01 — Deserialize-time validation** - `982a607e` (fix)
4. **Task D: WR-03 + IN-03 — Gate-script live probe + unified SID regex** - `0fd38963` (fix)
5. **Follow-up: dead_code scope fix for policy.rs** - `4af1e8f` (fix)

## Files Created/Modified

- `crates/nono-cli/src/agent_daemon/mod.rs` — WR-04/WR-05 fail-secure interpreter resolution; WR-02 delegation to canonical expander; 3 new unit tests
- `crates/nono/src/machine_policy.rs` — WR-02 canonical `expand_preset_tokens`; IN-01 `MachineEgressPolicy::validate()`; 14 new unit tests (8 validate_tests + 6 expand_tests)
- `crates/nono-cli/src/policy.rs` — WR-02 `expand_egress_preset_tokens` delegating to canonical; `#[allow(dead_code)]` replaced with `#[cfg_attr(not(test), allow(dead_code))]`
- `crates/nono-cli/Cargo.toml` — Added `Win32_System_SystemInformation` to windows-sys features (for GetWindowsDirectoryW)
- `scripts/gates/egress-policy-deny.ps1` — IN-03 unified SID regex; WR-03 live proxy probe block; HOST-GATED comment

## Decisions Made

- **WR-04 allowlist scope:** SystemRoot + ProgramFiles + ProgramFiles(x86) only; LOCALAPPDATA deliberately excluded (PATH-hijack vector). Documented with comment in code for operator extension.
- **WR-02 suffixes excluded from expand_preset_tokens:** AI-provider presets express wildcard coverage as `*.domain` hosts, not suffixes. Including suffixes would double-add when `raw_allowlist()` also returns them. Documented in function docstring.
- **#[cfg_attr(not(test), allow(dead_code))] instead of #[allow(dead_code)] or #[expect(dead_code)]:** `#[expect(dead_code)]` fires `unfulfilled_lint_expectations` in test targets (where the function IS used by `cfg(test)` callers), breaking `make clippy --all-targets`. `#[cfg_attr(not(test), allow(dead_code))]` scopes suppression to the binary target only. Semantically cleaner than blanket `#[allow]`.
- **WR-03 fallback to structural proof:** When `proxy_port` is absent from the daemon launch response (dev host, no daemon), the structural WFP-block proof is used. Live probe is mandatory only when the daemon IS running.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] #[expect(dead_code)] fired unfulfilled_lint_expectations in test targets**
- **Found during:** Task B (WR-02 wiring), verification step
- **Issue:** `expand_egress_preset_tokens` is called in `cfg(test)` callers. In test mode, `dead_code` doesn't fire → `#[expect(dead_code)]` produces `unfulfilled_lint_expectations` → `make clippy --all-targets` fails with `-D warnings`.
- **Fix:** Replaced `#[expect(dead_code)]` with `#[cfg_attr(not(test), allow(dead_code))]` — suppresses lint in binary target only, transparent in test targets.
- **Files modified:** `crates/nono-cli/src/policy.rs`
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` passes
- **Committed in:** `4af1e8f` (follow-up after Task B commit)

---

**Total deviations:** 1 auto-fixed (blocking lint in test target)
**Impact on plan:** Zero scope change. The fix is the correct scoping of a lint suppression attribute.

## Known Stubs

None — all code changes are functional. `expand_egress_preset_tokens` in policy.rs is a thin shim pending wiring (the plan documents this; the daemon path is already fully wired through `expand_preset_tokens_from_embedded`).

## Threat Flags

None — all changes address known threat-model items (T-crs-01 through T-crs-05). No new trust boundaries introduced.

## Verification Results

| Check | Result | Notes |
|-------|--------|-------|
| `cargo build --workspace --all-targets` | PASS | All targets build clean |
| `cargo clippy --workspace --all-targets --all-features -D warnings -D clippy::unwrap_used` | PASS | Zero lint errors |
| `cargo test -p nono -- machine_policy` | PASS | 36 tests (8 validate_tests + 6 expand_tests + 22 existing) |
| `cargo test -p nono-cli -- daemon_caps wr04` | PASS | 3 new WR-04/WR-05 tests + existing daemon_caps test |
| `cargo test -p nono-cli` | 4 pre-existing failures | profile_cmd + 3 protected_paths (baseline; env-sensitive Windows, documented in MEMORY.md) |
| `cargo clippy --target x86_64-unknown-linux-gnu` | PARTIAL→CI | Cross C compiler unavailable (host lacks x86_64-linux-gnu-gcc); WR-04/WR-05 are `#[cfg(windows)]` so non-Windows branches unaffected |
| `grep -c "S-1-15-2(?:-"` egress-policy-deny.ps1 | 2 | Both SID regex sites unified (IN-03) |
| `grep -c "evil.example.com"` egress-policy-deny.ps1 | 4 | Live proxy probe present (WR-03) |
| WR-03 live proxy probe runtime | PARTIAL→host-gated | Requires provisioned nono-agentd+proxy; structural change verified, not live-run |

## Self-Check: PASSED

- Commits exist: `0021b6c8`, `5cae06c8`, `982a607e`, `0fd38963`, `4af1e8f` — verified via `git log --oneline`
- Files modified: all 5 listed above exist with changes
- Tests: 14 new tests in machine_policy.rs + 3 new tests in agent_daemon/mod.rs = 17 new tests total, all PASS
- No dead `#[allow(dead_code)]` on the canonical expander (confirmed: `grep -A1 allow.dead_code crates/nono-cli/src/policy.rs` returns `#[cfg_attr(not(test)...`), not on the canonical fn in machine_policy.rs)
- No `Command::new("where")` in build_daemon_capability_set (confirmed: grep returns nothing)
- `unwrap_or_else(|_| "C:\\Windows"` fallback removed (confirmed: grep returns nothing)

---
*Quick task: 260625-crs*
*Completed: 2026-06-25*
