# Phase 32: sigstore-integration - Pattern Map

**Mapped:** 2026-05-10
**Files analyzed:** 11 (6 modify + 5 create)
**Analogs found:** 11 / 11 (all anchors have a clear analog in-tree)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono/src/trust/bundle.rs` (MOD) — `load_production_trusted_root` rewrite | library / trust primitive | file-I/O (sync) | `crates/nono/src/trust/bundle.rs::load_trusted_root` (sibling fn, lines 113-116) | exact (same module, sync sibling already exists) |
| `crates/nono/src/trust/bundle.rs` (MOD) — 2 failing test migrations at :877 + :914 | unit test | file-I/O | `crates/nono/src/trust/bundle.rs::tests::load_trusted_root_invalid_json` (lines 862-868) | exact (same test mod, drop the `#[tokio::test]` async harness) |
| `crates/nono/src/trust/mod.rs` (MOD) — `#[cfg(test)] pub fn load_test_trusted_root()` | library / trust test seam | file-I/O | `crates/nono/src/trust/bundle.rs::load_trusted_root` (existing `from_file` wrapper) | exact (delegates to existing `load_trusted_root`) |
| `crates/nono/tests/fixtures/trust-root-frozen.json` (NEW) | test fixture | static asset | (no in-tree TUF fixture; closest is `crates/nono/tests/fixtures/` checked-in keyed fixtures convention used by `bundle.rs` tests) | new-asset (no analog needed beyond directory convention) |
| `crates/nono-cli/src/setup.rs` (MOD) — `refresh_trust_root()` impl | CLI subcommand step | file-I/O + one-shot async network | `crates/nono-cli/src/setup.rs::install_windows_wfp_service` (lines 159-177) and sibling `start_windows_wfp_service` (lines 219-237) | exact (same `SetupRunner` method shape) |
| `crates/nono-cli/src/cli.rs` (MOD) — `SetupArgs.refresh_trust_root: bool`; `TrustVerifyArgs.{issuer, identity}: Option<String>` | clap argument struct | request-response | `crates/nono-cli/src/cli.rs::SetupArgs` lines 1991-2033 (composable bool flags); `TrustVerifyArgs` lines 2674-2692 | exact |
| `crates/nono-cli/src/trust_cmd.rs` (MOD) — `run_verify` enforce `--issuer` + `--identity`; polish `discover_oidc_token` error | CLI verify dispatcher | request-response | `crates/nono-cli/src/trust_cmd.rs::run_verify` (lines 740-839) and `verify_single_file` (lines 949-1047) — IN-PLACE harden | exact (same function, hardened) |
| `crates/nono-cli/tests/keyless_sign.rs` (NEW) | integration test | event-driven (mock HTTP) | `crates/nono-cli/tests/audit_attestation.rs::setup_isolated_home` (lines 38-80) — `NONO_TEST_HOME` + tempdir scaffold | role-match (hermetic + isolated home; mock servers are net-new) |
| `crates/nono-cli/tests/keyless_verify.rs` (NEW) | integration test | request-response (CLI subprocess) | `crates/nono-cli/tests/audit_attestation.rs::run_nono` (lines 12-27) — subprocess-with-`NONO_TEST_HOME` shape | exact |
| `crates/nono-cli/tests/broker_authenticode.rs` (NEW, `#[cfg(windows)]`) | integration test | request-response (subprocess) | `crates/nono-cli/src/exec_strategy_windows/launch.rs::broker_dispatch_tests::broker_launch_assigns_child_to_job_object` (lines 2247-2330) — broker artifact resolution + SKIP-when-missing | exact |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` (MOD) — `WindowsTokenArm::BrokerLaunch` Authenticode gate | CLI / process launch | request-response | `crates/nono-cli/src/exec_identity_windows.rs::query_authenticode_status` (lines 119-210) — Phase 28 chain-walker call site | exact (call existing API; no new FFI) |

> **Anchor correction:** RESEARCH § Sources records that the broker dispatch site lives at `launch.rs:1262+`, not `:2173+`. The `:2173+` line range is inside the `broker_dispatch_tests` test module. The pattern map and planner should treat **`launch.rs:1246-1438`** as the BrokerLaunch arm (production code) and **`launch.rs:2168-2330`** as the test module to model new tests after. CONTEXT canonical_refs and RESEARCH agree on the production target — only the line citation in CONTEXT is off.

## Pattern Assignments

### `crates/nono/src/trust/bundle.rs::load_production_trusted_root` (REWRITE)

**Analog:** sibling `load_trusted_root` already in the same file (lines 113-116) — that's the synchronous shape we collapse to.

**Existing async shape to replace** (lines 128-140):
```rust
/// Load the production Sigstore trusted root (embedded).
///
/// This uses the Sigstore public good instance trusted root that is
/// embedded in the `sigstore-trust-root` crate.
///
/// # Errors
///
/// Returns `NonoError::TrustPolicy` if the embedded root cannot be loaded.
pub async fn load_production_trusted_root() -> Result<TrustedRoot> {
    TrustedRoot::production()
        .await
        .map_err(|e| NonoError::TrustPolicy(format!("failed to load production trusted root: {e}")))
}
```

**Sync sibling pattern to copy** (lines 113-116):
```rust
pub fn load_trusted_root<P: AsRef<Path>>(path: P) -> Result<TrustedRoot> {
    TrustedRoot::from_file(path.as_ref())
        .map_err(|e| NonoError::TrustPolicy(format!("failed to load trusted root: {e}")))
}
```

**Critical async-to-sync API shape change (D-32-15 #1):**
- Function signature changes from `pub async fn load_production_trusted_root() -> Result<TrustedRoot>` to `pub fn load_production_trusted_root() -> Result<TrustedRoot>`.
- Five callers currently wrap in `rt.block_on(...)`; all must drop the wrapper. Verified via `Grep("load_production_trusted_root", path: crates/nono-cli/src)`:
  | Caller | Line | Current shape |
  |--------|------|---------------|
  | `crates/nono-cli/src/trust_cmd.rs` | 907-913 (in `verify_multi_subject_file`) | `rt.block_on(trust::load_production_trusted_root())` — KEYLESS arm only |
  | `crates/nono-cli/src/trust_cmd.rs` | 1027-1033 (in `verify_single_file`) | same shape — KEYLESS arm only |
  | `crates/nono-cli/src/trust_intercept.rs` | 371-376 | `rt.block_on(trust::load_production_trusted_root())` |
  | `crates/nono-cli/src/trust_scan.rs` | 247-255 | same |
  | `crates/nono-cli/src/trust_scan.rs` | 753-760 | same |
  | `crates/nono-cli/src/package_cmd.rs` | 446-452 | `rt.block_on(nono::trust::load_production_trusted_root())` |
- Each call site is keyed-vs-keyless-arm-conditional; the `tokio::runtime::Builder::new_current_thread()` block above each `block_on` should be removed entirely if the only async work in that scope was the trusted-root load. (`trust_cmd.rs:907-913` and `:1027-1033` are exactly that shape — runtime built solely to host this one call.)

**Error variant convention** (already in use, keep):
- `NonoError::TrustPolicy(String)` — for "cache missing" / "cache parse failure" (matches existing `load_trusted_root` error class at line 115).
- `NonoError::TrustVerification { path: String, reason: String }` — for "cache expired" (matches existing `verify_bundle` at lines 167-171).
- D-32-05 wording: `NonoError::TrustPolicy("Sigstore trusted root not initialized; run `nono setup --refresh-trust-root` (requires network).")` — fail-closed-with-recovery convention copied verbatim from D-32-03/05.

**Doc-comment divergence note (Pitfall 4):** New doc-comment must say "Diverges from upstream `sigstore_verify::TrustedRoot::production()` per Phase 32 D-32-01: this fork caches the trusted root explicitly under `<nono_home_dir()>/.nono/trust-root/` to keep verify offline." `tests/integration/test_upstream_drift.sh:257` already monitors `load_production_trusted_root` — add a same-line `# intentional fork: Phase 32 D-32-01` comment there.

---

### `crates/nono/src/trust/bundle.rs::tests` (TEST MIGRATION at :877 + :914)

**Analog:** sibling test `load_trusted_root_invalid_json` (lines 862-868) — sync `#[test]` body, no tokio harness.

**Tests to migrate** (current shape at lines 876-880 and 913-922):
```rust
#[tokio::test]
async fn load_production_trusted_root_succeeds() {
    let root = load_production_trusted_root().await;
    assert!(root.is_ok());
}

#[tokio::test]
async fn verify_bundle_with_invalid_digest() {
    let json = make_public_key_bundle_json("key");
    let bundle = Bundle::from_json(&json).unwrap();
    let root = load_production_trusted_root().await.unwrap();
    let policy = VerificationPolicy::default();
    let result =
        verify_bundle_with_digest("not-hex!", &bundle, &root, &policy, Path::new("test"));
    assert!(result.is_err());
}
```

**Target shape** (drops `#[tokio::test]`/`async`/`.await`; routes through new `load_test_trusted_root()`):
```rust
#[test]
fn load_production_trusted_root_succeeds() {
    // Phase 32 D-32-02: exercises the test seam, not the production cache
    // (which doesn't exist in CI). load_test_trusted_root reads the frozen
    // fixture at crates/nono/tests/fixtures/trust-root-frozen.json.
    let root = crate::trust::load_test_trusted_root();
    assert!(root.is_ok(), "frozen fixture must load: {:?}", root.err());
}

#[test]
fn verify_bundle_with_invalid_digest() {
    let json = make_public_key_bundle_json("key");
    let bundle = Bundle::from_json(&json).unwrap();
    let root = crate::trust::load_test_trusted_root().unwrap();
    let policy = VerificationPolicy::default();
    let result =
        verify_bundle_with_digest("not-hex!", &bundle, &root, &policy, Path::new("test"));
    assert!(result.is_err());
}
```

---

### `crates/nono/src/trust/mod.rs::load_test_trusted_root` (NEW `#[cfg(test)]` HELPER)

**Analog:** the existing public `load_trusted_root` (file-path) at `bundle.rs:113-116` is the body the helper delegates to. The path-resolution pattern is `env!("CARGO_MANIFEST_DIR")` joined with `tests/fixtures/...`.

**Pattern to copy** (RESEARCH § Pattern 3, derived from existing fixture-loading idiom in the trust tests):
```rust
// In crates/nono/src/trust/mod.rs (NEW HELPER — D-32-15 #2)
#[cfg(test)]
pub(crate) fn load_test_trusted_root() -> crate::Result<crate::trust::TrustedRoot> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("trust-root-frozen.json");
    crate::trust::bundle::load_trusted_root(&path)
}
```

**Visibility:** `pub(crate)` — only the in-crate test modules call this. Do NOT mark `pub`; that would re-export through `mod.rs::pub use bundle::{...}` and widen the library API surface.

**No `#[allow(dead_code)]`:** Both failing tests reference the helper; CLAUDE.md § Coding Standards forbids `#[allow(dead_code)]`.

---

### `crates/nono-cli/src/setup.rs::refresh_trust_root` (NEW METHOD on `SetupRunner`)

**Analog:** `crates/nono-cli/src/setup.rs::install_windows_wfp_service` (lines 159-177) — composable, fail-closed, prints `[X/Y]` phase index, returns `Result<()>`.

**Imports pattern** (lines 1-10) — already in scope; no new imports needed beyond `tokio::runtime::Builder`:
```rust
use crate::cli::SetupArgs;
use crate::profile;
use nono::{NonoError, Result};
use std::fs;
use std::path::Path;
```

**Composable subcommand-step pattern** (lines 159-177 — copy this shape):
```rust
#[cfg(target_os = "windows")]
fn install_windows_wfp_service(&self) -> Result<()> {
    if !crate::exec_strategy::is_admin_process() {
        return Err(NonoError::Setup(
            "Windows WFP service installation requires an elevated administrator session."
                .to_string(),
        ));
    }
    println!(
        "[{}/{}] Registering Windows WFP service (placeholder)...",
        self.install_phase_index(),
        self.total_phases()
    );
    let report = crate::exec_strategy::install_windows_wfp_service()?;
    println!("  * WFP service install: {}", report.status_label);
    println!("  * {}", report.details);
    println!();
    Ok(())
}
```

**Dispatch wiring** (`run()` body, lines 40-104 — append a new conditional in the same shape, NO admin check per D-32-01 / Pitfall 7):
```rust
// Existing pattern (lines 57-60):
#[cfg(target_os = "windows")]
if !self.check_only && self.install_wfp_service {
    self.install_windows_wfp_service()?;
}
// New conditional follows the same shape — but cross-platform (no #[cfg(target_os = "windows")])
// because TUF refresh is per-user, no admin, all-platforms:
if !self.check_only && self.refresh_trust_root {
    self.refresh_trust_root_step()?;
}
```

**Recommended impl shape** (RESEARCH § Pattern 1; cross-platform, no `is_admin_process()` check, uses `crate::config::nono_home_dir()` for D-27.1 test-home compatibility):
```rust
fn refresh_trust_root_step(&self) -> Result<()> {
    let cache_dir = crate::config::nono_home_dir()?
        .join(".nono")
        .join("trust-root");
    std::fs::create_dir_all(&cache_dir).map_err(NonoError::Io)?;

    println!(
        "[{}/{}] Refreshing Sigstore trusted root...",
        self.refresh_trust_root_phase_index(),
        self.total_phases()
    );

    // ONE-SHOT tokio runtime (the rest of `nono setup` is sync).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| NonoError::Setup(format!("tokio runtime: {e}")))?;
    let trusted_root = rt.block_on(
        sigstore_verify::trust_root::TrustedRoot::production()
    ).map_err(|e| NonoError::Setup(format!(
        "Failed to fetch Sigstore trusted root from https://tuf-repo-cdn.sigstore.dev: {e}"
    )))?;

    let json = serde_json::to_string_pretty(&trusted_root)
        .map_err(|e| NonoError::Setup(format!("serialize trusted root: {e}")))?;
    let cache_path = cache_dir.join("trusted_root.json");
    std::fs::write(&cache_path, json).map_err(NonoError::Io)?;

    println!("  * Sigstore trusted root cached at {}", cache_path.display());
    println!();
    Ok(())
}
```

**Note:** `sigstore_verify::trust_root::TrustedRoot::production()` is the ONLY async constructor; staying behind the library boundary by calling `nono::trust::load_production_trusted_root()` would defeat the purpose (that fn is being rewritten as the cache-reader). The CLI calls `sigstore_verify::trust_root::TrustedRoot::production()` directly here.

**`--check-only` integration** (Open Question 2 from RESEARCH; planner discretion per CONTEXT § Claude's Discretion): the existing `print_check_only_summary` at `setup.rs:818-833` (Windows) and `:951-953` (non-Windows) is the seam. A one-line "Trust root cache: <path> (refreshed YYYY-MM-DD; valid until YYYY-MM-DD)" or "Trust root cache: NOT INITIALIZED" addition matches the existing one-liner WFP-status convention. Pattern (lines 818-832):
```rust
#[cfg(target_os = "windows")]
fn print_check_only_summary() {
    let info = nono::Sandbox::support_info();
    let wfp = crate::exec_strategy::probe_windows_wfp_readiness();
    println!("Support status: {}", info.status_label());
    println!("{}", info.details);
    if let Ok(storage) = windows_storage_layout() {
        println!("User config root: {}", storage.user_config_root.display());
        // ... existing 4 lines ...
    }
    print_windows_foundation_report("");
    print_windows_wfp_readiness_report("", &wfp);
    let wfp_ready = wfp.status_label == "ready";
    print!("{}", trailing_usage_guidance(wfp_ready));
}
```

---

### `crates/nono-cli/src/cli.rs::SetupArgs` (MODIFY — ADD field)

**Analog:** existing `SetupArgs` field block (lines 1991-2033 — copy any of `register_wfp_service`, `install_wfp_service`, `start_wfp_service`).

**Pattern to copy** (lines 1998-2004):
```rust
/// Register the Windows WFP service (Windows only)
#[arg(long, help_heading = "OPTIONS")]
pub register_wfp_service: bool,

/// Register the Windows WFP service placeholder (Windows only)
#[arg(long, help_heading = "OPTIONS")]
pub install_wfp_service: bool,
```

**New field** (added in the same idiom):
```rust
/// Refresh the cached Sigstore trusted root from https://tuf-repo-cdn.sigstore.dev (per-user, no admin)
#[arg(long, help_heading = "OPTIONS")]
pub refresh_trust_root: bool,
```

**Constructor wiring** (`SetupRunner::new` at lines 25-38) — copy the same field-bool propagation:
```rust
impl SetupRunner {
    pub fn new(args: &SetupArgs) -> Self {
        Self {
            check_only: args.check_only,
            register_wfp_service: args.register_wfp_service,
            install_wfp_service: args.install_wfp_service,
            // ... existing fields ...
            refresh_trust_root: args.refresh_trust_root,  // NEW
            // ...
        }
    }
}
```

---

### `crates/nono-cli/src/cli.rs::TrustVerifyArgs` (MODIFY — ADD `--issuer` + `--identity` fields)

**Analog:** existing `TrustVerifyArgs` (lines 2674-2692) and the optional-field pattern in `TrustSignArgs::keyref` (line 2630-2631).

**Existing struct** (lines 2674-2692):
```rust
#[derive(Parser, Debug)]
#[command(disable_help_flag = true)]
pub struct TrustVerifyArgs {
    /// Instruction file(s) to verify
    #[arg(required_unless_present = "all")]
    pub files: Vec<PathBuf>,

    /// Verify all files matching trust policy patterns in CWD
    #[arg(long)]
    pub all: bool,

    /// Trust policy file (default: auto-discover)
    #[arg(long, value_name = "FILE")]
    pub policy: Option<PathBuf>,

    /// Print help
    #[arg(long, short = 'h', action = clap::ArgAction::Help, help_heading = "OPTIONS")]
    pub help: Option<bool>,
}
```

**New fields** (D-32-08 — pattern lifted from `TrustSignArgs::keyref` at line 2629-2631):
```rust
/// OIDC issuer URL (required for keyless verify; exact match)
#[arg(long, value_name = "URL")]
pub issuer: Option<String>,

/// OIDC identity regex (required for keyless verify; matched against SAN)
#[arg(long, value_name = "REGEX")]
pub identity: Option<String>,
```

> Both stay `Option<String>` — clap doesn't natively express "required when bundle is keyless". The fail-closed enforcement happens at the `run_verify` body in `trust_cmd.rs` (see next section).

---

### `crates/nono-cli/src/trust_cmd.rs::run_verify` + `verify_single_file` + `verify_multi_subject_file` (HARDEN)

**Analog:** the existing `verify_single_file` keyless arm (lines 1026-1043) is the surface to harden in place.

**Existing keyless verify body** (lines 1026-1043):
```rust
trust::SignerIdentity::Keyless { .. } => {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("failed to create async runtime: {e}"))?;
    let trusted_root = rt
        .block_on(trust::load_production_trusted_root())
        .map_err(|e| format!("failed to load Sigstore trusted root: {e}"))?;
    let sigstore_policy = trust::VerificationPolicy::default();
    trust::verify_bundle_with_digest(
        &file_digest_hex,
        &bundle,
        &trusted_root,
        &sigstore_policy,
        file_path,
    )
    .map_err(|e| format!("Sigstore verification failed: {e}"))?;
}
```

**Hardened keyless arm** (after D-32-01 sync rewrite + D-32-08 fail-closed regex enforcement; CRITICAL: regex match happens AFTER `verify_bundle_with_digest` returns OK — see Pitfall 1):
```rust
trust::SignerIdentity::Keyless { issuer: san_issuer, san } => {
    // D-32-08 fail-closed: --issuer and --identity must be provided.
    let user_issuer = args.issuer.as_deref().ok_or_else(||
        "keyless bundle requires --issuer <OIDC_URL> (exact match against signer)".to_string())?;
    let user_identity_pattern = args.identity.as_deref().ok_or_else(||
        "keyless bundle requires --identity <REGEX> (matched against bundle SAN)".to_string())?;

    // D-32-01 sync rewrite — no rt.block_on wrapper needed.
    let trusted_root = trust::load_production_trusted_root()
        .map_err(|e| format!("failed to load Sigstore trusted root: {e}"))?;

    // sigstore-verify policy: ONLY the issuer goes through (exact-equality
    // semantics per sigstore-verify-0.6.5/src/verify.rs:277). Identity regex
    // is OUR-side post-check (Pitfall 1).
    let sigstore_policy = trust::VerificationPolicy::default()
        .with_issuer(user_issuer.to_string());
    trust::verify_bundle_with_digest(
        &file_digest_hex, &bundle, &trusted_root, &sigstore_policy, file_path,
    ).map_err(|e| format!("Sigstore verification failed: {e}"))?;

    // D-32-08 SAN regex post-check.
    let regex = regress::Regex::new(user_identity_pattern)
        .map_err(|e| format!("invalid --identity regex `{user_identity_pattern}`: {e}"))?;
    if regex.find(&san).is_none() {
        return Err(format!(
            "keyless identity mismatch: SAN `{san}` does not match --identity `{user_identity_pattern}`"
        ));
    }
}
```

> **Issuer comparison through `validate_oidc_issuer`** is the recommended hardening if the planner wants URL-component-equality (vs. pure string equality). `crates/nono/src/trust/signing.rs::validate_oidc_issuer` (lines 86-130) is already a `nono` library export and applies the same anti-prefix-attack logic at signing-time. CLI-side reuse keeps the fail-closed contract symmetric.

**Polished `discover_oidc_token` error** (D-32-09; current at lines 658-674):

Existing message body (lines 666-672):
```rust
"no ambient OIDC credentials found. \
 Keyless signing requires a CI environment with OIDC support \
 (e.g., GitHub Actions with `permissions: id-token: write`)."
```

New text per D-32-09 wording (CONTEXT decisions verbatim):
```rust
"no ambient OIDC credentials found. \
 Keyless signing requires a CI environment with OIDC ambient identity \
 (GitHub Actions with `permissions: id-token: write`, GitLab CI, etc.). \
 For local development, use `nono trust sign --keyref <key>` instead."
```

---

### `crates/nono-cli/src/exec_strategy_windows/launch.rs::WindowsTokenArm::BrokerLaunch` (MODIFY — Authenticode gate)

**Analog:** the existing broker-resolution block at lines 1246-1265, plus `crates/nono-cli/src/exec_identity_windows.rs::query_authenticode_status` (lines 119-210, Phase 28 chain-walker — D-32-11/13 reuse).

**Existing dispatch site** (lines 1246-1265 — the broker-resolution block):
```rust
if matches!(arm, WindowsTokenArm::BrokerLaunch) {
    // D-07: Resolve broker path as sibling of the running nono.exe.
    let nono_exe = std::env::current_exe().map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to resolve current_exe for broker location: {e}"
        ))
    })?;
    let exe_dir = nono_exe.parent().ok_or_else(|| {
        NonoError::SandboxInit(format!(
            "Failed to resolve parent dir for broker location: {}",
            nono_exe.display()
        ))
    })?;
    let broker_path = exe_dir.join("nono-shell-broker.exe");
    if !broker_path.exists() {
        return Err(NonoError::BrokerNotFound { path: broker_path });
    }

    // D-02: Mark ConPTY pipe handles inheritable BEFORE CreateProcessW;
    // ...
```

**New Authenticode gate (insert after the broker-not-found check at line 1265, BEFORE handle inheritance work at 1267+)** — RESEARCH § Pattern 5:
```rust
// Phase 32 D-32-11/13/14: Authenticode self-trust-anchor.
// On every dispatch (no cache, D-32-14), require broker.exe's Authenticode
// signer subject + thumbprint to match nono.exe's own. Fail-closed; no
// escape hatch. Dev-build skip via install-layout detector (Pitfall 6).
if !is_dev_build_layout(&nono_exe) {
    use crate::exec_identity_windows::query_authenticode_status;
    use crate::exec_identity::AuthenticodeStatus;

    let nono_status = query_authenticode_status(&nono_exe)?;
    let broker_status = query_authenticode_status(&broker_path)?;

    let (nono_subject, nono_thumbprint) = match nono_status {
        AuthenticodeStatus::Valid { signer_subject, thumbprint } => (signer_subject, thumbprint),
        other => return Err(NonoError::TrustVerification {
            path: nono_exe.display().to_string(),
            reason: format!(
                "nono.exe Authenticode status is {other:?} (expected Valid). \
                 Self-trust-anchor unavailable; refusing to spawn broker."
            ),
        }),
    };
    let (broker_subject, broker_thumbprint) = match broker_status {
        AuthenticodeStatus::Valid { signer_subject, thumbprint } => (signer_subject, thumbprint),
        other => return Err(NonoError::TrustVerification {
            path: broker_path.display().to_string(),
            reason: format!(
                "broker.exe Authenticode status is {other:?} (expected Valid)."
            ),
        }),
    };
    if nono_subject != broker_subject || nono_thumbprint != broker_thumbprint {
        return Err(NonoError::TrustVerification {
            path: broker_path.display().to_string(),
            reason: format!(
                "broker.exe Authenticode signature does not match nono.exe — \
                 expected subject `{nono_subject}` thumbprint `{nono_thumbprint}`, \
                 got subject `{broker_subject}` thumbprint `{broker_thumbprint}`. Refusing to spawn."
            ),
        });
    }
}
```

**Dev-build skip helper** (Pitfall 6 — install-layout detector, NOT `#[cfg(debug_assertions)]`):
```rust
/// Returns true when nono.exe runs out of a Cargo target directory (dev build);
/// false otherwise. Skipping Authenticode verify in dev avoids failing
/// `cargo test --release` when broker.exe is unsigned. See Phase 32 Pitfall 6.
fn is_dev_build_layout(nono_exe_path: &Path) -> bool {
    let s = nono_exe_path.to_string_lossy();
    s.contains(r"\target\debug\") || s.contains(r"\target\release\")
        || s.contains("/target/debug/") || s.contains("/target/release/")
}
```

> Log at `tracing::info!` when the detector activates so production runs (which never hit it) are LOUD about any unexpected activation. RESEARCH Pitfall 6 specifies this.

---

### `crates/nono-cli/tests/keyless_sign.rs` (NEW INTEGRATION TEST)

**Analog:** `crates/nono-cli/tests/audit_attestation.rs::setup_isolated_home` (lines 38-80) — hermetic per-test `<NONO_TEST_HOME>` + tempdir scaffold.

**Pattern to copy — hermetic-test scaffolding** (lines 8-80 from audit_attestation.rs):
```rust
fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

fn run_nono(args: &[&str], home: &Path, cwd: &Path) -> Output {
    let mut cmd = nono_bin();
    cmd.args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        // Phase 27.1 (REQ-NTH-03): NONO_TEST_HOME is the production-code seam.
        .env("NONO_TEST_HOME", home);
    cmd.current_dir(cwd).output().expect("failed to run nono")
}

fn setup_isolated_home() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp_root = std::env::current_dir()
        .expect("cwd")
        .join("target")
        .join("test-artifacts");
    fs::create_dir_all(&temp_root).expect("create temp root");
    let tmp = tempfile::Builder::new()
        .prefix("nono-keyless-sign-it-")  // CHANGE the prefix per Phase 32
        .tempdir_in(&temp_root)
        .expect("tempdir");
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(home.join(".config")).expect("create config dir");
    fs::create_dir_all(home.join("AppData").join("Roaming")).expect("create AppData\\Roaming dir");
    fs::create_dir_all(home.join("AppData").join("Local")).expect("create AppData\\Local dir");
    fs::create_dir_all(home.join(".nono").join("rollbacks")).expect("create rollback dir");
    fs::create_dir_all(home.join(".nono").join("audit")).expect("create audit dir");
    // Phase 32 ADD: pre-create the trust-root cache dir so the test's
    // `nono setup --refresh-trust-root` (or the alternative: pre-seed
    // the frozen-fixture file) lands in the expected location.
    fs::create_dir_all(home.join(".nono").join("trust-root"))
        .expect("create trust-root dir");
    fs::create_dir_all(&workspace).expect("create workspace dir");
    (tmp, home, workspace)
}
```

**Mock Fulcio/Rekor pattern** (NEW — RESEARCH § Pattern 4; not in-tree, planner adds `httpmock@0.7` dev-dep per RESEARCH § Standard Stack):
```rust
#[tokio::test]
async fn keyless_sign_then_verify_roundtrip() {
    let fulcio_mock = httpmock::MockServer::start();
    let _fulcio = fulcio_mock.mock(|when, then| {
        when.method("POST").path("/api/v2/signingCert");
        then.status(200).body(test_fulcio_response_bytes());
    });

    let rekor_mock = httpmock::MockServer::start();
    let _rekor = rekor_mock.mock(|when, then| {
        when.method("POST").path("/api/v1/log/entries");
        then.status(201).body(test_rekor_entry_bytes());
    });

    // Inject mock URLs via SigningContext::with_config (the test seam).
    let config = sigstore_sign::SigningConfig {
        fulcio_url: fulcio_mock.url(""),
        rekor_url: rekor_mock.url(""),
        tsa_url: None,
        signing_scheme: sigstore_sign::SigningScheme::EcdsaP256Sha256,
        rekor_api_version: sigstore_sign::RekorApiVersion::V1,
    };
    let context = sigstore_sign::SigningContext::with_config(config);
    // ... rest of the keyless sign flow + roundtrip verify ...
}
```

> **Mock cert-bytes fixture:** `test_fulcio_response_bytes()` should be a captured-once-good X.509 chain with the right Fulcio v2 OID extensions (1.3.6.1.4.1.57264.1.12 / .14 / .18 — see `bundle.rs:51-67`). Same "captured-once-good" pattern as the frozen TUF root. Open Question 4 in RESEARCH leaves capture-once vs. on-demand-generation up to the planner; researcher recommends capture-once for first cut.

---

### `crates/nono-cli/tests/keyless_verify.rs` (NEW INTEGRATION TEST)

**Analog:** `crates/nono-cli/tests/audit_attestation.rs::run_nono` (lines 12-27) — subprocess-launching with NONO_TEST_HOME.

**Pattern to copy — `--issuer`/`--identity` fail-closed enforcement test** (subprocess shape from audit_attestation.rs lines 12-36 — the assert helpers `assert_success` are the model):
```rust
#[test]
fn keyless_verify_fails_when_issuer_missing() {
    let (_tmp, home, workspace) = setup_isolated_home();
    // Place a known-keyless bundle at workspace + companion file.
    // ...
    let output = run_nono(
        &["trust", "verify", "instruction.md"],   // missing --issuer + --identity
        &home,
        &workspace,
    );
    assert!(!output.status.success(),
        "verify must fail-closed when --issuer is missing");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--issuer"),
        "error must name the missing flag; got: {stderr}");
}

#[test]
fn keyless_verify_fails_when_identity_missing() { /* symmetric */ }

#[test]
fn keyless_verify_succeeds_with_matching_identity_regex() {
    // Positive case: --identity '^https://github\.com/.*/release\.yml@.*$'
    // matches the keyless bundle's SAN.
}

#[test]
fn keyless_verify_fails_with_nonmatching_identity_regex() {
    // Negative case: --identity '^https://gitlab\.com/.*$' against a
    // GitHub-issued bundle SAN — fail-closed with diagnostic naming SAN.
}

#[test]
fn discover_oidc_token_error_suggests_keyref() {
    // D-32-09: ensure `nono trust sign --keyless` (no ambient OIDC) error
    // mentions `--keyref` for local-dev recovery.
    let (_tmp, home, workspace) = setup_isolated_home();
    let output = run_nono(
        &["trust", "sign", "--keyless", "instruction.md"],
        &home,
        &workspace,
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--keyref"),
        "error must suggest --keyref for local dev; got: {stderr}");
}
```

---

### `crates/nono-cli/tests/broker_authenticode.rs` (NEW INTEGRATION TEST, `#[cfg(target_os = "windows")]`)

**Analog:** `crates/nono-cli/src/exec_strategy_windows/launch.rs::broker_dispatch_tests::broker_launch_assigns_child_to_job_object` (lines 2247-2284) — the broker artifact resolution + SKIP-when-missing pattern. Plus `crates/nono-cli/tests/exec_identity_windows.rs` (subprocess regression style, top of file `#![cfg(target_os = "windows")]`).

**Header pattern** (copy from `tests/exec_identity_windows.rs:34-35`):
```rust
#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]
```

**Broker-artifact resolution + SKIP-when-missing pattern** (lines 2253-2284):
```rust
let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
let workspace_root = PathBuf::from(&manifest).join("..").join("..");
let candidate_triple = workspace_root
    .join("target")
    .join("x86_64-pc-windows-msvc")
    .join("release")
    .join("nono-shell-broker.exe");
let candidate_default = workspace_root
    .join("target")
    .join("release")
    .join("nono-shell-broker.exe");
let broker_path = if candidate_triple.exists() {
    candidate_triple
} else if candidate_default.exists() {
    candidate_default
} else {
    eprintln!(
        "SKIP: broker artifact missing at {} and {} — pre-build via \
         `cargo build -p nono-shell-broker --release --target x86_64-pc-windows-msvc`.",
        candidate_triple.display(),
        candidate_default.display()
    );
    return;
};
```

**Test cases (D-32-11/12/13/14 acceptance):**
1. `same_signer_succeeds` — uses Phase 31 release-pipeline-built broker (matches `nono.exe`) → verify succeeds.
2. `broker_subject_mismatch_refuses_spawn` — needs two pre-signed test artifacts with DIFFERENT subjects (RESEARCH § Wave 0 Gaps lists this). Asserts `NonoError::TrustVerification` with text containing "Authenticode signature does not match nono.exe".
3. `unsigned_broker_refuses_spawn` — supplies a stub binary that's unsigned. Asserts `NonoError::TrustVerification` with text containing "expected Valid".
4. `dev_build_layout_skips_verify` — runs against a `target/debug/` path; asserts no Authenticode error fires (the install-layout detector kicks in).
5. `every_dispatch_re_verifies` — D-32-14: invokes broker dispatch twice, asserts `query_authenticode_status` is called twice (no cache).

---

### `crates/nono/tests/fixtures/trust-root-frozen.json` (NEW STATIC ASSET)

**Analog:** No in-tree TUF fixture exists. Closest convention is `crates/nono/src/trust/bundle.rs:1037+` test fixture (Sigstore-signed bundle from `always-further/test-sk-prov`) — same captured-once-good pattern.

**Capture procedure** (RESEARCH § Pattern 3):
1. On a developer machine with network access, run a one-shot helper that:
   ```rust
   let root = sigstore_verify::trust_root::TrustedRoot::production().await?;
   let json = serde_json::to_string_pretty(&root)?;
   std::fs::write("crates/nono/tests/fixtures/trust-root-frozen.json", json)?;
   ```
2. Commit the file.
3. D-32-06: pinned indefinitely; no rotation job.
4. Estimated size: ~30-100 KB (5 Fulcio CAs + 4 Rekor logs + 4 TSA chains).

---

## Shared Patterns

### Authentication / Trust-anchor
**Source:** `crates/nono-cli/src/exec_identity_windows.rs::query_authenticode_status` (lines 119-210)
**Apply to:** `exec_strategy_windows/launch.rs::WindowsTokenArm::BrokerLaunch` (D-32-13/14)

```rust
// Phase 28 chain-walker primitive (callable, fail-closed, RAII close-guard).
#[must_use = "ignoring the AuthenticodeStatus drops audit evidence"]
pub fn query_authenticode_status(path: &Path) -> Result<AuthenticodeStatus> {
    // ... WinVerifyTrust(WTD_CHOICE_FILE, WTD_REVOKE_NONE) + RAII close-guard ...
    // Returns Valid { signer_subject, thumbprint } | Unsigned | InvalidSignature | QueryFailed
}
```

`AuthenticodeStatus` enum (lines 70-101): four variants, fail-closed shape (REQ-AUDC-03). Phase 32 calls this twice per broker dispatch (once on `current_exe()`, once on broker path) and compares `signer_subject` + `thumbprint` for equality.

### Error Handling
**Source:** `crates/nono/src/error.rs` (lines 50-52, 187-194)
**Apply to:** All Phase 32 fail-closed paths (D-32-03, D-32-05, D-32-09, D-32-12)

Error variants — already in tree, no new variants needed:
```rust
// crates/nono/src/error.rs:50-52
#[error("Broker binary not found: {path:?}")]
BrokerNotFound { path: PathBuf },

// crates/nono/src/error.rs:187-194
#[error("Trust verification failed for {path}: {reason}")]
TrustVerification { path: String, reason: String },

#[error("Signing failed for {path}: {reason}")]
TrustSigning { path: String, reason: String },

#[error("Trust policy error: {0}")]
TrustPolicy(String),
```

| Decision | Variant | When |
|----------|---------|------|
| D-32-03 (cache expired) | `TrustVerification { path, reason }` | path = cache file path; reason = "Sigstore trusted root expired YYYY-MM-DD; run `nono setup --refresh-trust-root` (requires network)." |
| D-32-05 (cache missing) | `TrustPolicy(String)` | "Sigstore trusted root not initialized; run `nono setup --refresh-trust-root` (requires network)." |
| D-32-09 (no ambient OIDC) | `TrustSigning { path, reason }` (existing call) | reason mentions `--keyref` for local dev recovery |
| D-32-12 (broker mismatch) | `TrustVerification { path, reason }` | path = broker path; reason names expected vs. actual subject + thumbprint |

### Fail-Closed-with-Recovery diagnostic convention
**Source:** `crates/nono-cli/src/exec_strategy_windows/network.rs` (lines 425-441) — `--block-net` WFP-required diagnostic
**Apply to:** All four Phase 32 fail-closed paths (D-32-03/05/09/12)

Naming-the-recovery-command pattern (lines 425-431):
```rust
"the WFP service `{}` is not registered. Run `nono setup --install-wfp-service` first",
// ...
"the WFP service `{}` is registered but not running. Run `nono setup --start-wfp-service` first",
```

Phase 32 messages must follow the same shape: `"<problem> — run `<exact recovery command>` <prerequisite>"`. Examples:
- D-32-03: `"Sigstore trusted root expired 2026-05-09; run `nono setup --refresh-trust-root` (requires network)."`
- D-32-05: `"Sigstore trusted root not initialized; run `nono setup --refresh-trust-root` (requires network)."`
- D-32-12: `"broker.exe Authenticode signature does not match nono.exe — expected subject `<X>` thumbprint `<Y>`, got subject `<X'>` thumbprint `<Y'>`. Refusing to spawn."`

### Validation (regex / OIDC issuer)
**Source:** `crates/nono/src/trust/signing.rs::validate_oidc_issuer` (lines 86-130) — URL-component-equality, anti-prefix-attack
**Apply to:** `trust_cmd.rs::run_verify` keyless `--issuer` enforcement (D-32-08)

```rust
pub fn validate_oidc_issuer(iss: &str, pin: &str) -> Result<()> {
    let iss_url = url::Url::parse(iss).map_err(|e|
        NonoError::ConfigParse(format!("OIDC issuer URL '{iss}' is not a valid URL: {e}")))?;
    let pin_url = url::Url::parse(pin).map_err(|e|
        NonoError::ConfigParse(format!("OIDC issuer pin '{pin}' is not a valid URL: {e}")))?;
    // scheme + host + port component equality (NOT string prefix-match).
    // ...
}
```

**Identity regex** uses the existing `regress` crate dependency (`crates/nono/Cargo.toml:26` — `regress = "0.11"`), already pulled in for OIDC issuer pinning. Do NOT add another regex crate.

### Test scaffold (hermetic, `<NONO_TEST_HOME>`-isolated)
**Source:** `crates/nono-cli/tests/audit_attestation.rs::setup_isolated_home` (lines 38-80) and `run_nono` (lines 12-27)
**Apply to:** All three new Phase 32 test files (`keyless_sign.rs`, `keyless_verify.rs`, `broker_authenticode.rs`)

Key pieces:
- `tempfile::Builder::new().prefix(...).tempdir_in("target/test-artifacts")` — per-test ephemeral home.
- `.env("NONO_TEST_HOME", home)` — Phase 27.1 production-code seam.
- Pre-create `<home>/.nono/{rollbacks, audit, trust-root}` and Windows-style `<home>/AppData/{Roaming, Local}` so `dirs` resolves on Windows.
- `String::from_utf8_lossy(&output.stderr)` for assertions — matches existing convention.

### Path security (`nono_home_dir()` honoring `NONO_TEST_HOME`)
**Source:** `crates/nono-cli/src/config/mod.rs::nono_home_dir` (lines 100-133)
**Apply to:** D-32-01 cache directory location

```rust
pub fn nono_home_dir() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("NONO_TEST_HOME") {
        let path = PathBuf::from(&value);
        if !path.is_absolute() {
            return Err(NonoError::EnvVarValidation {
                var: "NONO_TEST_HOME".to_string(),
                reason: format!("must be an absolute path, got: {}", value),
            });
        }
        warn_once_test_home(&path);
        return Ok(path);
    }
    dirs::home_dir().ok_or(NonoError::HomeNotFound)
}
```

D-32-01 cache path: `crate::config::nono_home_dir()?.join(".nono").join("trust-root").join("trusted_root.json")` — same idiom as `audit_session.rs:36`, `hooks.rs:73`, `rollback_session.rs:40`.

---

## Async-vs-sync Function-Shape Inventory (CRITICAL)

Phase 32 D-32-15 #1 deliberately changes `pub async fn load_production_trusted_root()` → `pub fn load_production_trusted_root()`. The planner must touch all five caller sites to drop the now-unused `tokio::runtime::Builder::new_current_thread()` block + `rt.block_on(...)` wrapper:

| File | Line | Current | Target after rewrite |
|------|------|---------|----------------------|
| `crates/nono-cli/src/trust_cmd.rs` | 907-913 (in `verify_multi_subject_file`) | `let rt = ...; let trusted_root = rt.block_on(trust::load_production_trusted_root())` | `let trusted_root = trust::load_production_trusted_root()` |
| `crates/nono-cli/src/trust_cmd.rs` | 1027-1033 (in `verify_single_file`) | same | same |
| `crates/nono-cli/src/trust_intercept.rs` | 371-376 | `let rt = ...; rt.block_on(trust::load_production_trusted_root())` | `trust::load_production_trusted_root()` |
| `crates/nono-cli/src/trust_scan.rs` | 247-255 | same | same |
| `crates/nono-cli/src/trust_scan.rs` | 753-760 | same | same |
| `crates/nono-cli/src/package_cmd.rs` | 446-452 | `let rt = ...; let trusted_root = rt.block_on(nono::trust::load_production_trusted_root())` | `let trusted_root = nono::trust::load_production_trusted_root()` |

Removing the runtime build also removes the surrounding `tokio::runtime::Builder::new_current_thread().enable_all().build()` boilerplate at each site (the runtime exists only to host this one call). Watch for `clippy::unused_async` / `unused_imports` after the rewrite.

**One async caller stays async:** `trust_cmd.rs:385-394` — `run_sign_keyless` builds a runtime to drive `discover_oidc_token` + `sign_attestation`. That's separate (sign-side, not verify-side) and unchanged.

---

## File Sizes (for planner sequencing)

| File | LOC |
|------|-----|
| `crates/nono-cli/src/setup.rs` | 1157 |
| `crates/nono-cli/src/cli.rs` | 4381 |
| `crates/nono-cli/src/trust_cmd.rs` | 1871 |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | 2530 |
| `crates/nono-cli/src/exec_identity_windows.rs` | 691 |
| `crates/nono-cli/src/audit_attestation.rs` | 507 |
| `crates/nono-cli/tests/audit_attestation.rs` | 767 |
| `crates/nono/src/trust/bundle.rs` | ~1100 (production code + tests) |

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/nono/tests/fixtures/trust-root-frozen.json` | static test asset | n/a | No prior TUF-root fixture. Closest convention is `bundle.rs:1037+` Sigstore-signed bundle test (captured-once-good). Procedure: capture via on-demand `TrustedRoot::production().await` + `serde_json::to_string_pretty`, commit verbatim. |
| Mock Fulcio/Rekor cert-bytes (`test_fulcio_response_bytes`, `test_rekor_entry_bytes` in `keyless_sign.rs`) | mock HTTP fixture | event-driven | No prior mock-server-with-Fulcio-cert fixture. Open Question 4 in RESEARCH leaves capture-once vs. on-demand-generation up to the planner; researcher recommends capture-once for first cut. |
| Two pre-signed test artifacts with DIFFERENT subjects (for `broker_authenticode.rs::broker_subject_mismatch_refuses_spawn`) | static test asset | n/a | No prior in-tree pair of differently-signed binaries. Wave 0 Gap in RESEARCH. Options: (a) ship 2KB stub binaries each signed by a self-generated test CA, committed to `crates/nono-cli/tests/fixtures/`; (b) generate at test-time with `signtool` if present, SKIP cleanly otherwise. |

---

## Metadata

**Analog search scope:**
- `crates/nono/src/trust/` (full)
- `crates/nono-cli/src/{setup.rs, cli.rs, trust_cmd.rs, exec_identity_windows.rs, audit_attestation.rs, config/mod.rs}`
- `crates/nono-cli/src/exec_strategy_windows/{launch.rs, network.rs}`
- `crates/nono-cli/tests/{audit_attestation.rs, exec_identity_windows.rs, wfp_port_integration.rs}`
- `crates/nono/src/error.rs`

**Files scanned:** 13
**Pattern extraction date:** 2026-05-10
