---
quick_id: 260815-f2x
type: execute
status: complete
completed: 2026-08-15
base_head: f4fd994a81135c76619921c2e72c1f851138aedc
branch: milestone/v2.13-carryforward-closeout
pushed: false
commits:
  - hash: de77b1fd
    title: "fix(ci): split multi-command windows steps so each exit code is checked"
    files: [".github/workflows/ci.yml"]
  - hash: 0a34dc44
    title: "docs(117): correct the records that claimed the windows CI gate could fail the build"
    files:
      - ".planning/phases/117-fail-direction-contract-startup-self-attestation/117-VERIFICATION.md"
      - "proj/SPEC-windows-fail-direction-contract.md"
requirements: [CINT-03]
---

# Quick Task 260815-f2x: Fix the Fail-Open Windows CI Gate — Summary

Closed a fail-open verification gate inside Phase 117's own CI wiring — the exact defect class
Phase 117 exists to close — and corrected the five records that asserted the gate worked.

## What was wrong

CI run 31888431118 (2026-08-15, branch `milestone/v2.13-carryforward-closeout`) reported
`conclusion: success` for job `windows-layer-fault-injection` while that job's own log reported
`test result: FAILED. 1684 passed; 22 failed`.

Mechanism: the step `Run layer-fault-injection suites` held two `cargo test` invocations in one
`run:` block with no `shell:` override. On `windows-latest` that defaults to pwsh, where GitHub
Actions appends a trailing `exit $LASTEXITCODE` — so the step's status is the exit code of its
LAST native command only. The passing `nono-shell-broker` leg reset `$LASTEXITCODE` to 0 and
masked the failing `nono-sandbox-cli` leg.

The `windows-packaging` step `Build Windows release binaries` carried the identical latent shape:
a failed `cargo build --release -p nono-sandbox-cli` would pass the step whenever the broker build
then succeeded, letting a broken `nono.exe` reach packaging.

## Task 1 — `.github/workflows/ci.yml` (commit `de77b1fd`)

One cargo command per step, in both jobs. Chosen over `shell: bash` or a `$LASTEXITCODE` guard
because it removes the shell-semantics reasoning entirely and cannot silently regress when someone
later appends a third command.

| Job | Was | Now |
|---|---|---|
| `windows-layer-fault-injection` | `Run layer-fault-injection suites` (2 `cargo test`) | `Run layer-fault-injection suites (nono-sandbox-cli)` + `Run layer-fault-injection suites (nono-shell-broker)` |
| `windows-packaging` | `Build Windows release binaries` (2 `cargo build`) | `Build release binary: nono-sandbox-cli` + `Build release binary: nono-shell-broker` |

Both jobs carry a do-not-recombine note naming the mechanism and citing run 31888431118.

**Cargo commands are byte-identical** to before — same packages, same `--features
layer-fault-injection`, same `-- --test-threads=1`, same order (`nono-sandbox-cli` first), same
`--release` flags. Verified by parsing both files and comparing the extracted command lists:
identical and in the same order for both jobs. No `runs-on`, `needs`, `if:` or matrix change; all
14 other jobs compare byte-equal in parsed form, which is what proves the `changes` job's
`run_code_jobs` classifier region (read by `layer_registry_selfcheck.rs`) was not touched.

### RUSTFLAGS confirmation — landed on BOTH new packaging steps

Required duplication, not a copy-paste slip: dropping it from either step silently changes what is
built. Verified structurally, not by eye:

```
'Build release binary: nono-sandbox-cli'   shell=pwsh  RUSTFLAGS='-Dwarnings -C target-feature=+crt-static'
'Build release binary: nono-shell-broker'  shell=pwsh  RUSTFLAGS='-Dwarnings -C target-feature=+crt-static'
```

Both fault-injection steps correctly have **no** `shell:` key, preserving the pre-fix default.

### Comment accounting — before → after

Extracted every `#`-comment line inside the two edited job regions from both
`git show f4fd994a:.github/workflows/ci.yml` and the working tree, then diffed the lists.

**Zero removals. Zero duplicated lines. 33 → 48 comment lines (+15).**

| Pre-existing block | Was attached to | Now attached to | Disposition |
|---|---|---|---|
| **WR-20 coverage comment** (12 lines, `:343-354`) | job-level header above the `windows-layer-fault-injection:` key | unchanged — same position, byte-identical | Did not move. Its claim *"the cargo invocations below carry no `--bin`/`--test`/`--lib` target filter"* is already plural and remains true of both new steps, so no wording adjustment was needed. |
| **NR-08 broker-build note** (7 lines, `:379-385`) | `Build nono-shell-broker (release)` | unchanged — same step, byte-identical | That step was not split; the note did not move. |
| **CR-10 note** (8 lines, `:389-396`) | `Run layer-fault-injection suites` | `Run layer-fault-injection suites (nono-sandbox-cli)` — the FIRST of the two new steps | Moved as ONE block, not copied. It introduces the pair and its `--test-threads=1` rationale covers both. 6 new lines inserted **inside** the block, after the *"this is the build"* sentence, recording why the split exists. |

Additions (15 lines, all new, none duplicated):
- 6 lines inside the CR-10 block — the do-not-recombine rationale with the live-run citation.
- 1 line — `# Second leg of the CR-10 pair above; same --test-threads=1 rationale.` above the new
  `(nono-shell-broker)` step, tying it back rather than copying the CR-10 block onto it.
- 8 lines — the equivalent note above the `windows-packaging` split, so the same reasoning is
  discoverable there.

One mid-execution adjustment: my first draft opened both new notes with the identical sentence,
which made a byte-level duplicate-line check fail on that one shared line. Rather than caveat the
check, I reworded the packaging note's opener so the check is unambiguously green. The two notes
describe different steps and now read differently.

Encoding verified clean: the only non-ASCII codepoint in the whole file is U+2014 (em dash, already
present at HEAD) and there are zero U+FFFD replacement characters. A `?` seen in intermediate
console output was a cp1252 rendering artifact of the Bash tool, not file corruption.

### Class scan — post-fix and non-vacuity

The same script file was used for both runs, so the scan body is provably identical. It parses
`ci.yml` with pyyaml, walks every job whose `runs-on` contains `windows`, and flags any step whose
`run:` holds ≥ 2 non-comment command lines under a non-bash/sh shell (pwsh being the `windows-latest`
default when `shell:` is absent).

**Post-fix, working tree — TOTAL 1:**

```
FLAG ('windows-packaging', 'Validate Windows MSI contract', 'pwsh', 10)
TOTAL 1
```

**Pre-fix, `git show f4fd994a:.github/workflows/ci.yml` — TOTAL 3:**

```
FLAG ('windows-layer-fault-injection', 'Run layer-fault-injection suites', 'pwsh', 2)
FLAG ('windows-packaging', 'Build Windows release binaries', 'pwsh', 2)
FLAG ('windows-packaging', 'Validate Windows MSI contract', 'pwsh', 10)
TOTAL 3
```

Matches the plan's predicted 3. **The scan is proven capable of finding the class it now reports
absent** — it is not passing by absence, which is this project's recorded signature failure and
would have been an especially poor thing to introduce while fixing an instance of it.

**Allowlist — 1 entry:**

| Step | Reason it is not a defect |
|---|---|
| `windows-packaging` / `Validate Windows MSI contract` | Its guard uses an explicit `exit 1`, and the `.ps1` invocation is the step's FINAL command — so its exit code is the one the runner reads. Left untouched per scope. |

### Other verification

- YAML parses via pyyaml (parsed, not eyeballed).
- `git diff` hunks confined to the two job regions (lines 393–453); `git diff --stat` shows only
  `.github/workflows/ci.yml`.
- No test run needed — no code changed.

## Task 2 — the records (commit `0a34dc44`)

### `proj/SPEC-windows-fail-direction-contract.md`

New ledger row **`CI-01 (2026-08-15, quick task 260815-f2x)`**, appended to the RF-/WR-/CR- table,
matching the neighbouring rows' 3-column `| # | What was wrong | What changed |` shape (verified:
4 pipes, consistent with 52 of the 55 rows; the 3 outliers carry literal pipes in code spans). It
states that RF-11's and RF-12's resolution cells overclaimed, cites run 31888431118 and
`test result: FAILED. 1684 passed; 22 failed` against `conclusion: success`, names the mechanism in
one sentence, gives the fix with the new step names, and records both class-scan figures with the
date run. It states plainly that the suite does **not** pass and that triaging the 22 failures is a
separate follow-up.

Dated in-place pointers appended to RF-11's and RF-12's resolution cells (the RF-10 precedent for
correcting a stale cell rather than leaving it to be contradicted elsewhere). Their *"What was
wrong"* columns are untouched.

#### D-15 routing justification — why a ledger row and not just a review artifact

This was the judgement call most likely to be wrong in either direction: sweeping a real divergence
out of the ledger, or diluting the ledger with machinery noise.

The ledger's own scoping statement (`:127-138`) routes verification-machinery defects — *"a gate,
meta-test, source-scanning classifier, discovery scan, record-about-a-gate or CI wiring that is
narrower than the class its own doc-comment states"* — to the committed `117-REVIEW*` artifacts,
and reserves D-15 for contract-vs-code divergences. Its distinguishing question is *"would an
operator reading this contract be misled about what the confinement code actually does?"* It then
states the tie-break explicitly: **a finding that is both gets a row here as well.**

This one is both, and the deciding fact is that **this document's own cells are false**:

- **RF-11** (`:315`) claimed *"a `Windows Layer Fault Injection` CI job now run that build"*.
- **RF-12** (`:316`) offered *"the `make` target and CI job above run the rest"* as the remedy for
  *"none of this phase's own evidence executed."*

As run, the job could not fail on that evidence, so the remedy those cells assert is not the remedy
that shipped. An operator reading the SPEC was told a gate would fail the build when it structurally
could not.

**In-document precedent, which cuts cleanly in both directions:**

| Precedent | Disposition | Why |
|---|---|---|
| Rounds 4, 6, 8 — 20 findings, incl. round 6's **WR-04** (a fail-open `read_dir` *inside a gate*) | **No row.** Indexed in prose with per-finding dispositions. | Pure verification machinery. The SPEC's own text stayed true. |
| **RF-10** — SC4-2/SC4-4 still read *"Deferred to Plan 117-10 / 117-04"* after both plans landed | **Row.** | The SPEC's own cells were false. |
| **WR-19** — stale `launch.rs:2190`/`:2194` citation in the Manual verification table | **Row.** | The SPEC's own citation was false. |

This finding is the **RF-10 / WR-19 shape**, not the WR-04 shape. Hence a row. Note the contrast
with WR-04 specifically: WR-04 was *also* a fail-open inside a gate and got **no** row — because the
SPEC never made a claim about it. The discriminator is not "how bad is the machinery defect", it is
"does this document say something false".

#### SPEC wording-gate compliance

Three live Rust tests read this document by `include_str!` and would reject certain string classes.
Running them means building a crate that is currently RED, so per the plan I used the mechanical
diff-only grep as a stand-in. **This is a wording pre-check, not a test run.** On the added lines
only:

| Gate | Forbidden string class | Hits in the diff |
|---|---|---|
| `the_spec_ledger_does_not_point_abort_path_layers_at_the_event_log` | Windows Application event log on a `\|`-leading line | **0** |
| `the_spec_never_prescribes_the_medium_label_command` | `/setintegritylevel Medium` | **0** |
| `every_spec_symbol_citation_resolves_to_a_real_definition` | `file.rs::Symbol`-shaped citations | **0** |

The new row cites job and step NAMES and the run ID instead of symbols, by design.

### `117-VERIFICATION.md` — evidence corrected, verdict untouched

**The verdict is byte-identical to HEAD.** Verified by parsing both frontmatters and comparing:
`status` (`human_needed`), `score`, `overrides`, `overrides_applied` (1) and `re_verification` all
compare **EQUAL**; gap count (2) and `human_verification` count (3) unchanged; no top-level key
added or removed; `overrides[0].reason` and `gaps[0].override_ref` byte-identical.

**SC3's 10/13 row coverage and the operator override stand unchanged, and are neither restated nor
softened anywhere in this task's edits.**

What changed is the *evidence* about the CI job. Each corrected site states the same three facts
plainly:

1. As run on 2026-08-15 (run 31888431118) the gate was **fail-open** and could not fail the build.
2. It is **fixed** by this task, which splits the commands one per step (cited by new step name).
3. **Separately and still open:** the job is RED with 22 failures whose triage is an agreed
   follow-up, out of scope here.

| # | Site | Correction |
|---|---|---|
| 1 | frontmatter `gaps[0].reason` | *"backed by an actual build"* removed; now says the claim is backed by the WIRING as of the fix and by no executed build yet. |
| 2 | frontmatter `gaps[0].missing[1]` | Coverage half kept (the unfiltered invocations do run the WR-20 pin); gating half corrected — a WR-20 failure sits in the masked leg, so a run of the job as-was could not have converted WR-20 to verified. |
| 3 | Required Artifacts row for `.github/workflows/ci.yml` | `✓ VERIFIED` → `⚠ CORRECTED 2026-08-15 — ran the suites, but was FAIL-OPEN as run; wiring now fixed, job still RED`, and says outright that the cell was wrong about what the gate could DO. |
| 4 | Human Verification item 1 | Same three facts; stale `ci.yml:343-388` / `ci.yml:387` citations replaced with job and step names. |
| 5 | frontmatter `human_verification[0]` (`test` + `why_human`) | **Not in the plan's list of four.** Found during verification carrying the same claim and the same stale citations. Corrected to match. |

Stale-claim audit (whitespace-normalised, so line-wrapping cannot hide a hit):
`backed by an actual build` → **0**; `ci.yml:343-388` → **0**; `ci.yml:387` → **0**; all **3**
surviving occurrences of *"no new CI wiring is required"* are inside dated `CORRECTED 2026-08-15`
passages that negate them.

## Deviations from plan

**1. [Rule 2] A fifth site, not in the plan's list of four.** The plan named four sites in
`117-VERIFICATION.md`. Verification found the same false claim and the same stale `ci.yml:343-388`
/ `ci.yml:387` citations in frontmatter `human_verification[0]`. The plan's own done-criterion is
*"no record in the tree still asserts that the pre-fix job could fail the build"*, so leaving it
would have failed the task while passing its checklist. Corrected.

**2. [Rule 1] Two `no new CI wiring is required` assertions were false, not merely imprecise.**
Wiring *was* required — this task supplied it. Both are now dated corrections that keep the true
coverage half and negate the false gating half.

**3. Packaging step names quoted.** `Build release binary: nono-sandbox-cli` contains a `: ` which
is a YAML mapping indicator in an unquoted scalar. The plan's suggested names are kept verbatim and
double-quoted rather than renamed. YAML parse confirms.

**4. Comment opener reworded mid-execution.** See the comment accounting above.

## Discovery — reported, not acted on

**The SPEC line-count parenthetical was already wrong before this task.** The Required Artifacts
cell read *"was 344 lines; 372 after 2026-08-15"*. The actual count after `fba34606` was **399**,
not 372 — stale by 27 lines at the base commit. I corrected it to **400** (the post-`CI-01` count)
and recorded in the cell that the `372` figure was never right, rather than quietly overwriting it.

**A sibling parenthetical in the same table looks wrong too, and I did NOT touch it.** The
`crates/nono-cli/tests/layer_registry_selfcheck.rs` row reads *"2245 lines at this pass; 2418 after
2026-08-15"*; the file is currently **2400** lines. Out of scope for this task, which was permitted
to modify three named files only. Flagged for triage — same class as the one corrected above, which
suggests these line-count parentheticals are unmaintained generally and might be better replaced
with something machine-checked or removed.

## Scope compliance

- Only the three named files modified. No production code.
- `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md` untouched (parallel
  milestone v3.5 is open). **No GSD SDK state or roadmap writer was run.**
- Out-of-scope items left alone: the 22 failing tests, the clippy/cross-compile failures, the
  MSI-contract step, all `runs-on`/matrix/`needs`/`if:`, and the cargo commands themselves.
- Both commits carry the explicit `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` trailer
  (written into the message body — `git commit -s` stamps `oscarmackjr-twg` on this host).
- **Nothing pushed.** The operator decides when to push and re-run CI.

Note: `proj/` is gitignored-but-tracked in this repo, so `git add` emits an ignored-path hint.
Confirmed the SPEC change did land in `0a34dc44` (`git show --stat` lists both files; working tree
clean).

## Follow-ups for the operator

1. **Push and re-run CI.** The fixed wiring is unexercised — this task changed what the gate CAN
   do, and no run has yet been made with it. Expect `windows-layer-fault-injection` to go RED, which
   is the correct outcome.
2. **Triage the 22 failures.** Agreed separate follow-up; not touched here.
3. **The stale `layer_registry_selfcheck.rs` line-count parenthetical** (see Discovery).

## Self-Check: PASSED

- `.github/workflows/ci.yml` — FOUND, modified, parses.
- `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-VERIFICATION.md` —
  FOUND, frontmatter parses, protected keys equal to HEAD.
- `proj/SPEC-windows-fail-direction-contract.md` — FOUND, `CI-01` row present, table well-formed.
- Commit `de77b1fd` — FOUND in `git log`.
- Commit `0a34dc44` — FOUND in `git log`, contains both Task 2 files.
- Working tree clean apart from this quick task's own docs directory.
