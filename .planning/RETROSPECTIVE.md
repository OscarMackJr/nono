# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v2.2 — Windows/macOS Parity Sweep

**Shipped:** 2026-04-29
**Phases:** 3 (22, 23, 24) | **Plans:** 9 | **Commits since `v2.1`:** 146 (29 `feat(...)`)

### What Was Built

- Profile struct alignment with upstream v0.38–v0.40 — `unsafe_macos_seatbelt_rules`, `packs`, `command_args`, `custom_credentials.oauth2` deserialize on Windows; `OAuth2Config::token_url` rejects `http://` fail-closed; `claude-no-keychain` builtin profile loads via `Profile::load_builtin` and inherits `claude-code` agent groups except the keychain group (Phase 22 Plan 22-01).
- Policy tightening — orphan `override_deny` fails closed at profile load via `NonoError::SandboxInit` + `.exists()` pre-filter; `--rollback` ↔ `--no-audit` clap-level mutex with CL-01-M `--no-audit-integrity` carve-out preserved; `.claude.lock` moved to `allow_file` for both `claude-code` and `claude-no-kc`. Two PATTERN-MAP contradictions reconciled (fork already had POLY-02's `clap conflicts_with` wiring at `cli.rs:1602` and a stricter POLY-01 user-intent-only check at `policy.rs:838-885`) (Phase 22 Plan 22-02).
- Package manager subcommand tree — `nono pull/remove/update/search/list` flat-shape (mirrors upstream's actual subcommand registration) with Windows `%LOCALAPPDATA%\nono\packages\<name>` storage, Claude-Code hook registration via fork's `hooks.rs`, signed-artifact verification at install time. 6/8 cherry-picks landed; 2 streaming cherry-picks deferred to v2.3 backlog because they require Rule-4 architectural decisions (`ArtifactType::Plugin` enum variant, `bundle_json` field, `validate_path_within` belt-and-suspenders) (Phase 22 Plan 22-03).
- OAuth2 proxy + reverse-proxy gating — `nono-proxy/src/oauth2.rs` client-credentials Bearer-token injection with in-memory cache; reverse-proxy HTTP upstream restricted to loopback-only by default with `--allow-domain` strict-proxy composition; CL-03-M warns when literal `client_secret` value committed to profile JSON; CL-04-M skips OAuth2-only credentials in manifest export; HG-01-M redacts `OAuth2Config` secrets in `Debug` output (Phase 22 Plan 22-04).
- Audit integrity + DSSE attestation — hash-chained Merkle-rooted `audit-events.ndjson` per session via `AuditRecorder` lifecycle integrated into supervisor; `nono audit verify <id>` recomputes hash chain + Merkle root and fails closed on tamper; `--public-key-file <PATH>` flag pins attestation verification; HG-01-H upgrade landed cryptographic DSSE bundle verification (commit `cffb43b1`); 2 fixture-driven tests `#[ignore]`'d pending sigstore-rs `KeyPair::from_pkcs8` re-enablement (deferred to v2.3) (Phase 22 Plan 22-05a).
- Audit rename + Windows Authenticode + CLEAN-04 sweep — `prune` → `session cleanup` rename with formal `applied_labels_guard::audit_flush_before_drop` regression test (83 LOC) guaranteeing v2.1 CLEAN-04 byte-identical preservation; peer `nono audit cleanup` for ledger files; legacy `nono prune` hidden via `#[command(hide)]` + emits stderr deprecation warning; `WinVerifyTrust` records `Valid` / `Unsigned` / `InvalidSignature{hresult}` discriminant on Windows; chain-walker subject extraction deferred to v2.3 pending `Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip` features in `windows-sys` (Phase 22 Plan 22-05b after CONTEXT-STOP-3 split).
- Windows AIPC ledger emissions — new `RejectStage` enum (`BeforePrompt | AfterPrompt`) on `AuditEventPayload::CapabilityDecision` with `#[serde(default, skip_serializing_if = "Option::is_none")]` for backward-compat with Phase-22-shaped NDJSON files; `handle_windows_supervisor_message` emits `capability_decision` events at all 5 push sites (File + 5 AIPC HandleKinds) with WR-01-locked stage; `nono audit show <id>` surfaces `Capability Decisions: N (M before-prompt, K after-prompt rejections)` counter + `capability_decisions` JSON array; T-23-01 sanitization regression: `recorded_ledger_redacts_session_token` reads on-disk NDJSON and asserts no raw session token bytes appear (Phase 23 Plan 23-01).
- Parity-drift prevention — read-only twin-script drift inventory (`scripts/check-upstream-drift.{sh,ps1}`) with `$(OS)==Windows_NT` Makefile dispatch, 6-category path-prefix lookup, `--format table|json`, 3 frozen golden JSON fixtures enforcing byte-for-byte twin parity; GSD upstream-sync template at `.planning/templates/upstream-sync-quick.md` with byte-exact 6-line D-19 trailer; Mintlify long-form runbook at `docs/cli/development/upstream-drift.mdx`; PROJECT.md `## Upstream Parity Process` cross-link (Phase 24).

### What Worked

- **Cherry-pick-per-commit chain with `Upstream-commit:` trailer preserved provenance through ~33k LOC of churn.** The v2.1 Phase 20 pattern (one upstream commit → one fork commit, never squashed) scaled cleanly to v2.2 Phase 22's 6 plans across 5 upstream feature clusters. Reviewers can `git log --grep "Upstream-commit:"` and replay the exact upstream provenance per fork commit.
- **D-20 fallback (manual replay of cherry-pick) when D-02 thresholds breach.** Plan 22-05a Tasks 4-8 each tripped the D-02 conflict-volume threshold because upstream's commits depended on `audit_ledger.rs` + a `sign_statement_bundle` trust-signing refactor that didn't ship in fork v2.1. Manual replay preserved the same boundary discipline as cherry-pick (one upstream commit ≡ one fork commit) while avoiding hand-merging hundreds of lines under cherry-pick pressure. Pattern: when cherry-pick hits a structural prerequisite gap, manual replay with the trailer block preserved is strictly safer than skipping or squashing.
- **CONTEXT-STOP triggers as a circuit-breaker.** Plan 22-05's split into 22-05a + 22-05b at trigger #3 on upstream `4f9552ec` honored T-22-05-04's ABSOLUTE STOP guard for byte-identical CLEAN-04 invariants. The split installed a permanent `audit_flush_before_drop` regression test as a future-regression guard — converting a one-time gate into a structural invariant. Pattern: when a plan's risk profile inverts mid-execution, splitting along the inversion boundary AND installing a regression test for the invariant the split protected is the most durable resolution.
- **HG-01-H reviewer catch on structural-shape vs cryptographic verification.** Initial 22-05a implementation only verified the *structural shape* of `audit-attestation.bundle` (payload type, base64 envelope, signature presence). HG-01-H reviewer caught that this would silently accept a forged bundle whose signature was wrong. Cryptographic DSSE verification fail-closes on any signature mismatch (commit `cffb43b1`). Pattern: for any "verify" surface, the question to ask in code review is "does this fail-close on tamper, or does it pass-through on shape match?"
- **Parity-drift tooling produced its own regression fixture.** Plan 24-01's twin-script implementation generates 3 frozen golden JSON fixtures from real v0.37.1..v0.40.1 inventories. The fixtures are now both the spec for "what cross-platform commits look like" AND the regression guard for the script itself. Pattern: read-only inventory tools naturally produce their own test fixtures.

### What Was Inefficient

- **PATTERN-MAP contradictions surfaced only mid-execution.** Plan 22-02 reconciled two contradictions where the fork already had POLY-01's stricter check + POLY-02's `clap conflicts_with` wiring. Both should have been caught during 22-02 plan-phase pattern mapping. Lesson: pattern-mapping for fork-vs-upstream parity work should explicitly grep for the upstream behavior being ported BEFORE drafting the plan, not during execution.
- **Phase 22-05 split was unplanned and consumed re-planning context.** The split was the right call but burned ~30 min of re-context-loading mid-plan. Lesson: when a plan's structural prerequisites span >3 upstream commits, pre-stage the split in plan-phase rather than reactive-split during execute-phase.
- **Authenticode chain-walker deferral was discovered late.** Plan 22-05b discovered mid-execution that `windows-sys 0.59` doesn't expose `WTHelperProvDataFromStateData` without `Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip` features — gating `CRYPT_PROVIDER_DATA` shape. The discriminant-only fallback was the right scope decision but should have been caught during 22-05 research-phase. Lesson: for OS-API features behind feature-flags in `windows-sys` / `windows`, audit the feature gates during research, not implementation.
- **Phase 23 layer-2 deviation surfaced only at test-implementation time.** Plan 23-01 Step 7 fallback was authorized in plan, but the realization that `aipc_handle_brokering_integration` cannot reach `pub(super)` `handle_windows_supervisor_message` only landed during test scaffolding. The plan correctly anticipated this (Step 7 fallback exists), but the ANTICIPATED fallback eclipsed the PREFERRED layer-2 coverage. Lesson: when a plan has a "preferred + fallback" coverage shape, audit visibility constraints (`pub(super)` / `pub(crate)`) during plan-phase to choose between them upfront.

### Patterns Established

- **`Upstream-commit:` 6-line trailer block as a D-19 invariant.** v2.2 Phase 22 + Phase 24's GSD upstream-sync template now enforce this shape verbatim:
  ```
  Upstream-commit: <sha>
  Upstream-tag: <tag-or-null>
  Upstream-author: Name <email>
  Upstream-date: <iso8601>
  Co-Authored-By: Name <email>
  Signed-off-by: <DCO sign-off>
  ```
  Mechanical preservation across the chain means future upstream-sync work has a clear provenance discipline.
- **Empty provenance commits for superseded upstream commits.** Plan 22-01's `3bde347c` and Plan 22-04's 5 deferred-but-retained commits use empty commits with the trailer block to record "this upstream commit was reviewed; here's why it didn't land" provenance. Pattern: when an upstream commit is superseded or deferred, an empty commit with the trailer block is a more durable artifact than a code comment.
- **Twin-script drift inventory tooling.** `scripts/check-upstream-drift.{sh,ps1}` with `$(OS)==Windows_NT` Makefile dispatch + golden fixtures sets a template for cross-platform read-only inventory tooling. The 3 frozen golden JSON fixtures double as regression tests for the scripts themselves. Pattern: for cross-platform tooling, twin-script byte-for-byte parity enforced by golden fixtures > one-script-with-conditionals.
- **Discriminant-only fallback when full extraction is feature-gated.** Plan 22-05b records `WinVerifyTrust`'s `Valid` / `Unsigned` / `InvalidSignature{hresult}` discriminant alone when the chain-walker APIs are gated behind unavailable features. The discriminant captures the security-relevant trust-decision boundary; subject + thumbprint are nice-to-have audit metadata. Pattern: when a security check has a "discriminant + metadata" shape, ship the discriminant as the security boundary; defer metadata.
- **`RejectStage` enum on the wire as a verdict-matrix lock.** Plan 23-01 made the WR-01 reject-stage asymmetry observable in the ledger (BEFORE-prompt kinds carry zero-backend-call markers; AFTER-prompt kinds carry one-backend-call markers) without changing the WR-01 decision itself. Pattern: when a product-decision asymmetry is locked by regression tests, mirror it on the wire so future readers see the asymmetry explicitly per event rather than as an out-of-band convention.

### Key Lessons

1. **Pattern-mapping for fork-vs-upstream parity work should explicitly grep for the upstream behavior being ported.** Two PATTERN-MAP contradictions in Plan 22-02 (fork already had the upstream behavior) would have been caught with a 30-second `grep` during plan-phase. Cheap insurance.
2. **`windows-sys` feature gates change the API surface without changing the crate version.** Authenticode chain-walker extraction needs `Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip` features that aren't in the fork's `windows-sys 0.59` `Cargo.toml`. Audit feature gates during research, not implementation.
3. **For "verify" surfaces, the review question is "does this fail-close on tamper, or pass-through on shape match?"** HG-01-H caught structural-shape verification masquerading as cryptographic verification. Both compile, both pass tests against well-formed inputs, but only one fails-close on a forged bundle.
4. **Empty provenance commits beat code comments for deferred upstream work.** A `git log --grep "Upstream-commit:"` discovers all reviewed-but-deferred commits across the chain; a code comment is invisible to git tooling.
5. **Twin-script byte-for-byte parity > one-script-with-conditionals for cross-platform read-only tooling.** Golden fixtures enforce the parity mechanically; conditionals require human review on every change.
6. **Visibility constraints (`pub(super)`, `pub(crate)`) shape integration-test reachability.** Audit them during plan-phase, not test-implementation. Plan 23-01's Step-7 fallback was correctly anticipated but burned context to discover.

### Cost Observations

- Not instrumented for this milestone. The next milestone should record model mix + session count via the gsd session-report tooling.

---

## Milestone: v2.1 — Resource Limits, Extended IPC, Attach-Streaming & Cleanup

**Shipped:** 2026-04-21
**Phases:** 7 (16, 17, 18, 18.1, 19, 20, 21) | **Plans:** 25 | **Sessions:** not instrumented

### What Was Built

- Job Object resource limits — CPU percentage (`JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`), memory (`JobMemoryLimit`), wall-clock timeout (supervisor-side `Instant` + `TerminateJobObject`), process count (`ActiveProcessLimit`); `nono inspect` surfaces active caps via new `Limits:` block.
- `nono attach <id>` on detached Windows sessions — anonymous-pipe stdio bridged through the supervisor; stdin/stdout streaming live; Ctrl-]d clean detach + re-attach; friendly multi-attach busy error. Resize downgraded to documented limitation per D-07 (anonymous-pipe stdio is structurally exclusive of ConPTY; preserves Phase 15 `0xC0000142` fix).
- Extended IPC handle brokering (AIPC) — Socket / Pipe / Job Object / Event / Mutex handles broker-able over the Phase 11 capability pipe with `DuplicateHandle` MAP-DOWN semantics, server-side access-mask validation, and `capabilities.aipc` profile-widening schema. Containment-Job runtime guard via `CompareObjectHandles` prevents the supervisor-own-Job footgun structurally. Child-side SDK with 5 cross-platform `request_*` methods.
- Phase 18.1 gap closure — 5 HUMAN-UAT gaps (G-02..G-06) closed end-to-end: CONIN$ prompts now route through D-04 per-kind templates; JobObject broker uses `CreateJobObjectW` CREATE-if-not-exists parity; dispatcher broker-failure flip makes `(Approved, grant=None)` structurally unreachable; `Profile::resolve_aipc_allowlist()` wired end-to-end; 5 `wr01_*` regression tests lock the WR-01 reject-stage invariant.
- Cleanup workstream — fmt drift fix, 4 deterministic Windows test bugs (incl. UNC-prefix production bug in `query_path`), 10 WIP items triaged, `is_prunable` + `nono prune --older-than <DURATION>` + `--all-exited` + auto-sweep on `nono ps` + `NONO_CAP_FILE` structural no-op + one-shot cleanup (1343 files) + `docs/session-retention.md`.
- Upstream parity sync to v0.37.1 — `rustls-webpki` 0.103.12 security upgrade (RUSTSEC-2026-0098/0099), `keyring://` URIs, env-var filtering, `--allow-gpu` with NVIDIA/DRM/AMD/WSL2 Linux allowlist, GitLab ID tokens for trust signing with `validate_oidc_issuer` fail-closed validator. D-21 Windows-invariance held across 11 commits.
- Windows single-file filesystem grants — per-file Low-IL mandatory-label ACEs via `SetNamedSecurityInfoW` with mode-derived mask; `AppliedLabelsGuard` RAII lifecycle; ownership-skip pre-check for system-owned paths. Unblocks the `claude-code` profile's `git_config` group on Windows.

### What Worked

- **TDD cycles were crisp for gap-closure work.** Plans 18.1-01 (G-02) and 18.1-02 (G-04) both used RED commit → GREEN commit as the atomic structure. The 6 failing `build_prompt_text_*` tests compiled cleanly as `E0425: cannot find function in this scope` — a clean RED that made the GREEN's minimal scope obvious.
- **Empirical discovery via dedicated harness.** The `crates/nono-cli/examples/pipe-repro.rs` binary let us test 13 SDDL variants in minutes to root-cause the Windows 11 26200 `WRITE_RESTRICTED` + logon-SID co-requirement. Without the harness, we'd still be guessing. Pattern worth repeating: when debugging an undocumented OS behavior, spend ~1h building the minimal repro harness first; it pays back 10x.
- **D-19 cross-phase byte-identical preservation checks caught scope drift.** Every AIPC plan asserted `git diff --stat HEAD~N HEAD -- <out-of-scope-paths>` was empty; caught multiple would-be leaks of drive-by changes into cross-phase files.
- **Phase 18.1 dual-run widening proof was decisive.** Running the same rebuilt binary under `--profile claude-code` vs a widened `aipc-widen.json` profile and getting opposite outcomes (denial after prompt vs successful broker) is the cleanest possible validation that `Profile::resolve_aipc_allowlist()` threading is correct.
- **Phase 20 D-21 Windows-invariance as a structural guard.** Mechanical grep check (`zero *_windows.rs files touched`) across 11 commits prevented Windows-specific regressions during the Unix/macOS parity back-port.
- **Concurrent disjoint phases (19, 20, 21).** Phases 19 (cleanup), 20 (upstream parity), 21 (WSFG) ran without mutual interference because their scope was strictly disjoint. Pattern: when a milestone has multiple independent deliverables, call out the file-scope boundary in CONTEXT.md up front.

### What Was Inefficient

- **Phase 21 surfaced the supervisor-pipe regression late.** WSFG-03 (Phase 18 UAT close-out) was planned as Phase 21's final gate, but the first-ever end-to-end `claude-code → supervised → aipc-demo` run only became reachable AFTER Plans 21-02..21-04 landed — at which point the `WRITE_RESTRICTED` pipe ACCESS_DENIED regression surfaced, which was outside Phase 21's scope per its `<critical_rules>`. Result: carry-forward to a dedicated debug session + Phase 18.1 HUMAN-UAT re-run. Lesson: when a phase unblocks a previously-unreachable code path, assume the unblocked path has latent bugs and budget a debug session before promising live-UAT close-out in the same phase.
- **Plan 18-03 `AipcResolvedAllowlist::default()` seed was visible but deferred.** Plan 18-03 explicitly marked `Deferred Issue #1` for Profile threading. Three plans (18-03, 18-04, Phase 18.1's G-02/G-03/G-04) shipped before G-06 closed the gap. HUMAN-UAT discovered the gap end-to-end first. Lesson: deferred issues that block end-to-end validation should either be escalated to same-phase priority or explicitly gated from HUMAN-UAT.
- **CLEAN-02 hypothesis D-07 (parallel env contamination) was wrong.** Three days of debugging under the wrong hypothesis before the empirical data forced the pivot. The actual bugs were 4 distinct deterministic Windows platform issues — including a genuine production bug in `query_path`. Lesson: when flakes have been "in the backlog" for weeks, the "obvious cause" hypothesis (env contamination in test isolation) has already had a confirmation-bias filter applied. Start by writing a single deterministic reproducer per test before hypothesizing the cause.
- **AIPC acceptance shape evolved across Phase 18 → Phase 18.1.** Plan 18-03's Deferred Issue #1, Plan 18-01's JobObject `OpenJobObjectW` vs CREATE semantics, G-04's `(Approved, grant=None)` wire-protocol gap — all three surfaced only during HUMAN-UAT. Lesson: for protocol-heavy phases, stage a live end-to-end smoke test earlier (mid-phase) rather than gating entirely on post-implementation UAT.

### Patterns Established

- **`examples/<probe>.rs` harness for OS-behavior spelunking.** `pipe-repro.rs` established the pattern — a minimal binary under `crates/nono-cli/examples/` that exercises a narrowly-scoped OS API with parameterized inputs for rapid hypothesis testing. Worth preserving + extending.
- **Single-site flow-control tuple reshaping instead of type-level enforcement.** Plan 18.1-02 G-04 rewrote `let grant = if decision.is_granted() { ... }` → `let (decision, grant) = if decision.is_granted() { ... }` at ONE site, making the illegal `(Approved, grant=None)` shape unreachable without cascading into 23 test construction sites. Preferable to wire-protocol compile-time tightening when the tightening would be invasive.
- **Module-level docstring as verdict matrix.** Plan 18.1-04's `//!` docstring at the top of `capability_handler_tests` documents the WR-01 reject-stage matrix + CONTEXT D-14 deferral note. Future readers see the product decision inline with the tests that lock it. Pattern worth repeating for empirically-established invariants.
- **Decimal phases for same-milestone follow-ups.** Phase 18.1 followed Phase 18 for 5-gap HUMAN-UAT remediation. The decimal numbering kept provenance obvious (gaps surfaced during Phase 18 HUMAN-UAT) while keeping Phase 19, 20, 21 numbering stable.
- **Ownership pre-check for subtractive labels.** Phase 21 `try_set_mandatory_label` now skips system-owned paths. Pattern: when an OS security mechanism is subtractive (like Low-IL mandatory labels), test ownership before applying rather than failing on the OS error.

### Key Lessons

1. **Undocumented OS behavior requires systematic reproducers, not documentation searches.** MSDN does not describe the Windows 11 26200 `WRITE_RESTRICTED` + logon-SID co-requirement. The only path to root cause was the 13-SDDL-variant iteration. Build the harness first; read the docs second.
2. **End-to-end unblocking phases have hidden latent-bug budgets.** Phase 21 unblocked the `claude-code → aipc-demo` flow and surfaced TWO latent bugs (supervisor-pipe ACCESS_DENIED + `WSAStartup` gap) that had been present since Phase 11 / Phase 18-04 but unreachable. Budget a debug session after any phase that opens a new end-to-end code path.
3. **Deferred issues that block validation should be flagged as milestone-blockers, not plan-local issues.** Plan 18-03 Deferred Issue #1 blocked G-06 validation. Three plans shipped before it was fixed. Rule of thumb: if a deferred issue prevents end-to-end HUMAN-UAT from passing, it's a milestone-level blocker, not a plan-local tech-debt item.
4. **Empirical dual-run is the decisive validation for configuration wiring.** Running the same binary under two profiles with opposite outcomes is a cleaner wiring proof than any number of unit tests. Keep in the verification toolkit.
5. **D-21 Windows-invariance mechanical guards prevent regression during non-Windows work.** The `zero *_windows.rs files touched` grep across Phase 20's 11 commits caught nothing — because the guard was in place from commit 1. Mechanical guards are cheap insurance.

### Cost Observations

- Not instrumented for this milestone. Future milestones should record model mix + session count via the gsd session-report tooling.

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change |
|-----------|--------|-------|------------|
| v1.0 Windows Alpha | 4 | ~12 | Initial Windows-first cut; signed artifacts + WFP packaging |
| v2.0 Windows Gap Closure | 11 | 29 | Closed 7 feature gaps; introduced decimal-phase pattern (Phase 15) for carry-forward closure |
| v2.1 Resource Limits + AIPC + Cleanup | 7 | 25 | Added decimal-phase same-milestone gap closure (Phase 18.1); established `examples/<probe>.rs` harness pattern; mechanical D-21 invariance guards |
| v2.2 Windows/macOS Parity Sweep | 3 | 9 | Cherry-pick-per-commit chain with `Upstream-commit:` trailer scaled to ~33k LOC; CONTEXT-STOP triggers as circuit-breaker (22-05 split); D-20 manual-replay fallback when D-02 thresholds breach; twin-script drift inventory tooling; discriminant-only fallback when full extraction is feature-gated |

### Cumulative Quality

| Milestone | Tests (approx) | Clippy | Fmt | Notes |
|-----------|----------------|--------|-----|-------|
| v1.0 | baseline | clean | clean | — |
| v2.0 | +49 new (Phase 11 + 12 + 13 UAT scaffolding) | clean | clean | — |
| v2.1 | +108 new (18-01..04: 21 + 18.1: 19 + RESL: 8 + ATCH: 17 + CLEAN: 8 + WSFG: 5 + UPST: ~30) | clean | clean (post-19-01) | 5 deterministic Windows test bugs fixed incl. UNC-prefix prod bug |
| v2.2 | +60 new in Phase 23 alone (audit_integrity + capability_handler_tests + audit_commands + aipc_handle_brokering_integration + audit_flush_before_drop) + 30-assertion drift integration test (Phase 24) + ~80 new across Phase 22 plans | clean | clean | 2 fixture-driven attestation tests `#[ignore]`'d pending sigstore-rs upgrade; 1 authenticode chain-walker test `#[ignore]`'d pending `windows-sys` feature gates |

### Top Lessons (Verified Across Milestones)

1. **Empirical OS-behavior reproducers outperform documentation searches.** (v2.0 Phase 15 direction-b discovery; v2.1 Phase 21 debug session; both required harness-based iteration.)
2. **Deferred issues that block end-to-end validation are milestone-level blockers, not plan-local.** (v2.0 Phase 14 Plan 14-01 escalated to Phase 15; v2.1 Plan 18-03 Deferred Issue #1 escalated to Plan 18.1-03.)
3. **Decimal-phase numbering preserves provenance for carry-forward work.** (v2.0 Phase 15 carried Phase 14-01's architecture work; v2.1 Phase 18.1 carried Phase 18 HUMAN-UAT gaps; v2.2 Phase 22-05 split into 22-05a/b at CONTEXT-STOP-3.)
4. **D-21 Windows-invariance as a mechanical grep guard.** (v2.0 used it per-plan; v2.1 extended it to cross-phase byte-identical preservation checks; v2.2 extended it again to D-19 cross-phase byte-identical preservation across cherry-pick chains.)
5. **`Upstream-commit:` trailer block preserves provenance through structural churn.** (v2.1 Phase 20 introduced it; v2.2 Phase 22 scaled it to ~33k LOC + ~146 commits; Phase 24's GSD template enshrines the verbatim 6-line shape for future maintainers.)
6. **For "verify" surfaces, the review question is "does this fail-close on tamper, or pass-through on shape match?"** (v2.2 HG-01-H caught structural-shape verification masquerading as cryptographic verification.)
