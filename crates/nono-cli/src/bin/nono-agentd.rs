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
                    // `notify_one` stores a permit so a STOP is never lost even
                    // if the accept loop is not currently parked on `notified()`.
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

        let daemon_state = super::agent_daemon::DaemonState::new();

        rt.block_on(async {
            // Wave 2 (Plan 74-04) wires in the real accept loop:
            //   agent_daemon::run_accept_loop(daemon_state, shutdown).await
            //
            // Skeleton placeholder: park on the shutdown signal so the service
            // compiles, starts cleanly, and responds to SCM STOP without spinning.
            let _ = daemon_state;
            shutdown.notified().await;
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

        let daemon_state = super::agent_daemon::DaemonState::new();

        rt.block_on(async {
            // Wave 2 (Plan 74-04) wires in the real accept loop:
            //   agent_daemon::run_accept_loop(daemon_state, shutdown).await
            //
            // Skeleton placeholder: park until Ctrl-C or external notification.
            let _ = daemon_state;
            shutdown.notified().await;
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
