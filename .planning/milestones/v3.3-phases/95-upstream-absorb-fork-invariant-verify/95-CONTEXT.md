# Phase 95: Upstream Absorb + Fork-Invariant Verify - Context

**Gathered:** 2026-06-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb the **will-sync** and **split** clusters classified by the Phase 94
`94-DIVERGENCE-LEDGER.md` (the `nolabs-ai/nono` `v0.64.0..v0.65.1` window) into the
fork — cherry-picked with the `-x` trailer (or manual-replayed) and DCO-signed — then
**prove the Windows security model is unregressed** post-sync.

In scope (locked by the ledger routing — NOT re-litigated here):
- **Cluster A** (`9ce74e92`, will-sync): AF_UNIX mediation deadlock fix (4 bugs incl. dup2 bypass) on the Phase 87 SEC-01 code path.
- **Cluster B** (`11fd10e0`, tool-sandbox #1105, split): shared-surface extraction only (see D-01).
- **Cluster C** (`9b37dc52`, split): preserve the Phase 89 fork divergence, apply only the credentials_intent fix (see D-02).
- **UPST10-03 fork-invariant verification:** AppContainer/WFP/broker Windows backend, the ADR-86 audit/diagnostics library-boundary carve-out, and the `exec_strategy_windows/` denial-rendering fork — one explicit checklist entry per invariant.

Out of scope (deferred — see `<deferred>`):
- **Cluster D** (5 commits, won't-sync) release metadata + leapfrog floor → **Phase 97**.
- The cross-target clippy *toolchain stand-up* → **Phase 96** (Phase 95 only records the PARTIAL→96 deferral; see D-03).
- The full tool-sandbox subsystem (the parts of #1105 skipped by the shared-surface split).

</domain>

<decisions>
## Implementation Decisions

### Cluster B — tool-sandbox split depth
- **D-01:** **Shared-surface extraction only.** Extract the additive `crates/nono/src/audit.rs`
  event-type definitions (CR-02 carve-out — additive-only, MUST NOT touch `records_verified:
  event_count > 0`) plus the proxy-surface hunks from #1105 that apply cleanly. **SKIP** the
  tool-sandbox subsystem directory (absent in the fork) and **SKIP** the `tls_intercept/` hunks
  (the fork has no `crates/nono-proxy/src/tls_intercept/` dir — Cluster F carve-out). Taking the
  whole tool-sandbox feature would be a new capability and belongs in its own future phase, not here.

### Cluster C — Phase 89 fail-secure divergence conflict
- **D-02:** **Preserve fork divergence; apply the bug fix only.** Upstream `9b37dc52` reverses the
  fork's deliberate Phase 89 fail-secure proxy-activation behavior. Keep the fork's expression and
  the `proxy_activates_with_custom_credentials_only` guard test as a regression sentinel; apply
  ONLY the `credentials_intent` bug-fix block from the upstream commit. Do NOT adopt upstream's
  explicit-activation refactor wholesale.

### Cross-target clippy sequencing
- **D-03:** **Land in Phase 95, defer cross-target clippy to Phase 96.** Cherry-pick Clusters A & B
  in this phase; gate on **native Windows clippy + `make build` + `make test`** at land time. The
  cfg(linux)/cfg(macos) cross-target clippy gates (`x86_64-unknown-linux-gnu`,
  `x86_64-apple-darwin`) run in **Phase 96** against the synced tree and are recorded **PARTIAL→96**
  per `.planning/templates/cross-target-verify-checklist.md`. Every will-sync/split commit touching
  a cfg-gated Unix block carries the ledger's "Phase 95 cross-target clippy note" forward into the
  Phase 96 checklist. (Matches the established fork pattern — do not invert the 95→96 roadmap order.)

### Post-sync test gate strictness
- **D-04:** **No-NEW-failures vs documented baseline.** SC2 (`make test` green on Windows) passes if
  the cherry-picks introduce **zero new failures** relative to the known ~5-red Windows baseline
  (`nono-cli` profile_cmd + 3 protected_paths + the `nono` lib `try_set_mandatory_label`). Do NOT
  expand scope to fix the pre-existing baseline reds in this phase. The planner MUST capture the
  baseline red set at the phase-base commit BEFORE any cherry-pick, so "new" is provable.

### Claude's Discretion
- **Cluster D `sigstore-verify` dep-bump evaluation:** The ledger routed the `sigstore-verify`
  dependency evaluation to "Phase 95 DEPS" (the *version/release* metadata of Cluster D stays in
  Phase 97). The planner decides whether to absorb the `sigstore-verify` dep bump in this phase as
  dependency/security hygiene, or fold it into Phase 97 with the rest of Cluster D — guided by D-04
  (no-new-failures) and whether the bump is build-clean on the Windows host.
- Cherry-pick mechanics (`git cherry-pick -x` vs manual replay per commit), commit ordering, and
  plan/wave decomposition are the planner's call, subject to the locked decisions above.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The absorb contract (READ FIRST)
- `.planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md` — THE input contract: cluster
  routing (A/B/C/D), per-commit tables (sha · files-changed · windows-touch), the **Carve-out Re-touch
  Check** (CR-02 HIT additive-only, CR-01 clean, Cluster F HIT preserve-fork), the ADR per-cluster risk
  matrix, and the **Downstream routing** block that scopes this phase.

### Fork-invariant carve-outs (UPST10-03 — MUST stay unregressed)
- `proj/ADR-86-library-boundary-convergence.md` — the audit/diagnostics library-boundary carve-out;
  this window has NO library-boundary change, so the planner verifies it is *untouched*.
- `proj/ADR-87-cr02-audit-bypass.md` — CR-02 `crates/nono/src/audit.rs` `records_verified: event_count
  > 0` invariant; Cluster B's audit.rs hit MUST remain additive-only against this.

### Requirements + protocol
- `.planning/REQUIREMENTS.md` — UPST10-02 (absorb will-sync clusters, DCO) and UPST10-03 (fork-invariant
  verify + Windows `make build`/`make test` green).
- `CLAUDE.md` §Coding Standards — cross-target clippy MUST/NEVER rule (D-03), DCO sign-off form, the
  unwrap/panic/path-security rules every cherry-pick must satisfy.
- `.planning/templates/cross-target-verify-checklist.md` — the PARTIAL→CI deferral protocol that D-03
  routes the linux-gnu/apple-darwin clippy gates through (consumed by Phase 96).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/nono/src/audit.rs` — CR-02 carve-out; Cluster B's additive event types land here. Keep
  `records_verified: event_count > 0` byte-intact.
- `proxy_activates_with_custom_credentials_only` guard test (nono-proxy / proxy runtime tests) — the
  Phase 89 regression sentinel D-02 preserves.
- The AF_UNIX pathname-mediation code path (Phase 87 SEC-01, `crates/nono/src` supervisor IPC) —
  Cluster A's cherry-pick target; the fork already diverged here, so expect conflict resolution.

### Established Patterns
- Cherry-pick with `-x` + manual DCO trailer (`Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`)
  on every absorbed commit — the fork convention used in Phases 86–89.
- Fork-preserve invariants win any conflict with upstream (fail-secure; Windows model is non-negotiable).
- `make ci` (clippy + fmt + tests) before any push — clippy-green ≠ rustfmt-clean (fmt-check is part of
  the gate).

### Integration Points
- Cluster F carve-out paths — `crates/nono-proxy/src/{route,connect,reverse,server}.rs` +
  `crates/nono-cli/src/proxy_runtime.rs`; `tls_intercept/` is ABSENT (D-01 skips its hunks).
- `exec_strategy_windows/` denial-rendering fork (ADR-86 D-02 carve-out) — UPST10-03 verifies it is
  untouched by the sync.

</code_context>

<specifics>
## Specific Ideas

- All four discussed decisions resolved to the ledger's recommended default — the fork owner confirmed
  the conservative, tightly-scoped absorb (preserve divergence, defer cross-target to 96, no-new-failures
  gate). The planner should treat the ledger routing as authoritative and not re-open cluster dispositions.

</specifics>

<deferred>
## Deferred Ideas

- **Cluster D** (release metadata: v0.64.1/v0.65.0/v0.65.1 + leapfrog floor ≥ 0.65.0) → **Phase 97**
  (won't-sync; version bump after sync).
- **Full tool-sandbox subsystem absorb** (the #1105 hunks skipped by the D-01 shared-surface split —
  tool-sandbox dir, tls_intercept/) → future UPST cycle / its own phase if ever desired.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` — MSI VC++ x64 runtime prereq. Distribution/host-gated concern
  (FUT-03), not absorb/verify scope. Deferred.
- `20260611-poc-cert-broker-clean-host.md` — POC-cert broker trust on a clean host. Distribution/host-gated
  (FUT-03), not absorb/verify scope. Deferred.

</deferred>

---

*Phase: 95-Upstream Absorb + Fork-Invariant Verify*
*Context gathered: 2026-06-26*
