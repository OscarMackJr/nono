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
