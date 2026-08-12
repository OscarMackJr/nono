use crate::cli::{Cli, Commands};
use crate::telemetry::SecurityEventLayer;
use crate::{config, theme};
use nono::TelemetryConfig;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
#[cfg(target_os = "windows")]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

/// Phase 117-21 CR-02: tracks whether `init_tracing_with_security` selected
/// the private (file-log-succeeded) arm rather than one of the two arms that
/// route to `std::io::stderr` (the default no-`--log-file` arm and the
/// file-open-failure fallback arm) — both of which the confined child can
/// read back on the non-detached-stdio path (D-28). Defaults to `false`
/// (conservative: assume shared/readable unless proven private) so that any
/// caller running before `init_tracing`/`init_tracing_with_security` sees
/// the safe answer.
///
/// `#[cfg(target_os = "windows")]`: the only consumer,
/// `exec_strategy_windows::launch::apply_startup_attestation_gate`, is
/// itself Windows-only — this mirrors `output::print_attestation_downgrade_
/// banner`'s own Windows-only gating (CINT-02 is a Windows-only
/// self-attestation pass, D-09). Un-gating this item makes it dead code on
/// Linux/macOS builds under `-D warnings` (verified via the cross-target
/// clippy gate, D-35/D-11).
#[cfg(target_os = "windows")]
static TRACING_LOG_TARGET_IS_PRIVATE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Phase 117-27 WR-16: the canonical `--log-file` path recorded when
/// `init_tracing_with_security` selects the file-log-succeeded arm. `None`
/// means either that arm was never selected, or (fail-secure) no
/// granted-path check has run yet for this launch.
#[cfg(target_os = "windows")]
static TRACING_LOG_TARGET_PATH: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Phase 117-27 WR-16: the resolved filesystem paths this launch's own
/// `CapabilitySet` grants the confined child (read or write). Populated once
/// per launch by [`set_granted_read_paths`] before the child is ever
/// resumed, so `log_target_is_private()` can check whether the log file the
/// operator thinks is private actually sits inside a directory the child can
/// itself read.
///
/// `None` means (fail-secure) no granted-path check has run yet for this
/// launch — deliberately DISTINCT from `Some(vec![])`, which means the check
/// ran and this launch grants the child nothing. Phase 117-42 (WR-23): this
/// was a bare `Mutex<Vec<PathBuf>>` initialised to `Vec::new()`, so those two
/// states were indistinguishable and the "no granted-path check has run" case
/// documented on [`log_target_is_private`] could not be implemented. The
/// final expression there is `!granted_paths.iter().any(..)`, which on an
/// empty vec is `!false` = `true` ("private") — so the D-28 gate OPENED for
/// any path reaching it before [`set_granted_read_paths`] ran. Mirrors
/// [`TRACING_LOG_TARGET_PATH`]'s shape one declaration up.
#[cfg(target_os = "windows")]
static GRANTED_READ_PATHS: std::sync::Mutex<Option<Vec<PathBuf>>> = std::sync::Mutex::new(None);

/// Test-only override (Phase 117-27 WR-16): when `Some(v)`,
/// `log_target_is_private()` returns `v` immediately, preserving the
/// pre-WR-16 external behavior of [`set_log_target_is_private_for_test`] for
/// tests that only care about the private/shared distinction and not about
/// granted-path validation.
#[cfg(all(test, target_os = "windows"))]
static LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE: std::sync::Mutex<Option<bool>> =
    std::sync::Mutex::new(None);

/// Returns `true` only after `init_tracing_with_security` has selected the
/// `Some(path) => Ok(writer)` arm (the file-log target opened successfully)
/// AND (Phase 117-27 WR-16) the resolved log path falls OUTSIDE every path
/// this launch's own `CapabilitySet` grants the confined child. Callers
/// deciding whether a message may carry specific layer/mechanism names
/// should gate on this (D-28) rather than assume `tracing::warn!` never
/// reaches a channel the confined child can read.
///
/// Fail-secure on every unknown/error case (WR-16's own stated rule):
/// no `--log-file` arm selected, no granted-path check has run, or the
/// stored log path fails to canonicalize all return `false` ("not private").
///
/// # Fail-secure cases and the test that pins each (Phase 117-42, WR-23)
///
/// WR-16 wrote the rule above as prose and nothing checked it against the
/// code, which is how WR-23 shipped: the "no granted-path check has run"
/// clause was documented for two rounds while the data model could not
/// express it. Every early return below now maps to a named test, so a future
/// edit that adds a fail-secure branch without one is visible in review.
///
/// | Documented case | Branch | Test |
/// |---|---|---|
/// | file-log arm never selected | `!TRACING_LOG_TARGET_IS_PRIVATE` | `no_log_path_recorded_is_not_private` |
/// | arm selected, no path recorded | `stored_path == None` | `log_arm_selected_but_no_path_recorded_is_not_private` |
/// | stored path fails to canonicalize | `canonicalize(..).is_err()` | `uncanonicalizable_log_path_is_not_private` |
/// | no granted-path check has run | `granted_paths == None` | `no_granted_paths_recorded_is_not_private` |
/// | check ran, found nothing | `Some(vec![])` → not a gap | `granted_path_check_ran_and_found_nothing_is_still_private` |
/// | log path inside a granted path | final `any(..)` | `log_path_inside_a_granted_directory_is_not_private` |
/// | log path outside every granted path | final `any(..)` | `log_path_outside_every_granted_directory_is_private` |
#[cfg(target_os = "windows")]
pub(crate) fn log_target_is_private() -> bool {
    #[cfg(test)]
    {
        if let Some(v) = *LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            return v;
        }
    }

    if !TRACING_LOG_TARGET_IS_PRIVATE.load(std::sync::atomic::Ordering::Relaxed) {
        return false;
    }

    let stored_path = TRACING_LOG_TARGET_PATH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let Some(stored_path) = stored_path else {
        // File-log arm was selected but no path was ever recorded — an
        // internal inconsistency. Fail-secure: not private.
        return false;
    };

    let Ok(canonical_log_path) = std::fs::canonicalize(&stored_path) else {
        // Cannot resolve the log path (e.g. it no longer exists). Fail-secure.
        return false;
    };

    let granted_paths = GRANTED_READ_PATHS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let Some(granted_paths) = granted_paths else {
        // WR-23: no granted-path check has run for this launch. The doc
        // comment above has always claimed this returns `false`; before Phase
        // 117-42 the state was not representable, so an unpopulated check
        // reported "private" and opened the D-28 gate. Note this is NOT the
        // same as `Some(vec![])` — a check that RAN and found this launch
        // grants nothing must still be able to report a log target private.
        return false;
    };

    !granted_paths.iter().any(|granted| {
        // A not-yet-existing grant target must still be checked — never
        // skipped — so fall back to the stored path on canonicalization
        // failure rather than dropping the comparison.
        //
        // WR-24 (Phase 117-42) questioned whether this fallback can ever
        // match, on the grounds that `canonical_log_path` is verbatim
        // (`\\?\C:\...`, `Prefix(VerbatimDisk)`) while a raw path would be
        // `Prefix(Disk)`, and `Path::starts_with` is component-wise. The first
        // half is correct and is now pinned by
        // `verbatim_and_non_verbatim_prefixes_do_not_compare_equal`.
        //
        // The conclusion does not follow for production inputs, though: every
        // value reaching `granted` is a `FsCapability::resolved`, which is
        // ALWAYS produced by `path.canonicalize()` (`capability.rs:110`,
        // `:144`, `:361`, `:457`) and is therefore already verbatim — including
        // the not-yet-existing-file case, which joins a canonicalized parent
        // with a file name (`capability.rs:392-405`). The only site that ever
        // assigns a non-canonical `resolved` is `remap_procfs_self_references`
        // (`capability.rs:1455`), which rewrites Linux `/proc/self` paths and
        // cannot reach this Windows-only function. So the fallback compares
        // verbatim against verbatim and DOES preserve the comparison, exactly
        // as the first paragraph claims. Pinned by
        // `fallback_branch_still_matches_for_a_verbatim_nonexistent_grant`.
        let canonical_granted = std::fs::canonicalize(granted).unwrap_or_else(|_| granted.clone());
        // CLAUDE.md path security: component-wise `Path::starts_with`, never
        // a string `starts_with` on the rendered path.
        canonical_log_path.starts_with(&canonical_granted)
    })
}

/// Phase 117-27 WR-16: record the resolved filesystem paths this launch's
/// own `CapabilitySet` grants the confined child, once per launch, before
/// `apply_startup_attestation_gate` is ever reachable. Consumed by
/// [`log_target_is_private`]'s granted-path check.
#[cfg(target_os = "windows")]
pub(crate) fn set_granted_read_paths(paths: Vec<PathBuf>) {
    *GRANTED_READ_PATHS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(paths);
}

/// Test-only seam (Phase 117-21 Task 2, rewritten Phase 117-27 WR-16) to
/// drive both branches of `log_target_is_private()` deterministically
/// without going through a real `init_tracing_with_security` call or the
/// granted-path computation. External signature/behavior unchanged from
/// Phase 117-21 — existing `launch.rs` call sites are unaffected.
#[cfg(all(test, target_os = "windows"))]
pub(crate) fn set_log_target_is_private_for_test(private: bool) {
    *LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(private);
}

/// Test-only seam (Phase 117-27 WR-16): sets the recorded log-target path
/// and clears [`LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE`] so `log_target_is_private()`
/// runs the REAL granted-path computation rather than the override forcer.
/// Callers must also set `TRACING_LOG_TARGET_IS_PRIVATE` (directly, via
/// `super::TRACING_LOG_TARGET_IS_PRIVATE`) to simulate the file-log arm
/// having been selected — this seam only records the path.
#[cfg(all(test, target_os = "windows"))]
pub(crate) fn set_log_target_path_for_test(path: Option<PathBuf>) {
    *LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    *TRACING_LOG_TARGET_PATH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = path;
}

/// Test-only seam (Phase 117-27 WR-16): sets the recorded granted-read-paths
/// list and clears [`LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE`] so
/// `log_target_is_private()` runs the REAL granted-path computation rather
/// than the override forcer.
#[cfg(all(test, target_os = "windows"))]
pub(crate) fn set_granted_read_paths_for_test(paths: Vec<PathBuf>) {
    *LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    set_granted_read_paths(paths);
}

/// Test-only cleanup seam (Phase 117-27 WR-16): resets every static this
/// module's granted-path computation reads/writes to its safe default.
/// Tests call this both before and after driving the gate, mirroring the
/// existing serialization discipline (see [`lock_log_target_is_private_test`]).
#[cfg(all(test, target_os = "windows"))]
pub(crate) fn clear_log_target_is_private_test_state() {
    TRACING_LOG_TARGET_IS_PRIVATE.store(false, std::sync::atomic::Ordering::Relaxed);
    *TRACING_LOG_TARGET_PATH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    // WR-23: restore the NOT-YET-POPULATED state (`None`), not
    // `Some(Vec::new())` — the clear helper's job is to return every static to
    // its pre-launch default, and getting this backwards would silently make
    // every other test in this module exercise the wrong branch.
    *GRANTED_READ_PATHS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    *LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

/// Process-global lock serializing tests that drive
/// `TRACING_LOG_TARGET_IS_PRIVATE` via [`set_log_target_is_private_for_test`].
///
/// Rust unit tests run in parallel within the same process (same hazard
/// class as `test_env::ENV_LOCK`, documented there): without this lock two
/// tests setting opposite values race on the shared `AtomicBool` and observe
/// each other's state mid-assertion.
#[cfg(all(test, target_os = "windows"))]
pub(crate) static LOG_TARGET_IS_PRIVATE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Acquire [`LOG_TARGET_IS_PRIVATE_TEST_LOCK`], recovering from poisoning the
/// same way `test_env::lock_env` does (a prior panicking test must not
/// permanently deadlock every subsequent test in this binary).
#[cfg(all(test, target_os = "windows"))]
pub(crate) fn lock_log_target_is_private_test() -> std::sync::MutexGuard<'static, ()> {
    match LOG_TARGET_IS_PRIVATE_TEST_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(crate) fn normalize_legacy_flag_env_vars() {
    copy_legacy_env_var("NONO_NET_BLOCK", "NONO_BLOCK_NET");
    copy_legacy_env_var("NONO_NET_ALLOW", "NONO_ALLOW_NET");
    copy_legacy_env_var("NONO_ALLOW_PROXY", "NONO_ALLOW_DOMAIN");
    copy_legacy_env_var("NONO_PROXY_ALLOW", "NONO_ALLOW_DOMAIN");
    copy_legacy_env_var("NONO_PROXY_CREDENTIAL", "NONO_CREDENTIAL");
    copy_legacy_env_var("NONO_EXTERNAL_PROXY", "NONO_UPSTREAM_PROXY");
    copy_legacy_env_var("NONO_EXTERNAL_PROXY_BYPASS", "NONO_UPSTREAM_BYPASS");
}

pub(crate) fn collect_legacy_network_warnings() -> Vec<String> {
    let mut warnings = Vec::new();
    let args: Vec<String> = std::env::args().skip(1).collect();

    // (legacy, replacement, remove_by)
    for (legacy, replacement, remove_by) in [
        (
            "--allow-net",
            Some("network is unrestricted by default"),
            None,
        ),
        (
            "--net-allow",
            Some("network is unrestricted by default"),
            None,
        ),
        ("--allow-proxy", Some("--allow-domain"), None),
        ("--proxy-allow", Some("--allow-domain"), None),
        ("--proxy-credential", Some("--credential"), Some("v1.0.0")),
        ("--allow-bind", Some("--listen-port"), None),
        ("--allow-port", Some("--open-port"), None),
        ("--external-proxy", Some("--upstream-proxy"), None),
        ("--external-proxy-bypass", Some("--upstream-bypass"), None),
        ("--net-block", Some("--block-net"), None),
    ] {
        if args
            .iter()
            .any(|arg| arg == legacy || arg.starts_with(&format!("{legacy}=")))
        {
            let mut message = if let Some(replacement) = replacement {
                format!("Warning: `{legacy}` is deprecated; use `{replacement}` instead.")
            } else {
                format!("Warning: `{legacy}` is deprecated.")
            };
            if let Some(v) = remove_by {
                message.push_str(&format!(" Will be removed in {v}."));
            }
            warnings.push(message);
        }
    }

    // (legacy, replacement, remove_by)
    for (legacy, replacement, remove_by) in [
        ("NONO_NET_BLOCK", "NONO_BLOCK_NET", None),
        ("NONO_NET_ALLOW", "NONO_ALLOW_NET", None),
        ("NONO_ALLOW_PROXY", "NONO_ALLOW_DOMAIN", None),
        ("NONO_PROXY_ALLOW", "NONO_ALLOW_DOMAIN", None),
        ("NONO_PROXY_CREDENTIAL", "NONO_CREDENTIAL", Some("v1.0.0")),
        ("NONO_EXTERNAL_PROXY", "NONO_UPSTREAM_PROXY", None),
        ("NONO_EXTERNAL_PROXY_BYPASS", "NONO_UPSTREAM_BYPASS", None),
    ] {
        if std::env::var_os(legacy).is_some() {
            let mut message =
                format!("Warning: `{legacy}` is deprecated; use `{replacement}` instead.");
            if let Some(v) = remove_by {
                message.push_str(&format!(" Will be removed in {v}."));
            }
            warnings.push(message);
        }
    }

    warnings
}

pub(crate) fn print_legacy_network_warnings(warnings: &[String], silent: bool) {
    if silent {
        return;
    }

    for warning in warnings {
        eprintln!("  [nono] {warning}");
    }
}

pub(crate) fn init_theme(cli: &Cli) {
    let config_theme = config::user::load_user_config()
        .ok()
        .flatten()
        .and_then(|config| config.ui.theme);

    theme::init(cli.theme.as_deref(), config_theme.as_deref());
}

/// Initialize the global tracing subscriber.
///
/// # Arguments
///
/// - `cli` — parsed CLI arguments (controls verbosity, log-file path, silent mode).
/// - `telemetry_config` — optional telemetry configuration read from the HKLM
///   `MachineEgressPolicy` (Phase 83).  `None` uses [`TelemetryConfig::default()`]
///   which is default-ON per D-13.
///
/// # SecurityEventLayer registration (TELEM-04 SC-4)
///
/// A [`SecurityEventLayer`] is constructed from `telemetry_config` (or the
/// default) and registered in all three subscriber arms (file-log, file-fallback,
/// stderr).  On Windows, a `tracing-etw` layer is also added so that the
/// `tracing::warn!(target: "nono_security", …)` calls in
/// [`crate::telemetry::windows::emit_security_event`] are forwarded to the
/// registered ETW provider "nono".
pub(crate) fn init_tracing(cli: &Cli, telemetry_config: Option<TelemetryConfig>) {
    // ── Build the SecurityEventLayer (all platforms) ──────────────────────────
    //
    // Generate a per-session ID (16 hex chars from a random u64).
    // `rand` is an unconditional dep in Cargo.toml.
    let session_id = {
        use rand::RngExt as _;
        let mut rng = rand::rng();
        let mut buf = [0u8; 8];
        rng.fill(&mut buf[..]);
        buf.iter().map(|b| format!("{b:02x}")).collect::<String>()
    };

    let config = telemetry_config.unwrap_or_default();
    let security_layer = SecurityEventLayer::new(config, session_id);

    // Phase 92 Plan 03 (OQ-1 resolution): expose the layer for direct access
    // from execute_sandboxed (AUD-04 gate). SecurityEventLayer::clone() shares
    // the same Arc<Mutex<...>> inner, so both clones advance the same chain.
    // OnceLock::set silently fails if already set (double-init guard).
    let _ = crate::telemetry::SECURITY_LAYER.set(security_layer.clone());

    // Delegate to the platform-specific initialization that adds the ETW layer
    // (Windows) or skips it (non-Windows).  The separate helper avoids having
    // tracing-etw's complex generic types flow through all three match arms.
    init_tracing_with_security(cli, security_layer);
}

/// Inner tracing initialization — registers SecurityEventLayer in all three
/// subscriber arms (file-log, file-fallback, stderr).
///
/// On Windows, a `tracing-etw` LayerBuilder layer for the "nono" ETW provider
/// is registered so that the `tracing::warn!(target: "nono_security", ...)` calls
/// in [`crate::telemetry::windows::emit_security_event`] forward to ETW (D-01.1).
/// If ETW layer construction fails, we continue without it (D-03 non-fatal).
fn init_tracing_with_security(cli: &Cli, security_layer: SecurityEventLayer) {
    let env_filter = tracing_filter(cli);

    match cli.log_file.as_deref() {
        Some(path) => match SharedFileMakeWriter::new(path) {
            Ok(writer) => {
                // Phase 117-21 CR-02: only this arm's target is private to
                // the operator (a file the confined child does not share).
                // cfg-gated with the static's own definition above (Windows-
                // only consumer).
                //
                // Phase 117-27 WR-16: also record the path itself so
                // `log_target_is_private()` can check it against this
                // launch's own granted filesystem policy — "a file was
                // opened" alone is not proof the confined child cannot read
                // it back.
                #[cfg(target_os = "windows")]
                {
                    TRACING_LOG_TARGET_IS_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
                    *TRACING_LOG_TARGET_PATH
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) =
                        Some(path.to_path_buf());
                }
                let fmt_layer = tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_ansi(false)
                    .with_writer(writer);
                init_registry(env_filter, fmt_layer, security_layer);
            }
            Err(err) => {
                eprintln!(
                    "nono: failed to open log file {}: {}; falling back to stderr",
                    path.display(),
                    err
                );
                let fmt_layer = tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_writer(std::io::stderr);
                init_registry(env_filter, fmt_layer, security_layer);
            }
        },
        None => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_writer(std::io::stderr);
            init_registry(env_filter, fmt_layer, security_layer);
        }
    }
}

/// Compose the tracing registry with the format layer and security layer,
/// then call `.init()`.
///
/// On Windows this function also adds the tracing-etw layer (D-01.1).
///
/// The env_filter is applied as a per-layer filter on the fmt_layer via
/// `.with_filter(env_filter)` so that the SecurityEventLayer always receives
/// its events regardless of the verbosity setting (security events should
/// always pass through even when the log level is "off").
fn init_registry<W>(
    env_filter: EnvFilter,
    fmt_layer: tracing_subscriber::fmt::Layer<
        tracing_subscriber::Registry,
        tracing_subscriber::fmt::format::DefaultFields,
        tracing_subscriber::fmt::format::Format,
        W,
    >,
    security_layer: SecurityEventLayer,
) where
    W: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + Send + Sync + 'static,
{
    // Scope the fmt_layer to the env_filter so it only emits at the configured
    // verbosity.  SecurityEventLayer is unfiltered (always active).
    let filtered_fmt = fmt_layer.with_filter(env_filter);

    #[cfg(not(target_os = "windows"))]
    tracing_subscriber::registry()
        .with(filtered_fmt)
        .with(security_layer)
        .init();

    #[cfg(target_os = "windows")]
    {
        // Build the ETW layer for the "nono" provider (D-01.1).
        // If build() fails, continue without ETW (D-03 non-fatal).
        let base = tracing_subscriber::registry()
            .with(filtered_fmt)
            .with(security_layer);

        // The ETW layer type is fully determined here by `base`'s concrete type.
        match tracing_etw::LayerBuilder::new("nono").build() {
            Ok(etw_layer) => base.with(etw_layer).init(),
            Err(e) => {
                eprintln!("nono: telemetry: ETW layer init failed ({e}); ETW emit disabled");
                base.init();
            }
        }
    }
}

#[allow(clippy::disallowed_methods)] // Single-threaded at process startup, before any threads.
fn copy_legacy_env_var(old: &str, new: &str) {
    if std::env::var_os(new).is_some() {
        return;
    }

    if let Some(value) = std::env::var_os(old) {
        std::env::set_var(new, value);
    }
}

fn tracing_filter(cli: &Cli) -> EnvFilter {
    cli_log_override(cli)
        .map(EnvFilter::new)
        .unwrap_or_else(|| {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
        })
}

fn cli_log_override(cli: &Cli) -> Option<&'static str> {
    if cli.silent {
        return Some("off");
    }

    match cli_verbosity(cli) {
        0 => None,
        1 => Some("info"),
        2 => Some("debug"),
        _ => Some("trace"),
    }
}

fn cli_verbosity(cli: &Cli) -> u8 {
    match &cli.command {
        Commands::Learn(args) => args.verbose,
        Commands::Run(args) => args.sandbox.verbose,
        Commands::Shell(args) => args.sandbox.verbose,
        Commands::Wrap(args) => args.sandbox.verbose,
        Commands::Setup(args) => args.verbose,
        // Phase 112 SEC-07: ProxyArgs carries its own `verbose` field (not part
        // of the catch-all arm below, matching Setup/Shell/Wrap's shape).
        Commands::Proxy(args) => args.verbose,
        Commands::Why(_)
        | Commands::Classify(_)
        | Commands::Rollback(_)
        | Commands::Trust(_)
        | Commands::Audit(_)
        | Commands::Ps(_)
        | Commands::Stop(_)
        | Commands::Detach(_)
        | Commands::Attach(_)
        | Commands::Logs(_)
        | Commands::Inspect(_)
        | Commands::Prune(_)
        | Commands::Session(_)
        | Commands::Policy(_)
        | Commands::Profile(_)
        | Commands::Pull(_)
        | Commands::Remove(_)
        | Commands::Update(_)
        | Commands::Search(_)
        | Commands::List(_)
        | Commands::Pin(_)
        | Commands::Unpin(_)
        | Commands::Outdated(_)
        | Commands::OpenUrlHelper(_)
        | Commands::PackUpdateHintHelper(_)
        | Commands::ClaudeCodeHook
        | Commands::Completions(_)
        // Phase 74 D-05: daemon/agent verbs have no verbose flag.
        | Commands::Daemon(_)
        | Commands::Agent(_)
        // Phase 82 DEPLOY-06: health has no verbose flag.
        | Commands::Health(_)
        // Phase 93 Plan 02: override audit-emit has no verbose flag.
        | Commands::Override(_) => 0,
    }
}

#[derive(Clone)]
struct SharedFileMakeWriter {
    file: Arc<Mutex<File>>,
}

struct SharedFileWriter {
    file: Arc<Mutex<File>>,
}

impl SharedFileMakeWriter {
    fn new(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }
}

impl<'a> MakeWriter<'a> for SharedFileMakeWriter {
    type Writer = SharedFileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedFileWriter {
            file: Arc::clone(&self.file),
        }
    }
}

impl Write for SharedFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self
            .file
            .lock()
            .map_err(|_| io::Error::other("log file mutex poisoned"))?;
        guard.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut guard = self
            .file
            .lock()
            .map_err(|_| io::Error::other("log file mutex poisoned"))?;
        guard.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::SharedFileMakeWriter;
    use std::io::{Read, Write};
    use tracing_subscriber::fmt::writer::MakeWriter;

    #[test]
    fn shared_file_make_writer_appends_output() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let log_path = temp_dir.path().join("nono.log");
        let writer = SharedFileMakeWriter::new(&log_path).expect("create writer");

        let mut first = writer.make_writer();
        let mut second = writer.make_writer();
        first.write_all(b"first line\n").expect("first write");
        second.write_all(b"second line\n").expect("second write");
        first.flush().expect("first flush");
        second.flush().expect("second flush");

        let mut contents = String::new();
        std::fs::File::open(&log_path)
            .expect("open log")
            .read_to_string(&mut contents)
            .expect("read log");

        assert!(contents.contains("first line"));
        assert!(contents.contains("second line"));
    }

    /// Phase 117-27 WR-16: `log_target_is_private()`'s granted-path
    /// computation. Windows-only because every item under test
    /// (`log_target_is_private`, the `TRACING_LOG_TARGET_*` statics, the
    /// `_for_test` seams) is `#[cfg(target_os = "windows")]`.
    #[cfg(target_os = "windows")]
    mod log_target_is_private_tests {
        use super::super::*;

        /// (a) A `--log-file` path that canonicalizes to a location UNDER a
        /// path this launch's `CapabilitySet` grants the child must NOT be
        /// reported as private, even though the file-log arm was selected.
        #[test]
        fn log_path_inside_a_granted_directory_is_not_private() {
            let _test_lock = lock_log_target_is_private_test();
            clear_log_target_is_private_test_state();

            let temp_dir = tempfile::tempdir().expect("tempdir");
            let granted_dir = temp_dir.path().join("granted");
            std::fs::create_dir_all(&granted_dir).expect("create granted dir");
            let log_path = granted_dir.join("nono.log");
            std::fs::write(&log_path, []).expect("create log file");

            // Simulate the file-log-succeeded arm having been selected —
            // the two new `_for_test` seams only record path/granted-paths,
            // they clear the boolean override but do not themselves flip
            // `TRACING_LOG_TARGET_IS_PRIVATE`.
            TRACING_LOG_TARGET_IS_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
            set_log_target_path_for_test(Some(log_path.clone()));
            set_granted_read_paths_for_test(vec![granted_dir.clone()]);

            let result = log_target_is_private();

            clear_log_target_is_private_test_state();

            assert!(
                !result,
                "a log path inside a directory this launch grants the child must NOT \
                 be reported as private (WR-16) — log_path={log_path:?} \
                 granted_dir={granted_dir:?}"
            );
        }

        /// Perturbation proof (Phase 117-27 WR-16 Task 2 acceptance
        /// criteria): the same fixture as the test above, except the
        /// granted path passed to `set_granted_read_paths_for_test` no
        /// longer covers the log path — proves the positive test above is
        /// not vacuously true.
        #[test]
        fn perturbed_granted_path_no_longer_covering_log_path_flips_result_to_private() {
            let _test_lock = lock_log_target_is_private_test();
            clear_log_target_is_private_test_state();

            let temp_dir = tempfile::tempdir().expect("tempdir");
            let granted_dir = temp_dir.path().join("granted");
            std::fs::create_dir_all(&granted_dir).expect("create granted dir");
            let log_path = granted_dir.join("nono.log");
            std::fs::write(&log_path, []).expect("create log file");
            // A sibling directory that does NOT cover `log_path` — the
            // perturbation: the "granted path" no longer actually contains
            // the log file.
            let unrelated_dir = temp_dir.path().join("unrelated");
            std::fs::create_dir_all(&unrelated_dir).expect("create unrelated dir");

            TRACING_LOG_TARGET_IS_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
            set_log_target_path_for_test(Some(log_path.clone()));
            set_granted_read_paths_for_test(vec![unrelated_dir.clone()]);

            let result = log_target_is_private();

            clear_log_target_is_private_test_state();

            assert!(
                result,
                "perturbation check: once the granted path no longer covers the log \
                 path, log_target_is_private() must flip to `true` (private) — got \
                 {result}, log_path={log_path:?} unrelated_dir={unrelated_dir:?}"
            );
        }

        /// (b) A `--log-file` path that canonicalizes to a location outside
        /// every granted path must be reported as private.
        #[test]
        fn log_path_outside_every_granted_directory_is_private() {
            let _test_lock = lock_log_target_is_private_test();
            clear_log_target_is_private_test_state();

            let temp_dir = tempfile::tempdir().expect("tempdir");
            let granted_dir = temp_dir.path().join("granted");
            std::fs::create_dir_all(&granted_dir).expect("create granted dir");
            let outside_dir = temp_dir.path().join("outside");
            std::fs::create_dir_all(&outside_dir).expect("create outside dir");
            let log_path = outside_dir.join("nono.log");
            std::fs::write(&log_path, []).expect("create log file");

            TRACING_LOG_TARGET_IS_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
            set_log_target_path_for_test(Some(log_path.clone()));
            set_granted_read_paths_for_test(vec![granted_dir.clone()]);

            let result = log_target_is_private();

            clear_log_target_is_private_test_state();

            assert!(
                result,
                "a log path outside every granted directory must be reported as \
                 private — log_path={log_path:?} granted_dir={granted_dir:?}"
            );
        }

        /// (c) No log path recorded (the file-log arm was never selected) —
        /// fail-secure to `false` ("not private"), per WR-16's own stated
        /// rule ("unknown -> not private").
        #[test]
        fn no_log_path_recorded_is_not_private() {
            let _test_lock = lock_log_target_is_private_test();
            clear_log_target_is_private_test_state();

            // `TRACING_LOG_TARGET_IS_PRIVATE` left `false` by
            // `clear_log_target_is_private_test_state` above — the arm was
            // never selected.
            let result = log_target_is_private();

            clear_log_target_is_private_test_state();

            assert!(
                !result,
                "when the file-log arm was never selected, log_target_is_private() \
                 must fail-secure to false — got {result}"
            );
        }

        /// (d) WR-23 (Phase 117-42): the file-log arm WAS selected and a valid
        /// log path IS recorded, but no granted-path check has run for this
        /// launch. Fail-secure to `false`.
        ///
        /// This is the case `log_target_is_private`'s doc comment has always
        /// claimed ("no granted-path check has run ... returns false") and the
        /// code could not implement: `GRANTED_READ_PATHS` was a
        /// `Mutex<Vec<PathBuf>>` with nothing distinguishing "never populated"
        /// from "populated empty", and `!granted_paths.iter().any(..)` on an
        /// empty vec is `true` — so the D-28 gate OPENED.
        #[test]
        fn no_granted_paths_recorded_is_not_private() {
            let _test_lock = lock_log_target_is_private_test();
            clear_log_target_is_private_test_state();

            let temp_dir = tempfile::tempdir().expect("tempdir");
            let log_path = temp_dir.path().join("nono.log");
            std::fs::write(&log_path, []).expect("create log file");

            TRACING_LOG_TARGET_IS_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
            set_log_target_path_for_test(Some(log_path.clone()));
            // Deliberately NO set_granted_read_paths_for_test call.

            let result = log_target_is_private();

            clear_log_target_is_private_test_state();

            assert!(
                !result,
                "WR-23: with no granted-path check recorded, log_target_is_private() must \
                 fail-secure to false — got {result}, log_path={log_path:?}"
            );
        }

        /// (e) WR-23's discriminating sibling: a check that RAN and found this
        /// launch grants the child nothing is NOT the same state, and must
        /// still be able to report a log target private.
        ///
        /// Without this test, (d) is satisfiable by hardcoding `false`, or by
        /// collapsing both states back into one. The PAIR is what proves the
        /// two remain distinguishable.
        #[test]
        fn granted_path_check_ran_and_found_nothing_is_still_private() {
            let _test_lock = lock_log_target_is_private_test();
            clear_log_target_is_private_test_state();

            let temp_dir = tempfile::tempdir().expect("tempdir");
            let log_path = temp_dir.path().join("nono.log");
            std::fs::write(&log_path, []).expect("create log file");

            TRACING_LOG_TARGET_IS_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
            set_log_target_path_for_test(Some(log_path.clone()));
            // The check RAN; this launch grants the child nothing.
            set_granted_read_paths_for_test(vec![]);

            let result = log_target_is_private();

            clear_log_target_is_private_test_state();

            assert!(
                result,
                "a granted-path check that ran and found nothing must still report a log \
                 target outside every (vacuously zero) granted path as private — got \
                 {result}, log_path={log_path:?}"
            );
        }

        /// Task 3 gap (Phase 117-42): the doc comment's "no `--log-file` arm
        /// selected" clause covers TWO distinct branches, and only one had a
        /// test. This is the second: the arm WAS selected
        /// (`TRACING_LOG_TARGET_IS_PRIVATE == true`) but no path was ever
        /// recorded — an internal inconsistency that must fail secure.
        #[test]
        fn log_arm_selected_but_no_path_recorded_is_not_private() {
            let _test_lock = lock_log_target_is_private_test();
            clear_log_target_is_private_test_state();

            TRACING_LOG_TARGET_IS_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
            set_log_target_path_for_test(None);
            set_granted_read_paths_for_test(vec![]);

            let result = log_target_is_private();

            clear_log_target_is_private_test_state();

            assert!(
                !result,
                "the file-log arm reporting selected with no recorded path is an internal \
                 inconsistency and must fail-secure to false — got {result}"
            );
        }

        /// Task 3 gap (Phase 117-42): the doc comment's third documented
        /// fail-secure case — "the stored log path fails to canonicalize" —
        /// had no test before this plan.
        #[test]
        fn uncanonicalizable_log_path_is_not_private() {
            let _test_lock = lock_log_target_is_private_test();
            clear_log_target_is_private_test_state();

            let temp_dir = tempfile::tempdir().expect("tempdir");
            // Recorded but never created, so `canonicalize` fails.
            let missing = temp_dir.path().join("never-created.log");
            assert!(
                std::fs::canonicalize(&missing).is_err(),
                "precondition: the recorded log path must not canonicalize"
            );

            TRACING_LOG_TARGET_IS_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
            set_log_target_path_for_test(Some(missing.clone()));
            set_granted_read_paths_for_test(vec![]);

            let result = log_target_is_private();

            clear_log_target_is_private_test_state();

            assert!(
                !result,
                "a stored log path that cannot be canonicalized must fail-secure to false — \
                 got {result}, path={missing:?}"
            );
        }

        /// WR-24 discovery (Phase 117-42): establish BY EXECUTION whether a
        /// verbatim canonical path (`\\?\C:\...`, `Prefix(VerbatimDisk)`) and
        /// the same path spelled non-verbatim (`Prefix(Disk)`) compare equal
        /// under the component-wise `Path::starts_with`.
        ///
        /// The review asserted the `:131` fallback "can never match" because
        /// of exactly this mismatch. This test converts that claim into a
        /// recorded fact rather than a premise — see this plan's SUMMARY for
        /// the production-reachability half of the analysis.
        #[test]
        fn verbatim_and_non_verbatim_prefixes_do_not_compare_equal() {
            let temp_dir = tempfile::tempdir().expect("tempdir");
            let child = temp_dir.path().join("child.txt");
            std::fs::write(&child, []).expect("create child");

            let verbatim_child = std::fs::canonicalize(&child).expect("canonicalize child");
            let non_verbatim_parent = temp_dir.path().to_path_buf();

            // `tempfile` hands back a non-verbatim path; canonicalize does not.
            // If this precondition ever stops holding the assertion below is
            // meaningless, so check it explicitly rather than assuming.
            assert!(
                verbatim_child.to_string_lossy().starts_with(r"\\?\"),
                "precondition: canonicalize must yield a verbatim path on Windows — got \
                 {verbatim_child:?}"
            );
            assert!(
                !non_verbatim_parent.to_string_lossy().starts_with(r"\\?\"),
                "precondition: the tempdir path must be non-verbatim — got \
                 {non_verbatim_parent:?}"
            );

            assert!(
                !verbatim_child.starts_with(&non_verbatim_parent),
                "OBSERVED: Path::starts_with is component-wise and Prefix(VerbatimDisk) != \
                 Prefix(Disk), so a verbatim path does NOT start with its own non-verbatim \
                 parent. verbatim={verbatim_child:?} non_verbatim={non_verbatim_parent:?}"
            );

            // The same comparison with both sides canonical DOES match — so
            // the mismatch above is about spelling, not about the paths.
            let verbatim_parent =
                std::fs::canonicalize(temp_dir.path()).expect("canonicalize parent");
            assert!(
                verbatim_child.starts_with(&verbatim_parent),
                "control: with both sides canonical the comparison must succeed — \
                 child={verbatim_child:?} parent={verbatim_parent:?}"
            );
        }

        /// WR-24 (Phase 117-42): the `:131` fallback branch, which no test
        /// covered before this plan.
        ///
        /// The production input to that branch is a `FsCapability::resolved`
        /// value, which is ALWAYS produced by `path.canonicalize()` (see
        /// `crates/nono/src/capability.rs:110,144,361,457`) and is therefore
        /// verbatim even when the target does not exist — the
        /// not-yet-existing-file case joins a canonicalized parent with a file
        /// name (`capability.rs:392-405`). So the fallback compares
        /// verbatim-against-verbatim and DOES work.
        ///
        /// This pins that, so the comment on the fallback is verified rather
        /// than merely asserted.
        #[test]
        fn fallback_branch_still_matches_for_a_verbatim_nonexistent_grant() {
            let _test_lock = lock_log_target_is_private_test();
            clear_log_target_is_private_test_state();

            let temp_dir = tempfile::tempdir().expect("tempdir");
            let granted_dir = temp_dir.path().join("granted");
            std::fs::create_dir_all(&granted_dir).expect("create granted dir");
            let log_path = granted_dir.join("nono.log");
            std::fs::write(&log_path, []).expect("create log file");

            // A VERBATIM but non-existent grant target under the same tree —
            // exactly the shape capability.rs:392-405 produces. It cannot be
            // canonicalized (it does not exist), so it takes the fallback.
            let canonical_granted =
                std::fs::canonicalize(&granted_dir).expect("canonicalize granted dir");
            let nonexistent_verbatim_grant = canonical_granted.join("not-created-yet");
            assert!(
                std::fs::canonicalize(&nonexistent_verbatim_grant).is_err(),
                "precondition: the grant target must not exist, so the fallback is taken"
            );

            TRACING_LOG_TARGET_IS_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
            set_log_target_path_for_test(Some(log_path.clone()));
            // Both the covering canonical dir AND the non-existent verbatim
            // child are granted; the log file sits under the former.
            set_granted_read_paths_for_test(vec![
                nonexistent_verbatim_grant.clone(),
                canonical_granted.clone(),
            ]);

            let result = log_target_is_private();

            clear_log_target_is_private_test_state();

            assert!(
                !result,
                "the granted-path comparison must still see the covering grant — got \
                 {result}, log_path={log_path:?} granted={canonical_granted:?}"
            );
        }
    }
}
