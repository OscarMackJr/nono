//! nono CLI - Capability-based sandbox for AI agents
//!
//! This is the CLI binary that uses the nono library for OS-level sandboxing.

mod app_runtime;
mod audit_attestation;
mod audit_commands;
mod audit_integrity;
mod audit_session;
mod capability_ext;
// AI_AGENT marker classify verb (Phase 73 D-04). NOT cfg-gated — compiles on
// all platforms via the nono::AgentRegistry non-Windows stub.
mod classify_runtime;
// Daemon lifecycle (nono daemon start|stop|status|install|uninstall) and
// agent verbs (nono agent launch|list) — Phase 74 D-05 operator CLI surface.
// NOT cfg-gated: the module stubs non-Windows paths via cfg blocks internally.
mod agent_cli;
mod claude_code_hook;
mod cli;
mod cli_bootstrap;
mod command_blocking_deprecation;
mod command_display;
mod command_runtime;
mod completions;
mod config;
mod credential_runtime;
mod deprecated_policy;
mod deprecated_schema;
mod diagnostic;
mod diagnostic_formatter;
mod dynamic_tokens;
mod exec_identity;
#[cfg(target_os = "windows")]
mod exec_identity_windows;
#[cfg(not(target_os = "windows"))]
mod exec_strategy;
#[cfg(target_os = "windows")]
#[path = "exec_strategy_windows/mod.rs"]
mod exec_strategy;
mod execution_runtime;
mod format_util;
mod hooks;
// Session lifecycle hook runtime (Phase 58).
// Unix runtime (cfg(unix)) — gated unix-only per upstream daa55c8.
#[cfg(unix)]
mod hook_runtime;
// Windows runtime (cfg(windows)) — net-new fork work; stub in Plan 02,
// full implementation in Plan 03 Task 2.
#[cfg(windows)]
mod hook_runtime_windows;
mod instruction_deny;
mod launch_runtime;
mod learn;
mod learn_runtime;
#[cfg(target_os = "windows")]
mod learn_windows;
mod network_policy;
#[cfg(not(target_os = "windows"))]
mod open_url_runtime;
#[cfg(target_os = "windows")]
#[path = "open_url_runtime_windows.rs"]
mod open_url_runtime;
mod output;
// Phase 93 Plan 02: live-reject HMAC chain emission for `nono override audit-emit` (OQ-1 option a).
// Provides the reject-branch EventID 10008/10010 path that runs BEFORE nono.exe spawns
// the sandboxed child (nono-py fails closed and calls this subcommand to land the denial event).
mod override_audit_emit;
// Phase 93 Plan 03: `nono override request` — denial context bundle for the out-of-nono
// approver/KMS-signing pipeline (CLI-01).  Performs NO crypto and NO live check (D-07);
// only gathers scope paths/domains/repo_context/reason and emits a JSON bundle + nonce.
mod override_request;
// Phase 112 WR-01: registry of child pids owned by an in-process
// `std::process::Child`, so the Linux supervisor's orphan reaper never steals
// their exit status out from under libstd.
#[cfg(unix)]
mod owned_children;
mod pack_update_hint;
mod package;
mod package_cmd;
mod package_status;
mod platform;
mod policy;
mod profile;
mod profile_cmd;
mod profile_runtime;
#[cfg(not(target_os = "windows"))]
mod profile_save_runtime;
mod protected_paths;
mod proxy_command;
mod proxy_runtime;
#[cfg(not(target_os = "windows"))]
mod pty_proxy;
#[cfg(target_os = "windows")]
#[path = "pty_proxy_windows.rs"]
mod pty_proxy;
mod query_ext;
mod registry_client;
mod rollback_commands;
mod rollback_preflight;
mod rollback_runtime;
mod rollback_session;
mod rollback_ui;
mod sandbox_log;
mod sandbox_prepare;
mod sandbox_state;
mod session;
#[cfg(not(target_os = "windows"))]
mod session_commands;
#[cfg(target_os = "windows")]
#[path = "session_commands_windows.rs"]
mod session_commands;
mod setup;
// D-09 Phase 82: cert-store import logic (machine Root+TrustedPublisher + per-user
// CurrentUser\Root). The store-name list is the single source of truth here.
// Cross-target: non-Windows bodies are cfg-stubbed; compiles on Linux + macOS.
mod cert_trust;
// DEPLOY-06 Phase 82: read-only fleet diagnostic (nono health).
// Windows probes are cfg-gated; non-Windows stubs return degraded states so
// the crate builds cross-target without winreg or WFP deps.
mod health;
// D-09 Phase 82: idempotent first-run-in-user-context provisioner
// (scratch + cert + NODE_EXTRA_CA_CERTS). Windows-only module; the call site
// in command_runtime is cfg-gated to Windows so this module is never referenced
// on Linux / macOS.
#[cfg(target_os = "windows")]
mod provision_windows;
#[cfg(not(target_os = "windows"))]
mod startup_prompt;
mod startup_runtime;
mod state_paths;
mod supervised_runtime;
mod terminal_approval;
mod theme;
mod timeouts;
mod trust_cmd;
#[cfg(not(target_os = "windows"))]
mod trust_intercept;
#[cfg(target_os = "windows")]
#[path = "trust_intercept_windows.rs"]
mod trust_intercept;
mod trust_keystore;
mod trust_refresh;
mod trust_scan;
// Phase 84: SIEM/EDR telemetry layer (SecurityEventLayer + SecurityEvent schema
// + HMAC chain + path hashing + scrub integration).  Cross-platform: the
// Windows emitters are cfg-gated inside the module; the schema and chain code
// compile on Linux and macOS.
pub(crate) mod telemetry;
mod update_check;
mod why_runtime;
#[cfg(target_os = "windows")]
mod windows_wfp_contract;
mod wiring;

#[cfg(test)]
mod test_env;

// Quick task 260815-gfd: the single Windows fixture-ownership normalisation
// shared by every ownership-gated test in this crate. Test-only. It exists
// because the elevated `windows-latest` runner owns freshly created objects
// as BUILTIN\Administrators, which the user-SID equality predicate in
// `nono::path_is_owned_by_current_user` rejects — see the module docs.
#[cfg(test)]
#[cfg(target_os = "windows")]
mod test_ownership_windows;

// Phase 117 review WR-01: the single `#[cfg(test)]`-region classifier shared
// by every source-text drift gate in this crate. Test-only — it exists to stop
// those gates growing divergent private copies of the same predicate, which is
// exactly what WR-01 reports.
#[cfg(test)]
mod cfg_test_regions;

use app_runtime::run as run_cli;
use clap::Parser;
use cli::Cli;
use cli_bootstrap::{
    collect_legacy_network_warnings, init_theme, init_tracing, normalize_legacy_flag_env_vars,
    print_legacy_network_warnings,
};
use nono::Result;

const DETACHED_LAUNCH_ENV: &str = "NONO_DETACHED_LAUNCH";
const DETACHED_SESSION_ID_ENV: &str = "NONO_DETACHED_SESSION_ID";

pub(crate) use launch_runtime::rollback_base_exclusions;
// Upstream 72bcfd66 (#1225): pub(crate) use proxy_runtime::merge_dedup_ports removed;
// merge_dedup_ports is now only called inside proxy_runtime itself (no external callers).

fn main() {
    let legacy_network_warnings = collect_legacy_network_warnings();
    normalize_legacy_flag_env_vars();
    let cli = Cli::parse();
    // TELEM-04 / WR-01: read the HKLM machine-policy telemetry config and thread
    // it into the security-event layer so an admin's `TelemetryEnabled=0` opt-out,
    // `min_severity`, and channel are actually honored. The registry IS available
    // at this point (Cli is parsed); the prior `None` left the layer permanently
    // on the default-ON config, making the policy decorative.
    //
    // Telemetry is non-fatal (D-14): a malformed/absent policy degrades to
    // `TelemetryConfig::default()` (D-13 default-ON) — we never abort a run for a
    // telemetry read. Egress-policy validity is enforced separately on the daemon
    // path (D-07), so swallowing the egress error here does not weaken egress
    // enforcement.
    let telemetry_config = match nono::read_machine_egress_policy() {
        Ok(Some(policy)) => Some(policy.telemetry),
        Ok(None) => None,
        Err(_) => None,
    };
    init_tracing(&cli, telemetry_config);
    init_theme(&cli);
    print_legacy_network_warnings(&legacy_network_warnings, cli.silent);

    // Plan 20-03 D-10: emit startup warnings for deprecated command-blocking
    // surfaces (CLI flags, profile fields, manifest fields). Warnings-only;
    // enforcement behaviour is unchanged.
    let deprecation_warnings = command_blocking_deprecation::collect_cli_warnings(&cli);
    command_blocking_deprecation::print_warnings(&deprecation_warnings, cli.silent);

    if let Err(e) = run_cli(cli) {
        // Phase 36.5 D-36.5-A3 / D-36.5-D3: ActionRequired surfaces the
        // multi-line resolution text from `resolve_via` to stderr, then exits
        // non-zero. C FFI consumers see this as ErrConfigParse (-9) per D-36.5-B3.
        if let nono::NonoError::ActionRequired {
            expected,
            actual,
            resolve_via,
        } = &e
        {
            eprintln!("nono: action required: {resolve_via}");
            if !expected.is_empty() && !actual.is_empty() {
                eprintln!("  expected: {expected}");
                eprintln!("  actual:   {actual}");
            }
            std::process::exit(2);
        }
        for line in render_error_for_operator(&e) {
            eprintln!("{line}");
        }
        std::process::exit(1);
    }
}

/// Renders the lines printed to the operator for any `NonoError` that is not
/// `ActionRequired` (that variant has its own dedicated multi-line branch in
/// `main` above and never reaches this function).
///
/// Phase 117-21 WR-04: previously `main`'s generic fallback only ever printed
/// the bare `Display` line, even though `NonoError::remediation()` (Plan
/// 117-13, NR3-01) already computes a structured, operator-actionable
/// `ClearStaleLayerResidue { layer }` remediation for
/// `LayerAttestationFailed` — that guidance was invisible to an operator who
/// hit a genuine startup self-attestation failure. This function is factored
/// out of the inline `eprintln!` calls specifically so it is unit-testable
/// without a real `run_cli` invocation (no live process/CLI parse needed).
///
/// Always includes the `Display` line first; appends an additional
/// remediation line only when `e.remediation()` resolves to
/// `ClearStaleLayerResidue`. Other remediation variants are intentionally
/// left unrendered here — WR-04 scopes only the `LayerAttestationFailed`
/// case, and this function must not print a spurious line for errors whose
/// remediation this plan does not cover.
///
/// Phase 117-29 WR-17: the remediation line used to name the CLI's
/// check-only setup diagnostic for every failed layer, but that command
/// (`print_check_only_summary`) performs no mandatory-label inspection on
/// any path — it is only ever accurate for `MandatoryIntegrityLabel`, and
/// actively misleading for the other four layers
/// (`WfpEgressFilters`/`AppContainerProfile`/`JobObjectContainment`/a
/// forced-unavailable-seam abort). This now branches on `layer`: the
/// `MandatoryIntegrityLabel` case gets the real `icacls` remedy; every other
/// layer points at the Windows Application event log, where the per-layer
/// attestation record actually lives.
///
/// Phase 117-39 CR-06/WR-27: both arms were wrong.
///
/// - The `MandatoryIntegrityLabel` arm prescribed `icacls <path>
///   /setintegritylevel Medium`, which WRITES a Medium mandatory-label ACE.
///   `AppliedLabelsGuard`'s D-02 rule treats any prior label that is not an
///   exact Low+wanted-mask match as `SkipPreExistingLabel`, so an operator who
///   followed the only printed guidance aborted identically — verified on this
///   host: after `/setintegritylevel Medium`, `icacls` still reports
///   `Mandatory Label\Medium Mandatory Level:(NW)`. It also blamed "a prior
///   session that exited abnormally", a cause Plan 117-13 (NR3-01) had already
///   made NON-aborting (that residue is now recognized as self-healing).
///   The arm now names the causes that CAN still abort and prescribes REMOVAL
///   via `SetNamedSecurityInfoW(.., LABEL_SECURITY_INFORMATION, .., <empty
///   ACL>)` — the same mechanism `labels_guard::clear_mandatory_label` uses
///   internally, verified on this host to return `ERROR_SUCCESS` and clear the
///   label from a NON-elevated shell. (`Get-Acl -Audit`/`Set-Acl` was
///   evaluated and rejected: it requests the whole SACL and fails without
///   `SeSecurityPrivilege`, which a non-elevated operator does not hold.)
///
/// - The `_` arm pointed at the Windows Application event log, which receives
///   NOTHING on the abort path. Enumerated every `LayerAttestationFailed`
///   construction site in the workspace: all are plain `return Err(..)`, and
///   `emit_attestation_event` has exactly one production caller
///   (`launch.rs::apply_startup_attestation_gate`), inside the
///   `ProceedDowngraded` arm. The arm now points at the failing guard's own
///   `tracing` diagnostic, which does exist on this path.
fn render_error_for_operator(e: &nono::NonoError) -> Vec<String> {
    let mut lines = vec![format!("nono: {e}")];
    if let Some(nono::NonoRemediation::ClearStaleLayerResidue { layer }) = e.remediation() {
        let remediation = match layer.as_str() {
            "MandatoryIntegrityLabel" => format!(
                "nono:   {layer} could not be confirmed: a granted path already carries a \
                 mandatory-label ACE that is not the one this launch would apply (a \
                 third-party label, a different access mask, or a structurally-inert \
                 INHERIT_ONLY_ACE). nono never mutates a pre-existing label. Inspect with \
                 `icacls <granted-path>` (look for the `Mandatory Label\\...` line). The label \
                 must be REMOVED — `icacls /setintegritylevel Medium` writes a Medium label \
                 and re-triggers this same abort. In PowerShell: `Add-Type -Namespace N -Name \
                 L -MemberDefinition '[DllImport(\"advapi32.dll\",CharSet=CharSet.Unicode)] \
                 public static extern uint SetNamedSecurityInfoW(string p,int t,uint i,IntPtr \
                 o,IntPtr g,IntPtr d,byte[] s);'; [N.L]::SetNamedSecurityInfoW('<granted-path>',\
                 1,0x10,[IntPtr]::Zero,[IntPtr]::Zero,[IntPtr]::Zero,[byte[]]@(2,0,8,0,0,0,0,0))`"
            ),
            _ => format!(
                "nono:   {layer} could not be confirmed at startup. Re-run with `-vv \
                 --log-file <path>` and search that file for `{layer}` — the failing guard \
                 emits its own diagnostic there. (No Windows Application event log record is \
                 written on this path: the audit event is emitted only when nono PROCEEDS with \
                 a downgraded claim, never when it aborts.)"
            ),
        };
        lines.push(remediation);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::SandboxArgs;
    use crate::execution_runtime::execution_start_dir;
    use crate::launch_runtime::{
        resolve_requested_workdir, select_exec_strategy, select_threading_context,
        trust_interception_active,
    };
    use crate::proxy_runtime::{resolve_effective_proxy_settings, EffectiveProxySettings};
    #[cfg(target_os = "macos")]
    use crate::sandbox_prepare::maybe_enable_macos_launch_services;
    use crate::sandbox_prepare::PreparedSandbox;
    use crate::startup_runtime::allows_pre_exec_update_check;
    use nono::{AccessMode, CapabilitySet, FsCapability};

    fn sandbox_args() -> SandboxArgs {
        SandboxArgs::default()
    }

    /// The Medium-label command classification rule (CR-06 / WR-03), stated
    /// ONCE and applied to every mirror.
    ///
    /// `icacls <path> /setintegritylevel Medium` WRITES a Medium mandatory
    /// label — verified on this host to leave `Mandatory Label\Medium
    /// Mandatory Level:(NW)` in place, which D-02 reads back as
    /// `SkipPreExistingLabel`, aborting identically. It can therefore never be
    /// the remedy for a `MandatoryIntegrityLabel` attestation failure, and any
    /// surface that names it must name it as an explicit WARNING that it does
    /// not clear the condition.
    ///
    /// # Why a shared helper rather than two assertions
    ///
    /// WR-03 exists because the narrow needle survived in the function
    /// directly above the one WR-06 fixed, and WR-02 because the SPEC's D-15
    /// ledger went on prescribing the command as "the real remedy" long after
    /// the code stopped. Those are the same rule stated in three places. It is
    /// stated here once; `text` is whatever surface is being checked.
    ///
    /// Non-vacuity is the caller's job for the SPEC (where the command may
    /// legitimately vanish) and is asserted here for the rendered remediation,
    /// which is expected to keep warning about it.
    fn medium_label_command_mentions(label: &str, text: &str) -> usize {
        const MEDIUM_LABEL_CMD: &str = "/setintegritylevel Medium";
        // The qualifier must ATTACH to the mention. An unbounded "rest of the
        // text" window is vacuous on any long surface: the first draft of this
        // helper accepted the SPEC's restored prescriptive text because a
        // correction 400 characters later in the SAME ledger cell satisfied it.
        // Caught by running the perturbation, not by reading the code.
        const WINDOW: usize = 200;

        // Whitespace-normalise so a `\`-wrapped source literal or a
        // line-wrapped Markdown cell cannot split the needle — the exact
        // mechanism WR-01 found silently emptying a sibling gate.
        let normalised = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut mentions = 0usize;
        for (pos, _) in normalised.match_indices(MEDIUM_LABEL_CMD) {
            mentions += 1;
            let from = pos + MEDIUM_LABEL_CMD.len();
            // Char-boundary-safe truncation: the surfaces scanned contain
            // em-dashes and typographic quotes.
            let mut to = (from + WINDOW).min(normalised.len());
            while to > from && !normalised.is_char_boundary(to) {
                to -= 1;
            }
            let following = &normalised[from..to];
            assert!(
                following.contains("writes a Medium label") || following.contains("re-triggers"),
                "CR-06/WR-03: {label} names `{MEDIUM_LABEL_CMD}` without an attached warning \
                 (within {WINDOW} chars) that it does NOT clear the condition. It writes a \
                 Medium label and re-triggers the identical abort, so it can never be the \
                 remedy. Text: {text}"
            );
        }
        mentions
    }

    /// [`medium_label_command_mentions`] plus the non-vacuity floor, for
    /// surfaces that are expected to carry the warning.
    fn assert_medium_label_command_is_only_ever_negated(label: &str, text: &str) {
        let mentions = medium_label_command_mentions(label, text);
        assert!(
            mentions >= 1,
            "CR-06/WR-03 non-vacuity: {label} no longer warns about \
             `/setintegritylevel Medium` at all, so this guard now checks nothing. The \
             operator is expected to be told explicitly that it does NOT clear the \
             condition; restore the warning, or retire this guard deliberately. Text: {text}"
        );
    }

    /// WR-02: the SPEC is the other mirror of the CR-06 rule, and it was the
    /// one still prescribing the command.
    ///
    /// The Iteration-5 sweep for this class covered `crates/` but not `proj/`,
    /// so the D-15 ledger's WR-17 row went on recording, in the present tense,
    /// that `MandatoryIntegrityLabel` "gets the real remedy (`icacls <path>
    /// /setintegritylevel Medium`)" — a command `main.rs` carries a regression
    /// assertion against — and that every other layer "points at the Windows
    /// Application event log", a channel both `main.rs` and `output.rs` now
    /// assert is named only as a negation. The contract document an operator
    /// or the next planner reads prescribed what the code proves wrong.
    ///
    /// This closes the class rather than the instance: it applies the SAME
    /// helper to the SPEC that the rendered-string assertions apply to the
    /// remediation, so the two mirrors cannot state contradictory rules again.
    /// It deliberately does NOT assert a non-vacuity floor — the SPEC is
    /// allowed to stop mentioning the command entirely, and requiring a
    /// mention would pin prose that has no reason to persist.
    #[test]
    fn the_spec_never_prescribes_the_medium_label_command() {
        const SPEC: &str = include_str!("../../../proj/SPEC-windows-fail-direction-contract.md");

        let mut mentions = 0usize;
        for (idx, line) in SPEC.lines().enumerate() {
            mentions += medium_label_command_mentions(
                &format!("SPEC-windows-fail-direction-contract.md:{}", idx + 1),
                line,
            );
        }
        // Report the count so a reader of the test output can see whether the
        // scan found anything at all, without turning that into a floor.
        eprintln!(
            "WR-02: {mentions} `/setintegritylevel Medium` mention(s) in the SPEC, all \
             qualified as non-remedial"
        );
    }

    /// The class needle for the event-log rule, and the qualifiers that make a
    /// mention legitimate.
    ///
    /// Matched case-insensitively: the SPEC writes both "Windows Application
    /// Event Log" and "Windows Application event log", and a case-sensitive
    /// needle would be one more way to evade the rule.
    const EVENT_LOG_NEEDLE: &str = "windows application event log";

    /// A mention is legitimate only if one of these sits within
    /// [`EVENT_LOG_WINDOW`] characters of it, on either side.
    ///
    /// Two legitimate classes, and nothing else:
    ///
    /// - the **downgrade path**, which really does write there — pinned by the
    ///   audit event id and by the emitting symbol, both of which are facts
    ///   about that path rather than phrasings of it;
    /// - an **explicit negation**, which is how the abort path is allowed to
    ///   name it at all.
    const EVENT_LOG_QUALIFIERS: [&str; 4] = [
        "event id 10011",
        "emit_attestation_event",
        "explicit negation",
        "nothing is written there",
    ];

    /// Bounded, like [`medium_label_command_mentions`]'s. An unbounded "rest
    /// of the cell" window is vacuous on a 2000-character Markdown ledger row.
    const EVENT_LOG_WINDOW: usize = 200;

    /// Every mention of the event log in `text` that carries no attached
    /// qualifier, as `(offset, context)` pairs.
    ///
    /// # WR-04: this is a CLASS predicate, not a verb list
    ///
    /// The previous implementation was two present-tense verb phrases:
    ///
    /// ```text
    /// n.contains("points at the Windows Application event log")
    ///     || n.contains("point at the Windows Application event log")
    /// ```
    ///
    /// while its own doc stated the rule as the class "no ledger row asserts
    /// that non-label layers are POINTED AT the event log". `pointed at`,
    /// `pointing at`, `directs the operator to`, `refers … to`, `names … as
    /// the place to look` and `see the …` all evaded it, so the gate could
    /// relabel but never deny. This is the identical complaint round 2 raised
    /// as WR-06 against `!contains("see the Windows Application event log")`
    /// — and round 3 fixed that one correctly, then wrote the rejected narrow
    /// shape into this SPEC mirror in the same commit.
    ///
    /// The predicate is now "any mention that is not qualified", which no
    /// phrasing can evade: a new verb still has to explain itself.
    fn unqualified_event_log_mentions(text: &str) -> Vec<(usize, String)> {
        // Lowercase AND whitespace-normalise once, then work entirely in that
        // string: positions from a lowercased copy cannot index the original
        // safely, and the SPEC carries em-dashes and typographic quotes.
        let n = text
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let mut out = Vec::new();
        for (pos, _) in n.match_indices(EVENT_LOG_NEEDLE) {
            let mut from = pos.saturating_sub(EVENT_LOG_WINDOW);
            while from < pos && !n.is_char_boundary(from) {
                from += 1;
            }
            let mut to = pos
                .saturating_add(EVENT_LOG_NEEDLE.len())
                .saturating_add(EVENT_LOG_WINDOW)
                .min(n.len());
            while to > pos && !n.is_char_boundary(to) {
                to -= 1;
            }
            let ctx = &n[from..to];
            if !EVENT_LOG_QUALIFIERS.iter().any(|q| ctx.contains(q)) {
                out.push((pos, ctx.to_string()));
            }
        }
        out
    }

    /// WR-04 detector self-test: the predicate must FIRE on every phrasing the
    /// old two-verb needle let through, and stay SILENT on the real qualified
    /// mentions.
    ///
    /// The WR-05 raw-citation gate has one of these and it is why that gate
    /// can be trusted; this one had none, so its "perturbation" only ever
    /// exercised the single historical phrasing the needle was written from.
    #[test]
    fn the_event_log_detector_fires_on_the_whole_class() {
        for evader in [
            "| WR-17 | every other layer points at the Windows Application event log |",
            "| WR-17 | every other layer is pointed at the Windows Application event log |",
            "| WR-17 | pointing the operator at the Windows Application event log |",
            "| WR-17 | directs the operator to the Windows Application event log |",
            "| WR-17 | refers the operator to the Windows Application event log |",
            "| WR-17 | names the Windows Application event log as the place to look |",
            "| WR-17 | see the Windows Application event log for layer detail |",
            "| WR-17 | check the Windows Application Event Log for layer detail |",
        ] {
            assert!(
                !unqualified_event_log_mentions(evader).is_empty(),
                "WR-04: the detector missed a prescriptive phrasing — every one of these \
                 evaded the two-verb needle it replaces: {evader}"
            );
        }

        for legitimate in [
            "| WR-15 | now name the real Windows Application event log, event id 10011 |",
            "| WR-27 | with the Windows Application event log named ONLY as an explicit \
             negation, because nothing is written there on the abort path |",
            "| WR-15 | `emit_attestation_event` writes the Windows Application Event Log |",
        ] {
            assert!(
                unqualified_event_log_mentions(legitimate).is_empty(),
                "WR-04: the detector flagged a legitimately qualified mention — the downgrade \
                 path really does write there, and an explicit negation is how the abort path \
                 is allowed to name it: {legitimate}"
            );
        }

        // The window must be ATTACHED. A qualifier far away in the same
        // 2000-character ledger cell must NOT rescue the mention — that is the
        // exact vacuity the sibling helper's first draft shipped with.
        let far = format!(
            "| WR-17 | points at the Windows Application event log |{}| event id 10011 |",
            " filler".repeat(60)
        );
        assert!(
            !unqualified_event_log_mentions(&far).is_empty(),
            "WR-04: a qualifier {} characters away must not qualify the mention",
            " filler".repeat(60).len()
        );
    }

    /// WR-02: the SPEC must also state the event-log rule the code enforces.
    ///
    /// Same class, same document, same fix pass that missed it: `main.rs` and
    /// `output.rs` both assert the Windows Application event log is named only
    /// as an explicit negation on the abort path, while the SPEC's WR-17 row
    /// recorded "every other layer points at the Windows Application event
    /// log" as the resolution.
    ///
    /// The SPEC legitimately DESCRIBES the event log in other contexts (the
    /// downgrade path really does write there), so the rule is stated as: a
    /// D-15 ledger row may name the event log only with an ATTACHED qualifier
    /// establishing it as the downgrade-path destination or as an explicit
    /// negation. See [`unqualified_event_log_mentions`] for why this is a
    /// class predicate rather than a verb list (WR-04), and
    /// [`the_event_log_detector_fires_on_the_whole_class`] for the detector
    /// self-test that makes it trustworthy.
    #[test]
    fn the_spec_ledger_does_not_point_abort_path_layers_at_the_event_log() {
        const SPEC: &str = include_str!("../../../proj/SPEC-windows-fail-direction-contract.md");

        let mut offenders: Vec<String> = Vec::new();
        for (idx, line) in SPEC.lines().enumerate() {
            if !line.trim_start().starts_with('|') {
                continue;
            }
            for (_, ctx) in unqualified_event_log_mentions(line) {
                offenders.push(format!("SPEC:{}: …{ctx}…", idx + 1));
            }
        }

        assert!(
            offenders.is_empty(),
            "WR-02/WR-04: {} D-15 ledger mention(s) of the Windows Application event log carry \
             no attached qualifier (one of {:?} within {EVENT_LOG_WINDOW} chars). Nothing is \
             written there when nono ABORTS — every `LayerAttestationFailed` construction site \
             is a plain `return Err(..)` — and both `main.rs` and `output.rs` assert the code \
             names it only as a negation (WR-27/WR-06). Either qualify the mention as the \
             downgrade-path destination, or state it as an explicit negation:\n  {}",
            offenders.len(),
            EVENT_LOG_QUALIFIERS,
            offenders.join("\n  ")
        );
    }

    /// Phase 117-21 WR-04 / Phase 117-29 WR-17 / Phase 117-39 CR-06:
    /// `LayerAttestationFailed` for the `MandatoryIntegrityLabel` layer must
    /// render the `Display` line AND a remediation line that names a command
    /// which ACTUALLY CLEARS the condition.
    ///
    /// CR-06: the previous text prescribed `icacls <path> /setintegritylevel
    /// Medium`, which writes a Medium label — verified on this host to leave
    /// `Mandatory Label\Medium Mandatory Level:(NW)` in place, which D-02
    /// reads back as `SkipPreExistingLabel`, aborting identically. `icacls` is
    /// still named for INSPECTION; the removal step must not be it.
    #[test]
    fn render_error_for_operator_adds_remediation_for_layer_attestation_failed() {
        let e = nono::NonoError::LayerAttestationFailed {
            layer: "MandatoryIntegrityLabel".to_string(),
            reason: "residual ACE present".to_string(),
        };
        let lines = render_error_for_operator(&e);
        assert_eq!(
            lines.len(),
            2,
            "expected Display + remediation lines: {lines:?}"
        );
        assert!(lines[0].contains("MandatoryIntegrityLabel"));
        assert!(lines[0].contains("residual ACE present"));
        assert!(lines[1].contains("MandatoryIntegrityLabel"));
        // icacls remains the INSPECTION command.
        assert!(lines[1].contains("icacls"));
        // CR-06: but never as the remedy — it cannot remove a label.
        //
        // WR-03: this used to be `!contains("/setintegritylevel Medium`.")` —
        // command, closing backtick AND a sentence-final period. It rejected
        // exactly one historical sentence ending, so `` `icacls <path>
        // /setintegritylevel Medium` to clear this ``, or any phrasing that
        // does not end the sentence right there, evaded it entirely, while
        // the message claimed the far wider class "must never be prescribed
        // as the remedy". Match the CLASS — the command substring — and
        // exclude the one legitimate mention by requiring its NEGATION, the
        // same shape `render_error_for_operator_names_a_reachable_channel_
        // for_non_label_layers` uses for the event log.
        assert_medium_label_command_is_only_ever_negated("the rendered remediation", &lines[1]);
        // The remedy must name the removal mechanism.
        assert!(
            lines[1].contains("SetNamedSecurityInfoW"),
            "the remediation must name the label-REMOVAL mechanism: {}",
            lines[1]
        );
        // CR-06b: Plan 117-13 (NR3-01) made nono's own abnormal-exit residue
        // non-aborting, so blaming it is now a false cause.
        assert!(
            !lines[1].contains("exited abnormally"),
            "abnormal-exit residue is self-healing since NR3-01 and must not be blamed: {}",
            lines[1]
        );
    }

    /// Phase 117-29 WR-17 / Phase 117-39 WR-27: any layer other than
    /// `MandatoryIntegrityLabel` must be pointed at a channel that ACTUALLY
    /// RECEIVES a record on the abort path.
    ///
    /// WR-27: the previous text named the Windows Application event log, which
    /// receives nothing when nono aborts — every `LayerAttestationFailed`
    /// construction site in the workspace is a plain `return Err(..)`, and
    /// `emit_attestation_event` has exactly one production caller, inside
    /// `apply_startup_attestation_gate`'s `ProceedDowngraded` arm. The
    /// operator is now pointed at the failing guard's own `tracing`
    /// diagnostic, reachable via `-vv --log-file <path>`.
    #[test]
    fn render_error_for_operator_names_a_reachable_channel_for_non_label_layers() {
        let e = nono::NonoError::LayerAttestationFailed {
            layer: "WfpEgressFilters".to_string(),
            reason: "filter enumeration returned zero entries".to_string(),
        };
        let lines = render_error_for_operator(&e);
        assert_eq!(
            lines.len(),
            2,
            "expected Display + remediation lines: {lines:?}"
        );
        assert!(lines[1].contains("WfpEgressFilters"));
        assert!(
            lines[1].contains("--log-file"),
            "the operator must be pointed at a channel that exists on the abort path: {}",
            lines[1]
        );
        assert!(
            !lines[1].contains("nono setup --check-only"),
            "check-only performs no per-layer inspection (WR-17): {}",
            lines[1]
        );
        // WR-27: the event log must not be named as the place to look.
        //
        // WR-06: assert the CLASS, not one verb. The previous predicate was
        // `!contains("see the Windows Application event log")`, which any
        // other verb ("check the ...", "in the ...") evaded — a guard that
        // could relabel but never deny. The legitimate mention here is an
        // explicit NEGATION, so require the negation rather than narrowing
        // the needle. Kept in the same shape as `output.rs`'s source-text
        // mirror of this rule, so the two cannot state contradictory rules
        // for one condition.
        const EVENT_LOG: &str = "Windows Application event log";
        let normalised = lines[1].split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalised.contains(EVENT_LOG),
            "WR-06 non-vacuity: the remediation no longer mentions the event log at all, so \
             the negation check below verifies nothing. The `_` arm is expected to tell the \
             operator explicitly NOT to look there; restore it, or retire this guard \
             deliberately: {}",
            lines[1]
        );
        for (pos, _) in normalised.match_indices(EVENT_LOG) {
            let preceding = &normalised[..pos];
            assert!(
                preceding.ends_with("No ") || preceding.ends_with("no "),
                "WR-27/WR-06: the remediation mentions the Windows Application event log \
                 other than as an explicit negation, but no record is written there on the \
                 abort path: {}",
                lines[1]
            );
        }
    }

    /// Any other error variant renders unchanged from today — only the
    /// `Display` line, no spurious remediation line.
    #[test]
    fn render_error_for_operator_is_unchanged_for_other_variants() {
        let e = nono::NonoError::NoCapabilities;
        let lines = render_error_for_operator(&e);
        assert_eq!(lines.len(), 1, "no remediation line expected: {lines:?}");
        assert!(lines[0].starts_with("nono: "));
    }

    #[test]
    fn test_sensitive_paths_defined() {
        let loaded_policy = policy::load_embedded_policy().expect("policy must load");
        let paths = policy::get_sensitive_paths(&loaded_policy).expect("must resolve");
        assert!(paths.iter().any(|rule| rule.expanded_path.contains("ssh")));
        assert!(paths.iter().any(|rule| rule.expanded_path.contains("aws")));
    }

    #[test]
    fn test_dangerous_commands_defined() {
        let loaded_policy = policy::load_embedded_policy().expect("policy must load");
        let commands = policy::get_dangerous_commands(&loaded_policy);
        assert!(commands.contains("rm"));
        assert!(commands.contains("dd"));
        assert!(commands.contains("chmod"));
    }

    #[test]
    fn test_check_blocked_command_basic() {
        assert!(config::check_blocked_command("echo", &[], &[])
            .expect("policy must load")
            .is_none());
        assert!(config::check_blocked_command("ls", &[], &[])
            .expect("policy must load")
            .is_none());
        assert!(config::check_blocked_command("cat", &[], &[])
            .expect("policy must load")
            .is_none());
    }

    #[test]
    fn test_check_blocked_command_with_path() {
        let blocked = vec!["rm".to_string(), "dd".to_string()];
        assert!(config::check_blocked_command("/bin/rm", &[], &blocked)
            .expect("policy must load")
            .is_some());
        assert!(config::check_blocked_command("/usr/bin/dd", &[], &blocked)
            .expect("policy must load")
            .is_some());
        assert!(config::check_blocked_command("./rm", &[], &blocked)
            .expect("policy must load")
            .is_some());
    }

    #[test]
    fn test_check_blocked_command_allow_override() {
        let allowed = vec!["rm".to_string()];
        let blocked = vec!["rm".to_string(), "dd".to_string()];
        assert!(config::check_blocked_command("rm", &allowed, &blocked)
            .expect("policy must load")
            .is_none());
        assert!(config::check_blocked_command("dd", &allowed, &blocked)
            .expect("policy must load")
            .is_some());
    }

    #[test]
    fn test_check_blocked_command_extra_blocked() {
        let extra = vec!["custom-dangerous".to_string()];
        assert!(
            config::check_blocked_command("custom-dangerous", &[], &extra)
                .expect("policy must load")
                .is_some()
        );
        assert!(config::check_blocked_command("rm", &[], &extra)
            .expect("policy must load")
            .is_none());
    }

    #[test]
    fn test_check_blocked_command_uses_resolved_policy_only() {
        assert!(config::check_blocked_command("rm", &[], &[])
            .expect("policy must load")
            .is_none());
    }

    #[test]
    fn test_resolve_effective_proxy_settings_allow_net_clears_profile_proxy_state() {
        let args = SandboxArgs {
            allow_net: true,
            ..sandbox_args()
        };
        let prepared = PreparedSandbox {
            caps: CapabilitySet::new(),
            secrets: Vec::new(),
            rollback_exclude_patterns: Vec::new(),
            rollback_exclude_globs: Vec::new(),
            network_profile: Some("developer".to_string()),
            allow_domain: vec![crate::profile::AllowDomainEntry::Plain(
                "docs.python.org".to_string(),
            )],
            deny_domain: Vec::new(),
            // Non-empty so this test also proves --allow-net clears a
            // profile-declared no_proxy (mirrors the deny_domain assertion).
            no_proxy: vec!["internal-tool".to_string()],
            credentials: vec!["github".to_string()],
            custom_credentials: std::collections::HashMap::new(),
            upstream_proxy: None,
            upstream_bypass: Vec::new(),
            listen_ports: Vec::new(),
            capability_elevation: false,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy::Error,
            #[cfg(target_os = "linux")]
            af_unix_mediation: crate::profile::LinuxAfUnixMediation::Off,
            #[cfg(target_os = "linux")]
            proc_comm_notify: false,
            allow_launch_services_active: false,
            open_url_origins: Vec::new(),
            open_url_allow_localhost: false,
            bypass_protection_paths: Vec::new(),
            ignored_denial_paths: Vec::new(),
            suppressed_system_service_operations: Vec::new(),
            // Plan 34-08a Task 3 (D-20 replay of `1b412a7`): test fixture
            // has no env-filter allow-list.
            allowed_env_vars: None,
            // Plan 34-08a Task 4 (D-20 replay of v0.52.0 `3657c935`):
            // test fixture has no env-filter deny-list either.
            denied_env_vars: None,
            set_vars: None,
            profile_network_block: false,
            // Plan 18.1-03 G-06: test fixture has no loaded profile.
            loaded_profile: None,
            // Phase 58: test fixture has no session hooks.
            session_hooks: crate::profile::SessionHooks::default(),
            // Upstream cdeeb5b9: test fixture does not request HTTP/2.
            allow_http2_requested: false,
        };

        let effective = resolve_effective_proxy_settings(&args, &prepared);

        assert_eq!(
            effective,
            EffectiveProxySettings {
                network_profile: None,
                allow_domain: Vec::new(),
                deny_domain: Vec::new(),
                no_proxy: Vec::new(),
                credentials: Vec::new(),
            }
        );
    }

    #[test]
    fn test_resolve_effective_proxy_settings_merges_cli_and_profile() {
        let args = SandboxArgs {
            network_profile: Some("minimal".to_string()),
            allow_proxy: vec!["example.com".to_string()],
            proxy_credential: vec!["openai".to_string()],
            ..sandbox_args()
        };
        let prepared = PreparedSandbox {
            caps: CapabilitySet::new(),
            secrets: Vec::new(),
            rollback_exclude_patterns: Vec::new(),
            rollback_exclude_globs: Vec::new(),
            network_profile: Some("developer".to_string()),
            allow_domain: vec![crate::profile::AllowDomainEntry::Plain(
                "docs.python.org".to_string(),
            )],
            deny_domain: Vec::new(),
            // Profile-only (no CLI flag to merge) — proves the straight
            // clone-through in resolve_effective_proxy_settings.
            no_proxy: vec!["internal-tool".to_string()],
            credentials: vec!["github".to_string()],
            custom_credentials: std::collections::HashMap::new(),
            upstream_proxy: None,
            upstream_bypass: Vec::new(),
            listen_ports: Vec::new(),
            capability_elevation: false,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy::Error,
            #[cfg(target_os = "linux")]
            af_unix_mediation: crate::profile::LinuxAfUnixMediation::Off,
            #[cfg(target_os = "linux")]
            proc_comm_notify: false,
            allow_launch_services_active: false,
            open_url_origins: Vec::new(),
            open_url_allow_localhost: false,
            bypass_protection_paths: Vec::new(),
            ignored_denial_paths: Vec::new(),
            suppressed_system_service_operations: Vec::new(),
            // Plan 34-08a Task 3 (D-20 replay of `1b412a7`): test fixture
            // has no env-filter allow-list.
            allowed_env_vars: None,
            // Plan 34-08a Task 4 (D-20 replay of v0.52.0 `3657c935`):
            // test fixture has no env-filter deny-list either.
            denied_env_vars: None,
            set_vars: None,
            profile_network_block: false,
            // Plan 18.1-03 G-06: test fixture has no loaded profile.
            loaded_profile: None,
            // Phase 58: test fixture has no session hooks.
            session_hooks: crate::profile::SessionHooks::default(),
            // Upstream cdeeb5b9: test fixture does not request HTTP/2.
            allow_http2_requested: false,
        };

        let effective = resolve_effective_proxy_settings(&args, &prepared);

        assert_eq!(
            effective,
            EffectiveProxySettings {
                network_profile: Some("minimal".to_string()),
                allow_domain: vec![
                    crate::profile::AllowDomainEntry::Plain("docs.python.org".to_string()),
                    crate::profile::AllowDomainEntry::Plain("example.com".to_string()),
                ],
                deny_domain: Vec::new(),
                no_proxy: vec!["internal-tool".to_string()],
                credentials: vec!["github".to_string(), "openai".to_string()],
            }
        );
    }

    #[test]
    fn test_trust_interception_inactive_for_default_policy() {
        let policy = nono::trust::TrustPolicy::default();

        assert!(!trust_interception_active(Some(&policy)));
    }

    #[test]
    fn test_trust_interception_active_when_includes_exist() {
        let policy = nono::trust::TrustPolicy {
            includes: vec!["SKILLS.md".to_string()],
            ..nono::trust::TrustPolicy::default()
        };

        assert!(trust_interception_active(Some(&policy)));
    }

    #[test]
    fn test_select_exec_strategy_uses_supervised_for_plain_run() {
        assert_eq!(
            select_exec_strategy(false, false, false, false, false),
            exec_strategy::ExecStrategy::Supervised
        );
    }

    #[test]
    fn test_select_exec_strategy_uses_supervised_for_rollback() {
        assert_eq!(
            select_exec_strategy(true, false, false, false, false),
            exec_strategy::ExecStrategy::Supervised
        );
    }

    #[test]
    fn test_select_exec_strategy_uses_supervised_for_proxy() {
        assert_eq!(
            select_exec_strategy(false, true, false, false, false),
            exec_strategy::ExecStrategy::Supervised
        );
    }

    #[test]
    fn test_select_exec_strategy_uses_supervised_for_capability_elevation() {
        assert_eq!(
            select_exec_strategy(false, false, true, false, false),
            exec_strategy::ExecStrategy::Supervised
        );
    }

    #[test]
    fn test_select_exec_strategy_uses_supervised_for_trust_interception() {
        assert_eq!(
            select_exec_strategy(false, false, false, true, false),
            exec_strategy::ExecStrategy::Supervised
        );
    }

    #[test]
    fn test_select_exec_strategy_uses_supervised_for_detached_start() {
        assert_eq!(
            select_exec_strategy(false, false, false, false, true),
            exec_strategy::ExecStrategy::Supervised
        );
    }

    #[test]
    fn test_pre_exec_update_check_disabled_for_execution_commands() {
        let run = Cli::parse_from(["nono", "run", "--allow", "/tmp", "--", "/bin/sh"]);
        assert!(!allows_pre_exec_update_check(&run.command));

        let shell = Cli::parse_from(["nono", "shell", "--allow", "/tmp"]);
        assert!(!allows_pre_exec_update_check(&shell.command));

        let wrap = Cli::parse_from(["nono", "wrap", "--allow", "/tmp", "--", "/bin/sh"]);
        assert!(!allows_pre_exec_update_check(&wrap.command));
    }

    #[test]
    fn test_pre_exec_update_check_disabled_for_completions() {
        // `nono completions` is used in shell init scripts such as
        // `eval "$(nono completions zsh)"`.  It never shows an update
        // notification (it is dispatched directly without
        // run_command_with_update), so spawning the background update-check
        // thread would incur network I/O with no benefit.
        let completions = Cli::parse_from(["nono", "completion", "zsh"]);
        assert!(!allows_pre_exec_update_check(&completions.command));
    }

    #[test]
    fn test_pre_exec_update_check_disabled_for_pack_update_hint_helper() {
        let helper = Cli::parse_from([
            "nono",
            "pack-update-hint-helper",
            "always-further/claude",
            "1.0.0",
        ]);
        assert!(!allows_pre_exec_update_check(&helper.command));
    }

    #[test]
    fn test_pre_exec_update_check_enabled_for_non_exec_commands() {
        let why = Cli::parse_from(["nono", "why", "--path", "/tmp", "--op", "read"]);
        assert!(allows_pre_exec_update_check(&why.command));

        let ps = Cli::parse_from(["nono", "ps"]);
        assert!(allows_pre_exec_update_check(&ps.command));
    }

    #[test]
    fn test_select_threading_context_uses_crypto_for_trust_scan() {
        assert_eq!(
            select_threading_context(false, false, true, false),
            exec_strategy::ThreadingContext::CryptoExpected
        );
    }

    #[test]
    fn test_select_threading_context_uses_keyring_for_secrets_only() {
        assert_eq!(
            select_threading_context(true, false, false, false),
            exec_strategy::ThreadingContext::KeyringExpected
        );
    }

    #[test]
    fn test_resolve_requested_workdir_prefers_explicit_path() {
        let explicit = std::path::PathBuf::from("/tmp/nono-workdir");
        assert_eq!(
            resolve_requested_workdir(Some(&explicit)),
            std::path::PathBuf::from("/tmp/nono-workdir")
        );
    }

    #[test]
    fn test_execution_start_dir_keeps_workdir_when_covered() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir.path().canonicalize().expect("canonicalize");
        let mut caps = CapabilitySet::new();
        caps.add_fs(FsCapability::new_dir(dir.path(), AccessMode::Read).expect("grant"));

        let start_dir = execution_start_dir(dir.path(), &caps).expect("start dir");

        assert_eq!(start_dir, canonical);
    }

    #[test]
    fn test_execution_start_dir_falls_back_to_root_when_not_covered() {
        let dir = tempfile::tempdir().expect("tempdir");
        #[cfg(target_os = "windows")]
        let canonical = dir.path().canonicalize().expect("canonicalize");
        let caps = CapabilitySet::new();

        let start_dir = execution_start_dir(dir.path(), &caps).expect("start dir");

        #[cfg(target_os = "windows")]
        assert_eq!(start_dir, canonical);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(start_dir, std::path::PathBuf::from("/"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_maybe_enable_macos_launch_services_adds_rule_when_enabled() {
        let mut caps = CapabilitySet::new();

        let enabled = maybe_enable_macos_launch_services(
            &mut caps,
            true,
            true,
            &["https://claude.ai".to_string()],
            false,
        )
        .expect("launch services gate should apply");

        assert!(enabled, "launch services should be active");
        assert!(
            caps.platform_rules().iter().any(|r| r == "(allow lsopen)"),
            "lsopen platform rule should be present"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_maybe_enable_macos_launch_services_rejects_without_profile_opt_in() {
        let mut caps = CapabilitySet::new();

        let err = maybe_enable_macos_launch_services(
            &mut caps,
            true,
            false,
            &["https://claude.ai".to_string()],
            false,
        )
        .expect_err("missing profile opt-in should fail");

        assert!(
            err.to_string().contains("requires a profile"),
            "error should mention profile opt-in"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_maybe_enable_macos_launch_services_rejects_without_open_urls() {
        let mut caps = CapabilitySet::new();

        let err = maybe_enable_macos_launch_services(&mut caps, true, true, &[], false)
            .expect_err("missing open_urls should fail");

        assert!(
            err.to_string().contains("configure open_urls"),
            "error should mention open_urls"
        );
    }
}
