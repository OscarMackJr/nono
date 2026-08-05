# Phase 112: Security + Residual Sync - Context

**Gathered:** 2026-08-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb the 18-commit `security-residual-and-misc` cluster from the upstream `v0.66.0..v0.69.0`
window — the commits that map to none of v3.6's original 12 requirements and that no other v3.6
phase covers — under a fork-invariant security review kept **distinct from any release-cut phase**
(the v3.1 Phase 87 precedent).

Requirements: **SEC-01..SEC-09, RES-01, RES-02** (11 IDs).

The work-list is fixed and already itemized by Phase 108. This phase decides each commit's
disposition (adopt / adapt / won't-sync) against the live fork, executes the confirmed-real
absorbs, and records the outcome durably. It does **not** add new capabilities, and it does
**not** cut a release.

**Cluster accounting (verified live against the ledger, 2026-08-05):** the 18 commits are fully
itemized with no orphans —
- 10 D-18-named security anchors → SEC-01..SEC-08
- `f6f027511f` → SEC-09
- `f050643479`, `7fe0c8283810`, `0158d52f0a`, `762eb05bd3d` → RES-01 (registry/update-check)
- `503045801a` + test-only `4cc0af2c52`, `9840a16f35` → RES-02 (PTY-teardown + test-infra)

10 + 1 + 7 = 18 ✓. There is no unassigned remainder for the planner to adjudicate.

</domain>

<decisions>
## Implementation Decisions

### Phase structure and review placement

- **D-02 (LOCKED):** **Reality-check pass FIRST.** Wave 1 diff-verifies each SEC-01..SEC-09 target
  against the live fork and assigns a disposition (adopt / adapt / won't-sync) **before** any
  absorb plan is written or executed. Absorb waves then execute only confirmed-real work. This is
  the phase's answer to ROADMAP SC1's "fork-invariant review distinct from any release-cut phase" —
  the review is a gate at the front, not a report at the end.
  *Rationale, evidence-backed:* SEC-09 was proven during this discussion to be phantom work (see
  D-01). Discovering that mid-execution would have burned an executor. Assume other SEC items may
  be similarly absent, already-present, or adapt-down.

- **D-04 (LOCKED):** The three heavyweights stay **inside Phase 112**, one plan each, each in its
  own wave; the six small/medium SEC items are batched. No ROADMAP amendment, no requirement
  re-mapping.
  | Req | SHA | Diffstat | Plan shape |
  |-----|-----|----------|------------|
  | SEC-02a | `9b692e07` | 26 files, +4,425 | own plan, own wave |
  | SEC-07  | `2663e990` | 15 files, +1,927 | own plan, own wave |
  | SEC-01  | `0ecc476b` | 11 files, +1,873 | own plan, own wave |
  | SEC-03  | `a3243907` | 15 files, +611   | batched |
  | SEC-02b/04/06/08 | `3c59c62e`/`f943fb5a`/`ac5ccd70`/`a5a441c2` | 124–262 each | batched |
  | SEC-05/09/02c | `d84b4818`/`f6f02751`/`d033c631` | 4–99 each | batched |
  *Expect a large phase — plausibly 8–10 plans.* That is accepted, not a planning defect.
  **However:** D-02's reality-check pass runs first precisely so that if SEC-01/02a/07 collapse to
  mostly won't-sync or heavy adapt-down against fork divergence, the planner MAY consolidate them
  rather than mechanically producing three large plans. Size here is an upper bound, not a target.

### Per-commit dispositions

- **D-01 (LOCKED):** **SEC-09 (`f6f027511f`, #1301) → won't-sync (target subsystem absent),
  PLUS an explicit v3.7 revisit obligation.**
  *Evidence (verified live during discussion, not inferred):* upstream deletes a guard block gated
  on `tool_sandbox_runtime.is_some() && !loaded_secrets.is_empty() && tool_sandbox_initial_shim.is_none()`.
  That block is **absent from the fork's `crates/nono-cli/src/execution_runtime.rs`**; the fork has
  **no `command_policies`** anywhere; and tool-sandbox is the standing divergence deferred to v3.7
  (PR #1105, never absorbed). There is no guard in this fork to relax.
  *The obligation is the point:* Phase 112 MUST add a carry-forward note to the v3.7 tool-sandbox
  work-list so that when tool-sandbox IS absorbed, the fork **consciously decides** whether to
  bring the guard with it. Without this, the fork silently inherits upstream's relaxed posture by
  default. This is the same dangling-TODO failure mode ADR-111 was written to close.
  *Do not treat "won't-sync" as "no work" — the carry-forward note IS the deliverable.*

- **D-03 (LOCKED):** **RES-01 and RES-02 are skip-biased.** Default to skip-with-recorded-reasoning;
  absorb **only** where a task diff-proves the change applies to the fork's divergent
  implementation. ROADMAP SC2 accepts skip as a first-class outcome — but "silently dropped" is
  forbidden, so every skip needs written reasoning.
  *Why skip-biased:* RES-01 (registry/update-check) collides with the fork's own HKLM
  machine-policy spine (`HKLM\SOFTWARE\Policies\nono`, v3.0) and its divergent update behavior.
  RES-02's PTY surface diverged hard across v2.7–v2.13 (Low-IL broker arm, no-PTY broker arm,
  `WindowsTokenArm::BrokerLaunchNoPty`).

- **D-07 (LOCKED):** **ROADMAP SC3 is already satisfied out-of-band — do NOT re-absorb `373a67ae`
  (#1369).** `Cargo.lock` carries `crossbeam-epoch 0.9.20` (verified 2026-08-05) and Phase 111-04's
  `cargo audit` reported **0 vulnerabilities** (6 pre-existing allowed advisory warnings).
  RUSTSEC-2026-0204 is closed. SC3's own text anticipates this branch: *"If already closed by an
  out-of-band quick task, record that and confirm `cargo audit` is clean rather than re-absorbing."*
  Deliverable is a recorded confirmation, not a dependency bump.

### Verification depth

- **D-06 (LOCKED):** Linux-only enforcement items (**SEC-03** NVIDIA procfs, **SEC-05** Landlock
  execute-restriction `Refer`, **SEC-06** seccomp supervisor-ancestry) are verified via **both
  mandatory cross-target clippy gates + `cross test --target x86_64-unknown-linux-gnu`** on the
  affected modules. **No live-kernel UAT checkpoint** — the phase stays autonomous end-to-end.
  *Precedent:* Phase 111-01 used exactly this mechanism to prove a Windows-uncompilable test
  actually passed (`exec_strategy.rs` is `#[cfg(not(target_os = "windows"))]`; the crypto-threads
  test was confirmed live via `cross test`, `ok. 1 passed`).
  *Known starting state for SEC-05:* the fork **already grants `AccessFs::Refer` generally** —
  `crates/nono/src/sandbox/linux.rs:365` includes it in an access set, and `:53`/`:108` reference
  it. Upstream `d84b4818` adds 89 lines granting `Refer` specifically in the **execute-restriction
  layer**. The reality-check pass must establish whether that layer specifically lacks it, rather
  than assuming the whole change is new.

### Recording and durability

- **D-05 (LOCKED):** Dispositions for all 18 commits are recorded as a **standing-divergence
  addendum in `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`**, extending
  the same mechanism Phase 111-03 established for `e6d26871`/`34c2c975`. The ledger is the
  canonical work-list and the first place a future UPST sync looks.
  *Note the ledger self-declares "Ledger closed"* — Phase 111 already set the precedent for an
  explicit, locked exception appended after closure. Follow that shape; do not edit the closure
  declaration away.
  *No ADR-112 required.* If the reality-check pass surfaces a disposition carrying genuine security
  judgment (in particular any **adopt** that loosens an existing fork guard), the planner SHOULD
  escalate that one decision to an ADR mirroring ADR-108/ADR-111's shape — but that is
  contingent, not planned up front.

- **D-08 (verified fact, not a choice):** The 18-commit cluster is fully itemized; there is no
  orphan set. Planner must not invent a "remainder" bucket.

### Claude's Discretion

- Exact wave count and plan-to-requirement grouping beyond D-04's constraints.
- Whether the six batched SEC items land in one plan or two, and how they are ordered.
- The concrete format of the reality-check disposition table (columns, per-commit evidence style),
  provided every one of the 18 SHAs appears with an explicit disposition and cited evidence.
- Whether SEC-02's three commits (`9b692e07` / `3c59c62e` / `d033c631`) are absorbed as one unit or
  staged, given `9b692e07` is 4,425 insertions and `d033c631` is 4.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The work-list (primary input — this phase exists to consume it)
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — the canonical 100-commit
  ledger. Read specifically: **Cluster Summary** table (§ line ~20, the `security-residual-and-misc`
  row), **Requirement Coverage Gap (D-18/D-19)** section (§ line ~63, the 27-commit gap and the full
  SHA list), the **security-residual-and-misc per-commit table** (§ line ~406), and the
  **Rollup Support Notes** (§ line ~536, which itemizes the 11-security / 7-residual split).
  Also the **Phase 111 Standing Divergence Addendum** (§ line ~1648) — the shape D-05 extends.

### Library-boundary and disposition precedent (governs adopt-vs-adapt)
- `proj/ADR-86-library-boundary-convergence.md` — library stays policy-free; the Windows denial-
  rendering carve-out (`crates/nono-cli/src/exec_strategy_windows/`) is deliberate, not drift.
- `proj/ADR-111-resource-limits-boundary.md` — the fork **ADAPTs rather than adopts** when upstream
  pushes policy/enforcement into the core `nono` crate. Directly relevant: several SEC items touch
  `crates/nono/src/`.
- `proj/ADR-108-deny-domain-posture.md` — the ADAPT precedent and the ADR shape D-05 would mirror
  if escalation is needed.

### Verification contract (D-06)
- `.planning/templates/cross-target-verify-checklist.md` — **single source of truth** for both
  cross-target gates, the pinned cross image tag, and the per-gate decision tree. Do not duplicate
  the runbook into plans.
- `./CLAUDE.md` § "Cross-target clippy verification" — the MUST/NEVER rule. PARTIAL→CI is the
  fallback **only** on a documented runner failure; a stopped Docker daemon or absent tool does
  NOT qualify.
- `.planning/phases/111-core-carry-resource-cli-verify-release-leapfrog/111-04-VERIFICATION-NOTES.md`
  — the current fork-invariant verification baseline this phase inherits, including the 24-name
  workspace-test baseline and the orchestrator addendum documenting 3 additional `profile_cmd.rs`
  shared-fixture flakes (fixed shared `%TEMP%\nono-test-profile-init`, passes 10/10 in isolation).

### Requirement and roadmap source
- `.planning/REQUIREMENTS.md` — SEC-01..SEC-09, RES-01, RES-02 definitions (lines ~130-140).
- `.planning/ROADMAP.md` § "Phase 112: Security + Residual Sync" — goal, dependencies, and SC1–SC4.

### Project security rules (non-negotiable)
- `./CLAUDE.md` § "Security Considerations" — fail-secure, least-privilege, path-component
  comparison, and the explicit instruction that **Landlock is strictly allow-list and cannot
  express deny-within-allow** (directly constrains SEC-05).

</canonical_refs>

<code_context>
## Existing Code Insights

### Verified present in the fork (probed live 2026-08-05)
All SEC target subsystems exist — there are no free "target subsystem absent" passes except the
one proven for SEC-09:

| Req | Fork target | Status |
|-----|-------------|--------|
| SEC-01 | `crates/nono-proxy/src/config.rs`, `crates/nono-cli/src/profile/mod.rs` | present |
| SEC-02 | `crates/nono-proxy/src/oauth2.rs`, `crates/nono/src/supervisor/socket.rs` | present |
| SEC-03 | `crates/nono/src/sandbox/linux.rs`, `crates/nono/src/capability.rs` | present |
| SEC-04 | `crates/nono/src/trust/bundle.rs` | present |
| SEC-05 | `crates/nono/src/sandbox/linux.rs` — `AccessFs::Refer` **already granted generally** (`:365`) | present, partial |
| SEC-06 | `crates/nono/src/sandbox/linux.rs` (seccomp) | present |
| SEC-07 | `crates/nono-proxy/src/{server,external,reverse,token}.rs`; **no `Proxy` variant in the `cli.rs` Commands enum** | proxy present, subcommand absent |
| SEC-08 | `crates/nono-cli/src/exec_strategy/env_sanitization.rs` (`allow_vars`) | present |
| SEC-09 | guard **ABSENT** from `crates/nono-cli/src/execution_runtime.rs`; no `command_policies` | **absent → D-01** |
| RES-01 | `crates/nono-cli/src/app_runtime.rs`, `main.rs`, `output.rs` (update-check) | present |
| RES-02 | `crates/nono-cli/src/exec_strategy_windows/{launch,mod}.rs`, `exec_strategy.rs` (PTY) | present |

### Integration points and hazards
- **`upstream` remote is fetched and all target SHAs resolve locally** (`git cat-file -t` verified
  for `0ecc476b`, `f6f02751`, `373a67ae`, `d84b4818`, `ac5ccd70`). Plans can `git show` any of the
  18 commits directly — no network fetch step needed.
- **`crates/nono/src/` is ADR-86/ADR-111 territory.** SEC-03/05/06 land in the core library. Any
  upstream change that pushes *policy* (not primitives) into core must be ADAPTed CLI-side.
- **SEC-07 adds a new CLI subcommand**, which is an *addition*, not a rename — D-04's flag-freeze
  (from Phase 111 CONTEXT) prohibits renames, not additions. Flagged so the planner does not
  mis-apply the freeze and skip SEC-07.
- **`proj/` and `docs/cli/` are gitignored but tracked** — any new ADR or `.mdx` edit needs
  `git add -f`. A plain `git add` exits 1 and breaks `&& git commit` chains.

### Established patterns to reuse
- **Split-requirement handling:** CORE-02 (111-02 + 111-03) and PROF-03 (Phase 110) both span
  multiple plans; `requirements.mark-complete` must NOT be run until every contributing plan lands.
  The SDK verb auto-flips on the plan's own frontmatter and has to be caught and reverted by hand.
- **Selective porting:** 111-01 ported exactly one line of a 6-file upstream commit and documented
  why the other 5 files did not apply. That is the expected shape for adapt dispositions here.
- **Executor discipline:** commit after every task. Two executors on this repo died mid-phase, one
  at 163k tokens having written nothing to disk.

</code_context>

<specifics>
## Specific Ideas

- The fork-invariant review is a **front gate**, not a trailing report (D-02). The user explicitly
  chose this over the literal "dedicated review plan at the end" reading of the Phase 87 precedent,
  on the evidence that SEC-09 was phantom work.
- SEC-09's deliverable is a **carry-forward note into the v3.7 tool-sandbox work-list**, not a code
  change. The user chose the option that creates a future obligation over the one that simply
  closes the requirement.
- Skips must carry **written reasoning** — "explicitly skipped with recorded reasoning, never
  silently dropped" is ROADMAP SC2's own language and the user's skip-biased choice reinforces it.

</specifics>

<deferred>
## Deferred Ideas

- **Tool-sandbox subsystem absorb (PR #1105, upstream v0.65.0)** — standing divergence, owns the
  **v3.7 (Windows Tool-Sandbox Parity)** milestone. Phase 112 must not absorb it. D-01's SEC-09
  carry-forward note is filed *against* this future work, not executed here.
- **Splitting SEC-01 / SEC-02a / SEC-07 into a follow-on phase** — considered and explicitly
  rejected (D-04). Revisit only if the D-02 reality-check pass shows the heavyweights do NOT
  collapse and 112 becomes unmanageable.
- **ADR-112** — not planned. Contingent escalation only, per D-05, if a disposition carries genuine
  security judgment.
- **Live-kernel UAT for Linux enforcement (SEC-05/SEC-06)** — considered and rejected for this
  phase (D-06). If a future phase stands up live-Linux verification, these are candidates for
  re-verification.
- **Fixing `profile_cmd.rs`'s shared-fixture flake** (fixed `%TEMP%\nono-test-profile-init` →
  `tempfile::TempDir`) — out of scope; recorded in Phase 111's UAT and verification addendum.
- **Phase 108 prose arithmetic defects** — the NET per-commit table's summary says "5 map `none`"
  where the table has 6, and PROF says "4" where the table has 3. Errors cancel; flagged in the
  ledger for whoever revisits Plans 108-03/108-04. Not this phase's work.

</deferred>

<reviewed_todos>
## Reviewed Todos (not folded)

Two pending todos matched Phase 112 on generic keyword overlap only; both are release/UAT-infra
items belonging to the v3.5 parallel milestone, not to this security-absorb phase:
- `20260611-msi-vcredist-prereq.md` — MSI VCRedist prerequisite (score 0.60, keyword match only)
- `20260611-poc-cert-broker-clean-host.md` — clean-host cert-broker PoC (score 0.60, keyword match only)

</reviewed_todos>

---

*Phase: 112-security-residual-sync*
*Context gathered: 2026-08-05*
