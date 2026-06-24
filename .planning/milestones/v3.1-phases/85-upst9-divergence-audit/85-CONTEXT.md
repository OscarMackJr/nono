# Phase 85: UPST9 Divergence Audit - Context

**Gathered:** 2026-06-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Produce a complete, disposition-resolved `DIVERGENCE-LEDGER.md` for the upstream
`always-further/nono` `v0.62.0..v0.64.0` window (90 commits / 140 files), built in the
Phase 42/47/48 ledger shape. Every **substantive** commit is classified into the themed
clusters **A–M** from SEED-006; each cluster carries a disposition
(`will-sync` / `fork-preserve` / `split` / `won't-sync`) and an ADR-style L/M/H risk
verdict across the five standard dimensions (security, windows, maintenance, divergence,
contributor). Cross-cluster re-export dependencies are diff-inspected with **actual diffs**
(not `git --name-only`) to structurally close the `feedback_cluster_isolation_invalid`
hazard.

This phase produces an **audit artifact only** — no code is cherry-picked, moved, or
modified. The ledger is the gating input for the downstream cherry-pick phases:
86 (boundary convergence, themes A & B), 87 (security, theme C), 88 (feat+deps, themes
D/E/G/H/I/J/K/L/M + bumps), 89 (proxy, theme F).

**Out of scope (belongs to later phases):** any actual cherry-pick / code relocation; the
full boundary-convergence ADR (that is Phase 86 BND-03 — Phase 85 only records inline
ADR-style *risk verdicts*, not the decision rationale doc); the crate version leapfrog
(release-time, ≥ `0.65.0`).

</domain>

<decisions>
## Implementation Decisions

### Ledger granularity
- **D-01:** Dispositions are recorded at **cluster level** (one disposition + one ADR-style
  L/M/H risk verdict per theme A–M), each with a **nested per-commit SHA inventory** listing
  every substantive commit folded into that cluster. Matches the Phase 42/47/48 shape.
- **D-02:** A `split` cluster disposition is the mechanism for mixed-fate themes: when some
  commits in a cluster sync and others don't (e.g. theme M's `env_clear`-removal commit
  `e54cf9cb` vs the additive misc fixes), the cluster is marked `split` and the per-commit
  inventory annotates which commits are the carve-outs. No separate per-commit disposition
  column — the inventory note carries it.

### Disposition pre-commitment
- **D-03:** Themes **A & B** are **locked** to `will-sync / adopt-upstream` (milestone Key
  Decision — deliberately moving the audit stack + structured-diagnostics into the core
  `nono` crate, changing the policy-free-library boundary). Not re-litigated in the audit.
- **D-04:** Pre-lean the obvious dispositions and let the diff-inspection **confirm or
  overturn** them — do not start every non-A/B cluster from zero. Recorded leanings to carry
  into the audit (each MUST be validated, not assumed):
  - **C** (Linux AF_UNIX bypass #1096 + procfs-remap dedup #1064) → lean `will-sync`
    (security, must absorb).
  - **F** (proxy route/403/TLS-CONNECT/reactive-auth/customCredentials) → lean `split` /
    diff-careful — touches the **fork-divergent TLS-interception surface** (Phase 34 C11
    `fork-preserve`); the additive route/403 bits likely sync, the TLS-intercept bits need
    reconciliation.
  - **M** (misc) → lean `split` — the `env_clear` removal (`e54cf9cb`) collides with the
    fork's Windows `SystemRoot`/`windir` CLR baseline (`windows_hook_interpreter_spawn_gotchas`);
    most other misc fixes are additive `will-sync`.
  - **D/H/I/K/L** (set_vars, keyring timeout, $PACK_DIR hooks, update-check CI, profile
    namespace) → lean `will-sync` (additive, low conflict).
  - **E/G** (XDG state dirs, AWS auth) → lean `will-sync` with a Windows-path / mutual-exclusion
    reconciliation note.

### Diff-inspection scope (re-export hazard)
- **D-05:** Full **actual-diff** re-export inspection is **targeted at the shared-surface
  clusters**: A, B (boundary refactors — highest re-export risk), the diagnostic-touching
  surfaces (B's FFI + proxy `ProxyDiagnostic` + `error.rs`), and F (proxy TLS-intercept).
  `git --name-only` is sufficient for the clearly-additive feature clusters (D/H/I/K and the
  dependency bumps). The ledger must state, per cluster, which inspection depth was applied
  so the completeness of the hazard-closure is auditable.

### Noise-commit handling
- **D-06:** The ~77 non-substantive commits (dependabot / docs / merge) are documented in an
  **explicit "Excluded as noise" ledger section** stating the exclusion filter criteria plus
  the count and SHAs (or SHA ranges) excluded. The "every substantive commit is classified"
  completeness claim must be **independently verifiable** — a reader can confirm nothing
  substantive was silently dropped.

### Re-fetch / window boundary
- **D-07:** Window stays `v0.62.0..v0.64.0`. Upstream's highest tag at discuss-time is
  `v0.64.0` (verified via `git ls-remote --tags upstream` — no `v0.65.0` exists yet). The
  re-fetch at audit-open (SC#2) is a tip-check formality; **only if** upstream cuts a new tag
  before execution does the window extend, and the ledger must note the re-fetch result either
  way. A future release leapfrogs the crate version to ≥ `0.65.0` (release-time, not this phase).

### Claude's Discretion
- Exact column layout / table format of the ledger (follow the Phase 42/47/48 convention as
  closely as the archived shape allows — note: prior ledger files were archived out of the
  live planning tree, so reconstruct the shape from SEED-006 + the success criteria).
- How to bucket/order the noise SHAs (ranges vs enumerated) — whichever is most legible.

### Reviewed Todos (not folded)
Four todos surfaced via `todo.match-phase 85`, all weak (score 0.4, generic "phase"/"must"/"high"
keyword hits) and all unrelated to an upstream divergence audit — **none folded** (folding would
be scope creep):
- `20260611-msi-vcredist-prereq.md` — MSI VC++ runtime prereq (v2.10/v2.11 host-deploy debt).
- `20260611-poc-cert-broker-clean-host.md` — untrusted-POC-cert broker on clean host.
- `20260612-macos-rlimit-as-setrlimit-fails.md` — macOS RLIMIT_AS/setrlimit defect.
- `20260618-phase83-codereview-deferred.md` — deferred Phase 83 code-review.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents (researcher, planner) MUST read these before planning or implementing.**

### Scope source (the audit input)
- `.planning/seeds/SEED-006-upst9-v0.62-v0.64-sync-window.md` — the authoritative theme
  decomposition (A–M), per-theme new/modified function inventory, upstream SHAs, dependency
  bump list, and the fork-conflict notes. This is the primary worksheet for the ledger.

### Milestone framing
- `.planning/ROADMAP.md` § Phase 85 — Goal + 4 success criteria (the ledger acceptance bar).
- `.planning/REQUIREMENTS.md` — AUDIT-01 (ledger exists, per-cluster dispositions, re-fetch),
  AUDIT-02 (ADR-style L/M/H verdicts + diff-inspected re-export deps).
- `.planning/STATE.md` § Accumulated Context — the locked v3.1 milestone decisions (A&B
  adopt-upstream, phase sequencing, dependencies between phases).

### Library-boundary invariant (themes A & B context)
- `CLAUDE.md` § "Library vs CLI Boundary" — the policy-free-library invariant that A & B
  deliberately change; the table of what is library-side vs CLI-side today.
- `CLAUDE.md` § Coding Standards — Cross-target clippy MUST/NEVER rule (theme C is Linux-only
  cfg-gated; affects how its risk verdict is written).
- `.planning/templates/cross-target-verify-checklist.md` — the PARTIAL→deferred-to-CI escape
  for cfg-gated Unix edits (relevant to theme C's verdict, executed in Phase 87).

### Fork hazards to honor in the audit method
- `feedback_cluster_isolation_invalid` (memory) — WHY re-export surfaces must be diff-inspected
  with actual diffs, not `--name-only`; the structural reason SC#4 exists.
- `windows_hook_interpreter_spawn_gotchas` (memory) — the fork's Windows `env_clear` /
  `SystemRoot`/`windir` CLR baseline that theme M's `e54cf9cb` collides with.
- `project_v28_opened` (memory) — the fork-version-leapfrog rule (tag PAST upstream's highest).

### Process precedent
- Phase 42/47/48 `DIVERGENCE-LEDGER.md` shape (files archived out of the live tree — reconstruct
  from SEED-006 + success criteria; no live path available).

</canonical_refs>

<code_context>
## Existing Code Insights

### Current fork state (verified at discuss-time — grounds A & B divergence)
- **Audit is CLI-side** (theme A divergence confirmed): `crates/nono-cli/src/audit_attestation.rs`,
  `audit_commands.rs`, `audit_integrity.rs`, `audit_session.rs`. There is **no** `crates/nono/src/audit.rs`
  in the fork today — upstream A moves ~1773 LOC into the core crate.
- **Diagnostics are split** (theme B divergence): the fork already has `crates/nono/src/diagnostic.rs`
  (core) AND `crates/nono-cli/src/diagnostic_formatter.rs` (CLI). Upstream B introduces a full
  `crates/nono/src/diagnostic/` **module** (codes/observation/records/report/detail/mod), plus
  `NonoError::{diagnostic_code, remediation}` in `error.rs`.
- **No proxy or FFI diagnostic surface yet**: `crates/nono-proxy/src/diagnostic.rs` and
  `bindings/c/src/diagnostic.rs` do **not** exist in the fork — both are net-new in theme B.
- Upstream remote is configured (`upstream → github.com/always-further/nono`), so re-fetch +
  `git log v0.62.0..v0.64.0` / `gh api .../compare/v0.62.0...v0.64.0` are directly runnable.

### Established patterns
- The audit is a documentation/research deliverable; the executor writes `.md`, runs `git`/`gh`
  diff commands, and does NOT touch source. The 5-crate workspace version-pin discipline
  (`project_workspace_crates`) is only *recorded* here (for the dep-bump cluster) — applied in Phase 88.

### Integration points
- The ledger's per-cluster dispositions are the literal input contract for Phases 86–89: each
  downstream phase reads its theme's row(s). Disposition vocabulary and cluster IDs must match
  exactly what those phases reference (A→86, B→86, C→87, F→89, rest→88).

</code_context>

<specifics>
## Specific Ideas

- Reconstruct the ledger table shape from the Phase 42/47/48 precedent referenced in SEED-006;
  the archived ledger files are no longer in the live planning tree, so the success-criteria +
  SEED structure are the source of truth for format.
- Ledger lives in the phase directory: `.planning/phases/85-upst9-divergence-audit/DIVERGENCE-LEDGER.md`
  (or `85-DIVERGENCE-LEDGER.md` — planner to confirm naming against SDK conventions).

</specifics>

<deferred>
## Deferred Ideas

- **Full boundary-convergence ADR** (rationale + what stays CLI-side) → Phase 86 (BND-03).
  Phase 85 records only inline ADR-style L/M/H *risk verdicts* per cluster.
- **Actual cherry-pick / code relocation** for every theme → Phases 86–89 per disposition.
- **Crate version leapfrog to ≥ 0.65.0** → release-time, post-sync (not in v3.1's marker-only scope).

### Reviewed Todos (not folded)
See the "Reviewed Todos (not folded)" subsection under `<decisions>` — four weak-match,
unrelated host-execution/enterprise deferrals reviewed and deliberately left for their own
cadence.

</deferred>

---

*Phase: 85-upst9-divergence-audit*
*Context gathered: 2026-06-19*
