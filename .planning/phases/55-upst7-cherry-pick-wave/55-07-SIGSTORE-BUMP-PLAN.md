---
phase: 55-upst7-cherry-pick-wave
plan: 07
type: execute
wave: 5
depends_on: [55-02, 55-05, 55-06]
files_modified:
  - Cargo.lock
  - crates/nono-cli/Cargo.toml
  - crates/nono/Cargo.toml
  - crates/nono/src/scrub.rs
autonomous: true
requirements: [REQ-UPST7-02]
must_haves:
  truths:
    - "55-07-C13-DISPOSITION-RESOLUTION.md exists and documents the diff-inspection outcome (CLEAR or COLLISION) for the scrub.rs change vs the fork's Phase-49 trust-root surface"
    - "crates/nono/Cargo.toml sigstore-verify and sigstore-sign are bumped to 0.8.0"
    - "crates/nono-cli/Cargo.toml sigstore dependencies are bumped to 0.8.0"
    - "Cargo.lock is regenerated (workspace-wide) to reflect the sigstore 0.8.0 transitive dependency set"
    - "scrub.rs is ported (CLEAR outcome) or D-20 replayed / deferred (COLLISION outcome) per the disposition resolution"
    - "D-32-15 verify-is-offline invariant is NOT regressed: trusted_root.json is read via plain JSON deserialization (not TUF re-verification)"
    - "The Cargo bump and scrub.rs change each carry a verbatim D-19 trailer (or the scrub.rs commit carries Upstream-replayed-from: for D-20 replay)"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
  artifacts:
    - path: ".planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md"
      provides: "C13 diff-inspection artifact: upstream scrub.rs vs fork Phase-49 surface; upgrade-or-replay verdict"
    - path: "crates/nono/Cargo.toml"
      provides: "sigstore 0.8.0 bump"
    - path: "crates/nono/src/scrub.rs"
      provides: "Cow deref adjustment (CLEAR) or D-20 replay / scrub.rs held (COLLISION)"
    - path: "Cargo.lock"
      provides: "workspace-wide lockfile updated for sigstore 0.8.0 transitive deps"
  key_links:
    - from: "scrub.rs"
      to: ".planning/phases/49-sigstore-trust-root-poc-resilience-from-file-flag-release-as/49-CONTEXT.md"
      via: "D-55-02 diff-inspection: upstream Cow deref vs fork --from-file / trusted_root.json surface"
      pattern: "from_file|trusted_root|scrub"
---

<objective>
Cherry-pick cluster C13 (split) — sigstore 0.8.0 dep bump. This is the security-sensitive cluster requiring diff-inspection-first before any commit lands.

Wave 5: depends on C7 (55-02) for Cargo.lock/Cargo.toml compatibility and on C11 (55-05) + C12 (55-06) completing first to ensure the full baseline is stable before the sigstore bump lands.

The D-55-02 decision locks the diff-inspection-first posture:
1. Produce `55-07-C13-DISPOSITION-RESOLUTION.md` — compare upstream `e581569`'s `scrub.rs` change (minor Cow deref adjustments per the commit message) against the fork's Phase-49 trust-root surface.
2. If CLEAR: port both `crates/nono/Cargo.toml` bump + `scrub.rs` with D-19 trailer.
3. If COLLISION (D-32-15 offline-verify regression detected or fork's --from-file / trusted_root.json logic is disturbed): D-20 manual-replay the scrub.rs intent (`Upstream-replayed-from:` trailer) or hold scrub.rs; Cargo bump still lands.

The sigstore 0.8.0 Cargo bump ripples Cargo.lock workspace-wide (all 5 crates per D-55-E5). reqwest also bumps to 0.13.3 per the upstream commit message.

Purpose: Land the sigstore 0.8.0 dependency bump with verified scrub.rs safety.
Output: The C13 disposition-resolution artifact + 1 commit (or 2 if scrub.rs is a separate D-20 replay) on the held feature branch.
</objective>

<execution_context>
@C:\Users\OMack\.claude\get-shit-done\workflows\execute-plan.md
@C:\Users\OMack\.claude\get-shit-done\templates\summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/STATE.md
@.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md
@.planning/templates/upstream-sync-quick.md
@.planning/phases/55-upst7-cherry-pick-wave/55-CONTEXT.md
@.planning/phases/49-sigstore-trust-root-poc-resilience-from-file-flag-release-as/49-CONTEXT.md

<interfaces>
<!-- C13 commits (split cluster) -->
C13 commits:
  e581569  — chore(deps): update sigstore crates to 0.8.0  (v0.58.0)
  Files: Cargo.lock (large: +329/-41 lines), crates/nono-cli/Cargo.toml, crates/nono/Cargo.toml,
         crates/nono/src/scrub.rs (+4/-4 lines — Cow deref adjustments)

D-55-02 split intent:
  - Cargo.toml bumps (both nono + nono-cli) → will-sync straight port
  - scrub.rs → diff-inspect-first then port-if-clear / replay-if-collision

Phase-49 trust-root surface (binding for the diff-inspection):
  - crates/nono/src/scrub.rs (fork's current version)
  - .planning/phases/49-.../49-CONTEXT.md (--from-file, fixture cadence, trusted_root.json)
  - D-32-15 invariant: cached trusted_root.json read via plain JSON deserialization (NOT TUF re-verification)

Upstream scrub.rs change (from commit message): "Make minor Cow dereferencing adjustments in src/scrub.rs"
This is a small mechanical change — 4 lines removed, 4 lines added. The diff-inspection should be
quick: does the Cow deref change affect the fork's --from-file path or the trusted_root.json JSON
deserialization path?

D-55-E5 (5-crate workspace): The sigstore bump is in crates/nono/Cargo.toml + crates/nono-cli/Cargo.toml.
The other 3 crates (nono-proxy, nono-shell-broker, bindings/c) do not use sigstore directly — verify
after the bump that `cargo build --workspace` succeeds for all 5.

reqwest 0.13.3 transitive bump: this is a large Cargo.lock change (+329/-41). Run `cargo build --workspace`
to confirm the full workspace builds after the lockfile update.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Diff-inspection — produce 55-07-C13-DISPOSITION-RESOLUTION.md</name>
  <files>.planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md</files>
  <read_first>
    - crates/nono/src/scrub.rs (full current fork state — understand the fork's scrub/verification logic)
    - .planning/phases/49-sigstore-trust-root-poc-resilience-from-file-flag-release-as/49-CONTEXT.md (full read — Phase-49 trust-root surface: --from-file, fixture cadence, trusted_root.json, D-32-15 invariant)
    - .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md § Cluster C13
    - .planning/phases/55-upst7-cherry-pick-wave/55-CONTEXT.md decision D-55-02
    - git show e581569 (full diff — inspect the exact scrub.rs Cow deref lines changed)
  </read_first>
  <action>
Perform the D-55-02 diff-inspection and produce the resolution artifact.

Step 1: Get the exact upstream scrub.rs diff:
  `git show e581569 -- crates/nono/src/scrub.rs`
  
Step 2: Read the fork's current crates/nono/src/scrub.rs to identify:
  a. The --from-file code path (Phase-49 addition)
  b. The trusted_root.json JSON deserialization path (D-32-15 invariant)
  c. Any sigstore API call sites that the Cow deref change touches

Step 3: Determine whether the upstream Cow deref adjustment:
  - CLEARS the D-32-15 invariant: the change is mechanical (Cow::Borrowed → &str derefs) and does NOT alter the fork's --from-file path, fixture cadence, or trusted_root.json deserialization flow → verdict: CLEAR, port both Cargo bump and scrub.rs
  - COLLIDES with the D-32-15 invariant: the change modifies the verification code path in a way that would change how trusted_root.json is read or verified → verdict: COLLISION, D-20 manual-replay or hold scrub.rs

Step 4: Write `55-07-C13-DISPOSITION-RESOLUTION.md` in the Phase 48 D-48-C2 naming format. The artifact MUST contain:
  - Upstream commit SHA: e581569
  - Upstream scrub.rs diff lines (verbatim from `git show`)
  - Fork's scrub.rs Phase-49 surface description (the --from-file path, trusted_root.json deserialization, D-32-15 invariant)
  - Collision analysis: per-line verdict on whether each upstream change line affects the fork's surface
  - Overall verdict: CLEAR or COLLISION (with sub-verdict for scrub.rs: port-verbatim / D-20-replay / hold-with-rationale)
  - Cargo bump verdict: will-sync (always, per D-55-02)

Commit this artifact (planning file only — no source edits). DCO sign-off required.
  </action>
  <verify>
    <automated>
test -f ".planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md" && grep -c "CLEAR\|COLLISION" ".planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md"
    </automated>
    Must return >= 1 (at least one CLEAR or COLLISION verdict present).
  </verify>
  <acceptance_criteria>
    - 55-07-C13-DISPOSITION-RESOLUTION.md exists and is committed (planning files only; no crates/ edits)
    - Contains the exact upstream scrub.rs diff (from git show e581569)
    - Contains the fork's Phase-49 trust-root surface description
    - Contains a per-line collision analysis
    - Contains an overall verdict: CLEAR (port scrub.rs) or COLLISION (D-20 replay or hold)
    - Contains explicit statement about D-32-15 verify-is-offline invariant status
    - Cargo bump verdict: will-sync (stated explicitly)
    - Commit DCO: Signed-off-by: Oscar Mack Jr &lt;oscar.mack.jr@gmail.com&gt;
    - git diff for this commit shows zero changes to crates/ or bindings/ (planning docs only)
  </acceptance_criteria>
  <done>C13 disposition artifact produced; the upgrade-or-replay decision is documented with rationale before any code lands.</done>
</task>

<task type="auto">
  <name>Task 2: C13 Cargo bump + scrub.rs port or D-20 replay (per disposition resolution)</name>
  <files>Cargo.lock, crates/nono-cli/Cargo.toml, crates/nono/Cargo.toml, crates/nono/src/scrub.rs</files>
  <read_first>
    - .planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md (MUST read first — follow the verdict)
    - crates/nono/Cargo.toml (current state — locate sigstore-verify + sigstore-sign version entries)
    - crates/nono-cli/Cargo.toml (current state — locate sigstore entries)
    - crates/nono/src/scrub.rs (current state — apply the Cow deref changes if CLEAR verdict)
    - .planning/templates/upstream-sync-quick.md (D-19 trailer + D-20 Upstream-replayed-from: trailer for replay commits)
  </read_first>
  <action>
BRANCH A — If disposition-resolution verdict is CLEAR:
  Cherry-pick e581569 verbatim: `git cherry-pick e581569`
  Resolve conflicts:
    - Cargo.lock: accept the upstream changes (sigstore 0.8.0 + reqwest 0.13.3 transitive deps); may conflict with C7's Cargo.lock additions (jsonc-parser) — keep both.
    - Cargo.toml files: apply the version bumps for sigstore-verify + sigstore-sign to 0.8.0.
    - scrub.rs: apply the Cow deref adjustments verbatim.
  Amend with D-19 6-line trailer (Upstream-commit: e581569, Upstream-tag: v0.58.0).

BRANCH B — If disposition-resolution verdict is COLLISION on scrub.rs:
  Step 1: Apply ONLY the Cargo.toml + Cargo.lock changes (not the scrub.rs hunk).
    Create a new commit: `docs/fix: bump sigstore to 0.8.0 (Cargo.toml + Cargo.lock only)`.
    Manually update `crates/nono/Cargo.toml` sigstore-verify + sigstore-sign to "0.8.0".
    Manually update `crates/nono-cli/Cargo.toml` sigstore entries to "0.8.0".
    Run `cargo update -p sigstore-verify -p sigstore-sign` to regenerate Cargo.lock.
    Commit with D-19 trailer (Upstream-commit: e581569 partial — Cargo.toml only).
  Step 2: Apply scrub.rs as a D-20 manual-replay commit:
    Edit crates/nono/src/scrub.rs to apply the INTENT of the Cow deref change without disturbing the fork's Phase-49 --from-file or D-32-15 invariant.
    Commit with subject matching upstream intent + `Upstream-replayed-from: e581569` trailer instead of the D-19 `Upstream-commit:` trailer. Add a body comment documenting the collision and fork-specific handling.
    DCO Signed-off-by required.

After BRANCH A or B:
D-55-E5 check: verify `cargo build --workspace` includes all 5 crates (nono, nono-cli, nono-proxy, nono-shell-broker, bindings/c). If any crate fails to build due to the sigstore bump, document the failure and fix (e.g., adding compatible dep version in that crate's Cargo.toml).

D-55-E1: verify `git diff --name-only HEAD~{n} HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines.

D-55-E3 cross-target clippy: scrub.rs (crates/nono/src/) is NOT cfg-gated Unix code (it is cross-platform). However, if the sigstore 0.8.0 API introduces cfg-gated code in scrub.rs, check. Likely N/A — document.

D-55-E4: run `cargo test --workspace` and categorise vs Phase 54 baseline SHA.
  </action>
  <verify>
    <automated>
grep -E "sigstore-verify|sigstore-sign" crates/nono/Cargo.toml | grep -c "0.8.0"
    </automated>
    Must equal 2 (both sigstore deps bumped). Also verify:
      git diff --name-only HEAD~{n} HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      cargo build --workspace (exit 0)
      grep -c "0.8.0" crates/nono-cli/Cargo.toml (must find sigstore 0.8.0 entries)
  </verify>
  <acceptance_criteria>
    - 55-07-C13-DISPOSITION-RESOLUTION.md was read and its verdict followed
    - sigstore-verify and sigstore-sign are bumped to 0.8.0 in crates/nono/Cargo.toml
    - sigstore entries in crates/nono-cli/Cargo.toml are bumped to 0.8.0
    - Cargo.lock reflects the sigstore 0.8.0 transitive dependency set (reqwest 0.13.3 included)
    - scrub.rs: either (CLEAR) ported verbatim with D-19 trailer, or (COLLISION) D-20 replayed with Upstream-replayed-from: e581569 trailer, with rationale documented in commit body
    - D-32-15 verify-is-offline invariant: NOT regressed (trusted_root.json still read via plain JSON deserialization, not TUF re-verification) — verify by reading the post-cherry-pick scrub.rs
    - `git log --format="%B" HEAD~{n}..HEAD | grep -c "Upstream-commit:\|Upstream-replayed-from:"` >= 1
    - `git diff --name-only HEAD~{n} HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines (D-55-E1 PASS)
    - `cargo build --workspace` exits 0 (all 5 crates — D-55-E5)
    - Cross-target clippy: N/A for scrub.rs (cross-platform) or PARTIAL if cfg-gated blocks found
    - `cargo test --workspace` exits 0 or carry-forwards documented
    - Baseline-aware CI gate: lane-transition categories recorded (D-55-E4)
    - Feature branch NOT merged to main (D-55-03)
    - SUMMARY records the merge-gate note: Phase 55 feature branch merges to main ONLY after v0.58.0 is tagged + signed (D-55-03)
  </acceptance_criteria>
  <done>C13 sigstore 0.8.0 bump is on the feature branch; D-32-15 offline-verify invariant verified; all 5 crates build clean.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| sigstore API (scrub.rs) → trusted_root.json | D-32-15 invariant: no live TUF fetch; plain JSON deserialization only |
| Cargo.lock transitive deps → build | reqwest 0.13.3 + sigstore 0.8.0 transitive set enters the build |
| upstream cherry-pick e581569 → fork signing surface | HIGHEST SENSITIVITY — any regression in scrub.rs could degrade the signing verification posture |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-55-07-01 | Tampering | scrub.rs Cow deref — D-32-15 offline-verify invariant | mitigate | The diff-inspection in Task 1 is the primary control; Task 2 requires explicit post-cherry-pick verification that trusted_root.json is still read via plain JSON deserialization (grep: no TUF re-verification call introduced). Any regression = FAIL, do not merge. |
| T-55-07-02 | Tampering | sigstore 0.8.0 transitive dependency supply chain | mitigate | reqwest 0.13.3 is a high-use, well-audited crate; sigstore-rs is the project's own upstream dep already in Cargo.lock at a prior version. Cargo.lock pin prevents phantom version injection. Review Cargo.lock diff for unexpected new crates. |
| T-55-07-03 | Elevation of Privilege | sigstore API surface expansion in 0.8.0 | mitigate | Diff-inspection (Task 1) inspects whether 0.8.0 adds new API entry points that the fork's scrub.rs might accidentally call; the Cow deref change is mechanical and does not introduce new API calls per the commit message |
| T-55-07-04 | Denial of Service | Large Cargo.lock churn (329 added lines) | accept | Transitive dep churn is expected for a sigstore major-minor bump; `cargo build --workspace` is the gate; no runtime DoS risk from lock file size |
| T-55-07-SC | Tampering | sigstore/reqwest dependency legitimacy | mitigate | sigstore-rs is the fork's own upstream dependency (already pinned); reqwest is a canonical Rust HTTP crate. No [ASSUMED]/[SUS] packages; no blocking human checkpoint needed. Document in SUMMARY. |
</threat_model>

<verification>
1. `test -f ".planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md"` exits 0
2. `grep -c "CLEAR\|COLLISION" ".planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md"` >= 1
3. `grep -E "sigstore-verify|sigstore-sign" crates/nono/Cargo.toml | grep -c "0.8.0"` equals 2
4. `cargo build --workspace` exits 0 (all 5 crates)
5. `git diff --name-only HEAD~{n} HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines
6. D-32-15: `grep -n "TUF\|tuf\|update_trusted_root\|fetch" crates/nono/src/scrub.rs` shows no new live TUF fetch introduced
7. Feature branch NOT merged to main
</verification>

<success_criteria>
- 55-07-C13-DISPOSITION-RESOLUTION.md: diff-inspection artifact with CLEAR or COLLISION verdict and D-32-15 invariant statement
- sigstore 0.8.0 Cargo bump landed (both nono + nono-cli Cargo.toml); Cargo.lock updated workspace-wide
- scrub.rs: ported (CLEAR) or D-20 replayed (COLLISION) with correct trailer
- D-32-15 offline-verify invariant verified post-cherry-pick
- All 5 crates build clean (D-55-E5)
- D-55-E1, D-55-E3, D-55-E4 gates satisfied or PARTIAL-documented
- Feature branch held off main until v0.58.0 is tagged (D-55-03) — SUMMARY records this merge gate
</success_criteria>

<output>
Create `.planning/phases/55-upst7-cherry-pick-wave/55-07-SUMMARY.md` when done.
Include: C13 disposition resolution summary (CLEAR or COLLISION; scrub.rs lines examined; D-32-15 verdict); Cargo bump log; scrub.rs handling (verbatim port or D-20 replay with rationale); D-55-E5 5-crate build PASS; cross-target clippy status; D-55-E1 PASS; baseline-aware CI gate result; **MERGE GATE NOTE** — Phase 55 feature branch merges to main only after v0.58.0 is tagged + signed (D-55-03); D-55-E6 umbrella PR note.
</output>
