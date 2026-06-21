---
phase: 86-library-boundary-convergence
plan: "03"
subsystem: documentation
tags: [adr, documentation, library-boundary, claude-md, bnd-03]

dependency_graph:
  requires:
    - "86-01 (BND-01 — audit stack relocated to core)"
    - "86-02 (BND-02 — structured diagnostics to core + FFI)"
  provides:
    - "BND-03 — ADR-86 and CLAUDE.md boundary table update"
    - "proj/ADR-86-library-boundary-convergence.md — permanent record of boundary change"
    - "CLAUDE.md Library vs CLI Boundary table — updated to post-convergence reality"
  affects:
    - "Phase 87+ — future sync auditors can distinguish D-02 Windows carve-out from drift"
    - "nono-py + nono-ts — ADR documents stable FFI diagnostic surface"

tech_stack:
  added: []
  patterns:
    - "ADR-74 house style applied to ADR-86 (Status/Phase/Date/Authors header + Context/Decisions/Consequences/Alternatives)"
    - "CLAUDE.md table targeted-edit preserving unchanged rows"

key_files:
  created:
    - "proj/ADR-86-library-boundary-convergence.md"
  modified:
    - "CLAUDE.md"

decisions:
  - "ADR records all 5 BND-03 substance items: upstream boundary line, single fork carve-out (D-02 Windows), invariant change, new invariant, future-sync guidance"
  - "CLAUDE.md boundary table leading sentence updated from 'pure sandbox primitive' to 'applies ONLY what clients add to CapabilitySet — audit/diagnostics are observability primitives, not policy' (preserves the security principle without a factually false claim)"
  - "DiagnosticFormatter removed from In Library column; audit module + diagnostic/* added to In Library; diagnostic/formatter.rs + thin audit wrappers added to In CLI"
  - "D-02 carve-out note added as blockquote after the table with explicit ADR-86 reference"
  - "proj/ is .gitignored but existing files are force-tracked; ADR-86 added with git add -f (matches the established pattern for proj/*.md files)"

metrics:
  duration: "~30m"
  completed: "2026-06-19"
  tasks_completed: 2
  files_changed: 2
---

# Phase 86 Plan 03: BND-03 ADR and CLAUDE.md Update Summary

**One-liner:** Wrote ADR-86 documenting the library-boundary-convergence decisions (audit + diagnostics to core, D-02 Windows carve-out, future-sync guidance) and updated CLAUDE.md § Library vs CLI Boundary table to reflect the post-Wave-2 reality.

## Commits Produced

| Hash | Type | Description |
|------|------|-------------|
| `c42fd0d2` | docs | add ADR-86 library-boundary-convergence ADR |
| `f4c6ff24` | docs | update CLAUDE.md library-boundary table post convergence |

## Tasks Completed

### Task 1: proj/ADR-86-library-boundary-convergence.md

Created the boundary-convergence ADR following the ADR-74 house style. Covers all 5 BND-03 substance items from `86-RESEARCH.md § ADR Convention`:

1. **Upstream boundary line**: Cluster A (audit → core) + Cluster B (diagnostics → core + FFI) adopted verbatim from upstream v0.63.0–v0.64.0.
2. **Single deliberate fork carve-out**: Decision 3 explicitly names `exec_strategy_windows/` as the D-02 Windows denial rendering path — preserve-and-bridge, not converge.
3. **Invariant changed**: Prior "pure policy-free sandbox primitive with no audit/diagnostic logic" is partially retired.
4. **New invariant**: Library owns audit (integrity/ledger/attestation/merkle) + diagnostic facts/codes; CLI owns all rendering, UX, Windows denial paths (D-02).
5. **Future-sync guidance**: Callout paragraph explicitly states "Windows denial path is a deliberate fork carve-out — NOT drift" with re-evaluation guidance for the next upstream sync.

ADR also records Decision 4 (the `--workspace --all-targets` gate requirement from D-05), Consequences (FFI surface expansion, upstream sync friction reduction, cross-target PARTIAL), and 4 Alternatives Considered.

**Note on DCO:** The git commit carries `Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>` (matching the configured git user.name; the email is correct). All prior Phase 86 commits use the same git identity.

### Task 2: CLAUDE.md § Library vs CLI Boundary

Updated the boundary table to reflect post-Wave-2 reality:

- **Leading sentence** updated: "The library is a **pure sandbox primitive**" replaced with "The library applies ONLY what clients explicitly add to `CapabilitySet` — audit and diagnostics modules are observability primitives, not security policy" (preserves the policy-free security principle without a factually false architecture claim).
- **Added to In Library**: `` `audit` module (AuditRecorder, ledger append+verify, attestation sign/verify, merkle/inclusion-proof)`` and `` `diagnostic/*` (structured codes, diagnostic facts, SessionDiagnosticReport — NOT UX or rendering)``
- **Removed from In Library**: `DiagnosticFormatter` (now in CLI).
- **Added to In CLI**: `diagnostic/formatter.rs (DiagnosticFormatter, all rendering, flag suggestions)` and `audit_commands.rs, audit_session.rs (thin wrappers); audit business logic in core audit module`.
- **Added D-02 carve-out note** as a blockquote after the table, referencing `proj/ADR-86-library-boundary-convergence.md`.

## Gate Results

| Gate | Result |
|------|--------|
| `cargo build --workspace --all-targets` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo test -p nono` | 775 passed, 1 pre-existing failure (`try_set_mandatory_label` — env-specific, documented in MEMORY) |
| `cargo test -p nono-cli` | 1315 passed, 4 pre-existing failures (profile_cmd + 3 protected_paths — env-specific, documented in MEMORY) |
| `cargo test -p nono-ffi` | 47 passed, 0 failed |
| BND-01: `cargo test -p nono` (audit tests) | 11 audit tests PASS |
| BND-02: `bindings/c/src/diagnostic.rs` exists | PASS |
| BND-02: `crates/nono/src/diagnostic/` dir exists (6 files) | PASS |
| D-02: `exec_strategy_windows/` diff | 0 lines changed |
| Cherry-pick provenance | 8 "cherry picked from commit" lines in git log |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] proj/ directory is .gitignored**
- **Found during:** Task 1 commit attempt
- **Issue:** `git add proj/ADR-86-library-boundary-convergence.md` rejected with "ignored by .gitignore"
- **Fix:** Used `git add -f` — matches the established pattern for existing tracked files in `proj/` (`ADR-74-privilege-model.md`, `DESIGN-engine-abstraction.md`, `POC-zt-infra-e5-local-provisioner.md` are all force-tracked). The `.gitignore` comment reads "Not project state; generated and managed by GSD tooling at runtime" — clearly not the intent for ADR files.
- **No files modified:** git index only

**2. [Rule 1 - Documentation accuracy] "audit module" grep check fails on backtick-quoted form**
- **Found during:** Task 2 verification
- **Issue:** The plan's verification grep `grep -q "audit module"` fails because the table contains `` `audit` module `` (with backtick around `audit`). The content is correct and semantically accurate.
- **Fix:** None needed — the table is correct. The verification check is an approximate pattern; the actual content clearly shows the `audit` module in the In Library column.

## Known Stubs

None — this is a documentation-only plan. Both files are fully written with accurate, source-grounded content.

## Threat Flags

None — no network endpoints, auth paths, file access patterns, or schema changes introduced.

## Phase 86 Completion Summary

Phase 86 (Library-Boundary Convergence) is COMPLETE:
- **BND-01 (DONE):** Audit stack (AuditRecorder, ledger, merkle, attestation) relocated to `crates/nono/src/audit.rs`; 4 CLI files reduced to thin wrappers.
- **BND-02 (DONE):** Structured diagnostics model in core (6-file `diagnostic/` module); DiagnosticFormatter moved to CLI; 3 FFI extern-C fns; ProxyDiagnostic in proxy; NonoError gains diagnostic_code()/remediation().
- **BND-03 (DONE):** ADR-86 committed to `proj/`; CLAUDE.md boundary table updated.
- **D-02 (HONORED):** `exec_strategy_windows/` unchanged — 0 diff lines across all cherry-picks.
- **D-05 (HONORED):** `--workspace --all-targets` gate used throughout.
- **All 8 cherry-picks carry provenance** ("cherry picked from commit" markers in git log).

## Self-Check: PASSED

- [x] `proj/ADR-86-library-boundary-convergence.md` — FOUND
- [x] `grep "Status.*Accepted" proj/ADR-86-library-boundary-convergence.md` — OK
- [x] `grep "deliberate fork carve-out" proj/ADR-86-library-boundary-convergence.md` — OK
- [x] `grep "Windows denial path" proj/ADR-86-library-boundary-convergence.md` — OK
- [x] `grep "Future.*sync" proj/ADR-86-library-boundary-convergence.md` — OK
- [x] CLAUDE.md: `audit` module in In Library column — OK (backtick-quoted)
- [x] CLAUDE.md: `diagnostic/*` in In Library column — OK
- [x] CLAUDE.md: `diagnostic/formatter.rs` in In CLI column — OK
- [x] CLAUDE.md: D-02 carve-out note with ADR-86 reference — OK
- [x] CLAUDE.md: DiagnosticFormatter NOT in In Library column — CONFIRMED
- [x] Commits `c42fd0d2`, `f4c6ff24` in git log — FOUND
- [x] `cargo clippy --workspace --all-targets`: EXIT 0
- [x] `cargo fmt --all -- --check`: EXIT 0
