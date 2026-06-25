---
phase: quick-260625-crs
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/nono-cli/src/agent_daemon/mod.rs
  - crates/nono/src/machine_policy.rs
  - crates/nono-cli/Cargo.toml
  - scripts/gates/egress-policy-deny.ps1
autonomous: true
requirements: [WR-02, WR-03, WR-04, WR-05, IN-01, IN-03]

must_haves:
  truths:
    - "WR-04: interpreter dirs resolved via SearchPathW (not Command::new('where')) and validated with path-component comparison against a known-safe root"
    - "WR-05: canonicalize failure is a fatal Err; %SystemRoot% absence resolved via GetWindowsDirectoryW, not a hardcoded string fallback"
    - "WR-02: one canonical expander function exists in crates/nono/src/machine_policy.rs; both policy.rs and agent_daemon/mod.rs call it; the dead allow(dead_code) cli fn is removed"
    - "IN-01: MachineEgressPolicy::validate() rejects empty strings, whitespace-only strings, and strings with illegal DNS characters, surfacing PolicyLoadFailed; existing reader calls validate() after parse"
    - "WR-03: egress-policy-deny.ps1 SC-3 block adds a live loopback proxy probe for deny+allow; proxyLayerActive is set from the observed proxy decision"
    - "IN-03: Get-NonoBlockSids and Get-LaunchSid use the same anchored regex S-1-15-2(?:-\\d+)+"
    - "cargo test -p nono passes including new IN-01 validation tests"
    - "cargo test -p nono-cli passes including new WR-02 expander test"
    - "make clippy passes with no warnings (-D warnings -D clippy::unwrap_used)"
  artifacts:
    - path: "crates/nono/src/machine_policy.rs"
      provides: "canonical preset-token expander + MachineEgressPolicy::validate()"
      contains: "pub fn expand_preset_tokens"
    - path: "crates/nono-cli/src/agent_daemon/mod.rs"
      provides: "WR-04/WR-05 fail-secure interpreter resolution; WR-02 call site"
      contains: "SearchPathW"
    - path: "scripts/gates/egress-policy-deny.ps1"
      provides: "live proxy probe (WR-03) + unified SID regex (IN-03)"
      contains: "S-1-15-2(?:-\\d+)+"
  key_links:
    - from: "crates/nono-cli/src/agent_daemon/mod.rs"
      to: "crates/nono/src/machine_policy.rs"
      via: "nono::machine_policy::expand_preset_tokens call"
      pattern: "nono::machine_policy::expand_preset_tokens|machine_policy::expand_preset_tokens"
    - from: "crates/nono-cli/src/policy.rs"
      to: "crates/nono/src/machine_policy.rs"
      via: "nono::machine_policy::expand_preset_tokens call (replaces expand_egress_preset_tokens)"
      pattern: "machine_policy::expand_preset_tokens"
    - from: "crates/nono/src/machine_policy.rs (windows_reader)"
      to: "MachineEgressPolicy::validate()"
      via: "parse_policy calls validate() after deserializing each field"
      pattern: "validate"
---

<objective>
Address all 6 deferred Phase 83 code-review findings: two security fixes in agent_daemon
capability-set construction (WR-04, WR-05), one drift fix collapsing duplicate preset-token
expanders into the core crate (WR-02), one deserialize-time validation layer on
MachineEgressPolicy (IN-01), and two PowerShell gate-script hardening items (WR-03, IN-03).

Purpose: Close the known security gaps (interpreter-dir PATH hijack, canonicalize
fail-open, env-var fallback) and eliminate the dual-expander drift risk before the
codebase drifts further. Config-load failures must be fatal per CLAUDE.md.

Output:
- crates/nono/src/machine_policy.rs gains expand_preset_tokens (canonical, exported) and
  MachineEgressPolicy::validate()
- crates/nono-cli/src/agent_daemon/mod.rs gains SearchPathW-based interpreter resolution and
  fail-secure error paths; calls nono::machine_policy::expand_preset_tokens
- crates/nono-cli/src/policy.rs expand_egress_preset_tokens wired to
  nono::machine_policy::expand_preset_tokens (or removed if unused)
- scripts/gates/egress-policy-deny.ps1 gains live proxy probe and unified SID regex
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md

## Source files

@crates/nono/src/machine_policy.rs
@crates/nono-cli/src/agent_daemon/mod.rs
@crates/nono-cli/src/agent_daemon/launch.rs
@crates/nono-cli/src/policy.rs
@scripts/gates/egress-policy-deny.ps1

<interfaces>
<!-- Key types and contracts the executor needs. -->

From crates/nono/src/machine_policy.rs:
```rust
pub struct MachineEgressPolicy {
    pub allowed_suffixes: Vec<String>,
    pub allowed_hosts: Vec<String>,
    pub preset_tokens: Vec<String>,
    pub telemetry: TelemetryConfig,
}
impl MachineEgressPolicy {
    pub fn raw_allowlist(&self) -> Vec<String>;
    pub fn is_unconfigured(&self) -> bool;
}
// Windows reader (cfg(target_os = "windows")):
pub fn read_machine_egress_policy() -> Result<Option<MachineEgressPolicy>>;
```

From crates/nono-cli/src/agent_daemon/launch.rs (windows_impl):
```rust
pub(crate) use windows_impl::resolve_exe_path;  // uses SearchPathW
// SearchPathW is imported as:
use windows_sys::Win32::Storage::FileSystem::SearchPathW;
```

From crates/nono-cli/src/agent_daemon/mod.rs (existing, to be replaced):
```rust
// WR-04: interpreter resolution via Command::new("where") — INSECURE
let output = std::process::Command::new("where")
    .arg(interp_name.as_str())
    .output();

// WR-05: canonicalize fallback — FAIL-OPEN
let exe_parent_canon = std::fs::canonicalize(exe_parent).unwrap_or_else(|e| {
    tracing::warn!(...);
    exe_parent.to_path_buf()  // <- falls back to unresolved path
});

// WR-05: %SystemRoot% fallback — HARDCODED
let system_root_str = std::env::var("SystemRoot").unwrap_or_else(|_| {
    tracing::warn!(...);
    "C:\\Windows".to_string()  // <- hardcoded fallback
});

// WR-02: daemon-side expander (reads only `hosts`, misses `suffixes`):
fn expand_preset_tokens_from_embedded(tokens: &[String]) -> Result<Vec<String>, String>
```

From crates/nono-cli/src/policy.rs (existing, to be wired or removed):
```rust
// WR-02: CLI-side expander (excludes `suffixes`, currently dead code):
#[allow(dead_code)]
pub fn expand_egress_preset_tokens(tokens: &[String]) -> Result<Vec<String>>
```

The agent_daemon module is loaded by nono-agentd via `#[path]` and cannot reach
crate::policy. The canonical expander MUST live in the nono core crate (already a dep of both bins).

Note: expand_egress_preset_tokens in policy.rs receives the embedded JSON via
`crate::config::embedded::embedded_network_policy_json()`. The daemon's variant
uses `include_str!(concat!(env!("OUT_DIR"), "/network-policy.json"))`. The canonical
version in machine_policy.rs will take `json: &str` as a parameter, keeping the library
policy-free (the caller supplies the JSON, not the library).

The `suffixes` field question (WR-02): the current comment in policy.rs says suffixes are
intentionally excluded from preset expansion because the AI-provider presets use only the
`hosts` wildcard form. The canonical expander should match this: expand only `hosts` (not
`suffixes`) from preset groups, preserving current behavior. Document this choice explicitly.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task A: WR-04 + WR-05 — Fail-secure interpreter resolution in build_daemon_capability_set</name>
  <files>crates/nono-cli/src/agent_daemon/mod.rs, crates/nono-cli/Cargo.toml</files>
  <action>
Fix two security findings in the `#[cfg(target_os = "windows")]` `build_daemon_capability_set`
function in `crates/nono-cli/src/agent_daemon/mod.rs`.

**WR-05 fix — canonicalize failure is fatal (lines ~139-147):**

Replace the `unwrap_or_else` fallback on `std::fs::canonicalize(exe_parent)` with a hard
`?`-propagating error. The fallback was TOCTOU-adjacent (grant based on an unresolved path).
Map the `io::Error` to `NonoError::SandboxInit` with context:
  "build_daemon_capability_set: could not canonicalize exe parent dir {}: {e}"

**WR-05 fix — %SystemRoot% resolution (lines ~158-164):**

Replace `std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())` with a
call to `GetWindowsDirectoryW` via `windows_sys`. The API lives in
`windows_sys::Win32::System::SystemInformation::GetWindowsDirectoryW`. Add
`"Win32_System_SystemInformation"` to the `windows-sys` features list in
`crates/nono-cli/Cargo.toml` (under `[target.'cfg(target_os = "windows")'.dependencies]`).

Implement a private `fn get_windows_directory() -> nono::Result<std::path::PathBuf>`:
- Call `GetWindowsDirectoryW(null_mut(), 0)` to probe the required buffer length.
  SAFETY: probe call with null buffer and zero length is the documented Windows idiom
  for querying required size; no write occurs.
- If probe returns 0, return `Err(NonoError::SandboxInit("GetWindowsDirectoryW probe failed: {os_error}"))`.
- Allocate `Vec<u16>` of that length + 1, call `GetWindowsDirectoryW(buf.as_mut_ptr(), buf_len)`.
  SAFETY: buf is a writable Vec<u16> of the required capacity; GetWindowsDirectoryW writes
  at most buf_len characters including the null terminator.
- If written == 0 or written >= buf_len, return `Err(SandboxInit(...))`.
- Truncate buf to `written as usize`, convert with `OsString::from_wide` (via
  `std::os::windows::ffi::OsStringExt`), return as `PathBuf`.

Replace the `%SystemRoot%` block to call `get_windows_directory()?` and iterate over
`system_root.join("System32")` and `system_root.join("SysWOW64")` as before.

**WR-04 fix — interpreter dir PATH-hijack (lines ~179-231):**

Remove the `Command::new("where")` block entirely. Replace with a call to `resolve_exe_path`
from the sibling `launch` module, which already uses `SearchPathW` (the safe, absolute-path
resolver). Import it via `use super::launch::resolve_exe_path;` (within the `#[cfg(windows)]`
block — this is the same import that `control_loop.rs` already uses).

For each `interp_name` in `interpreter_names`:
1. Construct a `PathBuf::from(interp_name.as_str())` — may be a bare name (e.g. "python.exe")
   or an absolute path.
2. Call `resolve_exe_path(interp_path)`. On `Err`, log at debug level and `continue` (same
   skip-on-not-found semantics as before, since missing interpreters are non-fatal).
3. On `Ok(resolved_exe)`, take `resolved_exe.parent()`. If `None`, log debug and `continue`.
4. **Root-containment check (WR-04, path-component comparison):** Canonicalize the interpreter
   parent dir via `std::fs::canonicalize(interp_dir)`. On canonicalize failure, log
   `tracing::warn!` and `continue` — do NOT grant (fail-secure, consistent with WR-05).

   Build an allowlist of acceptable roots using component-aware `Path::starts_with` (NOT string
   `starts_with`) per CLAUDE.md path security rule. The allowlist is:
   - The canonicalized `%SystemRoot%` tree: call `get_windows_directory()?` (already implemented
     above); canonicalize the result.
   - `%ProgramFiles%`: read `std::env::var("ProgramFiles")`; if set and canonicalize succeeds,
     add it. If unset or canonicalize fails, skip (do not hardcode a fallback).
   - `%ProgramFiles(x86)%`: same pattern as `%ProgramFiles%`.

   // WR-04: Per-user/writable install locations (e.g. %LOCALAPPDATA%\Programs\...) are
   // deliberately excluded from the allowlist — they are the PATH-hijack vector this fix
   // closes. If a legitimately-installed interpreter is rejected, an operator can extend
   // this allowlist here. The check uses Path::starts_with (component-aware, not string
   // comparison) so a path like C:\WindowsEvil is never matched by the C:\Windows root.

   If the canonicalized interpreter dir is under NONE of the allowed roots:
   - `tracing::warn!(interp = %interp_name, dir = %canon_dir.display(), "build_daemon_capability_set: interpreter dir not under a known-safe root (WR-04); skipping grant")`
   - `continue`

5. After the root-containment check passes, call `caps.allow_path(&canon_interp_dir, AccessMode::Read)`.
   On `allow_path` error, return `Err(NonoError::SandboxInit(...))`.

Add a `#[cfg(windows)]` unit test (or extend the existing `daemon_caps_non_empty_for_known_profile`
test) asserting:
- An interpreter dir under `C:\Windows\System32` (a sub-path of the SystemRoot tree) passes the
  root-containment check.
- A synthetic dir under a simulated user-writable root (e.g. a `tempdir()` path NOT under
  SystemRoot/ProgramFiles) is rejected (warn+skip, not granted).
Keep both assertions `#[cfg(target_os = "windows")]`.

This eliminates the PATH-shim attack end-to-end: `SearchPathW` (not `where.exe`) produces the
absolute path, and the subsequent `Path::starts_with` component check confirms it resolves
inside a kernel-managed directory tree that a user-writable PATH entry cannot shadow.

Add a commit with DCO sign-off:
  `git commit -m "fix(agent-daemon): fail-secure interpreter resolution (WR-04, WR-05)" -s`
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  </action>
  <verify>
    <automated>cargo build -p nono-cli 2>&1 | tail -5 && cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used 2>&1 | grep -E "error|warning" | head -20</automated>
  </verify>
  <done>
    - `build_daemon_capability_set` no longer uses `Command::new("where")` or any string-based %SystemRoot% fallback.
    - canonicalize failure on exe_parent is a fatal Err, not a warn+continue.
    - GetWindowsDirectoryW is used to resolve the Windows directory; `unwrap_or_else(|_| "C:\\Windows")` is gone.
    - interpreter dirs resolved via `resolve_exe_path` (SearchPathW-backed), canonicalized, and validated against a `Path::starts_with` (component-aware) allowlist of SystemRoot + ProgramFiles roots; dirs outside the allowlist are warn+skipped (fail-secure).
    - `// WR-04` comment in fn body documents the deliberate exclusion of per-user install paths and the operator extension point.
    - Unit test asserts: SystemRoot sub-path passes; user-writable tempdir path is rejected.
    - `cargo clippy -p nono-cli` passes on native Windows target; cross-target PARTIAL noted below.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task B: WR-02 — Canonical preset-token expander in core nono crate</name>
  <files>crates/nono/src/machine_policy.rs, crates/nono-cli/src/agent_daemon/mod.rs, crates/nono-cli/src/policy.rs</files>
  <behavior>
    - expand_preset_tokens(&["anthropic"], json) where json contains a group "anthropic" with hosts ["*.anthropic.com"] returns Ok(vec!["*.anthropic.com"])
    - expand_preset_tokens(&["unknown-token"], json) returns Ok(vec![]) (fail-secure: unknown token = empty, not an error)
    - expand_preset_tokens(&[], json) returns Ok(vec![])
    - expand_preset_tokens(&["anthropic", "openai"], json) where both groups exist returns the union, sorted and deduplicated
    - expand_preset_tokens(&["tok"], json) where json is malformed returns Err(NonoError::PolicyLoadFailed { .. })
    - expand_preset_tokens with a group that has both `hosts` and `suffixes` only expands `hosts` (suffixes excluded — per the documented AI-provider preset convention; document this in the fn docstring)
  </behavior>
  <action>
**Step 1: Add `expand_preset_tokens` to `crates/nono/src/machine_policy.rs`**

Add a new public function below the `MachineEgressPolicy` impl block:

```
pub fn expand_preset_tokens(tokens: &[String], network_policy_json: &str) -> Result<Vec<String>>
```

- If `tokens.is_empty()` return `Ok(Vec::new())`.
- Parse `network_policy_json` as `serde_json::Value`. On parse failure, return
  `Err(NonoError::PolicyLoadFailed { reason: format!("expand_preset_tokens: failed to parse network-policy.json: {e}") })`.
- Extract `root["groups"]` as an object; if absent, return
  `Err(NonoError::PolicyLoadFailed { reason: "expand_preset_tokens: network-policy.json missing 'groups' object".to_string() })`.
- For each token:
  - If a group is found: extend `result` with each string in `group["hosts"]` array.
    Do NOT include `group["suffixes"]` — the AI-provider presets use only the wildcard-host
    form `*.domain` (document this decision with a comment: "Suffixes are intentionally
    excluded from preset expansion: AI-provider groups express wildcard coverage as
    `*.domain` entries in `hosts`, not as `suffixes`. Including `suffixes` would double-add
    them when the caller also includes `MachineEgressPolicy::raw_allowlist()` suffixes.").
  - If group not found: log at `tracing::debug!` level; do NOT error (unknown token = empty
    expansion, fail-secure per T-83-token-widen).
- Sort and dedup the result vec.
- Return `Ok(result)`.

`serde_json` is already a dependency of the `nono` crate (verify with `grep serde_json
crates/nono/Cargo.toml`). No new deps needed.

The function signature takes `network_policy_json: &str` to keep the library policy-free:
the caller supplies the JSON bytes; the library does not embed or load files.

**Step 2: Write tests in the `#[cfg(test)]` block of machine_policy.rs**

Add a `mod expand_tests` submodule inside `#[cfg(test)]` covering all 6 behavior cases
listed in the `<behavior>` block above. Use inline JSON strings for the network-policy
fixture; do not read files in tests. Example fixture:
```json
{"groups":{"anthropic":{"hosts":["*.anthropic.com"]},"openai":{"hosts":["*.openai.com"],"suffixes":[".openai.com"]}}}
```

**Step 3: Wire the daemon call site in `crates/nono-cli/src/agent_daemon/mod.rs`**

In `expand_preset_tokens_from_embedded`:
- Replace the manual serde_json::Value parsing + host-only loop with a call to
  `nono::machine_policy::expand_preset_tokens(tokens, EMBEDDED_NETWORK_POLICY_JSON)`.
- Map the `Err(NonoError::PolicyLoadFailed { reason })` to the caller's expected
  `Result<Vec<String>, String>` by calling `.map_err(|e| e.to_string())`.
- The function docstring must still note it operates on `EMBEDDED_NETWORK_POLICY_JSON`
  and delegates to the canonical expander. Remove the "Mirrors crate::policy" wording.
- The body shrinks to ~5 lines.

**Step 4: Wire the CLI call site in `crates/nono-cli/src/policy.rs`**

In `expand_egress_preset_tokens`:
- Replace the body with:
  ```
  let json = crate::config::embedded::embedded_network_policy_json();
  nono::machine_policy::expand_preset_tokens(tokens, json)
  ```
- Remove the `#[allow(dead_code)]` attribute — the function is no longer dead (it now
  delegates to the canonical implementation). If the function is still not called from
  non-test code after this, leave the attribute removal in place and add a note in the
  docstring; do NOT re-add `#[allow(dead_code)]` (the original finding was about that
  attribute existing alongside a duplicate expander; removing the duplication removes
  the justification for the attribute).

Add a commit with DCO sign-off:
  `git commit -m "refactor(policy): canonical preset-token expander in core crate (WR-02)" -s`
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  </action>
  <verify>
    <automated>cargo test -p nono -- machine_policy::expand_tests 2>&1 | tail -20</automated>
  </verify>
  <done>
    - `nono::machine_policy::expand_preset_tokens` exists and is `pub`.
    - `expand_preset_tokens_from_embedded` in agent_daemon/mod.rs delegates to it.
    - `expand_egress_preset_tokens` in policy.rs delegates to it; `#[allow(dead_code)]` removed.
    - All 6 test cases in `expand_tests` pass.
    - `cargo test -p nono` passes.
    - `cargo test -p nono-cli` passes.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task C: IN-01 — Deserialize-time validation on MachineEgressPolicy</name>
  <files>crates/nono/src/machine_policy.rs</files>
  <behavior>
    - MachineEgressPolicy::validate() on a policy with allowed_hosts = [""] returns Err(NonoError::PolicyLoadFailed { reason contains "empty" })
    - validate() on a policy with allowed_suffixes = ["   "] (whitespace-only) returns Err containing "empty" or "whitespace"
    - validate() on a policy with preset_tokens = ["bad token!"] (contains space, exclamation) returns Err containing "invalid"
    - validate() on a policy with preset_tokens = ["-leading-dash"] returns Err containing "invalid" (leading hyphen is not a valid DNS label start)
    - validate() on a valid policy (allowed_hosts=["api.github.com"], allowed_suffixes=["*.anthropic.com"], preset_tokens=["anthropic"]) returns Ok(())
    - validate() on MachineEgressPolicy::default() returns Ok(()) (empty lists are valid — no entries is unconfigured, not invalid)
    - The Windows reader (parse_policy) calls policy.validate() after constructing the struct and maps Err to Err(NonoError::PolicyLoadFailed)
    - A preset token "anthropic" (alphanumeric + hyphen) is valid; "anthropic_ai" (underscore) is invalid (not a DNS label character)
  </behavior>
  <action>
Add `pub fn validate(&self) -> Result<()>` to `impl MachineEgressPolicy` in
`crates/nono/src/machine_policy.rs`.

**Validation rules (IN-01):**

1. For each entry in `self.allowed_hosts`:
   - Trim the entry. If trimmed is empty, return
     `Err(NonoError::PolicyLoadFailed { reason: "allowed_hosts contains an empty or whitespace-only entry" })`.
   - Basic DNS sanity: every character must be ASCII alphanumeric, `-`, `.`, or `*`
     (wildcards are allowed for host entries that got normalized). If any character fails
     this check, return `Err(PolicyLoadFailed { reason: "allowed_hosts entry '{entry}' contains invalid characters (expected DNS label characters: a-z 0-9 - . *)" })`.

2. For each entry in `self.allowed_suffixes`:
   - Same empty/whitespace check.
   - Same character-set check (suffixes include `*.` prefix after normalization, so `*` and `.` are valid).

3. For each entry in `self.preset_tokens`:
   - Trim. If empty, return `Err(PolicyLoadFailed { reason: "preset_tokens contains an empty or whitespace-only entry" })`.
   - Preset tokens are group-name identifiers, not FQDNs. Valid characters: ASCII alphanumeric and `-`. No dots, no spaces, no underscores. If any character fails this check, return `Err(PolicyLoadFailed { reason: "preset_tokens entry '{token}' contains invalid characters (expected alphanumeric and hyphen only)" })`.
   - A leading `-` is also invalid (not a valid label start). Check `token.starts_with('-')` after trimming; return the same invalid-character error if true.

Do NOT validate that hosts are resolvable or that tokens exist in the embedded JSON — that
would make the library policy-aware. The validate() method checks only structural sanity.

**Wire into the Windows reader:**

In `windows_reader::parse_policy` (at the end, after constructing `MachineEgressPolicy`):
```rust
policy.validate().map_err(|e| match e {
    NonoError::PolicyLoadFailed { reason } => reason,
    other => other.to_string(),
})?;
```
This propagates as `Err(reason)` which `read_machine_egress_policy_impl` then wraps in
`Err(NonoError::PolicyLoadFailed { reason })` via the existing `.map_err(|reason| NonoError::PolicyLoadFailed { reason })` chain.

Since `validate()` only calls `Result::Err` with `PolicyLoadFailed` variants, the
`map_err` chain remains internally consistent.

**Tests:**

Add a `mod validate_tests` submodule in `#[cfg(test)]` covering all 8 behavior cases.
Do NOT gate these tests with `#[cfg(target_os = "windows")]` — validate() is platform-neutral.

Add a commit with DCO sign-off:
  `git commit -m "fix(machine-policy): deserialize-time validation on egress policy fields (IN-01)" -s`
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  </action>
  <verify>
    <automated>cargo test -p nono -- machine_policy::validate_tests 2>&1 | tail -20</automated>
  </verify>
  <done>
    - `MachineEgressPolicy::validate()` exists and is `pub`.
    - Empty/whitespace entries in any list field return `PolicyLoadFailed`.
    - Non-DNS-characters in host/suffix entries return `PolicyLoadFailed`.
    - Invalid characters in preset tokens (space, `_`, `!`) return `PolicyLoadFailed`.
    - Valid policy and empty policy return `Ok(())`.
    - `parse_policy` (Windows reader) calls `validate()` and maps the error.
    - All 8 test cases pass.
  </done>
</task>

<task type="auto">
  <name>Task D: WR-03 + IN-03 — Gate-script live proxy probe and unified SID regex</name>
  <files>scripts/gates/egress-policy-deny.ps1</files>
  <action>
Two fixes to `scripts/gates/egress-policy-deny.ps1`.

**IN-03 fix — unified SID regex (lines ~92, 101):**

In `Get-NonoBlockSids` (line ~92), change:
  `'(S-1-15-2-[\d-]+)'`
to:
  `'(S-1-15-2(?:-\d+)+)'`

In `Get-LaunchSid` (line ~101), change:
  `'sid=(S-1-15-2[^\s]+)'`
to:
  `'sid=(S-1-15-2(?:-\d+)+)'`

Both now use the same anchored pattern `S-1-15-2(?:-\d+)+` which requires at least one
`-digit` segment and rejects trailing non-digit garbage. Add a comment above each:
`# IN-03: unified SID regex; matches AppContainer package SIDs (one or more -NNN segments)`

**WR-03 fix — live loopback proxy probe in SC-3 (lines ~381-399):**

The existing SC-3 logic sets `$proxyLayerActive = $wfpBlockPresent` (structural proof only).
Replace/augment this block to add a live proxy probe:

After the `$wfpBlockPresent` calculation (after `$afterLaunch` assignment, around line ~379),
add the following PowerShell logic:

```powershell
# WR-03: live proxy probe — assert deny for an out-of-list host and allow for a listed host.
# HOST-GATED: this probe requires a provisioned host with nono-agentd + proxy running.
# If the proxy port is not available (no daemon), the probe is skipped and proxyLayerActive
# is set from structural WFP evidence only (same as before). The probe is NOT optional when
# the daemon IS running; a running proxy that fails the probe is a FAIL.
$proxyLayerActive = $false
$proxyProbeSkipped = $false
$proxyPort = $null

# Extract the proxy port from the launch response (daemon prints "proxy_port=NNNN").
if ($respSC3 -match 'proxy_port=(\d+)') {
    $proxyPort = [int]$Matches[1]
}

if ($null -ne $proxyPort -and $proxyPort -gt 0) {
    # Probe 1: CONNECT to out-of-list host (e.g. evil.example.com:443) → expect deny (non-200 or connection refused).
    $denyProbePass = $false
    try {
        $denyResp = Invoke-WebRequest -Uri "http://127.0.0.1:$proxyPort" `
            -Method 'CONNECT' `
            -Headers @{ Host = 'evil.example.com:443' } `
            -TimeoutSec 5 `
            -ErrorAction SilentlyContinue
        # A 200 response means the proxy allowed the CONNECT — that is a FAIL.
        $denyProbePass = ($null -eq $denyResp -or $denyResp.StatusCode -ne 200)
    } catch {
        # Connection refused or 4xx/5xx from proxy → deny confirmed.
        $denyProbePass = $true
    }

    # Probe 2: CONNECT to allowed host (api.anthropic.com:443) → expect allow (200 or TCP-established).
    $allowProbePass = $false
    try {
        $allowResp = Invoke-WebRequest -Uri "http://127.0.0.1:$proxyPort" `
            -Method 'CONNECT' `
            -Headers @{ Host = 'api.anthropic.com:443' } `
            -TimeoutSec 5 `
            -ErrorAction SilentlyContinue
        $allowProbePass = ($null -ne $allowResp -and $allowResp.StatusCode -eq 200)
    } catch [System.Net.WebException] {
        # A TCP-tunnel establishment followed by TLS failure may throw; that still
        # means the proxy accepted the CONNECT (tunnel opened). Inspect the exception.
        $allowProbePass = ($_.Exception.Response -ne $null -and
                           [int]$_.Exception.Response.StatusCode -eq 200) -or
                          ($_.Exception.Message -match '200|tunnel|established')
    } catch {
        $allowProbePass = $false
    }

    $proxyLayerActive = $denyProbePass -and $allowProbePass
    $detail['proxyDenyProbePass'] = $denyProbePass
    $detail['proxyAllowProbePass'] = $allowProbePass
    $detail['proxyPort'] = $proxyPort
} else {
    # No proxy port in launch response — daemon not running or not in proxy mode.
    # Fall back to structural proof: WFP block implies proxy-only mode was activated.
    $proxyLayerActive = $wfpBlockPresent
    $proxyProbeSkipped = $true
    $detail['proxyProbeSkipped'] = $true
    $detail['proxyProbeSkipReason'] = 'proxy_port not found in launch response; structural WFP proof used'
}
```

Update the `$detail` hash and the PASS/FAIL reason strings to reflect the probe results.
The PASS reason for SC-3 should read: "SC-3 proven: per-SID WFP block filter present AND
live proxy probe confirmed deny for evil.example.com + allow for api.anthropic.com (dual-layer
enforce wired)." — OR when probe was skipped: "SC-3 proven structurally: per-SID WFP block
filter present (proxy probe skipped: no proxy_port in daemon response)."

The existing final `if ($sc2Pass -and $wfpBlockPresent -and $proxyLayerActive)` gate remains.
The probe skipped case still passes the gate when `$wfpBlockPresent` is true (structural
proof is maintained as a fallback for hosts without the daemon running during gate execution).

**HOST-GATED notice:** Add a prominent comment at the top of the SC-3 section:
```
# HOST-GATED (WR-03): live proxy probe added. Full verification requires a provisioned
# host with nono-agentd + proxy running. This gate cannot be live-verified on a dev-only
# host (no daemon). Probe is skipped automatically when proxy_port is absent.
```

Add a commit with DCO sign-off:
  `git commit -m "fix(gates): live proxy probe + unified SID regex (WR-03, IN-03)" -s`
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  </action>
  <verify>
    <automated>powershell -File scripts/gates/egress-policy-deny.ps1 --help 2>&1 | head -5; grep -c "S-1-15-2(?:-\\d+)+" scripts/gates/egress-policy-deny.ps1</automated>
    <!-- WR-03 live probe: HOST-GATED — cannot be run on this dev host without nono-agentd running. -->
    <!-- Mark WR-03 runtime verification as PARTIAL in the SUMMARY: implemented, not live-run. -->
    <!-- IN-03 regex unification is statically verifiable by grep. -->
  </verify>
  <done>
    - Both SID-matching regexes use `S-1-15-2(?:-\d+)+` (confirmed by grep).
    - SC-3 block contains a live proxy probe for deny (evil.example.com) and allow (api.anthropic.com).
    - `proxyLayerActive` is set from the observed probe result when proxy_port is available.
    - Fallback to structural WFP proof when proxy_port is absent (host-gated skip path).
    - HOST-GATED comment present at SC-3 block header.
    - WR-03 runtime verification marked PARTIAL→host-gated in SUMMARY.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| PATH env → SearchPathW | Interpreter resolution crosses from user-controlled PATH into the system resolver; WR-04 moves this to the SearchPathW kernel path |
| Registry (HKLM) → MachineEgressPolicy | Admin-written registry values deserialized into policy; IN-01 adds input validation at this boundary |
| Proxy loopback probe → proxy process | Gate script sends CONNECT requests to the in-process proxy; no new trust crossing (loopback only) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-crs-01 | Elevation of Privilege | build_daemon_capability_set / where.exe | mitigate | WR-04: replace Command::new("where") with SearchPathW via resolve_exe_path; SearchPathW is called without a custom lpPath so it uses the system's secure search order |
| T-crs-02 | Tampering | build_daemon_capability_set / canonicalize fallback | mitigate | WR-05: canonicalize failure → fatal Err instead of silent path fallback; eliminates TOCTOU-adjacent grant on unresolved path |
| T-crs-03 | Tampering | build_daemon_capability_set / %SystemRoot% env var | mitigate | WR-05: GetWindowsDirectoryW replaces env::var("SystemRoot") fallback; GetWindowsDirectoryW is not spoofable via environment |
| T-crs-04 | Tampering | MachineEgressPolicy / HKLM registry inputs | mitigate | IN-01: validate() rejects empty, whitespace-only, and non-DNS-character entries; PolicyLoadFailed aborts startup on any violation |
| T-crs-05 | Spoofing | egress-policy-deny.ps1 / SID regex divergence | mitigate | IN-03: unified S-1-15-2(?:-\d+)+ regex in both functions; anchored pattern rejects non-numeric trailing segments |
| T-crs-SC | Tampering | npm/pip/cargo installs | accept | No new package installs in this plan; windows-sys feature extension only (existing dep) |
</threat_model>

<verification>
Run these checks in order after all four tasks are committed:

```
# 1. Build (all targets)
make build

# 2. Strict clippy (native Windows host)
make clippy

# 3. Core library tests (includes IN-01 validate_tests + WR-02 expand_tests)
cargo test -p nono

# 4. CLI tests (includes WR-02 expander delegation + WR-04/WR-05 compile check)
cargo test -p nono-cli

# 5. Cross-target clippy (CLAUDE.md MUST/NEVER)
# WR-04/WR-05 are cfg(target_os = "windows") — Linux/macOS targets will not compile
# the changed block. Verify that the non-Windows cfg branches still compile cleanly.
cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
# If cross toolchain unavailable: mark PARTIAL→CI per .planning/templates/cross-target-verify-checklist.md

# 6. SID regex verification (IN-03)
grep -c "S-1-15-2(?:-\\\\d+)+" scripts/gates/egress-policy-deny.ps1
# Must return 2 (one in Get-NonoBlockSids, one in Get-LaunchSid)

# 7. WR-03 probe presence (structural check only — no live daemon on dev host)
grep -c "evil.example.com" scripts/gates/egress-policy-deny.ps1
# Must return >= 1

# 8. Confirm no dead_code allow on expand_egress_preset_tokens
grep -A1 "allow(dead_code)" crates/nono-cli/src/policy.rs | grep "expand_egress_preset"
# Must return nothing (attribute removed)
```
</verification>

<success_criteria>
- WR-04: `build_daemon_capability_set` interpreter resolution uses `resolve_exe_path` (SearchPathW-backed); `Command::new("where")` is gone; interpreter dirs validated via `Path::starts_with` (component-aware) against a SystemRoot + ProgramFiles allowlist; dirs outside the allowlist are warn+skipped (fail-secure).
- WR-05: canonicalize failure on exe_parent is a fatal `Err`; `%SystemRoot%` is resolved via `GetWindowsDirectoryW`; no `unwrap_or_else` fallbacks on either.
- WR-02: `nono::machine_policy::expand_preset_tokens` is the canonical implementation; `expand_preset_tokens_from_embedded` and `expand_egress_preset_tokens` both delegate to it; `#[allow(dead_code)]` on the CLI fn is removed.
- IN-01: `MachineEgressPolicy::validate()` rejects empty/whitespace/non-DNS entries with `PolicyLoadFailed`; `parse_policy` calls it; 8 unit tests pass.
- WR-03: SC-3 block in egress-policy-deny.ps1 contains a live proxy probe for deny (evil.example.com) and allow (api.anthropic.com); runtime verification marked PARTIAL→host-gated.
- IN-03: Both SID regex sites use `S-1-15-2(?:-\d+)+`; `grep -c` returns 2.
- `make build` passes; `make clippy` passes; `cargo test -p nono` passes; `cargo test -p nono-cli` passes.
- Cross-target clippy: native green; PARTIAL→CI if cross toolchain unavailable.
- Each task committed separately with DCO sign-off `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
</success_criteria>

<output>
Create `.planning/quick/260625-crs-address-phase-83-code-review-deferred-fi/260625-crs-SUMMARY.md` when done.
</output>
