---
phase: 33
slug: windows-parity-upstream-0-52-divergence
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-10
---

# Phase 33 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `make` + `grep` + `bash` (structural checks; no Rust code changes in this phase) |
| **Config file** | `Makefile` (existing `check-upstream-drift` and `ci` targets) |
| **Quick run command** | `grep -c "^\| [a-f0-9]\{7,\}" .planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md` (row count); `grep -E "^- Disposition: (will-sync\|fork-preserve\|won't-sync)$"` (disposition enum) |
| **Full suite command** | `make ci` (clippy + fmt + tests — must remain green; docs-only phase) |
| **Estimated runtime** | ~5s structural checks; ~3–5 min `make ci` |

---

## Sampling Rate

- **After every task commit:** Run the relevant grep/structural check for the artifact just modified
- **After every plan wave:** Run `make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json"` (Wave 1) or `make ci` (final wave)
- **Before `/gsd-verify-work`:** `make ci` green + all REQ-1..REQ-5 acceptance greps return non-empty
- **Max feedback latency:** ~5 minutes (dominated by `make ci`)

---

## Per-Task Verification Map

*Populated by gsd-planner after wave/task IDs are assigned. Filled per the validators below.*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 33-XX-YY | XX  | W    | REQ-{1..5} | —          | N/A (docs-only) | structural / build | see validator block per REQ below | ✅ / ❌ W0 | ⬜ pending |

### Validator blocks (planner copies into task `<automated>` fields)

**REQ-1 — Drift audit + DIVERGENCE-LEDGER.md:**
```bash
# Wave 0 prep: ensure tags are local
git fetch upstream --tags

# Acceptance:
make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json" && echo "DRIFT_OK=$?"
test -f .planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md
# Every cluster has a disposition from the 3-value enum:
grep -E "^- Disposition: (will-sync|fork-preserve|won't-sync)$" .planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md | wc -l
# Header captures invocation (D-33-A1) + upstream HEAD sha + script version (D-33-A2):
grep -E "(check-upstream-drift|--from v0.40.1|--to v0.52.0)" .planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md
# Fork-only surface area section present (D-33-A3):
grep -E "^## Fork-only surface area" .planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md
```

**REQ-2 — Strategic decision ADR:**
```bash
test -f docs/architecture/upstream-parity-strategy.md
grep -E "^\*\*Status:\*\* Accepted$" docs/architecture/upstream-parity-strategy.md
# All three options scored (D-33-C1, C2):
grep -E "(continue|split-windows|freeze-at-v0.52)" docs/architecture/upstream-parity-strategy.md
# 5 criteria, Low/Med/High scoring:
grep -cE "(Maintenance cost|Security posture|User clarity|Contributor velocity|Roadmap optionality)" docs/architecture/upstream-parity-strategy.md  # expect ≥5
# Decision + Consequences + Alternatives sections:
grep -E "^## (Decision|Consequences|Alternatives)" docs/architecture/upstream-parity-strategy.md
```

**REQ-3 — PROJECT.md key-decisions row:**
```bash
# Row references the ADR (D-33-C1 / REQ-3 / RESEARCH Open Question 1):
grep -E "upstream-parity-strategy" .planning/PROJECT.md
# Row is in the 3-column key-decisions table (planner resolves table-vs-bullets ambiguity from research):
grep -B1 -A1 "upstream-parity-strategy" .planning/PROJECT.md
```

**REQ-4 — G-25-DRIFT-01 cross-reference:**
```bash
# Update section appended (D-33-D2 / NOT a frontmatter-only edit):
grep -E "^\*\*Update \(Phase 33, 2026-05-10\):\*\*" .planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md
# References both artifacts:
grep -E "(DIVERGENCE-LEDGER\.md|upstream-parity-strategy\.md)" .planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md
# Gap stays open (Phase 33 does NOT close it):
grep -E "status: open" .planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md
```

**REQ-5 — UPST3-sync placeholder in ROADMAP:**
```bash
# Placeholder phase queued (D-33-D1 — slot decided at plan-phase):
grep -E "UPST3.*Upstream.*Sync Execution" .planning/ROADMAP.md
# Depends on Phase 33:
grep -B5 "UPST3.*Sync Execution" .planning/ROADMAP.md | grep "Depends on.*Phase 33"
# Plans: 0 plans boilerplate (matches existing TBD-stub shape):
grep -A5 "UPST3.*Sync Execution" .planning/ROADMAP.md | grep -E "Plans: 0 plans"
```

**Cross-cutting — make ci green (no library mutations):**
```bash
make ci
# crates/nono/ byte-identical (D-19 invariant):
git diff --name-only main -- crates/nono/ | wc -l   # expect 0
```

---

## Wave 0 Requirements

- [ ] `git fetch upstream --tags` — fetch v0.43.1..v0.52.0 tags (NOT currently local per RESEARCH §8; required by REQ-1 invocation)
- [ ] `make check-upstream-drift --help` succeeds — drift-tool sanity check (Phase 24 D-04 artifact)
- [ ] Confirm `upstream` remote points at `https://github.com/always-further/nono.git` — `git remote -v | grep upstream`
- [ ] Resolve Open Question 1 from RESEARCH.md (PROJECT.md key-decisions target: 3-col table at L158–183 vs Requirements bullets) — locks the REQ-3 row shape before the planner writes the task action

*Existing infrastructure (drift-tool, ADR convention, Makefile `ci` target) covers all OTHER phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Per-cluster disposition decisions (`will-sync` vs `fork-preserve` vs `won't-sync`) | REQ-1 | SPEC Out-of-scope explicitly defers per-row dispositions to maintainer judgment during the audit walk | For each cluster: read the commit subjects, judge whether the change is upstream-relevant for the fork's Windows surface, assign disposition + 1-line rationale grounded in D-19/D-20 precedent |
| ADR option selection (A/B/C) + tiebreaker rationale | REQ-2 | SPEC requires a scored matrix; the picked option is the maintainer's strategic judgment grounded in audit findings | Score each option×criterion in Low/Med/High using ledger row counts + fork-only surface size as evidence; document picked option in Decision section; if tied, name tiebreaker explicit per D-33-C3 (defaults to security-posture lean per PROJECT.md core value) |
| ADR `Consequences > Future audit cadence` subsection content | REQ-2 (Specifics §) | The audit cadence (every-release / every-milestone / on-trigger) is a downstream-of-decision rule the maintainer sets based on chosen option | Add a 2–3 sentence subsection explaining when `make check-upstream-drift` runs in the chosen world-state |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify (structural greps, make targets) or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers the `git fetch upstream --tags` MISSING reference + Open Question 1 lock
- [ ] No watch-mode flags
- [ ] Feedback latency < ~5 min (`make ci` dominates)
- [ ] `nyquist_compliant: true` set in frontmatter after planner fills the Per-Task Verification Map

**Approval:** pending
