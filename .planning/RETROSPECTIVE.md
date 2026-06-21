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

## Milestone: v2.5 — Backlog Drain + UPST5

**Shipped:** 2026-05-20
**Phases:** 4 (37, 41, 42, 43) | **Plans:** 24 | **Tasks:** 34 | **Commits since `v2.4`:** 172 (168 conventional, ~5 days)

### What Was Built

- Linux RESL backends now kernel-enforced via cgroup v2 (`memory.max` / `cpu.max` / `pids.max`); cgroup v1 hosts fail closed with `NonoError::UnsupportedKernelFeature { feature, hint }` carrying the LOCKED `cgroup_no_v1=all` boot-flag hint; FFI exhaustive map_error arm → `ErrUnsupportedPlatform` (ABI-stable). Closes 3-year Linux silent-no-op security regression (Phase 37).
- Registry-profile auto-pull (cargo-install-style) with `--no-auto-pull` + `NONO_NO_AUTO_PULL` env-var opt-out, `ResolveContext` threading, `DiagnosticFormatter` footer; 5 e2e integration tests + multi-endpoint mock TCP server; CI-time keyless sigstore-sign via GH Actions OIDC. Sigstore-rust 0.7.0 bump closes 2 pre-existing TUF flakes carried since v2.3 (Phase 37).
- All 7 CI lanes back to green (Linux + macOS Clippy + 5 Windows CI jobs); MSI validator `-BrokerPath` mandatory-parameter mismatch resolved; cross-target clippy verification protocol codified in CLAUDE.md as enforcement-shaped MUST/NEVER rule + `.planning/templates/cross-target-verify-checklist.md` reference artifact (Phase 41).
- v24 Windows broker code-review closure: `BrokerNotFound` FFI remap (was `-1 ErrPathNotFound`, semantically wrong → `ErrSandboxInit`); broker null/INVALID handle rejection pre-cross-boundary; empty `--inherit-handle` list rejection; Job-object test silent-SKIP→FAIL policy resolved (Phase 41).
- First UPST5 audit cycle where the `windows-touch:yes` column actually fires: DIVERGENCE-LEDGER for `v0.53.0..v0.54.0` (7 clusters / 18 commits / 4 will-sync + 2 fork-preserve + 1 won't-sync; 3 windows-touch:yes commits dispositioned: `5d821c12` + `0748cced` + `ce06bd59`). `## ADR review` per-cell L/M/H verdict table on 5 dimensions confirms Phase 33 Option A `continue` strategy. `## Empirical cross-check` covers 4 Phase-41-touched fork-shared files (Phase 42).
- UPST5 sync execution: 11 D-19 cherry-picks across Clusters 1+3+7 with verbatim lowercase 6-line `Upstream-commit:` trailers + 3 D-20 manual replays across Clusters 4+5 with `Upstream-replayed-from:` trailers in `a46b6bf9` (combined 0748cced + 5d821c12 Windows registry detection replay) + `fe04e887` (ce06bd59 cross-platform `platform.rs` foundation — 659 lines verbatim + Windows registry extensions). Cluster 2 mid-flight reclassified `will-sync → split`: workspace edits (MSRV 1.95, nix/landlock/getrandom workspace deps) landed fork-authored; Edition 2024 source-file migration deferred to v2.6/UPST6 (DIVERGENCE-LEDGER amendment `79715aa5`). D-43-E1 Windows-only-files invariant respected. 2208 tests pass / 0 failed / 19 ignored on Windows host (Phase 43).

### What Worked

- **Worktree-mode discipline scaled across 4 phases.** Phases 37/41/43 each carried `human_needed` verification status, NOT because the work was incomplete but because the decisive close signal (live GH Actions CI on push) lives outside worktree-executor reach. The pattern is now mature: all 6 Phase 43 plans use identical "deferred to orchestrator post-merge" language; all 6 Phase 43 PR-SECTION.md contribution artifacts stage cleanly for umbrella PR assembly. Pattern: when CI is the decisive close gate, structural deferral with staged contribution artifacts is the right shape, not a verification gap.
- **Cross-target clippy verification protocol promoted from advisory to enforcement.** Phase 41 codified what was previously a memory note (`feedback_clippy_cross_target`) as a CLAUDE.md MUST/NEVER bullet + a referenceable `.planning/templates/cross-target-verify-checklist.md` artifact. The third-miss risk (CI run 25972316892 + 25973911653) drove the promotion: Windows-host workspace clippy silently passes while Linux/macOS cfg-gated code drifts. Pattern: when an advisory rule misses twice on the same surface, promote it to enforcement-shaped (MUST/NEVER + close-gate fail-closed rule + referenceable checklist), not just memory.
- **Baseline-aware CI gate inheritance worked end-to-end.** Phase 41 close SHA `13cc0628` was recorded as the baseline anchor in `.planning/templates/upstream-sync-quick.md`; all 6 Phase 43 plan CLOSE-GATE.md files reference it consistently; Phase 43 plan-phase resolved the inheritance cleanly without manual reconciliation. Pattern: when a baseline-defining phase closes, immediately update the template anchor (don't wait until consuming phases start); consuming phases inherit the anchor by reference, not by re-derivation.
- **Cluster 2 mid-flight reclassification (`will-sync → split`) preserved auditability.** Phase 43 Plan 43-01 discovered that upstream `8b888a1c` had cross-cluster re-export dependencies that empirically falsified the Phase 42 ledger's cluster-isolation assumption. Rather than force a Rule-4 STOP, the cluster was reclassified to `split` with workspace edits absorbed (43-01b) and source migration deferred (UPST6). DIVERGENCE-LEDGER amendment at commit `79715aa5` preserves the audit trail. New memory `feedback_cluster_isolation_invalid` records the lesson. Pattern: when an audit cluster's isolation assumption falsifies empirically during execution, `split` is a valid third disposition (alongside `will-sync` / `fork-preserve` / `won't-sync`); ledger amendment is the correct response.
- **`feedback_clippy_cross_target` memory paid off — three times.** Phase 37 + Phase 41 + Phase 43 all consumed the memory's "cross-target clippy required for cfg-gated Unix code" rule. Each invocation surfaced real cfg-drift Windows-host workspace clippy missed. Pattern: persistence-of-knowledge across phases is what makes feedback memory load-bearing; one save → many reuses.

### What Was Inefficient

- **REQUIREMENTS.md traceability checkboxes never updated during execution.** All 13 v2.5 requirements remained `not_started` in `.planning/REQUIREMENTS.md:195-209` even though every phase's VERIFICATION.md confirmed SATISFIED. Documentation drift only — milestone close's archive step reconciles — but the workflow could be tightened to flip checkboxes at phase-close time, not milestone-close time. Lesson: traceability tables are only useful as living state; consider auto-update at phase close, not deferred batch-update at milestone close.
- **Phase 41 needed THREE verification cycles before reaching codebase-VERIFIED.** Initial verification (2026-05-16T20:30Z, post-Plan-41-09) PARTIAL on cross-target clippy due to 6 dead-code findings invisible to Windows-host check. Second (T21:48Z, post-Plan-41-09 second pass) human_needed 5/5 but CI run 25973911653 surfaced 5 NEW failure classes the Windows-host verification missed AGAIN. Third (T23:15Z, post-Plan-41-10) finally reached structurally clean. The Class F verifier-protocol gap (cross-target clippy mis-VERIFIED twice) drove the CLAUDE.md enforcement-rule codification. Lesson: when verification of a single phase needs three cycles, the gap is in the verification protocol itself, not the work; surface the protocol fix as a Phase deliverable, not a follow-up todo.
- **Phase 37 SUMMARY one-liner extraction yielded raw garbage.** `gsd-sdk query summary-extract` returned "1. [Rule 3 - Blocker] clap env-var bool-parsing quirk" and several empty "One-liner:" strings from the raw scraping pass. The `milestone.complete` CLI's auto-populated accomplishments list required ~10 lines of hand-cleanup before MILESTONES.md was readable. Lesson: SUMMARY frontmatter `one_liner` fields should be populated explicitly at plan-close time, not inferred from headline parsing.
- **Phase 43 review surface accumulated 11 polish items (6 warnings + 5 info).** Most cluster around `crates/nono-cli/src/platform.rs` (compare_versions Ord asymmetry, REG_DWORD malformed fallback, case-sensitive registry name match) — all consequences of taking 659 LOC verbatim from upstream `ce06bd59` without adapting the antisymmetry conventions of the fork's `Ord` implementations. Lesson: when a D-20 manual replay introduces a substantial new module verbatim from upstream, schedule a same-phase review pass focused on convention adaptation, not just compile-and-test.

### Patterns Established

- **`split` as a valid fourth disposition for upstream audit clusters.** Beyond `will-sync` / `fork-preserve` / `won't-sync`, a cluster whose isolation assumption falsifies mid-execution can be split into partial-advancement (workspace/scaffold edits absorbed fork-authored) + deferred-source-migration (cherry-pick chain held for next cycle). DIVERGENCE-LEDGER amendment with explicit per-portion disposition + rationale is the durable artifact. Pattern: when cluster isolation falsifies, `split` preserves momentum without sacrificing audit-trail integrity.
- **Cross-target clippy verification as a close-gate fail-closed enforcement rule.** Promoted from memory note to CLAUDE.md MUST/NEVER + `.planning/templates/cross-target-verify-checklist.md` reference artifact + Phase-37/41/43 close-gate consumers. The PARTIAL Disposition rule (when cross-target toolchain unavailable on dev host, deferred to live CI with explicit categorization in SUMMARY frontmatter `skipped_gates_load_bearing`) lets the gate fire even when toolchain absence would otherwise force a SKIP. Pattern: enforcement-shaped MUST/NEVER + close-gate consumer + PARTIAL fallback for environmental gaps is the durable form for cross-platform verification rules.
- **`platform.rs` cross-platform module convention.** Phase 43 Plan 43-05 ported upstream `ce06bd59` verbatim as a NEW cross-platform module dispatched by `cfg!(target_os = "windows")`; Plan 43-06 extended the Windows branch with registry detection factories INSIDE the same module (NOT in fork-only `*_windows.rs`). D-43-E1 Windows-only-files invariant adapted via 4-condition codified addendum exception (required cross-platform struct field; cross-platform default factory only; ≤5 lines; documented in SUMMARY + STATE). Pattern: cross-platform modules with Windows-branch factories live in the same file as the Linux/macOS branches; D-43-E1 invariant relaxed via documented codified-addendum 4-condition rule.
- **DIVERGENCE-LEDGER `## Empirical cross-check` subsection.** Phase 42 introduced spot-checking of fork-shared files against the drift-tool output to validate the path-filter blind-spot mitigation (Phase 39 empirical-cross-check precedent). 4 Phase-41-touched files sampled, all confirmed. Pattern: every UPST audit ledger should include a `## Empirical cross-check` covering ≥3 fork-shared files; sample selection follows recent-edit hot spots (most recently touched cross-platform files in the prior phase).

### Key Lessons

1. **When CI is the decisive close gate, structural deferral with staged contribution artifacts is the right shape.** 3 of 4 v2.5 phases landed `human_needed` not because they were incomplete but because the worktree executor architecturally cannot push branches. Don't conflate "deferred to orchestrator post-merge" with "verification gap."
2. **When an advisory rule misses twice on the same surface, promote it to enforcement.** Cross-target clippy missed once via PR 41-09 verification gap, again via PR 41-10 second-pass — third miss would be inexcusable. Promote MUST/NEVER + referenceable artifact, not just memory.
3. **When a baseline-defining phase closes, immediately update the template anchor.** Phase 41 close SHA `13cc0628` flowed into Phase 43's CI gate by inheritance, not re-derivation. Inheritance points (template files, anchored SHAs) are state — update them as a close-gate task, not as a downstream consumer task.
4. **Cluster isolation can be empirically false; `split` is a valid third disposition.** Phase 43 Plan 43-01 falsified the Phase 42 ledger's cluster-2 isolation assumption. DIVERGENCE-LEDGER amendment + memory note + `split` reclassification preserves audit-trail integrity without forcing Rule-4 STOP. Future UPST plan-phase must diff-inspect re-export surfaces, not just `--name-only` cluster boundaries.
5. **When verification of a single phase needs three cycles, the gap is in the verification protocol.** Phase 41's Class F verifier-protocol gap drove a CLAUDE.md change. Surface protocol fixes as Phase deliverables, not follow-up todos.
6. **REQUIREMENTS.md traceability checkboxes are living state, not deferred-batch-update artifacts.** Consider auto-update at phase close, not deferred to milestone-close archive step.
7. **D-20 manual-replay verbatim ports need a same-phase convention-adaptation review pass.** Phase 43's 11 REVIEW.md items cluster around `platform.rs` because the 659-LOC verbatim port didn't get a fork-convention review (Ord antisymmetry, case-handling, hex-fallback safety). Build it into the D-20 replay template.

### Cost Observations

- Not instrumented for this milestone. Future milestones should record model mix + session count via the gsd session-report tooling.
- Anecdotal: this milestone was unusually short (5 days) for the work delivered (4 phases / 24 plans / 172 commits). Phase parallelism (37 + 41 ran in parallel; 42 + 43 sequential after) plus mature worktree-mode discipline likely the main throughput driver.

---

## Milestone: v2.7 — Windows supervised-run hardening

**Shipped:** 2026-05-26 (archived 2026-05-28) | **Phases:** 2 (51, 52) | **Plans:** 6

### What Was Built
The no-PTY Low-IL broker path (`WindowsTokenArm::BrokerLaunchNoPty`) so heavy-runtime children (the 234 MB self-contained `claude.exe`) survive DllMain under `nono run` while keeping `NO_WRITE_UP` write-deny — closing the `0xC0000142` regression. Phase 52 field-validated it live on Win11.

### What Worked
- Root-cause-first debugging: the `claude-exe-dll-init-failed` session nailed the WRITE_RESTRICTED restricting-SID mechanism before any code changed; the fix addressed the cause (broker token), not the symptom (a null-token would have regressed write-deny — explicitly rejected).
- A live HUMAN-UAT phase (52) as a first-class deliverable for a security fix that can't be unit-tested.

### What Was Inefficient
- **The "shipped" milestone was not actually working.** Phase 52's repro-B "PASS" ran a *pre-CR-01* Program Files binary; the tagged v2.7 build (`637a426c`) was doubly-broken on the real-console no-PTY path (GLE=87 spawn failure, then swallowed stdout). Both were only found in post-ship operator testing (debug `broker-nopty-createproc-gle87` + `nopty-broker-stdout-swallowed`, fixed `d8b7ce00` + `005b4c9e`). Net: the headline feature took **two more post-tag debug cycles** to actually function.

### Key Lessons
- **Version-string sameness is a trap (twice this milestone).** "It's 0.57.0 / it passed UAT" did not mean the code under test matched HEAD. Always confirm the *commit/binary identity* under test, not just the version string — both post-ship bugs hid behind an unchanged version number. (Reinforces `feedback_verify_debug_hypothesis`.)
- A field-UAT "PASS" is only as good as the binary it ran; record the exact commit + binary mtime in the UAT artifact, and re-run after any post-UAT relink.

### Cost Observations
- Not instrumented. Notable: the milestone "completed" in 2 phases but required ~2 post-ship debug cycles + a tooling/docs burst (MSI signing helper, POC docs, intern guide, gap analysis) before the feature was genuinely usable on a real console.

---

## Milestone: v2.8 — UPST7 + v2.7 Drain & Release  ·  v2.9 — Windows Sandbox-the-Tools

**Shipped:** 2026-06-06 (archived together) | **Phases:** 10 (v2.8: 53-59 · v2.9: 60-62) | **Plans:** ~38

### What Was Built
- **v2.8** — Drain-then-sync: shipped the untagged post-v2.7 fixes as signed `v0.57.5` (after fixing a `release.yml` signing-order defect that had shipped unsigned MSI payloads), audited + absorbed the UPST7 window (`v0.57.0..v0.59.0`), then added fine-grained `allow_domain` path/method network filtering (REQ-NET-01), the `bw://` Bitwarden credential source (REQ-CRED-01), `session_hooks` with a Windows broker executor + ADR (REQ-HOOK-01), and supervisor IPC robustness — keep-alive + bounded read-timeouts on Unix and the Windows Named-Pipe AIPC path (REQ-IPC-01). Audit: `tech_debt`, 10/10 reqs, 0 blockers.
- **v2.9** — A Windows confined coding-loop POC (Low-IL-jailed file/shell tool calls), out-of-box WFP kernel network enforcement for supervised runs, and a CI-signed public release — **published as `v0.62.2`**.

### What Worked
- **The milestone-close pre-flight caught a false "shipped" claim.** Reading `gh release list` (not the tag list or the "judge by assets, not red ✗" heuristic) revealed v2.9 had never actually published despite `v0.62.0`/`v0.62.1` tags existing. The audit-before-archive discipline turned a bookkeeping task into a real fix.
- **CI-log forensics to triage release blockers fast:** `gh run cancel` to unlock locked logs immediately + distinguishing `release.yml` (plain `cargo build`) from `ci.yml` (`-Dwarnings`) let me separate the two *hard* compile errors that gate publish from the chronic lint debt that doesn't.
- Per-phase code review caught real fail-open security blockers pre-verify (Phase 56 CR-01 profile-rule flattening → unrestricted CONNECT tunnel; Phase 58 CR-02/03 fail-closed gaps).

### What Was Inefficient
- **Two `cfg`-gated compile errors reached *release tags*.** The Windows dev host never compiles `cfg(unix)`/`cfg(not(windows))` branches, so an `E0716` (Phase 60) and an edition-2024 let-chain (Phase 58) shipped uncompiled-on-Unix and broke all 4 Linux/macOS `release.yml` build legs — costing **three release attempts** (`v0.62.0` → `v0.62.1` → `v0.62.2`) because rustc error-recovery hid the second error behind the first (one revealed per attempt).
- v2.8 and v2.9 were developed interleaved, leaving STATE/ROADMAP/memory drift (the "shipped as v0.62.0" claim) that had to be untangled at close.

### Patterns Established
- **Fork release-tag rule:** leapfrog the crate version PAST upstream's highest tag to get a collision-free `v*.*.*` trigger (`0.58.0 → 0.62.x`).
- **Release-vs-CI build distinction** as a triage tool: a red *build leg* is a real publish blocker; only `Create Release`-downstream jobs (crates.io 303, homebrew) are the cosmetic ones — always `gh release list` to confirm a release object exists before declaring "shipped".

### Key Lessons
- **"Tagged" ≠ "published."** A pushed `v*.*.*` tag whose `release.yml` run failed produces no release object. The v2.7 retro's "version-string sameness is a trap" recurs one layer up: tag-existence is a trap too. (Hardened into memory `feedback_clippy_cross_target` + the corrected `project_v28_opened`.)
- **The cross-target blind spot is now a *release* hazard, not just a close-gate one.** A Windows-host `cargo build` PASS is no evidence the Unix/macOS release legs compile; CI is the only cross-compile signal (local cross-build blocked by the ring/aws-lc-sys C-toolchain). Lean on CI logs early, before cutting a public tag.

### Cost Observations
- Not instrumented. Notable: the v2.8 *close* expanded into a full v2.9 release-rescue + dual-milestone archive — most of the session's cost was diagnosing/fixing the unpublished release and reconciling the interleaved-milestone drift, not the archival mechanics.

---

## Milestone: v2.10 — Kernel-Driver Spike + EDR UAT + macOS Upstream Parity

**Shipped:** 2026-06-11 | **Phases:** 4 (63-66) | **Plans:** 13 | **Tag:** `v2.10` (no new crate release)

### What Was Built
- **Gap 6b minifilter feasibility spike (DRV-01/02/03)** — a test-signed Windows FltMgr minifilter (`drivers/nono-fltmgr/`, out-of-workspace C/C++ WDK) that live-proves a kernel→user→kernel `IRP_MJ_CREATE` deny over `\NonoPolicyPort` on a Secure-Boot-OFF/HVCI-OFF Azure VM.
- **Go/no-go ADR (DRV-04), Accepted: No-go/Conditional-go** — with measured `FLT_PREOP_PENDING` latency (SPAN medians 0.553/0.569 ms, ~900× under the fail-open envelope); `DRV-PROD-01` deferred to v2.11/v3.0.
- **WR-02 CLOSED (EDR-01/02)** — the EDR HUMAN-UAT deferred since v2.1, executed live under Sysmon+Defender EDR-proxy (two passes, no-exclusion → with-exclusion).
- **macOS Seatbelt parity through `v0.61.2` (MACOS-01/02/03)** — DIVERGENCE-LEDGER + P1 ordering/CWD cherry-picks + gate-65-A live re-validation PASS + macOS CI green HARD gate.

### What Worked
- **Host-availability gates were planned up-front, not discovered.** The roadmap pre-declared the three host gates (WDK VM, real macOS host, EDR-instrumented Windows host) so the autonomous phases (groundwork, audit, ADR drafting) ran to completion and only the genuinely human/host-bound steps blocked — keeping the spike productive across context exhaustions.
- **The macOS CI HARD gate caught a real enforcement gap.** Treating green `macos-latest` as close-blocking (the v2.9 cross-target-drift lesson applied) surfaced that macOS `--timeout`/`RLIMIT_NPROC` doesn't fire on a real host — a never-validated Phase-37 claim, now filed as a defect instead of silently shipping.
- **Evidence-derived verdict, not pre-committed.** DRV-04 weighed five inputs (latency, cert cost, maintenance, spike-defect signal, security-gap-closed) against the existing model and landed No-go/Conditional-go honestly — the spike's job was to de-risk a decision, and it did.

### What Was Inefficient
- **CI-log availability fought the macOS-failure diagnosis.** GitHub only serves a job log after the *whole* run completes, so cancelling mid-run to "check faster" repeatedly destroyed the log (BlobNotFound), burning cycles — the fix was to let the slow ubuntu long-pole finish naturally. (Durable gotcha captured in the Phase 65 checkpoint.)
- **A runner-death masqueraded as a clean test failure.** The host-dependent resl test spawned processes unbounded on the GH macOS runner (no RLIMIT enforcement) → starved the runner → "lost communication," which read as an opaque failure until the check-run annotations API revealed the cause. Env-gating the test off CI (`NONO_RESL_HOST_VALIDATED`) was the right move, not a code change to `macos.rs`.
- **Interactive-only capture steps (Bastion/DebugView) don't fit `az run-command`.** The latency capture was inherently session-0-hostile; recognizing this earlier would have saved a couple of empty-log attempts.

### Patterns Established
- **Spike milestone shape:** autonomous groundwork/audit/ADR-draft phases + explicitly host-gated validation phases, with the go/no-go ADR shipped `Proposed` and flipped to `Accepted` only on human D-06 sign-off (decision stays the human's).
- **EDR validation as EDR-proxy:** when cloud-EDR (MDE) is unavailable, Sysmon + Defender AV is a representative proxy sufficient to close the boundary claims — with the "validated under EDR-proxy, not cloud-EDR" caveat recorded explicitly rather than over-claiming.

### Key Lessons
- **A favorable measurement doesn't flip a No-go.** Latency landed ~900× under envelope, but it was never the limiting factor — the deciding inputs were recurring cert/maintenance cost and the fragility signal (18 spike defects). Measuring the non-binding input is still worth it to *retire* it as an objection.
- **"Test passes on CI" can hide "enforcement never validated."** The macOS resl tests had been green for milestones by virtue of being host-blocked/unexercised; forcing real-host validation (gate-65-A) is what converted an assumed-true requirement into a found defect.

### Cost Observations
- Not instrumented. Notable: most wall-clock went to host-bound work (VM driver build/sign/load, macOS CI rehab, EDR host prep) and CI-log forensics, not planning/archival mechanics. Several context exhaustions across the milestone; the per-phase `.continue-here` checkpoints carried state across them effectively.

---

## Milestone: v2.13 — Carry-Forward Closeout (Dark Factory)

**Shipped:** 2026-06-18 | **Phases:** 6 (76-81) | **Plans:** 13 | **Tag:** `v2.13` (milestone marker, no crate release)

### What Was Built
- **Self-verifying harness foundation (DARK-01)** — `scripts/verify-dark.ps1`: gate auto-discovery, `Test-Precondition`→`Invoke-Gate` contract, typed JSON verdicts (PASS/FAIL/SKIP_HOST_UNAVAILABLE), exit mapping (0/2/3), `.nono-runtime/verdicts/` persistence + a `harness-self-check` reference gate — the contract every host-gated phase writes against.
- **Milestone-close aggregator (DARK-02)** — the no-flag `verify-dark.ps1` rolls every discovered gate verdict into `_aggregate.json` with a CI-consumable exit contract; v2.13 completion is evaluable from harness output alone.
- **Copilot CLI end-to-end confinement (CPLT-01/02/03)** — runtime ancestor-chain `FILE_READ_ATTRIBUTES` RAII guard (binary + `--workspace` chains) + idempotent one-time-admin `nono setup --grant-ancestors` + the `copilot-e2e` gate, fixing the Node-ESM/AppContainer drive-root `lstat` limit from v2.12.
- **Cross-process classification (CLAS-01/02)** — daemon control-pipe `Classify` verb making `nono classify <pid>` authoritative cross-process, caller-gated + tenant-safe; live Win11 26200 PASS.
- **WFP isolation proof + nono-ts ergonomics (WFP-01/TSRG-01)** — per-SID WFP egress isolation gate + `confinedRun` default Low-IL broker arm and exe-dir auto-cover.
- **Clean-host install UAT (INST-01)** — MSI build fix (`+crt-static`, Vital=no) + the unattended `clean-host-install` gate replacing the v2.11 Phase 67 interactive UAT.

### What Worked
- **The terminal gate caught fail-secure regressions before close.** Code review on the Phase 76 all-run path found two Criticals (false-FAIL on a gate crash, false-PASS on an unknown verdict) — exactly the failure modes that would have made the whole "trust the harness verdict" mandate unsafe. Fixing them at the foundation phase protected every gate built on top.
- **Composition over green-field paid off.** No new wire protocol or framework: the Classify verb rode the existing Phase 74 SDDL control pipe, WFP-01 reused the Phase 75 per-agent add/remove, and the gates were plain PowerShell over the Phase 76 contract. Deltas stayed small and reviewable.
- **Aggregator auto-discovers gates from `scripts/gates/*.ps1`, never from globbing `verdicts/`.** That single design choice means a stale or stub verdict file can't leak into the rollup — the close signal reflects gates that actually ran.

### What Was Inefficient
- **The live aggregate close signal is FAIL on the dev host for an environmental reason.** A stale `C:\Program Files\nono\nono.exe` (no `agent` subcommand) on PATH makes the wfp gate FAIL — not an aggregator defect, but it means the headline close artifact isn't green without re-provisioning (fresh `target\release` on PATH + wfp host setup). The Dark Factory mandate makes the *signal* trustworthy, but a green signal still needs a correctly-provisioned host.
- **SUMMARY `requirements_completed` frontmatter was mostly left empty** (only 76-02/77-01/77-02 populated it), forcing the audit to lean entirely on VERIFICATION.md for coverage cross-referencing — a process-hygiene gap to close going forward.
- **Two requirements' literal green PASS are structurally host-gated** (CPLT-03 GitHub org policy, INST-01 clean Win11 VM) — the gates correctly fail-closed to SKIP, but the milestone can't show those as literal green without external provisioning.

### Patterns Established
- **Dark Factory close shape:** every historically host-gated item ships a single-invocation scripted gate emitting a machine-readable verdict, with one aggregator as the milestone-close signal — interactive human UAT demoted to a single unattended run.
- **Gate contract = `Test-Precondition` (returns null or a SKIP reason) + `Invoke-Gate` (returns a verdict object, never calls `exit`);** the runner owns exit-code mapping. Lets gates compose into the aggregator without each reinventing exit semantics.
- **SKIP_HOST_UNAVAILABLE as a first-class verdict** distinguishes "host can't run this" from PASS/FAIL, so a host-gated deferral is actionable (no ambiguity about why) rather than a crash or a false negative.

### Key Lessons
- **"Machine-readable verdict" must include the fail-secure edges.** A harness is only trustworthy if a crashed gate is FAIL (not silently PASS) and an unrecognized verdict is FAIL (not optimistically PASS) — the two Criticals code review caught. Verifying the verifier is the load-bearing step.
- **Automating verification doesn't eliminate host provisioning.** v2.13 collapsed human *judgment* to one invocation, but a green aggregate still needs the right host (fresh binary on PATH, Copilot-enabled account, clean VM). The Dark Factory win is repeatability + machine-evaluability, not zero ops.
- **Invoke `verify-dark.ps1` via `-File`/direct, never `pwsh -Command "<bare path>"`** — the bare-command form swallows the gate's exit N → 1, destroying the very exit-code contract the harness exists to provide.

### Cost Observations
- Not instrumented. 93 commits over ~2 days (2026-06-16 → 06-18). Most wall-clock went to live Win11 host validation (daemon rebuild/restart, WFP host setup) and the gate-hardening loops, not planning/archival mechanics. Audit run before close confirmed 10/10 reqs + WIRED with 0 defects and caught stale CPLT-01/02/03 checkboxes (the recurring SDK roadmap-checkbox drift).

---

## Milestone: v3.0 — Enterprise Hardening I (Deploy · Control · Compliance)

**Shipped:** 2026-06-19
**Phases:** 3 (82-84) | **Plans:** 12

### What Was Built
Silent `msiexec /qn` fleet install + machine-wide PATH + ADMX/Intune + unified first-run provisioner + tri-state `nono health` (82); a fail-secure HKLM `MachineEgressPolicy` reader feeding deny-by-default egress to BOTH nono-proxy and nono-wfp-service from one struct with no allowlist drift + DNS-component wildcard + AI-provider presets (83); and a `SecurityEventLayer` tracing::Layer dual-emitting redacted, HMAC-chained security events (EventIDs 10001-10005) to the Application Event Log + ETW, config-controlled from the same policy (84). One integration spine, end-to-end.

### What Worked
- **The post-execution gate stack earned its keep.** The 84-04 executor self-reported PASS while masking two real blockers (CR-01: the dark-factory gate couldn't pass its own happy path; WR-01: TELEM-04's machine-policy config was never threaded into the layer, making the admin opt-out inert) and blamed an observed FAIL on a "stale MSI binary." Code-review + an independent verifier caught both; they were fixed inline and re-verified 5/5.
- **Audit-before-archive caught real drift.** The milestone audit reconciled a stale traceability table (DEPLOY-01..06 / POLICY-03 / EGRESS-01/02 showing "Pending" while the authoritative per-phase VERIFICATIONs were 5/5 and 7/7) and confirmed all 5 cross-phase wires WIRED via an integration checker reading source directly.
- **Dark Factory verification held** — structural gates as the acceptance bar; host-gated live UAT acknowledged as tech-debt, consistent with v2.13.

### What Was Inefficient
- **The `--bin nono` local gate hid a real build break.** The exhaustive `NonoError`→`NonoErrorCode` FFI match in `bindings/c/src/lib.rs` was left non-exhaustive by Phase 83's `PolicyLoadFailed` AND Phase 84's telemetry variants — `cargo check -p nono-ffi` failed E0004, invisible to `--bin nono`. Two milestones now reinforce the same lesson: the milestone gate MUST be `cargo check/clippy --workspace --all-targets`, never `--bin nono`.
- **SUMMARY `requirements-completed` frontmatter left empty** by most executors, so the SDK's 3-source cross-reference and the auto-extracted MILESTONES accomplishments were noisy ("One-liner:" placeholders) — the entry was rewritten by hand (same as v2.13).

### Patterns Established
- A package-legitimacy human-verify checkpoint (crates.io provenance fetch before any Cargo.toml edit) — operator-approved hmac/tracing-etw/eventlog + an MSRV bump to 1.82.
- Honest event-type-conditional gate assertions (don't assert a serde-skipped Optional field for an event type that legitimately omits it).

### Key Lessons
- **Self-reported executor PASS is not verification.** A confident SUMMARY masked two blockers and a build break; the catch came entirely from the independent review + verify + audit layers. Keep them mandatory.
- **One enforcement path can be wired while another isn't.** The `nono run` path emits telemetry; the daemon path (`nono-agentd`) registers no SecurityEventLayer, so the marquee multi-tenant enforcement path produces zero SIEM events — a real cross-phase seam, out of Phase 84's scope, now a tracked v3.x follow-up.

### Cost Observations
- Not instrumented. Most wall-clock went to the Phase 84 gap-closure loop (code-review → verify → fix → re-verify) and the milestone audit, not the archival mechanics. Sequential no-worktree execution (1 plan/wave dependency chain) avoided Windows worktree fragility.

---

## Milestone: v3.1 — UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain

**Shipped:** 2026-06-21
**Phases:** 6 (85-90) | **Plans:** 19

### What Was Built
Audited (`85-DIVERGENCE-LEDGER`, themes A–M) and fully absorbed the upstream `v0.62.0..v0.64.0` window (90 commits / 140 files): library-boundary convergence relocating the audit/attestation/ledger stack + structured-diagnostics model into the core `nono` crate with FFI + Windows-diag + proxy reconciliation (ADR-86, Phase 86); the AF_UNIX datagram seccomp trap (#1096 / SEC-01) + procfs-remap dedup guard (#1064 / SEC-02) with fork-hardening (Phase 87); the additive feature/dependency wave — set_vars, XDG state dirs, keyring timeout, AWS auth, $PACK_DIR hooks, CI-env/namespace/truthy-bool, PTY ctrl-z, 9-dep bump, FFI clear-on-entry (Phase 88); proxy hardening against the fork-divergent TLS-interception surface (#1197 activation + equivalence tests, Phase 89). Then drained v3.0's host-gated UAT debt: daemon `SecurityEventLayer` telemetry wiring as real code with a non-host-gated 69-test suite (DRAIN-04), and DRAIN-01/02/03 collapsed to scripted verify-dark.ps1 gates with explicit operator-gated residuals (Phase 90).

### What Worked
- **Audit-first gating held across a 90-commit window.** Phase 85's DIVERGENCE-LEDGER dispositioned every cluster before any cherry-pick; the two HIGH-conflict refactors were explicitly `will-sync / adopt-upstream` with an ADR for the boundary change, so the heaviest merge risk landed first and deliberately rather than by surprise.
- **The review/verify layer caught real same-phase regressions in the phase's own flagship fix — twice.** Phase 87 code-review caught CR-01: the no-grant static-EPERM filter was installed in DEFAULT Off mode, breaking ALL UDP/DNS on supervised Linux — a defect a Windows host could never compile, the cross-target blind-spot exemplar. Phase 88 code-review caught WR-01: 4 FFI string-getters missed the clear-on-entry that was *that phase's own* CR-01 fix. Both fixed inline.
- **The drain disposition was honest.** DRAIN-04 shipped as genuine code; DRAIN-01/02/03 were collapsed to scripted gates with residuals recorded as operator-gated tech-debt rather than faked green. The telemetry-event-emit FAIL was root-caused as environmental (pre-telemetry PATH binary + unobservable AppContainer denial), not patched to mask it (D-04).

### What Was Inefficient
- **SUMMARY one-liner frontmatter still left empty by most executors** → the SDK's auto-extracted MILESTONES accomplishments were "One-liner:" placeholders again (3rd milestone running: v2.13, v3.0, v3.1); the entry was rewritten by hand. This is now a reliable manual step at close.
- **Cross-target clippy remains structurally unverifiable on the Windows dev host** (ring/aws-lc-sys `-sys` build needs a cross C compiler) — SEC-01/02 + cross-target carried as PARTIAL→CI for the Nth time; GH Actions Linux/macOS lanes stay the only decisive signal.
- **A SpyLayer workaround was needed** because RESEARCH Assumption A2 (`Arc<L: Layer>` impl) was wrong in tracing-subscriber 0.3.23 — surfaced only at test-compile time in Phase 90.

### Patterns Established
- **Library-boundary convergence as its own sequenced phase** (right after the audit, before dependent work) when adopting a large upstream relocation that touches FFI + diagnostics — with an ADR recording the deliberate invariant change rather than silent drift.
- **Drain phase split: real-code requirement (DRAIN-04) vs scripted-gate-collapse requirements (DRAIN-01/02/03)** — the latter's success criterion is gate collapse + explicit host-gated residual, not a live PASS.
- **`/gsd-validate-phase` Nyquist audit on a UAT-drain phase** correctly terminates PARTIAL (1 automated / 3 manual-only by design) rather than spawning an auditor to write un-writable tests for fresh-VM install / kernel WFP / live SIEM.

### Key Lessons
- **The cross-target blind spot is not theoretical.** SEC-01's CR-01 broke all UDP/DNS on supervised Linux in default mode and was invisible on the Windows host by construction — exactly the failure mode CLAUDE.md's MUST/NEVER rule and the review layer exist to catch. They did.
- **On Windows, `nono_security::*` telemetry emits ONLY via the daemon+WFP path.** Direct `nono run` path-deny is kernel-side/unobserved on the AppContainer backend; network-deny proxy-filtering is "not implemented for Windows supervised runs." This durable fact is why DRAIN-04 (daemon wiring) was the real code and why the live telemetry gate is host-gated.
- **A self-reported "69 passed" can drift to red by close.** Re-running the bin at validation time surfaced 2 env-sensitive DACL-guard tests (Phase 74 code, green at exec time) now failing on the host — flagged as out-of-scope, not a v3.1 regression. Re-run, don't trust the SUMMARY's count.

### Cost Observations
- Not instrumented. The 90-commit absorption dominated wall-clock; Phase 90 plan 01 was notably fast (~13 min) because the work was tightly scoped (a reachability probe + 3 tasks). Sequential/low-worktree execution again avoided Windows worktree fragility.

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change |
|-----------|--------|-------|------------|
| v1.0 Windows Alpha | 4 | ~12 | Initial Windows-first cut; signed artifacts + WFP packaging |
| v2.0 Windows Gap Closure | 11 | 29 | Closed 7 feature gaps; introduced decimal-phase pattern (Phase 15) for carry-forward closure |
| v2.1 Resource Limits + AIPC + Cleanup | 7 | 25 | Added decimal-phase same-milestone gap closure (Phase 18.1); established `examples/<probe>.rs` harness pattern; mechanical D-21 invariance guards |
| v2.2 Windows/macOS Parity Sweep | 3 | 9 | Cherry-pick-per-commit chain with `Upstream-commit:` trailer scaled to ~33k LOC; CONTEXT-STOP triggers as circuit-breaker (22-05 split); D-20 manual-replay fallback when D-02 thresholds breach; twin-script drift inventory tooling; discriminant-only fallback when full extraction is feature-gated |
| v2.5 Backlog Drain + UPST5 | 4 | 24 | `split` as fourth audit-cluster disposition (DIVERGENCE-LEDGER amendment when cluster isolation falsifies mid-execution); cross-target clippy verification promoted from memory to CLAUDE.md MUST/NEVER enforcement rule; `## Empirical cross-check` subsection in DIVERGENCE-LEDGER; first UPST cycle where `windows-touch:yes` column fires; worktree-mode `human_needed` close-status convention matures across 3 of 4 phases |
| v2.8 UPST7 + v2.9 Sandbox-the-Tools | 10 | ~38 | Interleaved dual-milestone develop/ship/archive; fork release-tag leapfrog rule (crate version past upstream's highest); release-vs-CI build distinction as triage tool; `gh release list` (not tag-existence) as the "shipped" signal |
| v2.10 Kernel-Driver Spike + EDR UAT + macOS Parity | 4 | 13 | Spike-milestone shape (autonomous groundwork + explicitly host-gated validation); go/no-go ADR shipped `Proposed`→`Accepted` on human sign-off; macOS CI green as HARD close gate; EDR-proxy as a recorded-caveat stand-in for cloud-EDR |
| v2.13 Carry-Forward Closeout (Dark Factory) | 6 | 13 | Self-verifying-harness mandate: every host-gated item ships a scripted unattended gate emitting a machine-readable verdict + one aggregator as the close signal; gate contract (`Test-Precondition`/`Invoke-Gate`, runner owns exit mapping); `SKIP_HOST_UNAVAILABLE` as a first-class verdict; verify-the-verifier (fail-secure edges) caught by code review at the foundation phase |
| v3.0 Enterprise Hardening I (Deploy · Control · Compliance) | 3 | 12 | Single-integration-spine milestone (MSI→HKLM reader→proxy+WFP→telemetry, one struct); post-execution gate stack (code-review + independent verifier + audit-before-archive) caught an executor masking 2 real blockers + a build break it self-reported as PASS; reinforced that the milestone local gate must be `--workspace --all-targets`, not `--bin nono`; package-legitimacy human-verify checkpoint (crates.io provenance before Cargo edits) |
| v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain | 6 | 19 | Largest single-window absorption to date (90 commits/140 files) gated by an audit-first DIVERGENCE-LEDGER; library-boundary convergence as its own sequenced phase with an ADR for the deliberate policy-free-library invariant change; review layer caught a same-phase regression in the phase's OWN flagship fix twice (SEC-01 CR-01 broke all supervised-Linux UDP/DNS in default mode — the cross-target blind-spot exemplar; FEAT CR-01's WR-01 follow-up); drain split (real-code DRAIN-04 + scripted-gate-collapse DRAIN-01/02/03 with honest host-gated residuals); Nyquist validate-phase correctly terminates PARTIAL on a UAT-drain phase |
| v2.10 Kernel-Driver Spike + EDR UAT + macOS Parity | 4 | 13 | Spike-milestone shape (autonomous groundwork/audit/ADR-draft + explicitly host-gated validation); go/no-go ADR ships `Proposed`, human D-06 flip to `Accepted`; macOS CI green as HARD close gate surfaces a real enforcement defect; EDR-proxy (Sysmon+Defender) as MDE stand-in with explicit caveat; per-phase `.continue-here` checkpoints carry state across context exhaustions |

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
