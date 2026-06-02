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
- **v2.9 Windows Sandbox-the-Tools — Confined Coding Loop** — Phase 60 (planned; separate initiative from UPST7, builds on merged PR #4)

## Phases

- [x] **Phase 53: Release & Drain** — Tag v2.8 + v0.57.5, produce signed MSIs off the post-`005b4c9e` binary, verify release.yml, UAT WFP uninstall, drain 3 todos (shipped as v0.57.5 after a release.yml signing-order fix; all 5 SC verified)
- [ ] **Phase 54: UPST7 Audit** — Produce DIVERGENCE-LEDGER for upstream `v0.57.0..v0.59.0`; per-cluster dispositions + ADR re-confirm + re-export empirical cross-check
- [ ] **Phase 55: UPST7 Cherry-pick Wave** — Absorb cross-platform straight ports (JSONC, target_binary, opencode relocation, timeout constants, java-dev, proxy 502, denial/diagnostic polish) per Phase 54 dispositions
- [ ] **Phase 56: Fine-grained Network Filtering** — `allow_domain` URL path + HTTP method restrictions in nono-proxy; TLS-intercept endpoint-rules-before-credential-selection ordering fix
- [ ] **Phase 57: Bitwarden Credential Source** — `bw://` keystore backend alongside `keyring://`/`env://`/`file://`; `Zeroizing<String>` secret posture
- [ ] **Phase 58: Session Lifecycle Hooks** — `session_hooks` profile field; Unix upstream behavior preserved; Windows broker-spawned Low-IL execution design + ADR; fail-closed on hook failure
- [ ] **Phase 59: Supervisor IPC Robustness** — Keep-alive on transient child IPC close, bounded read-timeouts, robust accept; Unix named-socket hardening absorbed cross-platform-core; Windows Named-Pipe AIPC path translated (not cherry-picked)

### v2.9 — Windows Sandbox-the-Tools (separate track)

- [x] **Phase 60: Confined Coding Loop** — Make the merged PR #4 tool-mediation slice a usable coding agent for Windows POC users: confined Low-IL **file edits** (Write/Edit/MultiEdit/NotebookEdit) via per-call capability mapping instead of deny, plus a usable shell story (PowerShell-runner decision). Network/WebFetch/MCP/Task stay denied (out of POC scope). Input: `.planning/quick/260528-sch-spec-the-sandbox-the-tools-windows-tool-/260528-sch-SPEC.md` (§7 answered)
 (completed 2026-05-29)

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
- [ ] 54-01-UPST7-AUDIT-PLAN.md — DIVERGENCE-LEDGER for v0.57.0..v0.59.0: re-fetch + drift run, per-cluster dispositions + windows-touch, ADR review, empirical cross-check, SC4 TLS-intercept assessment, UPST8 stub

### Phase 55: UPST7 Cherry-pick Wave
**Goal**: The cross-platform straight-port clusters from the UPST7 audit are absorbed into the fork with correct D-19 trailers and the fork's invariants intact
**Depends on**: Phase 54 (dispositions are the input)
**Requirements**: REQ-UPST7-02
**Success Criteria** (what must be TRUE):
  1. Every `will-sync` cluster disposition from Phase 54 is executed: JSONC profile parsing, `target_binary` profile field, `opencode` pack relocation, configurable timeout constants, `java-dev` profile / `java_runtime` group (with Windows JDK paths via `platform.rs`), proxy 502 hardening, suppressed-denial annotations, canonical denial-path precompute, access-mode `rfind` split, and overflow-check tightening are all present in the fork tree
  2. Each absorbed upstream commit carries a verbatim lowercase 6-line `Upstream-commit:` trailer (D-19 convention) or `Upstream-replayed-from:` for D-20 replays; the D-43-E1 Windows-only-files invariant is respected
  3. Schema-collision checks confirm no canonical-section conflicts between absorbed upstream profile schema changes and the fork's `nono-profile.schema.json` / `policy.json` canonical sections
  4. `make ci` (or the Windows equivalent `cargo test --workspace`) passes with zero new test failures relative to the Phase 54 baseline SHA
**Plans**: TBD

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

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 53. Release & Drain | 3/4 | In Progress|  |
| 54. UPST7 Audit | 0/1 | Not started | - |
| 55. UPST7 Cherry-pick Wave | 0/TBD | Not started | - |
| 56. Fine-grained Network Filtering | 0/TBD | Not started | - |
| 57. Bitwarden Credential Source | 0/TBD | Not started | - |
| 58. Session Lifecycle Hooks | 0/TBD | Not started | - |
| 59. Supervisor IPC Robustness | 0/TBD | Not started | - |
| 60. Confined Coding Loop (v2.9) | 3/3 | Complete   | 2026-05-29 |

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

## References

- `.planning/PROJECT.md` — project context + current state.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.7).
- `.planning/REQUIREMENTS.md` — v2.8 requirements (REQ-RLS, REQ-DRN, REQ-UPST7, REQ-NET, REQ-CRED, REQ-HOOK, REQ-IPC).
- `.planning/quick/260527-sgo-upstream-v044-v059-gap-analysis/GAP-ANALYSIS.md` — UPST7 gap matrix + phase buckets (authoritative seed).
- `.planning/milestones/v2.7-ROADMAP.md` — archived v2.7 phase detail (Phases 51-52).
