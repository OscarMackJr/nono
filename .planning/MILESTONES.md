# Milestones

## v2.2 — Windows/macOS Parity Sweep

**Status:** ✅ SHIPPED 2026-04-29
**Started:** 2026-04-24
**Shipped:** 2026-04-29
**Branch:** `windows-squash` (continuing from v2.1; merge-to-main pending per quick-260428-rsu)

**Goal:** When v2.2 ships, a Windows user and a macOS user have the same `nono` commands available with the same flags and the same security guarantees. Close the Windows-vs-macOS drift opened by upstream `always-further/nono` shipping v0.38.0–v0.40.1 without Windows ports, and install drift-prevention tooling so v0.41+ becomes a maintenance task instead of a milestone-scale sync.

**Phases:** 3 phases (Phases 22–24).
**Plans shipped:** 9 plans (22 → 6 plans including the 22-05a/b split; 23 → 1 plan; 24 → 2 plans).
**Requirements:** 21 — PROF-01..04, POLY-01..03, PKG-01..04, OAUTH-01..03, AUD-01..05, DRIFT-01..02. Closed: 19 fully + 2 complete-partial (PKG-01 has 2 streaming cherry-picks deferred; AUD-03 has Authenticode chain-walker subject extraction deferred).

**Stats:**
- 146 commits since `v2.1` tag (29 `feat(...)` commits across phases 22/23/24).
- 154 files changed; +33,153 / −835 LOC across code + docs + planning artifacts.
- ~+8.4k LOC of Rust code (53 source files in `crates/`).

**Key accomplishments:**
- **Profile struct alignment (PROF-01..04)** — `unsafe_macos_seatbelt_rules`, `packs`, `command_args`, `custom_credentials.oauth2` deserialize on Windows; `claude-no-keychain` builtin profile shipped (Phase 22 Plan 22-01, 12 commits, `d7fc4ed8`).
- **Policy tightening (POLY-01..03)** — orphan `override_deny` fails closed at profile load; `--rollback`/`--no-audit` clap-level mutex with CL-01-M `--no-audit-integrity` carve-out preserved; `.claude.lock` moved to `allow_file` for both `claude-code` and `claude-no-kc` (Phase 22 Plan 22-02, 7 commits, `490a8a5c`).
- **Package manager (PKG-01..04, partial)** — `nono pull/remove/update/search/list` flat-shape subcommand tree with Windows `%LOCALAPPDATA%\nono\packages\<name>` storage, Claude-Code hook registration via fork's `hooks.rs`, signed-artifact verification at install time (Phase 22 Plan 22-03; 6/8 cherry-picks landed, 2 deferred to v2.3 backlog).
- **OAuth2 proxy + reverse-proxy gating (OAUTH-01..03)** — `nono-proxy/src/oauth2.rs` client-credentials Bearer-token injection; reverse-proxy HTTP upstream restricted to loopback-only by default with `--allow-domain` strict-proxy composition; CL-03-M literal `client_secret` warning + CL-04-M manifest-export skip + HG-01-M Debug redaction (Phase 22 Plan 22-04, 14 commits, `5c8df06a`).
- **Audit integrity + DSSE attestation (AUD-01, AUD-02, AUD-03 SHA-256, AUD-04)** — hash-chained Merkle-rooted ledger; cryptographic DSSE bundle verification (HG-01-H upgrade, commit `cffb43b1`); `prune` → `session cleanup` rename with formal `applied_labels_guard::audit_flush_before_drop` regression test (83 LOC) guaranteeing v2.1 CLEAN-04 byte-identical preservation; hidden `nono prune` deprecation alias; `nono audit cleanup` peer subcommand (Phase 22 Plans 22-05a + 22-05b after CONTEXT-STOP-3 split, `d15a3ab6` + `b5640cd4`).
- **Windows Authenticode exec-identity discriminant (AUD-03 Windows portion)** — `WinVerifyTrust` records `Valid` / `Unsigned` / `InvalidSignature{hresult}`; chain-walker subject extraction deferred to v2.3 pending `Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip` features in `windows-sys` (Phase 22 Plan 22-05b, commit `cb34a82a`).
- **Windows AIPC ledger emissions (AUD-05)** — `RejectStage` enum (`BeforePrompt | AfterPrompt`) on `AuditEventPayload::CapabilityDecision` locks the WR-01 verdict-matrix asymmetry on the wire; `handle_windows_supervisor_message` emits `capability_decision` events at all 5 push sites (File + 5 AIPC HandleKinds); `nono audit show <id>` surfaces a `Capability Decisions: N (M before-prompt, K after-prompt rejections)` counter + `capability_decisions` JSON array (Phase 23 Plan 23-01, 3 commits `427e1283` / `a9307802` / `263795a9`, 60 tests passing).
- **Parity-drift prevention (DRIFT-01, DRIFT-02)** — `make check-upstream-drift` twin scripts with `$(OS)==Windows_NT` Makefile dispatch + 6-category path-prefix lookup + 3 frozen golden JSON fixtures; GSD upstream-sync template at `.planning/templates/upstream-sync-quick.md` with byte-exact 6-line D-19 trailer; Mintlify long-form runbook at `docs/cli/development/upstream-drift.mdx`; PROJECT.md `## Upstream Parity Process` cross-link (Phase 24, 2026-04-27).

**Plan splits & deviations:**
- **Plan 22-05 → 22-05a + 22-05b** at CONTEXT STOP trigger #3 on upstream cherry-pick `4f9552ec`. T-22-05-04 ABSOLUTE STOP guard required CLEAN-04 invariants byte-identical AFTER every source-code commit; the split honored that gate and installed a permanent regression test as a future-regression guard.
- **Phase 23 layer-2 deviation** authorized by plan Step 7 — `aipc_handle_brokering_integration` cannot reach `pub(super)` `handle_windows_supervisor_message`; layer-1 multi-kind E2E in `capability_handler_tests` (`audit_integrity_records_5_handle_kinds_in_ledger`) provides the substitute coverage per the plan's authorized fallback clause.

**Known deferred items at close:** 20 (6 UAT bookkeeping gaps, 4 verification human_needed flags, 10 stale or pending quick-task index pointers including the 260428-rsu re-deferral pending PR-583 maintainer response). See STATE.md `## Deferred Items` for the full table. None block release.

**Deferred to v2.3 backlog:**
- PKG streaming follow-up (`58b5a24e` + `9ebad89a` + `115b5cfa` + `ArtifactType::Plugin` + `bundle_json` field).
- Audit-attestation hardening sweep (sigstore-rs `KeyPair::from_pkcs8` re-enablement; 2 `#[ignore]`'d fixture-driven tests).
- Authenticode chain-walker subject extraction (`Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip` features).
- WR-01 reject-stage unification.
- AIPC G-04 wire-protocol compile-time tightening.
- Cross-platform RESL Unix backends.

**Deferred to v3.0:** WR-02 EDR HUMAN-UAT item.

**Archive files:**
- `.planning/milestones/v2.2-ROADMAP.md`
- `.planning/milestones/v2.2-REQUIREMENTS.md`

Git tag: `v2.2`.

---

## v2.1 — Resource Limits, Extended IPC, Attach-Streaming & Cleanup

**Status:** ✅ SHIPPED 2026-04-21
**Started:** 2026-04-18
**Shipped:** 2026-04-21
**Branch:** `windows-squash` (continuing from v2.0 + Phase 15)

**Goal:** Deliver Job Object resource limits (CPU / memory / timeout / process-count), extend the Phase 11 capability pipe to broker additional handle types end-to-end, land attach-streaming on detached Windows sessions, sync to upstream v0.37.1 (including the rustls-webpki security upgrade), enable single-file filesystem grants on Windows so the `claude-code` profile runs cleanly, and clean up v2.0 WIP.

**Phases:** 7 phases (Phases 16–21 plus decimal Phase 18.1).
**Plans shipped:** 25 plans.
**Requirements:** 13 — RESL-01..04, AIPC-01, ATCH-01, CLEAN-01..04, UPST-01..04, WSFG-01..03.

**Key accomplishments:**
- Job Object resource limits — CPU/memory/timeout/process-count caps with kernel enforcement (Phase 16).
- `nono attach` on detached Windows sessions now streams child stdout live via anonymous-pipe stdio; friendly multi-attach error (Phase 17).
- AIPC handle brokering for Socket / Pipe / Job Object / Event / Mutex over the Phase 11 capability pipe + `capabilities.aipc` profile-widening schema + containment-Job runtime guard (Phases 18 + 18.1).
- 5 HUMAN-UAT gaps (G-02..G-06) closed in Phase 18.1 with live dual-run widening proof on rebuilt binary.
- Cleanup workstream — fmt drift, 4 Windows test flakes (incl. UNC-prefix `query_path` production bug), 10 WIP items triaged, `is_prunable` + `nono prune --older-than`/`--all-exited` + auto-sweep on `nono ps`, 1343-file one-shot prune on dev host (Phase 19).
- Upstream parity sync to v0.37.1 — rustls-webpki 0.103.12 security upgrade (RUSTSEC-2026-0098/0099), `keyring://` URIs, env-var filtering, `--allow-gpu` with NVIDIA Linux allowlist, GitLab ID tokens for trust signing (Phase 20).
- Windows single-file filesystem grants via per-file Low-IL mandatory-label ACEs + `AppliedLabelsGuard` RAII lifecycle + ownership-skip pre-check; unblocks `claude-code` profile's `git_config` group on Windows (Phase 21).

**Notable in-flight finding:** Windows 11 26200's `WRITE_RESTRICTED` tokens require BOTH a restricting-SID ACE AND a logon-SID ACE in the pipe DACL for the second-pass access check to pass — MSDN-undocumented; discovered via 13-variant systematic SDDL iteration in `crates/nono-cli/examples/pipe-repro.rs`. Fix in commit `938887f`.

**Known deferred items at close:** 17 (5 UAT bookkeeping gaps, 3 verification human_needed flags, 9 stale quick-task index pointers to already-removed directories). See STATE.md `## Deferred Items` for the full table. None block release.

**Archive files:**
- `.planning/milestones/v2.1-ROADMAP.md`
- `.planning/milestones/v2.1-REQUIREMENTS.md`

Git tag: `v2.1`.

---

## v2.0 — Windows Gap Closure (a.k.a. "Windows Parity")

**Status:** ✅ SHIPPED 2026-04-18 (with v2.0-known-issue carry-forward to Phase 15)
**Started:** 2026-04-06
**Shipped:** 2026-04-18
**Branch:** `windows-squash` (committed; push/merge-to-main pending per user)

**Goal:** Close the 7 remaining feature gaps between Windows and Unix platforms — `nono wrap`, session log commands, interactive ConPTY shell, port-granular WFP policy, proxy credential injection, ETW-based learn, and runtime capability expansion (stretch) — so everyday CLI usage reaches cross-platform parity.

**Phases:** 10 phases (Phases 5–14; Phase 15 created as v2.1 follow-up for the carry-forward).
**Plans shipped:** 28 firm plans. Plan 14-01 escalated to Phase 15.

**Key accomplishments:**
- WFP promoted to primary enforced network backend with SID-based filtering (Phase 06).
- `nono wrap` on Windows with Direct strategy + help-text correction (Phases 07, 14-02).
- Interactive `nono shell` via ConPTY on Windows 10 build 17763+ (Phase 08).
- Port-granular WFP policy + proxy credential injection (Phase 09).
- `nono learn` on Windows via ETW with Win32-format paths (Phase 10).
- Runtime capability expansion over named pipe with constant-time token auth (Phase 11).
- Human Verification UAT resolved with terminal verdicts on all 10 items (Phase 13).

**Known deferred items at close:**
- Detached-supervisor + ConPTY + restricted-token `0xC0000142 STATUS_DLL_INIT_FAILED` on sandboxed console grandchildren. Carried forward to Phase 15 per explicit user shipping decision.
- Affected UAT legs waived as `v2.0-known-issue`: P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3.
- P09-HV-1 live end-to-end waived as `no-test-fixture` (no built-in network-profile-with-credentials ships out of the box).

**Archive files:**
- `.planning/milestones/v2.0-ROADMAP.md`
- `.planning/milestones/v2.0-REQUIREMENTS.md`
- `.planning/milestones/v2.0-MILESTONE-AUDIT.md`

Git tag: `v2.0` (see `git show v2.0` for tagger signature).

---

## v1.0 — Windows Alpha (shipped 2026-03-31)

**Status:** ✅ SHIPPED 2026-03-31
**Git tag:** `v1.0`

**Delivered:** Windows is a first-class nono release target with signed artifacts, WFP service packaging, and no preview language anywhere.

**Key accomplishments:**
- Authenticode signing pipeline (sign-windows-artifacts.ps1 + release.yml gate).
- WFP service packaging via WiX 4 ServiceInstall/ServiceControl in machine MSI.
- All preview language removed from runtime, docs, CI, and README.
- Formal Windows promotion criteria (21 gates, all checked).
- Supervisor parity (attach, detach, ps, stop) — Phases 1–2.
- Snapshot/rollback for Windows filesystems — Phase 4.
- MSI packaging and code signing automation — Phase 4.

**Phases:** 4 (Phases 1–4). Requirements: SUPV-01..05, NETW-01..03, STAT-01..02, DEPL-01..02 (12 total).

(An earlier draft of this entry referred to this milestone as "v1.0 — WIN-1706 Option 1: Windows Library/Runtime Alignment" and was never properly closed; the real shipped content is what the `v1.0` git tag points at from 2026-03-31. That earlier draft is superseded by this entry.)
