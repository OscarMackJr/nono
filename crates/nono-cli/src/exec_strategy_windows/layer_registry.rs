//! Phase 117 Plan 01 (CINT-01): the code-resident Windows fail-direction
//! layer registry — the single source of truth every other Phase 117
//! deliverable (the `proj/` SPEC, Plan 08's startup self-attestation, the
//! Plan 12 per-layer forced-unavailable tests) reads from. D-01: the
//! registry wins; the markdown contract is generated from it or
//! drift-checked against it, never hand-maintained in parallel.
//!
//! # Evidence ledger (D-16 shape — command run, hit count, date measured)
//!
//! ## 1. SC4-3 resolution: `FirewallRulesEgress` is its own row, not a WFP sub-case
//!
//! RESEARCH left **SC4-3** ("is `FirewallRulesNetworkBackend` reachable in
//! production, or dead code superseded by WFP?") unresolved. Re-verified live
//! 2026-08-09 by reading `crates/nono-cli/src/exec_strategy_windows/network.rs`
//! directly (not re-grepped — this citation reuses the line numbers already
//! confirmed in 117-01-PLAN.md's `<interfaces>` block):
//!
//! - `network.rs:1500` (`select_network_backend`, the dispatch function) —
//!   `FirewallRulesNetworkBackend` is selected at the match arm
//!   `(WindowsNetworkPolicyMode::Blocked, WindowsNetworkBackendKind::FirewallRules)`.
//!   This is a live, reachable dispatch arm keyed on the policy's
//!   `active_backend`, not dead code: a policy compiled with
//!   `active_backend: FirewallRules` reaches this arm on every launch, and
//!   `WfpNetworkBackend` is a structurally distinct arm
//!   (`(Blocked, Wfp)`, `network.rs:1513-1516`) — the two backends are
//!   alternatives the policy compiler chooses between, not a fallback chain
//!   where one supersedes the other.
//! - `network.rs:1535` (`impl WindowsNetworkBackend for FirewallRulesNetworkBackend`,
//!   the `install()` method) — mechanism: per-program `netsh advfirewall
//!   firewall add rule` block rules (`dir=out`/`dir=in`, `action=block`,
//!   scoped to the staged program path). This is a materially different
//!   enforcement primitive from WFP's session/package-SID-scoped `FWP_MATCH_RANGE`
//!   allow filters (`network.rs:1602-1610` dispatches to
//!   `install_wfp_network_backend`) — program-path identity vs. SID identity,
//!   block-list vs. allow-list — so it earns its own registry row
//!   (`LayerId::FirewallRulesEgress`) rather than being folded into
//!   `WfpEgressFilters` as a variant.
//! - Fail-closed confirmed: `install()` (`network.rs:1540-1594`) propagates
//!   any `run_netsh_firewall` `Err` immediately (`network.rs:1567-1569` for the
//!   outbound rule, `:1583-1586` for the inbound rule), and on inbound-rule
//!   failure it rolls back the already-added outbound rule
//!   (`delete_firewall_rule`, `:1583`) before propagating — no partial-rule
//!   state is left installed while reporting success.
//!
//! **Verdict: `FirewallRulesEgress` is IN as its own row, `outcome:
//! ContractOutcome::Abort`.**
//!
//! ## 2. Cross-target `#[cfg(target_os = "linux"|"macos")]` presence table
//!
//! Confirmed live 2026-08-09 (direct grep for the literal `target_os =
//! "linux"` / `target_os = "macos"` cfg-attribute strings across each file;
//! reused verbatim from 117-01-PLAN.md's `<interfaces>` block, which was
//! itself re-verified the same day — Plans 04/09/10/12 should cite this
//! table rather than re-grepping):
//!
//! | File | Has literal linux/macos cfg? |
//! |---|---|
//! | `crates/nono/src/sandbox/mod.rs` | YES |
//! | `crates/nono/src/machine_policy.rs` | NO |
//! | `crates/nono-cli/src/telemetry/mod.rs`, `event.rs` | NO |
//! | `crates/nono-cli/src/command_runtime.rs` | YES |
//! | `crates/nono-cli/src/agent_daemon/launch.rs` | NO |
//! | `crates/nono-shell-broker/src/main.rs` | NO |
//! | `crates/nono-cli/src/exec_strategy_windows/*.rs` (all files) | NO |
//! | `crates/nono-cli/src/execution_runtime.rs`, `cli.rs`, `crates/nono/src/error.rs` | YES (RESEARCH §I, unchanged) |
//!
//! Consequence for D-11: this file itself (`exec_strategy_windows/*.rs`) has
//! no literal Unix `cfg` branch, so authoring it does not, on its own,
//! trigger the cross-target clippy MUST — but `execution_runtime.rs`/`cli.rs`
//! are in the blast radius of any later plan (03/08/09/10) that wires this
//! registry into the CLI's decision logic, and those DO carry Unix `cfg`
//! branches per the table above.
//!
//! ## 3. D-10 in/out call for every RESEARCH §A candidate row not already
//!    covered by CINT-01's six named layers
//!
//! - **Job Object containment — IN, full row (`LayerId::JobObjectContainment`).**
//!   `crates/nono-cli/src/exec_strategy_windows/launch.rs:375`
//!   (`apply_process_handle_to_containment`) assigns the suspended child to
//!   the containment Job Object via `AssignProcessToJobObject` and fails
//!   closed: its caller (`launch.rs:2134-2137`) terminates the suspended
//!   process and propagates `Err` on any assignment failure, before
//!   `ResumeThread` ever runs. This is a genuine confinement guarantee
//!   (process-tree lifetime + resource limits enforced by the kernel via the
//!   Job Object), not incidental bookkeeping, so it earns a full row exactly
//!   like the six CINT-01-named layers.
//! - **Env sanitization — OUT (data hygiene, not a kernel confinement
//!   guarantee).** `env_sanitization.rs`'s `is_dangerous_env_var`/
//!   `validate_env_var_patterns`/`validate_set_vars` filter which environment
//!   variables the child process inherits. A compromised child can read its
//!   own environment and cannot be made to "forget" a variable it never
//!   needed in the first place — this is a supply-side hygiene control (limit
//!   what secrets are handed to the child), not a structural barrier the OS
//!   enforces against the child's own actions the way a restricted token,
//!   mandatory label, AppContainer boundary, DACL grant, or WFP filter is.
//!   It stays out of this registry; it is not a layer whose absence widens
//!   what the confined process can DO, only what it was TOLD.
//! - **`validate_windows_launch_paths` interpreter coverage gate — IN, but as
//!   a pre-flight row, not a post-spawn attestation row
//!   (`LayerId::InterpreterCoverageGate`).**
//!   `crates/nono/src/sandbox/windows.rs:2247` (`validate_launch_paths`, the
//!   implementation `crates/nono/src/sandbox/mod.rs:925`'s
//!   `Sandbox::validate_windows_launch_paths` delegates to) already
//!   fail-secure-refuses: an interpreter the wrapper program will spawn that
//!   is NOT covered by the compiled filesystem policy causes the gate to
//!   return `Err` naming both the uncovered interpreter and the wrapper
//!   program (confirmed by the `validate_launch_paths_refuses_uncovered_interpreter`
//!   unit test, `crates/nono/src/sandbox/windows.rs:3903`). This check runs
//!   BEFORE spawn (`mod.rs:357-362`, inside `prepare_live_windows_launch`,
//!   ahead of the label/DACL/network guards), so there is nothing to
//!   re-attest against a live child afterward — the row's `probe` is
//!   `ProbeKind::NotApplicable` with a doc-comment note "validated pre-spawn,
//!   not re-checked post-spawn," per this task's instruction, rather than a
//!   D-21 post-spawn attestation probe.
//! - **Broker Authenticode trust gate — IN, scoped to the `Broker`/
//!   `BrokerLaunchNoPty` entry paths only (`LayerId::BrokerAuthenticodeTrustGate`).**
//!   `crates/nono-cli/src/exec_strategy_windows/launch.rs:2236`
//!   (`verify_broker_authenticode`), called at `launch.rs:1505-1506` and
//!   `:1828-1829` guarded by `is_dev_build_layout` (`launch.rs:2190`) — the
//!   gate is fail-closed in production install layouts and is skipped ONLY
//!   under a compile-time-baked dev-layout detector (`launch.rs:2194`'s
//!   parameterized core decision), never a runtime toggle. This only applies
//!   on the arms that actually spawn `nono-shell-broker.exe`
//!   (`BrokerLaunch`/`BrokerLaunchNoPty`), so it is IN as a row scoped to
//!   those entry paths, not a universal row.
//! - **Proxy egress — OUT (cross-platform, BOUND-01/Phase 119 territory, not
//!   a Windows-backend-composed layer).** The proxy (`nono-proxy`) enforces
//!   network policy identically on every platform; it is not part of the
//!   Windows-backend composite this registry enumerates (D-09: this registry
//!   is Windows-only, but specifically the Windows-*backend's own* layer
//!   composition — `restricted token ∩ mandatory label ∩ AppContainer ∩ DACL
//!   ∩ WFP`), and its cross-platform security-model boundary is explicitly
//!   Phase 119's BOUND-01/02/03 territory (D-12).
//!
//! ## 4. Blocker-1 (checker pass 2, 2026-08-09) — the AppContainer
//!    cross-process attestation gap
//!
//! RESEARCH §Summary-2 / §D: on the broker arms (`BrokerLaunch`/
//! `BrokerLaunchNoPty`), the process nono-cli's own `spawn_windows_child`
//! gate (`launch.rs`, `EntryPath::DirectCli`) observes IS
//! **`nono-shell-broker.exe`** itself — Medium-IL and deliberately
//! unconfined. The real AppContainer-confined grandchild is spawned later,
//! inside the broker's own separate **`CREATE_SUSPENDED`** window
//! (`nono-shell-broker/src/main.rs:537-661`, package-SID registration at
//! `:322-336`). Probing `AppContainerProfile` from `EntryPath::DirectCli`
//! would inspect the wrong process before the real target process even
//! exists — reproducing CINT-02's own failure mode inside its own fix.
//!
//! **Resolution:** `AppContainerProfile`'s expectancy is `expected: true`
//! ONLY at `(EntryPath::Broker, None)` and `(EntryPath::Daemon, None)` —
//! never at any `(EntryPath::DirectCli, ...)` cell. The `(EntryPath::Broker,
//! None)` cell is genuinely attested, just not by this crate's
//! `attest_and_decide` (Plan 08) — `nono-shell-broker` is a separate binary
//! that cannot call into `nono-cli`'s attestation module. It is attested by
//! `nono-shell-broker`'s own local probe-then-decide (Plan 11), driven by
//! the `NONO_BROKER_REQUIRED_LAYERS` env-var contract Plan 08's
//! `required_layers_for_broker()` derives directly from this row. This is
//! the registry's way of expressing "expected at this arm, attested at a
//! different gate site" — the `EntryPath::Broker` variant itself IS that
//! expression.
//!
//! No other row shares this gap: `DaclPackageSidGrant`/`WfpEgressFilters`/
//! `JobObjectContainment`'s dual `(DirectCli, ...)`/`(Daemon, ...)` citations
//! are each attested by that SAME process's own `attest_and_decide` call (the
//! daemon spawns and probes its own child directly — no intermediary
//! process, per RESEARCH §B's confirmation that `agent_daemon/launch.rs`
//! never delegates to a second binary); `RestrictedToken`/
//! `DaclSessionSidGrant` are `WriteRestricted`-arm-only and never present on
//! a broker arm at all.
//!
//! ## 5. Item-1 (checker pass 3, 2026-08-09) — the broker-outcome invariant
//!
//! Blocker-1's fix means `required_layers_for_broker()` (Plan 08) filters
//! registry rows to those with an `ArmExpectancy` matching
//! `(EntryPath::Broker, expected: true)` AND `outcome:
//! ContractOutcome::Abort` — any row expected at `EntryPath::Broker` with a
//! NON-`Abort` outcome is silently excluded from that filter AND already
//! excluded from `attest_and_decide`'s own dispatch (which never runs with
//! `EntryPath::Broker`), so such a row would go **completely unattested** on
//! the broker arm — the exact green-by-absence gap Blocker-1 just closed,
//! re-armed for the next added row.
//!
//! Every row's `ContractOutcome` is therefore spelled out explicitly in this
//! module (never inferred or left as a default — see the `LayerRegistryEntry`
//! population below), and a discovery-based unit test,
//! `broker_expected_rows_are_abort_only` (below), mechanically asserts
//! **every `EntryPath::Broker`-expected row must have
//! `ContractOutcome::Abort`** — iterating `all_entries()`, never naming
//! `AppContainerProfile` (or any other variant) by string, so a future
//! violating row fails the test on its own rather than requiring a reviewer
//! to notice the omission.
//!
//! Task 2 of this plan adds the `LayerId` enum, the expectancy-matrix types,
//! and the 13-row registry data below this doc comment. Task 3 adds the
//! exhaustive drift guard, the broker-abort-only invariant test, and wires
//! this module into `mod.rs`.

/// Identity of every layer the Windows backend composes into the deny-by-
/// composition confinement model, plus the deliberate structural-absence
/// row (`MinifilterAbsence`, ADR-65/D-33). D-01: this enum, not the
/// `proj/` SPEC, is the source of truth — the SPEC is generated from or
/// drift-checked against `ALL` (Task 3) and the registry data below.
///
/// D-11: platform-neutral declaration (no `#[cfg]` here) so this type
/// compiles and is usable (docs, drift tests, Phase 118's receipt type)
/// on every host; only the *population* (`all_entries`, Task 3) is
/// `#[cfg(target_os = "windows")]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayerId {
    /// WRITE_RESTRICTED token + per-session synthetic restricting SID.
    RestrictedToken,
    /// NO_WRITE_UP (± NO_READ_UP/NO_EXECUTE_UP) mandatory-label ACE on
    /// every compiled filesystem-policy path.
    MandatoryIntegrityLabel,
    /// Per-run AppContainer profile + derived package SID applied to the
    /// confined child's token at spawn time.
    AppContainerProfile,
    /// DACL grant of the synthetic per-session SID on writable filesystem
    /// grants — the `WriteRestricted`-arm double-check companion to
    /// `RestrictedToken`.
    DaclSessionSidGrant,
    /// DACL grant of the per-run AppContainer package SID on writable
    /// filesystem grants.
    DaclPackageSidGrant,
    /// DACL grant of `FILE_TRAVERSE` to the package SID on user-owned
    /// ancestors of the confined cwd.
    DaclAncestorTraverse,
    /// DACL grant of `FILE_READ_ATTRIBUTES` to the package SID on
    /// user-owned ancestors of the resolved binary / workspace chains.
    DaclAncestorReadAttrs,
    /// Session/package-SID-scoped WFP `ALE_USER_ID` allow-filter egress
    /// enforcement, installed via the elevated `nono-wfp-service`.
    WfpEgressFilters,
    /// Program-path-scoped `netsh advfirewall` block-rule egress
    /// enforcement — the legacy/fallback network backend, distinct from
    /// WFP (SC4-3, resolved above).
    FirewallRulesEgress,
    /// The deliberate structural-absence row: no production minifilter
    /// exists in this tree (ADR-65/D-33). Per-file read policy within one
    /// directory is explicitly not claimed.
    MinifilterAbsence,
    /// Job Object assignment (kill-group + `--timeout` enforcement) on the
    /// suspended child, with a deny-DACL on the job object itself.
    JobObjectContainment,
    /// Authenticode signature comparison between `nono.exe` and the
    /// sibling `nono-shell-broker.exe` before spawning the broker.
    BrokerAuthenticodeTrustGate,
    /// Pre-spawn static coverage check: does the compiled filesystem
    /// policy already cover the program, cwd, and every resolved
    /// interpreter path the wrapper will spawn.
    InterpreterCoverageGate,
}

/// Axis 2 of the D-08 expectancy matrix (RESEARCH §B): which *binary*
/// handles the spawn. Distinct from `WindowsTokenArm` (axis 1, decided by
/// `select_windows_token_arm`) — the daemon path bypasses that cascade
/// entirely (RESEARCH §B: "N/A — daemon path does not go through
/// `select_windows_token_arm` at all"), and the broker's own suspended
/// spawn of the real confined child (`nono-shell-broker/src/main.rs:537-661`)
/// is a second, independent `CREATE_SUSPENDED` window that `WindowsTokenArm`
/// does not model either — hence `Broker` is its own `EntryPath` variant,
/// not folded into `DirectCli`'s `BrokerLaunch`/`BrokerLaunchNoPty` arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryPath {
    /// `nono.exe` spawning directly (`exec_strategy_windows/launch.rs`).
    /// Covers direct `nono run`, the PTY/no-PTY broker arms' OWN spawn of
    /// `nono-shell-broker.exe` (the broker.exe process itself, not its
    /// grandchild), and the per-tool-call hook path (which re-enters this
    /// same entry point, RESEARCH §B row 4).
    DirectCli,
    /// `nono-shell-broker.exe` spawning the real confined grandchild inside
    /// its own separate `CREATE_SUSPENDED` window
    /// (`nono-shell-broker/src/main.rs:537-661`). This is the process
    /// Blocker-1 (above) exists to correctly attribute layers to.
    Broker,
    /// `nono-agentd.exe`'s daemon-side launch path
    /// (`agent_daemon/launch.rs`), structurally independent of
    /// `exec_strategy_windows/` (RESEARCH §Summary-1).
    Daemon,
}

/// One cell of the D-08 expectancy matrix: is `LayerId` X expected to be
/// active for the (entry_path, token_arm) combination this value names.
///
/// `token_arm` names a `WindowsTokenArm` variant
/// (`launch.rs::select_windows_token_arm`)
/// as a `&'static str` rather than importing that enum directly — a
/// deliberate decoupling choice so `layer_registry.rs` does not create a
/// circular concern with `launch.rs` (which will, in a later plan, import
/// FROM this module to look up per-arm expectancy). `None` means "no
/// `WindowsTokenArm` axis applies to this cell" — used only for
/// `EntryPath::Broker` and `EntryPath::Daemon` cells, both of which bypass
/// `select_windows_token_arm` entirely (see `EntryPath`'s doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArmExpectancy {
    pub entry_path: EntryPath,
    pub token_arm: Option<&'static str>,
    pub expected: bool,
}

/// The four-value D-05 outcome vocabulary, plus the distinct D-06
/// `FailOpenDefect` value. Every `LayerRegistryEntry` below spells this out
/// explicitly (Item-1 fix, checker pass 3) — never left inferred from the
/// row's `probe` kind or call site alone.
///
/// `FailOpen` and `FailOpenDefect` are two DISTINCT values (D-06 warning,
/// CONTEXT.md `<specifics>`): `FailOpen` is a row where the layer is
/// hardening-only and its absence does not widen the confinement claim,
/// justified in writing per instance (D-06). `FailOpenDefect` is the label
/// for a row where the code fails open and *shouldn't* — forcing either an
/// in-phase fix or a downgraded runtime claim plus a named successor
/// (D-13/D-14). No row in this initial 13-row inventory uses
/// `FailOpenDefect` — every enforced layer in this composite is already
/// fail-closed except the deliberate `MinifilterAbsence` carve-out, which
/// is `FailOpen` (justified), not `FailOpenDefect`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContractOutcome {
    /// Fail closed: the launch is refused entirely (`TerminateProcess` the
    /// suspended child + typed `NonoError`, D-22).
    Abort,
    /// The session proceeds but the human-visible claim is downgraded —
    /// D-05's second vocabulary value. No row in this initial inventory
    /// uses this value; reserved for a future row whose absence narrows
    /// (rather than eliminates) the confinement claim without warranting a
    /// full abort.
    ///
    /// # Phase 117 review NR-02: this is NOT the only route to a downgrade
    ///
    /// Because no row carries this outcome (nor `FailOpenDefect`), and the
    /// single `FailOpen` row is deliberately excluded from `downgraded`,
    /// `AttestationDecision::ProceedDowngraded` would be unreachable in a
    /// shipped build if this value were the only path to it — the D-27
    /// banner, the per-session dedup marker and the
    /// `LayerAttestationDowngraded` audit event would all be dead code.
    ///
    /// The live production route is
    /// [`LayerApplication::PartiallyApplied`]: an `Abort`-outcome row whose
    /// apply took effect on some but not all of its contracted targets
    /// proceeds with a downgraded claim rather than aborting or passing as
    /// the full baseline. `attestation.rs`'s
    /// `partially_applied_configured_only_row_proceeds_downgraded` and
    /// `launch.rs`'s
    /// `partially_applied_launch_is_downgraded_not_silently_passed` pin that
    /// route against the real registry.
    DegradeWithVisibleClaim,
    /// The `WindowsTokenArm` cascade (`select_windows_token_arm`) resolves
    /// a DIFFERENT token-construction mechanism that preserves the same
    /// confinement class, decided upfront (before any layer's post-spawn
    /// attestation runs) rather than reactively after a specific layer's
    /// failure. D-07: the alternate mechanism must be named, and the
    /// attestation must confirm the substitute actually took effect. No
    /// row in this initial inventory uses this value (see the doc comment
    /// on the registry population below for why).
    SubstituteEquivalentMechanism { alternate: &'static str },
    /// D-06: the layer is hardening-only; its absence does not widen the
    /// confinement claim. Requires a written justification, per instance.
    /// The ONLY row using this value today is `MinifilterAbsence`.
    FailOpen { justification: &'static str },
    /// D-06/D-13/D-14: the code fails open and should not. Forces an
    /// in-phase fix or a downgraded runtime claim plus a named successor.
    /// No row in this initial inventory uses this value.
    FailOpenDefect { successor: &'static str },
}

/// D-17: how a row's expectancy is (or, per Plan 08/11, will be)
/// established against the real spawned child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbeKind {
    /// A live `GetTokenInformation`/`IsProcessInJob`-shaped query against
    /// the suspended child's own token or job membership — an independent,
    /// third-party observation of kernel state (RESEARCH §C).
    LiveTokenOrJobQuery,
    /// This is Open Question 1's resolution (RESEARCH, resolved 117-08
    /// Task 1): a report *from* the enforcing component itself (the
    /// elevated `nono-wfp-service`, over its named-pipe IPC protocol),
    /// already fail-closed pre-spawn, but not an independent third-party
    /// kernel-state observation the way `LiveTokenOrJobQuery` is.
    ConfirmedByEnforcingComponentReport,
    /// No live re-observation exists; the apply-time `Result` from the
    /// supervisor's own mutating call (DACL grant, `netsh` rule add,
    /// Authenticode compare) IS the confirmation — there is no separate
    /// kernel object to re-query that would tell you anything the apply
    /// call did not already tell you (RESEARCH §C, DACL-grants row).
    ConfiguredOnly,
    /// The check is a pre-flight, build-time-of-the-launch-plan gate (the
    /// interpreter coverage gate) or the row documents a structural
    /// absence (`MinifilterAbsence`) — there is nothing to probe against a
    /// live child either way.
    NotApplicable,
}

/// Phase 117 review CR-09: whether the supervisor actually applied a given
/// layer for THIS launch.
///
/// `ProbeKind::ConfiguredOnly` rows used to classify
/// `EstablishedNotIndependentlyObservable` *unconditionally*, on the
/// reasoning that "reaching the attestation gate at all means the apply-time
/// gate already succeeded fail-closed". That reasoning is only sound for a
/// layer the supervisor genuinely tried to apply on this launch — it was
/// being applied to rows where NO apply ever happened (the daemon's
/// `DaclAncestorReadAttrs`, the never-granted `DaclSessionSidGrant`), which
/// turned the honest label "established" into a false claim, and it made
/// `AttestationDecision::Proceed` unreachable because every launch carried a
/// permanent "downgraded" baseline. The caller now states, per launch, which
/// layers it applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LayerApplication {
    /// The supervisor ran this layer's apply/enforce call for this launch,
    /// it succeeded (its own `Result` was `Ok`, fail-closed), AND it took
    /// effect on every target its contract requires it to cover.
    Applied,
    /// The supervisor did NOT apply this layer on this launch — including
    /// the case where the apply call returned `Ok` after doing nothing at
    /// all (Phase 117 review CR-14: `AppliedLabelsGuard::snapshot_and_apply`
    /// returns `Ok` with an all-skip entry list). For a row that is
    /// `expected: true` on this arm, that is exactly the negative the
    /// fail-direction contract exists to catch.
    ///
    /// This is the `Default` — a caller that forgets to report a layer fails
    /// secure rather than inheriting `Applied` or, worse, `NotApplicable`
    /// (which drops the row from the decision entirely).
    #[default]
    NotApplied,
    /// Phase 117 review CR-14/NR-02: the apply ran and took effect on SOME
    /// but not all of the targets its contract covers (e.g. the mandatory
    /// label was written on two of three policy paths because the third
    /// already carried a third-party label). The layer is genuinely
    /// established in part, so aborting would be wrong — but reporting it as
    /// the full baseline would be the exact "reported enforcing while
    /// partially inert" claim this phase exists to eliminate. This value is
    /// what makes [`ContractOutcome::DegradeWithVisibleClaim`]'s operator
    /// channel (`AttestationDecision::ProceedDowngraded`, the D-27 banner and
    /// audit event) reachable from a production launch.
    PartiallyApplied,
    /// This layer is not part of this launch's composition at all — e.g. a
    /// network row whose backend was not the one selected, or the broker
    /// Authenticode gate on a non-broker arm.
    NotApplicable,
}

/// Per-launch application facts for every `ProbeKind::ConfiguredOnly` /
/// `ConfirmedByEnforcingComponentReport` row, supplied by the caller that
/// owns the guards (CR-09). Rows probed live (`LiveTokenOrJobQuery`) or not
/// probed at all (`ProbeKind::NotApplicable`) never consult this.
///
/// Defaults to `NotApplied` for every `LayerApplication` field, NOT
/// `Applied` — a caller that forgets to report a layer fails secure. (The
/// three `Option<bool>` fields default to `None`/`NotApplicable`; see NR-03,
/// tracked separately.)
///
/// # CR-14: these are EFFECT facts, not construction facts
///
/// Every `LayerApplication` field below must be derived from what the
/// layer's guard actually achieved (`labels_guard::LabelCoverage`,
/// `dacl_guard::DaclGrantCoverage`), never from whether a guard object
/// exists. `AppliedLabelsGuard::snapshot_and_apply` returns `Ok` when it
/// recorded a skip for EVERY path, so "the guard exists" and "the layer is
/// in effect" are genuinely different propositions.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct AppliedLayers {
    pub mandatory_integrity_label: LayerApplication,
    pub dacl_package_sid_grant: LayerApplication,
    pub dacl_ancestor_traverse: LayerApplication,
    pub dacl_ancestor_read_attrs: LayerApplication,
    /// `Some(true)` when the `netsh advfirewall` backend was selected AND
    /// its rules installed; `Some(false)` when it was selected and did not
    /// install; `None` when a different backend (or none) was selected, so
    /// the row is not part of this launch's composition.
    pub firewall_rules_egress: Option<bool>,
    /// The WFP backend's COMPOSITION report — `Some(true)` when it was the
    /// backend this launch selected, `None` when it was not.
    ///
    /// WR-13: this field is **two-valued in practice, not tri-state**, and
    /// the doc used to claim otherwise ("same tri-state shape"). Its producer
    /// `mod.rs::wfp_composition_report` cannot return `Some(false)`, so
    /// `status(WfpEgressFilters)` never yields `LayerApplication::NotApplied`
    /// and `from_tristate`'s `Some(false)` arm is dead for this row. That is
    /// deliberate — see `wfp_composition_report`'s doc for why re-folding the
    /// filter count in here would undo NR-04's split — and the row's deny
    /// direction is carried by `AttestationInput::wfp_preconfirmed`
    /// (`launch.rs::derive_wfp_preconfirmed`), not by this field.
    ///
    /// Advertising a negative the type cannot express is the NR-04/NR-05
    /// defect class, which is why it is spelled out here rather than left to
    /// be rediscovered.
    pub wfp_egress_filters: Option<bool>,
    /// `Some(true)` when the Authenticode comparison ran and passed;
    /// `Some(false)` when it should have run and did not; `None` on a
    /// non-broker arm, or under the compile-time-baked dev-build-layout
    /// detector where the gate is deliberately not part of the composition.
    pub broker_authenticode_trust_gate: Option<bool>,
    // CR-14: there is deliberately NO `interpreter_coverage_gate` field.
    // `LayerId::InterpreterCoverageGate`'s registry row declares
    // `ProbeKind::NotApplicable` (it is a pre-flight, pre-spawn gate with
    // nothing to re-observe against a live child), so `classify_row` returns
    // `NotApplicable` for it WITHOUT ever consulting this struct. The field
    // that used to live here was a hardcoded `true` that no decision could
    // read — a report that looked like a fact and was neither read nor
    // checkable. `every_reported_layer_is_actually_consulted` pins the
    // invariant that a reported field must belong to a row whose probe kind
    // consults it.
}

fn from_tristate(applied: Option<bool>) -> LayerApplication {
    match applied {
        None => LayerApplication::NotApplicable,
        Some(true) => LayerApplication::Applied,
        Some(false) => LayerApplication::NotApplied,
    }
}

impl AppliedLayers {
    /// Exhaustive over every `LayerId` (no wildcard arm — T-117-06 mirror),
    /// so a new variant forces an explicit decision here rather than
    /// silently inheriting a permissive default.
    pub(crate) fn status(&self, id: LayerId) -> LayerApplication {
        match id {
            LayerId::MandatoryIntegrityLabel => self.mandatory_integrity_label,
            LayerId::DaclPackageSidGrant => self.dacl_package_sid_grant,
            LayerId::DaclAncestorTraverse => self.dacl_ancestor_traverse,
            LayerId::DaclAncestorReadAttrs => self.dacl_ancestor_read_attrs,
            LayerId::FirewallRulesEgress => from_tristate(self.firewall_rules_egress),
            LayerId::WfpEgressFilters => from_tristate(self.wfp_egress_filters),
            LayerId::BrokerAuthenticodeTrustGate => {
                from_tristate(self.broker_authenticode_trust_gate)
            }
            // CR-14: pre-flight gate, `ProbeKind::NotApplicable` — never
            // consulted by `classify_row`, so there is nothing honest to
            // report here.
            LayerId::InterpreterCoverageGate
            // CR-05: `DaclSessionSidGrant` is not applied by any shipped
            // call site (see its registry row) — its expectancy is empty, so
            // `classify_row` returns `NotApplicable` before ever reaching
            // here.
            | LayerId::DaclSessionSidGrant
            // Live-probed rows: their status comes from the kernel, not
            // from the caller's own report.
            | LayerId::RestrictedToken
            | LayerId::AppContainerProfile
            | LayerId::JobObjectContainment
            // Structural-absence row (ADR-65).
            | LayerId::MinifilterAbsence => LayerApplication::NotApplicable,
        }
    }
}

/// One row of the Windows fail-direction layer registry.
#[derive(Debug)]
pub(crate) struct LayerRegistryEntry {
    pub id: LayerId,
    pub name: &'static str,
    /// `file.rs::Symbol` citations of the enforcing call site(s),
    /// content-verified against the cited file's real definitions. Layers
    /// present on both the `DirectCli`/`Daemon` entry paths (RESEARCH Open
    /// Question 3, resolved) cite BOTH sites in this one flat list rather
    /// than a structural (entry_path, call_site) pair type.
    pub call_sites: &'static [&'static str],
    pub expectancy: &'static [ArmExpectancy],
    pub outcome: ContractOutcome,
    pub probe: ProbeKind,
}

/// `token_arm` string constants naming `WindowsTokenArm` variants
/// (`launch.rs::select_windows_token_arm`) by name, per `ArmExpectancy`'s decoupling
/// choice above. Kept as named constants (rather than inline literals in
/// every `ArmExpectancy`) so a rename of `WindowsTokenArm` surfaces as a
/// single-place update.
mod token_arm_names {
    pub(super) const NULL: &str = "Null";
    pub(super) const WRITE_RESTRICTED: &str = "WriteRestricted";
    pub(super) const LOW_IL_PRIMARY: &str = "LowIlPrimary";
    pub(super) const BROKER_LAUNCH: &str = "BrokerLaunch";
    pub(super) const BROKER_LAUNCH_NO_PTY: &str = "BrokerLaunchNoPty";
}

/// Expectancy shared by every row whose applicability runs uniformly
/// across all five `DirectCli` arms (`MandatoryIntegrityLabel`,
/// `InterpreterCoverageGate`, `FirewallRulesEgress` — each runs inside
/// `prepare_live_windows_launch`, which executes once before
/// `select_windows_token_arm` is even called, RESEARCH §A).
const fn all_direct_cli_arms_expectancy() -> [ArmExpectancy; 5] {
    [
        ArmExpectancy {
            entry_path: EntryPath::DirectCli,
            token_arm: Some(token_arm_names::NULL),
            expected: true,
        },
        ArmExpectancy {
            entry_path: EntryPath::DirectCli,
            token_arm: Some(token_arm_names::WRITE_RESTRICTED),
            expected: true,
        },
        ArmExpectancy {
            entry_path: EntryPath::DirectCli,
            token_arm: Some(token_arm_names::LOW_IL_PRIMARY),
            expected: true,
        },
        ArmExpectancy {
            entry_path: EntryPath::DirectCli,
            token_arm: Some(token_arm_names::BROKER_LAUNCH),
            expected: true,
        },
        ArmExpectancy {
            entry_path: EntryPath::DirectCli,
            token_arm: Some(token_arm_names::BROKER_LAUNCH_NO_PTY),
            expected: true,
        },
    ]
}

const ALL_DIRECT_CLI_ARMS_EXPECTANCY: [ArmExpectancy; 5] = all_direct_cli_arms_expectancy();

const RESTRICTED_TOKEN_EXPECTANCY: [ArmExpectancy; 1] = [ArmExpectancy {
    entry_path: EntryPath::DirectCli,
    token_arm: Some(token_arm_names::WRITE_RESTRICTED),
    expected: true,
}];

const MANDATORY_INTEGRITY_LABEL_EXPECTANCY: [ArmExpectancy; 6] = [
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::NULL),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::WRITE_RESTRICTED),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::LOW_IL_PRIMARY),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::BROKER_LAUNCH),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::BROKER_LAUNCH_NO_PTY),
        expected: true,
    },
    // The broker applies its own Low-IL mandatory label to the real
    // grandchild's primary token (`nono-shell-broker/src/main.rs:615-644`,
    // between CREATE_SUSPENDED and ResumeThread) — a second, independent
    // application of this same layer, per RESEARCH §D point 2.
    ArmExpectancy {
        entry_path: EntryPath::Broker,
        token_arm: None,
        expected: true,
    },
];

/// Blocker-1 (checker pass 2, see module doc comment above): `expected:
/// true` ONLY at `(EntryPath::Broker, None)` and `(EntryPath::Daemon,
/// None)` — never at any `(EntryPath::DirectCli, ...)` cell, including
/// `Some("BrokerLaunchNoPty")`. `nono-cli`'s own `spawn_windows_child` gate
/// observes `nono-shell-broker.exe` itself on that arm, never the real
/// AppContainer-confined grandchild.
/// Phase 117 review CR-03.1: the `EntryPath::Broker` cell is scoped to the
/// `BrokerLaunchNoPty` shape. `nono-shell-broker` only creates an
/// AppContainer when nono-cli passes `--app-container-name`, which only the
/// `BrokerLaunchNoPty` arm does (`launch.rs`'s broker-arg builder; the
/// broker's own `parse_args` additionally rejects `--no-pty` without it).
/// The legacy/PTY broker shape has no AppContainer at all, so an
/// unqualified `(Broker, None)` cell made `required_layers_for_broker()`
/// demand a layer that structurally cannot exist there — and the broker,
/// which ignored every name but this one, silently never checked it either.
const APP_CONTAINER_PROFILE_EXPECTANCY: [ArmExpectancy; 2] = [
    ArmExpectancy {
        entry_path: EntryPath::Broker,
        token_arm: Some(token_arm_names::BROKER_LAUNCH_NO_PTY),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::Daemon,
        token_arm: None,
        expected: true,
    },
];

/// Phase 117 review CR-05: **empty on purpose — this layer does not exist in
/// the shipped tree.**
///
/// The row previously claimed `expected: true` at `(DirectCli,
/// WriteRestricted)`, citing `dacl_guard.rs` / `mod.rs`'s
/// `AppliedDaclGrantsGuard::snapshot_and_apply` call. That call is the ONLY
/// production construction of the guard, and it is passed
/// `config.package_sid`, not `config.session_sid`. The synthetic per-session
/// restricting SID (`S-1-5-117-*`) is used solely to build the
/// `WRITE_RESTRICTED` token (`create_restricted_token_with_sid`) and is
/// granted on no DACL anywhere. Reporting the row
/// `EstablishedNotIndependentlyObservable` — whose documented meaning is
/// "the apply-time `Result` already happened and succeeded" — was a false
/// claim of an active confinement layer, emitted by the module whose entire
/// purpose is truthful layer accounting.
///
/// An empty expectancy makes `classify_row` return `NotApplicable` on every
/// arm, so the session claim no longer includes a layer that was never
/// applied. The `LayerId` variant is retained (Phase 118 receipts share this
/// vocabulary). Phase 117 gap closure (NR3-03) adds two tests in this file's
/// `#[cfg(test)] mod tests`: `dacl_session_sid_grant_is_not_claimed_anywhere`
/// fails the build if anyone re-adds an expectancy cell here without also
/// making the grant real in `AppliedLayers::status()`, and its converse
/// `every_consulting_row_with_an_expectancy_has_a_reported_field` protects
/// every OTHER `ConfiguredOnly`/`ConfirmedByEnforcingComponentReport` row
/// from the same fail-open shape.
///
/// **Open operator decision, deliberately NOT guessed here:** whether the
/// `WriteRestricted` arm SHOULD also grant `config.session_sid` write on the
/// writable grant set. `mod.rs`'s own `_applied_dacls` comment still
/// describes that as the mechanism ("so confined writes under
/// WRITE_RESTRICTED pass the restricting-SID double check"), and the grantee
/// was changed to the package SID in Plan 62-12. Restoring it would WIDEN
/// DACLs on user-owned paths, so it is a functional/security change for an
/// operator to make deliberately, not a review-fix side effect.
const DACL_SESSION_SID_GRANT_EXPECTANCY: [ArmExpectancy; 0] = [];

/// Phase 117 review WR-09: `execution_runtime.rs` sets
/// `package_sid: Some(..)` **unconditionally** for every Windows launch, so
/// `prepare_live_windows_launch` constructs the package-SID DACL guard, the
/// ancestor-traverse guard and the ancestor-read-attrs guard on EVERY
/// `DirectCli` arm — not just `BrokerLaunchNoPty`. Scoping the expectancy to
/// that one arm made three genuinely-applied layers classify
/// `NotApplicable` on the other four, silently omitting them from the
/// session's attested claim. The expectancy now matches the code.
///
/// Split from the WFP row's expectancy (which they used to share) because
/// CR-04 makes the network rows conditional on the resolved backend, a
/// distinction that does not apply to the DACL rows.
const DACL_PACKAGE_SID_SCOPED_EXPECTANCY: [ArmExpectancy; 6] = [
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::NULL),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::WRITE_RESTRICTED),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::LOW_IL_PRIMARY),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::BROKER_LAUNCH),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::BROKER_LAUNCH_NO_PTY),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::Daemon,
        token_arm: None,
        expected: true,
    },
];

/// Phase 117 review CR-06: the DAEMON never grants `FILE_READ_ATTRIBUTES`.
/// `DaemonDaclGuard::apply` performs exactly three operations —
/// `grant_sid_traverse_on_path` on read-only rules,
/// `grant_sid_write_on_path` on the workspace, and
/// `grant_sid_traverse_on_path` on workspace ancestors. The
/// `grant_sid_read_attributes_on_path` call sites this row cites
/// (`dacl_guard.rs`, `mod.rs`) are CLI-side files the daemon binary does not
/// even link. The `(Daemon, None)` cell is therefore dropped.
const DACL_ANCESTOR_READ_ATTRS_EXPECTANCY: [ArmExpectancy; 5] = ALL_DIRECT_CLI_ARMS_EXPECTANCY;

/// Phase 117 review CR-04: the row used to be `expected: true`
/// **unconditionally** at `(DirectCli, BrokerLaunchNoPty)` while its status
/// came solely from `wfp_preconfirmed`, which is `true` only for
/// `NetworkEnforcementGuard::WfpServiceManaged`. Two legitimate,
/// non-degraded configurations produced `false` — the `FirewallRules`
/// backend and "no network restriction requested" — and `Unconfirmed` on an
/// `Abort` row refused the launch. `BrokerLaunchNoPty` is the primary
/// supervised Windows arm, so that broke legitimate runs.
///
/// The expectancy now covers every arm on which WFP *can* be selected, and
/// applicability is decided by the RESOLVED network backend the caller
/// reports (`AppliedLayers::wfp_egress_filters`): a launch that did not
/// select the WFP backend classifies this row `NotApplicable`, not
/// `Unconfirmed`. The row is not applicable, not degraded.
const WFP_EGRESS_FILTERS_EXPECTANCY: [ArmExpectancy; 6] = DACL_PACKAGE_SID_SCOPED_EXPECTANCY;

/// `netsh advfirewall` block rules are program-path-scoped, not
/// session-scoped, and network enforcement is prepared inside
/// `prepare_live_windows_launch` (`mod.rs:484`) regardless of which
/// `WindowsTokenArm` is ultimately selected — so this row's expectancy
/// mirrors `MandatoryIntegrityLabel`'s "all `DirectCli` arms" shape rather
/// than being package-SID-scoped like the DACL/WFP rows above. Shares
/// `ALL_DIRECT_CLI_ARMS_EXPECTANCY` with `InterpreterCoverageGate`.
const FIREWALL_RULES_EGRESS_EXPECTANCY: [ArmExpectancy; 5] = ALL_DIRECT_CLI_ARMS_EXPECTANCY;

/// `MinifilterAbsence` applies uniformly to every `(entry_path, None)` —
/// the one row with no per-arm variation (RESEARCH §A).
const MINIFILTER_ABSENCE_EXPECTANCY: [ArmExpectancy; 3] = [
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: None,
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::Broker,
        token_arm: None,
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::Daemon,
        token_arm: None,
        expected: true,
    },
];

/// Job Object containment applies to whatever process `nono-cli` directly
/// spawns (any `DirectCli` arm — including `BrokerLaunch`/`BrokerLaunchNoPty`,
/// which job-contains `nono-shell-broker.exe` itself) and to the daemon's
/// own independently-created job (`agent_daemon/launch.rs:959`/`:1049`).
const JOB_OBJECT_CONTAINMENT_EXPECTANCY: [ArmExpectancy; 6] = [
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::NULL),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::WRITE_RESTRICTED),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::LOW_IL_PRIMARY),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::BROKER_LAUNCH),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::BROKER_LAUNCH_NO_PTY),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::Daemon,
        token_arm: None,
        expected: true,
    },
];

/// Scoped to the arms that actually spawn `nono-shell-broker.exe`.
const BROKER_AUTHENTICODE_TRUST_GATE_EXPECTANCY: [ArmExpectancy; 2] = [
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::BROKER_LAUNCH),
        expected: true,
    },
    ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some(token_arm_names::BROKER_LAUNCH_NO_PTY),
        expected: true,
    },
];

/// The 13-row Windows fail-direction layer registry (D-01/D-10). Every
/// `outcome` is spelled out explicitly (Item-1 fix, checker pass 3) —
/// never inferred from `probe` or `call_sites` alone.
const REGISTRY_ENTRIES: [LayerRegistryEntry; 13] = [
    LayerRegistryEntry {
        id: LayerId::RestrictedToken,
        name: "restricted-token",
        call_sites: &[
            "restricted_token.rs::create_restricted_token_with_sid",
            "launch.rs::spawn_windows_child",
        ],
        expectancy: &RESTRICTED_TOKEN_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::LiveTokenOrJobQuery,
    },
    LayerRegistryEntry {
        id: LayerId::MandatoryIntegrityLabel,
        name: "mandatory-integrity-label",
        call_sites: &[
            "labels_guard.rs::AppliedLabelsGuard::snapshot_and_apply",
            "mod.rs::prepare_live_windows_launch",
            "nono-shell-broker/src/main.rs::broker_resume_gate",
        ],
        expectancy: &MANDATORY_INTEGRITY_LABEL_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        // Phase 117 review CR-01: was `LiveTokenOrJobQuery`, dispatched to
        // `probe_integrity_level` against the CHILD TOKEN. Two defects:
        //
        // 1. Wrong object. This row is the mandatory-label ACE that
        //    `AppliedLabelsGuard::snapshot_and_apply` writes onto the
        //    compiled filesystem-policy PATHS (see `call_sites` above). The
        //    child token's own integrity level is a different kernel object
        //    entirely; confirming it would not attest what this row claims.
        // 2. The deny branch was unreachable. `GetTokenInformation(
        //    TokenIntegrityLevel)` succeeds for EVERY Windows token — every
        //    process has a mandatory label, Medium by default — and the
        //    classifier discarded the RID, so a fully unconfined Medium-IL
        //    child classified `Confirmed`. The guard could only say yes.
        //
        // The file-label ACE has no independent post-hoc kernel query worth
        // re-running (re-reading the ACE we just wrote tells us nothing the
        // apply `Result` did not), so `ConfiguredOnly` is the honest probe
        // kind. Per CR-09 the caller now supplies whether the apply actually
        // happened for THIS launch, so `ConfiguredOnly` is no longer a
        // free pass either.
        //
        // A genuine token-integrity-level observation IS performed, but on
        // the arm where it means something and by the binary that can see
        // the right process: `nono-shell-broker` re-reads its own
        // AppContainer grandchild's token RID and requires it to be
        // <= SECURITY_MANDATORY_LOW_RID before resuming it (CR-03).
        //
        // ⚠ WR-10 OPEN (NOT FIXED — grep `WR-10 OPEN`; recorded in the SPEC's
        // D-15 "Contract vs. code discrepancies" ledger): that broker
        // observation is filed under THIS `LayerId`, so one name covers two
        // different kernel objects. On `(DirectCli, ..)` the row means the
        // mandatory-label ACE on the compiled filesystem-policy paths (what
        // this row's own doc defines, and what `ConfiguredOnly` describes);
        // on `(Broker, None)` it means the grandchild's TOKEN integrity RID,
        // which says nothing about whether any file carries the contracted
        // ACE. RF-01 removed exactly this substitution from the CLI core on
        // the grounds that it "would not attest what this row claims", and
        // the broker arm reinstated it under the same name. So
        // `required_layers_for_broker` and `BROKER_ATTESTABLE_LAYERS` agree
        // on a NAME while disagreeing on the CLAIM, and a broker-arm
        // `MandatoryIntegrityLabel: Confirmed` does not mean what this row
        // says it means.
        //
        // The clean fix is a split (`LayerId::ChildTokenIntegrityLevel` with
        // its own `(Broker, None)` expectancy and `LiveTokenOrJobQuery`
        // probe; drop this row's `(Broker, None)` cell). Deferred because
        // `NONO_BROKER_REQUIRED_LAYERS` is a cross-binary wire contract and
        // the broker refuses to resume on any name it does not recognise
        // (RF-02) — renaming it is a lockstep two-binary change, not a
        // registry edit, and a mixed-version pair fails closed (no launch).
        probe: ProbeKind::ConfiguredOnly,
    },
    LayerRegistryEntry {
        id: LayerId::AppContainerProfile,
        // expected: true at (Broker, None) and (Daemon, None); attested
        // locally inside nono-shell-broker (Plan 11) via the
        // NONO_BROKER_REQUIRED_LAYERS env-var contract Plan 08's
        // required_layers_for_broker() derives from this row — never
        // dispatched through attest_and_decide, since that function only
        // runs at gate sites inside the nono-cli crate (DirectCli, Daemon).
        // outcome: Abort — see Task 3's broker-abort-only invariant test,
        // which enforces this mechanically.
        name: "app-container-profile",
        call_sites: &[
            "nono-shell-broker/src/main.rs::run",
            "nono-shell-broker/src/main.rs::broker_resume_gate",
            "agent_daemon/launch.rs::launch_agent",
        ],
        expectancy: &APP_CONTAINER_PROFILE_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::LiveTokenOrJobQuery,
    },
    LayerRegistryEntry {
        id: LayerId::DaclSessionSidGrant,
        name: "dacl-session-sid-grant",
        // Phase 117 review CR-05: no call site. The previously cited
        // `dacl_guard.rs`/`mod.rs` sites construct `AppliedDaclGrantsGuard`
        // with `config.package_sid` — that is `DaclPackageSidGrant`'s call
        // site, not this row's. Citing them here was the citation half of
        // the same false claim the emptied expectancy fixes.
        call_sites: &[],
        expectancy: &DACL_SESSION_SID_GRANT_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::ConfiguredOnly,
    },
    LayerRegistryEntry {
        id: LayerId::DaclPackageSidGrant,
        name: "dacl-package-sid-grant",
        call_sites: &[
            "dacl_guard.rs::AppliedDaclGrantsGuard::snapshot_and_apply",
            "mod.rs::prepare_live_windows_launch",
            "agent_daemon/launch.rs::DaemonDaclGuard::apply",
            "agent_daemon/launch.rs::launch_agent",
        ],
        expectancy: &DACL_PACKAGE_SID_SCOPED_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::ConfiguredOnly,
    },
    LayerRegistryEntry {
        id: LayerId::DaclAncestorTraverse,
        name: "dacl-ancestor-traverse",
        call_sites: &[
            "dacl_guard.rs::AppliedAncestorTraverseGuard::snapshot_and_apply",
            "mod.rs::prepare_live_windows_launch",
        ],
        expectancy: &DACL_PACKAGE_SID_SCOPED_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::ConfiguredOnly,
    },
    LayerRegistryEntry {
        id: LayerId::DaclAncestorReadAttrs,
        name: "dacl-ancestor-read-attrs",
        call_sites: &[
            "dacl_guard.rs::AppliedAncestorReadAttributesGuard::snapshot_and_apply_targets",
            "mod.rs::prepare_live_windows_launch",
        ],
        expectancy: &DACL_ANCESTOR_READ_ATTRS_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::ConfiguredOnly,
    },
    LayerRegistryEntry {
        id: LayerId::WfpEgressFilters,
        name: "wfp-egress-filters",
        call_sites: &[
            "network.rs::WfpNetworkBackend::install",
            "network.rs::assert_wfp_activation_installed_filters",
            "agent_daemon/launch.rs::wfp_filter_add",
        ],
        expectancy: &WFP_EGRESS_FILTERS_EXPECTANCY,
        // Open Question 1 (RESEARCH), resolved 117-08 Task 1: the
        // pre-spawn `installed_filter_count` IPC check already fails
        // closed, so the code's own behavior IS Abort — this matches the
        // code rather than the "normally downgrades" framing CONTEXT.md's
        // D-18 discussion assumed (see network.rs:1698-1800).
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::ConfirmedByEnforcingComponentReport,
    },
    LayerRegistryEntry {
        id: LayerId::FirewallRulesEgress,
        name: "firewall-rules-egress",
        // SC4-3 resolution (Task 1, above): reachable in production,
        // fails closed via run_netsh_firewall Err propagation with
        // partial-rule rollback.
        call_sites: &[
            "network.rs::FirewallRulesNetworkBackend::install",
            "network.rs::run_netsh_firewall",
        ],
        expectancy: &FIREWALL_RULES_EGRESS_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::ConfiguredOnly,
    },
    LayerRegistryEntry {
        id: LayerId::MinifilterAbsence,
        name: "minifilter-absence",
        // No call site: this layer does not exist in this tree (ADR-65 /
        // D-33 — the standing No-go/Conditional-go verdict). The row
        // documents structural absence, not a probeable mechanism.
        call_sites: &[],
        expectancy: &MINIFILTER_ABSENCE_EXPECTANCY,
        outcome: ContractOutcome::FailOpen {
            justification: "ADR-65: no minifilter exists; per-file read policy inside one directory is explicitly not claimed",
        },
        probe: ProbeKind::NotApplicable,
    },
    LayerRegistryEntry {
        id: LayerId::JobObjectContainment,
        name: "job-object-containment",
        call_sites: &[
            "launch.rs::create_process_containment",
            "launch.rs::apply_process_handle_to_containment",
            "agent_daemon/launch.rs::create_agent_job",
            "agent_daemon/launch.rs::assign_process_to_agent_job",
        ],
        expectancy: &JOB_OBJECT_CONTAINMENT_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::LiveTokenOrJobQuery,
    },
    LayerRegistryEntry {
        id: LayerId::BrokerAuthenticodeTrustGate,
        name: "broker-authenticode-trust-gate",
        call_sites: &["launch.rs::verify_broker_authenticode", "launch.rs::spawn_windows_child"],
        expectancy: &BROKER_AUTHENTICODE_TRUST_GATE_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        probe: ProbeKind::ConfiguredOnly,
    },
    LayerRegistryEntry {
        id: LayerId::InterpreterCoverageGate,
        name: "interpreter-coverage-gate",
        call_sites: &[
            "crates/nono/src/sandbox/windows.rs::validate_launch_paths",
            "crates/nono/src/sandbox/mod.rs::Sandbox::validate_windows_launch_paths",
            "mod.rs::prepare_live_windows_launch",
        ],
        expectancy: &ALL_DIRECT_CLI_ARMS_EXPECTANCY,
        outcome: ContractOutcome::Abort,
        // Pre-flight, build-time-of-the-launch-plan check — not a D-21
        // post-spawn attestation against a live child (RESEARCH §A).
        probe: ProbeKind::NotApplicable,
    },
];

/// Every `LayerId` variant, in declaration order. D-01/D-11: platform-
/// neutral (no `#[cfg]`) so drift tests and Phase 118's receipt type can
/// iterate it on any host. Mirrors the `NetworkAuditDenialCategory::ALL` +
/// `assert_all_variants_covered` idiom (`crates/nono/src/undo/types.rs:333-345`,
/// `:363-377`) exactly: this is the mechanism that fails the build when a
/// `LayerId` variant is added without a corresponding `ALL` entry and match
/// arm below.
pub(crate) const ALL: &[LayerId] = &[
    LayerId::RestrictedToken,
    LayerId::MandatoryIntegrityLabel,
    LayerId::AppContainerProfile,
    LayerId::DaclSessionSidGrant,
    LayerId::DaclPackageSidGrant,
    LayerId::DaclAncestorTraverse,
    LayerId::DaclAncestorReadAttrs,
    LayerId::WfpEgressFilters,
    LayerId::FirewallRulesEgress,
    LayerId::MinifilterAbsence,
    LayerId::JobObjectContainment,
    LayerId::BrokerAuthenticodeTrustGate,
    LayerId::InterpreterCoverageGate,
];

// IMPORTANT: match is exhaustive (no wildcard arm) so the compiler forces
// handling of every current and future LayerId variant — mirrors
// `undo/types.rs`'s `assert_all_variants_covered` guard style exactly.
// Adding a variant to `LayerId` above without adding a corresponding arm
// here fails to compile with `error[E0004]: non-exhaustive patterns:
// LayerId::<Variant> not covered`. This is `ALL`'s own compile-time drift
// guard: forgetting to keep this match (and `ALL`, alongside it) in sync
// with the enum is caught at build time, not left for a runtime test to
// discover — T-117-06's mitigation.
#[cfg_attr(not(test), allow(dead_code))]
fn assert_all_layer_ids_covered(id: &LayerId) {
    match id {
        LayerId::RestrictedToken
        | LayerId::MandatoryIntegrityLabel
        | LayerId::AppContainerProfile
        | LayerId::DaclSessionSidGrant
        | LayerId::DaclPackageSidGrant
        | LayerId::DaclAncestorTraverse
        | LayerId::DaclAncestorReadAttrs
        | LayerId::WfpEgressFilters
        | LayerId::FirewallRulesEgress
        | LayerId::MinifilterAbsence
        | LayerId::JobObjectContainment
        | LayerId::BrokerAuthenticodeTrustGate
        | LayerId::InterpreterCoverageGate => {}
    }
}

/// The populated Windows layer registry (D-11: only the population, not the
/// types above, is platform-gated).
#[cfg(target_os = "windows")]
pub(crate) fn all_entries() -> &'static [LayerRegistryEntry] {
    &REGISTRY_ENTRIES
}

/// Non-Windows stub: matches the `read_machine_egress_policy` split
/// (`crates/nono/src/machine_policy.rs:150-183`, `:696-705`) this plan's
/// `<interfaces>` block cites as the pattern to copy for any Windows-only
/// *population* function this file adds.
#[cfg(not(target_os = "windows"))]
pub(crate) fn all_entries() -> &'static [LayerRegistryEntry] {
    &[]
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    /// Phase 117 review CR-14, structural non-vacuity guard: a layer whose
    /// application `AppliedLayers` *reports* must belong to a row whose
    /// `ProbeKind` actually *consults* that report. Otherwise the field is a
    /// value that looks like a fact and can never influence a decision —
    /// which is exactly what `interpreter_coverage_gate: true` was (its row's
    /// probe is `ProbeKind::NotApplicable`, so `classify_row` returns before
    /// ever calling `status()`).
    ///
    /// D-32 discovery rule: iterates `ALL` and `all_entries()`; it names no
    /// specific `LayerId`, so re-adding a reported field for a
    /// non-consulting row fails this test without the test being touched.
    #[test]
    fn every_reported_layer_is_actually_consulted() {
        // Everything reported as positively as the type allows.
        let everything = AppliedLayers {
            mandatory_integrity_label: LayerApplication::Applied,
            dacl_package_sid_grant: LayerApplication::Applied,
            dacl_ancestor_traverse: LayerApplication::Applied,
            dacl_ancestor_read_attrs: LayerApplication::Applied,
            firewall_rules_egress: Some(true),
            wfp_egress_filters: Some(true),
            broker_authenticode_trust_gate: Some(true),
        };

        for id in ALL {
            if everything.status(*id) == LayerApplication::NotApplicable {
                continue;
            }
            let entry = all_entries()
                .iter()
                .find(|e| e.id == *id)
                .expect("every LayerId has a registry row");
            assert!(
                matches!(
                    entry.probe,
                    ProbeKind::ConfiguredOnly | ProbeKind::ConfirmedByEnforcingComponentReport
                ),
                "{id:?} is reported by AppliedLayers but its row's probe is {:?}, which \
                 classify_row resolves without ever consulting the report — the reported \
                 value can never influence a decision (CR-14)",
                entry.probe
            );
        }
    }

    /// Phase 117 gap closure (NR3-03): the canary the
    /// `DACL_SESSION_SID_GRANT_EXPECTANCY` doc comment promises.
    /// `DaclSessionSidGrant`'s row is hardcoded `NotApplicable` in
    /// `AppliedLayers::status()` (CR-05) — if its expectancy is ever widened
    /// without also making the grant real there, the row would silently
    /// drop out of every decision while still being counted as expected, a
    /// fail-open in the module whose entire purpose is truthful layer
    /// accounting. RF-03's operator decision (whether `WriteRestricted`
    /// should also grant `config.session_sid`) stays open — this test does
    /// not resolve it, it makes resolving it loud instead of silent.
    #[test]
    fn dacl_session_sid_grant_is_not_claimed_anywhere() {
        let entry = all_entries()
            .iter()
            .find(|e| e.id == LayerId::DaclSessionSidGrant)
            .expect("DaclSessionSidGrant has a registry row");
        assert!(
            entry.expectancy.is_empty(),
            "DaclSessionSidGrant's expectancy is no longer empty, but \
             AppliedLayers::status() still hardcodes NotApplicable for it (CR-05) — \
             the row would silently drop out of every decision while still being \
             counted as expected. Either revert the expectancy, or make status() \
             report a real field for this row (RF-03's open operator decision) \
             before widening the expectancy."
        );
    }

    /// Phase 117 gap closure (NR3-03), converse of
    /// `dacl_session_sid_grant_is_not_claimed_anywhere`: protects every OTHER
    /// row whose probe kind actually consults `AppliedLayers` from silently
    /// becoming unattestable the same way `DaclSessionSidGrant` was before
    /// CR-05. Discovery-based (D-32/Phase 115 V-01): iterates
    /// `all_entries()`, names no specific `LayerId`, so a future row added
    /// with this shape fails here without the test being touched.
    #[test]
    fn every_consulting_row_with_an_expectancy_has_a_reported_field() {
        // Every field maximally positive — if a row still reports
        // `NotApplicable` here, no possible caller report could ever make it
        // attestable.
        let all_positive = AppliedLayers {
            mandatory_integrity_label: LayerApplication::Applied,
            dacl_package_sid_grant: LayerApplication::Applied,
            dacl_ancestor_traverse: LayerApplication::Applied,
            dacl_ancestor_read_attrs: LayerApplication::Applied,
            firewall_rules_egress: Some(true),
            wfp_egress_filters: Some(true),
            broker_authenticode_trust_gate: Some(true),
        };

        for entry in all_entries() {
            let consults_report = matches!(
                entry.probe,
                ProbeKind::ConfiguredOnly | ProbeKind::ConfirmedByEnforcingComponentReport
            );
            if !consults_report {
                continue;
            }
            let has_expected_cell = entry.expectancy.iter().any(|arm| arm.expected);
            if !has_expected_cell {
                continue;
            }
            assert_ne!(
                all_positive.status(entry.id),
                LayerApplication::NotApplicable,
                "{:?}'s probe ({:?}) consults AppliedLayers and its expectancy has an \
                 `expected: true` cell, but AppliedLayers::status() reports \
                 NotApplicable even when every field is maximally positive — this row \
                 would silently drop out of every decision no matter what actually \
                 happened at launch (the exact fail-open DaclSessionSidGrant had \
                 before CR-05/NR3-03)",
                entry.id,
                entry.probe
            );
        }
    }

    /// Phase 117 review CR-14: the fail-secure default. A caller that
    /// forgets a layer must land on `NotApplied` (which classifies
    /// `Unconfirmed` on an expected row and aborts), NEVER `Applied` and
    /// never `NotApplicable` (which would drop the row from the decision).
    #[test]
    fn unreported_layer_application_defaults_to_not_applied() {
        assert_eq!(LayerApplication::default(), LayerApplication::NotApplied);
        let none_reported = AppliedLayers::default();
        for id in [
            LayerId::MandatoryIntegrityLabel,
            LayerId::DaclPackageSidGrant,
            LayerId::DaclAncestorTraverse,
            LayerId::DaclAncestorReadAttrs,
        ] {
            assert_eq!(
                none_reported.status(id),
                LayerApplication::NotApplied,
                "{id:?} must fail secure when unreported"
            );
        }
    }

    /// Item-1 fix (checker pass 3): every `EntryPath::Broker`-expected row
    /// must carry `outcome: ContractOutcome::Abort`. Blocker-1's fix means
    /// `required_layers_for_broker()` (Plan 08) filters registry rows to
    /// those with an `ArmExpectancy` matching `(EntryPath::Broker, expected:
    /// true)` AND `outcome: ContractOutcome::Abort` — a row expected at
    /// `EntryPath::Broker` with a NON-`Abort` outcome would be silently
    /// excluded from that filter AND already excluded from
    /// `attest_and_decide`'s own dispatch (which never runs with
    /// `EntryPath::Broker`), so it would go completely unattested on the
    /// broker arm — the exact green-by-absence gap Blocker-1 closed,
    /// re-armed for the next added row.
    ///
    /// D-32's discovery rule (Phase 115 V-01: "a test that names its
    /// targets is blind by construction"): this test iterates
    /// `all_entries()` and filters on `expectancy`/`probe` — it does NOT
    /// hardcode `LayerId::AppContainerProfile` or any other specific
    /// variant name. A future row added with `EntryPath::Broker` expectancy
    /// and a non-`Abort` outcome must fail this test without the test
    /// itself being touched.
    ///
    /// **Deviation (Rule 1, discovered running this test live):** the
    /// invariant is scoped to rows with an attestable `probe` (anything
    /// other than `ProbeKind::NotApplicable`). `MinifilterAbsence` is
    /// `expected: true` at every `(entry_path, None)` cell including
    /// `EntryPath::Broker` (Task 2, RESEARCH §A: "applies to every
    /// (entry_path, None) uniformly") with the deliberate, ADR-65-justified
    /// `FailOpen` outcome — running the unscoped invariant against it
    /// produces a false positive: `MinifilterAbsence` is not a layer that
    /// silently goes unattested on the broker arm, it is a row that
    /// documents a layer which does not exist and therefore has nothing to
    /// attest, on ANY arm (`probe: ProbeKind::NotApplicable`). The
    /// green-by-absence failure mode Blocker-1/Item-1 close is specifically
    /// "an attestable layer expected at the broker arm silently excluded
    /// from `attest_and_decide`'s dispatch" — that failure mode requires
    /// something attestable to begin with. Scoping on `probe` (a field
    /// every row already carries, not a name) preserves D-32's discovery
    /// requirement: a future row is exempted only if IT ALSO documents
    /// `ProbeKind::NotApplicable`, which is itself a deliberate, reviewable
    /// choice recorded on that row, not a name-based carve-out.
    #[test]
    fn broker_expected_rows_are_abort_only() {
        for entry in all_entries() {
            if entry.probe == ProbeKind::NotApplicable {
                continue;
            }
            let expected_at_broker = entry.expectancy.iter().any(|arm| {
                matches!(
                    arm,
                    ArmExpectancy {
                        entry_path: EntryPath::Broker,
                        expected: true,
                        ..
                    }
                )
            });
            if expected_at_broker {
                assert_eq!(
                    entry.outcome,
                    ContractOutcome::Abort,
                    "{:?} is expected at EntryPath::Broker but its outcome is {:?}, not Abort — \
                     a non-Abort broker-expected row is silently excluded from both \
                     attest_and_decide (Blocker-1) and required_layers_for_broker (Plan 08), and \
                     goes completely unattested on the broker arm",
                    entry.id,
                    entry.outcome
                );
            }
        }
    }

    /// Phase 117 review CR-02: every registry row expected at
    /// `(EntryPath::Daemon, expected: true)` with an attestable probe must be
    /// NAMED by `agent_daemon/launch.rs`'s hand-written attestation mirror.
    ///
    /// This is the daemon analog of `broker_expected_rows_are_abort_only`,
    /// which exists because a broker-expected row silently excluded from a
    /// filter "would go completely unattested on the broker arm — the exact
    /// green-by-absence failure mode this phase exists to close, re-armed for
    /// the next added row." That same hole was open one entry path over: a
    /// 14th `LayerId` declared `expected: true` at `(Daemon, None)` compiles,
    /// satisfies `all_entries_covers_every_layer_id`,
    /// `every_registry_row_has_a_test` (via a `MANUALLY_VERIFIED`/
    /// `ALSO_AUTOMATED` entry) and `spec_matches_registry` (name-only), and
    /// goes completely unattested on `nono agent launch`.
    ///
    /// # Why source text rather than a call
    ///
    /// `nono-agentd` is a separate binary that `#[path]`-includes only
    /// `agent_daemon/`, `telemetry/` and `agent_daemon/telemetry_init.rs`; it
    /// never declares `exec_strategy_windows`, so it CANNOT link this
    /// registry. `daemon_attest_and_decide` is therefore a hand-written
    /// mirror, and until this test nothing at all related the two.
    ///
    /// Names no `LayerId` (D-32 discovery rule): a future `(Daemon, ..)` row
    /// fails this without the test being touched.
    ///
    /// # SCOPE — what this deliberately does NOT prove
    ///
    /// That the daemon's decision for a named row MATCHES that row's declared
    /// `outcome`. It cannot, without the daemon consuming the registry.
    /// `DaclAncestorTraverse` is a live divergence: the row declares
    /// `ContractOutcome::Abort` at `(Daemon, None)` and the daemon
    /// deliberately warns and proceeds (WR-06: an absent ancestor traverse
    /// under-grants reach, it never widens confinement). That divergence is
    /// recorded in the SPEC's "Contract vs. code discrepancies" ledger per
    /// D-15. Whether the daemon arm should consume the registry at all — and
    /// so whether `(Daemon, ..)` cells should drive decisions rather than
    /// document them — is an open operator decision, NOT something this test
    /// pretends to have settled. Read this gate as "no daemon-expected row is
    /// unmentioned", not as "the registry drives the daemon".
    #[test]
    fn daemon_expected_rows_are_all_named_by_the_daemon_gate() {
        const DAEMON_GATE_SRC: &str = include_str!("../agent_daemon/launch.rs");

        /// Production lines of the daemon source, plus the number of lines
        /// dropped as `#[cfg(test)]` module bodies.
        ///
        /// BOTH exclusions are load-bearing, and both were established by
        /// perturbation rather than by inspection:
        ///
        /// - Comments: `daemon_attest_and_decide`'s doc comment lists every
        ///   row it models, so scanning comments makes this gate satisfiable
        ///   by prose alone — the "documented, not wired" shape CR-02 reports.
        /// - Test modules: the first version of this gate excluded comments
        ///   only, and renaming the production `layer = "DaclAncestorTraverse"`
        ///   left it GREEN, because `agent_daemon/launch.rs`'s own
        ///   `attestation_gate_tests` asserts on that same string. A gate that
        ///   a test assertion can satisfy is the defect one step over.
        fn production_lines(src: &str) -> (Vec<&str>, usize) {
            let lines: Vec<&str> = src.lines().collect();
            let mut out = Vec::new();
            let mut skipped = 0usize;
            let mut idx = 0usize;
            let mut pending_test_attr = false;
            while idx < lines.len() {
                let t = lines[idx].trim();

                if t.starts_with("#[cfg(") && (t.contains("test)") || t.contains("test,")) {
                    pending_test_attr = true;
                    idx += 1;
                    continue;
                }
                if pending_test_attr {
                    // Other attributes and doc comments may sit between the
                    // cfg attribute and the item it gates.
                    if t.starts_with("#[") || t.starts_with("///") {
                        idx += 1;
                        continue;
                    }
                    pending_test_attr = false;
                    // An INLINE `mod foo {` opens a test region (possibly
                    // nested inside `mod windows_impl`, hence the indent-aware
                    // closer); a bare `mod foo;` declaration opens nothing.
                    if (t.starts_with("mod ") || t.starts_with("pub mod ")) && t.ends_with('{') {
                        let indent = lines[idx].len() - lines[idx].trim_start().len();
                        let closer = format!("{}}}", " ".repeat(indent));
                        idx += 1;
                        while idx < lines.len() && lines[idx] != closer {
                            idx += 1;
                            skipped += 1;
                        }
                        idx += 1;
                        continue;
                    }
                }

                if t.starts_with("//") {
                    idx += 1;
                    continue;
                }
                out.push(lines[idx]);
                idx += 1;
            }
            (out, skipped)
        }

        let (code, skipped_test_lines) = production_lines(DAEMON_GATE_SRC);
        assert!(
            skipped_test_lines > 0,
            "CR-02 non-vacuity: no `#[cfg(test)]` module body was excluded from the daemon \
             source scan, so a test assertion naming a LayerId would satisfy this gate \
             without any production arm existing. Either the daemon file lost its test \
             modules, or `production_lines` no longer recognises them."
        );

        let mut checked = 0usize;
        for entry in all_entries() {
            if entry.probe == ProbeKind::NotApplicable {
                continue;
            }
            let expected_at_daemon = entry.expectancy.iter().any(|arm| {
                matches!(
                    arm,
                    ArmExpectancy {
                        entry_path: EntryPath::Daemon,
                        expected: true,
                        ..
                    }
                )
            });
            if !expected_at_daemon {
                continue;
            }
            checked += 1;
            let quoted = format!("\"{:?}\"", entry.id);
            assert!(
                code.iter().any(|l| l.contains(&quoted)),
                "{:?} is expected at (EntryPath::Daemon, None) with an attestable probe \
                 ({:?}), but daemon_attest_and_decide's source never names {quoted} outside a \
                 comment — the row goes completely unattested on `nono agent launch` (CR-02). \
                 The daemon cannot link this registry, so a hand-written arm naming the layer \
                 is the binding; add one, or drop the row's (Daemon, None) cell.",
                entry.id,
                entry.probe
            );
        }

        assert!(
            checked >= 5,
            "CR-02 non-vacuity: only {checked} daemon-expected row(s) were checked. The \
             registry declares AppContainerProfile, JobObjectContainment, WfpEgressFilters, \
             DaclPackageSidGrant and DaclAncestorTraverse at (Daemon, None); a lower count \
             means the expectancies were narrowed and this gate now covers almost nothing."
        );
    }

    /// Phase 117 review WR-08: `ProbeKind::ConfirmedByEnforcingComponentReport`
    /// has exactly ONE user, and adding a second must be a build failure.
    ///
    /// `classify_row`'s arm for this probe kind reads
    /// `AttestationInput::wfp_preconfirmed` — a WFP-specific fact. It used to
    /// do so for ANY row carrying this probe kind, keyed on nothing but the
    /// probe kind, so a second row (the kind's own doc invites one: "a report
    /// *from* the enforcing component itself") would have been confirmed or
    /// denied by evidence about a different mechanism entirely — the same
    /// wrong-object defect RF-01 removed from `MandatoryIntegrityLabel`.
    ///
    /// `classify_row` now keys the evidence by `LayerId` and fails CLOSED for
    /// any other row. This test is the loud half: it makes the coupling
    /// visible at the moment a second row is added, rather than letting that
    /// row silently classify `Unconfirmed` forever with no explanation.
    #[test]
    fn exactly_one_row_is_confirmed_by_enforcing_component_report() {
        let rows: Vec<LayerId> = all_entries()
            .iter()
            .filter(|e| e.probe == ProbeKind::ConfirmedByEnforcingComponentReport)
            .map(|e| e.id)
            .collect();

        assert_eq!(
            rows,
            vec![LayerId::WfpEgressFilters],
            "ProbeKind::ConfirmedByEnforcingComponentReport is consumed in \
             attestation.rs::classify_row by an arm whose evidence \
             (`AttestationInput::wfp_preconfirmed`) is WFP-SPECIFIC. Rows carrying this probe \
             kind today: {rows:?}. If you are adding a second one, give it its own evidence \
             channel on AttestationInput and its own arm in classify_row's `match entry.id` \
             FIRST — until you do, it classifies Unconfirmed (fail-closed) and will refuse \
             every launch. Do not widen the WFP arm to cover it (WR-08)."
        );
    }

    /// Sanity check that the registry actually has broker-expected rows to
    /// exercise the invariant above (a vacuously-true loop over zero
    /// matching rows would be a weaker test than it appears).
    #[test]
    fn at_least_one_row_is_broker_expected() {
        let count = all_entries()
            .iter()
            .filter(|entry| {
                entry.expectancy.iter().any(|arm| {
                    matches!(
                        arm,
                        ArmExpectancy {
                            entry_path: EntryPath::Broker,
                            expected: true,
                            ..
                        }
                    )
                })
            })
            .count();
        assert!(
            count > 0,
            "expected at least one EntryPath::Broker-expected row (e.g. AppContainerProfile, \
             MandatoryIntegrityLabel, MinifilterAbsence) to exercise \
             broker_expected_rows_are_abort_only"
        );
    }

    #[test]
    fn all_entries_covers_every_layer_id() {
        let covered: Vec<LayerId> = all_entries().iter().map(|e| e.id).collect();
        for id in ALL {
            assert!(
                covered.contains(id),
                "{id:?} is in LayerId::ALL but has no corresponding row in all_entries()"
            );
        }
        assert_eq!(
            covered.len(),
            ALL.len(),
            "all_entries() row count must match LayerId::ALL length"
        );
    }

    /// WR-05: assert the convention the rows actually use, and be
    /// non-vacuous about it.
    ///
    /// This was `every_call_site_string_names_a_line_number`, asserting
    /// `site.contains(".rs:")`. NR3-08 converted every citation to
    /// `"file.rs::Symbol"` form, which satisfies `.rs:` — so a test named
    /// "names a line number" verified only that the substring `.rs:` existed,
    /// and verified nothing at all for the two rows with `call_sites: &[]`
    /// (`DaclSessionSidGrant`, `MinifilterAbsence`). It provided false
    /// reassurance next to
    /// `layer_registry_selfcheck.rs::registry_call_sites_exist`, which is the
    /// real content check (it resolves each symbol in the cited file). This
    /// one is kept, renamed, and narrowed to the one thing it can check
    /// in-crate that the integration test does not: the citation SHAPE.
    #[test]
    fn every_call_site_string_is_symbol_form() {
        let mut checked = 0usize;
        for entry in all_entries() {
            for site in entry.call_sites {
                assert!(
                    site.contains(".rs::"),
                    "{:?}'s call_sites entry {site:?} is not \"file.rs::Symbol\" form. Raw \
                     \"file.rs:line\" citations drift silently (NR3-08: they drifted ~145 \
                     lines across one fix pass, and 15 of 15 sampled pointed at unrelated \
                     code); the symbol form is line-drift-immune and is what \
                     registry_call_sites_exist resolves by file CONTENT.",
                    entry.id
                );
                checked += 1;
            }
        }
        assert!(
            checked >= 20,
            "WR-05: only {checked} call site(s) checked — the loop went vacuous. Two rows \
             legitimately carry `call_sites: &[]` (DaclSessionSidGrant, MinifilterAbsence); a \
             count this low means the other rows lost their citations, which is the drift \
             this test exists to catch."
        );
    }
}
