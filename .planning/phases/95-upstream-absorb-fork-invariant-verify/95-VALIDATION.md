---
phase: 95
slug: upstream-absorb-fork-invariant-verify
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-26
---

# Phase 95 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) via `make` targets |
| **Config file** | `Makefile` (targets), `Cargo.toml` (workspace) — no separate test config |
| **Quick run command** | `cargo test -p nono --lib` (or the touched crate) |
| **Full suite command** | `make test` (workspace) |
| **Estimated runtime** | ~120–300 seconds (full workspace build + test) |

---

## Sampling Rate

- **After every task commit:** Run the touched crate's tests (`cargo test -p <crate>`)
- **After every plan wave:** Run `make build` + `make test` (Windows host)
- **Before `/gsd:verify-work`:** `make ci` (clippy + fmt + tests) green; no-new-failures vs the captured baseline (D-04)
- **Max feedback latency:** ~300 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| (planner fills) | — | — | UPST10-02 / UPST10-03 | — | Fork Windows security model unregressed post-sync | unit / integration | `make test` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Capture the Windows baseline-red test set at the phase-base commit BEFORE any cherry-pick (D-04). Known baseline (verified live at `0e3d1a68`): 5 reds — `try_set_mandatory_label` (nono lib) + `test_init_allowed_when_pack_has_same_short_name` + 3 `protected_paths` tests (nono-cli). Record so "no new failures" is provable.

*Existing Rust test infrastructure covers all phase requirements — no framework install needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Cross-target clippy (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`) | UPST10-03 | Toolchain stand-up deferred to Phase 96 (D-03); recorded PARTIAL→96 per cross-target-verify-checklist | Deferred — Phase 95 records the PARTIAL→96 note per cfg-gated commit |
| Fork-invariant: `exec_strategy_windows/` denial-rendering untouched | UPST10-03 | git-diff inspection gate | `git diff <base>..HEAD -- crates/nono-cli/src/exec_strategy_windows/` shows no unintended change |

*Most phase behaviors have automated verification (test names + grep gates pinned in RESEARCH.md).*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (baseline-red capture)
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
