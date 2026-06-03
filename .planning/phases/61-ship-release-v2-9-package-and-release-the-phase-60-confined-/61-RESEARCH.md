# Phase 61: Ship/Release v2.9 - Research

**Researched:** 2026-06-03
**Domain:** Release engineering — versioned dual-tag GitHub release of a Rust/Windows workspace via existing CI (`release.yml`)
**Confidence:** HIGH (all findings grounded in repo files at HEAD `1f656153`)

## Summary

This is a packaging/release phase, not a feature phase. Phases 60 (confined coding loop) and 62 (WFP enforcement) are built and on `main`. The release machinery (`release.yml`) exists and was live-verified on `v0.57.5`. The job is to (a) lockstep-bump the workspace 0.57.5 → 0.58.0, (b) confirm the D-09 security blocker is closed, (c) cut + push the dual tag `v2.9` + `v0.58.0`, (d) live-verify the CI signing pipeline, and (e) handle the superseded v0.57.4 release.

Two of the perceived risks are already neutralized by prior work: **D-09's robust fix is already implemented and committed on `main`** (Phase 60-03 hook-level CWD guard in `claude_code_hook.rs`), and **the superseded v0.57.4 GitHub release does not exist** on the `always-further/nono` remote (`gh release list` shows only `v0.57.5` as Latest). Both collapse from "build/fix" tasks to "verify and document" tasks. The genuine remaining risk is the **mechanical tag sequencing** (the tag IS the trigger — the tagged commit must already carry the 0.58.0 version) and confirming CI signing secrets are present at release time.

**Primary recommendation:** Treat this as: bump → verify D-09 is closed → commit → tag `v0.58.0` (triggers release.yml) → watch CI → then tag `v2.9` on the same commit. Do NOT relitigate the D-09 enforcement design — verify the existing guard instead.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions (D-01..D-09 — every one MUST be cited by a plan; decision-coverage gate)
- **D-01:** Release as a CI-signed public GitHub release via `release.yml` on a `v*` tag push (machine+user MSIs). Production path, not local POC-cert.
- **D-02:** Assumes real code-signing cert/secrets present in CI (signed v0.57.5). If CI signing material is unavailable at release time → release blocker to surface; do NOT fall back to self-signed POC cert for a public release.
- **D-03:** Dual-tag the milestone: `v2.9` AND `v0.58.0`. Minor bump is correct semver (two features).
- **D-04:** Bump workspace to 0.58.0 in lockstep — all crate `Cargo.toml` versions + internal path-dep `version` pins + `Cargo.lock` + MSI ProductVersion. `.wxs` ProductVersion flows from the build tag — do NOT hand-edit the generated `.wxs`.
- **D-05:** Ship current `main` — bundles Phase 60 + Phase 62 + untagged post-v2.7 drain fixes. ROADMAP's "0.57.5 binaries" wording is stale; shipped binaries are the 0.58.0 build off current main.
- **D-06:** release.yml live-verify on the fresh tag — confirm signed MSIs cleanly. Signing-ORDER + MSI-payload gates churned since v0.57.5; re-verify.
- **D-07:** Annotate or delete the superseded v0.57.4 GitHub release (shipped unsigned-payload MSIs).
- **D-08:** Confirm untagged v2.7 drain fixes present: broker GLE=87 (`d8b7ce00`), no-PTY relay stdout-echo (`005b4c9e`), WFP service-stop/uninstall (`0cbeb3be` + `b852826b`).
- **D-09 (pre-ship security blocker):** Runner profile must deny `~/.claude` (and project `.claude/`) regardless of `--allow-cwd`. Deny must win over `--allow-cwd` in the shipped `claude-code` runner profile. Verify with `nono why` / a deny-overlap test before tagging.

### Claude's Discretion
- Release-notes structure/wording (Windows confined tool-mediation POC + WFP out-of-box enforcement; honest "POC / defense-in-depth, not full isolation" framing). Source: `.planning/quick/260603-e0x-.../260603-e0x-WINDOWS-STATUS.md`.
- Whether the 0.58.0 bump is its own plan/wave vs folded into release-prep.
- Defining the phase's requirement ID (currently TBD) — propose a `REQ-RLS`-style requirement and register it in REQUIREMENTS.md.

### Deferred Ideas (OUT OF SCOPE)
- v2.8 UPST7 upstream sync (Phases 53–59); v3.0 deferrals (kernel minifilter Gap 6b, EDR HUMAN-UAT WR-02); claude.exe full read-grant model under AppContainer.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-RLS-03 (proposed) | The v2.9 milestone is published as a CI-signed public GitHub release: lockstep 0.58.0 workspace bump, dual tag `v2.9`+`v0.58.0` off current `main`, release.yml produces signed machine+user MSIs (Authenticode-valid wrapper AND payload), release notes written, superseded v0.57.4 release resolved. | This entire research; mirrors how v2.8 used REQ-RLS-01/02 (see STATE.md Phase 53 row). |
| REQ-RLS-04 (proposed) | The shipped `claude-code` tool runner cannot expose `~/.claude` (credentials/session state) or a project `.claude/` to a confined tool call regardless of `--allow-cwd` — verified by a deny-overlap test and the existing hook-level CWD guard. | D-09 section below; the fix is already on `main` (Phase 60-03), so this REQ is satisfied-pending-verification. |

> **Planner note:** REQUIREMENTS.md currently maps Phase 61 to "TBD". Register REQ-RLS-03/04 (or maintainer's chosen IDs) before planning so plans have a tracked requirement. The decision-coverage gate requires EACH of D-01..D-09 to be cited by at least one plan — thread D-04/D-05 into the bump plan, D-01/D-02/D-06 into the release-verify plan, D-03 into the tag plan, D-07/D-08 into a cleanup/verify plan, and D-09 into the security-verify plan.
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Version bump | Source (Cargo manifests + lockfile) | — | Crate versions are per-crate literals; no workspace.package.version |
| MSI ProductVersion | CI build script (`build-windows-msi.ps1`) | — | Derived from `-VersionTag` (the git tag), NOT from Cargo.toml |
| Signing | CI (`release.yml`) | — | Authenticode via repo secrets; never local for public release (D-01/D-02) |
| `~/.claude` deny enforcement | CLI hook (`claude_code_hook.rs`) | Runner profile (defense-in-depth) | Windows label backend cannot deny-within-allow; hook is the load-bearing boundary |
| Release publish + notes | CI (`softprops/action-gh-release`) | Operator (`gh` for v0.57.4 cleanup) | Tag push auto-publishes; manual `gh` for the superseded-release fixup |

## Standard Stack

No new dependencies. This phase uses existing tooling only:

| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| WiX | 7.0.0 | MSI generation (`build-windows-msi.ps1`) | Pinned via `WIX_VERSION` env in release.yml `[VERIFIED: release.yml:20]` |
| `gh` CLI | (available) | Inspect/annotate releases | Confirmed available in env `[VERIFIED: memory feedback_gh_available]` |
| `softprops/action-gh-release` | v2 | Publishes the GitHub release | `[VERIFIED: release.yml:437]` |
| `cargo` | stable | Bump verification (`cargo update -p`/`cargo build`) | Regenerates `Cargo.lock` |

**Package Legitimacy Audit:** N/A — no external packages installed this phase.

## Version-Bump File Set (D-04) — EXACT enumeration

All currently at `0.57.5` `[VERIFIED: grep across manifests]`. Lockstep change to `0.58.0`:

| File | What changes | Line (current) |
|------|--------------|----------------|
| `Cargo.toml` (workspace root) | **Nothing** — there is NO `[workspace.package].version` field; versions are per-crate | (none) |
| `crates/nono/Cargo.toml` | `version = "0.58.0"` | `:3` |
| `crates/nono-cli/Cargo.toml` | `version`, + path-dep pins `nono`, `nono-proxy`, `nono-shell-broker` | `:3, :43, :44, :158` |
| `crates/nono-proxy/Cargo.toml` | `version`, + path-dep pin `nono` | `:3, :21` |
| `crates/nono-shell-broker/Cargo.toml` | `version`, + path-dep pin `nono` | `:3, :19` |
| `bindings/c/Cargo.toml` | `version`, + path-dep pin `nono` | `:3, :18` |
| `Cargo.lock` | regenerated (`cargo build` or `cargo update --workspace --offline`) | — |
| `CHANGELOG.md` | add `## [0.58.0] - v2.9` section (current top is `## [Unreleased] - v2.6 Phase 45` — note CHANGELOG is stale vs shipped versions) | `:3` |

**Path-dep pin gotcha (CONFIRMED):** each consuming crate pins the internal dep both by `path` AND `version` (e.g. `nono = { version = "0.57.5", path = "../nono" }`). The `version` literal MUST be bumped too or `cargo publish` (release.yml `publish-crates` job) and `cargo build` version-resolution will mismatch. There are **6 path-dep pins** total across cli (3), proxy (1), broker (1), bindings/c (1). `[VERIFIED: project_workspace_crates memory + grep]`

**`tools/sign-fixture`** is a 6th workspace member (`Cargo.toml:9`) not in the CONTEXT canonical-ref list — check whether it carries a version literal or a path-dep pin on `nono`; bump if so. `[ASSUMED — not yet grepped; flag for planner]`

**MSI ProductVersion (do NOT hand-edit `.wxs`):** `build-windows-msi.ps1` computes `$msiVersion = ConvertTo-MsiVersion -Tag $VersionTag` (`:206`) and injects it into the here-string `<Package ... Version="$msiVersion">` (`:337`). `$VersionTag` comes from release.yml's `RELEASE_TAG` (the git tag, `:198`). `ConvertTo-MsiVersion` strips a leading `v` and requires ≥3 numeric components (`:50, :56`). So `v0.58.0` → MSI `0.58.0` automatically. The checked-in `dist/windows/nono-machine.wxs` / `nono-user.wxs` are GENERATED artifacts — never hand-edit. `[VERIFIED: build-windows-msi.ps1 + windows_msi_wxs_is_generated memory]`

**Tag-sequencing risk (the real one):** the tagged commit must ALREADY carry the 0.58.0 bump + D-09 verification. Sequence: bump+commit → push main → create+push `v0.58.0` (triggers release.yml) → after CI green, create+push `v2.9` on the same commit. release.yml triggers on `v*.*.*` (`:6`); `v2.9` is `v2.9` with only 2 components — **it would FAIL the `v*.*.*` glob** and (if it did match) `ConvertTo-MsiVersion` requires 3 components. This is fine: `v2.9` is a milestone marker tag, NOT a build trigger; only `v0.58.0` triggers a build. Confirm `v2.9` is intended as a non-building annotation tag. `[VERIFIED: release.yml:6 + build-windows-msi.ps1:56]`

## release.yml Current Shape (D-01 / D-06)

`[VERIFIED: .github/workflows/release.yml, read in full]`

- **Trigger:** `on: push: tags: ['v*.*.*']` + `workflow_dispatch` (manual tag input). `:3-12`
- **Signing material expected:** repo secrets `WINDOWS_SIGNING_CERT` (base64 PFX) + `WINDOWS_SIGNING_CERT_PASSWORD`. A dedicated `Check signing secrets (Windows)` step (`:124`) **fails the build closed** if either is empty — this is exactly the D-02 blocker surfacing mechanism. Also uses `CARGO_REGISTRY_TOKEN` (crates.io) + `HOMEBREW_CORE_TOKEN`.
- **Signing ORDER (Phase 53 fix — CONFIRMED PRESENT):**
  1. `Sign Windows binaries (pre-package)` (`:146`) — signs `nono.exe`, broker, wfp-service BEFORE MSI build, so MSIs embed signed payloads.
  2. `Package (Windows)` (`:166`) — builds machine + user MSIs from now-signed binaries.
  3. `Sign Windows MSIs` (`:218`) — signs the MSI wrappers.
  4. `Verify Authenticode signatures` (`:236`) — wrapper + loose-exe gate.
  5. **`Verify MSI payload signatures` (`:258`) — CONFIRMED PRESENT.** Does `msiexec /a` admin-extract of each MSI and Authenticode-verifies every payload `.exe` (fail-closed). The `.sys` driver is logged informationally (separate WHQL regime, `:294`). This is the gate that was missing when v0.57.4 shipped unsigned payloads.
- **Publishes (`release:` job, `:436`):** `softprops/action-gh-release@v2`, `draft: false`, `generate_release_notes: true`, files = `*.tar.gz *.zip *.msi *.exe *.deb trusted_root.json SHA256SUMS.txt`. So CI auto-generates release notes; the maintainer's hand-written v2.9 story is added by editing the published release after the run (or via `--notes`).
- **Downstream jobs:** `publish-crates` (crates.io, all 3 lib crates with `--allow-dirty` + 30s indexing sleeps), `update-homebrew-core`, and image-build.yml auto-triggers via `workflow_run`. **Known non-fatal "failures":** crates.io HTTP 303 + homebrew bump are cosmetic fork-job failures (per `project_release_yml_broken` memory) — do NOT treat them as release-blocking.
- **What would break a fresh `v0.58.0` push:**
  1. Missing/empty signing secrets → hard fail at `:124` (intended; D-02 blocker).
  2. `crates/nono-cli/data/windows/nono-wfp-driver.sys` must exist (checked-in pre-signed copy) — `Package` step fails closed at `:193` if absent. Verify it's present before tagging.
  3. The chronic `docker:` reusable-call startup_failure is FIXED (`:451` note) — do not re-add it.

## D-09: deny-`~/.claude` Runner-Profile Fix — STATUS: ALREADY IMPLEMENTED ON `main`

**This is the load-bearing finding. The robust fix the todo prescribed is already committed.** `[VERIFIED: claude_code_hook.rs + git log]`

The pending todo (`2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`) correctly diagnosed that `add_deny_access` is a **Windows NO-OP for the allow-overlap case** — confirmed still true: `validate_deny_overlaps` early-returns unless `cfg!(target_os = "linux")` (`policy.rs:1047`), there is no `Deny` AccessMode reaching the label backend. So a `deny` entry in a profile does NOT block on Windows.

The todo's prescribed **backend-independent solution** (hook refuses to wrap Bash when CWD covers `~/.claude` / project `.claude/`) **is implemented and committed in Phase 60-03**:

- `crates/nono-cli/src/claude_code_hook.rs:203-206` — the Bash arm calls `cwd_self_disable_risk_reason()` (cfg windows) and returns `deny_response` if the CWD is risky, BEFORE emitting any `--allow-cwd` runner command.
- `cwd_self_disable_risk_reason_for` (`:278`) denies when (a) `cwd_covers_home_claude_state` (covers `~/.claude`, is inside it, or covers `~/.claude.json[.lock]`) OR (b) a project-local `.claude/` exists under CWD (`:292`).
- `cwd_covers_home_claude_state` (`:300`) uses `path_covers` (component comparison, not string `starts_with` — closes the CR-01 fail-open per `ddb711dc`).
- **Tests already present** (`:686-725`): home-CWD-covers-`~/.claude` denied; `.claudefoo` NOT falsely matched (component-boundary); repo CWD with project `.claude/` child denied. Plus `pre_tool_use_file_tools_deny` confirms non-Bash tools deny.

The runner profile (`packages/claude-code/claude-code-tools-windows-runner.profile.json`) ALSO keeps the defense-in-depth layer the todo recommended: `extends: default` (NOT `claude-code`, so no `~/.claude` write grant), `"network": { "block": true }`, and `policy.add_deny_access: ["$HOME/.claude", "$WORKDIR/.claude"]` (valuable on macOS / non-overlap cases). `[VERIFIED: profile file]`

The misleading `policy.rs:1038` doc comment the todo flagged is ALSO already corrected — the current comment correctly states "On Windows this is also a no-op today" (`grep "structurally enforceable"` returns nothing). `[VERIFIED]`

### What the planner SHOULD do for D-09 (verify, not build)
1. **Verification task, not implementation.** Confirm the guard at `claude_code_hook.rs:203` and its tests pass on the 0.58.0 build (`cargo test -p nono-cli claude_code_hook` / the existing `cwd_covers_*` tests).
2. **`nono why` deny-overlap proof (per the CONTEXT specific idea):** run a confined scenario with `--allow-cwd` pointed at `~/.claude` and show the Bash wrap is REFUSED (the hook returns `permissionDecision: deny` with the self-disable reason), demonstrating deny wins over `--allow-cwd`. Note this is enforced at the HOOK layer, not the label backend — frame it honestly in release notes (the OS label cannot deny-within-allow on Windows; the hook is the boundary).
3. **Move the pending todo to resolved** (`.planning/todos/`) citing the Phase 60-03 commits (`ddb711dc`, `fe832dfc`, `309c94a4`) as the fix.

**Honest uncertainty:** I did not exercise the runtime `nono why` path or run the tests in this session (Windows live verify is operator-run). The CODE is present and committed; the residual risk is purely "does it still pass on the bumped build" — low. The one open question: D-09's wording says the *profile* must deny regardless of `--allow-cwd`; the actual enforcement is in the *hook*, which only fires when Claude Code is the launcher. A bare `nono run --profile claude-code-tools-windows-runner --allow-cwd ~/.claude` (no hook) would still grant `~/.claude` write on Windows. If the threat model includes direct CLI invocation (not just the hooked Claude loop), that gap remains — flag for the planner to decide if it's in scope or a documented limitation.

## D-07: Superseded v0.57.4 Release — STATUS: DOES NOT EXIST

`gh release view v0.57.4` → **"release not found"**; `gh release list` shows only `v0.57.5` (Latest). `[VERIFIED]`

So the unsigned-payload v0.57.4 release was apparently never published to (or already removed from) the `always-further/nono` remote `gh` is pointed at. D-07 collapses to a **verification + documentation task**: confirm no v0.57.4 (or v0.57.3) GitHub release exposes unsigned-payload MSIs, and record the finding. If a stale release surfaces under a different remote, the command shapes are:

- **Delete (recommended for a distribution hazard):** `gh release delete v0.57.4 --yes` (optionally `--cleanup-tag` to also drop the tag).
- **Annotate/deprecate (if keeping for history):** `gh release edit v0.57.4 --prerelease` or `gh release edit v0.57.4 --notes "DEPRECATED: shipped unsigned-payload MSIs; use v0.58.0+"`.

Recommendation: since it's a security hazard and not on the remote, **delete-if-found**; otherwise document "verified absent". `[VERIFIED for the absent case]`

## D-08: v2.7 Drain Fixes Present on `main`

Confirm these commits are ancestors of the tagged commit (`git merge-base --is-ancestor <sha> HEAD` for each): `d8b7ce00` (broker GLE=87), `005b4c9e` (no-PTY relay stdout-echo), `0cbeb3be` + `b852826b` (WFP service-stop/uninstall). All are referenced as on-`main` in STATE.md/memory. Simple `git log --oneline | grep` or `git merge-base --is-ancestor` verification task. `[CITED: STATE.md Deferred Items + memory]`

## Common Pitfalls

### Pitfall 1: Bumping crate version but forgetting the 6 internal path-dep `version` pins
**What goes wrong:** `cargo publish` (release.yml `publish-crates`) or build version-resolution mismatches; or the MSI/binary reports 0.58.0 while a sibling crate still pins `=0.57.5`.
**How to avoid:** bump all 5 crate `version` literals AND all 6 `version = "..."` pins on internal `path` deps (cli×3, proxy×1, broker×1, ffi×1); regenerate `Cargo.lock`; `cargo build --workspace` to confirm resolution. `[VERIFIED: grep]`

### Pitfall 2: Pushing `v2.9` and expecting/triggering a build
**What goes wrong:** `v2.9` is 2-component — won't match `v*.*.*`; even if forced, `ConvertTo-MsiVersion` throws on <3 components.
**How to avoid:** `v0.58.0` is the build-trigger tag; `v2.9` is a milestone marker only. Push `v0.58.0` first, verify CI, then add `v2.9` on the same commit.

### Pitfall 3: Treating cosmetic crates.io/homebrew job failures as release-blocking
**What goes wrong:** the release IS done once the `release` job publishes signed MSIs; the fork's crates.io (HTTP 303) + homebrew jobs "fail" cosmetically.
**How to avoid:** gate success on the `release` job + MSI payload-signature step, not the whole workflow being green. `[CITED: project_release_yml_broken]`

### Pitfall 4: D-09 framed as a backend deny (it is NOT on Windows)
**What goes wrong:** release notes or a test claiming the OS label denies `~/.claude` — false on Windows (no deny-within-allow). The hook is the boundary.
**How to avoid:** frame the protection as a hook-level fail-closed CWD guard (defense-in-depth), consistent with the honest "POC, not full isolation" framing.

## Runtime State Inventory (release/version-bump phase)

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — release artifacts are build outputs, not stateful stores | None |
| Live service config | GitHub release objects (v0.57.5 Latest); v0.57.4 absent | D-07: verify/delete-if-found |
| OS-registered state | None for the bump itself; installed MSIs register `nono-wfp-service` but that's downstream of install, not the release build | None at release time |
| Secrets/env vars | CI secrets `WINDOWS_SIGNING_CERT`, `WINDOWS_SIGNING_CERT_PASSWORD`, `CARGO_REGISTRY_TOKEN`, `HOMEBREW_CORE_TOKEN` — names unchanged; must be PRESENT (D-02) | Verify present before tagging |
| Build artifacts | `Cargo.lock` (regenerate); generated `dist/windows/*.wxs` (do NOT hand-edit — regenerated by CI from the tag) | Regenerate lockfile; leave .wxs alone |

## Validation Architecture

> `workflow.nyquist_validation` — config not re-read this session; treat section as included (key absent ⇒ enabled).

### Phase Requirements → Test Map
| Req | Behavior | Test Type | Command | Exists? |
|-----|----------|-----------|---------|---------|
| REQ-RLS-04 (D-09) | Hook refuses Bash wrap when CWD covers `~/.claude` / project `.claude/` | unit | `cargo test -p nono-cli --lib claude_code_hook` (covers `cwd_covers_home_claude_state`, `.claudefoo` non-match, project-`.claude` deny) | ✅ on main (60-03) |
| REQ-RLS-04 (D-09) | `nono why` shows `~/.claude` write refused even with `--allow-cwd` aimed at it | manual/UAT | operator: launch confined loop from `%USERPROFILE%\.claude`, attempt Bash write to `settings.json`, observe deny | ✅ behavior present; manual proof |
| REQ-RLS-03 (D-04) | Workspace resolves at 0.58.0 with all path-deps consistent | smoke | `cargo build --workspace` (regenerates lock, validates pins) | ✅ infra |
| REQ-RLS-03 (D-06) | release.yml produces Authenticode-valid MSI wrapper AND payloads | CI gate | release.yml steps `Verify Authenticode signatures` (`:236`) + `Verify MSI payload signatures` (`:258`) | ✅ in CI |
| REQ-RLS-03 (D-06) | Post-release MSI signature spot-check | manual | operator: `Get-AuthenticodeSignature` on the downloaded machine+user MSI + admin-extract payload check | manual |
| REQ-RLS-03 (D-08) | Drain fix commits are ancestors of tag | smoke | `git merge-base --is-ancestor <sha> HEAD` × 4 | infra |

### Sampling Rate
- **Per commit (bump):** `cargo build --workspace` + `cargo test -p nono-cli --lib claude_code_hook`.
- **Phase gate:** D-09 tests green + signing secrets confirmed present, BEFORE the tag push.
- **Post-tag:** watch the `release` job; verify the MSI payload-signature step passed.

### Wave 0 Gaps
- None — D-09 tests already exist (60-03); release.yml signing gates already exist (Phase 53). This phase adds verification + the bump, not new test infra.

## Security Domain

`security_enforcement` assumed enabled.

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V6 Cryptography | yes | Authenticode code signing via CI (`sign-windows-artifacts.ps1`); never hand-roll, never local-cert for public release (D-01/D-02) |
| V14 Config / Build | yes | Fail-closed signing-secret check (`:124`); MSI-payload-signature admin-extract gate (`:258`) |
| V1 Architecture | yes | D-09 hook-level fail-closed CWD guard (deny `~/.claude` exposure) |

| Threat | STRIDE | Mitigation |
|--------|--------|------------|
| Unsigned/tampered MSI payload shipped (the v0.57.4 hazard) | Tampering | Pre-package binary signing + admin-extract payload verification, fail-closed (release.yml) |
| Confined tool jail self-disables by rewriting `~/.claude/settings.json` via `--allow-cwd` | Elevation of Privilege / Tampering | Hook refuses to wrap Bash when CWD covers `~/.claude` or project `.claude/` (`claude_code_hook.rs:203`) |
| Silent fallback to self-signed POC cert for a public release | Spoofing | D-02 hard blocker; CI has no POC-cert path |

## Open Questions (RESOLVED)

1. **`tools/sign-fixture` version.** Is it a 6th versioned crate / does it pin `nono` by `version`? Not grepped this session — planner should confirm and include in the bump if so. `[ASSUMED]` **RESOLVED:** `tools/sign-fixture` is version `0.1.0` with `publish = false` and carries NO `nono` path-dep `version` pin -> it is EXCLUDED from the 0.58.0 lockstep bump (verified on disk; reflected in the 61-01 interfaces block).
2. **D-09 scope — hook vs bare CLI.** The enforcement is hook-only; a bare `nono run --profile claude-code-tools-windows-runner --allow-cwd ~/.claude` would still grant write on Windows (no backend deny-within-allow). Is direct-CLI in the threat model, or is "hooked Claude loop only" the documented boundary? Maintainer call. **RESOLVED:** the enforcement boundary is the hooked Claude Code loop; the bare-CLI `--allow-cwd ~/.claude` write is a DOCUMENTED LIMITATION per D-09, captured in 61-02 (61-D09-VERIFICATION.md) + the v2.9 release notes -- not a Phase 61 code task.
3. **`v2.9` annotation tag** — confirm it's intended as a non-building milestone marker (it cannot trigger/build under the `v*.*.*` + 3-component constraints). Likely yes. **RESOLVED:** `v2.9` is confirmed a NON-BUILDING 2-component milestone annotation tag; only `v0.58.0` (3-component) matches the `v*.*.*` glob and triggers release.yml (D-03).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `tools/sign-fixture` may carry a version literal / nono path-dep pin needing bump | Version-Bump File Set | Build/publish version mismatch if missed |
| A2 | The 4 drain-fix commits are ancestors of HEAD | D-08 | A missing drain fix would regress out of the release |
| A3 | CI signing secrets are still present (signed v0.57.5) | release.yml / D-02 | Release blocker at `:124` if absent |
| A4 | nyquist_validation enabled (config not re-read) | Validation Architecture | None material (section is informational) |

## Sources

### Primary (HIGH confidence)
- `.github/workflows/release.yml` (read in full) — trigger, signing order, payload-verify gate, publish, downstream jobs.
- `scripts/build-windows-msi.ps1` (`ConvertTo-MsiVersion`, `$msiVersion`, `<Package Version=>`) — MSI ProductVersion from tag.
- `crates/nono-cli/src/claude_code_hook.rs` — D-09 hook guard + tests (committed Phase 60-03).
- `crates/nono-cli/src/policy.rs:1030-1049` — `validate_deny_overlaps` Windows no-op; corrected doc comment.
- 5 crate `Cargo.toml` + root `Cargo.toml` — version literals + path-dep pins (grep).
- `packages/claude-code/claude-code-tools-windows-runner.profile.json` — runner profile (network block, add_deny_access, extends default).
- `gh release list` / `gh release view v0.57.4` — release inventory (v0.57.4 absent).
- `git log` / `git tag` — hook commit provenance + tag inventory.
- `.planning/phases/61-.../61-CONTEXT.md` — locked decisions.
- `.planning/todos/pending/2026-05-29-...deny-dotclaude...md` — the D-09 todo.

### Secondary (MEDIUM)
- `.planning/STATE.md` (lines 1-104) — version/MSI state, drain deferrals.
- Memory: `project_release_yml_broken`, `project_sandbox_the_tools`, `windows_msi_wxs_is_generated`, `project_workspace_crates`, `feedback_gh_available`.

## Metadata

**Confidence breakdown:**
- release.yml shape / signing gates: HIGH — read in full.
- Version-bump file set: HIGH — grepped all manifests; one `[ASSUMED]` on sign-fixture.
- D-09 status (already fixed): HIGH — code + tests + commit log all confirm.
- D-07 (v0.57.4 absent): HIGH — `gh` confirmed.
- Live runtime behavior (nono why, test pass on bumped build): MEDIUM — operator-run, code present.

**Research date:** 2026-06-03
**Valid until:** 2026-07-03 (stable release tooling; re-check if release.yml or manifests change before tagging)
