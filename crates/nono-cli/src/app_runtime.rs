use crate::agent_cli;
use crate::audit_commands;
use crate::classify_runtime;
use crate::claude_code_hook;
use crate::cli::{Cli, Commands, OverrideCommands, RunArgs, SessionCommands, SetupArgs};
use crate::command_runtime::{run_sandbox, run_shell, run_wrap};
use crate::completions::run_completions;
use crate::deprecated_policy;
use crate::health;
use crate::health::HealthVerdict;
use crate::learn_runtime::run_learn;
use crate::open_url_runtime::run_open_url_helper;
use crate::output;
use crate::package_cmd;
use crate::profile_cmd;
use crate::rollback_commands;
use crate::session_commands;
use crate::setup;
use crate::startup_runtime::{
    allows_pre_exec_update_check, run_detached_launch, show_update_notification,
};
use crate::trust_cmd;
use crate::update_check;
use crate::why_runtime::run_why;
use crate::{Result, DETACHED_LAUNCH_ENV};

pub(crate) fn run(cli: Cli) -> Result<()> {
    let mut update_handle = start_update_check_handle(&cli);
    dispatch_command(
        cli.command,
        cli.silent,
        cli.internal_supervisor,
        &mut update_handle,
    )
}

fn start_update_check_handle(cli: &Cli) -> Option<update_check::UpdateCheckHandle> {
    if !cli.silent && allows_pre_exec_update_check(&cli.command) {
        update_check::start_background_check()
    } else {
        None
    }
}

fn dispatch_command(
    command: Commands,
    silent: bool,
    internal_supervisor: bool,
    update_handle: &mut Option<update_check::UpdateCheckHandle>,
) -> Result<()> {
    match command {
        Commands::Learn(args) => run_learn(*args, silent),
        Commands::Run(args) => run_command_with_update(update_handle, silent, || {
            run_or_detach(*args, silent, internal_supervisor)
        }),
        Commands::Shell(args) => {
            run_command_with_banner_and_update(update_handle, silent, || run_shell(*args, silent))
        }
        Commands::Wrap(args) => {
            run_command_with_banner_and_update(update_handle, silent, || run_wrap(*args, silent))
        }
        Commands::Why(args) => run_command_with_update(update_handle, silent, || run_why(*args)),
        Commands::Classify(args) => {
            run_command_with_update(update_handle, silent, || run_classify(args))
        }
        // Phase 82 DEPLOY-06: fleet diagnostic.
        // run_health prints JSON to stdout (always, D-06) and returns the tri-state
        // HealthVerdict. The verdict is mapped to process::exit here — NOT inside
        // run_health itself (which keeps the Result-returning convention intact).
        // exit(0) healthy / exit(1) degraded / exit(2) broken.
        Commands::Health(args) => {
            run_command_with_update(update_handle, silent, || {
                let verdict = health::run_health(&args)?;
                let code = match verdict {
                    HealthVerdict::Healthy => 0i32,
                    HealthVerdict::Degraded => 1,
                    HealthVerdict::Broken => 2,
                };
                // Only exit non-zero here; exit(0) is the normal Ok(()) return path.
                if code != 0 {
                    std::process::exit(code);
                }
                Ok(())
            })
        }
        // Phase 74 D-05: daemon lifecycle and agent management verbs.
        // Thin clients over nono-agentd; daemon verbs drive the per-user SCM
        // service; agent verbs are fail-secure when the daemon is not running.
        Commands::Daemon(args) => {
            run_command_with_update(update_handle, silent, || agent_cli::run_daemon(args))
        }
        Commands::Agent(args) => {
            run_command_with_update(update_handle, silent, || agent_cli::run_agent(args))
        }
        Commands::Setup(args) => {
            run_command_with_banner_and_update(update_handle, silent, || run_setup(args))
        }
        Commands::Rollback(args) => run_command_with_update(update_handle, silent, || {
            rollback_commands::run_rollback(args)
        }),
        Commands::Trust(args) => {
            run_command_with_update(update_handle, silent, || trust_cmd::run_trust(args))
        }
        Commands::Audit(args) => {
            run_command_with_update(update_handle, silent, || audit_commands::run_audit(args))
        }
        Commands::Ps(args) => {
            run_command_with_update(update_handle, silent, || session_commands::run_ps(&args))
        }
        Commands::Stop(args) => {
            run_command_with_update(update_handle, silent, || session_commands::run_stop(&args))
        }
        Commands::Detach(args) => run_command_with_update(update_handle, silent, || {
            session_commands::run_detach(&args)
        }),
        Commands::Attach(args) => run_command_with_update(update_handle, silent, || {
            session_commands::run_attach(&args)
        }),
        Commands::Logs(args) => {
            run_command_with_update(update_handle, silent, || session_commands::run_logs(&args))
        }
        Commands::Inspect(args) => run_command_with_update(update_handle, silent, || {
            session_commands::run_inspect(&args)
        }),
        Commands::Prune(args) => run_command_with_update(update_handle, silent, || {
            // Plan 22-05b Task 3 (upstream `4f9552ec`): emit a stderr
            // deprecation note on every `nono prune` invocation. The
            // hidden alias delegates to the unchanged `run_prune` worker
            // so CLEAN-04 invariants stay byte-identical (Decision 2
            // LOCKED reframe). AUD-04 acceptance #3.
            //
            // Silent-mode preserves the deprecation note: AUD-04
            // acceptance #3 says "still works AND surfaces a deprecation
            // note" — silencing it would defeat the migration prompt.
            eprintln!("warning: `nono prune` is deprecated; use `nono session cleanup` instead");
            session_commands::run_prune(&args)
        }),
        Commands::Session(args) => run_command_with_update(update_handle, silent, || {
            // Plan 22-05b Task 2 (upstream `4f9552ec`): `nono session cleanup`
            // is the renamed entry point. It routes to the unchanged
            // `session_commands::run_prune` worker per Decision 2 LOCKED
            // reframe — `auto_prune_if_needed` + `AUTO_PRUNE_STALE_THRESHOLD`
            // stay byte-identical so the v2.1 Phase 19 CLEAN-04 invariants
            // (auto_prune_is_noop_when_sandboxed; NONO_CAP_FILE early-return
            // first statement) are preserved trivially.
            match args.command {
                SessionCommands::Cleanup(prune_args) => session_commands::run_prune(&prune_args),
            }
        }),
        Commands::Policy(args) => {
            run_command_with_update(update_handle, silent, || deprecated_policy::dispatch(args))
        }
        Commands::Profile(args) => {
            run_command_with_update(update_handle, silent, || profile_cmd::run_profile(args))
        }
        Commands::Pull(args) => {
            run_command_with_update(update_handle, silent, || package_cmd::run_pull(args))
        }
        Commands::Remove(args) => {
            run_command_with_update(update_handle, silent, || package_cmd::run_remove(args))
        }
        Commands::Update(args) => {
            run_command_with_update(update_handle, silent, || package_cmd::run_update(args))
        }
        Commands::Search(args) => {
            run_command_with_update(update_handle, silent, || package_cmd::run_search(args))
        }
        Commands::List(args) => {
            run_command_with_update(update_handle, silent, || package_cmd::run_list(args))
        }
        Commands::Pin(args) => {
            run_command_with_update(update_handle, silent, || package_cmd::run_pin(args))
        }
        Commands::Unpin(args) => {
            run_command_with_update(update_handle, silent, || package_cmd::run_unpin(args))
        }
        Commands::Outdated(args) => {
            run_command_with_update(update_handle, silent, || package_cmd::run_outdated(args))
        }
        Commands::OpenUrlHelper(args) => run_open_url_helper(args),
        Commands::PackUpdateHintHelper(args) => crate::pack_update_hint::run_refresh_helper(args),
        Commands::ClaudeCodeHook => claude_code_hook::run(),
        Commands::Completions(args) => run_completions(args),
        // Phase 93 Plan 02: `nono override audit-emit` — live-reject HMAC chain emission (OQ-1).
        // Phase 93 Plan 03: `nono override request` — denial context bundle for approver pipeline (CLI-01).
        Commands::Override(args) => match args.command {
            OverrideCommands::AuditEmit(emit_args) => {
                run_command_with_update(update_handle, silent, || {
                    crate::app_runtime::run_override_audit_emit(emit_args)
                })
            }
            OverrideCommands::Request(req_args) => {
                run_command_with_update(update_handle, silent, || {
                    crate::override_request::run_override_request(req_args)
                })
            }
        },
    }
}

/// Decode base64url meta and emit the rejection event into the HMAC chain.
///
/// Decodes `OverrideAuditEmitArgs.meta` (base64url-no-pad JSON → `OverrideAuditMeta`)
/// then calls `override_audit_emit::emit_override_audit_event`.  Any decode or emit
/// failure returns `Err` → process exits non-zero (AUD-04 fail-closed).
fn run_override_audit_emit(args: crate::cli::OverrideAuditEmitArgs) -> Result<()> {
    use base64::Engine as _;
    use nono::NonoError;

    let json = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(args.meta.as_bytes())
        .map_err(|e| {
            NonoError::SandboxInit(format!("override audit-emit: base64 decode failed: {e}"))
        })?;
    let meta = serde_json::from_slice::<crate::cli::OverrideAuditMeta>(&json).map_err(|e| {
        NonoError::SandboxInit(format!("override audit-emit: JSON parse failed: {e}"))
    })?;

    crate::override_audit_emit::emit_override_audit_event(&meta, args.kind)
}

fn run_command_with_update<T>(
    update_handle: &mut Option<update_check::UpdateCheckHandle>,
    silent: bool,
    command: impl FnOnce() -> Result<T>,
) -> Result<T> {
    show_update_notification(update_handle, silent);
    command()
}

fn run_command_with_banner_and_update<T>(
    update_handle: &mut Option<update_check::UpdateCheckHandle>,
    silent: bool,
    command: impl FnOnce() -> Result<T>,
) -> Result<T> {
    output::print_banner(silent);
    run_command_with_update(update_handle, silent, command)
}

fn run_or_detach(args: RunArgs, silent: bool, internal_supervisor: bool) -> Result<()> {
    if args.detached && !internal_supervisor && std::env::var_os(DETACHED_LAUNCH_ENV).is_none() {
        run_detached_launch(args, silent)
    } else {
        if !internal_supervisor {
            output::print_banner(silent);
        }
        run_sandbox(args, silent)
    }
}

fn run_setup(args: SetupArgs) -> Result<()> {
    let runner = setup::SetupRunner::new(&args);
    runner.run()
}

/// Phase 78 D-04: daemon-first classify routing.
///
/// On Windows, attempts the daemon control pipe for an authoritative verdict.
/// On daemon-absent (`is_pipe_not_found`), falls back to the Phase 73 structural
/// path (non-authoritative, with NOTE_* disclaimers). On any other error, propagates it.
/// On non-Windows, always uses the structural path.
fn run_classify(args: crate::cli::ClassifyArgs) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use crate::agent_cli::{classify_daemon_request, is_pipe_not_found};
        match classify_daemon_request(args.pid, args.json) {
            Ok(()) => return Ok(()),
            Err(e) if is_pipe_not_found(&e) || e.to_string().contains("daemon-absent") => {
                // Daemon not running — fall through to structural path below.
            }
            Err(e) => return Err(e),
        }
    }
    // Structural (non-authoritative) fallback — always used on non-Windows,
    // and on Windows when the daemon is absent.
    let registry = std::sync::Arc::new(std::sync::Mutex::new(nono::AgentRegistry::new()));
    classify_runtime::run_classify(args, registry)
}
