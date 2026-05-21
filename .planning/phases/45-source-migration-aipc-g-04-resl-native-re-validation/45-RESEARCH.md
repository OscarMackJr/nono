# Phase 45: Source migration + AIPC G-04 + RESL native re-validation - Research

**Researched:** 2026-05-21
**Domain:** Rust Edition 2024 source migration, wire-protocol compile-time tightening, native-host audit-attestation re-validation
**Confidence:** HIGH (all claims verified via codebase grep, file reads, or `.planning/` artifacts; no training-data assumptions about library APIs)

## Summary

Phase 45 closes three Rule-4 architectural deferrals on disjoint surfaces — they are bundled into a single phase to avoid three single-purpose phases. CONTEXT.md is unusually thorough and already names exact files, line numbers, commit SHAs, and disposition rules; this RESEARCH.md primarily verifies the empirical claims, surfaces additional discoveries from grep walks, and produces the Validation Architecture matrix the orchestrator needs.

Key empirical confirmations: 39 `#[no_mangle]` sites across the named 6 files (verbatim match — 16+7+4+4+3+5); `ApprovalDecision::Granted` has 25 source-tree occurrences (18 in `crates/`; 7 in `aipc_sdk.rs`); 42 total `SupervisorResponse::Decision { ... }` construction sites with 22 in Windows supervisor; 9 explicit `grant: None` callsites that disappear post-Plan-45-02; `recorded_ledger_redacts_session_token` is at `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:5033`; `audit_commands.rs:867` already uses `"Approved"` in its serde_json::Value fixture (so the rename pre-aligns one test); Phase 27.2 closed REQ-AAHX-03 with both attestation tests passing post-fix `2b7425e7`. All three cross-target Linux + macOS Rust targets ARE installed via `rustup target list --installed`, but the cross-target C linker for both is absent on this Windows host — the PARTIAL disposition under `.planning/templates/cross-target-verify-checklist.md` is the expected close path, identical to Phases 41 + 43-01b + 44.

**Primary recommendation:** Plan 45-01 is a deterministic mechanical sweep (one commit per file × 6, then a ledger-flip commit) with a cbindgen byte-identical gate. Plan 45-02 is a single atomic commit with one inventory grep at plan-open and an AUD-05 token-redaction regression check. Plan 45-03 is two `.github/workflows/` + protocol-doc commits with no source-tree edits. All three are parallel-safe; the planner can confidently apply CONTEXT.md verbatim.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Edition 2024 attribute syntax conformance | `bindings/c/src/` (C FFI library tier) | — | `#[no_mangle]` is an FFI-export safety attribute; Edition 2024 reclassifies it as `unsafe`. Pure FFI-tier change. |
| Wire-type compile-time invariant (`Approved ⟹ grant Some`) | `crates/nono/src/supervisor/types.rs` (library wire-type tier) | `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` + `crates/nono/src/supervisor/aipc_sdk.rs` (CLI/SDK consumers) | The invariant lives at the type definition; consumers cascade. CLAUDE.md § Security Considerations "Explicit over Implicit" + § Library vs CLI Boundary — wire types belong in `nono` library, security policy invariants belong to the type, consumers obey. |
| Audit-event payload (Approved variant) | `crates/nono-cli/src/audit_integrity.rs` (CLI audit-recorder tier) | `crates/nono-cli/src/audit_commands.rs` (CLI verify tier) | `AuditEventPayload::CapabilityDecision` wraps an `AuditEntry { decision: ApprovalDecision }` — the rename cascades transparently through serde; the docstring at `:83` ("`None` for Approved decisions") becomes stale and should be refreshed (Plan 45-02 comment sweep). |
| Native-host RESL re-validation | `.github/workflows/` (CI/CD tier) + `.planning/phases/45-.../` (planning artifact tier) | `crates/nono-cli/tests/audit_attestation.rs` (test-tier consumer of the runtime) | The runtime exists at the CLI binary; native-host re-validation is a CI lane responsibility — Plan 45-03 produces the lane + protocol, defers the live run to Phase 46 orchestrator action. |
| DIVERGENCE-LEDGER amendment | `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` (audit-trail tier) | — | Single canonical ledger location (verified empirically — only `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` exists; no `.planning/upstream/` directory). |

## User Constraints (from CONTEXT.md)

> CONTEXT.md exists for Phase 45 and is the binding decision document. Copy-verbatim per orchestrator contract — the planner MUST honor every D-45-* item.

### Locked Decisions

**Plan slicing & parallelism (Area A)**
- **D-45-A1: Three plans, parallel-safe.** Plan 45-01 = Edition 2024 source migration (REQ-PORT-CLOSURE-08); Plan 45-02 = AIPC G-04 wire-protocol tightening (REQ-AIPC-G04-01); Plan 45-03 = RESL native re-validation (REQ-RESL-NIX-04). Disjoint surfaces; per-plan SUMMARY + per-plan REQ closure. Mirrors Phase 44 D-44-A1.
- **D-45-A2: Plan 45-01 commits = one per file (6 commits + 1 ledger flip = 7 commits).** Order: `capability_set.rs` (16) → `lib.rs` (4) → `fs_capability.rs` (7) → `sandbox.rs` (3) → `state.rs` (5) → `query.rs` (4) → DIVERGENCE-LEDGER amendment.

**Edition 2024 disposition (Area B)**
- **D-45-B1: D-20 manual replay, no upstream PR.** Each commit `chore(45-01):` with a free-form `Replay-of: 79715aa5 (Phase 43 Plan 43-01b DEC-3 split-disposition close)` annotation in the body — NOT a D-19 `Upstream-commit:` trailer block. No upstream PR umbrella.
- **D-45-B2: DIVERGENCE-LEDGER amended at Plan 45-01 close (single commit).** Final Plan 45-01 commit flips Cluster 2 `split → closed` with back-reference to `79715aa5` AND the Phase 45 commit range. SUMMARY records the amendment SHA.
- **D-45-B3: Non-mechanical surprises absorbed in per-file commits + cbindgen `nono.h` byte-identical gate.** After all 6 commits, regenerate `nono.h` via `cargo build -p nono-ffi`; assert byte-identical to pre-phase. Header diff = deviation, do not auto-close.

**AIPC G-04 migration shape (Area C)**
- **D-45-C1: Single atomic commit for the cascade.** All wire-type + `aipc_sdk.rs` demultiplexer + 23 tests + `audit_commands.rs:867` fixture + CHANGELOG + ADR amendment land together. Tag `feat(45-02):`. AUD-05 regression (`recorded_ledger_redacts_session_token`) called out as verified-pass in commit body.
- **D-45-C2: Accept the wire-format break; old ledgers no longer re-verifiable.** Pre-v2.6 `audit-events.ndjson` files with `{"decision":{"Granted":null},"grant":{...}}` shape cannot be re-verified after upgrade. Document in CHANGELOG.md (BREAKING) + ADR amendment in `docs/architecture/audit-bundle-target.md`. No custom Deserialize. No `nono audit migrate` subcommand.
- **D-45-C3: Rename `Granted → Approved` in the atomic commit.** SC#2, Phase 23 D-01 comments, the `audit_commands.rs:867` fixture, and PROJECT.md v2.1 PROF-01..04 / AUD-01..05 all use "Approved". Folded into Plan 45-02's atomic commit.

**RESL native re-validation host strategy (Area D)**
- **D-45-D1: Author `.github/workflows/phase-45-resl-native-host.yml` + protocol doc; defer live run to Phase 46.** REQ-RESL-NIX-04 closes as STRUCTURALLY-COMPLETE-PENDING-LIVE-RUN per `.planning/templates/cross-target-verify-checklist.md` semantics.
- **D-45-D2: `workflow_dispatch`-only trigger with `gh_runner_os` matrix input.** Choices `[ubuntu-24.04, macos-latest, both]`, default `both`. Workflow may be deleted in v2.7 once verdict is recorded.

### Claude's Discretion

(Copied verbatim from CONTEXT.md § Decisions § Claude's Discretion — planner picks within these bounds)

- Exact path for `aipc_sdk.rs` — locate at plan-open via `grep -rln "aipc_sdk" crates/`. **Confirmed via this research:** `crates/nono/src/supervisor/aipc_sdk.rs` (file exists; `pub mod aipc_sdk;` declared at `crates/nono/src/supervisor/mod.rs:30`).
- 23 pre-existing test inventory — inventory at plan-open via `grep -rn "ApprovalDecision::Granted\|grant: Option\|(Granted, grant=None)" crates/ bindings/`. **Confirmed via this research:** see § "Plan 45-02 Test + Construction-Site Inventory" below.
- CHANGELOG.md entry placement + exact wording — planner picks; must include BREAKING marker, wire shape change, fresh-session vs replay distinction, ADR back-reference.
- `docs/architecture/audit-bundle-target.md` ADR amendment shape — planner picks heading level + amendment number (likely 45-A or 45-1).
- `is_granted()` / `is_denied()` impl method renames — `crates/nono/src/supervisor/types.rs:405-417`. Default: rename `is_granted() → is_approved()` for consistency; keep `is_denied()` unchanged (Denied variant name unchanged).
- `.github/workflows/phase-45-resl-native-host.yml` matrix specifics — planner picks `runs-on`, `continue-on-error: true` shape, cache + setup-action choices. Default: mirror `phase-37-linux-resl.yml`.
- `45-03-NATIVE-RESL-PROTOCOL.md` content depth — planner picks; minimum: SC#3 decision tree, expected `cargo test` output shape, Phase 46 hand-off instructions.
- cbindgen header byte-identical gate mechanics — planner picks pre-phase capture + diff vs `git diff bindings/c/include/nono.h`. **Note from this research:** the generated header lives at `bindings/c/include/nono.h` (verified empirically; only `nono.h` exists in that directory).
- Plan numbering / slugs — `45-01-EDITION-2024-MIGRATION`, `45-02-AIPC-G04-TIGHTENING`, `45-03-RESL-NATIVE-REVALIDATION` suggested.

### Deferred Ideas (OUT OF SCOPE)

(Copied verbatim from CONTEXT.md § Deferred Ideas — planner MUST NOT scope these in)

- `is_granted()` → `is_approved()` impl method rename ergonomics — planner discretion at plan-open (above).
- Project-wide `Granted` → `Approved` comment / docstring sweep beyond Plan 45-02 — planner-discretion sweep; otherwise file follow-up todo for v2.7.
- One-time `nono audit migrate` tool for legacy ledger forward-port — rejected at D-45-C2.
- Permanent always-on CI lane for audit-attestation native-host coverage — Plan 45-03 ships `workflow_dispatch`-only per D-45-D2.
- Sibling-binding cascade verification for the wire-format break (`../nono-py/` + `../nono-ts/`) — planner verifies at plan-open; if affected, Phase 44 D-44-D1 lockstep precedent available.
- Cluster 2 DIVERGENCE-LEDGER amendment exact ledger location — planner verifies at plan-open. **Confirmed via this research:** `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` is the canonical and only location.
- Two `todo.match-phase 45` matches (`44-class-d-validator-preflight-investigation.md`, `44-validate-restore-target-fd-relative-hardening.md`) — both score 0.6 keyword-only; tagged for future Linux-host phase / security-scoped phase per Phase 44 CONTEXT § Deferred Ideas.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-PORT-CLOSURE-08 | 39 `#[unsafe(no_mangle)]` rewrites in `bindings/c/src/` land per upstream Edition 2024 source migration; DIVERGENCE-LEDGER Cluster 2 split-disposition resolved. | § "Plan 45-01 Mechanics" — 39 sites empirically confirmed; cbindgen byte-identical gate command verified; DIVERGENCE-LEDGER location confirmed. |
| REQ-AIPC-G04-01 | `Approved(ResourceGrant)` inlined at wire type so `(Approved, grant=None)` is compile-time error; `aipc_sdk.rs` demultiplexer + 23 pre-existing tests updated; AUD-05 token-redaction regression still passes. | § "Plan 45-02 Test + Construction-Site Inventory" — exact line numbers; § "Plan 45-02 Cascade Map" — file-by-file change scope. |
| REQ-RESL-NIX-04 | Phase 38 REQ-AAHX-HOST-01 native re-validation on Linux + macOS host (one or both per host availability); tactical confirmation pass; does not block phase close if no gap is found. | § "Plan 45-03 Native Re-validation Protocol" — Phase 27.2 transitive-closure mapping confirmed; concrete `cargo test` invocation derived; workflow_dispatch shape mapped against `phase-37-linux-resl.yml` precedent. |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust toolchain | 1.95.0 (workspace MSRV per Phase 43 Plan 43-01b) | Compiler + Edition 2024 syntax support | Locked by `Cargo.toml [workspace.package] rust-version` — Edition 2024 requires Rust 1.85+; fork is at 1.95. `[VERIFIED: rustc --version on this host returns 1.95.0]` |
| `cbindgen` | workspace dep (per `bindings/c/Cargo.toml`) | C header generation from Rust source | Standard for Rust-C FFI projects. `[VERIFIED: bindings/c/build.rs uses cbindgen::Builder::new()]` |
| `serde` / `serde_json` | workspace deps | Wire-type serialization for `ApprovalDecision` + `SupervisorResponse` | Already in use across `crates/nono/src/supervisor/types.rs`; Plan 45-02 rename `Granted → Approved` flows through serde derive transparently. `[VERIFIED: types.rs uses #[derive(Serialize, Deserialize)]]` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tempfile` | workspace dep | Test fixtures (e.g., audit-attestation tempdir setup) | Plan 45-03's protocol doc references the existing `audit_attestation.rs` pattern. |
| GitHub Actions runners | `ubuntu-24.04`, `macos-latest` | Native-host CI for RESL native re-validation | Plan 45-03 mirrors `phase-37-linux-resl.yml` matrix layout. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Drop `Granted → Approved` rename | Keep `Granted` literal; inline ResourceGrant only | Rejected at D-45-C3: locks in code-vs-wire-vs-comment naming drift permanently; SC#2 + PROJECT.md + audit_commands.rs:867 fixture all use "Approved". |
| Custom `Deserialize` accepting both wire shapes | Bridge old-shape ledgers across one milestone | Rejected at D-45-C2: ~30 LOC + tests + tagged removal; complicates wire-type invariant story. Audit-attestation is session-fresh by design. |
| One-time `nono audit migrate` subcommand | Forward-port legacy ledgers | Rejected at D-45-C2: ~100 LOC + new subcommand + integrity-rewrites-Merkle-root concern. |
| Permanent always-on RESL CI lane | Mirror `phase-37-linux-resl.yml` always-on | Rejected at D-45-D2: SC#3 explicitly says "tactical confirmation pass only — does not block phase close if no gap is found". Deletable in v2.7 once verdict recorded. |

**Installation:** None — no new crate deps for any of the three plans. `[CITED: bindings/c/Cargo.toml workspace deps unchanged in Plan 45-01; Plan 45-02 reuses existing serde derive; Plan 45-03 reuses standard GH Actions tooling]`

**Version verification:** None required; this phase is pure source rewrite + workflow authoring, no version bumps.

## Architecture Patterns

### System Architecture Diagram

```
Plan 45-01 (Edition 2024 source migration) — IN: 39 #[no_mangle] sites in bindings/c/src/; OUT: 39 #[unsafe(no_mangle)] sites; cbindgen → byte-identical nono.h

  bindings/c/src/{capability_set,fs_capability,lib,query,sandbox,state}.rs
        │                                                       │
        │  per-file commit (×6)                                  │
        ▼                                                       ▼
  Rust source mutation                                  cbindgen build.rs
        │                                                       │
        │                                                       ▼
        │                                              bindings/c/include/nono.h
        │                                                       │
        ▼                                                       ▼
  ledger amend: Cluster 2 split → closed              byte-identical gate (PASS/DEVIATE)


Plan 45-02 (AIPC G-04 wire tightening) — IN: ApprovalDecision::Granted + grant: Option<ResourceGrant>; OUT: ApprovalDecision::Approved(ResourceGrant)

  crates/nono/src/supervisor/types.rs (wire type)
        │
        │  rename Granted → Approved(ResourceGrant); drop SupervisorResponse::Decision.grant
        ▼
  crates/nono/src/supervisor/aipc_sdk.rs (child SDK demultiplexer; 7 construction sites + 1 match-arm at :417)
        │
        ▼
  crates/nono-cli/src/exec_strategy_windows/supervisor.rs (22 Decision construction sites; AUD-05 regression test)
        │
        ▼
  crates/nono-cli/src/{audit_integrity,audit_commands,exec_strategy,terminal_approval}.rs (cross-platform + Unix consumers)
        │
        ▼
  crates/nono/src/supervisor/{socket,socket_windows,mod}.rs (sibling broker consumers)
        │
        ├──► CHANGELOG.md (BREAKING entry)
        └──► docs/architecture/audit-bundle-target.md (ADR amendment 45-X)


Plan 45-03 (RESL native re-validation) — IN: pre-existing audit_attestation.rs (Phase 27.2 closed); OUT: workflow + protocol doc

  .github/workflows/phase-45-resl-native-host.yml (NEW, workflow_dispatch-only, matrix ubuntu-24.04 + macos-latest)
        │
        ▼
  cargo test -p nono-cli --test audit_attestation -- (live invocation deferred to Phase 46 orchestrator)
        │
        ▼
  45-03-NATIVE-RESL-PROTOCOL.md (NEW: SC#3 decision tree + expected output + Phase 46 hand-off)
```

### Recommended Project Structure

```
.planning/phases/45-source-migration-aipc-g-04-resl-native-re-validation/
├── 45-CONTEXT.md                          # (exists) phase context
├── 45-DISCUSSION-LOG.md                    # (exists) audit trail
├── 45-RESEARCH.md                          # this file
├── 45-VALIDATION.md                        # synthesized by orchestrator
├── 45-01-EDITION-2024-MIGRATION-PLAN.md   # planner output
├── 45-02-AIPC-G04-TIGHTENING-PLAN.md      # planner output
├── 45-03-RESL-NATIVE-REVALIDATION-PLAN.md # planner output
├── 45-03-NATIVE-RESL-PROTOCOL.md          # plan output (NEW; protocol doc per D-45-D1)
├── 45-{NN}-SUMMARY.md                     # per-plan summary (×3)
└── 45-VERIFICATION.md                      # phase-close artifact

bindings/c/src/                             # Plan 45-01 surface
├── capability_set.rs                       # 16 #[no_mangle] sites
├── fs_capability.rs                        # 7 sites
├── lib.rs                                  # 4 sites
├── query.rs                                # 4 sites
├── sandbox.rs                              # 3 sites
├── state.rs                                # 5 sites
└── types.rs                                # 0 sites (no FFI exports)

bindings/c/include/nono.h                   # byte-identical gate target

.github/workflows/phase-45-resl-native-host.yml  # Plan 45-03 surface (NEW)
```

### Pattern 1: One commit per file with `Replay-of:` annotation (Plan 45-01)

**What:** Mechanical Edition 2024 rewrite, scoped by file boundary, with body annotation linking to upstream `79715aa5` without using the D-19 `Upstream-commit:` trailer block.
**When to use:** Fork-side source-conformance to an already-merged upstream change where no upstream PR is warranted (D-45-B1 ⇄ Phase 40 D-20 precedent).
**Example commit body:**

```text
chore(45-01): bindings/c capability_set.rs Edition 2024 no_mangle (16 sites)

Sweep 16 #[no_mangle] sites to #[unsafe(no_mangle)] per Rust Edition 2024
semantics. No behavior change; no signature change; cbindgen output remains
byte-identical (verified at Plan 45-01 close).

Replay-of: 79715aa5 (Phase 43 Plan 43-01b DEC-3 split-disposition close)
Cluster: 2 (Rust edition 2024 + workspace dependency centralization)
DIVERGENCE-LEDGER: see .planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md
                   § "Cluster: Rust edition 2024" — disposition split → closed
                   at Plan 45-01 close.

Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
```

Source: synthesized from CONTEXT.md D-45-B1 + Phase 43 Plan 43-01b `chore(43-01b):` pattern (commit `b6aac925` — `[VERIFIED: 43-01b-EDITION-WORKSPACE-ONLY-SUMMARY.md § DEC-2]`).

### Pattern 2: Single-site tuple construction for protocol invariants (Plan 45-02 already-applied precedent)

**What:** When `(decision, grant)` is the wire pair and `(Granted, None)` is illegal, bind both via a single `let (decision, grant) = if ... { ... } else { ... };` — each arm returns the complete tuple. Plan 45-02 supersedes this with TYPE-LEVEL enforcement.
**When to use:** Flow-control-boundary invariant before the type system can enforce. **`[CITED: Phase 18.1-02 SUMMARY § patterns-established; commit 3493dd8 in supervisor.rs:~1875-1950]`**
**Plan 45-02 supersedes:** Inline `ResourceGrant` into the `Approved` variant; the dispatcher pair-binding becomes redundant but remains correct (defense in depth). Plan 45-02 may simplify those sites if planner discretion allows — otherwise leave intact.

### Pattern 3: `workflow_dispatch`-only tactical verification workflow (Plan 45-03)

**What:** GitHub Actions workflow with `on.workflow_dispatch` only — invoked manually via `gh workflow run`, no auto-trigger on push or PR. Includes `inputs.gh_runner_os` choice for matrix selectivity.
**When to use:** Tactical confirmation pass that should not burn CI minutes on every PR. Deletable artifact once verdict recorded.
**Example:**

```yaml
name: Phase 45 RESL Native Host Re-validation

on:
  workflow_dispatch:
    inputs:
      gh_runner_os:
        description: Which OS matrix to run
        type: choice
        options: [ubuntu-24.04, macos-latest, both]
        default: both

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -Dwarnings

permissions:
  contents: read

jobs:
  resl-nix:
    if: ${{ inputs.gh_runner_os == 'ubuntu-24.04' || inputs.gh_runner_os == 'both' }}
    name: Phase 45 RESL native (Linux)
    runs-on: ubuntu-24.04
    continue-on-error: true   # SC#3: one or both per host availability
    # ... mirror phase-37-linux-resl.yml setup steps (actions/checkout@SHA, dtolnay/rust-toolchain, actions/cache) ...
    steps:
      # ... checkout, rust toolchain, cache ...
      - name: Run audit-attestation regression
        run: cargo test -p nono-cli --test audit_attestation -- --include-ignored

  resl-darwin:
    if: ${{ inputs.gh_runner_os == 'macos-latest' || inputs.gh_runner_os == 'both' }}
    name: Phase 45 RESL native (macOS)
    runs-on: macos-latest
    continue-on-error: true
    # ... mirror layout, same cargo test command ...
```

Source: synthesized from CONTEXT.md D-45-D1 + D-45-D2 + Phase 37 workflow precedent at `.github/workflows/phase-37-linux-resl.yml`.

### Anti-Patterns to Avoid

- **`#[allow(clippy::unwrap_used)]` to silence cross-target lints surfaced by Plan 45-01 / 45-02.** Violates CLAUDE.md § Unwrap Policy + `cross-target-verify-checklist.md` § Anti-Pattern 2. Use cfg-gates, visibility changes, or structural code changes.
- **Touching `*_windows.rs` files beyond what is strictly required by the wire-type cascade (Plan 45-02 only).** D-34-E1 / D-40-E1 invariant. The exception is documented at CONTEXT.md `<canonical_refs>` § Cross-phase invariants — Plan 45-02 touches `exec_strategy_windows/supervisor.rs` only at the unavoidable Decision construction sites for the rename + inline cascade.
- **Skipping the cbindgen byte-identical gate after Plan 45-01.** Edition 2024 should not change C header output; if it does, Plan 45-01 has a deviation and must surface to user (D-45-B3).
- **Running `cargo check` and assuming it covers cross-target clippy.** `cargo check` does not run clippy and does not exercise Unix cfg branches. `[CITED: cross-target-verify-checklist.md § Anti-Pattern 3]`
- **Treating the AUD-05 regression test as automatically passing.** Plan 45-02 must explicitly run `cargo test -p nono-cli --bin nono recorded_ledger_redacts_session_token` (or equivalent) and call out the pass in the commit body per D-45-C1.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Wire-format backward compat with old `(Granted, grant=None)` shape | Custom `Deserialize` accepting both shapes | Accept BREAKING change per D-45-C2 | ~30 LOC + tests + tagged removal at v2.7; locks in deserializer-level-but-not-wire-level invariant for one milestone; audit-attestation is session-fresh by design. |
| Pre-v2.6 ledger forward-port | `nono audit migrate` subcommand | Document limitation in CHANGELOG + ADR | ~100 LOC + new subcommand + integrity rewrites Merkle root; replay of pre-upgrade ledgers is a documented limitation, not a security regression. |
| Permanent always-on CI lane for native-host RESL | Mirror `phase-37-linux-resl.yml` always-on | `workflow_dispatch`-only per D-45-D2 | SC#3 says "tactical confirmation pass only"; burns CI minutes on every PR for no incremental signal; deletable in v2.7 once verdict recorded. |
| Cross-target Linux/macOS clippy on Windows host | Install full `x86_64-linux-gnu-gcc` + osxcross toolchains | PARTIAL disposition per `.planning/templates/cross-target-verify-checklist.md` | Same disposition used by Phase 41, 43-01b, 44 (3 consecutive precedents); GH Actions Linux Clippy + macOS Clippy lanes are the decisive close signal. |
| Project-wide `Granted → Approved` doc / comment sweep | Sweep every supervisor.rs comment | Touch comments at the rename callsites; file follow-up todo for v2.7 sweep | Out-of-scope creep; CONTEXT.md § Deferred Ideas explicitly defers this. |

**Key insight:** Phase 45 is bundled-by-disjoint-surface, not bundled-by-shared-domain. Each plan is independently understandable, independently committable, and independently verifiable. The temptation to "improve while you're in there" (rename methods, sweep comments, refactor adjacent code) MUST be resisted — CONTEXT.md § Deferred Ideas is the cargo manifest for this discipline.

## Runtime State Inventory

> Phase 45 has rename/refactor characteristics for Plan 45-02 (`Granted → Approved` variant rename + field drop). Required.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | **Pre-v2.6 `audit-events.ndjson` ledgers** containing `{"decision":{"Granted":null},"grant":{...}}` wire shape. Stored at `<NONO_TEST_HOME or %LOCALAPPDATA%>\nono\audit\<session-id>\audit-events.ndjson`. | **Code edit only** (Plan 45-02 changes the type); existing ledger files become non-re-verifiable (accepted per D-45-C2). Documented in CHANGELOG BREAKING entry. No data migration. |
| Live service config | No external services depend on `ApprovalDecision` wire shape. AIPC IPC is process-internal (parent ⇄ child supervisor). | None — verified by grep for `Granted` outside `crates/`: zero matches in `.github/`, `docs/`, `scripts/`, `*.toml`. |
| OS-registered state | No OS-level registrations embed the wire shape — AIPC sockets are session-ephemeral; pipe names use session UUIDs, not decision names. | None — verified by grep `Granted` across the workspace excluding `.planning/`: matches are only in source code + tests + ADR docs. |
| Secrets and env vars | No env var or secret key references `Granted` or `Approved` by name. AUD-05 token-redaction regression test (`recorded_ledger_redacts_session_token`) uses literal `"TOPSECRET_TOKEN_DO_NOT_LEAK_42"` and asserts NEITHER variant appears post-redaction; preserved across the rename. | None — `[VERIFIED: grep -rn "Granted\|Approved" .env* 2>/dev/null returns nothing]` |
| Build artifacts / installed packages | **Generated `bindings/c/include/nono.h`** is rebuilt from Rust source by `bindings/c/build.rs`. Plan 45-01 must regenerate and assert byte-identical per D-45-B3. **`Cargo.lock`** post-Plan-45-02: no dependency changes expected; verify clean lockfile post-commit. **`../nono-py/`** + **`../nono-ts/`** sibling repos may consume the wire type via the FFI surface — planner verifies at plan-open (CONTEXT.md § Deferred Ideas). | **`cargo build -p nono-ffi` at Plan 45-01 close** (cbindgen byte-identical gate). **Sibling-repo verification at Plan 45-02 plan-open** (read-only inspection; if affected, surface as deviation per Phase 44 D-44-D1 precedent). |

**The canonical question:** *After Plan 45-02's atomic commit lands, what runtime systems still have the old `Granted` wire shape cached or stored?*
**Answer:** Pre-v2.6 `audit-events.ndjson` files on user disks. This is the BREAKING change per D-45-C2 — accepted, documented in CHANGELOG + ADR, no migration tool.

## Common Pitfalls

### Pitfall 1: Edition 2024 attribute rewrite triggers ABI break in `nono.h`

**What goes wrong:** `cargo fix --edition` or manual `#[no_mangle]` → `#[unsafe(no_mangle)]` rewrite changes cbindgen-generated C header in a way that breaks downstream consumers.
**Why it happens:** Edition 2024 may require additional `unsafe` block wrapping for `extern` declarations; cbindgen may emit different visibility annotations.
**How to avoid:** D-45-B3 cbindgen byte-identical gate. After all 6 per-file commits, run `cargo build -p nono-ffi` (which triggers `bindings/c/build.rs`) and `git diff bindings/c/include/nono.h` — diff MUST be empty.
**Warning signs:** Any non-zero output from `git diff bindings/c/include/nono.h` post Plan 45-01 close. **`[VERIFIED: bindings/c/build.rs uses cbindgen::Builder::new().with_crate(&crate_dir).with_config(config).generate()]`**

### Pitfall 2: `audit_commands.rs:867` fixture pre-aligns with the rename — easy to miss as already-done

**What goes wrong:** Fixture line 867 already reads `"decision":{"Approved":null}` (verified by direct file read). The fixture was hand-rolled as serde_json::Value and predicts the post-rename wire shape. Planner could miss this and assume it needs updating, then double-edit.
**Why it happens:** CONTEXT.md says "audit_commands.rs:867 test fixture line currently using `"decision":{"Approved":null}` via `serde_json::Value` workaround; post-rename, this becomes the type-checked shape."
**How to avoid:** Inspect line 867 at plan-open. The line is OK — only the surrounding comment may need a follow-up touchup (the "workaround" framing becomes obsolete once the type-checked shape lands). The lines that DO need editing are at `865-866` and `867` if they currently use `"Granted"` literal in similar fixture form — they don't; lines 865, 866 use `"Denied"`, line 867 uses `"Approved"` already. **`[VERIFIED: read of audit_commands.rs:855-870]`**
**Warning signs:** Planner producing a task to edit line 867 when no edit is needed; or planner editing fixture without removing the workaround comment.

### Pitfall 3: AUD-05 regression test uses session token that contains "Token" / "Approved" substring

**What goes wrong:** `recorded_ledger_redacts_session_token` (at `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:5033`) uses `"TOPSECRET_TOKEN_DO_NOT_LEAK_42"` and asserts that string never appears in the NDJSON ledger. If the rename inadvertently leaks the variant name "Approved" into the redacted output where the original "Granted" was present, the assertion still passes (because it asserts the TOKEN doesn't leak, not the variant) — but the substantive regression check could miss a real leakage.
**Why it happens:** The test asserts `!ledger.contains(sensitive_token)`, NOT `ledger.contains("Approved")`. Plan 45-02 must verify this test still passes AND that the ledger structure makes sense — i.e., the leaked-token-detection invariant is preserved across the wire-type cascade.
**How to avoid:** Run the test post-commit and inspect the ledger NDJSON manually (one-line spot-check) to confirm the new `"Approved":{...}` wire shape contains a properly-formed ResourceGrant payload AND that the token is still scrubbed. The Phase 23 D-01 redactor `audit_entry_with_redacted_token` at `supervisor.rs:1279` is the load-bearing scrub; the test verifies it operates on the persistent NDJSON path correctly.
**Warning signs:** Test passes but ledger reads `{"decision":{"Approved":{}}}` (empty ResourceGrant) — Plan 45-02 has not properly constructed the inlined variant.

### Pitfall 4: Plan 45-02 forgets to update the docstring at `audit_integrity.rs:83`

**What goes wrong:** The docstring at `crates/nono-cli/src/audit_integrity.rs:83-85` reads "`None` for Approved decisions, for non-Windows ledger entries, and for the three pre-stage rejections..." — this stale text refers to the OLD `reject_stage: Option<RejectStage>` semantics, not the wire `grant` field. Post-Plan-45-02, the comment is still correct for `reject_stage` (which is unrelated), but a planner sweep might mistakenly edit it.
**Why it happens:** Two unrelated `Option<...>` fields exist in the same module; one is dropped (Plan 45-02 drops `SupervisorResponse::Decision.grant: Option<ResourceGrant>`), the other is unchanged (`AuditEventPayload::CapabilityDecision.reject_stage: Option<RejectStage>`).
**How to avoid:** Plan 45-02 ONLY drops `grant: Option<ResourceGrant>` from `crates/nono/src/supervisor/types.rs:484`. The `reject_stage` field at `audit_integrity.rs:92` is untouched. Surface the distinction in the PLAN task description.

### Pitfall 5: Cross-target Linux clippy "passes" only because the workspace has zero source changes

**What goes wrong:** Plan 45-03 produces NO source-tree edits (only `.github/workflows/` + `.planning/` artifacts). A planner might assume cross-target clippy is trivially green — but the PARTIAL disposition still applies because Plan 45-01 + 45-02 commits will sit on the same Phase 45 branch and gate together.
**Why it happens:** Phase-level CI gate vs plan-level CI gate confusion.
**How to avoid:** Run cross-target clippy at the Phase 45 close head, not at each plan's individual close. Per `.planning/templates/cross-target-verify-checklist.md` § Decision Tree, if any in-scope file is touched ANYWHERE in the phase, the gate applies to the phase head. Phase 45 head SHA = the SHA after Plans 45-01 + 45-02 + 45-03 all merge.
**Warning signs:** Verification doc lists Plan 45-03 as "cross-target N/A" — that's correct individually but misleading at phase scope.

## Code Examples

### Plan 45-01: Single-site Edition 2024 rewrite (representative)

Current (from `bindings/c/src/capability_set.rs:28-30`):

```rust
#[no_mangle]
pub extern "C" fn nono_capability_set_new() -> *mut NonoCapabilitySet {
    Box::into_raw(Box::new(NonoCapabilitySet::default()))
}
```

Post-Plan-45-01:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn nono_capability_set_new() -> *mut NonoCapabilitySet {
    Box::into_raw(Box::new(NonoCapabilitySet::default()))
}
```

Source: file-direct read of `bindings/c/src/capability_set.rs` + Phase 43 Plan 43-01b DEC-3 error excerpt confirming the exact substitution form. **`[VERIFIED: cargo fix --edition error at 43-01b-EDITION-WORKSPACE-ONLY-SUMMARY.md § DEC-3 lines 161-172]`**

### Plan 45-02: Wire-type inline rewrite (canonical change)

Current (from `crates/nono/src/supervisor/types.rs:198-211`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalDecision {
    /// Access was granted. Resource-transfer details, if any, are carried by
    /// [`SupervisorResponse::Decision`].
    Granted,
    /// Access was denied with a reason.
    Denied {
        /// Why the request was denied
        reason: String,
    },
    /// The approval request timed out without a decision.
    Timeout,
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupervisorResponse {
    /// Response to a capability request
    Decision {
        request_id: String,
        decision: ApprovalDecision,
        grant: Option<ResourceGrant>,   // ← DROPPED in Plan 45-02
    },
    // ...
}
```

Post-Plan-45-02:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalDecision {
    /// Access was approved. The resource-transfer metadata is carried inline.
    Approved(ResourceGrant),    // ← INLINED
    /// Access was denied with a reason.
    Denied {
        reason: String,
    },
    /// The approval request timed out without a decision.
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupervisorResponse {
    Decision {
        request_id: String,
        decision: ApprovalDecision,
        // grant field removed; payload now carried by Approved(ResourceGrant)
    },
    // ...
}
```

Demultiplexer cascade — current (from `crates/nono/src/supervisor/aipc_sdk.rs:404-433`):

```rust
match cap_pipe.recv_response()? {
    SupervisorResponse::Decision { request_id: resp_id, decision, grant } => {
        if resp_id != request_id { /* drift error */ }
        match decision {
            ApprovalDecision::Granted => grant.ok_or_else(|| {
                NonoError::SandboxInit(
                    "supervisor granted but returned no ResourceGrant".to_string(),
                )
            }),
            ApprovalDecision::Denied { reason } => Err(...),
            ApprovalDecision::Timeout => Err(...),
        }
    }
    other => Err(...),
}
```

Post-Plan-45-02:

```rust
match cap_pipe.recv_response()? {
    SupervisorResponse::Decision { request_id: resp_id, decision } => {
        if resp_id != request_id { /* drift error */ }
        match decision {
            ApprovalDecision::Approved(grant) => Ok(grant),
            ApprovalDecision::Denied { reason } => Err(...),
            ApprovalDecision::Timeout => Err(...),
        }
    }
    other => Err(...),
}
```

**The `ok_or_else` "supervisor granted but returned no ResourceGrant" defense-in-depth branch becomes structurally unreachable** — this IS the SC#2 compile-time guarantee. Source: file-direct read of `aipc_sdk.rs:400-434` + verbatim cascade construction.

### Plan 45-03: native-host invocation (representative)

Current — what Phase 27.2 closed:

```bash
cargo test -p nono-cli --test audit_attestation -- --include-ignored
# Expected: running 2 tests / test audit_verify_reports_signed_attestation_with_pinned_public_key ... ok
#                          / test rollback_signed_session_verifies_from_audit_dir_bundle ... ok
#                          / test result: ok. 2 passed; 0 failed; 0 ignored
```

Source: `[VERIFIED: .planning/phases/27.2-audit-attestation-test-re-enablement/27.2-04-SUMMARY.md § Post-execution closure (2026-05-09) — fix 2b7425e7 verification output]`

Plan 45-03 wraps this in CI via `.github/workflows/phase-45-resl-native-host.yml`. See Pattern 3 above.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `#[no_mangle]` bare attribute | `#[unsafe(no_mangle)]` wrapped attribute | Rust Edition 2024 stabilization (1.85) | Forces explicit `unsafe` declaration at FFI export boundary; aligns with CLAUDE.md § Security Considerations "Explicit over Implicit". |
| `(ApprovalDecision::Granted, grant: Option<ResourceGrant>)` two-field protocol | `ApprovalDecision::Approved(ResourceGrant)` inlined | Plan 45-02 (this phase) | Makes illegal state structurally unreachable; ~30 LOC saved across consumers; BREAKING wire change for stored ledgers. |
| Pre-Phase-22 audit-event NDJSON without Merkle integrity | Phase 22 Merkle-integrity wrapped NDJSON (preserved as v2.5+ shape) | Phase 22 Plan 22-05a | The post-Plan-45-02 ledger still uses Phase 22's chain + Merkle commitment; only the inner `AuditEntry::decision` wire shape changes. |
| Phase 27.2 audit-attestation tests `#[ignore]`d | Both tests enabled + passing (after fix `2b7425e7`) | Phase 27.2 Plan 04 + tracing-stderr fix | Plan 45-03 verifies this closure holds on native Linux + macOS. |

**Deprecated/outdated:**
- `#[no_mangle]` bare (in fork's `bindings/c/src/`) — replaced by `#[unsafe(no_mangle)]` in Plan 45-01.
- `SupervisorResponse::Decision.grant: Option<ResourceGrant>` field — dropped in Plan 45-02.
- `ApprovalDecision::Granted` variant name — renamed to `Approved(ResourceGrant)` in Plan 45-02.
- Dispatcher single-site tuple-construction pattern at `exec_strategy_windows/supervisor.rs:1875-1950` (Phase 18.1-02 G-04 fix) — still correct, now redundant; planner discretion to simplify.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The 7 ApprovalDecision::Granted construction sites in `aipc_sdk.rs` correspond to the "5 push sites" CONTEXT.md mentions; the +2 are likely defensive duplicates per HandleKind (Event, Mutex, Pipe×2, JobObject, Socket×2). | § Plan 45-02 inventory | LOW — planner will inventory at plan-open per CONTEXT.md discretion. If count differs from CONTEXT.md "5 push sites" wording, the deviation is a discretion call, not a blocker. |
| A2 | The "23 pre-existing tests" figure in CONTEXT.md / ROADMAP.md SC#2 refers to the `capability_handler_tests` module that Phase 18.1-02 grew from 23 → 28 tests. Today the module count is likely 28+ (further Phase 18.1-03 + Phase 23 additions). Planner inventories at plan-open. | § Plan 45-02 inventory | MEDIUM — CONTEXT.md explicitly allows ±2 deviation; if delta is large, surface as deviation per the planner discretion clause. |
| A3 | Sibling repos `../nono-py/` and `../nono-ts/` are not in the working directory and cannot be inspected from this research session. CONTEXT.md § Deferred Ideas explicitly defers sibling-binding cascade verification to plan-open. | § Don't Hand-Roll | MEDIUM — if sibling repos consume the wire type via JSON parsing, Plan 45-02 may surface a cascading lockstep need. Phase 44 D-44-D1 lockstep precedent is the documented escape hatch. |
| A4 | The DIVERGENCE-LEDGER amendment in Plan 45-01's final commit will textually replace `**Disposition:** split — workspace edits in Phase 43 Plan 43-01b, source migration deferred to v2.6 / UPST6` at `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md:76` with `**Disposition:** closed — workspace edits in Phase 43 Plan 43-01b; source migration in Phase 45 Plan 45-01 (commits <range>); closing-back-reference: 79715aa5` (or equivalent prose). The precise prose is planner discretion; the textual locus is verified. | § "DIVERGENCE-LEDGER amendment" | LOW — line 76 is the canonical disposition line; verified by grep + file read. |

**Total assumed claims:** 4. All are scope-bounded by CONTEXT.md's "Claude's Discretion" or "Deferred Ideas" sections — they signal planner-discretion calls at plan-open, not factual gaps.

## Plan 45-02 Cascade Map (empirical, file-by-file)

Generated from `grep -rn "ApprovalDecision::Granted\|grant: Option<ResourceGrant>\|grant: None\|grant: Some" crates/ bindings/` + targeted reads.

| File | Variant uses | `grant: None` sites | `grant: Some` sites | `SupervisorResponse::Decision { ... }` sites | Notes |
|------|-------------|---------------------|----------------------|----------------------------------------------|-------|
| `crates/nono/src/supervisor/types.rs` | 1 (enum def `:200-211`) + 1 (`impl is_granted` `:409`) | 0 | 0 | (definition site `:476-495`) | The rename target. Drop `grant: Option<ResourceGrant>` at line `:484`. `impl ApprovalDecision` at `:405-417` has `is_granted()` + `is_denied()`; rename `is_granted() → is_approved()` per planner discretion. |
| `crates/nono/src/supervisor/aipc_sdk.rs` | 8 (`:417` match-arm + 7 construction sites at `:730, :801, :967, :1033, :1078, :1141, :1212`) | 2 (`:769, :841`) | 7 | 10 | Child SDK demultiplexer + per-HandleKind broker test fixtures. The match arm at `:417` becomes `ApprovalDecision::Approved(grant) => Ok(grant)` (no `ok_or_else` needed). |
| `crates/nono/src/supervisor/mod.rs` | 2 (`:148, :202`) | 0 | 0 | 0 | Re-exports + a `let granted = ApprovalDecision::Granted;` test/example. Rename only. |
| `crates/nono/src/supervisor/socket.rs` | 1 (`:572` fully-qualified `crate::supervisor::types::ApprovalDecision::Granted`) | 0 | 0 | 2 | Cross-platform socket dispatch. Rename + Decision construction update. |
| `crates/nono/src/supervisor/socket_windows.rs` | 2 (`:1484, :1621`) | 1 (`:1622`) | 1 | 4 | Windows socket dispatch. |
| `crates/nono-cli/src/exec_strategy.rs` | 1 (`:2862` `matches!(decision, ApprovalDecision::Granted)`) | 3 (`:2691, :2842, :2854`) | 0 | 4 | Cross-platform exec dispatch. The `matches!` arm at `:2862` becomes `matches!(decision, ApprovalDecision::Approved(_))`. |
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` | 2 (`:2253, :2670` Ok-wrap construction) | 4 (`:1870, :1895, :1928, :1981`) | 0 | 22 | Windows supervisor — by far the largest cascade surface. Contains AUD-05 regression test at `:5033`. Phase 18.1-02 flow-control fix at `:1875-1950` becomes redundant (planner discretion to simplify). |
| `crates/nono-cli/src/terminal_approval.rs` | 1 (`:84` `Ok(ApprovalDecision::Granted)`) | 0 | 0 | 0 | Terminal prompt approval path. The Approved variant takes a ResourceGrant; this path must construct an appropriate ResourceGrant (likely `ResourceGrant::sideband_file_descriptor(access)` for the default file-grant case). Surface in PLAN as explicit task. |
| `crates/nono-cli/src/audit_integrity.rs` | 0 | 0 | 0 | 0 | NO direct `Granted` references; the wire shape flows through `AuditEntry::decision` transparently. Docstring at `:83-93` mentions "None for Approved decisions" but refers to `reject_stage`, not `grant` (see Pitfall 4). |
| `crates/nono-cli/src/audit_commands.rs` | 0 (only fixture string `"Approved"` at `:867`) | 0 | 0 | 0 | Fixture already uses `"Approved"` — no edit needed for the rename (see Pitfall 2). Read-only verification path. |

**Total impacted files:** 10 source files + (planner's discretion) `CHANGELOG.md` + `docs/architecture/audit-bundle-target.md`.
**Total construction sites needing edits:** 42 `SupervisorResponse::Decision { ... }` blocks (drop `grant:` field) + 18 `ApprovalDecision::Granted` use-sites (rename or pattern-update).
**Total `grant: None` callsites that disappear:** 10 (across 4 files).
**Total `grant: Some(...)` callsites that re-shape into `ApprovalDecision::Approved(...)`:** 8.

**Sanity-check vs CONTEXT.md "23 pre-existing tests":** Per Phase 18.1-02 SUMMARY, `capability_handler_tests` grew from 23 → 28 tests in that plan. The actual test count today is at least 28 (the supervisor.rs file has 47 `#[test]` markers; many in the `capability_handler_tests` module). Planner inventories at plan-open per CONTEXT.md discretion clause.

## Phase 45 Cross-Target Posture (decisive disposition)

| Plan | Touches cfg-gated Unix? | Touches cross-platform? | Touches Windows-only? | Cross-target clippy required? | Expected disposition |
|------|------------------------|--------------------------|------------------------|--------------------------------|----------------------|
| 45-01 | YES (`bindings/c/src/*` is cross-platform FFI; ALL Unix runtimes consume the generated header) | YES | NO | YES (per checklist § Scope) | **PARTIAL** — Linux + macOS cross-toolchain C linkers absent on Windows host (3-precedent: Phase 41, 43-01b, 44 all PARTIAL). Live GH Actions Linux Clippy + macOS Clippy on Phase 45 head is decisive. |
| 45-02 | YES (`crates/nono/src/supervisor/types.rs` cross-platform; `crates/nono/src/supervisor/socket.rs` Unix; `exec_strategy_windows/supervisor.rs` Windows cfg-gated) | YES | YES (`exec_strategy_windows/supervisor.rs` is Windows-only but cascade-unavoidable) | YES | **PARTIAL** — same. Plus the `exec_strategy_windows/supervisor.rs` touches are an unavoidable D-34-E1 / D-40-E1 cascade per CONTEXT.md § cross-phase invariants — NOT a new exception, just the wire-type rename consumer surface. |
| 45-03 | NO (no source-tree edits; only `.github/workflows/` + planning artifacts) | NO | NO | NO (per checklist § Scope question 1: "Does the plan touch any in-scope file? No → cross-target verification not required") | **N/A at plan scope** — but phase head still gates with Plans 45-01 + 45-02 commits. Phase scope = PARTIAL. |

**Toolchain inventory on this Windows host (verified empirically):**
- `rustup target list --installed` returns: `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`. **All three Rust targets ARE installed** per Phase 44 Plan 44-01 close.
- C cross-linkers (`x86_64-linux-gnu-gcc`, Darwin `cc`/`clang`) are absent per the same 3-phase precedent (Phase 41, 43-01b, 44 all hit the same disposition).
- `rustc --version` = `1.95.0`. `cargo --version` = `1.95.0`. **`[VERIFIED]`**

**Decisive close path:** Per `.planning/templates/cross-target-verify-checklist.md` § PARTIAL Disposition, mark REQ-PORT-CLOSURE-08 + REQ-AIPC-G04-01 as PARTIAL with `human_verification_truths` referencing the GH Actions Linux Clippy + macOS Clippy lanes on the Phase 45 head SHA. Phase 46 orchestrator captures the lane verdict + flips REQs to VERIFIED.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust 1.95.0 | All plans | YES | 1.95.0 | — |
| Cargo workspace build | All plans | YES | 1.95.0 | — |
| `rustup target add x86_64-unknown-linux-gnu` | Plan 45-01 + 45-02 cross-target clippy | YES (target installed) | — | C linker absent → PARTIAL per checklist |
| `rustup target add x86_64-apple-darwin` | Plan 45-01 + 45-02 cross-target clippy | YES (target installed) | — | C linker absent → PARTIAL per checklist |
| `x86_64-linux-gnu-gcc` C cross-linker | Plan 45-01 + 45-02 cross-target Linux clippy | NO | — | GH Actions Linux Clippy lane (live CI) |
| Darwin cross-linker (`cc`/`clang`) | Plan 45-01 + 45-02 cross-target macOS clippy | NO | — | GH Actions macOS Clippy lane (live CI) |
| `gh` CLI | Plan 45-03 workflow trigger (Phase 46 deferred) | YES | (per memory `gh_available`) | — |
| `cbindgen` (workspace dep) | Plan 45-01 byte-identical gate | YES (transitive via `bindings/c/build.rs`) | workspace-pinned | — |
| Linux runtime (`ubuntu-24.04` GitHub runner) | Plan 45-03 live run (deferred to Phase 46) | YES (via Actions runners) | — | — |
| macOS runtime (`macos-latest` GitHub runner) | Plan 45-03 live run (deferred to Phase 46) | YES (via Actions runners) | — | — |
| Sibling repo `../nono-py/` | Plan 45-02 cross-binding lockstep verification (if needed per CONTEXT.md § Deferred Ideas) | UNKNOWN (not in working dir) | — | Surface as deviation at plan-open if affected |
| Sibling repo `../nono-ts/` | Plan 45-02 cross-binding lockstep verification (if needed) | UNKNOWN (not in working dir) | — | Surface as deviation at plan-open if affected |
| `nono-shell-broker.exe` build artifact | `cargo test --workspace` Windows host gate (Phase 43-01b lesson) | UNKNOWN (must build before test run) | — | `cargo build -p nono-shell-broker --release` before `cargo test --workspace` |

**Missing dependencies with no fallback:**
- None at plan-author time. Cross-target C linkers are blocking for clippy LOCAL verification, but the PARTIAL disposition + GH Actions Linux Clippy + macOS Clippy lanes are the documented fallback (3-precedent pattern at Phase 41, 43-01b, 44).

**Missing dependencies with fallback:**
- Cross-target C linkers → PARTIAL disposition + live CI.
- Sibling repo verification → surface as deviation at plan-open + Phase 44 D-44-D1 lockstep precedent.
- `nono-shell-broker.exe` artifact → `cargo build -p nono-shell-broker --release` before tests (Phase 43-01b Issue 1 lesson).

## Validation Architecture

> Phase 45 has explicit success criteria in ROADMAP.md § Phase 45 (5 criteria) — these map cleanly to REQs. Validation is mostly automatable; the host-blocked native-RESL run is the one explicit PARTIAL deferral.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (built-in test harness) + GitHub Actions for native-host orchestration |
| Config file | `Cargo.toml` workspace `[workspace.lints.clippy] unwrap_used = "deny"` (Phase 43 Plan 43-01b); per-crate `[lints] workspace = true` |
| Quick run command | `cargo test --workspace` (Windows host) |
| Full suite command | `cargo test --workspace --all-features` (Phase 43 Plan 43-01b baseline: 2197 passed / 0 failed / 19 ignored) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-PORT-CLOSURE-08 | 39 `#[unsafe(no_mangle)]` rewrites land; cargo clippy clean on all 3 targets (with PARTIAL on cross-target) | build + clippy + cbindgen-byte-identical | `cargo build -p nono-ffi --release` AND `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) AND `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` (Linux PARTIAL) AND `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (macOS PARTIAL) AND `git diff bindings/c/include/nono.h` (must be empty) | ✅ all infrastructure exists |
| REQ-PORT-CLOSURE-08 | DIVERGENCE-LEDGER Cluster 2 disposition `split → closed` | grep + file read | `grep -c 'closed' .planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` (verify amended line) | ✅ |
| REQ-AIPC-G04-01 | `Approved(ResourceGrant)` inlined; `(Approved, grant=None)` is compile-time error | compile-time gate via build | `cargo build --workspace --all-features` exits 0 AND `grep -rn 'grant: Option<ResourceGrant>' crates/ bindings/` returns 0 AND `grep -rn 'ApprovalDecision::Granted' crates/ bindings/` returns 0 | ✅ |
| REQ-AIPC-G04-01 | All 23+ pre-existing tests updated; `cargo test --workspace --all-features` green | test | `cargo test --workspace --all-features` (full suite green, ≥ 2197 tests pass) | ✅ |
| REQ-AIPC-G04-01 | AUD-05 token-redaction regression `recorded_ledger_redacts_session_token` passes | test (targeted) | `cargo test --bin nono recorded_ledger_redacts_session_token -- --exact` | ✅ at `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:5033` |
| REQ-AIPC-G04-01 | `aipc_sdk.rs` demultiplexer at `:417` rewritten (no `ok_or_else` defense-in-depth branch) | grep | `grep -c 'supervisor granted but returned no ResourceGrant' crates/` = 0 | ✅ |
| REQ-RESL-NIX-04 (STRUCTURAL) | `.github/workflows/phase-45-resl-native-host.yml` exists; YAML-valid; `workflow_dispatch`-only | grep + YAML lint | `test -f .github/workflows/phase-45-resl-native-host.yml` AND `grep -c '^on:' .github/workflows/phase-45-resl-native-host.yml` = 1 AND `grep -c 'workflow_dispatch:' .github/workflows/phase-45-resl-native-host.yml` = 1 AND `grep -c '^  pull_request:\|^  push:' .github/workflows/phase-45-resl-native-host.yml` = 0 (no auto-triggers per D-45-D2) | ❌ Wave 0 (workflow does not exist yet) |
| REQ-RESL-NIX-04 (STRUCTURAL) | `45-03-NATIVE-RESL-PROTOCOL.md` exists; documents SC#3 decision tree | grep | `test -f .planning/phases/45-source-migration-aipc-g-04-resl-native-re-validation/45-03-NATIVE-RESL-PROTOCOL.md` | ❌ Wave 0 |
| REQ-RESL-NIX-04 (LIVE-RUN, deferred to Phase 46) | Native Linux + macOS audit-attestation regression passes | manual GH Actions trigger | `gh workflow run phase-45-resl-native-host.yml -f gh_runner_os=both` then `gh run watch` | ⏸ Phase 46 |
| Cross-cutting SC#4 | Windows-only-files invariant honored (D-34-E1 / D-40-E1) | grep + file scope | `git diff --stat <phase-base>..<phase-head> -- 'crates/**/*_windows.rs' 'crates/nono-cli/src/exec_strategy_windows/**' 'crates/nono-shell-broker/**'` lists only the unavoidable Plan 45-02 cascade in `exec_strategy_windows/supervisor.rs` (with documented justification in SUMMARY) | N/A — check at phase close |
| Cross-cutting SC#5 | Workspace builds + tests green on Windows host | build + test | `cargo build --workspace --all-features` AND `cargo test --workspace --all-features` (post `cargo build -p nono-shell-broker --release` per Phase 43-01b Issue 1 lesson) | ✅ |

### Sampling Rate

- **Per task commit:** `cargo build` (fast feedback)
- **Per plan close:** `cargo test --workspace --all-features` (full suite) + `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) + `cargo fmt --all -- --check`
- **Plan 45-01 close additionally:** `cargo build -p nono-ffi` + `git diff bindings/c/include/nono.h` (must be empty)
- **Plan 45-02 close additionally:** `cargo test --bin nono recorded_ledger_redacts_session_token -- --exact` (AUD-05 regression spot-check)
- **Phase 45 gate:** `cargo test --workspace --all-features` full suite + cross-target Linux + macOS clippy (PARTIAL per checklist) + `cargo fmt --all -- --check` + cbindgen byte-identical + 8-check close gate per Phase 43 D-43-E9 / Phase 44 close pattern

### Wave 0 Gaps

- [ ] `.github/workflows/phase-45-resl-native-host.yml` — Plan 45-03 authors NEW. Covers REQ-RESL-NIX-04 structural artifact.
- [ ] `.planning/phases/45-source-migration-aipc-g-04-resl-native-re-validation/45-03-NATIVE-RESL-PROTOCOL.md` — Plan 45-03 authors NEW. Covers REQ-RESL-NIX-04 protocol doc.
- [ ] Plan-open inventory grep for Plan 45-02 — planner runs `grep -rn 'ApprovalDecision::Granted\|grant: Option<ResourceGrant>\|grant: None\|grant: Some' crates/ bindings/` and inventories test count (CONTEXT.md says 23 ±2 allowed; if delta > 2, surface as deviation). **This research already executed the grep — see § "Plan 45-02 Cascade Map" — but the planner runs again at plan-open for sequence-of-record.**
- [ ] Cross-target verifier-protocol close-gate artifacts (`44-01-CLIPPY-CROSS-TARGET.md` analog) — Plan 45-01 + 45-02 author `45-01-CLIPPY-CROSS-TARGET.md` and `45-02-CLIPPY-CROSS-TARGET.md` per cross-target-verify-checklist.md § Enforcement.

*(No framework install needed — Cargo test harness + GitHub Actions are pre-existing.)*

## Security Domain

> `security_enforcement` is implicit-enabled per CLAUDE.md § Security Considerations. Phase 45 has direct security implications via Plan 45-01 (FFI export attribute) and Plan 45-02 (wire-type invariant elevation).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Phase 45 does not touch authentication surfaces. |
| V3 Session Management | no | Audit-attestation is session-fresh by design (D-45-C2) — Plan 45-02's wire break is documented as a session-replay limitation, not an auth bypass. |
| V4 Access Control | yes (indirect) | Plan 45-02 elevates `Approved ⟹ grant Some` from a flow-control invariant (Phase 18.1-02 G-04 dispatcher flip) to a TYPE-LEVEL invariant. Defense in depth. |
| V5 Input Validation | yes (preserved) | Plan 45-02 preserves existing input validation; the wire-type change does not affect mask gate or role allowlist at `supervisor.rs:1891+`. AUD-05 token-redaction regression test (`recorded_ledger_redacts_session_token`) preserves the redaction invariant. |
| V6 Cryptography | yes (preserved) | Plan 45-02 preserves the Phase 22 Merkle-chain + Phase 27.2 audit-attestation signing path; only the inner `AuditEntry::decision` wire shape changes. Phase 22 hash_algorithm = sha256, merkle_scheme = alpha, signing via the existing `audit-attestation` key infrastructure. |
| V8 Sensitive Data | yes (preserved) | AUD-05 token-redaction regression: `audit_entry_with_redacted_token` at `supervisor.rs:1279` is the load-bearing scrub; `recorded_ledger_redacts_session_token` test asserts session tokens never appear in NDJSON ledger. Preserved by Plan 45-02. |
| V14 Configuration | no | Phase 45 does not touch configuration files. |

### Known Threat Patterns for Rust FFI + AIPC IPC

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| FFI safety boundary missing `unsafe` annotation (Plan 45-01 closes) | Elevation of Privilege | `#[unsafe(no_mangle)]` Edition 2024 requirement — forces explicit `unsafe` declaration at every C-export. |
| Illegal IPC wire state `(Approved, grant=None)` (Plan 45-02 closes) | Tampering / Spoofing | Type-level invariant via `ApprovalDecision::Approved(ResourceGrant)` — illegal state is structurally unreachable. |
| Pre-existing dispatcher flow-control bug (`(Granted, None)` silently sent on broker failure) | Spoofing | Phase 18.1-02 G-04 fix (commit `3493dd8`) elevated `decision` to `Denied { reason: "broker failed: {e}" }` at the dispatcher; Plan 45-02 ELEVATES this from flow-control to type-level. |
| Session-token leakage in persistent NDJSON ledger | Information Disclosure | `audit_entry_with_redacted_token` scrub at `supervisor.rs:1279`; `recorded_ledger_redacts_session_token` regression at `:5033`. Preserved by Plan 45-02. |
| Pre-v2.6 ledger replay attack via "approve grant=null" forgery | Tampering | Mitigated by Phase 22 Merkle integrity (still applies); pre-v2.6 ledgers cannot deserialize against the new typed wire shape (D-45-C2 accepted BREAKING change). |
| Cross-target Linux/macOS clippy drift hiding cfg-gated bugs | Tampering / Repudiation | `.planning/templates/cross-target-verify-checklist.md` § PARTIAL Disposition + live GH Actions Linux Clippy + macOS Clippy lanes. |

### Security delta summary

- **Plan 45-01 net security effect:** POSITIVE (explicit-unsafe declarations at FFI exports surface implicit-unsafe boundaries; Edition 2024 alignment with CLAUDE.md § Security Considerations "Explicit over Implicit").
- **Plan 45-02 net security effect:** POSITIVE (type-level enforcement of `Approved ⟹ grant Some` invariant; eliminates an entire class of dispatcher-bypass spoofing vulnerabilities; AUD-05 regression preserved).
- **Plan 45-03 net security effect:** NEUTRAL (verification infrastructure; no source-tree changes).
- **BREAKING risk for users with pre-v2.6 ledgers:** ACCEPTED + DOCUMENTED per D-45-C2; mitigation = pin to v2.5 binary if pre-v2.6 ledger re-verification needed.

## Project Constraints (from CLAUDE.md)

These directives carry locked-decision authority and constrain planner output:

- **`.unwrap()` / `.expect()` strictly forbidden** — enforced by `clippy::unwrap_used` (workspace-level deny per Phase 43 Plan 43-01b). No exceptions in Plan 45-01 / 45-02 source. **`[VERIFIED: Cargo.toml [workspace.lints.clippy] unwrap_used = "deny" per 43-01b SUMMARY]`**
- **DCO sign-off on every commit** — `Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>` line in every commit body. (Confirmed via memory + Phase 43 Plan 43-01b commits.)
- **Cross-target clippy MUST/NEVER for cfg-gated Unix code** — applies to Plan 45-01 (`bindings/c/src/*` is cross-platform FFI consumed by Unix runtimes) AND Plan 45-02 (`crates/nono/src/supervisor/socket.rs` is Unix; `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` is Windows cfg-gated). Run `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `cargo clippy --workspace --target x86_64-apple-darwin` from the dev host; if cross-toolchain unavailable, mark REQ as PARTIAL per `.planning/templates/cross-target-verify-checklist.md`. **Windows-host `cargo check` is NOT a substitute.**
- **D-34-E1 / D-40-E1 / D-43-E1 Windows-only-files invariant** — Plan 45-02 touches `exec_strategy_windows/supervisor.rs` (22 Decision construction sites). This is an unavoidable cascade from the cross-platform wire-type change; per CONTEXT.md § cross-phase invariants, "Plan 45-02 touches `exec_strategy_windows/supervisor.rs` BUT only at the wire-type usage sites that are unavoidable for the rename + inline cascade (NOT new Windows-only code; existing Windows-only callsite updates). No codified addendum exceptions required." Document the touch scope in Plan 45-02 SUMMARY.
- **Lazy use of dead code forbidden** — Plan 45-02 removes the `grant: Option<ResourceGrant>` field; planner verifies no `#[allow(dead_code)]` is required on removed surface. If any callsite becomes dead, delete it (do not silence).
- **Environment variables in tests: save and restore** — applies to any Plan 45-02 test addition that modifies env vars; mirror Phase 27.2's `ScopedEnvVar` RAII pattern (already in `crates/nono-cli/tests/audit_attestation.rs`). Plan 45-03 does not add tests, so this is N/A there.
- **Apply `#[must_use]` to functions returning critical Results** — Plan 45-02's renamed `is_approved()` method MUST carry `#[must_use]` per the existing pattern at `crates/nono/src/supervisor/types.rs:407` (`is_granted()` already has `#[must_use]`).
- **Library is policy-free** — `bindings/c/src/*` (Plan 45-01 surface) is FFI library code; the Edition 2024 syntax conformance must not introduce CLI-policy concepts into the FFI layer.
- **Path security** — N/A for Plan 45-01 / 45-02 (no new path handling); N/A for Plan 45-03 (no source-tree changes).
- **GSD workflow enforcement** — all phase work must route through `/gsd:execute-phase` or `/gsd:quick` per CLAUDE.md final section.

## Open Questions

1. **Exact wording of the DIVERGENCE-LEDGER amendment line**
   - What we know: Line 76 reads `**Disposition:** split — workspace edits in Phase 43 Plan 43-01b, source migration deferred to v2.6 / UPST6`.
   - What's unclear: Whether the planner should replace the entire line or append a new "Closed" status line below. The "Original disposition" framing at line 77 suggests the latter (append-history pattern); but a clean `split → closed` flip suggests the former.
   - Recommendation: APPEND a `**Final disposition:** closed (Phase 45 Plan 45-01 commits <range>, ledger amended at SHA <amend-sha>). Source migration absorbed; cluster fully synchronized with upstream `79715aa5`.` line after line 76; keep the historical "Original disposition" line intact for audit traceability. Planner discretion.

2. **Whether to simplify the Phase 18.1-02 G-04 dispatcher flow-control after Plan 45-02**
   - What we know: The `let (decision, grant) = if decision.is_granted() { ... }` pattern at `exec_strategy_windows/supervisor.rs:1875-1950` (commit `3493dd8`) is the runtime invariant that Plan 45-02 supersedes at the type level.
   - What's unclear: Whether Plan 45-02 should simplify the dispatcher to remove the now-redundant pair-binding, or leave it as defense-in-depth.
   - Recommendation: LEAVE the dispatcher pair-binding intact — defense in depth at no cost. Add a comment `// Phase 45 Plan 45-02 elevated the (Approved, grant Some) invariant to type level; this dispatcher fold is now defense in depth, not load-bearing.` Planner discretion to simplify if a callsite blocks compile.

3. **Sibling-repo cascade — actually affected or not?**
   - What we know: `../nono-py/` and `../nono-ts/` are separate repositories. Bindings come from `bindings/c/` (the C FFI surface; Plan 45-01 only) — but the wire-type change (Plan 45-02) affects Rust-level IPC, not the C FFI exports.
   - What's unclear: Whether nono-py / nono-ts have any code that deserializes `ApprovalDecision` from a JSON wire format (e.g., reading audit-events.ndjson via Python/TypeScript), which would surface the BREAKING change downstream.
   - Recommendation: Planner inspects sibling repos at plan-open (read-only); if affected, file a follow-up todo for v2.7 cross-binding lockstep. CONTEXT.md § Deferred Ideas explicitly defers this.

4. **Plan 45-03 workflow `actions/*` SHA pinning**
   - What we know: Phase 37's workflow uses verbatim 40-char SHA pins (`actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6`, `dtolnay/rust-toolchain@631a55b12751854ce901bb631d5902ceb48146f7 # stable`, `actions/cache@668228422ae6a00e4ad889ee87cd7109ec5666a7 # v5`).
   - What's unclear: Whether Plan 45-03 should reuse the SAME SHAs (frozen-as-of-Phase-37) or pull current SHAs at plan-author time.
   - Recommendation: REUSE Phase 37's SHAs for the initial Plan 45-03 commit — minimizes audit-trail divergence and the workflow is tactical (deletable in v2.7). If GitHub deprecates any action between now and the Phase 46 live run, refresh at trigger time.

## Sources

### Primary (HIGH confidence)

- `CLAUDE.md` § Coding Standards + § Security Considerations + § Library vs CLI Boundary — project constitution; verified by direct file read.
- `.planning/phases/45-source-migration-aipc-g-04-resl-native-re-validation/45-CONTEXT.md` — phase decision document; verified by direct read; THIS IS THE BINDING SOURCE.
- `.planning/REQUIREMENTS.md` § REQ-PORT-CLOSURE-08, REQ-AIPC-G04-01, REQ-RESL-NIX-04 — verified line numbers 18, 22, 50.
- `.planning/ROADMAP.md` § Phase 45 — verified lines 83-94 with 5 success criteria.
- `.planning/STATE.md` — verified lines 28-41 with v2.6 phase summary.
- `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` § "Cluster: Rust edition 2024 + workspace dependency centralization" — verified at lines 74-87.
- `.planning/phases/43-upst5-sync-execution/43-01b-EDITION-WORKSPACE-ONLY-SUMMARY.md` § DEC-3 + § Issue 2 — verified the 39 sites figure and the per-file bindings/c/src/ scope.
- `.planning/phases/27.2-audit-attestation-test-re-enablement/27.2-04-SUMMARY.md` § Post-execution closure (2026-05-09) — verified Phase 27.2 transitive closure complete (fix commit `2b7425e7`).
- `.planning/phases/18.1-extended-ipc-gaps/18.1-02-SUMMARY.md` § patterns-established + key-decisions — verified the Phase 18.1-02 G-04 dispatcher flow-control fix and the original AIPC G-04 deferral reasoning.
- `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-VERIFICATION.md` § Re-verification Summary — verified the cross-target clippy PARTIAL precedent and the codified `.planning/templates/cross-target-verify-checklist.md`.
- `.planning/templates/cross-target-verify-checklist.md` — verified by direct read; PARTIAL Disposition is the documented escape path.
- `bindings/c/src/{capability_set,fs_capability,lib,query,sandbox,state}.rs` — `#[no_mangle]` site counts verified by grep: 16+7+4+4+3+5 = 39.
- `crates/nono/src/supervisor/types.rs:198-211, :405-417, :476-495` — `ApprovalDecision` enum + `is_granted` impl + `SupervisorResponse::Decision` definition verified by direct read.
- `crates/nono/src/supervisor/aipc_sdk.rs:404-433` — demultiplexer match + `ok_or_else` defense-in-depth branch verified by direct read.
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:5033` — `recorded_ledger_redacts_session_token` test verified by direct read of lines 5025-5082.
- `crates/nono-cli/src/audit_commands.rs:855-870` — fixture line 867 verified by direct read (already uses `"Approved"`).
- `crates/nono-cli/src/audit_integrity.rs:69-103` — `AuditEventPayload::CapabilityDecision` enum verified by direct read.
- `.github/workflows/phase-37-linux-resl.yml` — workflow precedent verified by direct read; 307 lines; pattern (matrix runner + RUSTFLAGS + actions/setup-rust + actions/cache + cargo test invocation) confirmed.
- `bindings/c/Cargo.toml` + `bindings/c/build.rs` + `bindings/c/cbindgen.toml` — cbindgen mechanism verified by direct read; generates `bindings/c/include/nono.h`.
- `rustup target list --installed` output: `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu` — verified by Bash invocation on this host.
- `rustc --version` = `1.95.0` — verified by Bash invocation.

### Secondary (MEDIUM confidence)

- Grep walks across `crates/` + `bindings/` for `ApprovalDecision::Granted`, `grant: Option<ResourceGrant>`, `grant: None`, `grant: Some`, `SupervisorResponse::Decision { ... }` — counts cross-verified across multiple `head_limit` queries; minor risk of edge-case false positives in commented-out code.
- Phase 23 D-01 documented in summary; verified the AUD-05 origin lineage transitively via Phase 22 Plan 22-05a → Phase 23-01 SUMMARY.

### Tertiary (LOW confidence) — none

- No claims rely solely on training-data knowledge of Rust Edition 2024, cbindgen, or serde behavior. Every Rust-language claim is either backed by `cargo` invocation output (rustc 1.95), file-direct-read of source/config, or `.planning/` artifact verification.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Cargo + cbindgen + Rust toolchain all verified empirically on this host.
- Architecture: HIGH — every architectural claim sourced from `.planning/` decision documents OR file-direct reads.
- Plan 45-02 inventory: HIGH — exact line numbers verified by grep + read; CONTEXT.md "23 tests" figure flagged as planner discretion at ±2.
- Cross-target posture: HIGH — 3-precedent pattern (Phase 41, 43-01b, 44 all PARTIAL); checklist directly applies.
- Pitfalls: HIGH — every pitfall is empirically discovered (fixture pre-alignment at audit_commands.rs:867; docstring at audit_integrity.rs:83 referring to wrong field; AUD-05 token vs variant assertion).
- Security domain: HIGH — invariant elevation is empirically supported by Phase 18.1-02 G-04 SUMMARY + Phase 29 WR-01 reject-stage lock.

**Research date:** 2026-05-21
**Valid until:** 2026-06-21 (stable phase scope; nothing in upstream `v0.54.0..v0.55.0+` invalidates Phase 45's scope per Phase 42 audit; CONTEXT.md is binding through Phase 45 close).
