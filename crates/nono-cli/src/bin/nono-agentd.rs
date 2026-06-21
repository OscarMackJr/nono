//! nono agent daemon binary.
//!
//! This is the persistent multi-tenant agent daemon introduced in Phase 74.
//! It runs as a per-user Windows service (`SERVICE_USER_OWN_PROCESS`) under the
//! interactive user's session token — the same privilege level as any other
//! process the user launches. It NEVER elevates to LocalSystem or SYSTEM.
//!
//! The daemon manages multiple concurrent confined AI agents over a single
//! named-pipe capability channel, tracking per-agent lifecycle via
//! `AgentTenant` RAII owning structs. When the daemon stops (or is killed),
//! `KILL_ON_JOB_CLOSE` fires on each agent's job object, terminating the
//! entire agent process group — agents die with the daemon (ADR-74 D-03).
//!
//! # Privilege model (ADR-74)
//!
//! - Runs as `SERVICE_USER_OWN_PROCESS` — NOT LocalSystem/SYSTEM.
//! - Contains NO WFP (Windows Filtering Platform) calls. Network policy in
//!   Phase 74 is profile-only. `nono-agentd` and `nono-wfp-service` are
//!   fully split binaries with no IPC channel between them.
//! - Foreground fallback: when invoked outside an SCM context (e.g. during
//!   development), `service_dispatcher` returns
//!   `ERROR_FAILED_SERVICE_CONTROLLER_CONNECT`; the binary falls through to
//!   foreground / on-demand mode WITHOUT panicking (non-fatal posture matching
//!   `nono-wfp-service.rs`).
//!
//! # Non-Windows support
//!
//! On non-Windows targets the binary is replaced by a diagnostic stub so the
//! workspace `cargo check` succeeds on Linux and macOS.

// ─── Non-Windows stub ────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("nono-agentd is Windows-only");
    std::process::exit(1);
}

// ─── Windows implementation ───────────────────────────────────────────────────

#[cfg(target_os = "windows")]
#[path = "../agent_daemon/mod.rs"]
mod agent_daemon;

// Make the telemetry module reachable from the daemon binary via #[path]-include,
// mirroring the agent_daemon include pattern above. The daemon binary has no lib
// target, so telemetry (normally part of the `nono` binary via main.rs) must be
// explicitly included here (DRAIN-04 D-02 / Pitfall 2).
#[cfg(target_os = "windows")]
#[path = "../telemetry/mod.rs"]
mod telemetry;

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::ffi::OsString;
    use std::process::ExitCode;
    use std::sync::Arc;
    use tokio::sync::Notify;
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };

    /// The SCM service name registered by `nono daemon install` (Phase 74 Wave 3).
    ///
    /// This is a per-user service name — it lives in `HKCU\SYSTEM\CurrentControlSet\Services`
    /// and does not require administrative elevation to install or start.
    const SERVICE_NAME: &str = "nono-agentd";

    /// Argument that tells the binary it has been launched by the SCM.
    ///
    /// When the binary is invoked as a service, the SCM passes this argument
    /// first. When invoked directly (developer / foreground path), it is absent
    /// and the binary falls through to `run_foreground_mode`.
    const SERVICE_MODE_ARG: &str = "--service-mode";

    // Wave 2 (Plan 74-04) will wire in the full event-log infrastructure:
    // const EVENT_LOG_SOURCE: &str    = "nono-agentd";
    // const EVENT_ID_AGENT_LAUNCHED: u32 = 2001;
    // const EVENT_ID_AGENT_REAPED: u32   = 2002;
    // const EVENT_ID_AUTH_DENIED: u32    = 2003;

    fn print_help() {
        println!("nono-agentd {}", env!("CARGO_PKG_VERSION"));
        println!("Persistent multi-tenant agent daemon (Phase 74)");
        println!();
        println!("Runs as a per-user Windows service (SERVICE_USER_OWN_PROCESS).");
        println!("Manages confined AI agents over a named-pipe capability channel.");
        println!();
        println!("Service contract:");
        println!("  service name: {SERVICE_NAME}");
        println!("  startup args: {SERVICE_MODE_ARG}");
        println!();
        println!("Supported options:");
        println!("  --help                 Show this message");
        println!("  --version              Show version information");
        println!("  {SERVICE_MODE_ARG:<22}Run the service entrypoint (launched by SCM)");
        println!("  --foreground           Run in foreground mode (dev/testing)");
    }

    define_windows_service!(ffi_service_main, service_main);

    fn service_main(arguments: Vec<OsString>) {
        if let Err(e) = run_service(arguments) {
            eprintln!("nono-agentd service failed: {}", e);
        }
    }

    fn run_service(_arguments: Vec<OsString>) -> windows_service::Result<()> {
        // Shutdown signal. The SCM control-dispatch thread invokes `event_handler`
        // on a STOP control and wakes the accept loop so the service can transition
        // to STOPPED cleanly.
        let shutdown = Arc::new(Notify::new());
        let shutdown_handler = Arc::clone(&shutdown);

        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop => {
                    // Call notify_one() twice — once for run_accept_loop and once for
                    // run_control_loop. Both loops park on the same Arc<Notify> and
                    // notify_one() only wakes ONE waiter per call, so we need two calls
                    // to guarantee both concurrent loops receive the shutdown signal.
                    shutdown_handler.notify_one();
                    shutdown_handler.notify_one();
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

        // ADR-74 Decision 1: USER_OWN_PROCESS — per-user service, NOT LocalSystem.
        // Callers must NOT change this to OWN_PROCESS without a new ADR revision.
        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::USER_OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })?;

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                windows_service::Error::Winapi(std::io::Error::other(format!(
                    "nono-agentd: failed to build tokio runtime: {}",
                    e
                )))
            })?;

        // D-04 SOLE read: resolve machine egress policy exactly once at daemon startup.
        // Absent key → Ok(None) → no-proxy path (D-07 fall-through).
        // Present-but-broken key → Err → abort with fail-secure (D-07 Pitfall 3).
        // D-06 restart-to-apply: this snapshot is held for the daemon lifetime.
        let (egress_domains, machine_policy_active) =
            match super::agent_daemon::resolve_machine_egress_policy(&[]) {
                Ok(result) => result,
                Err(e) => {
                    return Err(windows_service::Error::Winapi(std::io::Error::other(
                        format!(
                            "nono-agentd: machine egress policy load failed (fail-secure): {e}"
                        ),
                    )));
                }
            };

        let daemon_state =
            rt.block_on(async { build_daemon_state(machine_policy_active, &egress_domains).await });

        let daemon_state = match daemon_state {
            Ok(state) => Arc::new(state),
            Err(e) => {
                return Err(windows_service::Error::Winapi(std::io::Error::other(
                    format!("nono-agentd: proxy startup failed (fail-secure): {e}"),
                )));
            }
        };

        rt.block_on(async {
            // Wave 5 (Plan 74-07): run both loops concurrently so the daemon
            // serves BOTH the capability pipe and the operator control pipe.
            // Both are interruptible by the same `shutdown` notifier.
            tokio::join!(
                super::agent_daemon::accept_loop::run_accept_loop(
                    Arc::clone(&daemon_state),
                    Arc::clone(&shutdown),
                ),
                super::agent_daemon::control_loop::run_control_loop(
                    Arc::clone(&daemon_state),
                    Arc::clone(&shutdown),
                ),
            );
        });

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::USER_OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })?;

        Ok(())
    }

    fn run_service_mode() -> ExitCode {
        // ADR-74 Decision 3: non-fatal foreground fallback.
        // `service_dispatcher::start` returns `Err` with
        // `ERROR_FAILED_SERVICE_CONTROLLER_CONNECT` when invoked outside the SCM
        // (direct invocation during development). This is expected behavior — the
        // binary transitions to foreground mode rather than exiting with an error.
        match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!(
                    "nono-agentd: SCM service_dispatcher failed ({}); \
                     falling through to foreground mode",
                    e
                );
                // Non-fatal: fall through to foreground mode.
                run_foreground_mode()
            }
        }
    }

    fn run_foreground_mode() -> ExitCode {
        eprintln!(
            "nono-agentd {}: running in foreground mode (dev/testing). \
             Press Ctrl-C to stop.",
            env!("CARGO_PKG_VERSION")
        );

        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("nono-agentd: failed to build tokio runtime: {}", e);
                return ExitCode::from(1);
            }
        };

        let shutdown = Arc::new(Notify::new());
        let shutdown_clone = Arc::clone(&shutdown);

        // Install a Ctrl-C handler so the foreground mode can be stopped cleanly.
        if let Err(e) = ctrlc_setup(shutdown_clone) {
            eprintln!("nono-agentd: failed to install Ctrl-C handler: {}", e);
            // Non-fatal in foreground mode — continue without clean shutdown.
        }

        // D-04 SOLE read: resolve machine egress policy exactly once at daemon startup.
        let (egress_domains, machine_policy_active) =
            match super::agent_daemon::resolve_machine_egress_policy(&[]) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("nono-agentd: machine egress policy load failed (fail-secure): {e}");
                    return ExitCode::from(1);
                }
            };

        let daemon_state =
            rt.block_on(async { build_daemon_state(machine_policy_active, &egress_domains).await });

        let daemon_state = match daemon_state {
            Ok(state) => Arc::new(state),
            Err(e) => {
                eprintln!("nono-agentd: proxy startup failed (fail-secure): {e}");
                return ExitCode::from(1);
            }
        };

        rt.block_on(async {
            // Wave 5 (Plan 74-07): run both loops concurrently.
            tokio::join!(
                super::agent_daemon::accept_loop::run_accept_loop(
                    Arc::clone(&daemon_state),
                    Arc::clone(&shutdown),
                ),
                super::agent_daemon::control_loop::run_control_loop(
                    Arc::clone(&daemon_state),
                    Arc::clone(&shutdown),
                ),
            );
        });

        eprintln!("nono-agentd: foreground mode stopped.");
        ExitCode::SUCCESS
    }

    /// Install a Ctrl-C handler that notifies the shutdown signal.
    fn ctrlc_setup(shutdown: Arc<Notify>) -> std::io::Result<()> {
        use windows_sys::Win32::System::Console::{SetConsoleCtrlHandler, CTRL_C_EVENT};

        // Store the shutdown notifier in a thread-safe static for the handler callback.
        static SHUTDOWN_SIGNAL: std::sync::OnceLock<Arc<Notify>> = std::sync::OnceLock::new();
        let _ = SHUTDOWN_SIGNAL.set(shutdown);

        unsafe extern "system" fn ctrl_handler(
            ctrl_type: u32,
        ) -> windows_sys::Win32::Foundation::BOOL {
            if ctrl_type == CTRL_C_EVENT {
                if let Some(notify) = SHUTDOWN_SIGNAL.get() {
                    // Call notify_one() twice — once for run_accept_loop and once for
                    // run_control_loop. Both loops park on the same Arc<Notify>; one
                    // call only wakes ONE of the two concurrent loop waiters.
                    notify.notify_one();
                    notify.notify_one();
                }
                // Return TRUE to indicate we handled the event.
                1
            } else {
                // Return FALSE to let the default handler process it.
                0
            }
        }

        // SAFETY: ctrl_handler is a valid extern "system" fn with the correct signature
        // for SetConsoleCtrlHandler. It uses only thread-safe OnceLock access.
        let ok = unsafe { SetConsoleCtrlHandler(Some(ctrl_handler), 1) };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    /// Build `DaemonState`, starting the in-process proxy when a machine egress
    /// policy is active (D-04/EGRESS-01 wiring).
    ///
    /// When `machine_policy_active` is `false`, no proxy is started and
    /// `DaemonState::new()` is returned (legacy no-enforcement path, D-07 fall-through).
    ///
    /// When `machine_policy_active` is `true`:
    /// 1. Start `nono_proxy` with `strict_filter = true` + `allowed_hosts = egress_domains`
    ///    so that deny-by-default is structural (ProxyFilter::new_strict, EGRESS-01).
    /// 2. Bind on loopback, port 0 — OS assigns an ephemeral port (no hardcoded ports).
    /// 3. Return `DaemonState::new_with_proxy(port)` so every subsequent
    ///    `wfp_filter_add` call threads the same port (D-04 no-drift, EGRESS-02).
    ///
    /// Fail-secure: any proxy startup error returns `Err` — the caller must NOT
    /// start the daemon without a working proxy when machine policy is active.
    async fn build_daemon_state(
        machine_policy_active: bool,
        egress_domains: &[String],
    ) -> Result<super::agent_daemon::DaemonState, String> {
        if !machine_policy_active {
            // D-07 fall-through: no machine policy → no proxy → legacy path.
            tracing::debug!("nono-agentd: no machine egress policy; using legacy DaemonState");
            return Ok(super::agent_daemon::DaemonState::new());
        }

        // Machine policy is active (D-04/EGRESS-01).
        // Start the in-process proxy with strict filtering (deny-by-default).
        tracing::info!(
            allowed_hosts = egress_domains.len(),
            "nono-agentd: machine egress policy active; starting in-process proxy (EGRESS-01)"
        );

        let config = nono_proxy::config::ProxyConfig {
            bind_addr: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            bind_port: 0, // OS-assigned ephemeral port
            allowed_hosts: egress_domains.to_vec(),
            strict_filter: true, // EGRESS-01: deny-by-default (ProxyFilter::new_strict)
            routes: vec![],
            // Remaining fields default to their safe zero values.
            ..nono_proxy::config::ProxyConfig::default()
        };

        let handle = nono_proxy::server::start(config)
            .await
            .map_err(|e| format!("nono-agentd: in-process proxy failed to start: {e}"))?;

        let proxy_port = handle.port;
        tracing::info!(
            proxy_port,
            "nono-agentd: in-process proxy started on loopback:{proxy_port} (EGRESS-01)"
        );

        // Leak the handle so the proxy server runs for the daemon lifetime.
        // The daemon process exits when the SCM sends STOP (service mode) or
        // on Ctrl-C (foreground mode); proxy resources are reclaimed by the OS.
        // Using Box::leak is deliberate — there is no shutdown ordering that
        // would benefit from a Drop impl here (the proxy and daemon exit together).
        std::mem::forget(handle);

        Ok(super::agent_daemon::DaemonState::new_with_proxy(proxy_port))
    }

    pub(super) fn run() -> ExitCode {
        match std::env::args().nth(1).as_deref() {
            Some("--help") | Some("-h") => {
                print_help();
                ExitCode::SUCCESS
            }
            Some("--version") => {
                println!("nono-agentd {}", env!("CARGO_PKG_VERSION"));
                ExitCode::SUCCESS
            }
            Some(SERVICE_MODE_ARG) => run_service_mode(),
            Some("--foreground") => run_foreground_mode(),
            None | Some(_) => {
                // ADR-74 Decision 3: non-fatal foreground fallback.
                // Direct invocation without arguments → foreground mode
                // (mirrors nono-wfp-service.rs non-fatal posture).
                run_foreground_mode()
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn main() -> std::process::ExitCode {
    windows_impl::run()
}
