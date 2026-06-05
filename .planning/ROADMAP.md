---
milestone: v2.8
milestone_name: UPST7 + v2.7 Drain & Release
status: active
created: 2026-05-28
granularity: standard
---

# Roadmap — nono

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 01-12 (shipped 2026-03-31) — see [`milestones/v1.0-*`](milestones/)
- ✅ **v2.0 Windows Gap Closure** — Phases 13-18 — see [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md)
- ✅ **v2.1 Resource Limits / Extended IPC / Attach-Streaming** — see [`milestones/v2.1-ROADMAP.md`](milestones/v2.1-ROADMAP.md)
- ✅ **v2.2 Windows/macOS Parity Sweep** — see [`milestones/v2.2-ROADMAP.md`](milestones/v2.2-ROADMAP.md)
- ✅ **v2.3 Linux POC Unblock + Deferreds Closure** — see [`milestones/v2.3-ROADMAP.md`](milestones/v2.3-ROADMAP.md)
- ✅ **v2.4 Complete the Partial Ports + UPST4** — Phases 35, 36, 36.5, 39, 40 (shipped 2026-05-15) — see [`milestones/v2.4-ROADMAP.md`](milestones/v2.4-ROADMAP.md)
- ✅ **v2.5 Backlog Drain + UPST5** — Phases 37, 41, 42, 43 (shipped 2026-05-20) — see [`milestones/v2.5-ROADMAP.md`](milestones/v2.5-ROADMAP.md)
- ✅ **v2.6 UPST6 + v2.5 Drain** — Phases 44, 44.1, 45, 46, 47, 48, 49, 50 (shipped 2026-05-25) — see [`milestones/v2.6-ROADMAP.md`](milestones/v2.6-ROADMAP.md)
- ✅ **v2.7 Windows supervised-run hardening** — Phases 51, 52 (shipped 2026-05-26) — see [`milestones/v2.7-ROADMAP.md`](milestones/v2.7-ROADMAP.md)
- **v2.8 UPST7 + v2.7 Drain & Release** — Phases 53–59 (active)
- **v2.9 Windows Sandbox-the-Tools — Confined Coding Loop** — Phases 60, 61, 62 (Phase 60 complete; Phase 61 = ship/release; Phase 62 = out-of-box WFP network enforcement closing Phase 60's F-60-UAT-03; separate initiative from UPST7, builds on merged PR #4)

## Phases

- [x] **Phase 53: Release & Drain** — Tag v2.8 + v0.57.5, produce signed MSIs off the post-`005b4c9e` binary, verify release.yml, UAT WFP uninstall, drain 3 todos (shipped as v0.57.5 after a release.yml signing-order fix; all 5 SC verified)
- [x] **Phase 54: UPST7 Audit** — Produce DIVERGENCE-LEDGER for upstream `v0.57.0..v0.59.0`; per-cluster dispositions + ADR re-confirm + re-export empirical cross-check
- [ ] **Phase 55: UPST7 Cherry-pick Wave** — Absorb cross-platform straight ports (JSONC, target_binary, opencode relocation, timeout constants, proxy 502 hardening, pack-update-hint robustness, ENV_LOCK policy test, sigstore 0.8.0, denial/diagnostic polish) per Phase 54 dispositions (`java-dev`/`java_runtime`: 0 commits in v0.57.0..v0.59.0 per ledger empirical cross-check on platform.rs; UPST8 territory)
- [ ] **Phase 56: Fine-grained Network Filtering** — `allow_domain` URL path + HTTP method restrictions in nono-proxy; TLS-intercept endpoint-rules-before-credential-selection ordering fix
- [ ] **Phase 57: Bitwarden Credential Source** — `bw://` keystore backend alongside `keyring://`/`env://`/`file://`; `Zeroizing<String>` secret posture
- [ ] **Phase 58: Session Lifecycle Hooks** — `session_hooks` profile field; Unix upstream behavior preserved; Windows broker-spawned Low-IL execution design + ADR; fail-closed on hook failure
- [ ] **Phase 59: Supervisor IPC Robustness** — Keep-alive on transient child IPC close, bounded read-timeouts, robust accept; Unix named-socket hardening absorbed cross-platform-core; Windows Named-Pipe AIPC path translated (not cherry-picked)

### v2.9 — Windows Sandbox-the-Tools (separate track)

- [x] **Phase 60: Confined Coding Loop** — Make the merged PR #4 tool-mediation slice a usable coding agent for Windows POC users: confined Low-IL **file edits** (Write/Edit/MultiEdit/NotebookEdit) via per-call capability mapping instead of deny, plus a usable shell story (PowerShell-runner decision). Network/WebFetch/MCP/Task stay denied (out of POC scope). Input: `.planning/quick/260528-sch-spec-the-sandbox-the-tools-windows-tool-/260528-sch-SPEC.md` (§7 answered)
 (completed 2026-05-29)
- [ ] **Phase 61: Ship/Release v2.9** — Package and release the Phase 60 confined-coding-loop POC **and** Phase 62 WFP kernel network enforcement as a CI-signed public release: lockstep-bump the workspace to 0.58.0, dual-tag `v2.9`+`v0.58.0` off current `main`, produce CI-signed machine+user MSIs via `release.yml` (wrapper AND embedded payloads Authenticode-valid), and write release notes (honest POC / defense-in-depth framing for both features)
- [x] **Phase 62: Add WFP kernel network enforcement for Windows supervised runs** — Make `network.block:true` on a supervised `nono run` enforce WFP kernel filtering out of the box (machine MSI `start=auto`, in-run auto-start-or-fail-closed, non-elevated pipe SDDL); closes Phase 60's F-60-UAT-03. Service-only — no new kernel driver. (REQ-WFP-01, v2.9 track)
 (completed 2026-06-03)

## Phase Details

### Phase 53: Release & Drain
**Goal**: The post-v2.7 fixes ship as a real signed release and the carry-forward debt is cleared
**Depends on**: Nothing (first phase; drain work is independent of UPST7)
**Requirements**: REQ-RLS-01, REQ-RLS-02, REQ-DRN-01, REQ-DRN-02
**Success Criteria** (what must be TRUE):
  1. A v2.8 git tag exists; signed MSIs (machine + user) are produced off the post-`005b4c9e` `nono.exe` and the installed binary reports the v2.8 fork version (HUMAN-UAT: operator installs signed MSI, runs `nono --version`, confirms v2.8)
  2. `nono run --profile claude-code -- <binary> --version` on the real-console no-PTY supervised path exits 0 and prints the version — the doubly-broken path from the v2.7 tag (`d8b7ce00` + `005b4c9e` fixes) is confirmed working in the released binary
  3. `.github/workflows/release.yml` completes without `startup_failure` on a live `v*` tag push and produces signed release artifacts (HUMAN-UAT: push a `v2.8` tag, confirm GitHub Actions run to completion)
  4. An operator running elevated `sc stop` on the WFP service then `msiexec /x` confirms the service stops cleanly and uninstall removes the service/driver leaving nothing behind (HUMAN-UAT: requires elevated Windows host; closes `wfp-service-stop-uninstall` debug's remaining leg)
  5. The 3 pending todos in `.planning/todos/pending/` are resolved or explicitly re-dispositioned with a written rationale committed to the planning artifacts
**Plans**: 4 plans
Plans:
**Wave 1**
- [x] 53-01-PLAN.md — Version bump 0.57.3 to 0.57.4 across all 5 crate Cargo.toml files and path-dep pins
- [x] 53-02-PLAN.md — Drain: promote Todos 2+3 to REQUIREMENTS.md backlog with D-53-08 rationale; move pending files to done/

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 53-03-PLAN.md — Fix release.yml trigger (v*.*.*), update sign-poc-local.ps1, expand signing guide (CA-ready + fresh-cert procedure), push main

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 53-04-PLAN.md — HUMAN-UAT: CI run verify (REQ-RLS-02 PASS), signed MSI install verify (REQ-RLS-01 — failed on v0.57.4 unsigned payload, fixed release.yml + re-released v0.57.5, re-UAT PASS), elevated WFP stop/uninstall verify (REQ-DRN-01 PASS), no-PTY path verify on v0.57.5

### Phase 54: UPST7 Audit
**Goal**: The fork has a complete DIVERGENCE-LEDGER for upstream `v0.57.0..v0.59.0` with actionable dispositions for every cluster and a confirmed strategy for the cherry-pick wave
**Depends on**: Phase 53 (release ships first; audit may proceed concurrently but must not block drain)
**Requirements**: REQ-UPST7-01
**Success Criteria** (what must be TRUE):
  1. A `DIVERGENCE-LEDGER.md` exists in the Phase 54 directory covering upstream `v0.57.0..v0.59.0` with per-cluster dispositions (will-sync / fork-preserve / won't-sync / split), a `windows-touch` column, and an `## ADR review` section that confirms or revises Phase 33 Option A `continue`
  2. An `## Empirical cross-check` section verifies re-export surface isolation on fork-shared files via diff-inspect (not just `--name-only`), per the `feedback_cluster_isolation_invalid` lesson from Phase 43
  3. Upstream was re-fetched at audit-open, capturing any `v0.59.x` patch releases cut after 2026-05-27; the ledger frontmatter records the upstream HEAD SHA and date of the re-fetch
  4. The fork-divergent TLS-interception surface (Phase 34 C11 `fork-preserve`) is explicitly addressed with a diff-inspect note flagging whether the v0.59 TLS-intercept ordering fix applies cleanly or requires manual replay
**Plans**: 1 plan
Plans:
- [x] 54-01-UPST7-AUDIT-PLAN.md — DIVERGENCE-LEDGER for v0.57.0..v0.59.0: re-fetch + drift run, per-cluster dispositions + windows-touch, ADR review, empirical cross-check, SC4 TLS-intercept assessment, UPST8 stub (complete 2026-06-04; 40 commits / 14 clusters)

### Phase 55: UPST7 Cherry-pick Wave
**Goal**: The cross-platform straight-port clusters from the UPST7 audit are absorbed into the fork with correct D-19 trailers and the fork's invariants intact
**Depends on**: Phase 54 (dispositions are the input)
**Requirements**: REQ-UPST7-02
**Success Criteria** (what must be TRUE):
  1. Every `will-sync` cluster disposition from Phase 54 is executed: JSONC profile parsing, `target_binary` profile field, `opencode` pack relocation, configurable timeout constants, proxy 502 hardening, pack-update-hint robustness (atomic writes + detached refresh), ENV_LOCK policy test serialization, sigstore 0.8.0 dep bump (Cargo + scrub.rs verify-then-port), suppressed-denial annotations, canonical denial-path precompute, access-mode `rfind` split, and overflow-check tightening are all present in the fork tree. (`java-dev`/`java_runtime` has 0 commits in v0.57.0..v0.59.0 per Phase 54 empirical cross-check -- removed from SC1; UPST8 territory.)
  2. Each absorbed upstream commit carries a verbatim lowercase 6-line `Upstream-commit:` trailer (D-19 convention) or `Upstream-replayed-from:` for D-20 replays; the D-43-E1 Windows-only-files invariant is respected
  3. Schema-collision checks confirm no canonical-section conflicts between absorbed upstream profile schema changes and the fork's `nono-profile.schema.json` / `policy.json` canonical sections
  4. `make ci` (or the Windows equivalent `cargo test --workspace`) passes with zero new test failures relative to the Phase 54 baseline SHA
**Plans**: 7 plans
Plans:
**Wave 1** (parallel)
- [x] 55-01-PLANNING-AMEND-PROXY502-PLAN.md -- D-55-01 REQ/SC amendment (planning docs) + C4 proxy 502 hardening (d11193f + 4ad708d)

**Wave 2** *(blocked on Wave 1)*
- [x] 55-02-PROFILE-JSONC-TARGET-BINARY-PLAN.md -- C7 profile system: JSONC parsing + target_binary + opencode extraction + refactors (5 commits) + SC3 schema-collision check

**Wave 3** *(blocked on Wave 2 -- parallel pair)*
- [x] 55-03-PACK-HINT-ROBUSTNESS-PLAN.md -- C9 pack-update-hint: detached-process refresh + atomic state writes (74fbbf12 + b1a650a3)
- [x] 55-04-DIAGNOSTIC-DENIAL-POLISH-PLAN.md -- C10 diagnostic/denial polish: suppressed-denial annotations + canonical-path precompute + rfind split + bold-path footer (4 commits)

**Wave 4** *(blocked on Wave 3 -- parallel pair)*
- [ ] 55-05-TIMEOUT-CONSTANTS-PLAN.md -- C11 timeout constants: centralize timeouts.rs + overflow-check tightening (3 commits)
- [ ] 55-06-POLICY-ENV-LOCK-TEST-PLAN.md -- C12 policy test: ENV_LOCK serialization in test_all_groups_no_deny_within_allow_overlap (1a764d05)

**Wave 5** *(blocked on Waves 2+4)*
- [ ] 55-07-SIGSTORE-BUMP-PLAN.md -- C13 sigstore 0.8.0 split: diff-inspection-first + Cargo bump + scrub.rs verify-then-port (e581569)

### Phase 56: Fine-grained Network Filtering
**Goal**: Operators can scope `--allow-domain` entries to specific URL paths and HTTP methods, with TLS-intercept endpoint rules correctly evaluated before credential selection
**Depends on**: Phase 54 (diff-inspect note on TLS-interception surface required before implementation)
**Requirements**: REQ-NET-01
**Success Criteria** (what must be TRUE):
  1. `--allow-domain https://api.example.com/v1 --method GET` (or equivalent profile field) restricts proxy access to the specified path prefix and HTTP method; a sandboxed child attempting a disallowed path or method receives a proxy denial, not silent pass-through
  2. TLS-intercept endpoint rules are evaluated before credential selection: a request matching an endpoint-rule deny is rejected before credentials are injected (verifiable via proxy trace log or audit entry)
  3. `nono why --host api.example.com` surfaces path/method scoping rules in its output when the domain has path-scoped entries
  4. The Phase 34 C11 `fork-preserve` TLS-interception surface is preserved; the diff-inspect note from Phase 54 documents exactly which upstream v0.59 changes were applied as cherry-picks vs manual replays vs intentionally skipped
**Plans**: TBD
**UI hint**: yes

### Phase 57: Bitwarden Credential Source
**Goal**: Operators can load credentials from Bitwarden via `bw://` URIs alongside the existing keystore backends
**Depends on**: Phase 55 (cherry-pick wave may touch keystore surface; absorb first)
**Requirements**: REQ-CRED-01
**Success Criteria** (what must be TRUE):
  1. A `bw://` URI in a profile or `--credential` argument resolves a secret from Bitwarden (via the `bw` CLI or Bitwarden API) and makes it available to the sandboxed child without exposing the raw secret in any log, audit entry, or process argument list
  2. Secret fields are held in `Zeroizing<String>` and cleared on drop; the implementation satisfies `cargo clippy -D clippy::unwrap_used` with no exceptions
  3. `bw://` behaves identically to `keyring://`/`env://`/`file://` at the keystore abstraction boundary: the same `--credential` flag accepts all four schemes cross-platform with no platform-specific code paths above the keystore layer
**Plans**: TBD

### Phase 58: Session Lifecycle Hooks
**Goal**: Profiles can declare hooks that run at session start and stop, with Unix behavior preserved from upstream and Windows executing via a safe broker-spawned design
**Depends on**: Phase 55 (cherry-pick wave absorbs the `session_hooks` schema and any upstream cross-platform-core portions first)
**Requirements**: REQ-HOOK-01
**Success Criteria** (what must be TRUE):
  1. A profile with a `session_hooks` field runs the declared hooks at session start and stop on both Unix and Windows; hook output is visible in session logs
  2. On Unix, the upstream `hook_runtime` behavior is preserved exactly (gated unix-only as upstream ships it); no behavioral regression from the upstream implementation
  3. On Windows, hooks execute via a broker-spawned Low-IL process (no `fork`/`sh` assumption); an ADR is committed to `.planning/` documenting the Windows execution design decisions and any invariants the hook executor must preserve (e.g., mandatory-label enforcement, no unrestricted shell access)
  4. Hook resolution or execution failure is fail-closed: if a required hook cannot be found or exits non-zero, the session does not start (or stops with an error) — never silently skipped
**Plans**: TBD
**UI hint**: yes

### Phase 59: Supervisor IPC Robustness
**Goal**: The supervisor loop survives transient child IPC disconnects and enforces bounded read timeouts on both Unix and Windows
**Depends on**: Phase 55 (cherry-pick wave may touch supervisor IPC cross-platform-core portions)
**Requirements**: REQ-IPC-01
**Success Criteria** (what must be TRUE):
  1. A sandboxed child that closes its IPC connection and reconnects does not cause the supervisor to exit; the supervisor loop keeps alive and reaccepts the connection (verifiable via integration test or supervised-run repro)
  2. IPC read operations on the supervisor side enforce a bounded timeout (configurable or a documented constant); a child that holds an open connection without sending data does not block the supervisor indefinitely
  3. On Unix, the upstream named-socket hardening intent is absorbed (cross-platform-core portions cherry-picked per Phase 54 dispositions) with correct D-19 trailers
  4. On Windows, the Named-Pipe AIPC path (Phase 18) receives the equivalent robustness treatment (keep-alive, bounded timeouts, robust accept); implementation is documented as a translate-not-cherry-pick with the translation rationale in the plan SUMMARY
**Plans**: TBD

### Phase 60: Sandbox-the-Tools — Confined Coding Loop (v2.9)
**Goal**: A Windows POC user runs the Claude Code TUI at Medium IL and the agent completes a full coding loop — read, run commands, **and edit files** — with every side-effecting operation confined to a Low-IL `nono` jail. File edits work (confined) instead of being denied.
**Depends on**: PR #4 (merged `7488dbba` — the matcher/deny-by-default/runner-profile/self-disable foundation). NOT dependent on the v2.8 UPST7 phases.
**Input**: `.planning/quick/260528-sch-spec-the-sandbox-the-tools-windows-tool-/260528-sch-SPEC.md` (§7 answered, informed by the PR #4 review). Promote per SPEC Q6.
**Requirements**: REQ-STW-01 (confined file ops), REQ-STW-02 (usable shell story)
**Success Criteria** (what must be TRUE):
  1. **Confined file edits.** `Write`/`Edit`/`MultiEdit`/`NotebookEdit` tool calls execute as Low-IL `nono`-confined file operations scoped to the target path (per-call capability mapping derived from `tool_input`, SPEC Q3) — instead of being denied. HUMAN-UAT: a POC user has Claude edit a file in the project and the edit lands; a write outside the granted scope is denied at the OS boundary.
  2. **Usable shell story.** Bash tool calls either are presented honestly as a PowerShell tool runner with Claude steered to emit PowerShell (profile tool-description / system-prompt note), or a confined real-shell path is provided — typical run-command requests succeed without manually prompting "use PowerShell syntax" (SPEC Next-Slice #3).
  3. **Confinement invariants preserved.** The PR #4 self-disable CWD guard and deny-by-default for unwrappable surfaces remain intact; the per-call Write grant must not re-introduce a write path to `~/.claude` hook state (consistent with the merged guard + todo `2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`). Defense-in-depth labeling stays accurate.
  4. **End-to-end POC UAT.** A POC user completes a small read → edit → run task on a Win11 host with the experimental profile; edits are confined, an out-of-scope write is denied.
**Out of scope (this phase)**: network/`allow_domain` per-call grants, `WebFetch`/`WebSearch`, MCP-under-`nono`, `Task`/subagents (all remain denied); agent-process FS confinement (the agent stays Medium-IL with unconfined reads — documented, SPEC Q5).
**Plans**: 2 plans
Plans:
**Wave 1** (parallel — no file overlap)
- [x] 60-01-PLAN.md — Confined file-op arms: Write/Edit/MultiEdit deny+additionalContext + NotebookEdit informative deny + unit tests (REQ-STW-01)
- [x] 60-02-PLAN.md — PowerShell-steering CLAUDE.md update + runner profile verification + cross-target clippy PARTIAL note (REQ-STW-02)

### Phase 61: Ship/Release v2.9
**Goal**: Package and release the Phase 60 confined-coding-loop POC + Phase 62 WFP enforcement — lockstep-bump the workspace to 0.58.0, verify the D-09 deny-`~/.claude` hook guard, dual-tag `v2.9`+`v0.58.0` off current `main`, produce CI-signed machine+user MSIs via `release.yml`, and write release notes for the Windows confined tool-mediation + out-of-box WFP enforcement story (honest POC / defense-in-depth framing).
**Depends on**: Phase 60 (confined coding loop is code-complete + live-UAT PASS) + Phase 62 (WFP enforcement complete).
**Requirements**: REQ-RLS-03, REQ-RLS-04
**Plans**: 4 plans
Plans:
**Wave 1** (parallel — no file overlap)
- [x] 61-01-PLAN.md — Lockstep 0.58.0 version bump (5 crates + 6 path-dep pins + Cargo.lock) + CHANGELOG [0.58.0] + register REQ-RLS-03/04 (D-03, D-04)
- [x] 61-02-PLAN.md — D-09 verify-and-document: run the hook CWD-guard tests, write the honest hook-layer scope/limitation note, resolve the deny-`~/.claude` todo (D-09)

**Wave 2** *(blocked on 61-01)*
- [x] 61-03-PLAN.md — Pre-tag readiness: drain-fix ancestry (D-08), v0.57.4 verify-absent/delete-if-found (D-07), signing-secret pre-flight (D-02), driver-sys presence

**Wave 3** *(operator-gated; blocked on 61-01/02/03)*
- [ ] 61-04-PLAN.md — Release notes + tag/push v0.58.0 + release.yml live-verify + v2.9 annotation tag + post-release MSI signature spot-check (D-01, D-02, D-03, D-05, D-06)

### Phase 62: Add WFP kernel network enforcement for Windows supervised runs
**Goal**: Make `network.block:true` on a supervised/broker (Low-IL) `nono run` reliably enforce WFP kernel network filtering out of the box on a machine-MSI-installed host — with no manual `nono setup --start-wfp-service` step — and never silently pass through unenforced. Closes Phase 60's F-60-UAT-03 carry-forward. (WFP-via-service is already kernel-enforced; the deliverable is operational reliability, not a new kernel layer — `nono-wfp-driver.sys` minifilter stays v3.0-deferred.)
**Depends on**: Phase 61
**Requirements**: REQ-WFP-01
**Success Criteria** (what must be TRUE):
  1. **Out-of-box enforced block (HUMAN-UAT, real elevated Win11 host).** After installing the machine MSI, a non-elevated supervised `nono run` on the runner profile with `network.block:true` denies the confined child's outbound network while any explicitly-allowed ports still pass — with NO prior `nono setup --start-wfp-service`. (REQ-WFP-01, D-01; inverse of the Phase 60 `network.block:false` workaround.)
  2. **Boot-start posture survives reboot.** The machine MSI registers `nono-wfp-service` with `start=auto` so the SCM boot-starts it as SYSTEM; after a reboot the service is already running and the SC1 scenario succeeds without elevation. (D-01)
  3. **Fail-closed, never silent.** When a `network.block:true` run finds the service not running (stopped, dev layout), nono attempts to start it; if the start succeeds it enforces, and if it cannot (e.g. no elevation) it aborts fail-closed with an actionable error naming the exact elevated remediation command — it NEVER passes through unenforced and performs no netsh FirewallRules fallback. The decision path is unit-tested via the injectable-runner seam without requiring elevation or a running service. (D-03, D-04)
  4. **Clean uninstall preserved.** `sc stop` + `msiexec /x` of the machine MSI still leaves nothing behind (service fully removed); the `start=demand → start=auto` flip does not regress the Phase 53 / REQ-DRN-01 leave-nothing invariant. (regression guard)
  5. **Non-elevated pipe access.** The `nono-wfp-service` control-pipe SDDL grants the non-elevated supervised-run persona (Interactive Users) connect access, so a standard-user `nono run` reaches the boot-started service without "Access is denied". (research critical finding — blocking for D-01 end-to-end.)
**Out of scope (this phase)**: `nono-wfp-driver.sys` real kernel minifilter (Gap 6b, v3.0-deferred); per-process/AppID filter scoping; fine-grained `allow_domain` path/method filtering (that is Phase 56 / REQ-NET-01, proxy layer); the user-scope MSI (`nono-user.wxs`) cannot register SCM services, so boot-start is machine-MSI-only.
**Plans**: 13 plans (4 planned + 9 gap-closure)
Plans:
**Wave 1** (parallel -- no file overlap)
- [x] 62-01-PLAN.md -- D-03 auto-start hook + start=auto in build_wfp_service_create_args + 3 unit tests (network.rs)
- [x] 62-02-PLAN.md -- ServiceInstall Start=auto (nono-machine.wxs) + PIPE_SDDL IU ACE + SDDL unit test (nono-wfp-service.rs)
- [x] 62-03-PLAN.md -- REQ-WFP-01 in REQUIREMENTS.md + ROADMAP Phase 62 plan list (planning artifacts)
- [x] 62-05-PLAN.md -- GAP-CLOSURE F-62-01: MSI generator (build-windows-msi.ps1) + contract guard set ServiceInstall Start=auto; .wxs was a regenerated snapshot, not the build source
- [x] 62-06-PLAN.md -- GAP-CLOSURE F-62-UAT-01: drop the out-of-scope kernel-driver gate from build_wfp_probe_status so Ready (the only trigger for the service FwpmFilterAdd0 activation IPC) is reachable BFE+service-only, per D-05; retire stale SERVICE placeholder strings (network.rs); root-caused in debug wfp-driver-gate-placeholder
- [x] 62-07-PLAN.md -- GAP-CLOSURE F-62-UAT-02: set non-null filter displayData.name in add_policy_filter (nono-wfp-service.rs) so FwpmFilterAdd0 stops failing RPC_X_BAD_STUB_DATA (win32 1783); 7-point FWPM field audit confirmed name was the sole defect; root-caused in debug wfp-filter-add-1783
- [x] 62-08-PLAN.md -- GAP-CLOSURE F-62-UAT-03: wrap the ALE_USER_ID security descriptor in an FWP_BYTE_BLOB (windows-sys types sd as *mut FWP_BYTE_BLOB) so FwpmFilterAdd0 stops failing RPC_X_BAD_STUB_DATA (1783); second stacked 1783 cause on the SID block path
- [x] 62-09-PLAN.md -- GAP-CLOSURE F-62-UAT-04: make the WFP session PERSISTENT (drop FWPM_SESSION_FLAG_DYNAMIC) so the sublayer (startup engine) + filters (per-request engines) share one namespace; fixes FwpmFilterAdd0 FWP_E_WRONG_SESSION (0x8032000C); root-caused in debug wfp-wrong-session-dynamic. FOLLOW-UP 62-11 = uninstall WFP purge (REQ-DRN-01 leave-nothing) before SC4
- [x] 62-10-PLAN.md -- GAP-CLOSURE F-62-UAT-05: inject session_sid into the broker (BrokerLaunchNoPty) Low-IL child token as a WRITE_RESTRICTED restricting SID via new `nono::create_low_integrity_primary_token_with_sid` + `--session-sid` plumbing (launch.rs -> broker argv -> token build), fail-closed; the WFP ALE_USER_ID filter installed cleanly post-62-09 but matched NOTHING because the broker token lacked session_sid (curl reached the net). Closes the SC1 enforcement-MATCH gap; root-caused + DESIGNED in debug wfp-broker-token-no-sid. ⚠ SUPERSEDED by 62-12: live UAT showed this WRITE_RESTRICTED token crashes the confined child at startup (0xC0000142 STATUS_DLL_INIT_FAILED); design falsified, re-root-caused in debug wfp-write-restricted-0142
- [x] 62-11-PLAN.md -- GAP-CLOSURE (REQ-DRN-01 leave-nothing): add `--purge-wfp-objects` one-shot mode to nono-wfp-service (delete all NONO_SUBLAYER_GUID filters + FwpmSubLayerDeleteByKey0), invoked fail-open from `setup --uninstall-wfp` before service delete, so the now-PERSISTENT (62-09) sublayer/filters are removed by `msiexec /x`. Closes the SC4/SC5 gap 62-09 deferred; no WiX change
- [x] 62-12-PLAN.md -- GAP-CLOSURE F-62-UAT-05 (REDESIGN, supersedes 62-10): spawn the confined child as a per-run AppContainer (lowbox) — derive package SID S-1-15-2-* from name `nono.session.<uuid>` (DeriveAppContainerSidFromAppContainerName), spawn via STARTUPINFOEX + SECURITY_CAPABILITIES{AppContainerSid,0 caps} (starts cleanly), scope WFP by the package SID reusing the proven ALE_USER_ID SD path, retarget AppliedDaclGrantsGuard to the package SID, remove the dead WRITE_RESTRICTED broker code; fail-closed + single-source preserved. Fixes the 0xC0000142 startup crash; root-caused + DESIGNED in debug wfp-write-restricted-0142
- [x] 62-13-PLAN.md -- GAP-CLOSURE F-62-UAT-05 (AppContainer SPAWN FIX, spike-validated): broker registers the per-run AppContainer profile via CreateAppContainerProfile (RAII DeleteAppContainerProfile) BEFORE the SECURITY_CAPABILITIES spawn — 62-12 used Derive-only -> CreateProcessW ERROR_FILE_NOT_FOUND. Spike (examples/spike_wfp_appcontainer.rs) PROVED WFP blocks an AppContainer connection via the existing ALE_USER_ID(packageSid) path (both ALE_USER_ID + ALE_PACKAGE_ID block). Keep ALE_USER_ID; + package-SID FILE_TRAVERSE on user-owned cwd ancestors. Full claude.exe read-grant model deferred

**Wave 2** (blocked on Wave 1 -- requires code and MSI complete)
- [ ] 62-04-PLAN.md -- HUMAN-UAT: machine-MSI install, reboot, out-of-box enforced block, clean uninstall (REQ-WFP-01 SC1-SC5)

## Future Cycles

### UPST8 — Upstream v0.59.0… sync audit (placeholder)

**Goal**: Audit upstream `v0.59.0..<next-tag>` divergence per the Phase 33 ADR `continue` cadence rule. Inherits the audit-shape template from Phase 33 + 39 + 42 + 47 + 54 verbatim. The first deferred-from-UPST7 targets are **v0.60.0 (`9a05a4ff`), v0.61.0, and v0.61.1** (the 2026-06-04 UPST7 re-fetch surfaced all three past the locked `v0.57.0..v0.59.0` range; the deferred set is `v0.60.0..v0.61.1`, NOT v0.60.0 alone — and NOT the unrelated Feb-2026 v0.6.x tag line). Title may flip from `sync audit` to `sync execution` if the next cycle's commit set is small enough to skip a dedicated audit (auditor's call at UPST8 plan-phase).
**Depends on**: Phase 55 (UPST7 cherry-pick wave must close before UPST8 audit; cadence rule preserves linear ordering)
**Plans**: 0 / TBD
**Reference**: `docs/architecture/upstream-parity-strategy.md` § Future audit cadence

UPST8 fires when the maintainer decides the accumulated cherry-pick labor (v0.60.0..v0.61.1 deferred at Phase 54; will grow before UPST8 fires) warrants absorbing.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 53. Release & Drain | 3/4 | In Progress|  |
| 54. UPST7 Audit | 1/1 | Complete | 2026-06-04 |
| 55. UPST7 Cherry-pick Wave | 4/7 | In Progress|  |
| 56. Fine-grained Network Filtering | 0/TBD | Not started | - |
| 57. Bitwarden Credential Source | 0/TBD | Not started | - |
| 58. Session Lifecycle Hooks | 0/TBD | Not started | - |
| 59. Supervisor IPC Robustness | 0/TBD | Not started | - |
| 60. Confined Coding Loop (v2.9) | 3/3 | Complete   | 2026-05-29 |
| 61. Ship/Release v2.9 | 3/4 | In Progress|  |
| 62. WFP kernel network enforcement (Windows supervised) | 12/13 | Complete    | 2026-06-03 |

## Coverage

All 10 v1 requirements mapped:

| REQ-ID | Phase |
|--------|-------|
| REQ-RLS-01 | Phase 53 |
| REQ-RLS-02 | Phase 53 |
| REQ-DRN-01 | Phase 53 |
| REQ-DRN-02 | Phase 53 |
| REQ-UPST7-01 | Phase 54 |
| REQ-UPST7-02 | Phase 55 |
| REQ-NET-01 | Phase 56 |
| REQ-CRED-01 | Phase 57 |
| REQ-HOOK-01 | Phase 58 |
| REQ-IPC-01 | Phase 59 |
| REQ-STW-01 (v2.9 track) | Phase 60 |
| REQ-STW-02 (v2.9 track) | Phase 60 |
| REQ-WFP-01 (v2.9 track) | Phase 62 |

## References

- `.planning/PROJECT.md` — project context + current state.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.7).
- `.planning/REQUIREMENTS.md` — v2.8 requirements (REQ-RLS, REQ-DRN, REQ-UPST7, REQ-NET, REQ-CRED, REQ-HOOK, REQ-IPC).
- `.planning/quick/260527-sgo-upstream-v044-v059-gap-analysis/GAP-ANALYSIS.md` — UPST7 gap matrix + phase buckets (authoritative seed).
- `.planning/milestones/v2.7-ROADMAP.md` — archived v2.7 phase detail (Phases 51-52).
