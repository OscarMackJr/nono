---
phase: 71
slug: engine-agnostic-launch-productionization
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-13
---

# Phase 71 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p nono-cli --lib` |
| **Full suite command** | `make test` (or `cargo test --workspace`) |
| **Estimated runtime** | ~60–120 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli --lib`
- **After every plan wave:** Run `make test`
- **Before `/gsd:verify-work`:** Full suite must be green + the SC1 real-Win11 Aider HUMAN-UAT gate executed
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

> Populated by the planner from RESEARCH.md "## Validation Architecture". SC1 is the real-Win11 Aider end-to-end HUMAN-UAT gate (manual-only). SC2–SC5 have automated coverage.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 71-XX-XX | XX | X | ENG-0X | T-71-XX / — | {expected secure behavior} | unit | `cargo test -p nono-cli ...` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Coverage-gate interpreter-extension unit tests (ENG-02 / SC3) — covered vs uncovered interpreter path, named-binary diagnostic assertion
- [ ] R-B3 ownership pre-launch gate unit tests (ENG-02 / SC4) — user-owned vs admin-owned workspace, named ownership diagnostic
- [ ] Engine-profile resolution tests (ENG-03 / SC2) — `aider` + `langchain-python` profiles resolve broker flag + allow-groups + interpreter coverage
- [ ] SC5 foreign-job-collision negative test (P6) — assign-failure → fail-secure terminate, named diagnostic

*Real-Win11 broker-arm behavior (SC1) cannot be unit-tested — see Manual-Only.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Aider confined end-to-end on real Win11 (write inside workspace lands; write outside denied `NO_WRITE_UP`; in-process + subprocess ops confined transitively) | ENG-01 / SC1 | Broker arm requires a real host + real Aider install; CI/unit cannot exercise the Low-IL relabel + AppContainer path | See `71-HUMAN-UAT.md` (authored by planner) — run from dev-layout/signed `nono.exe` (R-B4 trust gate), profile-covered workspace |

*All other phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
