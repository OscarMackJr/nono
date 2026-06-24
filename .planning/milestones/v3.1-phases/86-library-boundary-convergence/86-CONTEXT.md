# Phase 86: Library-Boundary Convergence - Context

**Gathered:** 2026-06-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Relocate the audit/attestation/ledger stack and adopt the structured-diagnostics model
into the **core `nono` crate** (matching upstream `always-further/nono` `v0.62.0..v0.64.0`),
reducing `nono-cli` to thin wrappers, exposing diagnostics via FFI, reconciling the fork's
Windows diagnostic paths + the proxy `ProxyDiagnostic` surface, and recording the
boundary-convergence decision in an ADR + updated `CLAUDE.md`.

This phase **deliberately changes the policy-free-library invariant** (D-03 from Phase 85:
themes A & B locked `will-sync / adopt-upstream`). It absorbs exactly the **8 commits** the
DIVERGENCE-LEDGER assigned to Phase 86:
- **Theme A (audit → core):** `a5b2a516`, `aed35bec`, `0b27cfc2`, `e9529312` (~1773 LOC moved
  CLI-side → `crates/nono/src/audit.rs`).
- **Theme B (diagnostics → core + FFI):** `4ad8ba92`, `f867aba2`, `a6aa5995`, `7f319b9e`
  (single-file core `diagnostic.rs` → module dir; `NonoError::{diagnostic_code, remediation}`;
  net-new `bindings/c/src/diagnostic.rs` + `crates/nono-proxy/src/diagnostic.rs`).

**Out of scope (belongs to later phases):** theme C security fix (Phase 87); themes
D/E/G/H/I/J/K/L/M + dep bumps (Phase 88); theme F proxy hardening cluster (Phase 89); the
crate-version leapfrog to ≥ `0.65.0` (release-time). New capabilities of any kind.

</domain>

<decisions>
## Implementation Decisions

### Port mechanism (how the 8 commits land)
- **D-01:** **Cherry-pick + resolve conflicts.** `git cherry-pick` the 8 upstream commits in
  the locked ledger order (A: `a5b2a516` first, then `aed35bec`/`0b27cfc2`/`e9529312`;
  B: `4ad8ba92` → `f867aba2` → `a6aa5995` → `7f319b9e`), resolving each conflict against fork
  divergence. **Rationale:** preserves upstream commit provenance/messages (auditable lineage
  for the next sync), and the ledger already mapped the exact conflict surfaces. Expect heavy
  conflict churn on B (fork has a single-file core `diagnostic.rs`; fork has the
  `exec_strategy_windows/` consumers upstream lacks). The conflict-resolution target on shared
  surfaces is **upstream's end-state** (see D-03); the Windows conflict hunks get the
  preserve-and-bridge resolution (see D-02).
- **D-01a:** Honor the intra-B ordering constraint — `a6aa5995` (adds the diagnostic module +
  the 3 FFI fns + `ProxyDiagnostic*`) must land before any consumer referencing
  `nono::NonoRemediation`, `ProxyDiagnostic*`, or the FFI diagnostic-code fns. `4ad8ba92` first
  (migrates the existing diagnostic surface out of core — prerequisite for the new module layout).

### Windows diagnostic reconciliation (SC#3 — no regression)
- **D-02:** **Preserve-and-bridge (conservative).** Keep the fork's Windows diagnostic paths
  (`crates/nono-cli/src/exec_strategy_windows/{launch,network,mod}.rs` and the fork's existing
  core `diagnostic.rs` content where Windows depends on it) intact. Bridge them to the new
  structured model only **at the surface** — populate `diagnostic_code` / `remediation` so the
  FFI surface and the new model are satisfied — **without** rewriting the Windows
  denial-rendering logic. **Rationale:** lowest regression risk on the milestone's focus
  platform; SC#3 requires *no regression in Windows denial output*, not Windows convergence.
  Accept some short-term duplication between the bridged Windows path and the new core model.

### CLI boundary line (BND-03 ADR substance)
- **D-03:** **Match upstream's line exactly.** Move into `crates/nono/src/audit.rs` /
  `crates/nono/src/diagnostic/*` whatever upstream moved; leave CLI-side exactly what upstream
  leaves (audit command UX, argument parsing, file-path resolution, output rendering). The
  BND-03 ADR documents **upstream's line verbatim** as the new fork boundary, plus the
  *single* deliberate fork carve-out that D-02 introduces (Windows denial paths stay CLI-side,
  bridged). **Rationale:** least judgment, maximum convergence → smallest reconciliation surface
  for the *next* upstream sync (the whole point of adopting upstream's boundary).

### Test relocation (SC#1 — existing audit behavior preserved)
- **D-04:** **Move tests to core with the code.** Unit tests for the relocated audit logic
  (recorder lifecycle, merkle / inclusion-proof, ledger append+verify, attestation sign/verify)
  move into `crates/nono/src/audit.rs` alongside the moved code (matching upstream); `nono-cli`
  keeps only thin wrapper / integration tests. **Rationale:** tests live with their
  unit-under-test and prove core behavior directly; mirrors upstream's test placement, reducing
  future-sync friction.

### Verification gate (durable lesson — non-negotiable)
- **D-05:** The build/clippy verification gate for this phase **MUST** be
  `--workspace --all-targets` (not `--bin nono`). The FFI diagnostic surface adds a
  `NonoDiagnosticCode` repr-C enum + exhaustive `match` arms; a `--bin nono`-scoped gate
  **hides** `nono-ffi` E0004 exhaustive-match breaks (durable v3.0 lesson). The 3 new
  `pub extern "C"` fns (`nono_last_diagnostic_code`, `nono_last_remediation_json`,
  `nono_session_diagnostic_report_to_json`) must build clean with all arms covered.

### Claude's Discretion
- Exact conflict-resolution tactics per cherry-pick hunk (the ledger maps the surfaces; the
  executor resolves). Whether to squash or retain the 8 cherry-picked commits in the fork's
  history — provenance is the goal (D-01), but final commit shape is the executor's call within
  the fork's DCO sign-off convention.
- ADR file location/format (follow the fork's existing ADR convention; record in the phase dir
  and/or `proj/` per house style).
- Precise shape of the surface-level Windows bridge (which field maps from which existing
  Windows denial value) — research/planner to map file-by-file.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents (researcher, planner, executor) MUST read these before planning or implementing.**

### The gating audit (the literal input contract for this phase)
- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` — **primary.**
  § "Cluster A" + § "Cluster B" carry the per-commit SHA inventories, the cherry-pick ordering
  constraint (4ad8ba92 → … → a6aa5995), the actual-diff re-export findings, and the
  Pitfall-3 FFI-surface closure (the exact 3 `pub extern "C"` fns + `NonoDiagnosticCode` enum).
- `.planning/phases/85-upst9-divergence-audit/85-CONTEXT.md` — D-03 (A & B locked
  `will-sync / adopt-upstream`) + the verified fork-vs-upstream code state for A & B.
- `.planning/seeds/SEED-006-upst9-v0.62-v0.64-sync-window.md` — per-theme new/modified function
  inventory + upstream SHAs (the worksheet behind the ledger's A & B rows).

### Milestone framing
- `.planning/ROADMAP.md` § Phase 86 — Goal + 4 success criteria (the acceptance bar).
- `.planning/REQUIREMENTS.md` — BND-01 (audit → core, thin CLI wrappers, tested),
  BND-02 (structured diagnostics → core + FFI, reconciled w/ Windows + proxy `ProxyDiagnostic`),
  BND-03 (`CLAUDE.md` boundary update + boundary-convergence ADR).
- `.planning/STATE.md` § Accumulated Context — locked v3.1 milestone decisions + phase sequencing.

### The library-boundary invariant being changed (BND-03 substance)
- `CLAUDE.md` § "Library vs CLI Boundary" — the policy-free-library table that A & B
  deliberately change; **must be updated** in this phase to document the new core-crate audit +
  diagnostics modules (BND-03).
- `CLAUDE.md` § Coding Standards — Cross-target clippy MUST/NEVER rule + the
  `--workspace --all-targets` gate discipline (D-05).
- `proj/DESIGN-library.md` — library architecture / workspace layout (the boundary doc the ADR
  must reconcile against).

### Fork hazards to honor
- `windows_hook_interpreter_spawn_gotchas` (memory) — the fork's Windows diagnostic/CLR baseline
  context behind the preserve-and-bridge decision (D-02).
- `feedback_cluster_isolation_invalid` (memory) — why the re-export ordering in D-01a matters.
- `project_v30_opened` (memory) — the **durable** `--bin nono` gate lesson that hid the nono-ffi
  E0004 exhaustive-match break (D-05); the gate MUST be `--workspace --all-targets`.
- `feedback_clippy_cross_target` (memory) — Windows-host clippy cannot catch cfg-gated Unix
  drift; relevant if any cherry-pick hunk touches Unix cfg-gated code.

</canonical_refs>

<code_context>
## Existing Code Insights

### Fork starting state (verified at discuss-time)
- **Audit is CLI-side, no core module:** `crates/nono-cli/src/audit_attestation.rs`,
  `audit_commands.rs`, `audit_integrity.rs`, `audit_session.rs`. **No** `crates/nono/src/audit.rs`
  exists. Theme A creates it and reduces these 4 files toward thin wrappers.
- **Diagnostics are split, single-file core:** fork has `crates/nono/src/diagnostic.rs` (single
  file, `DiagnosticFormatter`) AND `crates/nono-cli/src/diagnostic_formatter.rs`. Upstream B
  replaces the core file with a **module dir** (`codes.rs`, `observation.rs`, `records.rs`,
  `report.rs`, `detail.rs`, `mod.rs`) + adds `NonoError::{diagnostic_code, remediation}` in
  `error.rs`. **This is the heaviest cherry-pick conflict surface.**
- **Net-new diagnostic surfaces:** `crates/nono-proxy/src/diagnostic.rs` (`ProxyDiagnostic`,
  `ProxyDiagnosticCode`, `ProxyDiagnosticSeverity`) and `bindings/c/src/diagnostic.rs` (3 FFI
  fns) do **not** exist in the fork — both arrive whole with `a6aa5995`.
- **FFI files today:** `bindings/c/src/{capability_set,fs_capability,lib,query,sandbox,state,types}.rs`.
  `types.rs` gains the `NonoDiagnosticCode` repr-C enum; new `diagnostic.rs` is added.
- **Windows diagnostic consumers (preserve-and-bridge targets, D-02):**
  `crates/nono-cli/src/exec_strategy_windows/{launch,network,mod}.rs`.

### Established patterns
- 5-crate workspace; internal path-dep `version` pins must stay synced across all 5 `Cargo.toml`
  files (`project_workspace_crates`) — relevant only if A/B touch crate manifests/versions.
- All commits require DCO sign-off (`Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`);
  cherry-picks must carry/append it.

### Integration points
- This phase's output (audit + diagnostics in core) is a **hard dependency** for downstream:
  Phase 87 (`capability.rs` / error surface), Phase 88 (features ride the reconciled error
  surface; XDG state touches audit roots), Phase 89 (proxy `ProxyDiagnostic` surface). Get the
  core surface right here.

</code_context>

<specifics>
## Specific Ideas

- The conflict-resolution "answer key" is upstream's end-state — when a cherry-pick conflict
  arises on a shared surface, resolve toward what upstream's tree looks like at that commit (the
  ledger documents the exact files per commit). The only sanctioned deviation is the Windows
  denial path (D-02).
- The BND-03 ADR should explicitly state the *one* fork carve-out from upstream's boundary line
  (Windows denial paths stay CLI-side, bridged) so future syncs know it's deliberate, not drift.

</specifics>

<deferred>
## Deferred Ideas

- **Full Windows convergence** (routing Windows denial output end-to-end through the new core
  structured-diagnostics model) — deliberately *not* done here (D-02 = bridge only). A future
  phase could converge the Windows path once the bridge has proven stable. Captured so it isn't
  lost.
- Theme C / D–M / F absorption and the crate-version leapfrog — Phases 87/88/89 + release-time,
  per the DIVERGENCE-LEDGER dispositions.

### Reviewed Todos (not folded)
Four todos surfaced via `todo.match-phase 86`, all weak generic-keyword matches (score ≤ 0.6 on
"phase"/"must"/"clean"/"code") and none related to library-boundary convergence — **none folded**
(folding would be scope creep). Same set Phase 85 reviewed and rejected:
- `20260611-msi-vcredist-prereq.md` — MSI VC++ runtime prereq (host-deploy debt).
- `20260611-poc-cert-broker-clean-host.md` — untrusted-POC-cert broker on clean host.
- `20260618-phase83-codereview-deferred.md` — deferred Phase 83 code-review.
- `20260612-macos-rlimit-as-setrlimit-fails.md` — macOS RLIMIT_AS/setrlimit defect.

</deferred>

---

*Phase: 86-library-boundary-convergence*
*Context gathered: 2026-06-19*
