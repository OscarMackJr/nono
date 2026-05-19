# Phase 37: Linux RESL backends + PKGS auto-pull - Pattern Map

**Mapped:** 2026-05-19
**Files analyzed:** 8 (3 new, 5 modified)
**Analogs found:** 8 / 8

## File Classification

| New / Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------------|------|-----------|----------------|---------------|
| `crates/nono/src/error.rs` (NEW variant `UnsupportedKernelFeature`) | library error variant + unit test | typed-value carrier | `NotSupportedOnPlatform { feature }` variant (same file, lines 42-52) + `BrokerNotFound { path }` unit test block (same file, lines 374-402) | exact (precedent variant added in Phase 25-01) |
| `bindings/c/src/lib.rs` (NEW match arm in `map_error`) | FFI error mapping | enum-to-code translation | `NotSupportedOnPlatform { .. } => NonoErrorCode::ErrUnsupportedPlatform` arm (same file, lines 139-142) | exact (same Phase 25-01 precedent) |
| `crates/nono-cli/src/cli.rs` (NEW `ProfileResolverArgs` struct + flatten into `RunArgs` + `WrapArgs`) | CLI args group | clap derive + flatten | `SandboxArgs` struct (same file, line 1469) flattened into `RunArgs` line 2080, `ShellArgs` line 2258, `WrapArgs` line 2277 | exact (same `#[command(flatten)]` shape) |
| `crates/nono-cli/src/profile/mod.rs` (NEW `ResolveContext` + thread through `load_profile` / `load_registry_profile`) | dispatch / context-param threading | request-response (sync, in-process) | Current `load_profile(name_or_path: &str)` signature (line 2178) — wrap as `load_profile_with_context(name, &ResolveContext)`; existing `load_profile()` becomes a thin wrapper | role-match (no exact "context param threading" precedent in this codebase — wrapper-with-default is the cleanest fit) |
| `.github/workflows/phase-37-linux-resl.yml` (NEW) | CI workflow | event-driven (PR / push) | `.github/workflows/ci.yml::test` job (lines 89-125) for `cargo test` shape | role-match (existing job uses `ubuntu-latest`; D-01 pins `ubuntu-24.04`; existing has no systemd / `loginctl` setup, so add the Pattern-5 (research) extension) |
| `crates/nono-cli/tests/auto_pull_e2e_linux.rs` (NEW) | integration test (Linux-gated) | request-response over TCP + subprocess | `crates/nono-cli/tests/resl_nix_linux.rs` (Linux-gated harness with `NONO_BIN = env!("CARGO_BIN_EXE_nono")` + `Command::new(NONO_BIN).args([...]).output()`) + `registry_client::tests::spawn_one_shot_server` (`crates/nono-cli/src/registry_client.rs:367-412`) for the std-only TCP mock | exact (both analogs live in the same crate; D-14 explicitly inherits the std-only TCP pattern) |
| `crates/nono-cli/src/session_commands.rs` (MODIFIED — string fix in `run_inspect`, lines 546-565) | display / formatter | data-transform (`SessionRecord` → stdout) | The existing `run_inspect` block itself (same file, lines 513-568) is the analog for shape; `#[cfg(target_os = "linux")]` cfg-gating precedent is in `crates/nono/src/error.rs:67-73` (Landlock cfg-gate) and in `crates/nono-cli/src/profile/mod.rs:2314` (`#[cfg(not(target_os = "windows"))]`) | role-match (mechanical string swap + add cfg gate; mirror in `session_commands_windows.rs:540,543,552`) |
| `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (MODIFIED — 4-of-5 `UnsupportedPlatform("cgroup_v2: ...")` swap to `UnsupportedKernelFeature`) | error construction site | enum construction | The existing `UnsupportedPlatform` call sites in the same `cgroup` submodule (lines 880-884, 889-893, 896-900, 957-961, 966-968, 969-971) ARE the analog (mechanical refactor of error-construction shape with site #4 at 930-934 intentionally KEPT) | exact (the swap target IS the analog) |

## Pattern Assignments

### `crates/nono/src/error.rs` — NEW `UnsupportedKernelFeature` variant

**Role:** library error variant + unit test
**Analog:** `NotSupportedOnPlatform { feature }` variant in the same file (lines 42-52) added by Phase 25-01.

**Variant declaration pattern** (`crates/nono/src/error.rs:42-52`):
```rust
/// A feature is not supported on this specific platform.
///
/// This is distinct from [`UnsupportedPlatform`] in that the platform itself
/// is supported, but a specific feature within that platform is not available.
/// For example, `--cpu-percent` is not supported on macOS because there is no
/// per-process CPU-quota equivalent, but nono itself runs fine on macOS.
///
/// The `feature` field contains a stable machine-readable identifier (e.g.
/// `"cpu_percent_macos"`) that tests and callers can match on.
#[error("Feature not supported on this platform: {feature}")]
NotSupportedOnPlatform { feature: String },
```

**Copy verbatim:** the `#[error("...")]` derive shape, doc-comment style ("distinct from X in that Y"), and structured-field convention. The new variant has TWO fields per D-05 (`feature: String, hint: String`) — both `String` for FFI-string compatibility (no `PathBuf` / `io::Error` because the failure is environmental, not path-bound). Final Display string format per research Pattern 1 + D-05:
```rust
#[error("Kernel feature not supported: {feature} ({hint})")]
UnsupportedKernelFeature { feature: String, hint: String },
```

**Unit test pattern** (`crates/nono/src/error.rs:374-402`):
```rust
#[cfg(test)]
mod broker_not_found_tests {
    use super::NonoError;
    use std::path::PathBuf;

    /// Phase 31 D-07: BrokerNotFound display surfaces the resolved path so
    /// operators can see exactly which sibling lookup failed.
    #[test]
    fn broker_not_found_displays_path() {
        let err = NonoError::BrokerNotFound {
            path: PathBuf::from("/tmp/missing-broker.exe"),
        };
        let s = err.to_string();
        assert!(
            s.contains("missing-broker.exe"),
            "BrokerNotFound display should include the path; got: {s}"
        );
    }

    #[test]
    fn broker_not_found_is_debug() {
        let err = NonoError::BrokerNotFound {
            path: PathBuf::from("foo.exe"),
        };
        let _ = format!("{err:?}");
    }
}
```

**Copy for Plan 37-01:** sibling `#[cfg(test)] mod unsupported_kernel_feature_tests { ... }` block. Test names per research Validation Architecture line 750: `unsupported_kernel_feature_display_contains_cgroup_no_v1_hint`. Required assertions: Display contains `"cgroup_v2"` (the `feature` field), contains `"cgroup_no_v1=all"` (the LOCKED D-07 hint substring), starts with `"Kernel feature not supported:"`, AND is pattern-matchable via `matches!(err, NonoError::UnsupportedKernelFeature { .. })` (mirror `action_required_is_pattern_matchable` at lines 324-335).

**Test-module attribute:** `#[cfg(test)]` + `#[allow(clippy::unwrap_used)]` (see the existing `tests` block at lines 270-271) — clippy `unwrap_used` is workspace-strict in production but permitted in test modules per CLAUDE.md.

---

### `bindings/c/src/lib.rs` — NEW match arm in `map_error`

**Role:** FFI error mapping
**Analog:** the existing `NotSupportedOnPlatform { .. }` arm in the same exhaustive `match` (lines 139-142) — also added by Phase 25-01.

**Exhaustive-match shape** (`bindings/c/src/lib.rs:72-144`, abbreviated):
```rust
pub(crate) fn map_error(e: &nono::NonoError) -> types::NonoErrorCode {
    use types::NonoErrorCode;
    set_last_error(&e.to_string());
    match e {
        nono::NonoError::PathNotFound(_) => NonoErrorCode::ErrPathNotFound,
        // ... many arms ...
        nono::NonoError::UnsupportedPlatform(_) => NonoErrorCode::ErrUnsupportedPlatform,
        // ...
        // Phase 25-01: platform-specific feature rejection (e.g., --cpu-percent on macOS).
        // Maps to ErrUnsupportedPlatform so FFI consumers see the same code as
        // UnsupportedPlatform but with a structured feature field in the message.
        nono::NonoError::NotSupportedOnPlatform { .. } => NonoErrorCode::ErrUnsupportedPlatform,
    }
}
```

**Function-level invariant** (comment block at lines 67-71):
> "Every `NonoError` variant is matched explicitly so the compiler will flag new variants that need a mapping, instead of silently falling through to `ErrUnknown`."

**Copy for Plan 37-01:** append the new arm at the end of the `match`, mirroring the existing arm's comment style (a 2–3-line `// Phase 37 D-06: ...` justifier referencing the decision). Per D-06, reuse `NonoErrorCode::ErrUnsupportedPlatform`:
```rust
// Phase 37 D-06: kernel feature missing because the OS is misconfigured
// (cgroup v1 instead of v2). Reuses ErrUnsupportedPlatform per D-06; the
// FFI consumer reads the typed feature+hint via nono_last_error() Display
// string. NO new NonoErrorCode is added (ABI-stable).
nono::NonoError::UnsupportedKernelFeature { .. } => NonoErrorCode::ErrUnsupportedPlatform,
```

**Workspace-cargo-check gate:** if this arm is missing, `cargo check --workspace` fails with `non-exhaustive patterns: NonoError::UnsupportedKernelFeature { .. } not covered`. Plan 37-01 must land both surfaces (error.rs + bindings/c/src/lib.rs) in the same commit per the "3-surface touch" pattern documented in research Pattern 1.

---

### `crates/nono-cli/src/cli.rs` — NEW `ProfileResolverArgs` struct

**Role:** CLI args group (shared between subcommands)
**Analog:** `SandboxArgs` struct (line 1469) — already flattened into `RunArgs` (line 2080), `ShellArgs` (line 2258), and `WrapArgs` (line 2277).

**Struct declaration pattern** (`crates/nono-cli/src/cli.rs:1468-1469`):
```rust
#[derive(Parser, Debug, Clone, Default)]
pub struct SandboxArgs {
    // ── Filesystem ───────────────────────────────────────────────────────
    /// Allow read+write access to a directory (recursive)
    #[arg(
        long,
        short = 'a',
        value_name = "DIR",
        env = "NONO_ALLOW",
        value_delimiter = ',',
        help_heading = "FILESYSTEM"
    )]
    pub allow: Vec<PathBuf>,
    // ...
}
```

**Env-var argument pattern (same file, `--capability-elevation` example at lines 2189-2194):**
```rust
/// Enable runtime capability elevation (seccomp-notify + approval prompts).
/// Overrides the profile's capability_elevation setting.
/// When enabled, the supervisor can grant access to paths not in the
/// initial capability set via interactive prompts.
#[arg(long, env = "NONO_CAPABILITY_ELEVATION", help_heading = "OPTIONS")]
pub capability_elevation: bool,
```

This is the exact analog for `--no-auto-pull` (clap `env = "..."` annotation gives "CLI > env > default" precedence automatically per D-10).

**Flatten-usage pattern** (`crates/nono-cli/src/cli.rs:2079-2082`):
```rust
#[derive(Parser, Debug)]
#[command(disable_help_flag = true)]
pub struct RunArgs {
    #[command(flatten)]
    pub sandbox: SandboxArgs,
    // ... other fields ...
}
```

Identical at `WrapArgs` (line 2276-2278):
```rust
#[derive(Parser, Debug)]
#[command(disable_help_flag = true)]
pub struct WrapArgs {
    #[command(flatten)]
    pub sandbox: WrapSandboxArgs,
    // ...
}
```

**Copy for Plan 37-02:** declare a new struct (per research Pattern 3):
```rust
#[derive(Parser, Debug, Clone, Default)]
pub struct ProfileResolverArgs {
    /// Disable cargo-install-style auto-pull when --profile references a
    /// registry pack not yet installed locally. Falls back to the legacy
    /// "profile not found" error.
    #[arg(long, env = "NONO_NO_AUTO_PULL", help_heading = "PROFILE")]
    pub no_auto_pull: bool,
}
```

Then add `#[command(flatten)] pub profile_resolver: ProfileResolverArgs,` to BOTH `RunArgs` (after line 2081) and `WrapArgs` (after line 2278). Per D-09, do NOT add to `PullArgs` (line 1098) — `nono pull` is explicit-install and the flag is meaningless there.

**Help-heading discipline:** the existing struct uses `help_heading = "FILESYSTEM" | "OPTIONS" | "RESOURCE LIMITS" | "ROLLBACK" | "QUERY"`. The new flag introduces a `"PROFILE"` heading; verify the existing `.github/scripts/check-cli-doc-flags.sh` (referenced in `ci.yml` line 79) accepts a new heading or update its allow-list in the same plan.

---

### `crates/nono-cli/src/profile/mod.rs` — `ResolveContext` parameter threading

**Role:** dispatch / context-param threading
**Analog:** the existing `load_profile(name_or_path: &str) -> Result<Profile>` (line 2178) — call sites in `app_runtime.rs` / `launch_runtime.rs` / `wrap` handlers all hit this single entry point.

**Existing dispatch shape** (`crates/nono-cli/src/profile/mod.rs:2178-2212`):
```rust
pub fn load_profile(name_or_path: &str) -> Result<Profile> {
    // Registry reference (namespace/name) — detect before the file path check
    if is_registry_ref(name_or_path) {
        return load_registry_profile(name_or_path);
    }

    // Direct file path: contains separator or ends with .json
    if name_or_path.contains('/') || name_or_path.ends_with(".json") {
        return load_profile_from_path(Path::new(name_or_path));
    }
    // ... name validation, user-profile lookup, built-in fallback ...
    Err(NonoError::ProfileNotFound(name_or_path.to_string()))
}
```

**Existing auto-pull dispatcher** (`crates/nono-cli/src/profile/mod.rs:2230-2247`):
```rust
fn load_registry_profile(name_or_path: &str) -> Result<Profile> {
    let package_ref = crate::package::parse_package_ref(name_or_path)?;
    let install_dir =
        crate::package::package_install_dir(&package_ref.namespace, &package_ref.name)?;

    // Check if pack is already installed
    if !install_dir.join("package.json").exists() {
        eprintln!("Profile '{}' not found locally.", package_ref.key());

        // Auto-pull from registry
        crate::package_cmd::run_pull(crate::cli::PullArgs {
            package_ref: name_or_path.to_string(),
            registry: None,
            force: false,
            init: false,
            help: None,
        })?;
    }
    // ... manifest read + artifact discovery ...
}
```

**Copy for Plan 37-02** (per research Pattern 3):
```rust
#[derive(Debug, Clone, Default)]
pub struct ResolveContext {
    pub no_auto_pull: bool,
}

pub fn load_profile_with_context(
    name_or_path: &str,
    ctx: &ResolveContext,
) -> Result<Profile> {
    if is_registry_ref(name_or_path) {
        return load_registry_profile_with_context(name_or_path, ctx);
    }
    // ... identical to existing load_profile body ...
}

// Keep existing entry point as a thin wrapper for callers that don't care.
pub fn load_profile(name_or_path: &str) -> Result<Profile> {
    load_profile_with_context(name_or_path, &ResolveContext::default())
}

fn load_registry_profile_with_context(
    name_or_path: &str,
    ctx: &ResolveContext,
) -> Result<Profile> {
    let package_ref = crate::package::parse_package_ref(name_or_path)?;
    let install_dir =
        crate::package::package_install_dir(&package_ref.namespace, &package_ref.name)?;

    if !install_dir.join("package.json").exists() {
        if ctx.no_auto_pull {
            // D-11: fall back to ProfileNotFound; the supervisor's DiagnosticFormatter
            // (separate concern) adds the footer noting --no-auto-pull is set.
            return Err(NonoError::ProfileNotFound(name_or_path.to_string()));
        }
        eprintln!("Profile '{}' not found locally.", package_ref.key());
        crate::package_cmd::run_pull(crate::cli::PullArgs { /* ... */ })?;
    }
    // ... existing manifest + artifact logic ...
}
```

**Call-site update:** wherever `load_profile(&args.sandbox.profile)` (or equivalent) is invoked from `RunArgs` / `WrapArgs` handlers, switch to `load_profile_with_context(&args.sandbox.profile, &ResolveContext { no_auto_pull: args.profile_resolver.no_auto_pull })`. Grep for `load_profile(` call sites before editing.

**Anti-pattern rejection** (per research Anti-Patterns line 458): MUST be struct-parameter only — never a thread-local / `static AtomicBool` / global. Globals break parallel test isolation in Rust's test runner (CLAUDE.md "tests in parallel within the same process" rule).

---

### `.github/workflows/phase-37-linux-resl.yml` — NEW workflow file

**Role:** CI workflow (sibling of `ci.yml`, NOT a modification per D-01)
**Analog:** `.github/workflows/ci.yml::test` job (lines 89-125) for the cargo-test shape, extended with the systemd-user-session setup per research Pattern 5.

**Closest existing `cargo test` job shape** (`.github/workflows/ci.yml:89-125`):
```yaml
test:
  name: Test
  needs: changes
  if: ${{ !startsWith(github.head_ref, 'dependabot/github_actions/') && needs.changes.outputs.run_code_jobs == 'true' }}
  runs-on: ${{ matrix.os }}
  strategy:
    fail-fast: false
    matrix:
      os: [ubuntu-latest, macos-latest]
  steps:
    - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6

    - name: Install system dependencies (Ubuntu)
      if: runner.os == 'Linux'
      run: sudo apt-get update && sudo apt-get install -y libdbus-1-dev pkg-config

    - name: Install Rust toolchain
      uses: dtolnay/rust-toolchain@631a55b12751854ce901bb631d5902ceb48146f7 # stable
      with:
        toolchain: stable

    - name: Cache cargo registry
      uses: actions/cache@668228422ae6a00e4ad889ee87cd7109ec5666a7 # v5
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: |
          ${{ runner.os }}-cargo-

    - name: Build
      run: cargo build --workspace --verbose

    - name: Run tests
      run: cargo test --workspace --verbose
```

**Copy for Plan 37-04 — patterns to preserve:**
- Top-level `on: pull_request: branches: [main]` + `push: branches: [main]`.
- `permissions: contents: read` (with `id-token: write` ADDED on the `pkgs-auto-pull` job for the D-13 keyless OIDC signing — see Pattern 6 below).
- `env: CARGO_TERM_COLOR: always` + `RUSTFLAGS: -Dwarnings`.
- Pinned action SHAs (e.g., `actions/checkout@de0fac2e...`, `dtolnay/rust-toolchain@631a55b1...`, `actions/cache@668228422...`).
- Per-runner cargo cache step.

**Differences from `ci.yml::test` that Plan 37-04 MUST introduce:**
- `runs-on: ubuntu-24.04` (PINNED per D-01; not `ubuntu-latest`, not a matrix).
- TWO separate jobs (`resl-nix` + `pkgs-auto-pull`) per D-04 — NOT a matrix.
- Systemd-user-session prep block (research Pattern 5, reproduced in the Shared Patterns section below).
- Different `cargo test` invocations per job:
  - `resl-nix`: `machinectl shell ${USER}@.host /usr/bin/env bash -c 'cd $GITHUB_WORKSPACE && cargo test -p nono-cli --test resl_nix_linux --test resl_nix_async_signal_safety --release'`
  - `pkgs-auto-pull`: keyless-sign step + `cargo test -p nono-cli --test auto_pull_e2e_linux --release`

**Workflow trigger filter** (per open question #1 in research, recommendation "always-on"): do NOT path-filter unless CI minute budget is constrained. If filtered, follow the `changes` classifier job pattern from `ci.yml:17-87`.

---

### `crates/nono-cli/tests/auto_pull_e2e_linux.rs` — NEW integration test

**Role:** integration test (Linux-gated, multi-test, subprocess + mock-TCP)
**Analog A (test harness):** `crates/nono-cli/tests/resl_nix_linux.rs` (lines 1-130).
**Analog B (multi-endpoint TCP server precedent):** `crates/nono-cli/src/registry_client.rs::tests::spawn_one_shot_server` (lines 367-412).

**Harness skeleton from Analog A** (`crates/nono-cli/tests/resl_nix_linux.rs:1-53`):
```rust
//! Phase 25-01 integration tests — Linux resource-limit enforcement.

#![cfg(target_os = "linux")]

use std::process::Command;
use std::time::Instant;

const NONO_BIN: &str = env!("CARGO_BIN_EXE_nono");

/// Returns `true` if the current process has a cgroup v2 delegation (single `0::/...` line).
fn cgroup_v2_available() -> bool {
    let Ok(content) = std::fs::read_to_string("/proc/self/cgroup") else {
        return false;
    };
    let trimmed = content.trim();
    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() != 1 { return false; }
    if !lines[0].starts_with("0::/") { return false; }
    // Also confirm the cgroup directory is writable (delegation check).
    let cg_path_rel = lines[0].trim_start_matches("0::/");
    let cg_path = format!("/sys/fs/cgroup/{cg_path_rel}/cgroup.subtree_control");
    std::fs::metadata(&cg_path)
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false)
}

/// Macro to skip test with an explanatory message if cgroup v2 is not available.
macro_rules! require_cgroup_v2 {
    () => {
        if !cgroup_v2_available() {
            eprintln!(
                "SKIP: cgroup v2 delegation not available on this host..."
            );
            return;
        }
    };
}
```

**Copy for Plan 37-05:** the new file MUST start with `#![cfg(target_os = "linux")]`, MUST use `const NONO_BIN: &str = env!("CARGO_BIN_EXE_nono");`, and MUST invoke `nono` via `Command::new(NONO_BIN).args([...]).output()`. The `require_cgroup_v2!` macro is NOT needed for the auto-pull test (the auto-pull path does not exercise cgroups) but the file CAN reuse the same cfg gate.

**Std-only TCP server pattern from Analog B** (`crates/nono-cli/src/registry_client.rs:367-412`):
```rust
fn spawn_one_shot_server(
    body: Vec<u8>,
    content_length_override: Option<u64>,
) -> (String, thread::JoinHandle<()>) {
    use std::io::{Read as _, Write as _};
    use std::net::Shutdown;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{}/artifact", addr);

    let handle = thread::spawn(move || {
        let (mut stream, _peer) = match listener.accept() {
            Ok(pair) => pair,
            Err(_) => return,
        };
        // Read (and discard) the request line + headers up to CRLF-CRLF.
        let mut buf = [0u8; 4096];
        let mut accumulated = Vec::with_capacity(4096);
        loop {
            let n = match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            accumulated.extend_from_slice(&buf[..n]);
            if accumulated.windows(4).any(|w| w == b"\r\n\r\n") { break; }
            if accumulated.len() > 64 * 1024 { break; }
        }
        let cl = content_length_override.unwrap_or(body.len() as u64);
        let response_head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            cl
        );
        let _ = stream.write_all(response_head.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
        let _ = stream.shutdown(Shutdown::Both);
    });

    (url, handle)
}
```

**Copy for Plan 37-05 — extension shape** (per research Pattern 4):
```rust
fn spawn_multi_endpoint_server(
    routes: HashMap<String, Vec<u8>>,  // path → body
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{}", addr);

    let handle = thread::spawn(move || {
        // Loop instead of single accept; route on parsed URL path.
        for accept in listener.incoming().take(routes.len() * 3) {
            let Ok(mut stream) = accept else { return };
            // ... same read-until-CRLF-CRLF accumulation as above ...
            // Parse path from "GET /path HTTP/1.1\r\n"
            let path = parse_path(&request_line);
            let body = routes.get(&path).cloned().unwrap_or_else(|| b"404".to_vec());
            let status = if routes.contains_key(&path) { 200 } else { 404 };
            // ... same response write/flush/shutdown ...
        }
    });
    (url, handle)
}
```

**Env-guard pattern** (per CLAUDE.md and the test stub at research line 600-616):
```rust
std::env::set_var("NONO_REGISTRY", &mock_url);
let _guard = scopeguard::guard((), |_| std::env::remove_var("NONO_REGISTRY"));
// OR use the EnvGuard RAII pattern from Phase 35-02 (see Shared Patterns).
```

**Acceptance-mapped test list** (per D-16):
- `auto_pull_happy_path` (REQ-PKGS-04 #1)
- `auto_pull_unknown_name_fails_closed` (#2)
- `auto_pull_signature_failure_aborts` (#3)
- `auto_pull_no_auto_pull_flag_falls_back_to_profile_not_found` (#4)
- (Recommended) `auto_pull_rejects_non_policy_pack_type` (research open question #4)

---

### `crates/nono-cli/src/session_commands.rs` — MODIFIED `run_inspect` Limits-block

**Role:** display / formatter (mechanical string fix + platform-aware cfg)
**Analog:** the existing block IS the analog for shape; the `#[cfg(target_os = "linux")]` precedent comes from `crates/nono/src/error.rs:67-73` (Landlock arms) and `crates/nono-cli/src/profile/mod.rs:2314` (`#[cfg(not(target_os = "windows"))]`).

**Current emission** (`crates/nono-cli/src/session_commands.rs:546-565`) — this is the drift site:
```rust
if let Some(limits) = record.limits.as_ref() {
    if !limits.is_empty() {
        println!("\nLimits:");
        if let Some(pct) = limits.cpu_percent {
            println!("  cpu:     {pct}% (hard cap)");
        }
        if let Some(bytes) = limits.memory_bytes {
            println!("  memory:  {} (job-wide)", format_bytes_human(bytes));
        }
        if let Some(secs) = limits.timeout_seconds {
            println!(
                "  timeout: {}",
                format_duration_human(std::time::Duration::from_secs(secs))
            );
        }
        if let Some(procs) = limits.max_processes {
            println!("  procs:   {procs} (active)");
        }
    }
}
```

**LOCKED targets per ROADMAP success criteria #1-3:**
- `cpu_percent: 25 (cgroup v2 cpu.max 25000 100000)`
- `memory: 100M (cgroup v2 memory.max)`
- `max_processes: 5 (cgroup v2 pids.max)`

**Copy for Plan 37-03 — cfg-gated emission** (per research open question #2 recommendation):
```rust
if let Some(pct) = limits.cpu_percent {
    #[cfg(target_os = "linux")]
    {
        // REQ-RESL-NIX-02 acceptance #2: LOCKED string.
        let quota = (pct as u32) * 1000; // 25% → 25000μs of 100000μs period
        println!("  cpu_percent: {pct} (cgroup v2 cpu.max {quota} 100000)");
    }
    #[cfg(target_os = "windows")]
    {
        println!("  cpu:     {pct}% (hard cap)");
    }
    #[cfg(target_os = "macos")]
    {
        println!("  cpu:     {pct}% (n/a — macOS deprioritized v2.5)");
    }
}
// ... similar for memory and max_processes ...
```

**Bytes formatter for the LOCKED "100M" shape:** the existing `format_bytes_human(bytes)` (`session_commands.rs:574-595`) emits `"100 MiB"` not `"100M"`. Plan 37-03 either adds a sibling `format_bytes_short(bytes)` helper (emit `"100M"` / `"1G"` style) OR reformats inline. Mirror the existing parser convention (`crate::cli::parse_byte_size`) which uses K/M/G/T multipliers.

**Mirror in `session_commands_windows.rs:540,543,552`:** SAME current shape (`"cpu:     {pct}% (hard cap)"` / `"memory:  ... (job-wide)"` / `"procs:   ... (active)"`). If Plan 37-03 emits cfg-gated strings, the Windows file's emission is implicitly the `#[cfg(target_os = "windows")]` branch — so the file may not need editing UNLESS the dispatch into it is target-cfg-gated at the caller. Verify the caller-side dispatch before deciding.

---

### `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — MODIFIED 4-of-5 `UnsupportedPlatform` swap

**Role:** error construction site (mechanical refactor inside `cgroup` submodule)
**Analog:** the existing sites themselves are the analog.

**Site 1 — empty `/proc/self/cgroup`** (`supervisor_linux.rs:879-885`):
```rust
let trimmed = contents.trim();
if trimmed.is_empty() {
    return Err(NonoError::UnsupportedPlatform(
        "cgroup_v2: /proc/self/cgroup is empty (not running under a cgroup v2 \
         delegated hierarchy; is systemd the init system?)"
            .into(),
    ));
}
```

**Site 2 — multi-line (v1/hybrid)** (`supervisor_linux.rs:886-894`):
```rust
let mut lines = trimmed.lines();
let first = lines.next().unwrap_or("");
if lines.next().is_some() {
    return Err(NonoError::UnsupportedPlatform(
        "cgroup_v2: /proc/self/cgroup has multiple lines (cgroup v1 or hybrid mode \
         detected; nono requires pure cgroup v2 with systemd delegation)"
            .into(),
    ));
}
```

**Site 3 — missing `0::` prefix** (`supervisor_linux.rs:895-901`):
```rust
let cgroup_rel = first.strip_prefix("0::").ok_or_else(|| {
    NonoError::UnsupportedPlatform(format!(
        "cgroup_v2: /proc/self/cgroup line does not start with '0::' \
         (got: {first:?}); this indicates cgroup v1 or hybrid mode"
    ))
})?;
```

**Site 4 — PATH-TRAVERSAL GUARD (KEEP AS-IS)** (`supervisor_linux.rs:904-934`):
```rust
// WR-03: Validate the constructed path stays within /sys/fs/cgroup.
//
// We perform two complementary component-level checks (NOT string
// operations) per CLAUDE.md § Path Handling:
//
//   1. `Path::starts_with("/sys/fs/cgroup")` rejects entries that, after
//      `trim_start_matches('/')`, somehow produce a path that does not
//      have `/sys/fs/cgroup` as a component prefix. ...
//
//   2. We additionally reject any path containing a `Component::ParentDir`
//      (`..`). A well-formed cgroup-v2 delegated path from
//      `/proc/self/cgroup` never contains `..`; its presence indicates a
//      malicious or compromised /proc entry attempting to redirect path
//      construction outside `/sys/fs/cgroup` (e.g., `0::/../../etc`).
//
// Both checks fail closed with `NonoError::UnsupportedPlatform`.
use std::path::Component;
if !abs_path.starts_with("/sys/fs/cgroup")
    || abs_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
{
    return Err(NonoError::UnsupportedPlatform(format!(
        "cgroup_v2: constructed cgroup path {abs_path:?} escapes /sys/fs/cgroup \
         (path traversal detected in /proc/self/cgroup content)"
    )));
}
```

**Why site 4 stays as `UnsupportedPlatform`** (per research open question #3 recommendation):
> "this is 'your kernel is fine, but `/proc` content is malformed/malicious', not 'your kernel needs reboot'. The hint 'cgroup v2 required; boot with ...' would mislead the user — boot flags don't fix /proc manipulation."

**Site 5 — read / metadata failures** (`supervisor_linux.rs:957-972`):
```rust
let contents = std::fs::read_to_string("/proc/self/cgroup").map_err(|e| {
    NonoError::UnsupportedPlatform(format!(
        "cgroup_v2: failed to read /proc/self/cgroup: {e}"
    ))
})?;
let delegated = Self::detect_from_str(&contents)?;
match std::fs::metadata(&delegated) {
    Ok(m) if m.is_dir() => Ok(delegated),
    Ok(_) => Err(NonoError::UnsupportedPlatform(format!(
        "cgroup_v2: delegated path {delegated:?} exists but is not a directory"
    ))),
    Err(e) => Err(NonoError::UnsupportedPlatform(format!(
        "cgroup_v2: delegated path {delegated:?} is not accessible: {e}"
    ))),
}
```

**Copy for Plan 37-01 — swap shape for sites 1, 2, 3, 5** (per research Pattern 2 + D-07):
```rust
return Err(NonoError::UnsupportedKernelFeature {
    feature: "cgroup_v2".into(),
    hint: "cgroup v2 required; boot with systemd.unified_cgroup_hierarchy=1 or cgroup_no_v1=all".into(),
});
```

**Async-signal-safety invariant** (per research Anti-Patterns line 455 + Pitfall #3 line 528-536):
> "Any change in `exec_strategy.rs` near the post-fork child arm that introduces a `format!` or `String::new()` would deadlock..."
> "The `resl_nix_async_signal_safety.rs` test will fail loudly if any `format!` appears between `CR-01-CHILD-ARM-START` and `CR-01-CHILD-ARM-END` sentinels."

All 5 swap sites are in the PARENT-process pre-fork `detect` / `detect_from_str` paths — NOT in the post-fork child arm — so `.into()` + struct construction is fine. Plan 37-01 MUST NOT touch `place_self_in_cgroup_raw` or anything between the child-arm sentinels.

---

## Shared Patterns

### Pattern A — `NonoError` 3-surface touch (Plan 37-01)

**Source:** `crates/nono/src/error.rs:42-52` + `bindings/c/src/lib.rs:139-142`
**Apply to:** Plan 37-01 (`error.rs` variant + `bindings/c/src/lib.rs` FFI arm)

Single-commit landing. If only the variant is added without the FFI arm, `cargo check --workspace` fails with `non-exhaustive patterns`. The compiler IS the enforcement boundary for this pattern per the comment block at `bindings/c/src/lib.rs:67-71`.

### Pattern B — Tests-touch-env save/restore RAII (Plans 37-02, 37-05)

**Source:** CLAUDE.md § Coding Standards "Environment variables in tests"; Phase 35-02 `EnvGuard` precedent
**Apply to:** any test in Plans 37-02 / 37-05 that mutates `NONO_NO_AUTO_PULL`, `NONO_REGISTRY`, `HOME`, `TMPDIR`, `XDG_CONFIG_HOME`, or `NONO_TEST_HOME`

```rust
let original = std::env::var("NONO_REGISTRY").ok();
std::env::set_var("NONO_REGISTRY", &mock_url);
let _guard = scopeguard::guard(original, |orig| match orig {
    Some(v) => std::env::set_var("NONO_REGISTRY", v),
    None => std::env::remove_var("NONO_REGISTRY"),
});
```

Rust tests run in parallel within the same process. An unrestored env var causes flaky failures in unrelated tests (e.g., `config::check_sensitive_path` fails when another test temporarily sets `HOME`).

### Pattern C — Pinned-SHA GitHub Action references (Plan 37-04)

**Source:** `.github/workflows/ci.yml` lines 24, 99, 106, 111, 133, etc.
**Apply to:** `.github/workflows/phase-37-linux-resl.yml`

Every `uses:` reference MUST be SHA-pinned with a version tag comment:
```yaml
- uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6
- uses: dtolnay/rust-toolchain@631a55b12751854ce901bb631d5902ceb48146f7 # stable
- uses: actions/cache@668228422ae6a00e4ad889ee87cd7109ec5666a7 # v5
- uses: actions/upload-artifact@bbbca2ddaa5d8feaa63e36b76fdaad77386f024f # v7.0.0
```

Never `@v6` / `@main` / `@latest` — supply-chain hardening convention established in `ci.yml`.

### Pattern D — Systemd-user-session + cgroup-v2 cpu delegation (Plan 37-04)

**Source:** research Pattern 5 (lines 422-450); systemd CGROUP_DELEGATION.md (cited)
**Apply to:** `.github/workflows/phase-37-linux-resl.yml::resl-nix` job ONLY (the `pkgs-auto-pull` job does not need cgroup delegation)

```yaml
- name: Install dbus-user-session and configure cgroup v2 cpu controller delegation
  run: |
    sudo apt-get update
    sudo apt-get install -y dbus-user-session
    sudo mkdir -p /etc/systemd/system/user@.service.d
    sudo tee /etc/systemd/system/user@.service.d/delegate.conf <<'EOF'
    [Service]
    Delegate=cpu cpuset io memory pids
    EOF
    sudo systemctl daemon-reload

- name: Enable lingering for runner user
  run: sudo loginctl enable-linger $USER

- name: Wait for user session and verify delegated controllers
  run: |
    timeout 10 bash -c 'until systemctl --user is-active default.target; do sleep 0.5; done' || true
    cat /sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service/cgroup.controllers

- name: Run RESL-NIX Linux tests under systemd user session
  run: |
    machinectl shell ${USER}@.host /usr/bin/env bash -c \
      'cd ${{ github.workspace }} && cargo test -p nono-cli --test resl_nix_linux --test resl_nix_async_signal_safety --release'
```

Default Ubuntu user delegation grants `memory pids` ONLY — the `cpu` controller drop-in is REQUIRED per Pitfall #1.

### Pattern E — Keyless OIDC signing in CI (Plan 37-04 pkgs-auto-pull job)

**Source:** research lines 624-654; Sigstore docs (cited)
**Apply to:** `.github/workflows/phase-37-linux-resl.yml::pkgs-auto-pull` job

```yaml
pkgs-auto-pull:
  runs-on: ubuntu-24.04
  permissions:
    contents: read
    id-token: write  # required for GitHub Actions OIDC
  steps:
    - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd
    - uses: dtolnay/rust-toolchain@631a55b12751854ce901bb631d5902ceb48146f7

    - name: Build nono binary
      run: cargo build --workspace --release

    - name: Sign fixture pack with sigstore-sign (keyless via GH Actions OIDC)
      run: |
        mkdir -p target/fixture-pack
        # ... build the artifact, manifest.json, etc. ...
        cargo run --release -p sigstore-sign --example sign_blob -- \
          target/fixture-pack/artifact.tar.gz \
          -o target/fixture-pack/artifact.tar.gz.sigstore.json
      env:
        SIGSTORE_ID_TOKEN_AUDIENCE: sigstore

    - name: Run auto-pull e2e test
      run: cargo test -p nono-cli --test auto_pull_e2e_linux --release
      env:
        NONO_FIXTURE_PACK_DIR: ${{ github.workspace }}/target/fixture-pack
```

The `id-token: write` permission is scoped to this job ONLY (NOT inherited at workflow level) — minimum-privilege per CLAUDE.md § Security.

### Pattern F — `#[cfg(target_os = "...")]` over runtime branching (Plan 37-03)

**Source:** CLAUDE.md § Coding Standards "Explicit Over Implicit"; `crates/nono/src/error.rs:67-73`; `crates/nono-cli/src/profile/mod.rs:2314`
**Apply to:** Plan 37-03 `run_inspect` string emission

Per research open question #2: use compile-time `#[cfg]` gates, NOT `cfg!()` runtime macros. Compiler verifies all 3 platform arms are well-formed; runtime branches risk dead-code warnings on platforms that never hit them.

### Pattern G — `lazy use of dead code` discipline (all plans)

**Source:** CLAUDE.md § Coding Standards "Lazy use of dead code"
**Apply to:** all plans

> "Avoid `#[allow(dead_code)]`. If code is unused, either remove it or write tests that use it."

Plan 37-01: if the `UnsupportedKernelFeature` variant is added without a construction site, the unit test in the `#[cfg(test)]` block constitutes the use (acceptable). Plan 37-02: `ProfileResolverArgs` is referenced via flatten, so clap derive uses it. Plan 37-04: `.github/workflows/*` files are not Rust code (rule N/A).

### Pattern H — Cross-target clippy gate (all plans, close-time)

**Source:** CLAUDE.md § Cross-target clippy verification; memory `feedback_clippy_cross_target`
**Apply to:** every Phase 37 plan before close

```bash
cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

If toolchain absent on Windows host, follow `.planning/templates/cross-target-verify-checklist.md` and mark verification REQ as PARTIAL. Especially load-bearing for Plans 37-01 (touches `#[cfg(target_os = "linux")]` blocks in `supervisor_linux.rs`) and 37-03 (introduces new `#[cfg]` gates).

---

## No Analog Found

| File | Role | Data Flow | Reason / Fallback |
|------|------|-----------|-------------------|
| (none) | — | — | All 8 files have at least a role-match analog in-tree. Research's "Don't Hand-Roll" table (lines 464-475) confirms every problem has an in-tree precedent. |

The only "weak match" is the `ResolveContext` parameter threading — there is no exact precedent for "thread a Default-able context struct through an existing top-level function via a wrapper-with-default". The pattern is mechanical and well-trodden in Rust; research Pattern 3 specifies the exact shape, and the wrapper-with-default keeps existing call-sites compiling without modification.

---

## Metadata

**Analog search scope:**
- `crates/nono/src/error.rs` (full read)
- `bindings/c/src/lib.rs` (lines 1-200 read; map_error lines 72-144 verified)
- `crates/nono-cli/src/cli.rs` (targeted reads: lines 1080-1230, 1460-1520, 2050-2300)
- `crates/nono-cli/src/profile/mod.rs` (targeted read: lines 2150-2325)
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (targeted read: lines 870-980)
- `crates/nono-cli/src/session_commands.rs` (targeted read: lines 505-595)
- `crates/nono-cli/src/session_commands_windows.rs` (grep confirmation at lines 540, 543, 552)
- `crates/nono-cli/src/registry_client.rs` (targeted read: lines 340-500)
- `crates/nono-cli/tests/resl_nix_linux.rs` (lines 1-130 read)
- `.github/workflows/ci.yml` (lines 1-230 read)

**Files scanned:** 10 (8 analog targets + 2 cross-reference files)
**Pattern extraction date:** 2026-05-19
**Phase researcher source:** `.planning/phases/37-linux-resl-backends-pkgs-auto-pull/37-RESEARCH.md` (856 lines, full sectional read)
**Phase context source:** `.planning/phases/37-linux-resl-backends-pkgs-auto-pull/37-CONTEXT.md` (152 lines, full read; D-01..D-16 + Claude's Discretion items)
