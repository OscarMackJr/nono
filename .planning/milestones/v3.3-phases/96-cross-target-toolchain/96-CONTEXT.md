# Phase 96: Cross-Target Toolchain - Context

**Gathered:** 2026-06-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Stand up the cross-target clippy gates **locally on this Windows dev host** so the
fork stops auto-deferring them to CI:

- Make the `x86_64-unknown-linux-gnu` clippy gate **provably runnable locally** and run it
  against the post-Phase-95 synced tree, fixing any drift it surfaces in cfg-gated Unix code.
- **Resolve** the `x86_64-apple-darwin` gate to one of two explicit end-states: passing locally,
  or a written hard-blocker record that commits it to PARTIAL→CI with rationale.
- Update CLAUDE.md + `.planning/templates/cross-target-verify-checklist.md` to retire the
  PARTIAL→CI *default* for the gate(s) now provably runnable, per-gate and evidence-based.

**Scope anchor (XTGT-01..04).** This phase is about the *toolchain + protocol*, not about
re-running the Phase 95 absorb. The "Phase 95 cross-target clippy note" carried forward on every
will-sync/split commit (Phase 95 D-03) is the drift surface this phase clears.

**Host reality discovered during scout (do not re-derive):**
- `rustup` already has both `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin` std targets.
- `cross` 0.2.5 + Docker 29.5.3 are installed → the linux-gnu path is expected to be runnable now.
- No `osxcross`, no `cargo-zigbuild`, no `zig` → apple-darwin is the genuinely hard target
  (macOS SDK licensing from a Windows host).

Out of scope (deferred — see `<deferred>`):
- Crate leapfrog (≥0.65.0), release pipeline, runbook → **Phase 97**.
- Cross-target verification of the `nono-py` / `nono-ts` binding repos (separate repos; this
  phase covers the **workspace** cfg-gated Unix surface only).

</domain>

<decisions>
## Implementation Decisions

### Linux-gnu mechanism
- **D-01:** **`cross clippy` is the canonical mechanism.** Document and run
  `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`.
  cross 0.2.5 + Docker 29.5.3 are already installed; the cross image ships
  `x86_64-linux-gnu-gcc` so the C-linking crates (`aws-lc-sys`, `ring`) link cleanly. Zero new
  host install; reproducible via the pinned cross image. (Chosen over a native gnu-gcc linker
  install and over WSL2.)
- **D-02:** **The `cross` form replaces the bare `cargo clippy --target` string** in CLAUDE.md and
  the checklist. It **discharges SC#2's `cargo clippy` contract** because `cross clippy` runs
  `cargo clippy` *inside* the pinned Linux container — same lints, real Linux cfg branches. The
  bare `cargo clippy --workspace --target x86_64-unknown-linux-gnu ...` string is NOT independently
  runnable on this host (no native linker), so it must not remain the documented "runnable" command.

### apple-darwin disposition
- **D-03:** **Time-boxed single-path attempt, then hard-blocker.** Evaluate exactly ONE approach
  (cargo-zigbuild first — clippy needs the macOS SDK only for *linking*, and zig can stub much of
  that). Capped at **one plan / one wave** of effort.
- **D-04:** **Stop conditions (whichever hits first):**
  (a) the bounded single-path effort produces no clean `clippy` exit by end of its plan/wave, OR
  (b) any path requires **acquiring/extracting the proprietary macOS SDK on the Windows host** —
  stop immediately on licensing grounds, do not attempt the extraction.
  On stop → write the **XTGT-03(b) hard-blocker record** (osxcross/SDK-from-Windows infeasibility,
  with rationale) and commit apple-darwin to **PARTIAL→CI**. If the bounded attempt *does* yield a
  clean clippy run, apple-darwin flips to local-runnable instead.

### Drift-fix scope
- **D-05:** **Fix ALL drift the linux-gnu gate surfaces** — including upstream-inherited lints —
  because this is the first local run of the gate and SC#2 requires it to **exit 0** under
  `-D warnings -D clippy::unwrap_used`. **No `#[allow(...)]` silencing** of cross-target lints
  (checklist Anti-pattern 2 / CLAUDE.md Unwrap Policy); use cfg-gates, visibility changes, or
  structural fixes. There is no no-new-since-baseline bound here — a green gate is the deliverable.

### Doc home + retirement (XTGT-04)
- **D-06:** **Setup + canonical invocation live in
  `.planning/templates/cross-target-verify-checklist.md`** (it already owns the "Cross-Toolchain
  Setup" section and the PARTIAL disposition being retired). CLAUDE.md gets a **one-line pointer**,
  not a duplicate runbook.
- **D-07:** **Retirement is per-gate and evidence-based.** Rewrite the checklist decision tree so:
  - **linux-gnu** → "MUST run locally via `cross clippy`; PARTIAL only on a *documented* Docker/cross
    failure" (the auto-default-to-PARTIAL is retired for this gate).
  - **apple-darwin** → disposition follows D-03/D-04's outcome: stays explicitly PARTIAL→CI *with the
    hard-blocker rationale* if the bounded attempt fails, or also flips to local-required if it passes.
  Do NOT retire both gates' default unconditionally — the checklist must not contradict the actual
  apple-darwin outcome.

### Claude's Discretion
- Pinning/recording the exact `cross` image tag, plan/wave decomposition, the precise cargo-zigbuild
  invocation tried, and whether to wire the linux-gnu gate into a `make` target are the
  planner's/researcher's call, subject to the locked decisions above.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The protocol being retired/rewritten (READ FIRST)
- `.planning/templates/cross-target-verify-checklist.md` — the PARTIAL→CI deferral protocol, the
  Q1/Q2/Q3 decision tree, the "Cross-Toolchain Setup" section, the PARTIAL Disposition prose, and
  the Anti-Patterns list. D-06/D-07 rewrite this file; it is also where setup lives.
- `CLAUDE.md` §Coding Standards → "Cross-target clippy verification" bullet — the MUST/NEVER rule
  that names the `cargo clippy --workspace --target ...` invocations; D-02 updates it to the `cross`
  form + a pointer to the checklist.

### What this phase is clearing (the drift surface)
- `.planning/phases/95-upstream-absorb-fork-invariant-verify/95-CONTEXT.md` §D-03 — the decision that
  deferred the cross-target gates to Phase 96 and carried a "Phase 95 cross-target clippy note" on
  every will-sync/split commit touching cfg-gated Unix code.
- `.planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md` — per-commit `windows-touch`
  / cfg-gated annotations that indicate which absorbed commits to expect drift from.

### Requirements
- `.planning/REQUIREMENTS.md` — XTGT-01 (toolchain installed + documented), XTGT-02 (linux-gnu clippy
  passes; drift fixed in-milestone), XTGT-03 (apple-darwin passes OR documented hard-blocker →
  PARTIAL→CI), XTGT-04 (CLAUDE.md + checklist updated, default retired for runnable gates).
- `.planning/ROADMAP.md` §Phase 96 — Success Criteria 1–4 (the literal pass/fail contract).

### Cfg-gated Unix scope (what the gate exercises)
- `crates/nono/src/sandbox/{linux,macos}.rs`, `crates/nono-cli/src/exec_strategy/`,
  `bindings/c/src/` — the in-scope file classes from the checklist's § Scope; the linux-gnu gate
  must compile + lint these clean.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`cross` 0.2.5 + Docker 29.5.3 (installed)** — the runner for the linux-gnu gate (D-01). No new
  host toolchain install required for linux-gnu.
- **Both rustup std targets already added** (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`) —
  the "rustup target add" half of the checklist's one-time setup is already done.
- **`.planning/templates/cross-target-verify-checklist.md`** — the doc being rewritten already
  contains the setup section, decision tree, and PARTIAL prose to edit in place (D-06/D-07).

### Established Patterns
- `make ci` (clippy + fmt + tests) before push — clippy-green ≠ rustfmt-clean (see memory
  `feedback_fmt_check_in_verify_gate`); the new gate is additive to, not a replacement for, `make ci`.
- No raw `#[allow(clippy::unwrap_used)]` / `#[allow(dead_code)]` to silence lints (CLAUDE.md +
  checklist Anti-pattern 2) — drift is fixed structurally (D-05).
- DCO sign-off on every commit: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.

### Integration Points
- The gate's verdict feeds the verifier: per the checklist, a runnable-and-green gate lets a
  Unix-touching REQ flip to VERIFIED instead of PARTIAL→CI. D-07's rewrite changes which gates the
  verifier may flip locally.
- Phase 97 depends on this: its release tree must pass the same cross-target verification, so the
  retired-default rewrite (D-07) is the protocol Phase 97 inherits.

</code_context>

<specifics>
## Specific Ideas

- Canonical linux-gnu command (verbatim): `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`.
- apple-darwin first-attempt tool: **cargo-zigbuild** (rationale: clippy needs the macOS SDK only for
  linking; zig can stub much of it — best evidence-to-effort path before declaring a blocker).
- Licensing wall is the hard stop: no extraction of the proprietary macOS SDK on the Windows host.

</specifics>

<deferred>
## Deferred Ideas

- **Crate leapfrog ≥0.65.0, signed MSI/wheel/npm pipeline, operator runbook** → Phase 97 (XTGT phase
  verifies the toolchain; Phase 97 consumes it across the release tree).
- **Cross-target verification of `nono-py` / `nono-ts`** (separate binding repos) — out of scope for
  this workspace-focused phase; revisit if those repos grow cfg-gated Unix surface.
- **Wiring the linux-gnu gate into a dedicated `make` target / CI parity job** — noted as possible
  planner discretion, not a locked requirement.

</deferred>

---

*Phase: 96-cross-target-toolchain*
*Context gathered: 2026-06-26*
