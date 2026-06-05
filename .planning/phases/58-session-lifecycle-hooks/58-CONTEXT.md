# Phase 58: Session Lifecycle Hooks - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Profiles can declare a `session_hooks` field (`before` / `after`) that runs
**script files outside the sandbox** at session start and stop. This phase
delivers three things:

1. **Schema + profile surface** — the `session_hooks` type (`before`, `after`,
   `timeout_secs`) on the profile, threaded into `to_raw_profile()` /
   `LaunchFlags`. This is the production hunk Phase 55 explicitly deferred here
   (`SessionHooks` does **not** exist in the fork yet — only the test-only
   `ENV_LOCK` change from upstream `1a764d05` landed in 55).
2. **Unix runtime** — port upstream's `hook_runtime.rs` (commit `daa55c8`,
   gated unix-only per `1335351`) with one deliberate fork divergence: nono is
   **fail-closed**, not upstream's fail-open (see D-01).
3. **Windows runtime + ADR** — a Windows-safe broker-spawned **Low-IL** hook
   executor (no `fork`/`sh` assumption), plus an ADR committed to `.planning/`
   documenting the Windows execution design and the invariants the executor
   must preserve.

**Out of scope:** Supervisor IPC robustness (Phase 59). The existing
Claude-Code `hooks` field (PreToolUse/PostToolUse install — `HooksConfig` in
`profile/mod.rs`) is a **different** concept and is not touched.

</domain>

<decisions>
## Implementation Decisions

### Fail-policy (reconciling upstream fail-open vs REQ-HOOK-01 fail-closed)
- **D-01:** **Fail-closed on BOTH platforms**, overriding upstream's fail-open
  behavior. This is a deliberate, documented fork divergence.
- **D-02:** **SC2 is reinterpreted** as "preserve the upstream runtime
  *mechanism* exactly" (script validation, env-file pattern, timeout +
  process-group kill, env-var filtering) **while hardening the fail-policy to
  fail-closed**. This reinterpretation MUST be recorded as a fork invariant in
  the ADR and in the Unix port's module docs, and called out in the phase
  SUMMARY/VERIFICATION so the SC2 "preserved exactly" criterion is satisfied
  against the *mechanism*, not against upstream's fail-open semantics.
- **D-03:** **Before-hook failure** (resolution failure or non-zero exit) →
  **the session does not start.** Never silently skipped.
- **D-04:** **After-hook failure** → loud error surfaced in logs **AND nono
  exits non-zero** so CI/automation sees it. The session already ran (can't be
  un-run), but the failure is never swallowed. Satisfies SC4's "stops with an
  error, never silently skipped." Mirror the fork's existing diagnostic-footer
  pattern for the loud error.

### Windows trust level (the ADR core)
- **D-05:** Windows hooks execute as **confined Low-IL via the broker**, using
  the **`LowIlPrimary` (primary-token) broker arm** — NOT `WriteRestricted`
  (the .NET/PowerShell CLR cannot start under WRITE_RESTRICTED; known from
  Phase 60, [[project_sandbox_the_tools]]). Hooks are **confined, not
  host-trusted** — this is the deliberate divergence from upstream's
  "outside the sandbox with host privileges" semantics, and is the ADR's
  central invariant.
- **D-06:** **Hook filesystem scope = session-dir write + cwd write + read on
  the script path, and nothing else.** Specifically: write to
  `~/.nono/sessions/<id>/` (the session dir, incl. the env file) and the run's
  cwd; read the hook script path. Everything broader is **denied at the OS
  boundary.** This is the minimal grant that lets a setup/cleanup hook be
  useful without becoming a host-privilege escape.

### Env-file export (NONO_ENV_FILE) on Windows
- **D-07:** **Port the env-export mechanism to Windows.** Before-hooks can
  export `KEY=VALUE` env vars via the session-dir env file; the (Medium-IL)
  parent reads, filters, and injects them into the child.
- **D-08:** Windows env-file creation uses a **restrictive ACL + `CREATE_NEW`**
  (the Windows equivalent of upstream's `O_EXCL` + `0o600`) — no clobber, not
  readable/writable by lower-IL or other principals.
- **D-09:** Parent applies the **same `is_dangerous_env_var()` filter**,
  **extended for Windows** to cover Windows-significant vars (at minimum
  `PATH`, `PATHEXT`, `COMSPEC`; research to confirm the full Windows danger
  set). The Low-IL-writer → Medium-IL-reader trust gap is mitigated by the ACL
  + the dangerous-var filter; the ADR must name this gap and its mitigation.

### Windows "vetted hook" bar
- **D-10:** **Parity port** of upstream's defense-in-depth checks onto Windows
  primitives — no new allowlist concept. Required checks before any hook runs:
  - absolute + **canonical path** (resolve via `\\?\`),
  - regular file (not a dir/device/reparse-surprise),
  - **`path_is_owned_by_current_user`** (existing fork helper),
  - **no world-writable / no lower-IL-writable ACL** on the file *and* its
    parent dir,
  - **mandatory-label check** consistent with the fork's label helpers.
  Pair `path_is_owned_by_current_user` with an effective-rights ACL mask check
  (per [[feedback_windows_mandatory_label_write_owner]] discipline).

### Claude's Discretion
- **Windows interpreter / exec path** (DEFERRED TO RESEARCH): how a hook script
  is actually launched under the Low-IL broker — direct `CreateProcess` of the
  script (`.exe` native; `.ps1`/`.cmd` via registered handler or explicit
  `powershell.exe -NoProfile -File`) vs standardizing on a PowerShell runner.
  Research determines the safest exec path given `CreateProcess` + IL
  constraints + the Phase 60 PowerShell-steering direction. Constraint:
  **script-file references only, no inline scripts** (preserve upstream's
  no-JSON-injection rule).
- **`timeout_secs` enforcement on Windows**: upstream uses `setpgid` + `killpg`
  on timeout; Windows has no direct equivalent. Research the Windows mechanism
  (Job Object kill / `TerminateJobObject` on the broker-spawned process tree)
  to honor the user-configured `timeout_secs`. No hardcoded default (match
  upstream — timeout only when the profile sets it).
- **Session-id generation/reuse**: follow the fork's existing `session::`
  helpers (`generate_session_id`) already referenced in `execution_runtime.rs`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirement & roadmap
- `.planning/REQUIREMENTS.md` → **REQ-HOOK-01** (the `session_hooks` field;
  Unix preserved, Windows broker-spawned/Low-IL ADR; **fail-closed**).
- `.planning/ROADMAP.md` → **Phase 58** Goal + Success Criteria (SC1–SC4).

### Upstream source to port (fork has both commits fetched via `upstream` remote)
- upstream commit **`daa55c8`** "feat: session lifecycle hooks (#954)" — the
  feature. Touches `data/nono-profile.schema.json`, `execution_runtime.rs`,
  `hook_runtime.rs` (610 lines, new), `launch_runtime.rs`, `main.rs`,
  `profile/mod.rs`, `sandbox_prepare.rs`, `tests/schema_shape.rs`. **NOTE:
  upstream is fail-OPEN — D-01 changes this to fail-closed.**
- upstream commit **`1335351`** "refactor(hook_runtime): gate module unix-only,
  drop dead non-unix branches" — confirms upstream's runtime is **unix-only**;
  the Windows path is net-new fork work.
- upstream commit **`1a764d05`** — the test-only `ENV_LOCK` hunk already landed
  in Phase 55; the `session_hooks` production hunk in
  `policy.rs::to_raw_profile()` was deferred to THIS phase.

### Divergence ledger & cherry-pick history
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` → **Cluster C8**
  (lines ~205–219) — disposition `split`, Windows-touch yes, rationale for the
  Phase 55 (schema/cross-platform) vs Phase 58 (runtime + Windows ADR) split.
- `.planning/phases/55-upst7-cherry-pick-wave/55-06-SUMMARY.md` (§ "Production
  code (deferred)") + `55-VERIFICATION.md` (deferral row) — exactly what was
  deferred here and why (`SessionHooks` type does not exist in fork yet).

### Fork integration points (existing code)
- `crates/nono-cli/src/execution_runtime.rs` — where upstream wired
  before/after hooks (`execute_sandboxed`); already has the Windows broker arm
  selection (`windows_low_il_broker`, `LowIlPrimary`/`BrokerLaunchNoPty`).
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` (`WindowsTokenArm`,
  ~line 1107) + `restricted_token.rs` / `labels_guard.rs` / `dacl_guard.rs` —
  the Low-IL broker + label/ACL primitives the Windows hook executor builds on.
- `crates/nono-cli/src/profile/mod.rs` (existing `HookConfig`/`HooksConfig`,
  ~line 1763 — the *different* Claude-Code-hook concept; do not conflate) and
  `crates/nono-cli/src/policy.rs` (`to_raw_profile`, ~line 118).
- `crates/nono-cli/data/nono-profile.schema.json` — schema surface to extend.

### Cross-cutting discipline
- `CLAUDE.md` § Coding Standards — **cross-target clippy MUST** rule (this phase
  touches cfg-gated Unix code in the new `hook_runtime.rs` AND Windows
  `exec_strategy_windows/` — both `--target x86_64-unknown-linux-gnu` and
  `--target x86_64-apple-darwin` verification required, or mark PARTIAL per
  `.planning/templates/cross-target-verify-checklist.md`).

### ADR to author (output of this phase)
- New ADR under `.planning/` (location is Claude's discretion — follow the
  fork's existing ADR convention; Phase 33 Option A ADR is the precedent)
  documenting the Windows hook execution design (D-05–D-10) and its invariants:
  mandatory-label enforcement, Low-IL confinement, session-dir+cwd-only scope,
  no unrestricted shell access, the Low-IL-writer→Medium-IL-reader env trust gap
  + mitigation, and the fail-closed divergence from upstream.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Low-IL broker arm** (`LowIlPrimary` in `exec_strategy_windows/launch.rs`,
  selected via `profile.windows_low_il_broker`) — the spawn path for confined
  Windows hook execution. CLR-capable (unlike `WriteRestricted`).
- **`restricted_token.rs` / `labels_guard.rs` / `dacl_guard.rs`** — token,
  mandatory-label, and DACL-grant/revoke RAII guards (e.g.
  `grant_sid_write_on_path` / `AppliedDaclGrantsGuard` from Phase 60) for
  scoping the hook's session-dir + cwd write grant (D-06).
- **`path_is_owned_by_current_user`** helper — reuse for the Windows vet bar
  (D-10).
- **`session::generate_session_id`** — already referenced in
  `execution_runtime.rs`; reuse for the `~/.nono/sessions/<id>/` path.

### Established Patterns
- The fork's execution path matches upstream's enough that `daa55c8`'s
  integration points (`execution_runtime.rs`, `launch_runtime.rs`,
  `sandbox_prepare.rs`, `main.rs`, `profile/mod.rs`) **all exist in the fork** —
  the Unix port is structurally feasible without architecture surgery.
- Fail-closed + loud diagnostic footer is the fork's established failure idiom
  (reuse for D-03/D-04).
- `#[cfg(unix)]` gating of `hook_runtime` + `Vec::new()` fallback on
  `not(unix)` is the upstream pattern (`1335351`); the fork instead supplies a
  **real Windows implementation** rather than the empty fallback.

### Integration Points
- Before-hook env injection happens in `execute_sandboxed` before the env_vars
  vector is built; after-hook runs after the child returns. Both already have
  `#[cfg(unix)]` shaped slots in upstream's diff — the fork adds the Windows
  arm alongside.
- Env-file is created in the session dir, written by the hook (Low-IL on
  Windows), read+filtered by the parent (Medium-IL) — the trust boundary that
  D-08/D-09 protect.

</code_context>

<specifics>
## Specific Ideas

- Preserve upstream's **script-file-references-only** rule (no inline scripts)
  on both platforms — it's the no-JSON-injection guarantee.
- The env-export feature is the *only* channel by which a before-hook
  influences the run; treat its Windows trust boundary (Low-IL writer → parent
  reader) as a first-class security item in the ADR, not an afterthought.

</specifics>

<deferred>
## Deferred Ideas

- **Profile-level fail-open toggle** — considered and rejected for this phase
  (D-01 chose unconditional fail-closed). If a real use case for upstream's
  fail-open ever appears, it can be a future opt-in field; not built now.
- **Explicit hook allowlist in the profile** — considered for the vet bar and
  rejected (D-10 chose owner+ACL parity, no allowlist). Revisit only if
  owner+ACL proves insufficient.
- Anything touching Supervisor IPC robustness → **Phase 59** (REQ-IPC-01).

</deferred>

---

*Phase: 58-session-lifecycle-hooks*
*Context gathered: 2026-06-05*
