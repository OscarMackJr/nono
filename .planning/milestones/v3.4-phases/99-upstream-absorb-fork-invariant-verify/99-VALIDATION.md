---
phase: 99
slug: upstream-absorb-fork-invariant-verify
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-30
validated: 2026-06-30
---

# Phase 99 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `99-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | None — `Cargo.toml` workspace; orchestrated via `make test` / `make ci` |
| **Quick run command** | `cargo test -p nono-proxy --lib` (Phase 89 proxy guard tests) |
| **Full suite command** | `make ci` (clippy + fmt + tests) |
| **Estimated runtime** | ~120–300 seconds (full `make ci`); quick proxy test ~5s |

---

## Sampling Rate

- **After every task commit:** `cargo test -p nono-proxy --lib` after each Cluster A or Cluster C commit; `cargo test -p nono --lib sandbox::linux` after the Cluster D commit
- **After every plan wave:** `make ci` (clippy + fmt + full test suite)
- **Phase gate (before `/gsd:verify-work`):** BOTH cross-target clippy gates GREEN (`cross clippy` x86_64-unknown-linux-gnu + `cargo-zigbuild clippy` x86_64-apple-darwin, `-D warnings -D clippy::unwrap_used`, **no PARTIAL→CI**) + `make ci` clean + code-review/verifier pass
- **Max feedback latency:** ~300 seconds (full `make ci`)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 99-02/03 | A | — | UPST11-02 | T-99-01/06/15 | NetworkIntent adopted; `denied_endpoint_returns_403_and_audit` still passes | unit | `cargo test -p nono-proxy --lib` | ✅ | ✅ green |
| 99-02 | A | W0 | UPST11-02 | T-99-03 | `WSL2ProxyFallback` preserved post-#1225 | unit | `cargo test -p nono-cli --bin nono deviation_preserved` → `profile/mod.rs:8621 test_wsl2_proxy_policy_deviation_preserved` | ✅ | ✅ green |
| 99-02 | A | W0 | UPST11-02 | T-99-01 | `CompiledEndpointPolicy` compat preserved | unit | `cargo test -p nono-cli --bin nono deviation_preserved` → `proxy_runtime.rs:681 test_compiled_endpoint_policy_compat_deviation_preserved` | ✅ | ✅ green |
| 99-04 | D | — | UPST11-02 | T-99-07/08 | linux.rs seccomp/cgroup + AF_UNIX no-grant EPERM survive 9P addition | unit | `cargo test -p nono --lib sandbox::linux` (`linux.rs:3955`, `:4546`) | ✅ | ✅ green |
| 99-05 | F | — | UPST11-03 | T-99-11/12 | Workspace builds after sigstore-trust-root bump + Cargo.lock regen | build | `make build` | ✅ | ✅ green (attested) |
| 99-07 | verify | — | UPST11-04 | T-99-17 | linux-gnu cross-target clippy GREEN | build | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | ✅ | ✅ green (manual, human-approved) |
| 99-07 | verify | — | UPST11-04 | T-99-17 | apple-darwin cross-target clippy GREEN | build | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✅ | ✅ green (manual, human-approved) |
| 99-07 | verify | W0 | UPST11-04 | T-99-18/19 | Fork-invariant carve-out checklist (none regressed) | manual | D-10 checklist in `99-07-SUMMARY.md` — 3 "Verified unregressed" verdicts | ✅ | ✅ green (manual, human-approved) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*Validated 2026-06-30: all 8 contract rows green. D-08 deviation tests (rows 2-3) and D-10 checklist (row 8) — the three Wave-0 deliverables marked `❌ W0` at plan time — now exist and pass (live-confirmed). Cross-target clippy + D-10 (rows 6-8) are inherently manual (Docker/zig host-gated + judgment); executed and human-approved per 99-07-SUMMARY / 99-VERIFICATION.md.*

---

## Wave 0 Requirements

- [x] D-08 test for `WSL2ProxyFallback` deviation — `profile/mod.rs:8621 test_wsl2_proxy_policy_deviation_preserved` (live-passes)
- [x] D-08 test for `CompiledEndpointPolicy` compat — `proxy_runtime.rs:681 test_compiled_endpoint_policy_compat_deviation_preserved` (live-passes)
- [x] D-10 fork-invariant carve-out checklist — `99-07-SUMMARY.md` with 3 "Verified unregressed" verdict lines

*Existing infrastructure (`cargo test`, the Phase 89 proxy guards, the linux.rs seccomp/cgroup tests, both cross-target clippy gates, `make ci`) covers all other phase requirements. All Wave-0 deliverables complete.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Both cross-target clippy gates GREEN | UPST11-04 | Requires Docker (`cross`) + zig (`cargo-zigbuild`) toolchains; not part of `cargo test` | Run both gate commands from `.planning/templates/cross-target-verify-checklist.md`; both must exit 0 |
| Fork-invariant carve-out checklist | UPST11-04 | Judgment-based confirmation that AppContainer/WFP/broker, ADR-86 boundary, and `exec_strategy_windows/` carve-out are unregressed | Complete D-10 checklist with a verdict line per invariant; pair with code-review + verifier pass |
| `always-further → nolabs-ai` org-ref applied; `OscarMackJr/nono` fork identity preserved | UPST11-03 | String-level review of which references rewrite vs which stay | Grep for both org strings post-Cluster-E; confirm only upstream refs migrated |

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (D-08 deviation tests, D-10 checklist) — all filled
- [x] No watch-mode flags
- [x] Feedback latency < 300s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-06-30

---

## Validation Audit 2026-06-30

| Metric | Count |
|--------|-------|
| Requirements audited | 8 contract rows (UPST11-02/03/04) |
| Gaps found | 0 (3 Wave-0 items deferred at plan time now filled) |
| Resolved (automated, live-green) | 5 (nono-proxy 192/192; D-08 ×2; linux.rs seccomp; make build) |
| Manual-only (executed + human-approved) | 3 (linux-gnu + apple-darwin cross clippy; D-10 checklist) |
| Escalated | 0 |

**Method:** State-A audit. No auditor spawn required — no MISSING/PARTIAL automatable gaps. Live-confirmed: `cargo test -p nono-cli --bin nono deviation_preserved` (2/2 ok) and `cargo test -p nono-proxy --lib` (192/192 ok). All tests pre-existed from execution; no new test files generated or committed. The 3 manual-only rows are inherently non-`cargo test`-able (Docker `cross` + zig `cargo-zigbuild` host-gated; D-10 is judgment-based) and were executed + human-approved per 99-07-SUMMARY.md / 99-VERIFICATION.md.
