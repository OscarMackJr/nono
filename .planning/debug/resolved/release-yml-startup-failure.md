---
slug: release-yml-startup-failure
status: resolved
trigger: |
  release.yml startup_failure on tag push. Every push run completes in 0s with
  conclusion `failure`, "This run likely failed because of a workflow file issue",
  and zero jobs executed. Confirmed at v2.7 ship (pushed v2.7 + v0.57.2 tags
  2026-05-26 → two release.yml runs, both 0s startup_failure). Chronic per
  `gh run list --workflow=release.yml` back to at least 2026-04-27 (through every
  milestone close v2.3/v2.4/v2.5/v2.6/v2.7).
created: 2026-05-26
updated: 2026-05-26
---

# Debug Session: release.yml startup_failure on tag push

## Symptoms

- **Expected:** pushing a `v*` tag runs the Release workflow → build matrix →
  Create Release (signed MSIs, tarballs, checksums) → docker/crates/homebrew jobs.
- **Actual:** the run fails at startup in 0s, no jobs execute, GitHub says
  "This run likely failed because of a workflow file issue."
- **Scope:** every push/tag for months; no automated release has ever been produced.
  All MSIs to date are locally-built and unsigned (see [[project_release_yml_broken]]).
- **Local YAML validity:** `python -c "import yaml; yaml.safe_load(...)"` PASSES —
  so it is NOT a plain syntax error; it is an Actions-level validation failure.

## Root Cause (CONFIRMED — high confidence)

release.yml's `docker` job (lines 384-393) invokes a **reusable workflow**:

```yaml
  docker:
    needs: release
    uses: ./.github/workflows/image-build.yml   # reusable-workflow call
    with: { ref: ..., version: ..., push: true }
    secrets: inherit
```

But `image-build.yml` declares only:

```yaml
on:
  workflow_run:
    workflows: ["Release"]
    types: [completed]
  workflow_dispatch:
    inputs: { ref, version, push }
```

It has **NO `on: workflow_call:` trigger**. GitHub Actions requires a called
(`uses:`) workflow to define `workflow_call`. A `uses:` pointing at a workflow that
lacks `workflow_call` (or whose `workflow_call` inputs don't match the `with:` keys)
is rejected during workflow VALIDATION — which fails the ENTIRE calling workflow at
startup (0s, no jobs, "workflow file issue"). This is the exact observed symptom and
it is static in the file, hence chronic.

**Compounding fact:** the `docker` job is also REDUNDANT. image-build.yml already
auto-runs via `workflow_run: workflows: ["Release"]` when Release completes. So
Docker image builds are already wired post-release; the reusable-call `docker` job
is a contradictory leftover that both (a) breaks startup and (b) duplicates intent.

## Evidence

- timestamp: 2026-05-26
  finding: |
    release.yml:388 `uses: ./.github/workflows/image-build.yml` is the ONLY `uses: ./`
    in the file (grep confirmed). image-build.yml:3-23 `on:` block has workflow_run +
    workflow_dispatch only, NO workflow_call. release.yml `needs:` graph (lines 315,
    386, 397, 435): build←release←{docker, publish-crates, update-homebrew-core};
    NOTHING has `needs: docker`, so the docker job is a removable leaf.

## Fix Options

- **A (recommended): remove the `docker` job from release.yml (lines 384-393).**
  Restores startup; Docker still builds via image-build.yml's existing `workflow_run`
  trigger after Release completes. Smallest change, removes the contradiction, no
  dependency breakage (docker is a leaf).
- **B: add `on: workflow_call:` (inputs ref/version/push) to image-build.yml.**
  Makes the reusable call valid, BUT image-build.yml would then fire on both
  workflow_call (inside release) AND workflow_run (after release) → double Docker
  builds unless the workflow_run trigger is also removed/guarded. Larger surface.

## Resolution

root_cause: "release.yml docker job uses: a reusable workflow (image-build.yml) that has no on: workflow_call trigger; invalid reusable-workflow reference fails the whole workflow at startup validation (0s, no jobs)."
fix: "Option A (user-selected): removed the docker reusable-call job from release.yml. Docker image publishing remains wired via image-build.yml's existing workflow_run: workflows: [Release] trigger (auto-runs after Release completes). Left a guard comment in place of the job to prevent re-adding the broken reusable call."
verification: "Local: `python -c import yaml` parses release.yml OK; jobs now build/release/publish-crates/update-homebrew-core (docker removed); no `uses: ./` job refs remain (only inside the guard comment); no `needs: docker` dangling. Definitive verification is the NEXT v* tag push reaching the build matrix instead of 0s startup_failure (actionlint not installed locally to pre-validate reusable-workflow resolution)."
files_changed: [".github/workflows/release.yml"]
