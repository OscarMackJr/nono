# Phase 90: v3.0 Host-Gated UAT Drain - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-20
**Phase:** 90-v3-0-host-gated-uat-drain
**Areas discussed:** DRAIN-04 test shape, Daemon tracing init, Drain closeout artifact, min_severity/opt-out parity

---

## DRAIN-04 test shape (non-host-gated test)

| Option | Description | Selected |
|--------|-------------|----------|
| Integration: event flows to layer | Drive a synthesized `nono_security::network_deny` event through the daemon-registered layer; assert it is processed (chain advances / SecurityEvent built) | ✓ |
| Unit: daemon registers the layer | Assert the daemon init constructs + registers a layer from policy.telemetry; registration only | |
| Both layers of test | Unit registration test + integration event→layer test | |

**User's choice:** Integration: event flows to layer (Recommended)
**Notes:** Grounded in scout finding that the in-process proxy emits `network_deny` inside the daemon process (`nono-proxy/src/audit.rs:204`), so an event→layer flow is testable without a live host. Registration-only would not prove the wiring actually carries a denial to emit.

---

## Daemon tracing init

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated minimal daemon init | Small daemon-side helper registering SecurityEventLayer (+ETW); sources TelemetryConfig from already-resolved policy.telemetry; keeps nono-agentd standalone | ✓ |
| Share the CLI's init_tracing() | Refactor cli_bootstrap::init_tracing into a shared path both call | |

**User's choice:** Dedicated minimal daemon init (Recommended)
**Notes:** Avoids coupling the standalone daemon binary to the CLI `Cli` arg type and bootstrap. Mirrors `init_registry` composition without the env-filter verbosity machinery the daemon doesn't need.

---

## Drain closeout scope (DRAIN-01/02/03)

| Option | Description | Selected |
|--------|-------------|----------|
| Run + record verdicts, no gate code | Run existing verify-dark gates on dev host, capture verdict JSON, record host-gated residuals in 90-HUMAN-UAT.md; no gate-script changes unless broken | ✓ |
| Also harden/extend the gates | Treat gate scripts as needing new code in Phase 90 | |

**User's choice:** Run + record verdicts, no gate code (Recommended)
**Notes:** Scout confirmed the 5 gate scripts already exist and implement the SKIP_HOST_UNAVAILABLE host-gated pattern. A broken gate found during the run is handled as a debug finding, not planned feature work. Verdict recording mirrors the prior 88-HUMAN-UAT.md.

---

## min_severity / opt-out parity (DRAIN-03 ↔ DRAIN-04)

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — in scope, reuse policy.telemetry | Daemon threads already-resolved policy.telemetry (enabled + min_severity) into its layer; admin opt-out applies to daemon-launched denials | ✓ |
| No — CLI TELEM-01/04 already covers it | Daemon uses default TelemetryConfig; opt-out covered only on CLI path | |

**User's choice:** Yes — in scope, reuse policy.telemetry (Recommended)
**Notes:** Folds DRAIN-03's HKLM→emit admin control into the daemon path. Must preserve the Phase 83 D-04 SOLE-read contract — thread the telemetry config from the existing `resolve_machine_egress_policy` read rather than adding a second HKLM read.

---

## Claude's Discretion

- Exact name/location of the daemon tracing-init helper (module vs function), subject to the "minimal, standalone, no CLI coupling" constraint.
- The precise synthesized-event mechanism in the integration test (in-process `tracing::warn!` vs direct `on_event` drive).

## Deferred Ideas

- Capturing child-agent-process denials under daemon telemetry (separate concern if daemon-process registration proves insufficient).
- True live UAT (real clean VM, live SIEM, real dual-layer WFP on a second host) remains operator-gated host-gated tech-debt.
