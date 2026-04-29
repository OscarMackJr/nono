# Roadmap: nono Windows Parity & Quality

This roadmap tracks the path to full Windows/Unix parity and ongoing quality-of-life work for `nono`.

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 1–4 (shipped 2026-03-31; tag `v1.0`)
- ✅ **v2.0 Windows Gap Closure** — Phases 5–15 (shipped 2026-04-18; tag `v2.0`)
- ✅ **v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup** — Phases 16–21 + 18.1 (shipped 2026-04-21; tag `v2.1`)
- ✅ **v2.2 Windows/macOS Parity Sweep** — Phases 22–24 (shipped 2026-04-29; tag `v2.2`)

## Phases

<details>
<summary>✅ v1.0 Windows Alpha (Phases 1–4) — SHIPPED 2026-03-31</summary>

- [x] Phase 1: Windows Control Foundation (3/3 plans) — completed 2026-04-04
- [x] Phase 2: Persistent Sessions (4/4 plans) — completed 2026-04-04
- [x] Phase 3: Network Sandboxing (4/4 plans) — completed 2026-04-04
- [x] Phase 4: State Integrity & Deployment (3/3 plans) — completed 2026-04-05

See `.planning/milestones/v1.0-*` if archived separately; the `v1.0` git tag points at the formal shipped state.

</details>

<details>
<summary>✅ v2.0 Windows Gap Closure (Phases 5–15) — SHIPPED 2026-04-18</summary>

- [x] Phase 5: Windows Detach Readiness Fix (1/1 plan) — completed 2026-04-05
- [x] Phase 6: WFP Enforcement Activation (2/2 plans) — completed 2026-04-06
- [x] Phase 7: Quick Wins (2/2 plans) — completed 2026-04-08
- [x] Phase 8: ConPTY Shell (1/1 plan, UAT-driven) — completed 2026-04-10
- [x] Phase 9: WFP Port-Level + Proxy Filtering (4/4 plans) — completed 2026-04-10
- [x] Phase 10: ETW-Based Learn Command (3/3 plans) — completed 2026-04-10
- [x] Phase 11: Runtime Capability Expansion — stretch (2/2 plans) — completed 2026-04-11
- [x] Phase 12: Milestone Bookkeeping Cleanup (3/3 plans) — completed 2026-04-11
- [x] Phase 13: v2.0 Human Verification UAT (1/1 plan) — resolved 2026-04-18
- [x] Phase 14: v2.0 Fix Pass (2/3 plans, 1 escalated to Phase 15) — complete-with-carry-forward 2026-04-18
- [x] Phase 15: Detached Console + ConPTY Architecture Investigation (3/3 plans) — completed 2026-04-18

Full details: `.planning/milestones/v2.0-ROADMAP.md`.

</details>

<details>
<summary>✅ v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup (Phases 16–21 + 18.1) — SHIPPED 2026-04-21</summary>

- [x] Phase 16: Resource Limits — RESL-01..04 (2/2 plans) — completed 2026-04-18
- [x] Phase 17: Attach-Streaming — ATCH-01 (2/2 plans) — completed 2026-04-19
- [x] Phase 18: Extended IPC — AIPC-01 (4/4 plans) — completed 2026-04-19
- [x] Phase 18.1: Extended IPC Gap Closure (4/4 plans) — completed 2026-04-21
- [x] Phase 19: Cleanup — CLEAN-01..04 (4/4 plans) — completed 2026-04-19
- [x] Phase 20: Upstream Parity Sync — UPST-01..04 (4/4 plans) — completed 2026-04-19
- [x] Phase 21: Windows Single-File Filesystem Grants — WSFG-01..03 (5/5 plans) — completed-with-issues 2026-04-20 (supervisor-pipe regression surfaced + resolved 2026-04-20; Phase 18.1 closed the 5 AIPC UAT gaps)

Full details: `.planning/milestones/v2.1-ROADMAP.md`.

</details>

<details>
<summary>✅ v2.2 Windows/macOS Parity Sweep (Phases 22–24) — SHIPPED 2026-04-29</summary>

- [x] Phase 22: UPST2 — Upstream v0.38–v0.40 Parity Sync (6/6 plans, PROF + POLY + PKG + OAUTH + AUD-01..04) — completed 2026-04-28
- [x] Phase 23: Windows Audit-Event Retrofit (1/1 plan, AUD-05) — completed 2026-04-29
- [x] Phase 24: Parity-Drift Prevention (2/2 plans, DRIFT-01 + DRIFT-02) — completed 2026-04-27

Full details: `.planning/milestones/v2.2-ROADMAP.md`.

</details>

## Progress Table

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Windows Control Foundation | v1.0 | 3/3 | Complete | 2026-04-04 |
| 2. Persistent Sessions | v1.0 | 4/4 | Complete | 2026-04-04 |
| 3. Network Sandboxing | v1.0 | 4/4 | Complete | 2026-04-04 |
| 4. State Integrity & Deployment | v1.0 | 3/3 | Complete | 2026-04-05 |
| 5. Windows Detach Readiness Fix | v2.0 | 1/1 | Complete | 2026-04-05 |
| 6. WFP Enforcement Activation | v2.0 | 2/2 | Complete | 2026-04-06 |
| 7. Quick Wins | v2.0 | 2/2 | Complete | 2026-04-08 |
| 8. ConPTY Shell | v2.0 | 1/1 | Complete | 2026-04-10 |
| 9. WFP Port-Level + Proxy Filtering | v2.0 | 4/4 | Complete | 2026-04-10 |
| 10. ETW-Based Learn Command | v2.0 | 3/3 | Complete | 2026-04-10 |
| 11. Runtime Capability Expansion | v2.0 | 2/2 | Complete | 2026-04-11 |
| 12. Milestone Bookkeeping Cleanup | v2.0 | 3/3 | Complete | 2026-04-11 |
| 13. v2.0 Human Verification UAT | v2.0 | 1/1 | Resolved | 2026-04-18 |
| 14. v2.0 Fix Pass | v2.0 | 2/3 | Complete-with-carry-forward | 2026-04-18 |
| 15. Detached Console + ConPTY Investigation | v2.0 | 3/3 | Complete | 2026-04-18 |
| 16. Resource Limits (RESL) | v2.1 | 2/2 | Complete | 2026-04-18 |
| 17. Attach-Streaming (ATCH) | v2.1 | 2/2 | Complete | 2026-04-19 |
| 18. Extended IPC (AIPC) | v2.1 | 4/4 | Complete | 2026-04-19 |
| 18.1. Extended IPC Gap Closure | v2.1 | 4/4 | Complete | 2026-04-21 |
| 19. Cleanup (CLEAN) | v2.1 | 4/4 | Complete | 2026-04-19 |
| 20. Upstream Parity Sync (UPST) | v2.1 | 4/4 | Complete | 2026-04-19 |
| 21. Windows Single-File Grants (WSFG) | v2.1 | 5/5 | Complete-with-issues | 2026-04-20 |
| 22. UPST2 — Upstream v0.38–v0.40 Parity Sync | v2.2 | 6/6 | Complete (SECURED + REVIEW-FIX 7/7 + UAT 10/10 + 1 spec-error skipped; 22-03 partial close — 6/8 cherry-picks, 2 deferred to v2.3; Authenticode chain-walker deferred to v2.3) | 2026-04-28 |
| 23. Windows Audit-Event Retrofit | v2.2 | 1/1 | Complete | 2026-04-29 |
| 24. Parity-Drift Prevention | v2.2 | 2/2 | Complete | 2026-04-27 |

## Backlog (v2.3 carry-forward)

- **PKG-streaming follow-up** (deferred from Plan 22-03, 2026-04-28). Land the two PKG cherry-picks held back from 22-03's chain: upstream `58b5a24e` (`validate_relative_path` input-string check) and `9ebad89a` (streaming `bytes`→`PathBuf` refactor + `tempfile::TempDir` + size limits + HTTP timeouts + `semver` dep). Prerequisites the follow-up plan must own:
  1. Introduce `ArtifactType::Plugin` enum variant + plumbing (closes the deferred-divergence comment at `crates/nono-cli/src/package_cmd.rs:631-643`).
  2. Decide explicitly whether the fork keeps `validate_path_within` (canonicalize-and-component-compare) as defense-in-depth alongside upstream's `validate_relative_path` input-string pre-check, or adopts upstream's pattern verbatim. **Recommendation: keep both** — fork's stance is stricter and matches CLAUDE.md § Path Handling.
  3. Cherry-pick `58b5a24e` with `validate_path_within` retained as belt-and-suspenders.
  4. Cherry-pick `9ebad89a` with the streaming + tempdir + semver machinery, plus the `bundle_json` field on `DownloadedArtifact` that 22-03's `73e1e3b8` skipped.
  5. Cherry-pick `115b5cfa` (`load_registry_profile` auto-pull) which Plan 22-01's empty provenance commit `3bde347c` deferred until package machinery landed.
  Rationale: each prerequisite is a Rule-4 architectural decision that exceeds cherry-pick scope. Hand-merging 9ebad89a (~+267/-109 LOC across 5 files) under cherry-pick pressure is the highest-risk option in the chain and is the wrong place to make those decisions. See `.planning/milestones/v2.2-phases/22-upst2-upstream-v038-v040-parity-sync/22-03-PKG-SUMMARY.md` § Deviations and `22-03-PKG-PROGRESS.md` for full context.

- **Audit-attestation D-13 fixtures re-enablement** (deferred from Plan 22-05b, 2026-04-28). The 2 `#[ignore]`'d tests in `crates/nono-cli/tests/audit_attestation.rs` blocked by KeyPair `from_pkcs8` re-enablement (sigstore-rs 0.6.4 doesn't expose it). Two viable paths: (a) sigstore-rs upgrade (architectural Rule 4 — may cascade through other crates); (b) fork-internal pkcs8 parser (architectural Rule 4 — adds parsing surface). Either approach should land before publishing v2.2 attestation as production-ready. Document as part of v2.3 audit-attestation hardening sweep.

  Companion deferral on the SAME backlog row: Plan 22-05b's `crates/nono-cli/src/exec_identity_windows.rs::parse_signer_subject` + `parse_thumbprint` chain walkers are similarly deferred — `windows-sys 0.59` does not expose `WTHelperProvDataFromStateData` / `WTHelperGetProvSignerFromChain` without the `Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip` features (`CRYPT_PROVIDER_DATA` shape gated). Plan 22-05b records the `WinVerifyTrust` discriminant alone (Valid/Unsigned/InvalidSignature{hresult}) and sets `signer_subject = "<unknown>"` + empty thumbprint on Valid, plus marks the `authenticode_signed_records_subject` substring assertion test `#[ignore]`. Re-enable the chain walkers (either via Catalog/Sip features OR an in-tree pkcs8 parser providing equivalent walking) alongside the D-13 fixture re-enablement above.

- **WR-01 reject-stage unification** (deferred from v2.1 18.1-04). Align all 5 AIPC HandleKinds on the same reject stage. Currently Event/Mutex/JobObject reject BEFORE prompt; Pipe/Socket reject AFTER prompt. Locked by `wr01_*` regression tests AND now also locked on the wire by Plan 23-01's `RejectStage` discriminator.

- **AIPC G-04 wire-protocol compile-time tightening** (deferred from v2.1 Plan 18.1-02). `Approved(ResourceGrant)` inline at the wire type so `(Approved, grant=None)` becomes a compile-time error. Cascades into `aipc_sdk.rs` child SDK demultiplexer + 23 pre-existing tests.

- **Cross-platform RESL Unix backends** (deferred from v2.1). cgroup v2 / rlimit ports of Windows Job Object caps. Reverse-direction drift (Windows shipped first).

- **WR-02 EDR HUMAN-UAT item** (v3.0). Requires EDR-instrumented runner; no host available.
