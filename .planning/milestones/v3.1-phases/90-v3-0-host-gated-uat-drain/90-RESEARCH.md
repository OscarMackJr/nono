# Phase 90: v3.0 Host-Gated UAT Drain - Research

**Researched:** 2026-06-20
**Domain:** Daemon-side tracing/telemetry wiring (Rust, tracing-subscriber, Windows-gated) + PowerShell scripted-gate closeout
**Confidence:** HIGH (all claims grounded in directly-read source; no external library guesswork — this is a wiring phase against existing in-repo code)

## Summary

Phase 90 has two disjoint work-streams. **DRAIN-04 is real code**: register a `SecurityEventLayer` in the `nono-agentd` daemon process so the in-process proxy's `nono_security::network_deny` events are captured, with a non-host-gated test proving the event reaches the layer's `on_event` path. **DRAIN-01/02/03 are scripted-gate closeout**: the `verify-dark.ps1` gates already exist and are mature; run them, capture verdict JSON, and record host-gated residuals in a `90-HUMAN-UAT.md` doc.

The single most consequential discovery for planning: **`nono-cli` has NO library target.** It compiles as two independent binaries (`nono` from `src/main.rs`, `nono-agentd` from `src/bin/nono-agentd.rs`). The `telemetry` module (`SecurityEventLayer`) is declared `pub(crate) mod telemetry;` *inside main.rs*, so it is compiled **only into the `nono` binary**. The `agent_daemon` module is `#[path]`-included into `nono-agentd.rs` and is explicitly documented as a **standalone binary that "cannot reach `crate::policy`"** etc. Therefore `nono-agentd` today has **no access whatsoever** to `SecurityEventLayer`. D-02's "minimal daemon-side tracing-init helper" must first *make the telemetry layer reachable from the daemon binary* — that is the core architecture decision, not an afterthought.

A second decisive discovery: there is **no public/observable side-effect** of `on_event`. The HMAC chain `sequence`/`head` fields are `pub(crate)`, `inner` is private, and the only emit is `windows::emit_security_event` (a no-op-ish stderr/tracing path on non-Windows). The existing unit tests assert chain advance by reaching `layer.inner.lock()` **from the same module**. The D-01 test must therefore live in module scope that can see those internals (same `#[path]`-included tree), or the layer must gain a tiny `pub(crate)` test accessor (e.g. `chain_sequence()`).

**Primary recommendation:** Make the telemetry source files reachable from `nono-agentd` by adding `#[path = "../telemetry/mod.rs"] mod telemetry;` to `nono-agentd.rs` (mirroring how `agent_daemon` is already `#[path]`-included). Add a new `#[path = "../agent_daemon/telemetry_init.rs"]`-style minimal helper module that builds a `SecurityEventLayer` from the threaded `TelemetryConfig` and composes a registry mirroring `init_registry` (non-Windows: registry+layer; Windows: +`tracing-etw` "nono" arm, non-fatal). Thread `policy.telemetry` out of `resolve_machine_egress_policy` by extending its return tuple. Place the D-01 test as an inline `#[cfg(test)] mod tests` in the daemon-telemetry helper, driving `on_event` directly (or via `tracing::subscriber::with_default`) and asserting the `pub(crate)` chain sequence incremented.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Daemon tracing-subscriber registration | `nono-agentd` binary (daemon process) | — | The subscriber is a per-process global; must be set once in the daemon's own process, after policy resolution. |
| `SecurityEventLayer` (capture `nono_security::*`) | `telemetry` module (shared source) | — | Already platform-agnostic; reused verbatim — must be made *reachable* from the daemon binary. |
| HKLM policy read (incl. `telemetry`) | `nono` core crate (`machine_policy.rs`) | daemon (`resolve_machine_egress_policy` SOLE read) | Library owns the registry read; daemon owns the single call site (Phase 83 D-04 SOLE-read contract). |
| `nono_security::network_deny` emit | `nono-proxy::audit` (in-process inside daemon) | — | The proxy runs **in-process** in the daemon (`build_daemon_state` → `nono_proxy::server::start`), so its events reach a daemon-registered layer. |
| ETW/Application-Log sink | `telemetry::windows` (Windows-only) | — | OS sink; Windows-gated; non-fatal if source unregistered. |
| Scripted gate execution + verdict persistence | `scripts/verify-dark.ps1` (runner, WR-04) | `scripts/gates/*.ps1` (verdict producers) | Gates RETURN verdict dicts; only the runner persists + maps exit codes. |

## Standard Stack

This is a wiring phase against **already-vendored** crates. No new dependencies are introduced. The relevant in-tree crates and their roles:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tracing` | (workspace) | Event emission (`tracing::warn!(target: "nono_security::network_deny", …)`) | Already the project's logging substrate. [VERIFIED: read crates/nono-proxy/src/audit.rs:203] |
| `tracing-subscriber` | (workspace) | Registry + `Layer` composition (`registry().with(layer).init()`) | The CLI's `init_registry` already uses it. [VERIFIED: read cli_bootstrap.rs:194-217] |
| `tracing-etw` | 0.2.3 (MSRV 1.82) | Windows ETW "nono" provider arm (non-fatal `LayerBuilder::new("nono").build()`) | Already used in CLI `init_registry`; MSRV-driven Rust 1.82 bump. [VERIFIED: read cli_bootstrap.rs:209; CLAUDE.md tech-stack] |
| `hmac` + `sha2` + `zeroize` + `rand` | (workspace) | `SecurityEventLayer` HMAC chain + ephemeral key | Already wired in `telemetry/mod.rs`; reused unchanged. [VERIFIED: read telemetry/mod.rs:38-48,209] |

**No `npm`/`pip`/`cargo` install is required for DRAIN-04** — every type (`SecurityEventLayer`, `TelemetryConfig`, `TelemetrySeverity`, `MachineEgressPolicy`) already exists. There is **no Package Legitimacy Audit section** because this phase installs no external packages.

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| PowerShell 7 (`pwsh`) | host | Run `verify-dark.ps1 -Gate <name>` | DRAIN-01/02/03 closeout only. |
| `windows-service` | (workspace) | Daemon `run_service` SCM entrypoint | Already present; init helper is called from inside it. [VERIFIED: read nono-agentd.rs:51-59] |

## Package Legitimacy Audit

**N/A — this phase installs no external packages.** All code reuses crates already pinned in the workspace `Cargo.toml` (synced to `0.62.2` internal path-deps). No registry lookups performed; no new dependency lines added.

## Architecture Patterns

### System Architecture Diagram

```
                      nono-agentd process (Windows service / foreground)
   ┌──────────────────────────────────────────────────────────────────────────┐
   │  startup (run_service OR run_foreground_mode)                             │
   │     │                                                                     │
   │     ▼                                                                     │
   │  resolve_machine_egress_policy(&[])   ── SOLE HKLM read (Phase 83 D-04) ──│
   │     │  returns (allowlist, active, TelemetryConfig)  ◄── D-03 EXTENSION   │
   │     │       │ Err → abort fail-secure                                     │
   │     │       │ None → TelemetryConfig::default() (default-ON)              │
   │     ▼       ▼                                                             │
   │  daemon_telemetry_init(telemetry_config, session_id)  ◄── D-02 NEW HELPER │
   │     │   builds SecurityEventLayer::new(cfg, sid)                          │
   │     │   registry().with(fmt?).with(security_layer)[.with(etw)].init()     │
   │     │   (set ONCE — global subscriber; OnceLock/try_init guard)           │
   │     ▼                                                                     │
   │  build_daemon_state → nono_proxy::server::start (IN-PROCESS proxy)        │
   │     │                                                                     │
   │     │  on a denied egress request:                                        │
   │     ▼                                                                     │
   │  nono_proxy::audit::log_network_deny()                                    │
   │     └─ tracing::warn!(target:"nono_security::network_deny",               │
   │                       host=…, port=…, agent_pid=…)  ── EVENT SOURCE ──────│
   │                       │                                                   │
   │                       ▼  (captured by the registered global subscriber)   │
   │  SecurityEventLayer::on_event()                                           │
   │     ├─ target starts_with "nono_security::" ? yes                         │
   │     ├─ config.enabled ? (D-03 admin opt-out)                              │
   │     ├─ severity ≥ min_severity ? (D-03 threshold)                         │
   │     ├─ advance_chain()  ── HMAC sequence++ / head mutates ◄── D-01 ASSERT │
   │     └─ windows::emit_security_event() → ETW + Application Log (Win-only)   │
   └──────────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure (delta only)

```
crates/nono-cli/src/
├── bin/nono-agentd.rs            # ADD: #[path="../telemetry/mod.rs"] mod telemetry;
│                                 #        #[path="../agent_daemon/telemetry_init.rs"] (or inline) helper decl
│                                 # ADD: call daemon_telemetry_init(...) in run_service + run_foreground_mode
├── agent_daemon/
│   ├── mod.rs                    # EDIT: resolve_machine_egress_policy return → add TelemetryConfig
│   └── telemetry_init.rs         # NEW (D-02): minimal daemon tracing-init helper + D-01 inline test
└── telemetry/mod.rs              # OPTIONAL: add pub(crate) test accessor (chain_sequence) if test drives on_event
```

### Pattern 1: Daemon tracing-init helper (D-02) — MIRROR `init_registry`, do NOT share it

**What:** A standalone function that builds the `SecurityEventLayer` from a `TelemetryConfig` and composes the registry exactly like `cli_bootstrap::init_registry`, but **without** the CLI `Cli` type, env-filter verbosity flags, or file-log arms the daemon doesn't have.

**When to use:** Once at daemon startup, after `resolve_machine_egress_policy`, in both `run_service` and `run_foreground_mode`.

**Example (composition shape — mirror, not reuse):**
```rust
// Source: mirrors crates/nono-cli/src/cli_bootstrap.rs:178-217 (init_registry)
// Lives in a NEW daemon-only helper. No `Cli`, no EnvFilter verbosity, no file-log arm.
use crate::telemetry::SecurityEventLayer;     // reachable via #[path]-include in nono-agentd.rs
use nono::TelemetryConfig;
use tracing_subscriber::prelude::*;

pub(super) fn daemon_telemetry_init(config: TelemetryConfig, session_id: String) {
    let security_layer = SecurityEventLayer::new(config, session_id);

    // SecurityEventLayer is unfiltered (security events always pass). The daemon has
    // no -v/-vv verbosity surface, so a minimal "warn" env filter is sufficient (or
    // omit the fmt layer entirely — the daemon already uses tracing::info!/debug!).
    #[cfg(not(target_os = "windows"))]
    {
        let _ = tracing_subscriber::registry()
            .with(security_layer)
            .try_init();           // try_init — NEVER panic if a subscriber is already set
    }

    #[cfg(target_os = "windows")]
    {
        let base = tracing_subscriber::registry().with(security_layer);
        match tracing_etw::LayerBuilder::new("nono").build() {
            Ok(etw) => { let _ = base.with(etw).try_init(); }
            Err(e) => {
                eprintln!("nono-agentd: telemetry: ETW layer init failed ({e}); continuing");
                let _ = base.try_init();          // D-03 non-fatal
            }
        }
    }
}
```
[CITED: crates/nono-cli/src/cli_bootstrap.rs:178-217] — note the CLI uses `.init()` (panics if already set); the daemon should use `.try_init()` because `run_service_mode` may fall through to `run_foreground_mode`, risking a double-init. See Pitfall 1.

### Pattern 2: Thread `policy.telemetry` through the SOLE read (D-03)

**What:** Extend `resolve_machine_egress_policy`'s return so the already-read `TelemetryConfig` surfaces without a second `read_machine_egress_policy()` call.

**Current signature** (confirmed):
```rust
// Source: crates/nono-cli/src/agent_daemon/mod.rs:352
pub(crate) fn resolve_machine_egress_policy(
    per_user_domains: &[String],
) -> nono::Result<(Vec<String>, bool)>
```
`MachineEgressPolicy.telemetry` is a **`TelemetryConfig` (not `Option`)** — `#[serde(default)]`, so an absent telemetry sub-section deserializes to `TelemetryConfig::default()` (default-ON). [VERIFIED: read crates/nono/src/machine_policy.rs:182]

**Recommended change** — extend the tuple to `(Vec<String>, bool, TelemetryConfig)`:
- `Some(policy)` branch → return `policy.telemetry` (already in hand).
- `None` branch → return `TelemetryConfig::default()` (matches the CLI's `Ok(None) => None → unwrap_or_default()` semantics at main.rs:175,120).
- `Err` is still propagated via `?` → daemon aborts fail-secure (preserved).

**Parity reference** (CLI):
```rust
// Source: crates/nono-cli/src/main.rs:173-178
let telemetry_config = match nono::read_machine_egress_policy() {
    Ok(Some(policy)) => Some(policy.telemetry),   // → Some(cfg)
    Ok(None)         => None,                      // → init_tracing unwrap_or_default()
    Err(_)           => None,                      // CLI swallows (telemetry non-fatal)
};
```
**Discrepancy flagged (see Open Questions Q1):** the CLI *swallows* the egress `Err` here because egress validity is enforced separately on the daemon path. The daemon's `resolve_machine_egress_policy` does the **opposite** — `Err → abort fail-secure`. D-03 explicitly says "Fail-secure posture from `resolve_machine_egress_policy` (Err → abort) is preserved." So threading telemetry through the existing return is correct and does NOT change the Err behavior. There is no conflict — just two intentionally different postures for two different code paths.

### Pattern 3: D-01 integration-style test (drive event → assert chain advanced)

The layer's only test-observable state is `pub(crate)` (`inner.chain.sequence`, `inner.chain.head`). The existing tests reach it via `layer.inner.lock()` from the **same module** (`telemetry/mod.rs` inline tests at line 436). Two viable mechanisms:

**(a) `tracing::subscriber::with_default` + the real proxy emit shape (closest to D-01's "integration-style"):**
```rust
// Drive a synthesized nono_security::network_deny event through a scoped subscriber
// whose ONLY layer is the SecurityEventLayer, then assert the chain advanced.
// Requires a pub(crate) accessor on SecurityEventLayer (see note below).
use tracing_subscriber::prelude::*;
let layer = SecurityEventLayer::new(TelemetryConfig::default(), "test-session".into());
// hold a handle to read sequence after — Arc the layer, or assert via accessor.
let subscriber = tracing_subscriber::registry().with(layer);
tracing::subscriber::with_default(subscriber, || {
    // EXACT shape emitted by nono_proxy::audit::log_network_deny (audit.rs:203):
    tracing::warn!(target: "nono_security::network_deny",
                   host = "blocked.example.com", port = 443u16,
                   agent_pid = std::process::id(), "network deny");
});
// assert the chain advanced (sequence == 1)
```
Problem: `with_default` *moves* the layer into the subscriber, so you cannot read `inner` afterward. Mitigation: add a small `pub(crate) fn chain_sequence(&self) -> u64` accessor and register the layer behind an `Arc` (the `Layer` impl is on `SecurityEventLayer`, and `Arc<L>: Layer` is provided by tracing-subscriber), so a clone of the `Arc` remains readable after the closure.

**(b) Direct `on_event` drive (simplest, fewest moving parts):**
```rust
// Construct the layer and call on_event directly with a hand-built Event.
// tracing does not expose a public Event constructor, so the with_default path
// (a) is the idiomatic way to manufacture a real Event. Prefer (a).
```
**Recommendation:** Use **(a)** — it exercises the real target/field shape (`nono_security::network_deny`, `host`/`port`/`agent_pid`) that the proxy actually emits, satisfying D-01's "drives a synthesized `nono_security::network_deny` tracing event … and asserts the event is actually processed (HMAC chain advances)." Add `pub(crate) fn chain_sequence(&self) -> u64` to `SecurityEventLayer` and register via `Arc`. The assertion is `sequence == 1` (genesis is 0; `advance_chain` does `saturating_add(1)`). [VERIFIED: read telemetry/mod.rs:139,309,453]

**Field/target shape the layer matches on** (confirmed by reading `on_event`):
- Target prefix gate: `event.metadata().target().starts_with("nono_security::")` (mod.rs:238)
- Sub-target → type: `…ends_with("network_deny")` → `NetworkDeny` (mod.rs:257)
- Fields visited: only `path` and `host` (string or debug). `port`/`agent_pid` are ignored by the visitor (mod.rs:340-354). For `network_deny`, `host` is present → emitted; `path` absent → no path hash. [VERIFIED: read telemetry/mod.rs:236-355]

### Anti-Patterns to Avoid
- **Refactoring/sharing `cli_bootstrap::init_tracing`** — D-02 forbids it. The daemon must not pull in the `Cli` arg type. Mirror `init_registry`'s *composition*, write fresh.
- **A second `read_machine_egress_policy()` call in the daemon** — violates Phase 83 D-04 SOLE-read. Thread the config through the existing return.
- **`.init()` in the daemon helper** — panics if a subscriber already exists; the foreground-fallback path means init could be reached twice. Use `try_init()` + a `OnceLock`/`Once` guard.
- **Asserting on the Windows ETW/Application-Log sink in the D-01 test** — that is the *host-gated* path (the `telemetry-event-emit` gate covers it). The non-host-gated test asserts the **in-process chain advance**, not OS log presence.
- **Putting the D-01 test in `crates/nono-cli/tests/`** — those integration tests link against the (nonexistent) nono-cli lib and cannot see `agent_daemon`/`telemetry` internals. The test MUST be inline `#[cfg(test)]` in the `#[path]`-included daemon tree.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HMAC chain / event scrub / severity filter | A new daemon telemetry pipeline | `SecurityEventLayer` (reuse verbatim) | Already platform-agnostic, tested, HMAC+zeroize+domain-separated (D-06). [VERIFIED: telemetry/mod.rs:189-326] |
| Registry composition | A bespoke subscriber stack | Mirror `init_registry` shape | Identical ETW-arm non-fatal logic already proven. |
| Verdict JSON persistence | Re-implement persistence in the doc/gate | `verify-dark.ps1` WR-04 owns it | Gates RETURN dicts; runner persists to `.nono-runtime/verdicts/<gate>.json`. [VERIFIED: verify-dark.ps1:52-79] |
| Telemetry config parsing | Re-parse HKLM in the daemon | `MachineEgressPolicy.telemetry` from the SOLE read | Library already deserializes it with `#[serde(default)]`. |

**Key insight:** DRAIN-04 is almost entirely a *reachability + registration* problem, not a logic problem. The hard parts (HMAC, scrubbing, ETW non-fatal, severity threshold, default-ON) are done. The work is making `SecurityEventLayer` visible to the standalone daemon binary and registering it once with the right `TelemetryConfig`.

## Runtime State Inventory

> Rename/refactor/migration phase? **No** — this is additive wiring + closeout. The standard 5-category inventory does not apply (no strings renamed, no datastores re-keyed). One adjacent runtime fact worth recording:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Live service config | The `nono-agentd` per-user Windows service (`HKCU\…\Services\nono-agentd`) holds a daemon process whose subscriber is set at process start. A code change requires the **service to be restarted** for the new subscriber to take effect (matches Phase 83 D-06 "restart-to-apply"). | None in code — operator note in `90-HUMAN-UAT.md`: restart `nono-agentd` after upgrade. |
| OS-registered state | The ETW "nono" provider + Application-Log "nono" source are registered by the Phase-82 MSI / first-run, not by this phase. | None — reused. |
| Stored / Secrets / Build artifacts | None — verified by reading the daemon binary; no new keys, env vars, or egg-info-class artifacts. | None. |

## Common Pitfalls

### Pitfall 1: Double subscriber init across the foreground-fallback path
**What goes wrong:** `run_service_mode()` calls `service_dispatcher::start`; on failure (dev/no-SCM) it **falls through to `run_foreground_mode()`**. If the helper uses `.init()` and is called in both, the second call panics ("a global default subscriber has already been set").
**Why it happens:** `tracing`'s global subscriber can be set exactly once per process.
**How to avoid:** Use `try_init()` (returns `Result`, ignore the err) AND/OR a `std::sync::Once`/`OnceLock` guard around the helper. Confirm `run_service` and `run_foreground_mode` each call the helper but the guard makes the second a no-op. [VERIFIED: read nono-agentd.rs:210-228 fall-through]
**Warning signs:** Panic at daemon startup in foreground mode; a `SetGlobalDefaultError` in logs.

### Pitfall 2: Telemetry module not reachable from the daemon binary
**What goes wrong:** Writing `use crate::telemetry::SecurityEventLayer;` in `agent_daemon` fails to compile — `telemetry` is declared in `main.rs` (the `nono` binary), not in `nono-agentd.rs`.
**Why it happens:** No lib target; modules are per-binary via `#[path]`.
**How to avoid:** Add `#[cfg(target_os = "windows")] #[path = "../telemetry/mod.rs"] mod telemetry;` (and its submodules resolve relatively: `event.rs`, `windows.rs`, `syslog.rs` sit beside `mod.rs`, so the path-include pulls the whole dir). Verify the telemetry module's internal `pub mod event; pub mod windows; pub mod syslog;` resolve under the new include path. [VERIFIED: read telemetry/mod.rs:32-34; nono-agentd.rs:41-43 uses the same #[path] idiom for agent_daemon]
**Warning signs:** `E0433: failed to resolve: use of undeclared crate or module telemetry`.

### Pitfall 3: Cross-target blind spot (CLAUDE.md MUST)
**What goes wrong:** The daemon init + ETW arm is `#[cfg(target_os = "windows")]`. A Windows-host `cargo check` compiles only the Windows branch; Linux/macOS clippy lanes (the ones CI runs) never see it — and the non-Windows `daemon_telemetry_init` branch (registry + `try_init`, no ETW) can rot undetected.
**Why it happens:** `nono-agentd.rs`'s real body is entirely `#[cfg(target_os = "windows")]`; the non-Windows arm is a 2-line stub (`eprintln!("nono-agentd is Windows-only"); exit(1)`). The telemetry module itself, however, **compiles on all platforms** (it's used by the `nono` binary cross-platform). [VERIFIED: read nono-agentd.rs:33-37; telemetry/mod.rs is platform-agnostic]
**How to avoid:** Run `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `--target x86_64-apple-darwin` per `.planning/templates/cross-target-verify-checklist.md`. **Mark PARTIAL→CI if the cross-toolchain C-linker is unavailable on the Windows host** (prior phases hit `x86_64-linux-gnu-gcc not found` on aws-lc-sys/ring). See Cross-target risk below.
**Warning signs:** Green Windows `cargo check`, red Linux/macOS CI clippy on the head SHA. This is the exact class that bit Phases 41/48/87/88.

### Pitfall 4: D-01 test relies on a Windows-only side-effect
**What goes wrong:** Asserting the test by checking `windows::emit_security_event` output makes the "non-host-gated" test Windows-only and unobservable.
**Why it happens:** The emit sink is `#[cfg(target_os = "windows")]`-gated and writes to ETW/Event Log.
**How to avoid:** Assert on the **in-process HMAC chain advance** (`sequence == 1`) via a `pub(crate)` accessor — platform-agnostic, runs on the Windows dev host AND in Linux/macOS CI. This is precisely what D-01 specifies ("HMAC chain advances / a `SecurityEvent` is built"). [VERIFIED: telemetry/mod.rs:309 advance_chain is called before the Windows emit at :324]

### Pitfall 5: PowerShell gate invocation swallows the exit code (MEMORY durable)
**What goes wrong:** `pwsh -Command "scripts/verify-dark.ps1 -Gate telemetry-event-emit"` collapses the gate's real exit N → 1, destroying the PASS/FAIL/SKIP distinction.
**How to avoid:** Invoke via `-File` or direct dot-path: `pwsh -File scripts/verify-dark.ps1 -Gate <name>`. The gate file header itself documents this rule. [VERIFIED: telemetry-event-emit.ps1:52-54; MEMORY project_v213_opened]

## Code Examples

### D-03: extend the SOLE-read return (the only egress-policy change)
```rust
// Source: crates/nono-cli/src/agent_daemon/mod.rs:352-398 (extend, do not duplicate the read)
pub(crate) fn resolve_machine_egress_policy(
    per_user_domains: &[String],
) -> nono::Result<(Vec<String>, bool, nono::TelemetryConfig)> {
    let machine_policy = nono::read_machine_egress_policy()?;   // SOLE read — unchanged
    match machine_policy {
        Some(policy) => {
            let telemetry = policy.telemetry.clone();           // already in hand (D-03)
            // … existing allowlist expansion …
            Ok((allowlist, true, telemetry))
        }
        None => Ok((per_user_domains.to_vec(), false, nono::TelemetryConfig::default())), // default-ON
    }
}
```
Both call sites (`run_service` line 157, `run_foreground_mode` line 258) destructure the 2-tuple today and MUST be updated to the 3-tuple, then pass the telemetry config into `daemon_telemetry_init(...)` **before** `build_daemon_state` starts the proxy. [VERIFIED: read nono-agentd.rs:157,258]

### The exact event the proxy emits (D-01 test target)
```rust
// Source: crates/nono-proxy/src/audit.rs:203-209 (in-process inside the daemon)
tracing::warn!(
    target: "nono_security::network_deny",
    host = host, port = port, agent_pid = std::process::id(),
    "network deny"
);
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Daemon registers NO subscriber → daemon-path `nono_security::*` events are dropped | Daemon registers `SecurityEventLayer` after policy resolution | Phase 90 (this) | Daemon-launched agent denials become observable telemetry. |
| CLI telemetry config was a permanent `None` (decorative) | CLI threads `policy.telemetry` (Phase 84 WR-01 / TELEM-04) | Phase 84/main.rs:173 | The daemon now mirrors that parity. |

**Deprecated/outdated:** The `nono-agentd.rs:74-78` commented-out "Wave 2 will wire in the full event-log infrastructure" block is stale TODO scaffolding (EventID constants) — not the path this phase takes (the `SecurityEventLayer` + ETW/Application-Log sink is the real telemetry path). Do not revive it.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A `#[path = "../telemetry/mod.rs"]`-include into `nono-agentd.rs` correctly pulls the telemetry submodules (`event.rs`/`windows.rs`/`syslog.rs`) since they sit beside `mod.rs`. | Pitfall 2 | If Rust resolves the path-included module's children relative to the *including* file, the submodule decls may need their own `#[path]`. Mitigation: planner adds a Wave-0 compile probe. Low risk — `agent_daemon` is already path-included with its own child modules (`accept_loop`, etc.) and compiles. |
| A2 | `Arc<SecurityEventLayer>` implements `Layer` so the D-01 test can register a clone and read `chain_sequence()` after `with_default`. | Pattern 3 | tracing-subscriber provides `impl<S, L: Layer<S>> Layer<S> for Arc<L>` — standard. If absent, fall back to a custom test-sink wrapper or a direct `advance_chain` probe. |
| A3 | Cross-toolchain C-linker (`x86_64-linux-gnu-gcc`) is unavailable on this Windows dev host (per prior-phase history), so DRAIN-04 cross-target clippy will be PARTIAL→CI. | Cross-target risk | If the toolchain IS present, the REQ can be fully VERIFIED instead of PARTIAL — strictly better. |

## Open Questions

1. **Egress `Err` posture parity** — The CLI swallows the HKLM read error (telemetry non-fatal); the daemon aborts on it (egress fail-secure). D-03 says preserve the daemon's abort. *Resolved:* threading telemetry through the existing return does not change `Err` behavior — no action needed beyond the tuple extension. Documented in Pattern 2.
2. **Helper location** (Claude's discretion per CONTEXT.md) — new `agent_daemon/telemetry_init.rs` (`#[path]`-included) vs an inline fn in `nono-agentd.rs`. *Recommendation:* a new `telemetry_init.rs` module beside `agent_daemon/mod.rs`, `#[path]`-included into `nono-agentd.rs`, so the D-01 inline test colocates with the helper and stays out of the binary's top-level file. Subject to D-02 (minimal, standalone, no `Cli`).
3. **`pub(crate)` accessor vs test colocated in telemetry/mod.rs** — Adding `chain_sequence()` to `SecurityEventLayer` is the cleanest cross-binary-visible assertion hook. Alternatively the D-01 assertion could live as an additional inline test in `telemetry/mod.rs` (same module sees `inner`). *Recommendation:* add the tiny `pub(crate) fn chain_sequence(&self) -> u64` accessor — it lets the daemon-side test (which is where D-01 wants it, exercising the daemon registration) assert without reaching private fields, and harms nothing.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust 1.82 toolchain (host = Windows) | DRAIN-04 compile/test | ✓ | 1.82 | — |
| `x86_64-unknown-linux-gnu` clippy target | Cross-target verify | ✗ (assumed, A3) | — | PARTIAL→CI per checklist |
| `x86_64-apple-darwin` clippy target | Cross-target verify | ✗ (assumed, A3) | — | PARTIAL→CI per checklist |
| `pwsh` (PowerShell 7) | DRAIN-01/02/03 gates | ✓ | host | — |
| Admin elevation | `telemetry-event-emit`, gates needing Event Log/logman | conditional | — | gate returns SKIP_HOST_UNAVAILABLE |
| Clean Win11 VM | `clean-host-install` live step | ✗ | — | SKIP_HOST_UNAVAILABLE → host-gated residual |
| Second host + kernel WFP | `wfp-egress-isolation` dual-layer | ✗ | — | SKIP_HOST_UNAVAILABLE → host-gated residual |
| Live SIEM ingestion | DRAIN-03 live step | ✗ | — | host-gated residual (operator) |

**Missing dependencies with no fallback (blocking):** none — every missing item maps to either PARTIAL→CI (cross-target) or SKIP_HOST_UNAVAILABLE (live gates), which is the intended drained-to-tech-debt outcome.

## DRAIN-01/02/03 Closeout Mechanics (scripted gates)

**Gate runner contract** (`scripts/verify-dark.ps1`, confirmed by full read):
- Invocation: `pwsh -File scripts/verify-dark.ps1 -Gate <name>` (single gate) or `-All`. **NEVER** `-Command "<bare path>"`.
- Each gate file dot-sources two functions: `Test-Precondition` (returns `$null` → run, or a reason string → `SKIP_HOST_UNAVAILABLE`) and `Invoke-Gate` (returns an ordered verdict dict `{gate,verdict,reason,detail,timestamp}` with `verdict ∈ {PASS,FAIL,SKIP_HOST_UNAVAILABLE}`).
- Exit-code mapping (single-gate): **PASS=0, FAIL=2, SKIP_HOST_UNAVAILABLE=3, harness-internal=4**.
- WR-04 persistence: runner writes `.nono-runtime/verdicts/<gate>.json` **before** emitting the stdout line. The `90-HUMAN-UAT.md` doc *references/summarizes* these — it does NOT re-implement persistence (D-05).
- `-All` emits a `{gates:[…],overall}` rollup with precedence `HARNESS_ERROR > FAIL > PASS_WITH_SKIPS > PASS`.
[VERIFIED: read scripts/verify-dark.ps1 in full]

**Gate→requirement map (D-06) and expected dev-host verdicts:**

| Req | Gate(s) | Expected on THIS dev host | Residual host-gated live step |
|-----|---------|---------------------------|-------------------------------|
| DRAIN-01 | `clean-host-install.ps1`, `deploy-silent-install.ps1` | likely SKIP_HOST_UNAVAILABLE (needs fresh Win11 VM) | Live silent-MSI install on a clean VM (operator). |
| DRAIN-02 | `wfp-egress-isolation.ps1`, `egress-policy-deny.ps1` | `egress-policy-deny` may PASS (proxy-layer, in-process); `wfp-egress-isolation` likely SKIP (needs admin + WFP service + 2nd host) | Live dual-layer (proxy+kernel WFP) block on a second host (operator). |
| DRAIN-03 | `telemetry-event-emit.ps1` | SKIP_HOST_UNAVAILABLE unless run **elevated** with `nono` on PATH (precondition requires admin for Get-WinEvent + logman) | Live SIEM ingestion of the emitted events (operator). |
[VERIFIED: read telemetry-event-emit.ps1 Test-Precondition:137-169 — requires Administrator role + (recent nono event OR nono on PATH)]

**`90-HUMAN-UAT.md` doc shape (D-05)** — mirror `88-HUMAN-UAT.md` (read, confirmed pattern):
- YAML frontmatter: `status` (partial/complete), `phase`, `source`, `started`, `updated`.
- `## Current Test` line.
- `## Tests` — one `###` block per gate/live-step with `expected:`, `why_human:` (the host-gated reason), `result:` (PASS/FAIL/SKIP verdict from the JSON, or `[pending]` for the operator-gated live step).
- `## Summary` — totals (total/passed/issues/pending/skipped/blocked).
- `## Gaps`.
Each Phase-90 entry records the **gate verdict captured on this host** plus the **residual live step that stays operator-gated**. [VERIFIED: read .planning/phases/88-…/88-HUMAN-UAT.md]

## Security Domain

This phase touches a security-critical telemetry path. Relevant controls (no new attack surface — wiring of an existing, audited layer):

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V7 Logging (security event integrity) | yes | HMAC-SHA256 chain with ephemeral zeroized key + domain separators (D-05/D-06), reused verbatim from `SecurityEventLayer`. |
| V8 Data Protection (no secret/path leakage) | yes | Path hashing (D-08) + `scrub_value` on free-text (D-10); `reason` deliberately NOT forwarded by the proxy emit. [VERIFIED: audit.rs:199-209; mod.rs:274-289] |
| V6 Cryptography | yes | `hmac`+`sha2`+`zeroize` — never hand-rolled. |

| Threat Pattern | STRIDE | Mitigation |
|----------------|--------|------------|
| Telemetry tamper / event forgery | Tampering / Repudiation | Per-session HMAC chain; head advances per event (D-01 asserts this). |
| Daemon fails open (no telemetry) on policy error | — | D-03 preserves `resolve_machine_egress_policy` Err→abort; telemetry absent→default-ON. |
| Sensitive path/secret in Event Log | Information disclosure | Path hash + scrub; `telemetry-event-emit` gate SC-3 asserts no raw paths. |

## Sources

### Primary (HIGH confidence — direct source reads)
- `crates/nono-cli/src/telemetry/mod.rs` — `SecurityEventLayer`, `on_event`, `advance_chain`, target/field matching, `pub(crate)` chain state
- `crates/nono-cli/src/cli_bootstrap.rs:107-217` — `init_tracing`/`init_registry` composition + ETW non-fatal arm (D-02 mirror reference)
- `crates/nono-cli/src/bin/nono-agentd.rs` — standalone binary, `run_service`/`run_foreground_mode`, SOLE-read call sites, foreground fallback, non-Windows stub
- `crates/nono-cli/src/agent_daemon/mod.rs:328-398` — `resolve_machine_egress_policy` (2-tuple return, standalone-binary constraints, inline test module)
- `crates/nono-cli/src/main.rs:158-178` — CLI telemetry-config parity (`Ok(Some)→Some(policy.telemetry)`, `None`, `Err`→swallow)
- `crates/nono-proxy/src/audit.rs:194-209` — exact `nono_security::network_deny` in-process emit shape
- `crates/nono/src/machine_policy.rs:102-210,524-536` — `TelemetryConfig` (non-Option, serde default), `MachineEgressPolicy.telemetry`, `read_machine_egress_policy` signature + non-Windows stub
- `scripts/verify-dark.ps1` (full) — verdict classes, exit mapping, WR-04 persist, `-Gate`/`-All`
- `scripts/gates/telemetry-event-emit.ps1` (full) — DRAIN-03 gate contract, admin precondition, invocation rule
- `.planning/phases/88-…/88-HUMAN-UAT.md` — D-05 doc-shape reference
- `.planning/templates/cross-target-verify-checklist.md` — PARTIAL→CI disposition
- `CLAUDE.md` — cross-target MUST, unwrap policy, library/CLI boundary

### Secondary / Project Constraints (from CLAUDE.md)
- Strictly no `.unwrap()`/`.expect()` in production (`clippy::unwrap_used`) — the helper's `try_init`/match-fallbacks must comply.
- DCO sign-off on every commit.
- Cross-target clippy MUST for any cfg-gated Unix-touching change (the non-Windows daemon-init arm qualifies).
- Library stays policy-free; the daemon (CLI tier) owns policy threading.

## Metadata

**Confidence breakdown:**
- DRAIN-04 architecture & event shape: HIGH — every type, target, and field read directly from source.
- DRAIN-04 test mechanism: HIGH for the assertion target (chain sequence); MEDIUM on the exact `Arc<Layer>`/accessor ergonomics (A2) — a Wave-0 compile probe will confirm.
- DRAIN-01/02/03 gate mechanics: HIGH — runner and one gate read in full; remaining gates inferred from the identical dot-source contract documented in `telemetry-event-emit.ps1`.
- Cross-target disposition: MEDIUM — assumes toolchain absence (A3) consistent with prior phases; verify at execution.

**Research date:** 2026-06-20
**Valid until:** ~2026-07-20 (stable in-repo code; only invalidated by edits to telemetry/mod.rs, nono-agentd.rs, or the gate scripts)
