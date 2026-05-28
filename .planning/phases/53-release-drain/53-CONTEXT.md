# Phase 53: Release & Drain - Context

**Gathered:** 2026-05-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Ship the untagged post-v2.7 fixes as a **real signed release** and clear the
carry-forward debt. Concretely, Phase 53 delivers:

1. A signed v2.8 release: workspace bumped to `0.57.4`, tag `v0.57.4` pushed,
   `release.yml` produces signed (machine + user) MSIs off the post-`005b4c9e`
   binary; an operator installs the MSI and confirms `nono --version` →
   `0.57.4` and the no-PTY supervised path works.
2. `release.yml` verified live — the chronic 0s `startup_failure` is gone and a
   real `v*` tag push runs to completion and produces signed artifacts.
3. WFP elevated live-uninstall UAT closed (REQ-DRN-01).
4. The 3 pending todos resolved or re-dispositioned with committed rationale
   (REQ-DRN-02).

**Out of scope:** any UPST7 audit/cherry-pick work (Phases 54–55), network
filtering, credentials, hooks, IPC robustness (Phases 56–59). Requirements are
fixed by REQUIREMENTS.md — this phase clarifies HOW to release/drain, not what.

</domain>

<decisions>
## Implementation Decisions

### Signing identity (REQ-RLS-01)
- **D-53-01:** v2.8 ships **POC self-signed, CA-ready**. Use the self-signed POC
  signing posture for the actual v2.8 artifacts, but also wire/document the
  commercial-CA / Azure-Trusted-Signing path so a future production swap is a
  **secrets-only change** (no code change). Rejected: blocking v2.8 on acquiring
  a commercial cert.
- **D-53-02:** CI uses a **fresh self-signed cert** generated for this release
  (provenance born in/for CI), NOT a re-export of the existing 5-machine POC
  cert. Consequence: the new public `.cer` MUST be **redistributed** to every
  trusted machine's `LocalMachine\Root` + `LocalMachine\TrustedPublisher`
  stores before they accept v2.8-signed binaries; old installs' trust does not
  carry over. The broker trust gate (`verify_broker_authenticode`, D-32-12)
  passes only when the installed `nono.exe` **and** `nono-shell-broker.exe` are
  signed by a cert chaining to a locally-trusted root.

### Artifact source + CI verification (REQ-RLS-02)
- **D-53-03:** **CI is the artifact source.** `release.yml` on the tag push
  produces the official POC-signed MSIs (GitHub Release artifacts).
  `scripts/sign-poc-local.ps1` is demoted to a **dev/offline fallback only** —
  it is no longer the canonical artifact producer.
- **D-53-04:** The fresh POC PFX is base64'd into the repo secret
  `WINDOWS_SIGNING_CERT` with its password in `WINDOWS_SIGNING_CERT_PASSWORD`
  (the names `release.yml` already checks). Without these, the workflow
  fail-closes at the "Check signing secrets" step — that fail-closed behavior is
  correct and must be preserved. **Security note:** a private key (POC cert)
  lives in repo secrets — acceptable only because it is a self-signed,
  internal-only POC cert, not a production identity.

### Version & tag scheme
- **D-53-05:** Bump workspace `0.57.3 → 0.57.4` across **all 5 crate
  `Cargo.toml` files + their internal path-dep `version` pins** (per the
  5-crate workspace lesson). `nono --version` reports `0.57.4`. Rationale:
  cleanly distinct from the **stale local `v0.57.3` MSIs** (built at `3fbafac2`,
  pre-`005b4c9e`) so the CI artifacts are unambiguously the real release.
- **D-53-06:** **Semver tag drives the build; `v2.8` is a non-firing marker.**
  Push `v0.57.4` to trigger the official signed-artifact build. Refine
  `release.yml`'s `on.push.tags` to a **semver-only glob** (e.g. `v*.*.*`,
  which matches `v0.57.4` but NOT the two-segment `v2.8`) so the `v2.8`
  milestone tag is a pure marker that does not fire a redundant build. This
  refinement folds into the `release.yml` fix work (REQ-RLS-02). The `v2.8`
  milestone tag is still cut at the same commit (dual-tag tradition preserved).

### Drain (REQ-DRN-01 / REQ-DRN-02)
- **D-53-07:** Todo 1 (WFP elevated live-UAT) **IS** the REQ-DRN-01 HUMAN-UAT —
  an operator runs elevated `sc stop nono-wfp-service`, `nono setup
  --uninstall-wfp`, then `msiexec /x`, confirming both `nono-wfp-service` and
  `nono-wfp-driver` are gone with nothing left behind and no residual WFP
  filters/sublayer. This **also** validates the compile-only Fix #2b WiX custom
  action `CaUninstallWfpServices` (`59808e2d`) at runtime, including the
  upgrade-vs-uninstall condition (`NOT UPGRADINGPRODUCTCODE`) and fail-open
  (`Return="ignore"`) behavior. If the deferred type-34 CA fails to launch
  `nono.exe`, fall back to the immediate-CA + `CustomActionData` pattern that
  passes the resolved `[INSTALLFOLDER]`.
- **D-53-08:** Todos 2 + 3 are **promoted to backlog**, not done in-phase, to
  satisfy REQ-DRN-02's "re-dispositioned with committed rationale" clause:
  - Todo 2 (`44-class-d-validator-preflight-investigation`) → REQUIREMENTS v2
    "Deferred" / backlog as a tracked low-priority, Linux-host-gated item.
    Rationale: security equivalence is already proven (the Class D either-or
    assertion proves both branches deny the read); this is a latent-diagnostic
    investigation, not a security gap.
  - Todo 3 (`44-validate-restore-target-fd-relative-hardening`) → REQUIREMENTS
    v2 "Deferred" / backlog as a tracked item carving a **dedicated
    security-scoped phase** with a target window. Rationale: the todo itself
    self-describes as ~2-3 weeks cross-platform (`O_NOFOLLOW` + fd-relative
    `openat`/`mkdirat`/`renameat`/`fchmodat` on Linux/macOS, `NtCreateFile`-
    based or documented defense-in-depth on Windows) and warrants its own phase.
  - After promotion, **close the pending todo files** (move to `todos/done/`)
    with a pointer to the backlog entries so they are roadmap-visible rather
    than loose todos.

### Execution constraints (HUMAN-UAT gating)
- **D-53-09:** Three success criteria are **operator-gated** and cannot be
  completed by the agent shell — they require a real, elevated Windows 11 host:
  - REQ-RLS-01: install the signed MSI, run `nono --version`, confirm `0.57.4`
    and that the no-PTY supervised path (`nono run --profile claude-code -- <bin>
    --version`) exits 0 and prints the version.
  - REQ-RLS-02: push the `v0.57.4` tag, watch the GitHub Actions run to
    completion, confirm signed artifacts.
  - REQ-DRN-01: the elevated WFP stop/uninstall UAT.
  Plans must structure these as explicit HUMAN-UAT checkpoints, not auto-claimed.
- **D-53-10:** Local `main` is **32 commits ahead of `origin/main`** (nothing
  pushed since before the v2.7 fixes). Pushing `main` to `origin` is a
  **mechanical prerequisite** for the tag to be meaningful (`release.yml` checks
  out the tag ref). The push includes the post-v2.7 fixes
  (`d8b7ce00`, `005b4c9e`, `0cbeb3be`, `b852826b`, `59808e2d`) plus the v2.8
  milestone-open planning commits.

### Claude's Discretion
- Exact semver glob pattern for the `release.yml` trigger refinement
  (`v*.*.*` vs an equivalent that excludes two-segment milestone tags) — pick
  whatever cleanly distinguishes `v0.57.4` from `v2.8`.
- Mechanics of generating the fresh CI cert (PowerShell `New-SelfSignedCertificate`
  vs reuse of `sign-poc-local.ps1`'s cert-gen logic) and how the public `.cer`
  is exported for redistribution.
- Backlog file format/location for the promoted todos (REQUIREMENTS.md v2
  section vs a dedicated backlog file) — keep consistent with prior milestone
  practice.

### Folded Todos
All 3 pending todos (matched by `todo.match-phase 53`, all tagged
`resolves_phase: 53`) are folded into this phase's scope:
- **`2026-05-27-wix-auto-uninstall-wfp-custom-action-plus-live-uat.md`**
  (score 0.6) — Fix #2b WiX CA + elevated live-UAT. Folded as the REQ-DRN-01
  work (see D-53-07).
- **`44-class-d-validator-preflight-investigation.md`** (score 0.4) — folded
  for disposition; outcome: **backlog promotion** (see D-53-08).
- **`44-validate-restore-target-fd-relative-hardening.md`** (score 0.4) —
  folded for disposition; outcome: **backlog promotion** (see D-53-08).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` §"Phase 53: Release & Drain" — goal, dependencies,
  5 success criteria.
- `.planning/REQUIREMENTS.md` — REQ-RLS-01, REQ-RLS-02, REQ-DRN-01, REQ-DRN-02
  (and the v2 "Deferred" section where Todos 2+3 will be promoted, plus
  REQ-RLS-ATTEST-01 which may fold in if cheap).
- `.planning/PROJECT.md` — current state (v2.7 shipped, the untagged post-v2.7
  fixes inventory).

### Release pipeline & signing
- `.github/workflows/release.yml` — the workflow to verify/fix; trigger
  (`on.push.tags: 'v*'`), `RELEASE_TAG` derivation (line ~19), signing-secrets
  check (lines ~124–139), MSI build (lines ~141–191), sign + verify steps
  (lines ~196–225).
- `scripts/sign-poc-local.ps1` — self-signed POC signing path (now offline
  fallback per D-53-03); documents the critical EXE-before-MSI signing order
  and the broker trust-gate requirement.
- `scripts/sign-windows-artifacts.ps1` — the CI signing script invoked by
  `release.yml`.
- `scripts/build-windows-msi.ps1` — machine/user MSI builder; hosts the
  `CaUninstallWfpServices` WiX custom action (Fix #2b).
- `docs/cli/development/windows-signing-guide.mdx` — required CI secrets, what
  gets signed, timestamping, the local-POC section, and the CA / Azure Trusted
  Signing production path (the "CA-ready" target of D-53-01). NOTE: this file is
  in the gitignored-but-tracked `docs/cli/development/` tree — edits need
  `git add -f`.

### Drain / WFP uninstall
- `.planning/debug/resolved/wfp-service-stop-uninstall.md` — the debug session
  whose remaining live-verify leg REQ-DRN-01 closes (Fix #1 `0cbeb3be`,
  Fix #2a `b852826b`, Fix #2b `59808e2d`).
- `.planning/todos/pending/2026-05-27-wix-auto-uninstall-wfp-custom-action-plus-live-uat.md`
  — todo 1 (REQ-DRN-01 work; exact UAT steps + the immediate-CA fallback).
- `.planning/todos/pending/44-class-d-validator-preflight-investigation.md`
  — todo 2 (backlog promotion).
- `.planning/todos/pending/44-validate-restore-target-fd-relative-hardening.md`
  — todo 3 (backlog promotion).
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — WFP service (Fix #1
  SERVICE_CONTROL_STOP handling).
- `crates/nono-cli/src/setup.rs` — `nono setup --uninstall-wfp` (Fix #2a).

### Versioning
- `Cargo.toml` (workspace) + all 5 crate `Cargo.toml` files — `0.57.3 → 0.57.4`
  bump targets, including internal path-dep `version` pins (5-crate workspace
  lesson — CLAUDE.md is stale claiming 3).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `sign-poc-local.ps1` already implements self-signed cert generation, the
  EXE-then-MSI signing order, and `.cer` export — reuse its cert-gen logic to
  produce the fresh CI cert (D-53-02) and as the offline fallback (D-53-03).
- `release.yml` already has the full sign → verify → upload pipeline with
  fail-closed secrets gating; the work is supplying the secret + fixing the
  startup-failure + refining the trigger, not building the pipeline.
- The `CaUninstallWfpServices` WiX custom action already exists in
  `build-windows-msi.ps1` (compile-validated at `59808e2d`); REQ-DRN-01 UAT
  validates it rather than authoring it.

### Established Patterns
- Dual-tag scheme: `v2.X` milestone tag + `v0.57.X` semver tag at the same
  commit (precedent: `v2.7` + `v0.57.2` at `637a426c`). D-53-06 preserves the
  tradition but makes only the semver tag fire CI.
- Fail-secure on missing signing secrets (release.yml D-13) — never upload
  unsigned artifacts. Preserve.
- Broker self-trust-anchor gate (D-32-12): unsigned Program Files install
  refuses to spawn the broker. The fresh-cert `.cer` redistribution (D-53-02)
  is what makes the signed v2.8 install pass this gate.

### Integration Points
- `release.yml` ↔ repo secrets (`WINDOWS_SIGNING_CERT*`) ↔ the fresh POC cert.
- The bumped `0.57.4` version ↔ MSI filenames (`RELEASE_TAG`) ↔ `nono --version`
  output asserted in REQ-RLS-01 UAT.
- Pushed `main` (32 commits) ↔ the `v0.57.4` tag ref that `release.yml` checks
  out.

</code_context>

<specifics>
## Specific Ideas

- The "doubly-broken" v2.7 no-PTY path is fixed by BOTH `d8b7ce00` (broker
  HANDLE_LIST dedup, `CreateProcessAsUserW` GLE=87) and `005b4c9e` (no-PTY relay
  stdout echo). REQ-RLS-01 UAT must confirm BOTH are present in the released
  binary — `nono run --profile claude-code -- <bin> --version` must exit 0 AND
  print the version.
- REQ-RLS-ATTEST-01 (`actions/attest-build-provenance`) is v2-deferred but "may
  fold into REQ-RLS-02 if cheap" — planner may evaluate opportunistically once
  `release.yml` is healthy; not required.

</specifics>

<deferred>
## Deferred Ideas

- **Commercial CA / Azure Trusted Signing production identity** — wired/
  documented this phase (CA-ready, D-53-01) but the actual cert acquisition +
  swap is deferred. Future: secrets-only change.
- **Todo 2 — Linux validator-preflight investigation** — promoted to backlog
  (D-53-08); Linux-host-gated, low priority, security equivalence proven.
- **Todo 3 — `validate_restore_target` fd-relative TOCTOU hardening** —
  promoted to backlog as a dedicated future security-scoped phase (D-53-08);
  ~2-3 weeks cross-platform.
- **Broader heavy-runtime audit (REQ-WSRH-AUDIT-01)** — already v2-deferred in
  REQUIREMENTS.md; not touched here.

### Reviewed Todos (not folded)
None — all 3 matched todos were folded (1 as work, 2 as backlog-promotion
dispositions).

</deferred>

---

*Phase: 53-release-drain*
*Context gathered: 2026-05-28*
