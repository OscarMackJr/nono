---
phase: 96
slug: cross-target-toolchain
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-26
---

# Phase 96 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner + clippy gates (no new framework) |
| **Config file** | none — `Cross.toml` + `Cargo.toml` already present |
| **Quick run command** | `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` (native, fast feedback) |
| **Full suite command** | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` |
| **Estimated runtime** | ~native: 30–90s · cross: several minutes (first run pulls/builds image) |

---

## Sampling Rate

- **After every task commit:** Run native `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` (fast; catches obvious regressions before paying cross runtime).
- **After every plan wave:** Run `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` (the real gate).
- **Before `/gsd:verify-work`:** The cross linux-gnu gate must exit 0; the apple-darwin disposition (pass OR hard-blocker record) must be written.
- **Max feedback latency:** native ~90s; cross gate per-wave.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 96-01-* | 01 | 1 | XTGT-01 | — | Docker Linux engine up; `cross clippy` reaches the container with a real `x86_64-linux-gnu-gcc` | integration | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` (reaches compile) | ✅ | ⬜ pending |
| 96-01-* | 01 | 1 | XTGT-02 | — | linux-gnu clippy gate exits 0; cfg-gated Unix drift fixed structurally (no `#[allow]`) | integration | same as above, exit 0 | ✅ | ⬜ pending |
| 96-02-* | 02 | 2 | XTGT-03 | — | apple-darwin: bounded cargo-zigbuild attempt → clean clippy OR documented SDK-licensing hard-blocker | integration | `cargo zigbuild clippy --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (one bounded run) | ✅ | ⬜ pending |
| 96-03-* | 03 | 3 | XTGT-04 | — | checklist + CLAUDE.md retire PARTIAL→CI *default* per-gate, evidence-based | source | `grep -c "cross clippy" .planning/templates/cross-target-verify-checklist.md` ≥ 1 | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- Existing infrastructure covers all phase requirements (Rust toolchain, clippy, cross, Docker already installed). No test framework install needed.
- The apple-darwin path requires installing `zig` + `cargo-zigbuild` — this is an in-scope install step inside Plan 02, not a separate Wave 0 test-infra dependency.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Docker Desktop Linux engine is running before the cross gate | XTGT-01/02 | Daemon state is host-environment, not assertable in CI from this host | Confirm `npipe:////./pipe/dockerDesktopLinuxEngine` reachable / `docker info` exits 0 before running the gate |
| apple-darwin SDK-licensing hard-blocker disposition | XTGT-03 | The D-04 stop condition is a human licensing judgement (do NOT extract proprietary SDK) | Reviewer confirms the hard-blocker record cites the SDK/linker failure signature and commits apple-darwin to PARTIAL→CI |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s (native quick run)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
