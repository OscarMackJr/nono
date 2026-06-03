---
phase: 61
slug: ship-release-v2-9-package-and-release-the-phase-60-confined
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-03
---

# Phase 61 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: 61-RESEARCH.md § Validation Architecture. This is a release/packaging phase —
> most "validation" is verification + CI gates, not new test infra. Wave 0 gaps: NONE
> (D-09 tests exist since Phase 60-03; release.yml signing gates exist since Phase 53).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust workspace) + PowerShell/CI for release |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p nono-cli --lib claude_code_hook` |
| **Full suite command** | `cargo build --workspace && cargo test -p nono-cli` |
| **Estimated runtime** | ~2–4 min (workspace build) |

---

## Sampling Rate

- **After the version-bump commit:** `cargo build --workspace` (regenerates `Cargo.lock`, validates the 0.58.0 path-dep pins) + `cargo test -p nono-cli --lib claude_code_hook`.
- **Phase gate (BEFORE the tag push):** D-09 hook tests green AND CI signing secrets confirmed present.
- **Post-tag:** watch the `release` job; confirm the `Verify MSI payload signatures` step passed.
- **Max feedback latency:** ~4 min (local build); CI release job is the terminal gate.

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | Status |
|-----|----------|-----------|-------------------|--------|
| REQ-RLS-04 (D-09) | Hook refuses Bash wrap when CWD covers `~/.claude` / project `.claude/` | unit | `cargo test -p nono-cli --lib claude_code_hook` (`cwd_covers_home_claude_state`, `.claudefoo` non-match, project-`.claude` deny) | ✅ exists on main (60-03) |
| REQ-RLS-04 (D-09) | `nono why` shows `~/.claude` write refused even with `--allow-cwd` aimed at it | manual/UAT | operator: confined loop from `%USERPROFILE%\.claude`, attempt Bash write to `settings.json`, observe deny | ⬜ manual |
| REQ-RLS-03 (D-04) | Workspace resolves at 0.58.0, all path-deps consistent | smoke | `cargo build --workspace` | ⬜ pending |
| REQ-RLS-03 (D-06) | release.yml produces Authenticode-valid MSI wrapper AND payloads | CI gate | release.yml `Verify Authenticode signatures` (:236) + `Verify MSI payload signatures` (:258) | ✅ in CI |
| REQ-RLS-03 (D-06) | Post-release MSI signature spot-check | manual | operator: `Get-AuthenticodeSignature` on downloaded machine+user MSI + admin-extract payload check | ⬜ manual |
| REQ-RLS-03 (D-08) | The 4 v2.7 drain-fix commits are ancestors of the tag | smoke | `git merge-base --is-ancestor <sha> HEAD` × 4 | ⬜ pending |
| REQ-RLS-03 (D-07) | Superseded v0.57.4 GitHub release absent (or deleted) | smoke | `gh release view v0.57.4` → not found | ✅ confirmed absent |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* D-09 unit tests exist (Phase 60-03); release.yml signing/verify gates exist (Phase 53). This phase adds verification + the version bump + tagging, not new test infra.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Confined-loop `~/.claude` write deny | REQ-RLS-04 | Requires the live hooked Claude Code loop on a Win11 host | Launch confined loop from `%USERPROFILE%\.claude`; have the agent attempt a Bash write to `settings.json`; confirm deny |
| Post-release MSI signature | REQ-RLS-03 | Requires the published GitHub-release artifacts | `Get-AuthenticodeSignature` on downloaded machine+user MSI; admin-extract payload binaries + verify they're signed |

---

## Validation Sign-Off

- [ ] Version-bump build green (`cargo build --workspace`)
- [ ] D-09 hook tests green before tag push
- [ ] CI signing secrets confirmed present (release.yml :124 fail-closed)
- [ ] release.yml `Verify MSI payload signatures` passed on the v0.58.0 tag
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
