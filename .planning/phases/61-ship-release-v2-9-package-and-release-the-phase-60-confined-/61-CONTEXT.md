# Phase 61: Ship/Release v2.9 - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Package and release the **v2.9 Windows confined-coding-loop** work as a CI-signed public
GitHub release. This phase ships what Phases 60 + 62 built — it does NOT add new product
capability. In scope: version bump, the deny-`~/.claude` runner-profile security fix (folded
in as a pre-ship blocker), CI-signed machine+user MSIs via `release.yml` on a `v*` tag,
milestone tag + push, release notes, and pre-ship cleanup/verification (release.yml
live-verify, superseded-release annotation, v2.7-drain confirmation).

Out of scope: any new sandbox/enforcement feature; the v2.8 UPST7 upstream-sync work
(Phases 53–59); v3.0 deferrals (kernel minifilter, EDR UAT).
</domain>

<decisions>
## Implementation Decisions

### Signing & distribution target
- **D-01:** Release as a **CI-signed public GitHub release** via `release.yml` on a `v*` tag
  push — CI-signed machine+user MSIs published as a GitHub release (matches the ROADMAP
  goal). This is the production path, not the local POC-cert path.
- **D-02:** Assumes the real code-signing cert/secrets are present in CI (they signed
  v0.57.5 per [[project_release_yml_broken]]). If CI signing material is unavailable at
  release time, that is a release blocker to surface — do NOT silently fall back to the
  self-signed POC cert for a public release.
- The local `scripts/sign-poc-local.ps1` / POC-cert (319E507E…) path is NOT the v2.9
  distribution channel (it remains available for internal controlled testing only).

### Version & tag scheme
- **D-03:** Tag the milestone **`v2.9`** AND cut **`v0.58.0`** (dual-tag, mirroring the
  fork's v2.8+v0.57.5 pattern). A minor bump is correct semver for a release carrying two
  features (Phase 60 confined loop + Phase 62 WFP enforcement).
- **D-04:** Bump the workspace to **0.58.0 in lockstep** — all 5 crate `Cargo.toml` versions
  + internal path-dep `version` pins + `Cargo.lock` + MSI ProductVersion, per the established
  bump procedure ([[project_workspace_crates]]; reference quick tasks 260522-di8, 260525-vbp).
  The `.wxs` ProductVersion flows from the build tag — do NOT hand-edit the generated `.wxs`
  ([[windows_msi_wxs_is_generated]]).

### Release scope / binary baseline
- **D-05:** Ship **current `main`** — the release bundles Phase 60 (confined coding loop) +
  Phase 62 (WFP kernel network enforcement, 5/5 UAT PASS) + the untagged post-v2.7 drain
  fixes. The ROADMAP goal's "0.57.5 binaries" wording is stale (predates Phase 62); the
  shipped binaries are the 0.58.0 build off current main.

### Pre-ship readiness (ALL selected — part of this phase)
- **D-06:** **release.yml live-verify** on the `v2.9`/`v0.58.0` tag — confirm the pipeline
  produces signed MSIs cleanly on a fresh tag push. The signing-ORDER + MSI-payload-signature
  gates have churned since the v0.57.5 verification (see [[project_release_yml_broken]] /
  [[project_v28_opened]] Phase 53), so re-verify rather than assume.
- **D-07:** **Annotate or delete the superseded v0.57.4 GitHub release** — it shipped
  unsigned-payload MSIs (distribution hazard carried since v2.8). Resolve as part of shipping
  v2.9 so the latest public artifacts are the only signed ones.
- **D-08:** **Confirm the untagged v2.7 drain fixes are present** in the shipped binaries:
  broker `CreateProcessAsUserW` GLE=87 (`d8b7ce00`), no-PTY relay stdout-echo (`005b4c9e`),
  WFP service-stop/uninstall (`0cbeb3be` + `b852826b`). These were the original v2.8 drain
  intent and must not regress out of the release.

### Folded Todos
- **D-09 (pre-ship security blocker):** Fold in the pending todo **"Runner profile must deny
  `~/.claude` (and project `.claude/`) regardless of `--allow-cwd`"** (captured 2026-05-29).
  Rationale: shipping a confined-tool-mediation POC whose runner profile can be coaxed into
  exposing `~/.claude` (credentials/session state) via `--allow-cwd` is a real security gap in
  exactly what v2.9 distributes. Treat as a release blocker — the deny rule must win over
  `--allow-cwd` in the shipped `claude-code` runner profile. Verify with `nono why` /
  a deny-overlap test before tagging.

### Claude's Discretion
- Exact release-notes structure/wording (must tell the Windows confined tool-mediation story
  + the WFP out-of-box enforcement; honest "POC / defense-in-depth, not full isolation"
  framing per the Phase 60 verdict). The maintainer-facing summary at
  `.planning/quick/260603-e0x-windows-native-development-status-summar/260603-e0x-WINDOWS-STATUS.md`
  is good source material.
- Whether the 0.58.0 version bump is its own plan/wave vs folded into the release-prep plan
  (planner's call).
- Defining the phase's requirement ID (currently "TBD") — propose a `REQ-RLS`-style release
  requirement and register it in REQUIREMENTS.md so the plan has a tracked requirement.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Release pipeline & signing
- `.github/workflows/release.yml` — the CI release pipeline: builds, signs (Authenticode +
  sigstore/TUF), harvests MSIs, publishes the GitHub release on a `v*` tag. The sign-before-MSI-harvest
  order + the `Verify MSI payload signatures` admin-extract gate were added in Phase 53 — preserve them.
- `scripts/build-windows-msi.ps1` — generates the `.wxs` from here-strings and builds machine+user
  MSIs (ProductVersion from the build tag). EDIT THE SCRIPT, never the generated `.wxs`
  ([[windows_msi_wxs_is_generated]]).
- `scripts/validate-windows-msi-contract.ps1` — MSI contract validator (CI gate); keep in lockstep
  with any `build-windows-msi.ps1` change.
- `scripts/sign-poc-local.ps1` — local POC-cert signing flow (NOT the v2.9 channel; reference only).

### Versioning
- `Cargo.toml` (workspace root) + the 5 crate `Cargo.toml` files (`crates/nono`, `crates/nono-cli`,
  `crates/nono-proxy`, `crates/nono-shell-broker`, `bindings/c`) + `Cargo.lock` — lockstep 0.58.0 bump
  ([[project_workspace_crates]]). `CHANGELOG.md` — add the `## [0.58.0]` / v2.9 section.

### Runner-profile security fix (D-09)
- `crates/nono-cli/data/policy.json` — the `claude-code` runner profile policy (deny rules vs
  `--allow-cwd`). Reference todo: `.planning/todos/pending/2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`.
- Related durable gotcha: `add_deny_access` is a Windows NO-OP for the allow-overlap case
  ([[project_sandbox_the_tools]]) — the deny must be enforced where it actually takes effect, not
  assumed from a deny-rule presence.

### Release content / story
- `.planning/STATE.md` — current version/MSI/SHA state, v2.7-drain deferred items, superseded-v0.57.4 note.
- `.planning/PROJECT.md` — v2.9 vs v2.8 milestone framing; Validated requirements.
- `.planning/quick/260603-e0x-windows-native-development-status-summar/260603-e0x-WINDOWS-STATUS.md`
  — maintainer-facing summary; good release-notes source.
- `.planning/phases/62-.../62-HUMAN-UAT.md` + `62-SECURITY.md` — the WFP enforcement validation to cite.
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `release.yml` already produces signed MSIs on `v*` tags (live-verified on the v0.57.5 tag) —
  the release mechanism exists; this phase exercises + re-verifies it, not builds it.
- The lockstep version-bump procedure is well-trodden (quick tasks 260514-0gu, 260522-di8, 260525-vbp).
- `dist/windows/nono-v0.57.12-*.msi` (local POC-signed) already exist as a fallback reference,
  but are NOT the v2.9 public artifacts (D-01).

### Established Patterns
- Dual-tag milestone releases: `v2.X` + `v0.57.X` (e.g., v2.8 + v0.57.5). v2.9 → `v2.9` + `v0.58.0`.
- `.wxs` is generated, not source — bump via build tag, never hand-edit ([[windows_msi_wxs_is_generated]]).

### Integration Points
- A `v*` tag push triggers `release.yml`; the tag IS the release trigger — sequence the bump +
  commit + tag carefully so the tagged commit carries the 0.58.0 version + the D-09 security fix.
</code_context>

<specifics>
## Specific Ideas

- Release-notes story = "Windows confined tool-mediation POC" (Phase 60) + "out-of-box WFP kernel
  network enforcement" (Phase 62), framed honestly as a POC / defense-in-depth (per the Phase 60
  verdict and the §3/§4 framing in the 260603-e0x maintainer summary).
- The deny-`~/.claude` fix should be verified by an actual deny-overlap test (`nono why` showing
  `~/.claude` denied even with `--allow-cwd` pointed at it), not just a policy.json diff.
</specifics>

<deferred>
## Deferred Ideas

- **v2.8 UPST7 upstream sync** (Phases 53–59: JSONC profiles, `target_binary`, `allow_domain`
  path+method, `bw://` creds, session hooks, supervisor IPC) — separate milestone track, not this
  release.
- **v3.0 deferrals** — signed kernel minifilter (Gap 6b), EDR HUMAN-UAT (WR-02).
- **claude.exe full read-grant model under AppContainer** — Phase 62 deferral; not a release task.

### Reviewed Todos (not folded)
None deferred — the one matched todo (deny-`~/.claude`) was FOLDED as D-09.
</deferred>

---

*Phase: 61-ship-release-v2-9-package-and-release-the-phase-60-confined*
*Context gathered: 2026-06-03*
