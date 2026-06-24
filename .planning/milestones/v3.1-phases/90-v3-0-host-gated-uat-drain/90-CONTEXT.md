# Phase 90: v3.0 Host-Gated UAT Drain - Context

**Gathered:** 2026-06-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Drain the v3.0 host-gated UAT debt for the v3.1 milestone. Two distinct kinds of work:

1. **Real code (DRAIN-04):** Wire daemon-side security telemetry. `nono-agentd` currently
   sets up **no tracing subscriber at all**, so daemon-launched agent denials emit nothing.
   This phase registers a `SecurityEventLayer` in the daemon process so daemon-path
   `nono_security::*` denial events are captured and emitted — real wired code with a
   non-host-gated test.

2. **Closeout (DRAIN-01/02/03):** The `verify-dark.ps1` scripted gates for these items
   **already exist and are mature**. This phase runs them on the dev host, records verdicts,
   and explicitly marks each residual live step as host-gated tech-debt. **No gate-script
   code changes** are expected unless a gate is found broken.

**In scope:** DRAIN-04 daemon telemetry wiring + integration test; running existing scripted
gates and recording verdicts/host-gated residuals for DRAIN-01/02/03.

**Out of scope:** New gate-script features; live clean-VM / live-SIEM / dual-layer-WFP runs
on real hosts (these remain acknowledged host-gated tech-debt, operator-gated); any crate
publish (milestone-marker only, future release leapfrogs to ≥ 0.65.0).
</domain>

<decisions>
## Implementation Decisions

### DRAIN-04 — Daemon telemetry wiring (real code)
- **D-01:** The required **non-host-gated test is integration-style**: it drives a synthesized
  `nono_security::network_deny` tracing event through the daemon-registered `SecurityEventLayer`
  and asserts the event is actually processed (HMAC chain advances / a `SecurityEvent` is built).
  Registration-only (unit) is insufficient — the test must prove the event reaches the layer's
  `on_event` path, not just that the layer was constructed. (Rationale: the in-process proxy
  emits `nono_security::network_deny` **inside the daemon process**, so an event→layer flow is
  testable without a live host.)
- **D-02:** The daemon registers the layer via a **dedicated minimal daemon-side tracing-init
  helper** — NOT by refactoring/sharing the CLI's `cli_bootstrap::init_tracing`. The helper
  registers `SecurityEventLayer` (+ the `tracing-etw` "nono" provider layer on Windows, D-03
  non-fatal if ETW build fails) and keeps `nono-agentd` a standalone binary with no coupling
  to the CLI `Cli` arg type / bootstrap. Mirror the existing `init_registry` composition.
- **D-03:** The daemon's `SecurityEventLayer` **must honor the same HKLM `policy.telemetry`**
  (`enabled` opt-out + `min_severity` threshold) as the CLI. Source it from the
  `MachineEgressPolicy` that the daemon **already reads** in
  `resolve_machine_egress_policy` (`nono::read_machine_egress_policy()` → `policy.telemetry`),
  which currently discards the telemetry field. This folds DRAIN-03's admin opt-out /
  `min_severity` HKLM→emit control into the daemon path. Absent policy → `TelemetryConfig::default()`
  (default-ON, matching the CLI's `None` branch). Fail-secure posture from
  `resolve_machine_egress_policy` (Err → abort) is preserved — the SOLE-read contract (D-04 of
  Phase 83) must not gain a second HKLM read; thread the already-resolved telemetry config through.

### DRAIN-01/02/03 — Scripted-gate closeout
- **D-04:** Closeout = **run the existing `verify-dark.ps1` gates on this dev host, capture
  verdict JSON, and record verdicts + explicitly-host-gated residual live steps**. No gate-script
  code changes unless a gate is found broken during the run (treat a broken gate as a debug
  finding, not planned feature work).
- **D-05:** Verdicts and host-gated residuals are recorded in a **`90-HUMAN-UAT.md`** doc in the
  phase dir (mirrors the prior `88-HUMAN-UAT.md` pattern), capturing per-gate verdict
  (PASS / FAIL / SKIP_HOST_UNAVAILABLE) and the residual live step that stays operator-gated.
  Gate ownership of verdict JSON persistence stays with `verify-dark.ps1` (WR-04) — the doc
  references/summarizes, it does not re-implement persistence.
- **D-06:** Gate→requirement mapping for the closeout:
  - DRAIN-01 → `clean-host-install.ps1` + `deploy-silent-install.ps1`
  - DRAIN-02 → `wfp-egress-isolation.ps1` + `egress-policy-deny.ps1`
  - DRAIN-03 → `telemetry-event-emit.ps1`

### Claude's Discretion
- Exact name/location of the daemon tracing-init helper (e.g. a new `agent_daemon::telemetry`
  module vs a function in `nono-agentd.rs`) is the planner/researcher's call, subject to D-02
  (minimal, standalone, no CLI coupling).
- The precise synthesized-event mechanism in the D-01 integration test (in-process
  `tracing::warn!` vs a direct `on_event` drive) is an implementation detail for research.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 90 scope & requirements
- `.planning/ROADMAP.md` §"Phase 90: v3.0 Host-Gated UAT Drain" — goal, success criteria, DRAIN-01..04
- `.planning/REQUIREMENTS.md` §"v3.0 Host-Gated UAT Drain (DRAIN)" — DRAIN-01..04 acceptance text
- `.planning/milestones/v3.0-MILESTONE-AUDIT.md` — the original host-gated tech-debt this phase drains
- `.planning/milestones/v3.0-ROADMAP.md` / `v3.0-REQUIREMENTS.md` — v3.0 DEPLOY-01/03/05, EGRESS-02, TELEM-01/04 source items

### DRAIN-04 telemetry wiring (real code)
- `crates/nono-cli/src/telemetry/mod.rs` — `SecurityEventLayer`, `on_event`, severity/min_severity filtering (reuse target)
- `crates/nono-cli/src/cli_bootstrap.rs` §`init_tracing` / `init_registry` — the CLI registration pattern to mirror (D-02), incl. the Windows `tracing-etw` arm
- `crates/nono-cli/src/bin/nono-agentd.rs` — daemon binary; `run_service`/`run_foreground_mode` are where init must land; currently NO subscriber
- `crates/nono-cli/src/agent_daemon/mod.rs` §`resolve_machine_egress_policy` (line 352) — SOLE HKLM read; currently discards `policy.telemetry` (D-03 threads it through)
- `crates/nono-cli/src/main.rs:173-178` — how the CLI sources `policy.telemetry` → `init_tracing` (the parity reference)
- `crates/nono-proxy/src/audit.rs:204` — `nono_security::network_deny` emit site that runs **in-process** in the daemon (the D-01 test's event source)
- `crates/nono-cli/src/telemetry/windows.rs` — `emit_security_event` ETW/Application-Log sink
- `docs/adr/telemetry-tamper-evidence.md` — telemetry HMAC-chain / domain-separator design (D-05/D-06 from Phase 84)

### DRAIN-01/02/03 scripted gates (closeout)
- `scripts/verify-dark.ps1` — the unattended gate runner (verdict classes, exit-code mapping, WR-04 persist)
- `scripts/gates/clean-host-install.ps1`, `scripts/gates/deploy-silent-install.ps1` — DRAIN-01
- `scripts/gates/wfp-egress-isolation.ps1`, `scripts/gates/egress-policy-deny.ps1` — DRAIN-02
- `scripts/gates/telemetry-event-emit.ps1` — DRAIN-03
- Prior pattern: `88-HUMAN-UAT.md` (host-gated verdict-recording doc shape, D-05)
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `SecurityEventLayer::new(config: TelemetryConfig, session_id: String)` (telemetry/mod.rs) —
  already platform-agnostic; constructs ephemeral HMAC key + salt. Directly reusable by the daemon.
- `init_registry(...)` composition in `cli_bootstrap.rs` — shows the exact registry/ETW arm to
  replicate in the daemon's minimal init helper (D-02).
- `resolve_machine_egress_policy` — already performs the SOLE `read_machine_egress_policy()`;
  extend its return (or a sibling) to surface `policy.telemetry` so DRAIN-04 reuses the read (D-03).
- Mature `verify-dark.ps1` + 5 gate scripts — DRAIN-01/02/03 need no new gate code (D-04).

### Established Patterns
- **SOLE HKLM read (Phase 83 D-04):** the daemon must not add a second `read_machine_egress_policy`
  call. Thread telemetry config from the existing read.
- **Fail-secure policy load (D-07):** present-but-broken policy → abort; absent → fall-through to
  default. The telemetry path must preserve this (absent → default-ON `TelemetryConfig`).
- **D-03 non-fatal ETW:** if `tracing-etw` layer build fails, continue without it (never abort).
- **Gate contract:** gates RETURN verdict objects; only `verify-dark.ps1` persists/exits (WR-04).

### Integration Points
- New daemon tracing-init helper called once at daemon startup in BOTH `run_service` (service mode)
  and `run_foreground_mode` (dev mode) in `nono-agentd.rs`, after policy resolution so the
  telemetry config is available.
- The in-process proxy (`build_daemon_state` → `nono_proxy::server::start`) emits
  `nono_security::network_deny` in-process — the registered layer captures it; this is the
  concrete event source the D-01 integration test exercises.

### Cross-target note (CLAUDE.md MUST)
- `nono-agentd.rs` is `#[cfg(target_os = "windows")]`-gated and the ETW arm is Windows-only.
  Any commit here MUST be cross-target-clippy verified (linux-gnu + apple-darwin) or marked
  PARTIAL→CI per `.planning/templates/cross-target-verify-checklist.md`. The Windows-only daemon
  init is exactly the cross-target blind-spot class flagged in prior phases.
</code_context>

<specifics>
## Specific Ideas

- The DRAIN-04 test should key off the **proxy's in-process `network_deny`** because it is the
  one denial source provably emitted within the daemon process — avoid designing the test around
  child-agent-process denials (whose emission topology is a research question, not a locked decision).
- Keep daemon telemetry init **minimal** — the daemon is a standalone binary; do not pull in the
  CLI `Cli` type or env-filter verbosity flags it doesn't have.
</specifics>

<deferred>
## Deferred Ideas

- Capturing **child-agent-process** denials (path_deny from the spawned confined agent's own
  process) under daemon telemetry — if daemon-process registration proves insufficient for
  some event types, propagating the layer into child agents is a separate concern, not Phase 90.
- True live UAT execution (real clean VM, live SIEM ingestion, real dual-layer WFP on a second
  host) remains acknowledged host-gated tech-debt — operator-gated, not drained by code here.

None folded from todos — discussion stayed within phase scope.
</deferred>

---

*Phase: 90-v3-0-host-gated-uat-drain*
*Context gathered: 2026-06-20*
