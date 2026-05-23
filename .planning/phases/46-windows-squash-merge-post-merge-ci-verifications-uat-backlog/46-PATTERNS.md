# Phase 46: windows-squash merge + post-merge CI verifications + UAT backlog — Pattern Map

**Mapped:** 2026-05-23
**Files analyzed:** 5 NEW + 3 UPDATE = 8 destinations
**Analogs found:** 8 / 8 (all destinations have a named canonical analog in CONTEXT.md `<canonical_refs>`)

> **Planner orientation.** Each destination below carries (a) classification, (b) the named analog, (c) the concrete excerpts to copy, and (d) any planner-discretion forks called out in CONTEXT.md `<decisions>`. Excerpts are inline — planner does not need to re-open the analogs.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `.planning/architecture/v2.6-upstream-merge-deferral-ADR.md` (NEW) | architecture-decision-record | doc, scoring + decision + consequences | `docs/architecture/upstream-parity-strategy.md` | role-match (different dir per planner discretion fork) |
| `.github/workflows/phase-46-uat-backlog.yml` (NEW) | ci-workflow | workflow_dispatch matrix → cargo test | `.github/workflows/phase-45-resl-native-host.yml` | exact (D-46-C2 mirror) |
| `.planning/phases/35-upst3-closure-quick-wins/35-HUMAN-UAT.md` (NEW) | uat-record | doc, frontmatter + per-scenario test rows | `.planning/phases/37-linux-resl-backends-pkgs-auto-pull/37-HUMAN-UAT.md` | exact (per D-46-C4) |
| `.planning/phases/35-upst3-closure-quick-wins/35-VERIFICATION.md` (NEW) | verification-record | doc, frontmatter + truths + req coverage | `.planning/phases/37-linux-resl-backends-pkgs-auto-pull/37-VERIFICATION.md` | exact (per D-46-C4) |
| `.planning/phases/36-upst3-deep-closure/36-HUMAN-UAT.md` (NEW) | uat-record | doc, frontmatter + per-scenario test rows | `.planning/phases/37-linux-resl-backends-pkgs-auto-pull/37-HUMAN-UAT.md` | exact (per D-46-C4) |
| `.planning/phases/36-upst3-deep-closure/36-VERIFICATION.md` (NEW) | verification-record | doc, frontmatter + truths + req coverage | `.planning/phases/37-linux-resl-backends-pkgs-auto-pull/37-VERIFICATION.md` | exact (per D-46-C4) |
| `.planning/quick/260428-rsu-refresh-stack-onto-upstream-tip/260428-rsu-SUMMARY.md` (UPDATE) | summary-status-flip | doc, frontmatter status field flip | self (current file) + terminal-state pattern from `260424-mrg-merge-windows-squash-to-main/SUMMARY.md` | exact + role-match |
| `.planning/templates/upstream-sync-quick.md:102` (UPDATE) | template-baseline-registry | doc, single-line SHA replacement | self (read the 3-line block; identical pattern) | exact (literal line replace) |
| `.planning/REQUIREMENTS.md` (UPDATE) | requirements-checkbox-flip | doc, `[ ]` → `[x]` per REQ row | self (REQ-PORT-CLOSURE-08 + REQ-RESL-NIX-04 + REQ-REVIEW-FU-01 are already `[x]` examples in file) | exact (literal char flip) |

---

## Pattern Assignments

### `.planning/architecture/v2.6-upstream-merge-deferral-ADR.md` (NEW; architecture-decision-record)

**Analog:** `docs/architecture/upstream-parity-strategy.md` (Phase 33 ADR — closest precedent per D-46-A2; the existing ADR shape is named explicitly in CONTEXT.md `<canonical_refs>`).

**Planner discretion fork (CONTEXT.md `<decisions>` § Claude's Discretion bullet 1):** The decision document names the path `.planning/architecture/v2.6-upstream-merge-deferral-ADR.md` but observes that the EXISTING ADRs all live in `docs/architecture/` and lack the `-ADR.md` suffix. Planner picks: keep the literal D-46-A2 path (creates `.planning/architecture/` as a new directory) OR conform to existing convention at `docs/architecture/v2.6-upstream-merge-deferral.md`. The latter has stronger precedent (4 existing ADRs at that path); the former honors D-46-A2 verbatim. CONTEXT.md `<canonical_refs>` confirms `.planning/architecture/` does not yet exist; this would be the first file there.

**Header pattern** (analog lines 1-8 — six required header fields, plain-text shape per D-33-C4):

```markdown
# Upstream Parity Strategy (continue / split-windows / freeze-at-v0.52)

**Status:** Accepted
**Date:** 2026-05-11
**Phase:** 33 (v2.4 windows-parity-upstream-0-52-divergence)
**Decision IDs:** D-33-A1, D-33-A2, D-33-A3, D-33-B1, D-33-B2, D-33-B3, D-33-C1, D-33-C2, D-33-C3, D-33-C4, D-33-D1, D-33-D2
**Related artifact:** [DIVERGENCE-LEDGER.md](../../.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md)
```

For Phase 46 the analog substitutions are:
- Title: e.g. `# v2.6 Upstream Merge Deferral (feature-flag-equivalent rollout for windows-squash → main)`
- **Decision IDs:** `D-46-A1, D-46-A2, D-46-A3, D-46-A4`
- **Related artifact:** back-reference to `260428-rsu-SUMMARY.md` + `260428-rsu-CONTEXT.md` + `260428-rsu-PLAN.md` (the abandoned-path runbook per CONTEXT.md `<canonical_refs>` REQ-MERGE-01 sources).

**Six-section structure** (analog headings — copy this exact ordering per D-33-C4 "convention match"):

```markdown
## Context
## Goals
## Non-goals
## Decision Table          ← Markdown table with per-option L/M/H per criterion
## Decision               ← Chosen option + reasoning + reversibility note
## Consequences           ← ### Positive / ### Negative / ### Future audit cadence subsections
## Alternatives Considered  ← One ### per rejected option, narrative form
## References             ← ### Internal / ### Related ADRs (convention references) subsections
```

**Decision Table pattern** (analog lines 47-53 — per-option scored row with Verdict cell; D-33-C1 + D-33-C2 qualitative L/M/H scoring):

```markdown
| Option | Maintenance cost | Security posture | User clarity | Contributor velocity | Roadmap optionality | Verdict |
|--------|-----------|------------------|--------------|---------------------|---------------------|---------|
| **A (chosen) — Continue bidirectional parity** | **Med** — Per-sync labor sustains: ... | **High** — Continued parity preserves the option to evolve ... | **High** — Single CLI surface ... | **Med** — Drift-audit + cherry-pick gate ... | **High** — All v2.4+ doors stay open ... | **Accepted** |
| B — Split Windows into nono-windows fork | **Low** — ... | **High** — ... | **Low** — ... | **Low** — ... | **Low** — ... | Rejected: split foreclosure cost > parity labor saving. ... |
| C — Freeze fork at v0.52, stop chasing upstream | **Low** — ... | **Med** — ... | **Med** — ... | **High** — ... | **Low** — ... | Rejected: forecloses upstream security flow-in. ... |
```

For Phase 46 the three options to score are named in CONTEXT.md `<decisions>` D-46-A1: (a) feature-flag-equivalent rollout = ACCEPTED, (b) re-poll maintainer + decide on response = REJECTED, (c) resume 260428-rsu force-rebase = REJECTED. Planner picks the criterion columns; suggested mirror of Phase 33's 5-column shape (maintenance-cost / security-posture / user-clarity / contributor-velocity / roadmap-optionality) for cross-ADR consistency.

**Future audit cadence (Consequences subsection) pattern** (analog line 96):

```markdown
### Future audit cadence

Every upstream minor release (v0.53.0, v0.54.0, ...) triggers a drift audit via `make check-upstream-drift ARGS="--from v0.52.0 --to v0.5N.0 --format json"` ... The audit cadence is "per upstream release, lazily-evaluated"; if upstream goes quiet for a quarter, no audit fires; ...
```

For Phase 46 this section codifies **D-46-A3 maintainer-response triggers only** (no fork-side calendar, no drift-quantification trigger). Suggested four-bullet shape:

```markdown
### Revival triggers (maintainer-response only)

Resume scope determined at trigger time. ANY of:
1. Maintainer comments on PR 725 or 726 with directional guidance (rebase, close-and-restage, alternate approach).
2. Maintainer takes substantive action on either PR (review submitted, label change, close, merge).
3. Maintainer requests a different approach via issue / discussion / direct comm.
4. (Explicitly NOT a trigger: v3.0 milestone calendar, drift-quantification threshold — per D-46-A3 these were considered and rejected.)
```

**Per-phase umbrella PR codification pattern** (D-46-A4 — new content; no direct analog cell in Phase 33 ADR but the shape mirrors the "Future audit cadence" subsection):

```markdown
### Go-forward upstream-contribution mode (per-phase umbrella PR)

While PRs 725/726 remain held, the fork's upstream contribution mode is the per-phase umbrella PR pattern (memory `project_cross_fork_pr_pattern`). Precedent: Phase 22 (UPST2), Phase 33 (this ADR's parent phase), Phase 39, Phase 42, Phase 43 (umbrella PR opened by Phase 46 Plan 46-02). PR 922 (Phase 40) is the live exemplar. GitHub's one-PR-per-branch-pair rule means per-plan upstream PRs require per-plan feature branches; the umbrella pattern collapses N plans into 1 PR per phase.
```

**References pattern** (analog lines 116-131 — two subsections):

```markdown
## References

### Internal
- [`33-SPEC.md`](../../.planning/phases/.../33-SPEC.md) — Locked requirements REQ-1..5 ...
- [`33-CONTEXT.md`](../../.planning/phases/.../33-CONTEXT.md) — Decisions D-33-A1..D2 ...
- [`PROJECT.md` § Key Decisions](../../.planning/PROJECT.md) — Where this decision's summary row lives ...

### Related ADRs (convention references)
- [`audit-bundle-target.md`](audit-bundle-target.md) — Phase 27.2 ADR (closest convention match ...).
- [`upstream-parity-strategy.md`](upstream-parity-strategy.md) — Phase 33 ADR (per-option scoring/verdict pattern).
```

For Phase 46, the Internal subsection should reference: `46-CONTEXT.md` (this file), `260428-rsu-SUMMARY.md` + `-CONTEXT.md` + `-PLAN.md`, `REQUIREMENTS.md § REQ-MERGE-01`, `ROADMAP.md § Phase 46 SC#1`, `PROJECT.md § v2.6 UPST6 + v2.5 Drain`. The Related ADRs subsection should cite `upstream-parity-strategy.md` (the parent Phase 33 ADR), and the other 4 existing ADRs at `docs/architecture/`.

---

### `.github/workflows/phase-46-uat-backlog.yml` (NEW; ci-workflow)

**Analog:** `.github/workflows/phase-45-resl-native-host.yml` (D-46-C2 mirror; CONTEXT.md `<canonical_refs>` names this verbatim as the Plan 46-03 template).

**File-level comment header pattern** (analog lines 1-17 — purpose + tactical-by-design + deletion target + invocation example):

```yaml
# Phase 45 — Native RESL re-validation (REQ-RESL-NIX-04)
#
# Tactical confirmation pass: verifies the Phase 27.2 audit-attestation
# transitive-closure (REQ-AAHX-01..03) holds on a native Linux + macOS host.
# Closes the Phase 38 REQ-AAHX-HOST-01 deferral folded into v2.6 as
# REQ-RESL-NIX-04 per ROADMAP § Phase 45 (success criterion 3).
#
# workflow_dispatch-only (NOT a permanent CI lane) per D-45-D2.
# Deletable in v2.7 once the verdict is recorded in
# 45-03-NATIVE-RESL-PROTOCOL.md § Closure Disposition.
#
# Invocation (Phase 46 orchestrator action):
#   gh workflow run phase-45-resl-native-host.yml -f gh_runner_os=both
#   gh run watch
#
# SC#3 explicitly says this REQ does not block phase close if no gap is
# found. Both jobs carry continue-on-error so one OS green is sufficient.
```

For Phase 46 the substitutions are:
- Title: `Phase 46 — UAT backlog drain (REQ-UAT-BL-01..02)`
- Purpose: "Tactical re-execution of Phase 35 + 36 HUMAN-UAT + VERIFICATION items deferred at v2.4 close; per D-46-C2 workflow_dispatch-only, deletable in v3.0 once verdicts are recorded in `46-03-SUMMARY.md`."
- Deletable: `v3.0` (planner verifies milestone numbering)
- Invocation comment: matches the planner-picked workflow_dispatch input shape (see below)

**workflow_dispatch input pattern** (analog lines 21-31 — `gh_runner_os: { type: choice, options: [...], default: both }`):

```yaml
on:
  workflow_dispatch:
    inputs:
      gh_runner_os:
        description: Which OS matrix to run
        type: choice
        options:
          - ubuntu-24.04
          - macos-latest
          - both
        default: both
```

Planner picks per CONTEXT.md Claude's Discretion bullet 4: mirror the `gh_runner_os` shape exactly (recommended for consistency with Phase 45) OR introduce a simpler trigger (no input — both OSes always). Recommendation: copy verbatim.

**Top-level env + permissions pattern** (analog lines 33-38 — minimum-required permissions, deny-by-default):

```yaml
env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -Dwarnings

permissions:
  contents: read
```

**Per-OS conditional job pattern** (analog lines 41-72 — `if:` gate on the input; `continue-on-error: true` so one-OS-green is sufficient; 30-minute timeout):

```yaml
jobs:
  resl-nix:
    if: ${{ inputs.gh_runner_os == 'ubuntu-24.04' || inputs.gh_runner_os == 'both' }}
    name: Phase 45 RESL native (Linux)
    runs-on: ubuntu-24.04
    timeout-minutes: 30
    continue-on-error: true

    steps:
      - name: Checkout
        uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@631a55b12751854ce901bb631d5902ceb48146f7 # stable
        with:
          toolchain: stable

      - name: Cache cargo registry + target
        uses: actions/cache@668228422ae6a00e4ad889ee87cd7109ec5666a7 # v5
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-phase45-resl-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-phase45-resl-

      - name: Build workspace
        run: cargo build --workspace --release --verbose

      - name: Run audit-attestation regression
        run: cargo test -p nono-cli --test audit_attestation -- --include-ignored
```

**Critical conventions extracted from the analog:**
1. **SHA-pinned actions** (3 distinct actions all pinned at commit SHA with comment trailer): `actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6`, `dtolnay/rust-toolchain@631a55b12751854ce901bb631d5902ceb48146f7 # stable`, `actions/cache@668228422ae6a00e4ad889ee87cd7109ec5666a7 # v5`. Phase 37 D-15 / WR-09 enforcement applies — planner MUST reuse these exact pinned SHAs (or newer equivalents from Phase 37/45 if those evolved).
2. **Per-OS cache key** uses `${{ runner.os }}-phase45-resl-...` namespace; for Phase 46 substitute `phase46-uat-backlog`.
3. **`continue-on-error: true` on both per-OS jobs** — D-46-C3 maps cleanly: items that fail because the test fixture is intrinsically unavailable get a `no-test-fixture` waiver in 46-03-SUMMARY rather than blocking the workflow.
4. **`runs-on: ubuntu-24.04`** (Linux job) + **`runs-on: macos-latest`** (macOS job) per D-46-C1.
5. **Two-job structure** — analog has `resl-nix` (Linux) + `resl-darwin` (macOS); Phase 46 should mirror as `uat-backlog-linux` + `uat-backlog-macos` (or planner-picked slugs).

**Planner picks per CONTEXT.md `<decisions>` § Claude's Discretion bullet 4:**
- The actual `cargo test` invocations replace `cargo test -p nono-cli --test audit_attestation -- --include-ignored`. Plan 46-03 inventory step determines which 8/11 UAT + 5/7 verification items can be cargo-invoked vs which waive as `no-test-fixture` (CONTEXT.md `<decisions>` D-46-C3 + Claude's Discretion bullet 5).
- The target-OS-specific test targets per UAT item (some items are Linux-only, some macOS-only, some host-agnostic — categorize per item inventory).

---

### `.planning/phases/35-upst3-closure-quick-wins/35-HUMAN-UAT.md` (NEW; uat-record)

**Analog:** `.planning/phases/37-linux-resl-backends-pkgs-auto-pull/37-HUMAN-UAT.md` (D-46-C4 canonical template; CONTEXT.md `<canonical_refs>` names Phase 37/41/43 HUMAN-UAT.md as the cross-checks).

**Important variant note:** Phase 41 + 43 HUMAN-UAT.md DROP the per-test `why_human:` field that Phase 37 keeps inside the test rows; in Phase 41 + 43 the `why_human:` lives only in the corresponding VERIFICATION.md `human_verification:` frontmatter block. Phase 37 carries `why_human:` only in VERIFICATION.md's frontmatter, not in HUMAN-UAT.md's body. The HUMAN-UAT.md body shape is consistent across all three analogs — the `## Tests` section uses numbered `###` headings with `expected:` + `result:` only.

**Frontmatter pattern** (analog Phase 37 lines 1-7):

```markdown
---
status: partial
phase: 37-linux-resl-backends-pkgs-auto-pull
source: [37-VERIFICATION.md]
started: 2026-05-20T03:48:00Z
updated: 2026-05-20T03:48:00Z
---
```

**For Phase 35 backfill** (post-execution shape — workflow has run, verdicts recorded):

The analog for the *post-execution* terminal state is `.planning/phases/50-corp-network-tuf-refresh-via-os-root-store-replace-or-wrap-t/50-HUMAN-UAT.md` lines 1-10:

```markdown
---
phase: 50
slug: corp-network-tuf-refresh-via-os-root-store-replace-or-wrap-t
created: 2026-05-21
closed: 2026-05-23
status: passed
scenarios: 1
result: 1/1 pass
recording_location: 50-VERIFICATION.md § "Scenario 1 — TLS-inspecting corporate proxy refresh"
---
```

For Phase 35 the substitutions would be (mirroring Phase 37 shape but post-execution):

```markdown
---
status: passed
phase: 35-upst3-closure-quick-wins
source: [35-VERIFICATION.md]
started: 2026-05-23T<HH:MM:SS>Z
updated: 2026-05-23T<HH:MM:SS>Z
backfilled_in: phase-46-plan-46-03
backfill_rationale: "v2.4 close left HUMAN-UAT.md absent (human_needed deferred to v2.6 native host per memory project_v26_opened); Phase 46 Plan 46-03 backfills with verdicts from phase-46-uat-backlog.yml CI runs + no-test-fixture waivers per D-46-C3."
---
```

**Body sections** (analog Phase 37 lines 9-48):

```markdown
## Current Test

[awaiting human testing]    ← post-execution: replace with `[all tests complete]` or the last test name

## Tests

### 1. <short title>
expected: <verbose what-should-happen>
result: [pending]     ← post-execution: `pass` / `fail` / `no-test-fixture (waived per ...)` / `skipped`

### 2. <short title>
expected: ...
result: [pending]

## Summary

total: N
passed: 0
issues: 0
pending: N
skipped: 0
blocked: 0

## Gaps
```

For Phase 35 the planner inventories the 11 UAT items at plan-open (CONTEXT.md `<decisions>` § Claude's Discretion bullet 5 — grep `.planning/milestones/v2.4-MILESTONE-AUDIT.md` rows 70-114 + 273-274 + Phase 35 SUMMARYs at `35-{01,02,03}-SUMMARY.md`). v2.4-MILESTONE-AUDIT rows 116-121 confirm 2 of the Phase 35 items already passed at v2.4 close (env_filter_tests, profile_cli debug-syntax) — those get `result: pass (pre-passed v2.4)` markers. Remaining 9 items resolve via Plan 46-03's `phase-46-uat-backlog.yml` runs or `no-test-fixture` waivers per D-46-C3.

**Summary roll-up shape** post-execution (mirror of Phase 37's `## Summary` block — line 35-42):

```markdown
## Summary

total: 11
passed: <N>
issues: 0
pending: 0
skipped: 0
blocked: 0
no-test-fixture: <M>    ← additional row per D-46-C3 SC#5 explicit allowance
```

Per D-46-C3 the target is "at least 8/11 pass, ≤3 waived". Per-item rationale lives in `46-03-SUMMARY.md`, not in `35-HUMAN-UAT.md`.

---

### `.planning/phases/35-upst3-closure-quick-wins/35-VERIFICATION.md` (NEW; verification-record)

**Analog:** `.planning/phases/37-linux-resl-backends-pkgs-auto-pull/37-VERIFICATION.md` (D-46-C4 canonical template).

**Frontmatter pattern (post-execution terminal state)** — analog Phase 37 lines 1-29 (initial verification, status `human_needed`); for the Phase 35 backfill the planner targets the terminal `status: passed` shape, mirroring `.planning/phases/44.1-oidc-fail-closed-remediation-req-review-fu-01-t-44-01-cr-01/44.1-VERIFICATION.md` lines 1-13:

```markdown
---
phase: 44.1-oidc-fail-closed-remediation-req-review-fu-01-t-44-01-cr-01
verified: 2026-05-20T22:30:00Z
status: passed
score: 8/8 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: none
  previous_score: n/a
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---
```

For Phase 35 backfill, the planner blends both shapes: Phase 37's `re_verification:` block records the transition from `human_needed → passed`; Phase 44.1's simpler shape applies if the planner classifies the backfill as initial-verification rather than re-verification. Suggested shape:

```markdown
---
phase: 35-upst3-closure-quick-wins
verified: 2026-05-23T<HH:MM:SS>Z
status: passed
score: <N>/<11> UAT items verified (M items waived per `no-test-fixture` per D-46-C3)
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: n/a (v2.4 close did not produce 35-VERIFICATION.md)
  previous_verified: n/a
  trigger: "Phase 46 Plan 46-03 backfill per D-46-C4; phase-46-uat-backlog.yml CI runs + no-test-fixture waivers close REQ-UAT-BL-01."
  gaps_closed:
    - "<verbatim item description> → <pass | no-test-fixture (waiver in 46-03-SUMMARY § <heading>)>"
    - ...
  gaps_remaining: []
  regressions: []
backfilled_in: phase-46-plan-46-03
---
```

**Body sections (canonical six-section structure)** — analog Phase 37 lines 31-170 (six major H2 sections per CONTEXT.md `<canonical_refs>` shape):

```markdown
# Phase <N>: <phase name> Verification Report

**Phase Goal:** <verbatim from ROADMAP.md or SPEC.md if present>

**Verified:** <ISO-8601 timestamp>
**Status:** passed
**Re-verification:** Yes (backfilled per Phase 46 Plan 46-03 D-46-C4)

## Goal Achievement

### Observable Truths

| #   | Truth (Success Criterion) | Status | Evidence |
| --- | ------------------------- | ------ | -------- |
| 1   | <criterion>                | VERIFIED | <evidence> |
| 2   | <criterion>                | VERIFIED | <evidence> |
...

**Score:** N/N truths verified

### Deferred Items

<list or "No items deferred to later phases.">

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| ...      | ...      | VERIFIED | ... |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| ...  | ...| ... | WIRED  | ...     |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| ...      | ...           | ...    | Yes                | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| ...      | ...     | ...    | PASS   |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| REQ-PORT-CLOSURE-01 | 35-01 | ... | SATISFIED | ... |
| REQ-PORT-CLOSURE-06 | 35-02 | ... | SATISFIED | ... |
| REQ-PORT-CLOSURE-07 | 35-03 | ... | SATISFIED | ... |

**No orphaned requirements.**

### Anti-Patterns Found

<table or "No CRITICAL findings.">

### Human Verification Required

<post-execution: "All HUMAN-UAT items closed via phase-46-uat-backlog.yml CI runs (run-id <run-id>) + no-test-fixture waivers in 46-03-SUMMARY § <heading>. See 35-HUMAN-UAT.md for per-item verdicts.">

### Gaps Summary

**No goal-blocking gaps.** All <N> Phase 35 success criteria are now satisfied; the v2.4-close `human_needed` deferral closed via Phase 46 Plan 46-03 backfill per D-46-C4.

---

_Verified: <timestamp>_
_Verifier: Claude (gsd-verifier) — Phase 46 backfill_
```

**Planner discretion:** Phase 35's per-plan SUMMARYs (`35-01-WIN-ENV-FILTER-SUMMARY.md`, `35-02-LINUX-LANDLOCK-PROFILES-SUMMARY.md`, `35-03-WIN-TEST-HYGIENE-SUMMARY.md`) ARE NOT touched per D-46-C4. Only HUMAN-UAT.md + VERIFICATION.md get backfilled. The `### Required Artifacts` + `### Behavioral Spot-Checks` tables can either (a) reach back to the existing per-plan SUMMARY evidence and re-tabulate, or (b) reference the SUMMARY artifacts as the evidence source and leave the table minimal. Recommendation: option (b) — backfills should not re-do verification work that happened at v2.4 close; they record the transition out of `human_needed` and reference original evidence.

---

### `.planning/phases/36-upst3-deep-closure/36-HUMAN-UAT.md` (NEW; uat-record)

**Analog + pattern:** identical to `35-HUMAN-UAT.md` above. Same Phase 37 / 50 / 44.1 templates apply. Substitutions:
- Phase identifier: `36-upst3-deep-closure`
- Source per-plan SUMMARYs (CONTEXT.md `<canonical_refs>`): `36-01a..36-03-SUMMARY.md` (6 plan summaries; REQ-PORT-CLOSURE-02 + 04 + 05; deprecated_schema port, yaml_merge wiring, ExecConfig refactor)
- Item count: target 7 verification items (per CONTEXT.md `<decisions>` D-46-C3; v2.4-MILESTONE-AUDIT row 273-274 confirms `Verification gaps | 7`)
- Pre-passed item at v2.4 close (per v2.4-MILESTONE-AUDIT rows 116-121): docs MDX bypass_protection (1 of 7)
- Remaining 6 items resolve via Plan 46-03's `phase-46-uat-backlog.yml` runs or `no-test-fixture` waivers

---

### `.planning/phases/36-upst3-deep-closure/36-VERIFICATION.md` (NEW; verification-record)

**Analog + pattern:** identical to `35-VERIFICATION.md` above. Same Phase 37 / 44.1 templates apply. Substitutions:
- Phase identifier: `36-upst3-deep-closure`
- Requirements coverage: REQ-PORT-CLOSURE-02, REQ-PORT-CLOSURE-04, REQ-PORT-CLOSURE-05 (per CONTEXT.md `<canonical_refs>` "REQ-UAT-BL-01..02 sources" subsection)
- Score: target `<N>/7 verification items verified (M items waived per `no-test-fixture` per D-46-C3)`

---

### `.planning/quick/260428-rsu-refresh-stack-onto-upstream-tip/260428-rsu-SUMMARY.md` (UPDATE; summary-status-flip)

**Analog 1 (current file — what changes):** Read self at lines 1-10. The current frontmatter:

```yaml
---
quick_id: 260428-rsu
slug: refresh-stack-onto-upstream-tip
description: Refresh the stack (PRs 725 + 726) onto upstream's new tip before a human reviewer engages
started: 2026-04-28
resumed: 2026-04-29
status: re-deferred
deferred_until: maintainer-response on PRs 725/726 (see Outreach posted 2026-04-29)
runbook: 260428-rsu-PLAN.md
---
```

**Analog 2 (terminal-state shape):** `.planning/quick/260424-mrg-merge-windows-squash-to-main/SUMMARY.md` lines 1-9 (`status: complete` form):

```yaml
---
slug: mrg-merge-windows-squash-to-main
status: complete
type: git-operations
date: 2026-04-24
path_chosen: C (consolidation now, DCO deferred)
push_policy: stage-locally-do-not-push
executed: steps 1–5 + step 7 (step 6 cargo sanity skipped — fast-forward advances to already-validated commit)
---
```

**Per D-46-A2 the planner applies a minimal-context edit:** flip `status: re-deferred → closed-via-v2.6-rollout` and add a `closed:` field + ADR back-reference field. Suggested resulting frontmatter:

```yaml
---
quick_id: 260428-rsu
slug: refresh-stack-onto-upstream-tip
description: Refresh the stack (PRs 725 + 726) onto upstream's new tip before a human reviewer engages
started: 2026-04-28
resumed: 2026-04-29
closed: 2026-05-23
status: closed-via-v2.6-rollout
closure_disposition: feature-flag-equivalent rollout per ROADMAP § Phase 46 SC#1 (D-46-A1); maintainer-response triggers retained per D-46-A3 — see ADR.
adr: .planning/architecture/v2.6-upstream-merge-deferral-ADR.md
runbook: 260428-rsu-PLAN.md
---
```

**Body amendment pattern (recommended single new section at top of body, before existing "# Quick Task 260428-rsu — Summary: RE-DEFERRED ..." H1):**

```markdown
> **2026-05-23 update (Phase 46 close, D-46-A1/A2/A3/A4):**
> Closed via the SC#1 "feature-flag-equivalent rollout with the gate-state explicitly documented" path. The 504-commit / 77-conflict rebase scope and the maintainer-non-response since 2026-04-29 outreach were not improving; Phase 46 Plan 46-01 lands a new ADR at `.planning/architecture/v2.6-upstream-merge-deferral-ADR.md` capturing alternative paths considered, why feature-flag-equivalent was chosen, the maintainer-response revival trigger set, and the per-phase umbrella PR pattern as the go-forward upstream-contribution mode. PRs 725/726 remain OPEN; the 2026-04-29 outreach remains the canonical comm. Revival on maintainer response only (no fork-side calendar trigger).
>
> The "Re-deferral conditions" section below remains accurate — the new ADR codifies them as the revival trigger set rather than supersedes them.

# Quick Task 260428-rsu — Summary: RE-DEFERRED (awaiting maintainer response)
[... existing body unchanged ...]
```

**Critical edit-target precision:** The planner makes exactly two edits — (1) frontmatter swap shown above, (2) inserted `>` blockquote section between the closing frontmatter `---` and the existing `# Quick Task 260428-rsu — Summary: RE-DEFERRED ...` H1 line at original line 12. The existing body (Timeline / Decisions / Rebase attempt / Outreach / Re-deferral conditions / What did NOT happen today / Files produced / Watch-list) is NOT rewritten — those sections remain accurate per CONTEXT.md `<decisions>` D-46-A2 ("flip status to closed-via-v2.6-rollout with a back-reference to the new ADR" — back-reference, not rewrite).

---

### `.planning/templates/upstream-sync-quick.md:102` (UPDATE; template-baseline-registry)

**Analog:** self (read the file at the line in question — analog and target are the same).

**Current state** (lines 96-106 — the `## Baseline-aware CI gate` section with the load-bearing SHA at line 102):

```markdown
## Baseline-aware CI gate

For upstream-sync waves landing on top of a known-green baseline, the close
gate flags ONLY `success → failure` transitions vs the baseline. Drift
accumulates across milestones; reset at each milestone-internal cleanup.

**Current baseline SHA:** `13cc0628`
**Last reset:** Phase 41 close (REQ-CI-03), 2026-05-16 → cleaning the
pre-existing red carried forward from baseline `a72736bb`.
**Reset cadence:** Every milestone-internal cleanup phase (see Phase 41 for
the v2.5 precedent).
```

**Target edit (per D-46-D3 — minimal-context-safe single-line replacement at line 102 + 2-line update at 103-104):**

```markdown
**Current baseline SHA:** `<PHASE-46-CLOSE-SHA>`
**Last reset:** Phase 46 close (REQ-CI-FU-03), 2026-05-<DD> → post-merge baseline for v2.6 windows-squash-merge + post-merge CI verifications + UAT backlog drain. Previous baseline: `13cc0628` (Phase 41 close, 2026-05-16, v2.5 precedent).
**Reset cadence:** Every milestone-internal cleanup phase (see Phase 41 + Phase 46 for the v2.5 + v2.6 precedents).
```

**Critical:**
- The exact 8-char SHA goes at line 102 between backticks. Planner resolves the actual close SHA at plan-execution time (Plan 46-02's last task per D-46-D3 + per CONTEXT.md § Phase 46 plan + commit map).
- The "Last reset" line (103-104) MUST cite REQ-CI-FU-03 (not REQ-CI-03 — that was the Phase 41 REQ; Phase 46 uses the `-FU-` infix) per REQUIREMENTS.md line 14.
- Phase 47 audit + Phase 48 sync inherit this anchor per CONTEXT.md `<code_context>` § Integration Points: Phase 46 → Phase 47 (UPST6 audit baseline) and Phase 46 → Phase 48 (post-merge baseline).

---

### `.planning/REQUIREMENTS.md` (UPDATE; requirements-checkbox-flip)

**Analog:** self. Lines 12-14, 26-27, 42 each carry a `- [ ]` checkbox-prefixed REQ row. The literal flip pattern is `- [ ] **REQ-...**` → `- [x] **REQ-...**`.

**Per-plan ownership** (CONTEXT.md `<decisions>` § Claude's Discretion bullet 3 — planner default: each plan flips its own REQs at plan-close):

- **Plan 46-01 flips:** Line 42 — `REQ-MERGE-01`
- **Plan 46-02 flips:** Lines 12, 13, 14 — `REQ-CI-FU-01`, `REQ-CI-FU-02`, `REQ-CI-FU-03`
- **Plan 46-03 flips:** Lines 26, 27 — `REQ-UAT-BL-01`, `REQ-UAT-BL-02`

**Current-state excerpts** (verbatim from REQUIREMENTS.md):

```markdown
### CI Follow-up (post-merge orchestrator coordination)

- [ ] **REQ-CI-FU-01**: Phase 37 `.github/workflows/phase-37-linux-resl.yml` live run on `ubuntu-24.04` completes green; Success Criterion 6 closed.
- [ ] **REQ-CI-FU-02**: Phase 43 umbrella PR opened with all 6 PR-SECTION.md contribution artifacts concatenated; orchestrator `gh pr create` executed.
- [ ] **REQ-CI-FU-03**: Baseline-aware CI lane diff vs Phase 41 close SHA `13cc0628` verified — zero `success → failure` transitions.
```

```markdown
### UAT Backlog

- [ ] **REQ-UAT-BL-01**: Phase 35 + 36 human-UAT backlog (11 scenarios) executed on native Linux/macOS host; all items reach `pass` or documented `no-test-fixture` waiver.
- [ ] **REQ-UAT-BL-02**: Phase 35 + 36 verification backlog (7 items) executed on native Linux/macOS host.
```

```markdown
### Branch Merge

- [ ] **REQ-MERGE-01**: `windows-squash` → `main` merge landed with PR-583 maintainer response gate moved OR feature-flag-equivalent rollout documented. Re-deferred at v2.3 (2026-04-29 per quick-260428-rsu, commit `7911ef0e`) + v2.4 + v2.5 scope-locks.
```

**Target edits (literal character flips — `[ ]` → `[x]` only; no other text touched):**

Single-char replacement at 6 locations. Note the existing REQ-PORT-CLOSURE-08 (line 18), REQ-RESL-NIX-04 (line 22), REQ-REVIEW-FU-01 (line 31), REQ-TEST-HYG-01..04 (lines 35-38), REQ-AIPC-G04-01 (line 50) are ALREADY `[x]` — they prove the canonical flipped shape.

---

## Shared Patterns

### Backfill provenance trailer
**Source:** none direct; this is a new convention introduced by D-46-C4
**Apply to:** Both backfilled HUMAN-UAT.md files (35 + 36) + both backfilled VERIFICATION.md files (35 + 36)

When backfilling artifacts that "should have been" produced at the original phase close, add a frontmatter field announcing the backfill:

```yaml
backfilled_in: phase-46-plan-46-03
backfill_rationale: "<one-line why-it-was-deferred-at-original-close + how Phase 46 closed it>"
```

This makes the audit trail explicit — readers landing on `35-HUMAN-UAT.md` without context understand it was produced ~6 months after Phase 35 close, not at original close time.

### `gh` CLI use across Plan 46-02
**Source:** memory `gh_available` + CONTEXT.md `<canonical_refs>` § REQ-CI-FU-01..03
**Apply to:** All `gh workflow run` / `gh pr create` / `gh pr view` / `gh run watch` invocations in Plan 46-02

Three invocation shapes per D-46-B3 (all 4 fired in parallel):

```bash
gh workflow run phase-37-linux-resl.yml
gh workflow run phase-45-resl-native-host.yml -f gh_runner_os=both
gh pr create --base main --head <feat/phase-43-upst5-sync> \
  --title "feat: UPST5 sync execution (Phase 43)" \
  --body "$(cat .planning/phases/43-upst5-sync-execution/43-01b-PR-SECTION.md \
              .planning/phases/43-upst5-sync-execution/43-02-PR-SECTION.md \
              .planning/phases/43-upst5-sync-execution/43-03-PR-SECTION.md \
              .planning/phases/43-upst5-sync-execution/43-04-PR-SECTION.md \
              .planning/phases/43-upst5-sync-execution/43-05-PR-SECTION.md \
              .planning/phases/43-upst5-sync-execution/43-06-PR-SECTION.md)"
gh run watch    # final observation step (4th "action" is actually waiting + CI lane diff observation)
```

PR title pattern mirrors PR 922 (Phase 40 precedent per memory `project_cross_fork_pr_pattern`). Branch name `feat/phase-43-upst5-sync` is suggested by CONTEXT.md Claude's Discretion bullet 7; planner finalizes.

**Recovery pattern after each invocation (per memory `feedback_windows_worktree_cwd`):**

```bash
cd /c/Users/OMack/Nono && pwd && git branch --show-current
# verify cwd is repo root + branch is the expected Phase 46 working branch
```

### DCO sign-off on all commits
**Source:** `CLAUDE.md` § Coding Standards "Commits"
**Apply to:** Every commit produced by Plans 46-01, 46-02, 46-03 (doc-only + YAML-only commits included)

```
Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Per memory `user_identity` the DCO form is `Oscar Mack Jr <oscar.mack.jr@gmail.com>`.

### Cross-target clippy verification
**Source:** `CLAUDE.md` § Coding Standards "Cross-target clippy verification" bullet + `.planning/templates/cross-target-verify-checklist.md`
**Apply to:** N/A across all 3 plans — Phase 46 has no source touches per CONTEXT.md `<domain>` (Plan 46-01 doc-only; Plan 46-02 orchestrator-only no source touches; Plan 46-03 doc + YAML only).

Recorded here explicitly so the planner does NOT inadvertently add a cross-target clippy step to any plan's actions. The `_environmental` carve-out per D-46-D2 inherits the Windows-host cross-target clippy carve-out for the CI lane diff but does NOT require running it from the Phase 46 working host.

### REQUIREMENTS.md checkbox-flip atomic ownership
**Source:** CONTEXT.md `<decisions>` § Claude's Discretion bullet 3 + memory `project_workspace_crates`-style atomic ownership
**Apply to:** All 3 plans

Each plan flips ONLY the REQs it owns (see per-plan ownership table in the REQUIREMENTS.md UPDATE section above). The default is intentional — it preserves per-plan SUMMARY closure semantics ("plan close flips its own REQs"); a consolidated flip at Plan 46-03 close was considered (CONTEXT.md `<deferred>` bullet 4) and rejected in favor of atomic ownership.

---

## No Analog Found

None. Every destination file has a named analog in CONTEXT.md `<canonical_refs>` and the analog files were read in this pass. Two destinations (the 35/36 backfilled HUMAN-UAT.md files) introduce a new `backfilled_in:` convention that has no direct precedent — captured under "Shared Patterns" above so the planner can apply it uniformly.

## Metadata

**Analog search scope:**
- `docs/architecture/*.md` (5 ADRs)
- `.github/workflows/phase-{37,45}-*.yml` (named analogs)
- `.planning/phases/{37,41,43,44.1,50}/{*-HUMAN-UAT,*-VERIFICATION}.md` (named + cross-checks)
- `.planning/quick/{260424-mrg,260428-rsu,260522-rct,260522-di8}/SUMMARY.md` (status-flip terminal-state patterns)
- `.planning/templates/upstream-sync-quick.md` lines 80-129 (canonical baseline registry context window)
- `.planning/REQUIREMENTS.md` lines 1-60 (full REQ table for cross-checking flipped vs unflipped checkbox shape)

**Files scanned:** 11 reads (no re-reads); 4 globs; 2 greps.

**Pattern extraction date:** 2026-05-23.
