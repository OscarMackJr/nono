# Phase 85: UPST9 Divergence Audit - Research

**Researched:** 2026-06-19
**Domain:** Upstream divergence audit — `always-further/nono` `v0.62.0..v0.64.0`
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Dispositions recorded at **cluster level** (one disposition + one ADR-style L/M/H
  risk verdict per theme A–M), each with a **nested per-commit SHA inventory** listing every
  substantive commit folded into that cluster. Matches the Phase 42/47/48 shape.
- **D-02:** A `split` cluster disposition is the mechanism for mixed-fate themes: per-commit
  inventory note carries carve-out annotation. No separate per-commit disposition column.
- **D-03:** Themes **A & B** are **locked** to `will-sync / adopt-upstream`. Not re-litigated.
- **D-04:** Pre-lean dispositions, diff-inspection confirms or overturns:
  - C → lean `will-sync`; F → lean `split`; M → lean `split`; D/H/I/K/L → lean `will-sync`;
    E/G → lean `will-sync` with Windows-path reconciliation note.
- **D-05:** Full actual-diff re-export inspection targeted at shared-surface clusters: A, B,
  diagnostic-touching surfaces (B's FFI + proxy `ProxyDiagnostic` + `error.rs`), and F.
  `--name-only` is sufficient for clearly-additive feature clusters (D/H/I/K + dep bumps).
  Per-cluster, the ledger states which inspection depth was applied.
- **D-06:** ~55 non-substantive commits documented in an explicit "Excluded as noise" section
  with filter criteria, count, and SHAs. Independently verifiable.
- **D-07:** Window stays `v0.62.0..v0.64.0`. Re-fetch at audit-open is a tip-check formality;
  note result either way. No `v0.65.0` exists at discuss-time (verified).

### Claude's Discretion

- Exact column layout / table format (follow Phase 42/47/48 convention reconstructed from
  SEED-006 and the success criteria).
- How to bucket/order the noise SHAs (ranges vs enumerated — whichever is most legible).

### Deferred Ideas (OUT OF SCOPE)

- Full boundary-convergence ADR (rationale + what stays CLI-side) → Phase 86 (BND-03).
- Actual cherry-pick / code relocation for any theme → Phases 86–89.
- Crate version leapfrog to ≥ 0.65.0 → release-time.
- Four reviewed todos (MSI VC++ prereq, POC-cert broker, macOS RLIMIT_AS, Phase 83 code-review)
  — not folded (scope creep).

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AUDIT-01 | `DIVERGENCE-LEDGER.md` exists for `v0.62.0..v0.64.0` (Phase 42/47/48 shape) classifying every substantive commit into themed clusters A–M, each with `will-sync` / `fork-preserve` / `split` / `won't-sync` disposition; re-fetched at audit-open to capture any `v0.65.0`. | Ledger format reconstructed from prior UPST7/UPST8 ledgers (git history); re-fetch protocol verified; window confirmed at 35 substantive / 55 noise commits. |
| AUDIT-02 | Each cluster carries an ADR-style L/M/H risk verdict; cross-cluster re-export deps diff-inspected (not `--name-only`), closing the `feedback_cluster_isolation_invalid` hazard. | Five-dimension risk rubric documented; diff-inspection command set specified; shared-surface clusters A/B/F identified and flagged for actual-diff scrutiny. |

</phase_requirements>

---

## Summary

Phase 85 produces a single documentation artifact: `DIVERGENCE-LEDGER.md` for upstream
`always-further/nono` `v0.62.0..v0.64.0`. This research provides the executor with everything
needed to write that artifact correctly: the canonical ledger format (reconstructed from UPST7
Phase 54 and UPST8 Phase 69 ledgers recovered from git history), the exact commit enumeration
method (drift tool + SHA-safe invocation), the noise exclusion filter criteria, the five-dimension
ADR risk rubric, the diff-inspection command set for re-export hazard closure, and fork-state
ground truth for themes A & B (the two locked `will-sync` clusters).

The most important discovery is the **UPST7 and UPST8 ledger formats** recovered directly from
git history commits `07b0a23d` (Phase 54, 40 commits, 14 clusters) and `0e212834` (Phase 69, 9
commits, 4 clusters). These provide the exact table schema, section ordering, frontmatter keys,
and narrative pattern the planner must replicate.

The second critical finding is the **exact commit window**: `git fetch upstream` was run, confirms
upstream/main is at `0153757001d21805a8218213e32add462d3322a1` which equals the upstream `v0.64.0`
tag — no `v0.65.0` exists. The drift tool reports **35 substantive commits** (not the 90-total or
13-theme-preview from SEED-006; the discrepancy is explained below). The 90-count is the full
`git log` total including merges and non-path-filtered commits.

**Primary recommendation:** Follow the Phase 54 UPST7 ledger shape exactly, adapted for 13 themes
(A–M from SEED-006) instead of 14 clusters. The executor runs the drift tool with `--from 52809dda
--to 01537570` (SHA-safe, avoids tag collision), produces JSON output to `ci-logs-local/drift/`,
then classifies all 35 substantive commits into A–M, writes the ledger, and performs actual-diff
re-export inspection on clusters A, B, and F.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Divergence ledger authorship | Documentation | — | Executor writes `.md`, runs `git`/`gh` diff commands; no source modifications |
| Commit enumeration (drift tool) | CLI/scripts | git | `scripts/check-upstream-drift.sh` is the canonical tool; git rev-list for validation |
| Re-export hazard inspection | git show | — | `git show <sha>` on lead commits; executor reads diffs for `pub use`/`pub mod` additions |
| Fork-state verification | Codebase grep | — | `grep`/`ls` on the live tree to ground divergence claims |
| Upstream tag tip-check | git ls-remote | — | Read-only; run before opening the ledger |

---

## Research Priority 1: Ledger Table Schema (Reconstructed)

The Phase 54 (UPST7) and Phase 69 (UPST8) `DIVERGENCE-LEDGER.md` files were recovered from git
history (commits `07b0a23d` and `0e212834`). [VERIFIED: git history — live repo].

### YAML Frontmatter Keys

```yaml
---
phase: 85-upst9-divergence-audit
plan: 01
ledger_type: upst9-audit
range: 52809dda..01537570 (v0.62.0..v0.64.0)
range_note: "<optional; note any window extension>"
upstream_head_at_audit: <sha from git ls-remote at audit-open>
refetch_date: <date of re-fetch>
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'make check-upstream-drift ARGS="--from 52809dda --to 01537570 --format json"'
fork_baseline: v0.62.0 (upstream SHA 52809dda; Phase 69 UPST8 sync point)
total_unique_commits: 35
date: <execution date>
---
```

### Section Order (matches Phase 54 UPST7 shape)

1. `## Headline` — paragraph summary: total commits, cluster count, disposition breakdown
   (`will-sync N / fork-preserve N / split N / won't-sync N`), `windows-touch:yes` clusters,
   re-export scan result, ADR review outcome
2. `## Reproduction` — verbatim invocation, JSON output path, upstream_head_at_audit,
   refetch_date, drift_tool sha pin, total_unique_commits, SHA collision guard note,
   auditor-rerun instructions
3. `## Cluster Summary` — markdown table:
   `| cluster_id | theme | commits | disposition | windows-touch | rationale |`
4. `### Cluster A: <name>` through `### Cluster M: <name>` (or however many SEED-006 themes
   are non-empty after merge/split) — each cluster section contains:
   - `**Commits:** N — <comma-separated subjects>`
   - `**Disposition:** will-sync | fork-preserve | split | won't-sync`
   - `**Windows-touch:** yes | no`
   - `**Rationale:** <narrative>`
   - `**Cross-cluster re-export check:** <actual-diff result or "placeholder—Task N">`
   - Per-commit table:
     `| sha | subject | upstream-tag | categories | files-changed | windows-touch |`
5. `## ADR review` — per-dimension L/M/H with narrative (see Risk Rubric below)
6. `## Empirical cross-check` — ≥4 spot-check files with git log walk results
7. `## Cross-cluster re-export deps detected` — summary of actual-diff findings
8. `## Excluded as noise` — (see D-06 protocol) [ASSUMED: naming; prior ledgers used inline
   cluster non-entries; UPST9 D-06 requires an explicit named section]
9. Any theme-specific assessment section (e.g., `## TLS-intercept assessment` for theme F,
   per Phase 54 UPST7 precedent which had `## TLS-intercept clean-apply assessment`)

### Per-Commit Table Schema

Columns exactly as in prior ledgers:

```
| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| <8-char sha> | <commit subject> | <v0.63.0 or v0.64.0> | <drift-tool categories> | <N> | yes/no |
```

- `sha`: 8-char abbreviated (matches drift tool output)
- `upstream-tag`: which upstream release contains this commit
- `categories`: from drift tool `"categories"` array (profile, policy, package, proxy, audit, other)
- `files-changed`: count from `git show <sha> --stat | tail -1`
- `windows-touch`: `yes` if commit touches `*_windows.rs`, `exec_strategy_windows/`, WFP, registry;
  `no` otherwise. [VERIFIED: git history — prior ledgers use this exact definition]

### File Location

Per `<specifics>` in CONTEXT.md, the ledger lives at:
`.planning/phases/85-upst9-divergence-audit/DIVERGENCE-LEDGER.md`

Naming convention note: prior ledgers used a prefixed name (`54-DIVERGENCE-LEDGER.md`,
`69-DIVERGENCE-LEDGER.md`). The CONTEXT.md `<specifics>` suggests either
`DIVERGENCE-LEDGER.md` or `85-DIVERGENCE-LEDGER.md` — planner to confirm against SDK conventions.
The UPST7 and UPST8 ledgers used the prefixed form. [VERIFIED: git history].

---

## Research Priority 2: Commit-Window Enumeration Method

### SHA Collision Guard (CRITICAL)

The local fork tag `v0.62.0` resolves to `3c5e902570c394483e360102268e53111d93d139` (a fork release
commit on divergent history). Upstream's actual `v0.62.0` is `52809dda3b9ec5d7a237c26ac5e90840052993d9`,
verified at Phase 69 close and confirmed as still correct (the tag has not moved). [VERIFIED: git
history + git rev-parse].

**NEVER use `--from v0.62.0` (tag name) in git commands.** Always use `--from 52809dda`.
**NEVER use `--to v0.64.0` (tag name).** Always use `--to 01537570`. [ASSUMED: v0.64.0 tag name
collision status — the fork may or may not have a local v0.64.0 tag; executor MUST verify with
`git rev-parse v0.64.0` before assuming the SHA is safe].

### Step 1: Re-fetch (SC#2 / D-07 tip-check)

```bash
# Branches-only (avoids tag collision on v0.62.0 and potentially v0.63.0/v0.64.0)
git fetch upstream

# Verify upstream/main SHA
git rev-parse upstream/main

# Check for any new tags past v0.64.0
git ls-remote upstream 'refs/tags/v0.65*' 'refs/tags/v0.66*'
```

**At research time:** `git fetch upstream` was run (2026-06-19). Result: upstream/main =
`0153757001d21805a8218213e32add462d3322a1` = v0.64.0 tag SHA. No v0.65.0 exists.
`git ls-remote upstream 'refs/tags/v0.65*'` returned empty. D-07 window extension does NOT fire.
[VERIFIED: live git remote — ls-remote confirmed].

Record the re-fetch date and upstream_head_at_audit in the ledger frontmatter regardless.

### Step 2: Verify SHAs are locally available

```bash
git cat-file -t 52809dda   # expect: commit (upstream v0.62.0)
git cat-file -t 01537570   # expect: commit (upstream v0.64.0)
```

At research time: both SHAs exist locally after `git fetch upstream`. [VERIFIED: git cat-file].

### Step 3: Run the drift tool (canonical substantive-commit enumeration)

```bash
# Windows host (PowerShell/Git Bash):
bash scripts/check-upstream-drift.sh --from 52809dda --to 01537570 --format json \
  > ci-logs-local/drift/$(date -u +%Y%m%dT%H%M%SZ)-v062-v064-upst9.json

# Or via Makefile:
make check-upstream-drift ARGS="--from 52809dda --to 01537570 --format json"
```

**Drift tool sha pin** (assert before run):
```bash
git log -1 --format=%H -- scripts/check-upstream-drift.sh
# Expected: 0834aa664fbaf4c5e41af5debece292992211559
```

### Step 4: Verify commit counts

```bash
# Total commits in window (including merges, docs, CI, noise):
git log --oneline 52809dda..01537570 | wc -l   # expect: ~90

# Merge commits only:
git log --merges --oneline 52809dda..01537570 | wc -l   # expect: ~23

# Non-merge commits:
git log --no-merges --oneline 52809dda..01537570 | wc -l  # expect: ~67

# Substantive (drift-tool path filter):
git log --no-merges --oneline 52809dda..01537570 \
  -- crates/nono/src/ crates/nono-cli/src/ crates/nono-proxy/src/ crates/nono/Cargo.toml \
  ':(exclude)*_windows.rs' ':(exclude)crates/nono-cli/src/exec_strategy_windows/' | wc -l
# expect: 35
```

[VERIFIED: live git — these exact counts confirmed at research time].

### Confirmed Window Breakdown

| Category | Count | Source |
|----------|-------|--------|
| Total commits in window | 90 | git log count (matches SEED-006 claim) |
| Merge commits | 23 | git log --merges |
| Non-merge commits | 67 | git log --no-merges |
| **Substantive (drift-tool filter)** | **35** | drift tool path filter |
| Noise (non-merge, out-of-filter) | 32 | 67 − 35 |
| Noise (merge + out-of-filter non-merge) | 55 | 90 − 35 |

SEED-006 stated "~13 substantive" — this was a preview count of the themed engineering clusters
(A–M), not the per-commit count. The drift tool's 35 represents the actual per-commit total with
path filtering applied. [VERIFIED: drift tool run + git counts].

### The 35 Substantive Commits (Canonical List)

Drift tool output from `--from 52809dda --to 01537570`, run at research time [VERIFIED: live]:

```
01537570  chore: release v0.64.0 (#1201)
bd4b6b7f  refactor(proxy): separate proxy intent from activation (#1199)
4179ce03  fix(pty): ctrl-z hangs when running with a PTY (#1135)
a6aa5995  feat(diagnostics): expose structured diagnostics for library and FFI clients (#1171)
cc11b389  feat(update-check): discover ci environments on update (#1113)
724bb207  fix: proxy should activate with customCredentials set (#1197)
76b7b695  Refactor forward_inner_request (#1192)
5bb098cd  feat: [aws] implement aws_auth config (#1166)
8e0d94f9  fix(cli): use XDG config paths consistently (#1179)
a0bba5eb  feat(output): show blocked macos grants in capability summary (#1178)
0b27cfc2  refactor(audit): move attestation logic to core library (#1148)
b0b2c743  fix(proxy): stop allow_domain endpoint route from shadowing credential catch-all (#1132)
b5f8db5c  fix(proxy): respect upstream_proxy in TLS CONNECT intercept path (#1048) (#1091)
7f319b9e  fix(diagnostic): replace deprecated nono learn with nono run (#1170)
a5d623fd  fix(proxy): return 403 + audit for denied non-CONNECT requests (#1077)
084638b7  chore: release v0.63.0 (#1161)
4ad8ba92  refactor(diagnostic): move diagnostic UX out of core nono crate (#1155)
e8293b36  feat(cli): move runtime state to XDG state dirs (#1152)
7c9abd3b  fix(proxy): keep connection open for reactive proxy auth on CONNECT (#1151)
6d452b00  chore(deps): bump typify from 0.6.2 to 0.7.0 (#1156)
f867aba2  fix: report the actual blocked operation instead of the readable target path (#1150)
e2086877  fix(linux): trap sendto/sendmsg to prevent AF_UNIX datagram bypass (#1096)
42e5bf73  fix(cli): accept truthy env values for bool flags (#1136)
7d274cf7  feat: add $PACK_DIR support to session_hooks for store pack support (#1073)
7e076d2d  refactor(pull_ui): remove sigstore provenance display (#1144)
c6b13345  feat(keyring): add NONO_KEYRING_TIMEOUT_SECS for keychain access (#977)
d48aeb7b  feat(environment): add set_vars for static env injection (#1134)
e9529312  fix(audit): address ledger review and clippy
6d88638e  refactor(profiles): standardize profile names with namespace
ee7a3bda  fix: replace stale nono.dev schema domains with nono.sh
aed35bec  refactor(audit-ledger): move audit ledger logic to library crate
a5b2a516  refactor(audit): move audit integrity logic to nono crate
6b3eb013    fix: guard deduplicate() against inheriting procfs-remap originals (#1064)
9800f307  feat(pack-verification): skip pack verification on dry runs
e54cf9cb  Remove env_clear from the session_hook subprocess
```

---

## Research Priority 3: Noise-Commit Exclusion (D-06)

### Exclusion Filter Criteria

The drift tool applies these path filters (D-11 from the tool's source):

**Included paths:**
- `crates/nono/src/`
- `crates/nono-cli/src/`
- `crates/nono-proxy/src/`
- `crates/nono/Cargo.toml`

**Excluded patterns (even if under included paths):**
- `*_windows.rs`
- `crates/nono-cli/src/exec_strategy_windows/`

Any commit not touching the included paths (after exclusion) is **noise** for this audit.

**Additional noise categories** (always noise regardless of path filter):
- Merge commits (`git log --merges`)
- `docs(*)` commits touching only `.md`/`.sh`/`.json` outside source paths
- `chore(deps)` bumps touching only `Cargo.lock` and non-`crates/nono/Cargo.toml` manifests
- CI configuration changes (`.github/workflows/`, `.github/ISSUE_TEMPLATE/`)
- Build tooling (COPR, AUR, packaging scripts)
- Test-only commits touching `tests/integration/` without touching `crates/`

### The 55 Noise Commits (for "Excluded as noise" section)

**23 merge commits** (all excluded — git treats merges as non-unique per drift tool design):
`849cda42`, `c3dc6a42`, `a9f3aefe`, `bb01eb1c`, `0d066b60`, `46f7b23e`, `addc818a`, `172ffffe`,
`a1726551`, `a4709b2e`, `dd518afb`, `2d96b45c`, `e8d16b62`, `4d2359da`, `2f996687`, `6490c209`,
`40f29137`, `e566a067`, `e1d977b9`, `bfb29c1a`, `42b2b783`, `b91d45aa`, `6deb1664`

**32 non-merge, out-of-filter-path commits:**
`89c3785f` (ci: ubuntu integration tests),
`5413a0b3` (fix(policy): go_runtime go-build cache — touches `crates/nono-cli/data/policy.json`
  only, which is excluded from the drift tool path filter),
`79090c2e` (cbindgen bump `0.29.3→0.29.4`), `23b457b8` (which bump `8.0.2→8.0.3`),
`aad901f6` (docs(allow-cwd)), `e06f4628` (chore: 3-day Dependabot cooldown),
`4bced737` (docs: credential-injection anchor), `b24c72fc` (docs: networking),
`3f8d24ad` (docs: install), `2f4af7d2` (docs: quickstart link), `8bf56083` (chore: agents.md),
`a793fe2f` (ignore bump `0.4.25→0.4.26`), `d603e103` (chrono bump `0.4.44→0.4.45`),
`d2fc3a6b` (time bump `0.3.47→0.3.49`), `cca68d99` (zeroize bump `1.8.2→1.9.0`),
`5a34a459` (chore: issue template), `f7836bbe` (docs: macOS diagnostics.suppress_system_services),
`54c551eb` (docs: readme update), `ec76e15f` (docs: readme refine), `ad4a26d6` (docs: readme link),
`bf547151` (readme improvement), `196f911b` (ci: actions/attest), `314bd74e` (test: WSL2 landlock
  V4+ detection — touches `tests/integration/test_wsl2.sh` only),
`abae954e` (docs: copr install instructions), `bba57157` (chore: actions/checkout bump),
`5a257179` (fix: cargo vendor copr srpms), `2b374525` (build: copr source rpm packaging),
`1be86838` (fix(aur): ssh-keyscan), `d1fc7d69` (cbindgen bump `0.29.2→0.29.3`),
`a8ccd160` (hyper bump `1.9.0→1.10.1`), `6181a1f9` (docs: cli-quickstart),
`e58de4d6` (x509-parser bump `0.16.0→0.18.1`)

**Note on dep bump SHAs:** The Cargo dep bumps (`a793fe2f`, `d603e103`, `d2fc3a6b`, `cca68d99`,
`79090c2e`, `23b457b8`, `d1fc7d69`, `a8ccd160`, `e58de4d6`) touch only `Cargo.lock` and
non-core-crate manifests (e.g., `crates/nono-cli/Cargo.toml` or other crate `Cargo.toml` files
that are NOT `crates/nono/Cargo.toml`). They are excluded from the drift tool. The dep version
targets themselves are captured in SEED-006's dependency-bump table and are absorb targets for
Phase 88 (DEPS-02). Recording them in the "Excluded as noise" section fulfills D-06 completeness.

---

## Research Priority 4: Diff-Inspection Commands (Re-export Hazard)

The `feedback_cluster_isolation_invalid` hazard states: a commit's file list (`--name-only`) does
not reveal whether it adds `pub use` or `pub mod` statements that pull in symbols from other
clusters. Actual diff inspection of added lines is required for shared-surface clusters.
[VERIFIED: memory — canonical project hazard, enforced by AUDIT-02].

### Commands for Actual-Diff Inspection

```bash
# Full diff for a single commit (shows added/removed lines, not just file names)
git show <sha>

# Grep for re-export additions only (new pub use / pub mod / extern crate lines)
git show <sha> | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'

# Show only files changed (the insufficient name-only approach — NEVER rely on this alone)
git show <sha> --name-only
```

### Target Clusters for Actual-Diff Inspection (D-05)

| Cluster | Reason | Lead Commits to Inspect |
|---------|--------|------------------------|
| A | 1773 LOC into `crates/nono/src/audit.rs`; lib.rs re-exports | `a5b2a516`, `aed35bec`, `0b27cfc2`, `e9529312` |
| B | New `crates/nono/src/diagnostic/` module; `error.rs` +137 LOC; FFI `bindings/c/src/diagnostic.rs` | `4ad8ba92`, `a6aa5995`, `f867aba2` |
| F (proxy) | Fork-divergent TLS surface (Phase 34 C11 `fork-preserve`); `tls_intercept/` vs fork's `route.rs` | `b5f8db5c`, `76b7b695`, `bd4b6b7f`, `724bb207` |

For clusters D, H, I, K, L, and the release-bump rows in M: `--name-only` is sufficient (D-05).

### Re-export Scan Pattern (from Phase 54 UPST7 precedent)

For each lead commit in A/B/F:
1. Run `git show <sha>` — read all `+` lines for `pub use`, `pub mod`, `pub(crate)` additions
2. Record: "Clean — no cross-cluster re-export dep" or "Detected: <symbol> from <module> ..."
3. If a function-call dep is detected (as in UPST7 C5→C3), record as a **prerequisite** for the
   downstream phase that cherry-picks the caller

### Phase 34 C11 `fork-preserve` Context for Cluster F

The fork does NOT carry `crates/nono-proxy/src/tls_intercept/` — upstream's TLS intercept module.
The fork instead carries `route.rs` + `connect.rs` + `credential.rs` + `reverse.rs` as its proxy
surface. [VERIFIED: live tree — `ls crates/nono-proxy/src/` confirms no `tls_intercept/` dir].

For commits `b5f8db5c` (upstream_proxy in TLS CONNECT) and `76b7b695` (Refactor
forward_inner_request): the diff-inspect must determine which lines touch `tls_intercept/` (absent
in fork → won't-apply) vs which lines touch fork-shared files (`server.rs`, `credential.rs`,
`route.rs`). This is the same pattern as UPST7 Phase 54 C5 (`22e6c40`).

For `724bb207` (proxy activates with customCredentials): touches `credential.rs` and
`credential.rs load_with_diagnostics` — fork carries `credential.rs`, so this is a shared surface.
Diff-inspect to determine conflict level against the fork's Windows credential-injection rewrite.

---

## Research Priority 5: Cluster-to-Fork-State Grounding

### Theme A: Audit/Attestation MOVED INTO core `nono` crate

**Fork state verified at research time** [VERIFIED: live tree]:
- `crates/nono/src/audit.rs` — DOES NOT EXIST (confirmed `ls crates/nono/src/`)
- `crates/nono-cli/src/audit_attestation.rs` — EXISTS
- `crates/nono-cli/src/audit_commands.rs` — EXISTS
- `crates/nono-cli/src/audit_integrity.rs` — EXISTS
- `crates/nono-cli/src/audit_session.rs` — EXISTS
- `crates/nono/src/lib.rs` re-exports: NO audit re-export (confirmed by reading `lib.rs`)

**Divergence confirmed:** Fork has audit CLI-side; upstream has moved ~1773 LOC to
`crates/nono/src/audit.rs`. Theme A divergence is REAL and the adopt-upstream (D-03 lock) is
the deliberate choice to converge. [VERIFIED: live tree].

**Windows-touch:** `no` — the upstream audit commits touch `crates/nono/src/audit.rs` and the
CLI audit stubs; neither is Windows-specific. [VERIFIED: git show for each A commit].

### Theme B: Structured Diagnostics Model

**Fork state verified at research time** [VERIFIED: live tree]:
- `crates/nono/src/diagnostic.rs` — EXISTS; contains `DiagnosticFormatter`, `DenialRecord`,
  `IpcDenialRecord`, `SandboxViolation`, `CommandContext`, `DenialReason`, `DiagnosticMode`
- `crates/nono/src/lib.rs` re-exports: `pub use diagnostic::{CommandContext, DenialReason,
  DenialRecord, DiagnosticFormatter, DiagnosticMode, IpcDenialRecord, SandboxViolation};`
- `crates/nono-cli/src/diagnostic_formatter.rs` — EXISTS (CLI-side profile-resolver footer;
  distinct from the library `DiagnosticFormatter` above)
- `crates/nono-proxy/src/diagnostic.rs` — DOES NOT EXIST
- `bindings/c/src/diagnostic.rs` — DOES NOT EXIST

**Critical upstream structure:** Upstream's theme B introduces `crates/nono/src/diagnostic/`
as a **module directory** (subdirs: `codes.rs`, `observation.rs`, `records.rs`, `report.rs`,
`detail.rs`, `mod.rs`) — replacing/expanding the fork's single `diagnostic.rs` file.
The upstream also adds `NonoError::{diagnostic_code, remediation}` to `error.rs`.

**Reconciliation complexity:** The fork's `diagnostic.rs` contains the fork's existing
`DiagnosticFormatter` and associated types. Upstream's B refactor is a superset — it moves
diagnostic UX OUT of the core crate in one commit (`4ad8ba92`) and then adds a RICHER structured
model back (`a6aa5995`). The fork executor must reconcile the fork's current `diagnostic.rs`
surface with the new module layout. [VERIFIED: live tree + git show].

**Windows-touch:** `yes` — theme B's `error.rs` changes and FFI additions (`bindings/c/src/`)
touch surfaces that have Windows-relevant diagnostic paths. The fork's Windows diagnostic output
(denial messages, sandbox event logging) references types from `diagnostic.rs`. The ledger must
flag `windows-touch: yes` for cluster B.

### Theme C: Linux AF_UNIX + procfs-remap

**Fork state:** `crates/nono/src/sandbox/linux.rs` exists; `crates/nono/src/capability.rs`
exists; `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` exists.
[VERIFIED: live tree — `ls` confirmed].

**Cross-target clippy required** for cluster C: the commits touch `#[cfg(target_os = "linux")]`
blocks. Per CLAUDE.md MUST/NEVER rule and `.planning/templates/cross-target-verify-checklist.md`,
any verification must run `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND
`--target x86_64-apple-darwin`. On Windows dev host without cross-toolchain, mark REQ PARTIAL →
deferred-to-CI. Record this in theme C's ledger risk verdict.

**Windows-touch:** `no` — both commits (`e2086877`, `6b3eb013`) are `cfg(linux)` only.
[VERIFIED: SEED-006 + git show commit subjects].

### Theme E: XDG State Dirs

**Fork state:** `crates/nono-cli/src/state_paths.rs` — DOES NOT EXIST. [VERIFIED: live tree].
The upstream introduces `state_paths.rs` (+422 LOC). The fork currently resolves runtime paths
inline across audit, session, rollback modules.

**Windows reconciliation note (D-04):** The `user_state_dir` function in upstream uses XDG
conventions (`$XDG_STATE_HOME` / `~/.local/state`). The fork's Windows v3.0 scratch-space
provisioner resolves to `%LOCALAPPDATA%\nono\`. The ledger must record this reconciliation need.

### Theme M: env_clear Removal Collision

**The collision** [VERIFIED: git show e54cf9cb + live tree]:
- Upstream `e54cf9cb` removes `cmd.env_clear()` from `crates/nono-cli/src/hook_runtime.rs` only
  (not from `exec_strategy.rs`). The commit's rationale: "session hooks run on the host,
  outside the sandbox; no security boundary to enforce."
- The fork's `hook_runtime.rs` (Unix path) contains `cmd.env_clear()` at line 196.
- The fork's `hook_runtime_windows.rs` contains `cmd.env_clear()` at lines 196 and 301, and
  RE-ADDS `SystemRoot`, `windir`, `SystemDrive` after `env_clear()` to prevent CLR init failure
  (`0xFFFF0000` / `-65536` — documented in `windows_hook_interpreter_spawn_gotchas`).

**Why `split` is the correct disposition for theme M:**
The upstream's removal of `env_clear` from `hook_runtime.rs` can be absorbed for the Unix path
(`hook_runtime.rs`). But the Windows path (`hook_runtime_windows.rs`) needs its `env_clear` +
SystemRoot-restore sequence kept — the removal breaks CLR startup. The per-commit inventory for
theme M must annotate `e54cf9cb` as "apply to `hook_runtime.rs` (Unix); SKIP for
`hook_runtime_windows.rs`."

### Theme J: PTY ctrl-z fix

**Fork state:** `crates/nono-cli/src/pty_proxy.rs` exists; grep for `leave_screen_for_suspension`
returns no match — the PTY ctrl-z fix (`4179ce03`) is NOT in the fork yet. [VERIFIED: live tree].

### Features not yet in fork (confirmed at research time)

| Feature | Theme | File | Status |
|---------|-------|------|--------|
| `NONO_KEYRING_TIMEOUT_SECS` | H | `crates/nono/src/keystore.rs` | Missing — grep confirms |
| `state_paths.rs` | E | `crates/nono-cli/src/state_paths.rs` | Missing — `ls` confirms |
| PTY ctrl-z fix | J | `crates/nono-cli/src/pty_proxy.rs` | Missing — grep confirms |
| `detect_ci_provider` | K | `crates/nono-cli/src/update_check.rs` | Missing — grep confirms |
| `set_vars` in `validate_set_vars` | D | `crates/nono-cli/src/exec_strategy/env_sanitization.rs` | Missing — grep confirms |

[VERIFIED: live tree grep + ls].

---

## Research Priority 6: Five-Dimension ADR Risk Rubric

Per AUDIT-02 and Phase 54 UPST7 precedent, each cluster disposition carries an ADR-style L/M/H
verdict across five dimensions. This rubric makes verdicts consistent and auditable.
[VERIFIED: git history — UPST7 ledger § ADR review].

### Dimension Definitions and Scoring Guide

| Dimension | L (Low) | M (Medium) | H (High) |
|-----------|---------|------------|----------|
| **security** | Cross-platform additive feature; no security-relevant surface | Modifies auth/credential/audit surface; non-security-critical path | Closes a real security hole (bypass, credential leak) OR introduces security-critical primitive into core crate |
| **windows** | Zero Windows-touching files; no Windows-specific behavior change | Modifies cross-platform file with Windows-aware logic; needs Windows-path reconciliation | Touches Windows-specific module, WFP, broker, or IL/DACL surface; may break Windows behavior |
| **maintenance** | Purely additive; fork absorbs it cleanly; future upstream updates in this area remain easy | Some adaptation needed (fork-local pattern must be updated); moderate forward-maintenance risk | Major structural change that creates an ongoing fork-vs-upstream divergence point; hard to track in future audits |
| **divergence** | Will-sync, clean absorption; reduces total fork divergence | Split/partial absorption; leaves a controlled fork-specific surface | Fork-preserve or won't-sync of a large upstream surface; increases tracked divergence |
| **contributor** | Absorption with no or minimal fork-specific adaptation required; low PR friction | Absorption with documented reconciliation steps; moderate PR effort | Requires ADR-level decision (boundary change, policy change); high human review overhead |

### Per-Cluster Pre-scored Leanings (for ledger drafting)

| Cluster | Theme | security | windows | maintenance | divergence | contributor | Overall |
|---------|-------|----------|---------|-------------|------------|-------------|---------|
| A | Audit→core | H | L | H | H | H | H |
| B | Diagnostics→core+FFI | M | H | H | H | H | H |
| C | AF_UNIX + procfs | H | L | L | L | L | H (security dominant) |
| D | set_vars | L | L | L | L | L | L |
| E | XDG state dirs | L | M | M | L | L | M |
| F | Proxy hardening | M | L | M | M | M | M |
| G | AWS auth | L | L | L | L | L | L |
| H | Keyring timeout | L | L | L | L | L | L |
| I | $PACK_DIR hooks | L | L | L | L | L | L |
| J | PTY ctrl-z | L | L | L | L | L | L |
| K | update-check CI | L | L | L | L | L | L |
| L | Profile namespace | L | L | L | L | L | L |
| M | Misc fixes (split) | L | M | L | L | L | M (due to env_clear collision) |
| Release bumps | Embedded in clusters | L | L | L | L | L | L |
| Dep bumps | Separate cluster | L | L | L | L | L | L |

**ADR review section narrative template** (matches Phase 54 UPST7):
```
## ADR review

| dimension | score | notes |
|-----------|-------|-------|
| security  | H/M/L | <1-sentence rationale> |
| windows   | H/M/L | <1-sentence rationale> |
| maintenance | H/M/L | <1-sentence rationale> |
| divergence | H/M/L | <1-sentence rationale> |
| contributor | H/M/L | <1-sentence rationale> |

**Outcome:** (a) Confirm. Phase 33 ADR Option A 'continue' remains valid — [1-2 sentence summary].
```

---

## Standard Stack

### Core (audit production tools)

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| `scripts/check-upstream-drift.sh` | SHA `0834aa66` | Canonical substantive-commit enumeration | Project standard since Phase 47; drift-tool-sha-pin in every ledger |
| `git log --no-merges --format=...` | system git | Raw commit log for noise extraction | Standard git; no external dep |
| `git show <sha>` | system git | Actual diff for re-export inspection | Required by D-05 / AUDIT-02 |
| `git ls-remote upstream` | system git | Upstream tip-check (D-07) | Required by SC#2 |

### Output Format

The ledger is a single Markdown file with YAML frontmatter. No tooling required for generation —
the executor writes it using the Write tool based on diff output and analysis.

---

## Architecture Patterns

### System Architecture Diagram

```
upstream/main (remote)
    |
    v
git fetch upstream   [Step 1: tip-check]
    |
    v
git ls-remote upstream 'refs/tags/v0.65*'  [Step 2: window extension check]
    |
    v  (D-07: no v0.65.0, window stays 52809dda..01537570)
    |
    v
drift tool: --from 52809dda --to 01537570  [Step 3: enumerate 35 substantive commits]
    |
    +---> JSON output (ci-logs-local/drift/)  [gitignored, not committed]
    |
    v
theme A-M classification  [map 35 commits to SEED-006 themes]
    |
    +---> [per-commit table for each cluster]
    |
    v
actual-diff inspection (clusters A, B, F)  [git show <sha> | grep pub use]
    |
    v
noise exclusion section (55 commits documented)
    |
    v
DIVERGENCE-LEDGER.md  [the deliverable]
```

### Recommended Cluster Grouping for UPST9

Based on SEED-006 mapping and the 35-commit drift tool output, the 35 substantive commits map
to approximately 11–14 clusters:

| Cluster ID | Theme | Commits | Disposition |
|------------|-------|---------|-------------|
| A | Audit→core (3+1 review) | `a5b2a516`, `aed35bec`, `0b27cfc2`, `e9529312` | will-sync (locked D-03) |
| B | Diagnostics→core+FFI | `4ad8ba92`, `a6aa5995`, `f867aba2` | will-sync (locked D-03) |
| C | AF_UNIX bypass + procfs | `e2086877`, `6b3eb013` | lean will-sync (D-04) |
| D | set_vars env injection | `d48aeb7b` | lean will-sync (D-04) |
| E | XDG state dirs | `e8293b36`, `8e0d94f9` | lean will-sync (D-04) |
| F | Proxy hardening | `b0b2c743`, `a5d623fd`, `b5f8db5c`, `7c9abd3b`, `76b7b695`, `bd4b6b7f`, `724bb207` | lean split (D-04) |
| G | AWS auth | `5bb098cd` | lean will-sync (D-04) |
| H | Keyring timeout | `c6b13345` | lean will-sync (D-04) |
| I | $PACK_DIR hooks | `7d274cf7` | lean will-sync (D-04) |
| J | PTY ctrl-z | `4179ce03` | lean will-sync (additive) |
| K | update-check CI + misc CI | `cc11b389` | lean will-sync (D-04) |
| L | Profile namespace | `6d88638e` | lean will-sync (D-04) |
| M | Misc fixes | `42e5bf73`, `a0bba5eb`, `ee7a3bda`, `7e076d2d`, `9800f307`, `e54cf9cb` | lean split (D-04) |
| Release/dep-bumps | Won't-sync (version mgmt) | `084638b7`, `01537570`, `6d452b00` | won't-sync |

Note: `7f319b9e` (fix(diagnostic): replace deprecated nono learn) is in the drift output but not
in SEED-006. It touches `crates/nono-cli/src/` diagnostic surfaces — fold into cluster B or M.
[ASSUMED: exact folding; executor must classify based on file inspection].

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Commit enumeration | Custom git rev-list script | `scripts/check-upstream-drift.sh` | Drift tool handles path filter, categorization, merge exclusion, JSON output; re-invented from scratch would drift from prior ledgers |
| Re-export detection | Manual file reading | `git show <sha> \| grep '^+'` | Grep on actual diff additions is the established pattern (Phase 54) |
| Noise SHA list | Deriving from scratch | Research priority 3 above | Pre-enumerated; executor can use the 55-SHA list directly |
| Upstream tag verification | Assuming tag names | `git ls-remote upstream refs/tags/v0.6*` | SHA collision (local `v0.62.0` ≠ upstream `v0.62.0`) is live; always verify via ls-remote |

---

## Common Pitfalls

### Pitfall 1: Tag Name SHA Collision

**What goes wrong:** Using `git log v0.62.0..v0.64.0` or drift tool `--from v0.62.0` produces
garbage (~1889 commits) because the local fork tag `v0.62.0` = `3c5e9025` (fork release, divergent
history) and does NOT match upstream's `v0.62.0` = `52809dda`.
**Why it happens:** The fork's version leapfrog strategy (Project_v28 lesson) sets fork tags
past upstream's highest; the UPST8 audit created the local `v0.62.0` tag for the fork's release.
**How to avoid:** Always use explicit SHAs: `--from 52809dda --to 01537570`.
**Warning signs:** `git log --count` returns implausibly large numbers (>50 for the known window).
[VERIFIED: UPST8 ledger § Reproduction, confirmed at research time].

### Pitfall 2: Counting 90 as Substantive

**What goes wrong:** SEED-006 says "90 commits, 140 files" — someone mistakes 90 for substantive.
**Why it happens:** SEED-006 counted the full window total, not the drift-tool-filtered set.
**How to avoid:** Drift tool on the SHA-safe range returns 35 substantive commits. Use 35 as
`total_unique_commits`. The 90 is the full `git log` count (merges + docs + CI + all noise).

### Pitfall 3: Skipping Actual-Diff on Theme B FFI

**What goes wrong:** Marking theme B's FFI additions as "clean" based on `--name-only`, missing
that `bindings/c/src/lib.rs` has new `pub extern "C"` functions that reference `diagnostic`
module types. If the downstream Phase 86 cherry-pick lands without these being mapped, the FFI
exhaustive-match arms (the `nono-ffi` milestone gate lesson) will break.
**Why it happens:** `bindings/c/src/` is in the drift tool's excluded-windows exclusion list
only for `*_windows.rs` — the FFI files (`lib.rs`, `types.rs`, `diagnostic.rs`) ARE in scope
for the filter but `bindings/c/` is NOT in the drift tool's path list at all. Check manually.
**How to avoid:** For cluster B, explicitly run `git show a6aa5995 -- bindings/c/src/` to see
FFI surface additions.

### Pitfall 4: Missing `7f319b9e` Classification

**What goes wrong:** `7f319b9e` (`fix(diagnostic): replace deprecated nono learn with nono run`)
appears in the drift tool output but is NOT listed in SEED-006. If the executor only follows
SEED-006's theme list, this commit is silently dropped — violating AUDIT-01's "every substantive
commit is classified" requirement.
**Why it happens:** SEED-006 was a preview, not a complete audit. The drift tool is authoritative.
**How to avoid:** After running the drift tool, cross-check each of the 35 SHAs against SEED-006.
Any SHA not in SEED-006 must be folded into the most appropriate existing cluster (B for
diagnostic, M for misc, etc.) with an inline note.

### Pitfall 5: env_clear Removal (Theme M) Applied to Windows Path

**What goes wrong:** `e54cf9cb` removes `env_clear()` from `hook_runtime.rs` (Unix). If cherry-
picked blindly, it also appears to invalidate the Windows equivalent. But `hook_runtime_windows.rs`
uses `env_clear()` + SystemRoot/windir/SystemDrive re-add to prevent CLR init failure. A blind
apply would break the Windows hook path.
**Why it happens:** The commit touches `hook_runtime.rs` not `hook_runtime_windows.rs` — the
Windows file is safe. But the ledger disposition must note this explicitly so the Phase 88
executor does not infer "applies to both hook_runtime files."
**How to avoid:** The theme M ledger entry must annotate `e54cf9cb` as "apply to
`hook_runtime.rs` (Unix path only); `hook_runtime_windows.rs` retains `env_clear()` + baseline
restore per `windows_hook_interpreter_spawn_gotchas`."

---

## Validation Architecture

Per `.planning/config.json` — nyquist_validation not explicitly disabled, treat as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) |
| Config file | `Makefile` targets |
| Quick run command | N/A — this phase produces documentation only |
| Full suite command | `make ci` (clippy + fmt + tests) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AUDIT-01 | DIVERGENCE-LEDGER.md exists, all 35 substantive commits classified | Manual review | Ledger reviewed by plan-checker | ✅ N/A (doc artifact) |
| AUDIT-02 | ADR risk verdicts present; actual-diff re-export inspection documented per cluster | Manual review | Plan-checker reads ledger sections | ✅ N/A (doc artifact) |

**This phase is documentation-only.** Verification is by plan-checker reading the ledger artifact,
not automated test execution. `make ci` is not a gate for this phase; the gate is the Phase 85
plan verification check. [VERIFIED: CONTEXT.md § Phase Boundary — "no code is cherry-picked,
moved, or modified"].

### Wave 0 Gaps

None — no test infrastructure needs to be created for this phase.

---

## Runtime State Inventory

**Phase type:** Documentation/audit artifact only. Not a rename/refactor/migration phase.
This section is SKIPPED. [VERIFIED: CONTEXT.md § Phase Boundary].

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `git` | All diff/log commands | ✓ | system git (Windows) | — |
| `upstream` remote | Re-fetch, ls-remote | ✓ | `https://github.com/always-further/nono.git` | — |
| `scripts/check-upstream-drift.sh` | Drift tool (SHA `0834aa66`) | ✓ | SHA `0834aa66` confirmed | PowerShell twin `check-upstream-drift.ps1` |
| `bash` | Drift tool (`.sh` variant) | ✓ | Git Bash on Windows | Use `.ps1` twin |
| Upstream `v0.62.0` SHA | Range lower bound | ✓ | `52809dda` exists locally (post `git fetch`) | — |
| Upstream `v0.64.0` SHA | Range upper bound | ✓ | `01537570` exists locally (post `git fetch`) | — |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** The `.sh` drift tool requires `bash` — available via
Git Bash on Windows; PowerShell twin is the fallback.

---

## Security Domain

This phase produces a documentation artifact — no new code, no new attack surface. The security
domain assessment for runtime capabilities applies to Phase 86–89 (the cherry-pick phases), not
Phase 85.

**Applicable ASVS categories for this phase:** None (docs-only phase).

The security-relevant finding for downstream phases:
- Theme C (`e2086877`) is explicitly a security fix (AF_UNIX datagram bypass). Its ledger risk
  verdict must rate `security: H`. The downstream phase (87) will need ASVS V5 (input validation)
  and the cross-target clippy gate.

---

## Project Constraints (from CLAUDE.md)

| Directive | Impact on Phase 85 |
|-----------|-------------------|
| DCO sign-off required on all commits | The ledger commit must include `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` |
| No `.unwrap()` or `.expect()` | N/A — no Rust code written in this phase |
| Cross-target clippy for cfg-gated Unix edits | N/A for this phase; recorded in theme C's risk verdict for Phase 87 |
| `--workspace --all-targets` gate (not `--bin nono`) | N/A — no code gate for this phase |
| Repo must stay PUBLIC | Ensure no `build_notes/` or `.gsd/` files are staged before any git push |
| Library is policy-free-primitive | Themes A & B deliberately change this; locked `will-sync` per D-03; ADR in Phase 86 |
| `make ci` after every session | N/A — no source changes; standard docs commit workflow |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The ledger file should be named `DIVERGENCE-LEDGER.md` or `85-DIVERGENCE-LEDGER.md` (CONTEXT.md says "planner to confirm") | Research Priority 1, File Location | Low — only affects filename; prior ledgers used prefixed form |
| A2 | `7f319b9e` folds into cluster B or M (not in SEED-006; assigned by executor judgment) | Research Priority 6, cluster grouping | Low — classification is auditable; executor documents the fold |
| A3 | Local fork tag `v0.64.0` does NOT exist (avoiding collision) — executor must verify `git rev-parse v0.64.0` before drift tool run | Research Priority 2, SHA Collision Guard | Medium — if tag exists and collides, `--to v0.64.0` would be incorrect; use SHA `01537570` unconditionally |
| A4 | The cluster grouping in Research Priority 6 maps all 35 drift-tool commits correctly | Architecture Patterns, cluster table | Low — draft only; executor diff-inspects each SHA and may split or merge clusters |

**If this table were empty:** All claims in this research were verified or cited. The few assumptions
above are low-risk drafting choices that the executor resolves during ledger production.

---

## Open Questions

1. **`7f319b9e` cluster assignment**
   - What we know: `7f319b9e` (`fix(diagnostic): replace deprecated nono learn with nono run`)
     appears in the drift tool output; it touches diagnostic CLI surfaces
   - What's unclear: Should it fold into cluster B (diagnostic model) or M (misc fixes)?
   - Recommendation: Inspect `git show 7f319b9e --name-only`; if it touches only
     `crates/nono-cli/src/` files already in B's scope, fold into B; if it's a standalone
     CLI diagnostic string change, fold into M.

2. **`5413a0b3` (go_runtime policy.json) noise classification**
   - What we know: Touches `crates/nono-cli/data/policy.json` — outside drift tool's path filter
   - What's unclear: SEED-006 theme M lists it; drift tool excludes it. Is it noise or substantive?
   - Recommendation: It is noise per the drift tool's D-11 path filter (policy.json ≠
     crates/nono-cli/src/). Record it in the noise section. The go-build cache group change would
     be absorbed as part of a future policy.json sync pass, not UPST9.

3. **Whether `bd4b6b7f` (refactor: separate proxy intent from activation) is theme F or standalone**
   - What we know: It's in the drift tool output; SEED-006 lists it under cluster F
   - What's unclear: Does it create a prerequisite dependency (like UPST7 C5→C3)?
   - Recommendation: Diff-inspect as part of the cluster F actual-diff pass.

---

## Sources

### Primary (HIGH confidence)

- git history (`07b0a23d`, `0e212834`) — UPST7 and UPST8 divergence ledgers recovered verbatim;
  exact table schema, section order, frontmatter keys, narrative pattern confirmed
- Live git repo (`git fetch upstream`, `git ls-remote`, `scripts/check-upstream-drift.sh`) —
  35 substantive commits enumerated; SHA collision guard confirmed; upstream/main = v0.64.0 verified
- `.planning/phases/85-upst9-divergence-audit/85-CONTEXT.md` — decisions D-01 through D-07
- `.planning/seeds/SEED-006-upst9-v0.62-v0.64-sync-window.md` — theme A–M decomposition
- `CLAUDE.md` — library boundary table, coding standards, cross-target clippy MUST/NEVER
- `.planning/templates/cross-target-verify-checklist.md` — PARTIAL disposition protocol
- Live tree inspection (`ls`, `grep`) — fork state for themes A, B, E, J, K, M confirmed

### Secondary (MEDIUM confidence)

- Memory `feedback_cluster_isolation_invalid` — re-export hazard canonical description
- Memory `windows_hook_interpreter_spawn_gotchas` — env_clear / CLR init failure documented
- `.planning/REQUIREMENTS.md` — AUDIT-01, AUDIT-02 acceptance criteria
- `.planning/ROADMAP.md` — Phase 85 success criteria

### Tertiary (LOW confidence)

- SEED-006 cluster-to-commit mapping — used as a pre-classification guide; the drift tool output
  is authoritative and may differ in a few entries (e.g., `7f319b9e`)

---

## Metadata

**Confidence breakdown:**
- Ledger format: HIGH — recovered from two prior UPST ledgers in git history
- Commit enumeration: HIGH — drift tool run live; exact 35-commit list confirmed
- Fork state grounding (A, B, E, J, K, M): HIGH — live tree verified
- Noise exclusion list: HIGH — derived from live git log comparison
- ADR risk rubric: MEDIUM — adapted from Phase 54 UPST7 precedent; consistent with prior use

**Research date:** 2026-06-19
**Valid until:** 2026-07-19 (30 days for stable documentation research; upstream `v0.65.0` cut
would re-trigger tip-check per D-07)
