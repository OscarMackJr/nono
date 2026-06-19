use crate::cli::{Cli, Commands};
use crate::telemetry::SecurityEventLayer;
use crate::{config, theme};
use nono::TelemetryConfig;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriter;

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

    for (legacy, replacement) in [
        ("--allow-net", Some("network is unrestricted by default")),
        ("--net-allow", Some("network is unrestricted by default")),
        ("--allow-proxy", Some("--allow-domain")),
        ("--proxy-allow", Some("--allow-domain")),
        ("--proxy-credential", Some("--credential")),
        ("--allow-bind", Some("--listen-port")),
        ("--allow-port", Some("--open-port")),
        ("--external-proxy", Some("--upstream-proxy")),
        ("--external-proxy-bypass", Some("--upstream-bypass")),
        ("--net-block", Some("--block-net")),
    ] {
        if args
            .iter()
            .any(|arg| arg == legacy || arg.starts_with(&format!("{legacy}=")))
        {
            let message = if let Some(replacement) = replacement {
                format!("Warning: `{legacy}` is deprecated; use `{replacement}` instead.")
            } else {
                format!("Warning: `{legacy}` is deprecated.")
            };
            warnings.push(message);
        }
    }

    for (legacy, replacement) in [
        ("NONO_NET_BLOCK", "NONO_BLOCK_NET"),
        ("NONO_NET_ALLOW", "NONO_ALLOW_NET"),
        ("NONO_ALLOW_PROXY", "NONO_ALLOW_DOMAIN"),
        ("NONO_PROXY_ALLOW", "NONO_ALLOW_DOMAIN"),
        ("NONO_PROXY_CREDENTIAL", "NONO_CREDENTIAL"),
        ("NONO_EXTERNAL_PROXY", "NONO_UPSTREAM_PROXY"),
        ("NONO_EXTERNAL_PROXY_BYPASS", "NONO_UPSTREAM_BYPASS"),
    ] {
        if std::env::var_os(legacy).is_some() {
            warnings.push(format!(
                "Warning: `{legacy}` is deprecated; use `{replacement}` instead."
            ));
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
        | Commands::Health(_) => 0,
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
}
