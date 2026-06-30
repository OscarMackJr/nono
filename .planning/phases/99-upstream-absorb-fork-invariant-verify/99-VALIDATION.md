---
phase: 99
slug: upstream-absorb-fork-invariant-verify
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-30
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
| (planner) | A | — | UPST11-02 | — | NetworkIntent adopted; `denied_endpoint_returns_403_and_audit` still passes | unit | `cargo test -p nono-proxy --lib` | ✅ | ⬜ pending |
| (planner) | A | — | UPST11-02 | — | `WSL2ProxyFallback` preserved post-#1225 | unit | New test (D-08), `profile/mod.rs` or `proxy_runtime.rs` test module | ❌ W0 | ⬜ pending |
| (planner) | A | — | UPST11-02 | — | `CompiledEndpointPolicy` compat preserved | unit | New test (D-08) or coverage from `denied_endpoint_returns_403_and_audit` | ❌ W0 | ⬜ pending |
| (planner) | D | — | UPST11-02 | — | linux.rs seccomp/cgroup + AF_UNIX no-grant EPERM survive 9P addition | unit | `cargo test -p nono --lib sandbox::linux` | ✅ | ⬜ pending |
| (planner) | F | — | UPST11-03 | — | Workspace builds after sigstore-trust-root bump + Cargo.lock regen | build | `make build` | ✅ | ⬜ pending |
| (planner) | verify | — | UPST11-04 | — | linux-gnu cross-target clippy GREEN | build | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | ✅ | ⬜ pending |
| (planner) | verify | — | UPST11-04 | — | apple-darwin cross-target clippy GREEN | build | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✅ | ⬜ pending |
| (planner) | verify | — | UPST11-04 | — | Fork-invariant carve-out checklist (none regressed) | manual | Code review + D-10 checklist completion | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*Note: actual `{N}-{plan}-{task}` IDs and final wave assignment are set by the planner; this map is the validation contract those tasks must satisfy.*

---

## Wave 0 Requirements

- [ ] D-08 new test(s) for `WSL2ProxyFallback` deviation (profile/mod.rs or proxy_runtime.rs test module)
- [ ] D-08 new test(s) for `CompiledEndpointPolicy` compat — or confirm coverage from `denied_endpoint_returns_403_and_audit`
- [ ] D-10 fork-invariant carve-out checklist document (verdict lines, not just template)

*Existing infrastructure (`cargo test`, the Phase 89 proxy guards, the linux.rs seccomp/cgroup tests, both cross-target clippy gates, `make ci`) covers all other phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Both cross-target clippy gates GREEN | UPST11-04 | Requires Docker (`cross`) + zig (`cargo-zigbuild`) toolchains; not part of `cargo test` | Run both gate commands from `.planning/templates/cross-target-verify-checklist.md`; both must exit 0 |
| Fork-invariant carve-out checklist | UPST11-04 | Judgment-based confirmation that AppContainer/WFP/broker, ADR-86 boundary, and `exec_strategy_windows/` carve-out are unregressed | Complete D-10 checklist with a verdict line per invariant; pair with code-review + verifier pass |
| `always-further → nolabs-ai` org-ref applied; `OscarMackJr/nono` fork identity preserved | UPST11-03 | String-level review of which references rewrite vs which stay | Grep for both org strings post-Cluster-E; confirm only upstream refs migrated |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (D-08 deviation tests, D-10 checklist)
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
