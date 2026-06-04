---
phase: 55-upst7-cherry-pick-wave
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - .planning/REQUIREMENTS.md
  - .planning/ROADMAP.md
  - crates/nono-proxy/src/connect.rs
autonomous: true
requirements: [REQ-UPST7-02]
must_haves:
  truths:
    - "REQUIREMENTS.md REQ-UPST7-02 prose matches the audit-of-record (no java-dev/java_runtime; C9/C12/C13 listed)"
    - "ROADMAP.md Phase 55 SC1 prose matches the audit-of-record (same drops and additions)"
    - "54-DIVERGENCE-LEDGER.md is unchanged (immutable)"
    - "nono-proxy returns 502 with an audit entry when the upstream connect fails (C4 commit d11193f)"
    - "nono-proxy preserves the upstream error message and sanitises the 502 reason line (C4 commit 4ad708d)"
    - "Each cherry-picked commit carries a verbatim 6-line D-19 trailer + DCO Signed-off-by"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
  artifacts:
    - path: ".planning/REQUIREMENTS.md"
      provides: "Updated REQ-UPST7-02 prose (java-dev dropped; C9/C12/C13 added)"
      contains: "C9.*pack-update-hint"
    - path: ".planning/ROADMAP.md"
      provides: "Updated Phase 55 SC1 (java-dev dropped; C9/C12/C13 added)"
      contains: "C9.*pack-update-hint"
    - path: "crates/nono-proxy/src/connect.rs"
      provides: "502 hardening: audit entry on upstream connect failure + sanitised reason line"
  key_links:
    - from: "55-01-PLANNING-AMEND-PROXY502-PLAN.md"
      to: ".planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md"
      via: "ledger C4 row as cherry-pick source"
      pattern: "d11193f|4ad708d"
---

<objective>
Plan 55-01 does two things in one wave:

1. **D-55-01 REQ/SC amendment (planning docs only):** Amend `.planning/REQUIREMENTS.md` § REQ-UPST7-02 and `.planning/ROADMAP.md` § Phase 55 SC1 to match the Phase 54 audit-of-record. Drop the phantom `java-dev`/`java_runtime` item (0 commits in v0.57.0..v0.59.0 per the empirical cross-check on `platform.rs`); add the omitted clusters C9 (pack-update-hint robustness), C12 (ENV_LOCK policy test), and C13 (sigstore 0.8.0 split). The `54-DIVERGENCE-LEDGER.md` stays byte-identical (it is the immutable audit-of-record). This is a planning-artifact commit only — zero source edits.

2. **C4 proxy 502 hardening (upstream commits d11193f + 4ad708d):** Cherry-pick the two proxy security hardening commits from upstream `v0.57.0..v0.59.0` cluster C4. Both touch only `crates/nono-proxy/src/connect.rs` (cross-platform; no cfg-gated Unix code; no Windows-specific files). The proxy surface is disjoint from all other Phase 55 clusters — safe to execute in Wave 1 in parallel with the planning-docs amendment.

Purpose: Get the planning artifacts accurate before any code lands; harden the proxy's failure path against upstream connect errors.

Output: Updated REQUIREMENTS.md + ROADMAP.md (D-55-01 amendment); two new commits on the held feature branch carrying the C4 cherry-picks with D-19 trailers.
</objective>

<execution_context>
@C:\Users\OMack\.claude\get-shit-done\workflows\execute-plan.md
@C:\Users\OMack\.claude\get-shit-done\templates\summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/STATE.md
@.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md
@.planning/phases/54-upst7-audit/54-01-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
@.planning/phases/55-upst7-cherry-pick-wave/55-CONTEXT.md

<interfaces>
<!-- Key upstream commits for C4 (from 54-DIVERGENCE-LEDGER.md § Cluster C4) -->
<!-- Cherry-pick source: upstream remote, range v0.57.0..v0.59.0 -->

C4 commits (chronological per git log, oldest-first):
  d11193fa — fix(proxy): return 502 with audit entry on upstream connect failure  (v0.58.0)
  4ad708d6 — fix(proxy): preserve upstream error and sanitise 502 reason line     (v0.58.0)

Both touch ONLY: crates/nono-proxy/src/connect.rs
windows-touch: no
Cross-cluster re-export check: clean (per ledger)

D-19 trailer format (verbatim from .planning/templates/upstream-sync-quick.md):
  Upstream-commit: <8-char sha>
  Upstream-tag: v0.58.0
  Upstream-author: <name> <email>
  Co-Authored-By: <name> <email>
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>

D-55-E2 field-order rule: Upstream-commit → Upstream-tag → Upstream-author → Co-Authored-By → Signed-off-by (full) → Signed-off-by (handle).

Upstream remote alias: upstream (https://github.com/always-further/nono.git)
Feature branch for Phase 55 work: create as upst7-cherry-pick-wave off Phase 54 baseline SHA
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: D-55-01 — Amend REQUIREMENTS.md and ROADMAP.md to match the audit-of-record</name>
  <files>.planning/REQUIREMENTS.md, .planning/ROADMAP.md</files>
  <read_first>
    - .planning/REQUIREMENTS.md (full file — read before editing)
    - .planning/ROADMAP.md (full file — read before editing)
    - .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md § Cluster Summary table rows C9/C12/C13 + § Empirical cross-check (platform.rs 0 commits confirmation)
    - .planning/phases/55-upst7-cherry-pick-wave/55-CONTEXT.md decisions D-55-01 block
  </read_first>
  <action>
Edit REQUIREMENTS.md § REQ-UPST7-02 prose:
  - Remove the `java-dev` profile / `java_runtime` group item from the enumerated list. Add an explicit parenthetical: "(java-dev/java_runtime: no upstream commits in v0.57.0..v0.59.0 per ledger empirical cross-check on platform.rs; UPST8 territory)".
  - Add C9 (pack-update-hint robustness: atomic state writes + detached-process refresh), C12 (ENV_LOCK policy test serialization), and C13 (sigstore 0.8.0 split: Cargo.toml bump + scrub.rs verify-then-port) to the will-sync enumeration.
  - Cite the 54-DIVERGENCE-LEDGER.md as authority in a parenthetical.

Edit ROADMAP.md § Phase 55 SC1 prose (the "what must be TRUE" success criterion #1):
  - Drop the java-dev / java_runtime enumeration from SC1.
  - Add C9, C12, C13 to the SC1 list.
  - Mirror the same "(java-dev/java_runtime: 0 commits in range per ledger; UPST8)" note.

Do NOT edit .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md — it is immutable.
Do NOT change any other ROADMAP or REQUIREMENTS section.

Commit only the two planning files with a clear commit message body. DCO sign-off required. No `Upstream-commit:` trailer (this is a fork-internal planning amendment, not a cherry-pick). Example subject: "docs(55): D-55-01 amend REQ-UPST7-02 + SC1 to match 54 audit-of-record".
  </action>
  <verify>
    <automated>git diff HEAD~1 HEAD -- .planning/REQUIREMENTS.md | grep -c "java-dev"</automated>
    Note: the diff must show the java-dev line REMOVED (context line only, not added). Also verify:
      grep -c "C9" .planning/REQUIREMENTS.md (must be >= 1)
      grep -c "C12" .planning/REQUIREMENTS.md (must be >= 1)
      grep -c "C13" .planning/REQUIREMENTS.md (must be >= 1)
      git diff HEAD~1 HEAD -- .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md (must be empty — immutable)
  </automated>
  </verify>
  <acceptance_criteria>
    - REQUIREMENTS.md contains no reference to "java-dev" or "java_runtime" as a Phase-55 will-sync item
    - REQUIREMENTS.md mentions C9 (pack-update-hint), C12 (ENV_LOCK test), C13 (sigstore 0.8.0) as Phase-55 scope
    - ROADMAP.md Phase 55 SC1 no longer lists java-dev/java_runtime as a will-sync target
    - ROADMAP.md Phase 55 SC1 includes C9, C12, C13 in the will-sync list
    - 54-DIVERGENCE-LEDGER.md git diff is empty (byte-identical to pre-task state)
    - Commit message body explains the D-55-01 authority (ledger empirical cross-check on platform.rs)
    - Commit carries: Signed-off-by: Oscar Mack Jr &lt;oscar.mack.jr@gmail.com&gt;
    - git diff HEAD~1 HEAD -- crates/ bindings/ scripts/ Makefile is empty (zero source edits)
  </acceptance_criteria>
  <done>Planning artifacts are accurate and committed; the ledger is untouched.</done>
</task>

<task type="auto">
  <name>Task 2: C4 cherry-pick — proxy 502 hardening (d11193f + 4ad708d)</name>
  <files>crates/nono-proxy/src/connect.rs</files>
  <read_first>
    - crates/nono-proxy/src/connect.rs (full file — read before cherry-picking to understand current fork state)
    - .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md § Cluster C4 (both commit rows, disposition, cross-cluster re-export check result)
    - .planning/templates/upstream-sync-quick.md (D-19 trailer field order + rules; fork-divergence catalog for Windows-only invariant)
    - .planning/phases/55-upst7-cherry-pick-wave/55-CONTEXT.md decisions D-55-E1 + D-55-E2 + D-55-E3 + D-55-E4
  </read_first>
  <action>
Create or check out the Phase 55 held feature branch (e.g., `upst7-cherry-pick-wave`) off the Phase 54 baseline SHA. D-55-03: this branch is held off main until v0.58.0 is tagged.

Cherry-pick the two C4 commits from the upstream remote in chronological order (oldest-first):
  1. d11193fa — fix(proxy): return 502 with audit entry on upstream connect failure
  2. 4ad708d6 — fix(proxy): preserve upstream error and sanitise 502 reason line

For each cherry-pick:
  a. Run `git cherry-pick <upstream-sha>` (from upstream remote).
  b. If conflicts arise in connect.rs, resolve by applying the upstream intent while preserving the fork's credential-injection and RouteStore decoupling patterns from crates/nono-proxy/src/connect.rs (the fork's connect.rs is a divergent surface from Phase 09/11).
  c. After resolving (if any), amend the commit to append the verbatim D-19 6-line trailer block (D-55-E2). Field order is FIXED: Upstream-commit → Upstream-tag → Upstream-author → Co-Authored-By → Signed-off-by (full name) → Signed-off-by (github handle). Use the 8-char abbreviated SHA for Upstream-commit. Upstream-tag is v0.58.0 for both. Append TWO Signed-off-by lines: `Oscar Mack Jr &lt;oscar.mack.jr@gmail.com&gt;` and `oscarmackjr-twg &lt;oscar.mack.jr@gmail.com&gt;`.
  d. Verify the amended commit message contains the trailer before moving to the next commit.

D-55-E1 invariant check: after both cherry-picks, confirm `git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns zero lines.

D-55-E3: connect.rs has no cfg-gated Unix code (it is a pure cross-platform Rust file with no `#[cfg(target_os = ...)]` blocks). Cross-target clippy is NOT required for this cluster — document as N/A in the SUMMARY.

D-55-E4: run `cargo test -p nono-proxy` and categorise the result relative to the Phase 54 baseline SHA. Document as green→green PASS or red→red carry-forward.
  </action>
  <verify>
    <automated>
git log --format="%B" HEAD~2..HEAD | grep -c "^Upstream-commit:"
    </automated>
    Must equal 2 (one per cherry-picked commit).
    Also verify:
      git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      git diff --name-only HEAD~2 HEAD | grep -v "connect.rs" (must return 0 lines — only connect.rs touched)
      cargo build -p nono-proxy (exit 0)
      cargo test -p nono-proxy (exit 0 or pre-existing failures documented)
  </verify>
  <acceptance_criteria>
    - Exactly 2 cherry-pick commits on the feature branch, each touching ONLY crates/nono-proxy/src/connect.rs
    - Each commit message ends with the verbatim 6-line D-19 trailer: Upstream-commit (8-char), Upstream-tag (v0.58.0), Upstream-author, Co-Authored-By, Signed-off-by (full name), Signed-off-by (handle)
    - `git log --format="%B" HEAD~2..HEAD | grep -c "^Upstream-commit:"` equals 2
    - `git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines (D-55-E1 PASS)
    - `cargo build -p nono-proxy` exits 0
    - `cargo test -p nono-proxy` exits 0 or pre-existing red→red carry-forwards are documented
    - Cross-target clippy status: N/A (no cfg-gated Unix code in connect.rs) — documented in SUMMARY
    - Baseline-aware CI gate: lane-transition categorisation recorded (D-55-E4)
    - The feature branch has NOT been merged to main (D-55-03 held-branch invariant)
  </acceptance_criteria>
  <done>C4 cherry-picks are on the feature branch with correct D-19 trailers; proxy hardening is committed; planning docs are amended per D-55-01.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| proxy → upstream server | connect.rs mediates outbound connections; a 502 path now returns structured error info upstream to the caller |
| upstream cherry-pick → fork | upstream commit d11193f/4ad708d absorbed into the fork's proxy surface |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-55-01-01 | Information Disclosure | connect.rs 502 reason sanitisation | mitigate | C4 commit 4ad708d explicitly sanitises the 502 reason line to prevent upstream error internals leaking to proxy clients; verify sanitise function strips control chars and limits length |
| T-55-01-02 | Spoofing | D-19 trailer integrity | mitigate | Each cherry-picked commit carries the upstream SHA in the trailer; `grep -c "^Upstream-commit:"` gate verifies presence; immutable after push |
| T-55-01-03 | Tampering | 54-DIVERGENCE-LEDGER.md (immutable audit-of-record) | mitigate | Task 1 acceptance criteria requires `git diff HEAD~1 HEAD -- .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` to be empty |
| T-55-01-SC | Tampering | npm/pip/cargo installs during cherry-pick | accept | No new cargo dependencies introduced by C4 (connect.rs change is pure logic, no new Cargo.toml entries); no package legitimacy audit entry needed |
</threat_model>

<verification>
After both tasks complete, verify the plan as a whole:

1. `git log --format="%B" HEAD~2..HEAD | grep -c "^Upstream-commit:"` equals 2
2. `git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines
3. `git diff HEAD~3 HEAD~2 -- .planning/REQUIREMENTS.md | grep "C9"` — C9 appears in the diff (added to REQ-UPST7-02)
4. `git diff HEAD~3 HEAD~2 -- .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` — empty (ledger immutable)
5. `cargo build -p nono-proxy` exits 0
6. Feature branch has NOT been merged to main
</verification>

<success_criteria>
- REQUIREMENTS.md and ROADMAP.md Phase 55 SC1 accurately reflect the audit-of-record (java-dev dropped; C9/C12/C13 added) — D-55-01 DONE
- Two C4 cherry-pick commits on the held feature branch with correct D-19 trailers — C4 DONE
- 54-DIVERGENCE-LEDGER.md is byte-identical to its pre-plan state — audit-of-record immutability preserved
- `cargo build -p nono-proxy` exits 0
- No windows-only files touched (D-55-E1)
- Feature branch held off main (D-55-03)
</success_criteria>

<output>
Create `.planning/phases/55-upst7-cherry-pick-wave/55-01-SUMMARY.md` when done.
Include: D-55-01 amendment summary (what changed in REQUIREMENTS.md + ROADMAP.md); C4 cherry-pick log (2 commits, shas, trailer verification result); conflict-file inventory (what conflicted in connect.rs if anything); baseline-aware CI gate result for nono-proxy; cross-target clippy status (N/A for C4); D-55-E1 windows-invariant status (PASS); held-branch status (D-55-03).
</output>
