# Windows Fail-Direction Contract

**Status:** Living specification (CINT-01, Phase 117). This document describes shipped
behaviour; it is not a decision record. Genuine decisions that produced this document's
content live in `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-CONTEXT.md`.

**Source of truth:** D-01 — the code-resident registry at
`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` (`LayerId`, `ALL`,
`all_entries()`) is authoritative. This document is drift-checked against that registry by
`crates/nono-cli/tests/layer_registry_selfcheck.rs::spec_matches_registry`, which fails the
build the moment a `LayerId` variant exists in the registry without a corresponding row here.
If this document and the registry ever disagree, the registry wins and the disagreement is a
bug in this document, not in the registry.

**Consumers:** Phase 118's per-session enforcement receipts (D-04, the shared layer-identity
type) and Phase 119's security-model boundary statement (D-12's Unix non-coverage note, handed
over below).

## Scope

**D-09: Windows-only.** This contract enumerates only the layers the Windows backend
(`crates/nono-cli/src/exec_strategy_windows/`, `crates/nono-shell-broker/`,
`crates/nono-cli/src/agent_daemon/`) composes into its deny-by-composition confinement model.
It is not a general nono confinement contract — see "Windows layers only" below for what this
deliberately excludes and why.

## Startup-only claim (D-20)

**This contract, and the self-attestation pass CINT-02 builds on top of it, describe a single
point in time: process launch.** The attestation gate runs once, against the real spawned
child while it is still suspended (`CREATE_SUSPENDED`), before `ResumeThread` — see D-21. It
never re-runs during the life of the session.

**Consequence stated plainly, per D-20:** mid-session layer removal is outside this claim. If
an external actor tears down the WFP filters for a running session's package SID after launch,
or otherwise revokes a grant this contract describes as "confirmed" at startup, that
degradation is not detected, not reported, and not part of what this document or the
attestation pass promises. A confirmed claim describes the state observed at
`CREATE_SUSPENDED`, not a continuous guarantee. Continuous / periodic re-attestation is
explicitly out of scope for this phase (see CONTEXT.md's Deferred Ideas).

## Layer registry

One row per `LayerId` variant (13 total), transcribed directly from
`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs`. Each row's "Expected on" column
summarizes the `ArmExpectancy` matrix (D-08); see the registry source for the exact per-arm
cells. The four-value D-05 outcome vocabulary plus D-06's `FailOpen` is used verbatim from
`ContractOutcome`. `ProbeKind` values match D-17/D-18's attestation vocabulary.

| `LayerId` | Name | Enforcing call site(s) | Expected on | Outcome | Probe |
|---|---|---|---|---|---|
| `RestrictedToken` | restricted-token | `restricted_token.rs:55`, `launch.rs:1403` | `DirectCli` / `WriteRestricted` arm only | `Abort` | `LiveTokenOrJobQuery` |
| `MandatoryIntegrityLabel` | mandatory-integrity-label | `labels_guard.rs:83`, `mod.rs:425`, `nono-shell-broker/src/main.rs:615-644` | All 5 `DirectCli` arms (`Null`/`WriteRestricted`/`LowIlPrimary`/`BrokerLaunch`/`BrokerLaunchNoPty`) + `Broker` (the broker's own second, independent application to the real grandchild's token) | `Abort` | `ConfiguredOnly` — see RF-01. Was `LiveTokenOrJobQuery`; that probe read the child TOKEN's integrity level, a different kernel object from the mandatory-label ACE this row describes, and succeeded for every Windows token so it could never deny. On the `Broker` arm `nono-shell-broker` DOES perform a real token-RID check (`<= SECURITY_MANDATORY_LOW_RID`) — that is the arm where the observed process is the confined grandchild. |
| `AppContainerProfile` | app-container-profile | `nono-shell-broker/src/main.rs:322-336`, `:536-556`, `agent_daemon/launch.rs:636-650` | `Broker` **scoped to the `BrokerLaunchNoPty` spawn shape** (RF-02: the legacy/PTY broker shape is spawned without `--app-container-name` and creates no AppContainer at all), `Daemon` only — **never** any `DirectCli` cell, including `BrokerLaunchNoPty` (Blocker-1: `nono-cli`'s own spawn observes `nono-shell-broker.exe` itself, not the real confined grandchild) | `Abort` | `LiveTokenOrJobQuery` — the probed SID is compared against the expected per-run package SID, not merely tested for presence (RF-05) |
| `DaclSessionSidGrant` | dacl-session-sid-grant | *(none in the shipped tree — see RF-03)* | **Nowhere — expectancy is empty.** RF-03: the only production construction of `AppliedDaclGrantsGuard` is passed `config.package_sid`, not `config.session_sid`; the synthetic per-session restricting SID is granted on no DACL anywhere. The row previously claimed `(DirectCli, WriteRestricted)` and reported `EstablishedNotIndependentlyObservable` for a grant that was never applied. | `Abort` (unreachable — the row is never expected) | `ConfiguredOnly` |
| `DaclPackageSidGrant` | dacl-package-sid-grant | `dacl_guard.rs:92`, `mod.rs:436-440`, `agent_daemon/launch.rs:133`, `:764` | All 5 `DirectCli` arms + `Daemon` (RF-04: `execution_runtime.rs` sets `package_sid: Some(..)` unconditionally, so the guard is constructed on every arm — the previous `BrokerLaunchNoPty`-only cell made a genuinely applied layer classify `NotApplicable` on four arms of five) | `Abort` | `ConfiguredOnly` — classified from the caller's per-launch application report (RF-06) |
| `DaclAncestorTraverse` | dacl-ancestor-traverse | `dacl_guard.rs:236`, `mod.rs:449-455` | All 5 `DirectCli` arms + `Daemon` (RF-04) | `Abort` | `ConfiguredOnly` — RF-06 |
| `DaclAncestorReadAttrs` | dacl-ancestor-read-attrs | `dacl_guard.rs:401`, `mod.rs:473-482` | All 5 `DirectCli` arms. **Not `Daemon`** — RF-07: `DaemonDaclGuard::apply` performs traverse-on-read-only-rules, write-on-workspace and traverse-on-workspace-ancestors, and never calls `grant_sid_read_attributes_on_path`; the cited call sites are CLI-side files the daemon binary does not link. | `Abort` | `ConfiguredOnly` — RF-06 |
| `WfpEgressFilters` | wfp-egress-filters | `network.rs:1602-1610`, `:1730`, `:1775-1800`, `agent_daemon/launch.rs:444` | All 5 `DirectCli` arms + `Daemon`, **conditional on the resolved network backend** (RF-08): `NotApplicable` unless this launch actually selected the WFP backend. See SC4-5 for the "confirmed, not unobservable" correction. | `Abort` | `ConfirmedByEnforcingComponentReport` |
| `FirewallRulesEgress` | firewall-rules-egress | `network.rs:1500-1533`, `:1535-1594` | All 5 `DirectCli` arms (program-path-scoped, not session-scoped), **conditional on the resolved network backend** (RF-08): `NotApplicable` unless this launch actually selected the `netsh advfirewall` backend. | `Abort` | `ConfiguredOnly` — RF-06 |
| `MinifilterAbsence` | minifilter-absence | *(none — no such layer exists in this tree, ADR-65/D-33)* | Every `(entry_path, None)` cell: `DirectCli`, `Broker`, `Daemon` — the one row with no per-arm variation | `FailOpen` — justification: "ADR-65: no minifilter exists; per-file read policy inside one directory is explicitly not claimed" | `NotApplicable` |
| `JobObjectContainment` | job-object-containment | `launch.rs:375`, `:2134-2137`, `agent_daemon/launch.rs:959`, `:1049` | All 5 `DirectCli` arms + `Daemon` | `Abort` | `LiveTokenOrJobQuery` |
| `BrokerAuthenticodeTrustGate` | broker-authenticode-trust-gate | `launch.rs:1505-1506`, `:1828-1829`, `:2190`, `:2236` | `DirectCli` / `BrokerLaunch` and `BrokerLaunchNoPty` arms only (scoped to the arms that spawn `nono-shell-broker.exe`) | `Abort` | `ConfiguredOnly` |
| `InterpreterCoverageGate` | interpreter-coverage-gate | `crates/nono/src/sandbox/windows.rs:2247`, `crates/nono/src/sandbox/mod.rs:925`, `mod.rs:357-362` | All 5 `DirectCli` arms (runs once, before arm selection) | `Abort` | `NotApplicable` — pre-flight, build-time-of-the-launch-plan gate; validated before spawn, not re-checked against a live child afterward |

## Windows layers only — the Unix boundary (D-12)

**Linux Landlock and macOS Seatbelt fail-direction are a named boundary deliberately not
enumerated in this document.** This is not an omission — D-09 scopes this contract to the
Windows-backend layer composition, and the reasoning is on the record here so Phase 119's
security-model boundary statement (BOUND-02/03) can pick it up as a recorded decision rather
than re-deriving it.

**Nearest analog:** Linux Landlock already performs a structurally similar best-effort
downgrade. `crates/nono/src/sandbox/linux.rs:184` declares `ABI_PROBE_ORDER`, the
highest-to-lowest sequence of Landlock ABI versions the sandbox will accept
(`[V6, V5, V4, V3, V2, V1]`); `detect_abi_uncached()` (`linux.rs:209-230`) walks that list and
accepts the first ABI version the running kernel actually supports, silently enforcing fewer
Landlock rule types on an older kernel than it would on a newer one. That is a real
fail-direction decision — a form of graceful degradation, not a hard requirement of "the exact
ABI version compiled against or abort" — and it deserves the same contract-level treatment this
document gives the Windows composite. It is out of scope here only because CINT-01 as written
scopes this phase's registry to Windows, not because the underlying question does not exist on
Linux.

**Disposition:** handed to Phase 119 as a recorded decision, not silence. Phase 119's
BOUND-02/BOUND-03 own writing the equivalent contract (or an explicit "why not" for) the Linux
ABI-downgrade behavior and the macOS Seatbelt backend's own fail-direction posture.

## Latency budget (D-24)

The token and job probes this document's rows use are cheap local calls
(`GetTokenInformation`, `IsProcessInJob`); the project constraint is explicit that the Windows
backend must maintain zero startup latency. The actual measured cost of the startup
self-attestation pass (Plan 08/09/10) is recorded per gated path below. Placeholders are filled
in by Plan 117-12, which owns the measurement:

Measured 2026-08-10 on the development host via dedicated `std::time::Instant`-wrapped unit
tests (real Win32 calls against a real `GetCurrentProcess()` pseudo-handle, not a null/mock
handle — a null handle short-circuits every probe on its own `Err` before reaching the real
`GetTokenInformation`/`IsProcessInJob`/`TokenAppContainerSid` calls, which would measure
error-path cost, not the real probe cost this table needs). None of these are asserted as strict
pass/fail bounds beyond a generous 250ms sanity guard — the numbers below are what the project's
"zero startup latency" constraint (CLAUDE.md) actually costs today, not a benchmark target.

| Gated path (D-23) | Attestation cost | Measurement |
|---|---|---|
| Direct `nono run` (`DirectCli`, `launch.rs` gate point) | **394.4µs** | `exec_strategy_windows::attestation::latency_measurement::attest_and_decide_direct_cli_write_restricted_write_restricted_latency` — `attest_and_decide()` called with `EntryPath::DirectCli`/`WindowsTokenArm::WriteRestricted` against a real `GetCurrentProcess()` handle. |
| Daemon `nono agent launch` (`Daemon`, `agent_daemon/launch.rs` gate point) | **35.7µs** | `agent_daemon::launch::windows_impl::attestation_gate_tests::daemon_attest_and_decide_latency` — `daemon_attest_and_decide()` against a real `GetCurrentProcess()` handle. |
| Broker arm (`Broker`, `nono-shell-broker/src/main.rs` gate point) | **82.8µs** (pre-RF measurement; RF-02 adds a second real Win32 probe to this path, so the current figure is higher — re-measure via the test named here) | `nono-shell-broker`'s `broker::broker_resume_gate_tests::broker_resume_gate_latency_with_real_probes` — `nono::attestation::probe_app_container_sid()` **and** `probe_integrity_level()` (both real Win32 calls) plus `broker_resume_gate()`'s own pure decision logic, against a real `GetCurrentProcess()` handle. |
| Per-tool-call hook (`claude_code_hook.rs`, re-enters the `DirectCli` gate point) | **394.4µs attestation + 1.6962ms cold / 74.8µs warm dedup marker** (see below) | Direct-path attestation cost (row 1 above) plus `output::tests::attestation_downgrade_banner_cold_vs_warm_dedup_marker_latency`, which measures `print_attestation_downgrade_banner`'s per-session dedup marker check SEPARATELY cold (first occurrence this session: `create_dir_all` + `write`, **1.6962ms**) versus warm (an identical repeat: `path.exists()` stat only, **74.8µs**) — the hook path re-enters the direct spawn path on every tool call, so the WARM figure is the steady-state per-invocation cost for a long-running session; the COLD figure is paid once per session per distinct downgraded-layer-set. |

**Broker-arm note:** `broker_resume_gate` itself (the pure decision function, excluding the
probe calls) is allocation-only string/slice comparison with no OS calls — its own cost is not
separable from the measured figure above with `std::time::Instant`'s resolution on this host;
the probe calls dominate.

## Contract vs. code discrepancies

Per D-15, discrepancies between what this contract says and what the code actually does live
here, in a standing section, rather than being quietly reconciled. Every row states its
resolution (fixed here / deferred to which plan) per D-13/D-14 — none reads "quietly
reconciled." Every grep below was re-run live against this working tree on **2026-08-09** and
the command shown is the literal one executed (D-16: greps discover their targets, they do not
confirm a pre-named answer).

| # | Finding | Command run | Hit count | Severity (D-13) | Resolution |
|---|---|---|---|---|---|
| SC4-1 | `crates/nono/src/sandbox/windows.rs::apply()` validates `CapabilitySet` *shape* only — it never confirms the CLI's separate WFP runtime activation actually happened. This is the ADR-86-documented carve-out (library applies only what's in `CapabilitySet`; the CLI owns the runtime WFP pipeline), not a bug — but a reader of `apply()` alone should not assume network enforcement occurred just because `apply()` returned `Ok(())`. | `grep -n "^pub fn apply\(" crates/nono/src/sandbox/windows.rs` | 1 (`:52`), measured 2026-08-09 | Low — already correctly carved out by ADR-86; the risk is documentation clarity, not behavior. Recorded as `fail-open` (justified under ADR-86), not `fail-open-defect`. | **Record only — no code change needed.** Cited here with the ADR-86 boundary explicit. |
| SC4-2 | The `PreparedWindowsLaunch` struct-field comments assert Rust drop order is "reverse-of-declaration" for **struct fields**. This is factually wrong: Rust struct fields drop in forward declaration order; only local stack bindings are LIFO/reverse-of-declaration. The code's actual runtime behavior is still correct today (declaration order already matches the intended labels→dacls→ancestors→network unwind sequence), but the comment's stated mechanism is wrong and could mislead a future editor who reorders fields "to preserve" the (incorrect) rule. | `grep -n "reverse-of-declaration" crates/nono-cli/src/exec_strategy_windows/*.rs` | 6, measured 2026-08-09 (`launch.rs:290` — correctly reverse-of-declaration, since that one IS a local stack binding in `create_process_containment`; `mod.rs:298`, `:307`, `:318`, `:328`, `:922` — the five struct-field-comment sites that are factually wrong) | Low functional risk today (the code happens to behave correctly); Medium latent risk (a future field reorder trusting the wrong stated rule could silently break the unwind sequence). Not a `fail-open-defect` — nothing fails open today, this is a documentation-accuracy defect. | **RESOLVED IN THIS PHASE by Plan 117-10** (RF-10 corrects this cell, which still read "Deferred to Plan 117-10" after that plan had landed). `mod.rs`'s `PreparedWindowsLaunch` doc comment now states the correct rule — struct fields drop in FORWARD declaration order, and this struct's declaration order already IS the desired labels→dacls→ancestors→network unwind order — and instructs future editors to place a new field where it should drop rather than relying on a reverse rule that does not exist for struct fields. |
| SC4-3 | `FirewallRulesNetworkBackend` (the legacy/fallback `netsh advfirewall` egress backend) — is it reachable in production, or dead code superseded by WFP? Resolved by Plan 01 (this phase, prior wave): it is a live, reachable dispatch arm keyed on the compiled policy's `active_backend` (`network.rs:1500`, the `(Blocked, FirewallRules)` match arm), a structurally distinct alternative to WFP's `(Blocked, Wfp)` arm — not a fallback chain where one supersedes the other. Fails closed via `run_netsh_firewall` `Err` propagation with partial-rule rollback (`network.rs:1567-1586`). | `grep -n "FirewallRulesNetworkBackend" crates/nono-cli/src/exec_strategy_windows/network.rs` | 1 declaration site plus the `install()` impl and dispatch arm, measured 2026-08-09 (see `layer_registry.rs`'s module doc comment, "Evidence ledger" §1, for the full citation trail) | N/A — resolved, not an open risk. | **Closed by Plan 01.** `FirewallRulesEgress` is promoted to its own registry row (`LayerId::FirewallRulesEgress`, `outcome: Abort`) rather than folded into `WfpEgressFilters` as a variant. Not a residual discrepancy — recorded here only so the SC4 ledger shows its resolution alongside the still-open items. |
| SC4-4 | `set_windows_wfp_test_force_ready`'s fault-injection toggle is gated by a **runtime** check (`std::env::var_os("NONO_TEST_HARNESS").is_none())`, not a compile-time feature. D-30 requires the seam be compiled out of release builds entirely — a runtime-gated "turn off a confinement layer" switch is live attack surface: the flag exists and is settable in every build, production included, and the only thing standing between an attacker (who can already spawn the process) and flipping it is an environment variable. | `grep -n "NONO_TEST_HARNESS" crates/nono-cli/src/exec_strategy_windows/mod.rs` | 4, measured 2026-08-09 (`:557` doc comment, `:581` doc comment, `:589` the check, `:592` the warning message) | Medium — always-compiled in every release binary, gated only by an attacker-settable env var; currently affects only the `WfpEgressFilters` row (does not silently widen a boundary the operator does not already control via other flags), and requires local process-spawn control to exploit. | **RESOLVED IN THIS PHASE by Plan 117-04** (RF-10 corrects this cell, which still described the runtime gate as current behaviour after that plan had removed it). The `NONO_TEST_HARNESS` runtime gate no longer exists: every fault-injection seam, and `--dangerous-force-wfp-ready` itself, is behind `#[cfg(feature = "layer-fault-injection")]` and is not compiled into a default build at all. The "Finding" column above is retained verbatim as the historical record of what was wrong; the behaviour it describes is no longer shipped. |
| SC4-5 | **New finding, this plan.** CONTEXT.md's D-18 discussion assumed WFP filter state would *normally* sit in an "established, not independently observable" state on ordinary sessions, because a non-elevated supervisor cannot re-query live WFP filter engine state directly. That premise is correct for a *live re-query* — but the pre-spawn `installed_filter_count` IPC check (`assert_wfp_activation_installed_filters`) already fails closed **before** `CREATE_SUSPENDED` even fires, for the specific session's package SID / policy, over the existing named-pipe IPC to the elevated `nono-wfp-service`. The code's own behavior is already `Abort`, not "normally downgrades" — the code is correct; D-18's framing in CONTEXT.md was the thing that needed correcting. | `grep -n "installed_filter_count\|assert_wfp_activation_installed_filters" crates/nono-cli/src/exec_strategy_windows/network.rs` | 9, measured 2026-08-09 (declaration/call/test sites; the load-bearing pair is `:1730` the call and `:1775` the function definition) | Low — this corrects the contract's classification, not the code; the code's fail-closed behavior is already what D-18 wants. | **Record only — no code change needed.** This document's `WfpEgressFilters` row above states `outcome: Abort` / `probe: ConfirmedByEnforcingComponentReport` (a report *from* the elevated enforcing component, not an independent live kernel-state observation — hence not `LiveTokenOrJobQuery`), matching the code as it actually behaves rather than the "unobservable, normally downgraded" framing CONTEXT.md's D-18 discussion assumed. |

## Structural constraints

**Broker-abort-only invariant (Item-1, checker pass 3).** Every registry row expected at
`EntryPath::Broker` (with an attestable `probe`, i.e. anything other than
`ProbeKind::NotApplicable`) carries `outcome: ContractOutcome::Abort` — no
`DegradeWithVisibleClaim`, `SubstituteEquivalentMechanism`, or `FailOpenDefect` value is
permitted on a broker-expected row. This is a structural constraint, not an incidental
property: `required_layers_for_broker()` (Plan 08) filters registry rows to those with
`(EntryPath::Broker, expected: true)` **and** `outcome: Abort`; a row expected at the broker
arm with any other outcome would be silently excluded from that filter *and* already excluded
from `attest_and_decide`'s own dispatch (which never runs with `EntryPath::Broker` — that
attestation lives inside `nono-shell-broker` itself, a separate binary, per Blocker-1's
resolution). Such a row would therefore go **completely unattested** on the broker arm — the
exact green-by-absence failure mode this phase exists to close, re-armed for the next added
row if the constraint were only enforced by convention.

This is why every `EntryPath::Broker`-expected row in the table above **with an attestable
probe** (`MandatoryIntegrityLabel`, `AppContainerProfile`) reads `Abort`: broker-arm rows
structurally cannot carry a degrade or fail-open option, because there is no attestation call
site on that arm capable of downgrading a claim gracefully — only `nono-shell-broker`'s own
probe-then-decide (Plan 11), which can only proceed or abort.

`MinifilterAbsence` is `expected: true` at `(Broker, None)` and reads `FailOpen`, not `Abort`.
That is **not** a violation and this paragraph previously contradicted both the table above and
the code (RF-09): the invariant is scoped to rows with `probe != ProbeKind::NotApplicable`,
exactly as `broker_expected_rows_are_abort_only` implements it. A row that documents a layer
which does not exist has nothing to attest on any arm, so it cannot go "silently unattested" —
the failure mode the invariant closes requires something attestable to begin with.

**Enforced by:** `broker_expected_rows_are_abort_only`
(`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs`, `#[cfg(all(test,
target_os = "windows"))] mod tests`) — a discovery-based unit test that iterates
`all_entries()` and asserts the invariant for every row with an attestable probe, never naming
a specific `LayerId` variant by string. A future row added with `EntryPath::Broker` expectancy
and a non-`Abort` outcome fails this test without the test itself needing to change.

## Downgrade banner behavior

**Per-session dedup policy (Item-2, checker pass 3).** `claude_code_hook.rs` re-enters the
direct spawn path on every single tool call. On a host where a row — WFP is the named candidate
under D-18's own accepted tension — persistently sits non-`Confirmed`, an un-deduplicated
downgrade banner would fire on every single tool invocation, which is both noisy and would
train an operator to ignore the signal entirely.

**Decision:** the downgrade banner dedupes **per session**, not per spawn. The first occurrence
of a given downgraded-layer-set always prints, for every session — this preserves the D-27/
Warning-8 no-silent-downgrade property in full. Only an *identical repeat* of that same
downgraded-layer-set within the same session is suppressed. `--silent` never suppresses the
first occurrence of a downgrade banner; no new global silence switch is introduced by this
policy. In practice, an operator on an affected host observes one banner per session, not one
banner per tool call.

This is a contract-visible behavior — an operator can rely on "one banner per session" as the
observable shape of a persistent downgrade, not an implementation detail buried in code
comments.

**RF-06: what counts as a downgrade.** A `ProbeKind::ConfiguredOnly` row that the supervisor
actually applied for this launch classifies `EstablishedNotIndependentlyObservable` and is **not**
a downgrade — it is the expected baseline for a layer with no independent post-hoc query. Before
this review-fix pass every such row was counted as downgraded unconditionally, which made
`AttestationDecision::Proceed` unreachable on every production path: the non-silenceable banner
printed on every Windows session forever and an un-deduplicated `LayerAttestationDowngraded`
record was appended to the HMAC audit chain on every per-tool-call hook invocation. A
permanently-on warning is functionally identical to no warning, and it meant a session could
never be reported as fully attested — the state this phase exists to be able to claim. A row that
is expected on this arm and was **not** applied now classifies `Unconfirmed` and hits its own
per-row outcome (overwhelmingly `Abort`).

**RF-15 (iteration 2): "applied" means the apply took EFFECT, and there is a third state.**
RF-06's caller-supplied report was derived from *guard construction* — `mandatory_integrity_label`
was the literal `true`, and the DACL fields were `config.package_sid.is_some()`, which
`execution_runtime.rs` sets unconditionally. `AppliedLabelsGuard::snapshot_and_apply` returns
`Ok` when it recorded a skip for **every** path, so a launch that wrote zero mandatory-label ACEs
was still reported established and reached `Proceed`. Each guard now reports its own coverage
(`labels_guard::LabelCoverage`, `dacl_guard::DaclGrantCoverage`), and the contract carries three
application states rather than two:

| Guard coverage | `LayerApplication` | Decision |
|---|---|---|
| Nothing in this launch's policy for the layer to act on | `NotApplicable` | Row dropped |
| The apply ran and covered every non-exempt target | `Applied` | `EstablishedNotIndependentlyObservable`, the expected baseline — not a downgrade |
| Covered some targets, not all (a path already carried a third-party mandatory label; a writable path is not owned so its DACL cannot be edited) | `PartiallyApplied` | `ProceedDowngraded` — the D-27 banner and audit event fire |
| Zero targets covered, on a non-empty policy | `NotApplied` | `Unconfirmed` → `Abort` |

**RF-16 (iteration 2): the downgrade channel must be reachable.** No registry row carries
`DegradeWithVisibleClaim` or `FailOpenDefect`, and the single `FailOpen` row
(`MinifilterAbsence`) is deliberately excluded from `downgraded` — so after RF-06,
`AttestationDecision::ProceedDowngraded` was unreachable on every production path and the D-27
banner, the per-session dedup marker, the `LayerAttestationDowngraded` audit event and the
`session_id` path validator guarding the marker were all dead code in the shipped build.
Iteration 1's defect was "the signal always fires"; RF-06 inverted it to "the signal never
fires". `LayerApplication::PartiallyApplied` (RF-15) is the live production route into that
channel. Two tests pin it — `attestation.rs`'s
`partially_applied_configured_only_row_proceeds_downgraded` (decision core) and `launch.rs`'s
`partially_applied_launch_is_downgraded_not_silently_passed` (real registry, real job-contained
suspended child). A D-26-tightened requirement on a partially established layer still aborts.

**Contract-exempt skips are NOT downgrades.** A mandatory-label skip on a path nono does not own
(`C:\Windows`, granted read by the `system_read_windows` policy group) is a documented D-02
exemption, not a coverage gap — labelling it is structurally impossible for an unprivileged user
and unnecessary on the merits. Counting it would fire the banner on essentially every profile,
recreating the permanently-on warning RF-06 removed. Likewise a read-only rule needs no
package-SID DACL ACE. Only genuine gaps downgrade: a path carrying a label nono did not write,
and a WRITABLE path whose DACL nono cannot edit.

## Review-fix pass (code review 2026-08-10)

A deep code review of this phase found that several probes and guards in the shipped pass could
not return a negative result, so the phase failed its own stated goal ("nono can no longer report
enforcing while a confinement layer is silently inert"). The RF-NN identifiers below are cited
from the registry table and the sections above. Full per-finding evidence is in
`.planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW-FIX.md`.

| # | What was wrong | What changed |
|---|---|---|
| RF-01 | `MandatoryIntegrityLabel` probed the child TOKEN's integrity level and discarded the RID. `GetTokenInformation(TokenIntegrityLevel)` succeeds for every Windows token, so a fully unconfined Medium-IL child classified `Confirmed`; the deny branch was unreachable, and the probed object was not the file-ACE the row describes. | Row is `ConfiguredOnly`; the token-RID observation moved to `nono-shell-broker`, which observes the actual confined grandchild and requires `<= SECURITY_MANDATORY_LOW_RID`. |
| RF-02 | The broker's resume gate sat inside `if is_app_container`, so the legacy/PTY broker arm was never gated (and had no suspend window). It matched only the literal `"AppContainerProfile"` and silently ignored every other required name, including `MandatoryIntegrityLabel`. An absent `NONO_BROKER_REQUIRED_LAYERS` became `""` via `unwrap_or_default()` and read as "require nothing". | Gate hoisted to cover both spawn shapes; the legacy arm spawns `CREATE_SUSPENDED`; unknown required names refuse resume; a missing or empty contract refuses resume; `MandatoryIntegrityLabel` is genuinely attested. |
| RF-03 | `DaclSessionSidGrant` was reported "established" on every `WriteRestricted` launch for a grant that is never applied. | Expectancy emptied; the discrepancy is recorded above and in the registry source. **Open operator decision:** whether the `WriteRestricted` arm should also grant `config.session_sid` (it would widen DACLs on user-owned paths) — deliberately not guessed. |
| RF-04 | Three genuinely-applied DACL layers were expected only at `BrokerLaunchNoPty` and therefore classified `NotApplicable` on four arms of five, silently omitted from the attested claim. | Expectancy widened to all five `DirectCli` arms, matching the unconditional `package_sid`. |
| RF-05 | The AppContainer and restricted-SID probes discarded the value they returned, so a child in a *different* AppContainer attested identically to the correct one. | The broker and daemon cores compare the probed package SID against the expected per-run value. |
| RF-06 | `ConfiguredOnly` rows classified `EstablishedNotIndependentlyObservable` unconditionally — a free pass applied even on paths where no apply ever happened. | The caller reports, per launch, which layers it applied; an expected-but-unapplied row classifies `Unconfirmed`. See "Downgrade banner behavior" above. |
| RF-07 | The daemon reported `DaclAncestorReadAttrs` as established although `DaemonDaclGuard::apply` never grants read-attributes. | Row dropped from the daemon's set and from its `(Daemon, None)` expectancy cell. |
| RF-08 | `WfpEgressFilters` was `expected: true` unconditionally on `BrokerLaunchNoPty` while its status came only from the WFP pre-spawn report, so every non-WFP-backed launch on the primary supervised Windows arm was refused. | Both network rows are conditional on the resolved backend: not applicable, not degraded. |
| RF-09 | The "Structural constraints" paragraph asserted every `Broker`-expected row reads `Abort`, contradicting the table's own `MinifilterAbsence` row and the test that enforces the invariant. | Paragraph reconciled with the table and with `broker_expected_rows_are_abort_only`'s `probe != NotApplicable` scoping. |
| RF-10 | SC4-2 and SC4-4 still read "Deferred to Plan 117-10 / 117-04" after both plans had landed in this same phase. | Both moved to a resolved state. |
| RF-11 | `--dangerous-force-wfp-ready` is compiled out of default builds, but four Windows block-net integration tests still passed it and were gated only on `target_os`. Two failed; two passed for the wrong reason (a clap parse error satisfies `!status.success()`). | The four tests are gated on `feature = "layer-fault-injection"`, and `make test-layer-fault-injection` plus a `Windows Layer Fault Injection` CI job now run that build. |
| RF-12 | The CINT-03 forced-unavailable suites, the eight in-crate seam regression tests and the D-32 `every_registry_row_has_a_test` drift gate were behind a feature no build, `make` target or CI job enabled — so none of this phase's own evidence executed, and adding a 14th `LayerId` failed nothing. | `layer_registry_meta_test.rs` (pure source-text scanning) drops the feature gate entirely; the `make` target and CI job above run the rest. |
| RF-13 | `HKLM\SOFTWARE\Policies\nono\RequiredLayers` is documented as a fleet control but is never read and never enforced — a silently ignored security control. | The reader now DETECTS a configured sub-key and emits a loud `RequiredLayersNotEnforced` warning; the type's doc comment states plainly that it is not enforced. **Open operator decision:** where the already-read machine policy is carried to the Windows launch path, and acceptance that a fleet registry key can then refuse launches. Enforcement is NOT implemented. |
| RF-15 | *(Iteration 2, CR-14.)* `AppliedLayers` reported guard **construction**, not guard **effect**: `mandatory_integrity_label` and `interpreter_coverage_gate` were literal `true`s and the DACL fields were the unconditionally-`Some` `config.package_sid`. A launch that wrote zero mandatory-label ACEs was reported fully attested. | Each guard reports its own coverage; `LayerApplication` gains `PartiallyApplied` and a fail-secure `#[default]` of `NotApplied`; the dead `interpreter_coverage_gate` field is removed (its row's `ProbeKind::NotApplicable` meant `classify_row` never consulted it). See RF-15 above. |
| RF-16 | *(Iteration 2, NR-02.)* `AttestationDecision::ProceedDowngraded` was unreachable on every production path, making the D-27 banner, dedup marker and `LayerAttestationDowngraded` audit event dead code. | `LayerApplication::PartiallyApplied` routes an `Abort`-outcome row into `downgraded`. See RF-16 above. |
| RF-14 | *(Found during the fix pass, not in the review.)* A Plan 117-10 unit-test fixture constructed `NetworkEnforcementGuard::FirewallRules { staged_dir: PathBuf::from("."), .. }`; the guard's `Drop` ran an unconditional `remove_dir_all` on that field, deleting the whole `crates/nono-cli` package tree (cargo's test CWD) every time the test ran. | `cleanup_network_enforcement_staging` refuses any path that is not a strict subdirectory of `%TEMP%/nono-net-block`, compared by path components; the fixture points at a never-created path under that root. |

## Manual verification (D-31)

Per D-31, a row that cannot be tested on an ordinary host is a **loud, named gap** — never a
silent `#[ignore]`. CINT-03's own wording is "a contract entry with no such test is not
satisfied"; the entries below are the rows for which this plan's automated
`crates/nono-cli/tests/layer_force_unavailable.rs` cannot itself provide that test on an ordinary
(non-elevated, non-domain-joined, non-production-signed) development host, mechanically enforced
loud by `crates/nono-cli/tests/layer_registry_meta_test.rs`'s `host_gated_rows_are_loud` test
(every entry below must appear, by name, in this section, or that test fails the build).

### `LayerId`-scoped rows

| `LayerId` | Why manual | Manual verification steps |
|---|---|---|
| `DaclSessionSidGrant` | RF-03: this layer does not exist in the shipped tree — the only production construction of `AppliedDaclGrantsGuard` is passed `config.package_sid`, and the synthetic per-session restricting SID is granted on no DACL anywhere. The row's expectancy is empty, so there is nothing to force unavailable. | Manual verification is a CODE READ, not a run: confirm `grep -rn "session_sid" crates/nono-cli/src` still shows no DACL grant of `config.session_sid`. If that ever changes, the registry expectancy must be restored in the same commit. See RF-03 for the open operator decision on whether the `WriteRestricted` arm SHOULD grant it. |
| `WfpEgressFilters` | Requires a live, elevated `nono-wfp-service` and a non-elevated daemon session (per-SID WFP is daemon-path only, `nono agent launch`, not direct `nono run`). | Install/start `nono-wfp-service` as admin; from a **non-elevated** shell run a confined daemon session with the WFP layer forced unavailable; assert the contracted `Abort` outcome. |
| `MinifilterAbsence` | Structurally untestable — no minifilter driver exists in this tree (ADR-65 stands). The row documents a deliberate structural absence, not a probeable mechanism. | Not a skip: recorded here as an explicit "structurally absent / not applicable" entry citing ADR-65, per Phase 116 D-08's `structurally-blocked` row form. |
| `FirewallRulesEgress` | No force-unavailable seam was shipped for this row (Plans 04/06/07 covered WFP + the CLI-side token/label/DACL/JobObject/AppContainer layers only); adding one is a new production-code seam outside this plan's test-file-scoped remit. | Temporarily block `netsh advfirewall` (e.g. a conflicting rule, or a restricted execution policy) and confirm a `nono run` with the `FirewallRules` backend selected aborts with the partial-rule-rollback behavior at `network.rs:1567-1586`. |
| `BrokerAuthenticodeTrustGate` | The gate is skipped entirely under `is_dev_build_layout()` (`launch.rs:2190`/`:2194`) — active only in a signed, production (non-dev-layout) install. | From a signed release install outside `target/...`, stage a `nono-shell-broker.exe` signed by a different identity than `nono.exe` and confirm the broker-arm spawn refuses (`broker_authenticode.rs::broker_signature_mismatch_refuses_spawn` already covers `verify_broker_authenticode` directly; this item is the live-install end-to-end round-trip). |
| `InterpreterCoverageGate` | A pre-flight, build-time-of-the-launch-plan check (`probe: ProbeKind::NotApplicable`), not a D-21 post-spawn attestation — no force-unavailable seam applies. | The fail-closed mechanism is already proven at the library-unit level: `crates/nono/src/sandbox/windows.rs::validate_launch_paths_refuses_uncovered_interpreter`. A full CLI-level round-trip additionally requires reconstructing `resolve_interpreter_paths`'s shebang/PATH resolution shape end-to-end — deferred as a follow-up. |
| `AppContainerProfile` | The real AppContainer-confined child is spawned INSIDE a separate `nono-shell-broker.exe` process (Blocker-1); nono-cli's own gate never attests this layer. Driving the `BrokerLaunchNoPty` arm externally requires a real console session — found, during this plan's own execution, to be console-fragile even from a PowerShell-wrapped `cargo test` harness (broker spawn fails with GLE=87 under git-bash/MSYS per project memory). | The underlying seam has real, passing in-crate coverage (`nono-shell-broker/src/main.rs::run_fails_when_app_container_forced_unavailable`, Plan 07, 24/24 passing). Manual steps: from a real (non-git-bash) PowerShell console, run `nono run --profile claude-code` with `NONO_FORCE_UNAVAILABLE_APP_CONTAINER=1` set and confirm the broker-arm spawn refuses. |
| `DaclAncestorTraverse` | Shares `dacl_guard.rs`'s ONE `DACL_GRANT_FORCE_UNAVAILABLE` flag with `DaclSessionSidGrant`/`DaclPackageSidGrant`, but `AppliedAncestorTraverseGuard::snapshot_and_apply` is constructed STRICTLY AFTER `AppliedDaclGrantsGuard::snapshot_and_apply` in `prepare_live_windows_launch` (`mod.rs:449` then `:462`) — arming the shared flag always aborts the launch at the FIRST guard via its `?`, so this row's own apply function is never reached by any external, black-box subprocess test. | Direct evidence is the in-crate unit test in `dacl_guard.rs`'s `#[cfg(test)]` module (117-06: "8 feature-gated regression tests proving each hook short-circuits before its real OS call"), which calls `AppliedAncestorTraverseGuard::snapshot_and_apply` directly. |
| `DaclAncestorReadAttrs` | Identical reasoning to `DaclAncestorTraverse` — same shared flag; `AppliedAncestorReadAttributesGuard::snapshot_and_apply_targets` is constructed even later (`mod.rs:486`), strictly after `applied_dacls`'s `?` would already have returned. | Direct evidence is the in-crate unit test in `dacl_guard.rs`'s `#[cfg(test)]` module (117-06). |
| `RestrictedToken` | The seam is checked LATE — inside `spawn_windows_child` (`launch.rs:1620`), AFTER the Windows `Supervised`-strategy session file + capability-pipe event loop has already started. Empirically, on this plan's development host, killing/unwinding a piped-stdio `nono.exe` child spawned from a `cargo test` harness process AFTER that event loop starts reproducibly stalls the child's own teardown — even across a bounded-wait-and-kill retry loop (4 attempts × 45s). This is a host/harness characteristic, not a defect: two independent, isolated single-invocation reproductions via PowerShell's `Start-Process` (outside `cargo test`'s own subprocess management) completed correctly in well under a second each, producing the exact expected diagnostic. | From a real PowerShell console, run `target\debug\nono.exe run -- cmd /c echo hello` with `NONO_FORCE_UNAVAILABLE_RESTRICTED_TOKEN=1` set; confirm stderr contains `Startup self-attestation failed for layer RestrictedToken: forced unavailable by test seam`. |
| `JobObjectContainment` | Same late-checked, event-loop-already-started class as `RestrictedToken` (`apply_process_handle_to_containment`, `launch.rs:404-422`) and the same empirically-observed host/harness teardown-stall characteristic. | From a real PowerShell console, run `target\debug\nono.exe run -- cmd /c echo hello` with `NONO_FORCE_UNAVAILABLE_JOB_OBJECT=1` set; confirm the `JobObjectContainment` diagnostic. |

Both `RestrictedToken` and `JobObjectContainment` have a shipped, compiled-out (D-30)
`layer-fault-injection`-gated force-unavailable seam AND this plan's env-var bridge
(`NONO_FORCE_UNAVAILABLE_RESTRICTED_TOKEN`/`NONO_FORCE_UNAVAILABLE_JOB_OBJECT`,
`crates/nono-cli/src/command_runtime.rs`) — the mechanism is proven correct (see the manual
reproduction evidence above); what is not reliably automatable from THIS harness is the
subprocess-management/teardown timing, not the seam itself.

### Cross-cutting security assumptions

Distinct from the `LayerId`-scoped rows above: a cross-cutting security assumption not tied to
one registry row (Blocker-3, checker pass 2 — the concrete pickup Plan 09's flagged assumption
needed and did not get in the prior pass).

| Assumption | Why manual | Manual verification steps |
|---|---|---|
| `etw-applog-child-readability` | Plan 09 added `SecurityEvent.downgraded_layers` on the documented-but-unverified assumption that ETW/Application Event Log is unreadable by a Low-IL/AppContainer child (D-28's "layer-specific detail stays off channels the confined process can read"). A static SDDL parse could be added as a follow-up, but asserting a specific SDDL shape from memory without live verification risks baking in a wrong "safe" claim — worse than a named, loud manual gap. | Run `wevtutil gl Application` (or the equivalent `EvtOpenChannelConfig`/`EvtGetChannelConfigProperty` Win32 call for `EvtChannelConfigPropertyAccess`) and inspect the returned `channelAccess` SDDL string for any ACE granting read (`0x1`, generic-read) to a SID broader than the expected administrative/eventlog-reader set — in particular check whether `S-1-1-0` (Everyone) or any low-integrity/AppContainer-relevant SID is present. If such a SID is present, `SecurityEvent.downgraded_layers` is NOT operator-only and D-28's channel-separation property is violated; escalate as a new SC4 discrepancy. |

**Enforced by:** `crates/nono-cli/tests/layer_registry_meta_test.rs`'s `every_registry_row_has_a_test`
(D-32 discovery), `host_gated_rows_are_loud`, and `security_assumptions_are_loud` — all three fail
the build if a row is added to `MANUALLY_VERIFIED`/`MANUAL_SECURITY_ASSUMPTIONS` without a
non-empty reason, or if a listed entry stops appearing in this section.
