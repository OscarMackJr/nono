---
phase: 117-fail-direction-contract-startup-self-attestation
fixed_at: 2026-08-14T21:30:00Z
review_path: .planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md
iteration: 3
round: 5
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 117: Code Review Fix Report — round 5

**Source review:** `117-REVIEW.md` (round 4, reviewed 2026-08-14T18:20:00Z; 0 Critical + 8
Warning = 8 findings, all in scope)
**Scope:** `critical_warning` — the review reported 0 Info findings, so this is every finding.

> Round-3's fix report is preserved at `117-REVIEW-FIX.round3.md`; round 1's at
> `117-REVIEW-FIX.round1.md`.

| | Count | Findings |
|---|---|---|
| Fixed | 8 | WR-01, WR-02, WR-03, WR-04, WR-05, WR-06, WR-07, WR-08 |
| Skipped | 0 | — |

7 commits on `gsd-reviewfix/117-r5-193`, fast-forwarded onto
`milestone/v2.13-carryforward-closeout`. All carry DCO sign-off.

Round 4 was the first with zero blockers. **No runtime-behaviour change was made this round** —
every commit touches guards, records or CI configuration. The one thing that is not a review
finding (the D-28 filesystem gate's fail-open read) is test code as well.

---

## The root, not the eight leaves

The brief's framing is correct and it is what I built to. Round 3 unified two disagreeing
`#[cfg(test)]` classifiers into `crates/nono-cli/src/cfg_test_regions.rs` — the right move — but
**propagated the classifier to both consumers while propagating the correctness assertion to only
one**. WR-01, WR-02 and WR-03 are three faces of that single omission, so they are one commit
(`aa1765cb`) and the fix is structural:

```rust
// cfg_test_regions.rs
impl ProductionScan<'_> {
    /// Assert the split is correct, and fail loudly with the evidence if it is not.
    pub fn assert_split_is_correct(&self, label: &str) { … }
}
```

Every consumer now calls that one function. There is no longer a shape of this gate a third
consumer can adopt while skipping the property, because the property is not something a consumer
writes — it is something it calls. The three checks inside it are ordered worst-direction-first:

1. `unclosed_regions` — **over-claim**: production text silently unscanned (WR-03).
2. `leaked_test_attributes` — **under-claim**: test assertions able to satisfy the caller (WR-02).
3. `skipped_regions` non-empty — the zero-region case neither of the above can see.

`skipped_lines()` is kept, but its doc now says in as many words that it is a floor and never a
correctness check, and *why*: it is monotone in the wrong direction.

---

## Requirement 2 — verified by round 4's exact perturbations

Round 4's demonstration was: one blank line before `agent_daemon/launch.rs`'s
`mod attestation_gate_tests {` moves 387 lines of test code into the production half; the daemon
gate's `skipped_lines() > 0` floor stays satisfied; renaming the real production
`layer = "DaclAncestorTraverse"` site then leaves the gate GREEN, satisfied by a test assertion at
line 2225.

| # | Perturbation | Result | Note |
|---|---|---|---|
| P1 | one blank line before `mod attestation_gate_tests {` | **GREEN** | and correctly so — see below |
| P2 | P1 **+** rename the production `layer = "DaclAncestorTraverse"` | **FAILS** | round 4: stayed GREEN |
| P3 | rename alone, no blank line | **FAILS** | baseline sanity |
| P4 | narrow the classifier window back to the round-3 shape, keeping the blank line | **FAILS** | guard half works independently |

**On P1 being green.** The brief asks for the blank-line perturbation to make the gate FAIL. It
does not, and that is the fix working rather than the fix missing: I closed the defect on **both**
halves, so with the widened classifier the blank line no longer causes a leak, and there is
nothing left for the gate to detect. P2 is the decisive test — it is the exact combination round 4
proved could stay green, and it now fails:

```
DaclAncestorTraverse is expected at (EntryPath::Daemon, None) with an attestable probe
(ConfiguredOnly), but daemon_attest_and_decide's source never names "DaclAncestorTraverse"
outside a comment — the row goes completely unattested on `nono agent launch` (CR-02).
```

P4 exists precisely because P1's greenness is otherwise unfalsifiable. Reverting only the
classifier's widened window, with the blank line still in place, produces:

```
WR-01/WR-02: 6 test attribute(s) survived into the PRODUCTION half of agent_daemon/launch.rs
(first at line Some(1870)), so at least one `#[cfg(test)]` module leaked into the scan and its
assertions can satisfy the caller's gate on their own.
```

So the guard catches the leak even when the classifier does not — which is the property that
matters for the *next* evading shape, the one neither of us has thought of. All four perturbations
were reverted; `git diff` confirms `agent_daemon/launch.rs` is untouched at final HEAD.

---

## Fixed

### WR-01 + WR-02 + WR-03: propagate the property, not the helper

**Files:** `cfg_test_regions.rs`, `exec_strategy_windows/layer_registry.rs`, `output.rs`
**Commit:** `aa1765cb`

**WR-02 (the root).** `layer_registry.rs:1452` kept `skipped_lines() > 0`. Both consumers, and the
`main.rs` half of `output.rs`'s gate (which had only `!skipped_regions.is_empty()` — the same
wrong-direction predicate, on the file whose test module asserts directly on the rendered
remediation text), now call `assert_split_is_correct`.

**WR-01.** The pending-attribute window admitted three shapes while its doc stated the class.
Widened to "anything that is not the gated item": blank lines, plain `//` comments, block-comment
bodies. `cfg` attributes are now joined across source lines until their brackets balance, so
`#[cfg(all(` on its own line is recognised — `is_cfg_test_attr` compacts whitespace before
matching so the joined form and the single-line form classify identically. The leak check matches
the test-attribute **class** (`#[test]`, `#[tokio::test]`, `#[rstest::test]`), not one literal;
pinning only `#[test]` would have made the leak check itself the narrow needle this round keeps
finding.

Three new unit tests, one per shape. Shape (2)'s fixture is the **real occurrence** the review
cited — `crates/nono-proxy/src/credential.rs`'s test module, written in this codebase's own idiom
today — not a synthetic one.

**WR-03.** "Closer not found" produced a region running to EOF with no signal. `unclosed_regions`
now records it and `assert_split_is_correct` rejects it first, before the leak check, because it
is the worse direction. The closer is matched against the module line's **original leading
whitespace** (so tab-indented modules close) and admits a trailing comment (`} // end of tests`).
Four tests cover it, including two `#[should_panic]` proofs that the assertion actually fires.

**Two-direction:** test-only. Denies more (four previously-passing shapes now fail the build);
exposes nothing — no runtime path, output, file or wire format touched.

---

### WR-04: the SPEC event-log gate matched two verbs, not the class

**File:** `main.rs` · **Commit:** `2b7da685`

`84e48ffe` fixed `main.rs`'s rendered-string guard by class and wrote the rejected narrow shape
into the SPEC mirror **in the same commit**. The predicate is now "any mention that is **not
qualified**", which no phrasing can evade — a new verb still has to explain itself. A mention is
legitimate only with one of four qualifiers attached within 200 characters, each chosen to be a
**fact about the legitimate path rather than a phrasing of it**:

| Qualifier | Why it is legitimate |
|---|---|
| `event id 10011` | the downgrade audit event — the path that really does write there |
| `emit_attestation_event` | that path's emitting symbol |
| `explicit negation` | how the abort path may name it |
| `nothing is written there` | ditto |

Matched case-insensitively: the SPEC writes both "Windows Application Event Log" and "…event
log", and a case-sensitive needle is one more evasion. All four real mentions (`SPEC:276`,
`:292` ×2, `:294`) are qualified today; I measured every window rather than assuming.

Adds the **detector self-test** the sibling WR-05 gate has and this one lacked — which is why the
round-3 "perturbation" only ever exercised the single historical phrasing the needle was written
from. Eight prescriptive phrasings must fire (`points at`, `is pointed at`, `pointing … at`,
`directs … to`, `refers … to`, `names … as the place to look`, `see the …`, `check the … Event
Log`); the three real qualified mentions must stay silent; and a qualifier 420 characters away
must **not** rescue a mention — the attached-window vacuity the sibling helper's own first draft
shipped with.

**PERTURBATION:** rewrite the WR-27 row as "with every other layer pointed at the Windows
Application event log" — a phrasing the old needle permitted — and the gate **FAILS**, naming
`SPEC:294` and listing the four qualifiers it wanted. Reverted.

---

### WR-05: the WR-14-record gate searched all of `error.rs`

**File:** `tests/layer_registry_selfcheck.rs` · **Commit:** `f62b22a3`

**Scoping to the whole record was not enough, and I only learned that by running the
perturbation.** The record's closing paragraph costs the real fix as "rippling through … the C
FFI", so with the block-scoped version in place I deleted the FFI-reachability sentence — WR-07's
one correction — and the gate still **PASSED**. The needles are now scoped to the **case-(3)
claim** (`"(3) is unreachable through"` … `"(1) cannot fire"`), where `"FFI"` can only have come
from the sentence being pinned. That perturbation then **FAILS**.

Both scope delimiters must be present: a missing opener or closer panics with a specific message
rather than silently widening back toward the whole file, which is the fail-OPEN direction. A
size band (`400..record.len()`) catches a collapsed or runaway extraction.

Same file, from the requirement-1 sweep: `every_open_marker_in_code_has_a_ledger_row` had
`!markers.is_empty()` — if the walk-back stops recognising **one** marker shape, that marker goes
silently unchecked while the floor stays satisfied by the others. Added the parser-correctness
property (**every scanned file containing ` OPEN` must yield a parsed marker**) and made the scan
take every occurrence on a line rather than only the first.

**PERTURBATIONS:** delete the FFI-reachability sentence → **FAILS**. Rewrite the marker id so the
file contains ` OPEN` text but no parseable marker → **FAILS** naming the blind file. (A weaker
first attempt — breaking only one of the two ids on that line — correctly stayed green, because
the line's `grep \`WR-14 OPEN\`` hint still carries a parseable marker.) Both reverted.

---

### WR-06: the injectivity claim exceeded the mechanism

**File:** `output.rs` · **Commit:** `cb7eab00`

Three records claimed a property an assertion over **six hand-written keys** did not establish.
Exhaustive is cheap, so the claim is made **true** rather than reworded away: all 2^13 subsets of
a 13-name vocabulary, sorted and comma-joined exactly as production builds the key, each asserted
to map to distinct content, with a `seen.len() == 8192` count check so a broken enumeration cannot
pass vacuously. The vocabulary is synthetic and fixed-shape on purpose — `output.rs` must not
learn the layer identity type (D-28's structural half), and the launch-side sibling is what binds
the real vocabulary end to end.

The strength framing is corrected too: two passes over the same fixed-key SipHash-1-3 permutation
differing only by a domain prefix is 128 bits of output **width** against accidental collision —
all WR-04 ever needed — and not a security level against a chosen-input adversary. `output.rs:266`
and the test doc both say what is actually checked.

**PERTURBATION (the discriminating one):** truncate the digest input to the key's first 25
characters. All six sample keys still differ within 25 chars, so the old assertion **PASSES**; the
exhaustive sweep **FAILS**, naming the colliding pair `"Layer00,Layer01,Layer02,Layer03"` /
`"…,Layer04"`. Reverted.

---

### WR-07: the SPEC gates did not run in CI for the change class they guard

**Files:** `.github/workflows/ci.yml`, `tests/layer_registry_selfcheck.rs` · **Commit:** `2a25cf69`

`run_code_jobs` was `false` when every changed file matched `\.md$`, so a pull request editing
**only** the SPEC ran zero jobs — and every SPEC defect this phase found was a SPEC-only edit.

I did **not** take the review's suggested shape of dropping the blanket `\.md$`. That would run
the full matrix on every `.planning/` artifact commit, which is a large unrelated cost the finding
does not call for. `^proj/` is force-included instead: that is where the contract documents live
and where the test tree reads from.

The comment the review asked for "at both ends" is a **gate** at the other end.
`every_markdown_file_gated_by_a_test_runs_the_code_jobs` is discovery-based: it finds Markdown
reads in the crate's source and test tree in **both** house idioms — `include_str!` with the
directory in the path, and `workspace_root().join("proj").join("….md")` without it — resolves each
basename to a real file to get its tree, and fails if that tree has no force-include clause. A
second contract document added later is covered without the test being touched. Comment lines are
excluded from the scan so prose about the rule (including this gate's own doc, which names the
SPEC) is not an instance of it — I hit exactly that self-match on the first draft.

**PERTURBATION:** delete the `|| [[ "${file}" =~ ^proj/ ]]` clause → **FAILS**:

```
WR-07: 1 Markdown file(s) are read and asserted on by the test tree but are NOT force-included …
  proj/SPEC-windows-fail-direction-contract.md (read by src/main.rs) — no `=~ ^proj/ ]]` clause
```

Reverted.

---

### WR-08: the per-file literal floor had one literal of headroom

**File:** `output.rs` · **Commit:** `bf28746c`

The floor is now `max(10, recorded_baseline / 2)`. For `attestation_downgrade_event.rs` that is
`max(10, 26/2) = 13`, so the two-literal refactor clears it by 11 instead of crossing it by 1,
while a genuine collapse still trips it. Baselines are **explicit constants**, not recomputed from
the current source — a derived baseline would make the floor unconditionally satisfiable, which is
the failure mode the gate exists to catch — and an `assert_eq` pins the baseline list against the
surface list so a newly added surface file cannot arrive unfloored. Measured counts print on every
run, so drift is legible without a failure.

Measured at final HEAD: `output.rs` 429, `launch.rs` 757, `attestation.rs` 66,
`attestation_downgrade_event.rs` 26, `main.rs` 204.

**PERTURBATIONS:** raise the small file's baseline to 208 → **FAILS** with "floor 104 = max(10,
baseline 208 / 2)", so the proportional floor genuinely binds. Drop `main.rs` from `BASELINE` →
**FAILS** with "covers 4 file(s) but 5 were scanned". Both reverted.

---

### (Not a review finding) Two wrong-direction checks in the CR-01 D-28 filesystem gate

**File:** `exec_strategy_windows/launch.rs` · **Commit:** `f7cea97f`

Found by applying requirement 1's question to every non-vacuity check in the phase's guards rather
than only the two cited. This is the phase's most security-relevant gate:

1. **Fail-open read.** `let Ok(bytes) = std::fs::read(file) else { continue; };` — an unreadable
   file was **silently skipped**. In a D-28 guard an unscannable file is precisely the one that
   could be hiding the plaintext layer set. It is now a violation of the guard's premise, reported
   with the IO error.
2. `!files.is_empty()` is more satisfied the more files exist, so it could not distinguish "the
   one expected marker" from "one of two expected artefacts still being written". The
   announcement writes exactly one file; that is now `assert_eq!(files.len(), 1)`, with the
   message stating that a second file under a child-readable sessions path is a D-28 event
   needing review rather than a widened floor.

---

## Requirement 1: the full sweep, and what it cleared

Every non-vacuity check in the phase's guards, with the question "could this stay green as the
thing it protects degrades?" applied to each.

| Check | Verdict | Action |
|---|---|---|
| `layer_registry.rs` `skipped_lines() > 0` | **wrong direction** | replaced (WR-02) |
| `output.rs` `!scan.skipped_regions.is_empty()` ×2 (two-file loop, `main.rs` half) | **wrong direction** | replaced (WR-02) |
| `selfcheck` `!markers.is_empty()` | **wrong direction** | parser-correctness property added |
| `launch.rs` D-28 `!files.is_empty()` + skip-on-unreadable | **wrong direction / fail-open** | fixed (`f7cea97f`) |
| `output.rs` `PER_FILE_FLOOR: 25` | right direction, mis-calibrated | proportional (WR-08) |
| `output.rs` `checked >= 500` global literal floor | right direction | kept |
| `output.rs` `hits >= floor` per file, `main_mentions >= 1` | right direction | kept |
| `layer_registry.rs` `checked >= 5` daemon rows | right direction | kept |
| `layer_registry.rs` `checked >= 2` call sites | right direction | kept |
| `selfcheck` `rows.len() >= 40`, `checked >= 20` | right direction | kept |
| `launch.rs` `LayerId::ALL >= 13` needle floor | right direction (reads the enum) | kept |
| `meta_test.rs` `also_automated_entries_are_non_vacuous` | right direction (unreadable ⇒ failure) | kept |
| `meta_test.rs` `manual_verification_section_excludes_the_registry_table` | `section.len() < spec.len()` is weak, but the sibling `!contains("## Layer registry")` **is** the correctness check and degrades loudly | kept, recorded |
| `main.rs` `the_spec_never_prescribes_the_medium_label_command` (deliberately no floor) | correct — the SPEC may stop mentioning the command | kept |

---

## Requirement 3: no narrow needles in what I wrote this round

Three predicates written this round could each have been a narrow needle. Each is a class:

- the event-log gate matches **any unqualified mention**, not a verb list (WR-04);
- the test-attribute leak check matches `#[test]` **and** `::test]`-suffixed forms, not one
  literal;
- the region closer matches `}` at the module's own indent **with an optional trailing comment**,
  not an exact whole-line string.

And every needle is scoped to its region: the event-log gate to ledger rows (`trim_start()`-aware,
so an indented row is not missed), the WR-14 needles to the case-(3) claim, the CI gate's Markdown
discovery to non-comment lines.

---

## Requirement 6: earlier rounds re-confirmed at final HEAD (`f7cea97f`)

| Fix | Evidence at final HEAD |
|---|---|
| **WR-02 (FFI)** | `types.rs:218 LayerAttestationFailed = 15`; `:244` conversion arm; `nono.h:138` doc block; `lib.rs:211` `NonoError::LayerAttestationFailed` arm. Intact. |
| **WR-08** (probe-kind evidence keyed by `LayerId`) | `attestation.rs` `ConfirmedByEnforcingComponentReport` arm still keys on `entry.id` and fails CLOSED; `exactly_one_row_is_confirmed_by_enforcing_component_report` passes. Intact. |
| **WR-11** (reserved device names) | `output.rs:449` `RESERVED` list + `eq_ignore_ascii_case` rejection at `:460`. Intact. |
| **CR-03 / RF-13** | `machine_policy.rs:197 RequiredLayersPolicy`, `:275 required_layers`, `:683` reader; SPEC `:262` RF-13 row says "reads the sub-key into `RequiredLayersPolicy.required`". Intact and consistent. |
| **WR-16** (`log_target_is_private` fail-secure) | `cli_bootstrap.rs:101`, still the canonicalize-and-component-check-against-granted-paths form. Intact. |
| **CR-01 digest marker** | `output.rs:225` writes only `attestation_downgrade_marker_content(dedup_key)`; reader at `:280` compares content; `.v2` filename suffix present. Intact, and now additionally protected by the exhaustive injectivity sweep and the hardened D-28 filesystem gate. |

No earlier fix was undone. The only earlier-round code I *changed* is `output.rs`'s and
`layer_registry.rs`'s non-vacuity blocks and the D-28 gate's read loop — all strictly strengthened.

---

## Verification

All gates run **at final HEAD** (`f7cea97f`) unless noted.

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | **PASS** |
| Windows `cargo clippy -p nono-sandbox-cli --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | **PASS** (exit 0) |
| `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) | **PASS** (exit 0), 3m08s |
| `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** (exit 0) — twice: 16m16s at `bf28746c`, 7m05s at final HEAD |
| `cargo test -p nono-sandbox-cli --bin nono` | 1669 passed, **14 failed** (baseline) |
| `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` | 19 passed |
| `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` | 10 passed |
| `cargo test --workspace` | 1669 + 844 + 51 + 40 + 18 passed, 14 failed (baseline) |

**Note on the Linux gate.** Both cross-target gates were run **locally and to completion** — no
PARTIAL→CI fallback. The Linux gate was run twice: once at `bf28746c` (16m16s, cold) and again at
final HEAD after `f7cea97f` (7m05s, warm cache), both exit 0. The only change between the two
revisions is inside a `#[cfg(all(test, target_os = "windows"))]` module, which is cfg'd out
entirely under a Unix target, so the outcome could not have differed — but CLAUDE.md requires the
gate, not the argument, so it was re-run rather than reasoned about. The macOS gate was run once,
at final HEAD.

### Regression baseline

The 14 failures are the known Windows-host baseline, **identical by name** to the list recorded in
round 3 (`config::tests::*` HOME/USERPROFILE env races ×6, `protected_paths::tests::*` ×3,
`profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`,
`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`,
`exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`,
`exec_strategy::launch::broker_dispatch_tests::broker_launch_assigns_child_to_job_object`,
`exec_strategy::launch::write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file`).

| | Passed (`--bin nono`) | Failed |
|---|---|---|
| Phase base `334530af` (recorded round 2) | 1641 | 14 |
| Round-2 HEAD | 1646 | 14 |
| Round-3 HEAD `860d4772` | 1657 | 14 |
| **This branch `f7cea97f`** | **1669** | **14** |

**Zero regressions; +12 tests over round 3.**

---

## Left open, with their recorded rationale (unchanged)

1. **CR-02 defect 1** — operator decision: does the daemon arm consume the registry? Options A/B
   costed in the SPEC ledger. The gate deliberately proves "no daemon-expected row is unmentioned",
   not "the registry drives the daemon", and its doc says so.
2. **WR-12 + WR-14 are one change.** Landing the machine-policy plumbing makes WR-14's
   mis-targeted remediation reachable. Both markers say so; the SPEC has a WR-14 row; and the
   record's case-(3) claim is now gate-protected at the right scope.
3. **WR-10** — operator decision: accept the lockstep two-binary rename, or keep one `LayerId`
   covering two kernel objects with the divergence documented.
4. **WR-14 is reachable for embedders today**, not only "when WR-12 lands" —
   `nono::attestation::probe_in_job` is public API.

---

## What still deserves a human read

Nothing this round changes runtime behaviour, so the CR-01 disclosure argument from round 3 stands
unmodified and its two open items are unchanged (the filename/content coupling is still ungated;
unforgeability is still structurally out of reach on the same-user, same-IL token arms).

One judgement call is mine and should be reviewed as a decision rather than a fix: **WR-07's scope.**
The review proposed dropping the blanket `\.md$` from the CI classifier; I force-included `^proj/`
instead, because the proposed shape would run the full CI matrix on every `.planning/` artifact
commit. If the intent was the broader change, the clause is one line and the sync gate will keep
whatever is chosen honest.

---

## Commits

| # | Hash | Finding |
|---|---|---|
| 1 | `aa1765cb` | WR-01 + WR-02 + WR-03 (one root) |
| 2 | `2b7da685` | WR-04 |
| 3 | `f62b22a3` | WR-05 (+ marker-discovery sweep) |
| 4 | `cb7eab00` | WR-06 |
| 5 | `2a25cf69` | WR-07 |
| 6 | `bf28746c` | WR-08 |
| 7 | `f7cea97f` | requirement-1 sweep: D-28 filesystem gate (not a review finding) |

---

_Fixed: 2026-08-14_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 3 (round 5)_
