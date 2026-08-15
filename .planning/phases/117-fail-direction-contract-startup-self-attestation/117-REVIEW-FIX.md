---
phase: 117-fail-direction-contract-startup-self-attestation
fixed_at: 2026-08-14T23:55:00Z
review_path: .planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md
iteration: 5
round: 9
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 117: Code Review Fix Report — round 9

**Source review:** `117-REVIEW.md` (round 8, reviewed 2026-08-15T05:20:00Z; 1 Critical + 4
Warning = 5 findings, all in scope)
**Scope:** `critical_warning` — the review reported 0 Info findings, so this is every finding.

> Round 7's fix report is preserved at `117-REVIEW-FIX.round7.md`; round 5's at
> `117-REVIEW-FIX.round5.md`; round 3's at `117-REVIEW-FIX.round3.md`; round 1's at
> `117-REVIEW-FIX.round1.md`.

| | Count | Findings |
|---|---|---|
| Fixed | 5 | CR-01, WR-01, WR-02, WR-03, WR-04 |
| Skipped | 0 | — |

5 commits on `gsd-reviewfix/117-r9-1875`, fast-forwarded onto
`milestone/v2.13-carryforward-closeout`. All carry DCO sign-off. Final HEAD `f2d638df`.

---

## Where each run happened, and why it matters

The brief's central instruction: **a test that touches the filesystem is verified in the real
repository at `C:\Users\OMack\Nono`, not only in a clean worktree.** Round 7 reported CR-01's
target as "20 passed" from inside `Temp\sv-117-reviewfix-*`, where the defect structurally cannot
appear.

Every run below is labelled. The short version:

| Run | Where | Result |
|---|---|---|
| CR-01 reproduction, before any change | **REAL TREE**, clean, at `b338c842` | **19 passed, 1 failed** — reproduced |
| CR-01 defect condition simulated | worktree, `.claude/worktrees/` + `.gsd/` copies staged | pre-fix **FAILS**, post-fix **PASSES** |
| `layer_registry_selfcheck` at final HEAD | **REAL TREE** | **20 passed, 0 failed** |
| Non-vacuity: four `ci.yml` clauses dropped one at a time | **REAL TREE** | **all four FAIL**, each naming its own tracked file |
| `--bin nono` at final HEAD | **REAL TREE**, quiet machine | **1688 passed, 12 failed** (all baseline) |
| Round-6 perturbation table | worktree (source-text only, no filesystem dependence) | reproduces |
| Cross-target clippy, both gates | worktree | **exit 0** |

**A clean worktree is structurally blind to this defect class**, so I did not rely on one: I
reproduced the condition *inside* the worktree by creating the gitignored trees, and confirmed
pre-fix code fails there and post-fix code does not. That gives a discriminating perturbation
rather than an environment-shaped pass, and the real tree then confirmed it end to end.

**One result differed between the two trees and it was not the fix.** `--bin nono` in the real
tree while a Docker cross-compile was running showed one extra failure,
`output::tests::attestation_downgrade_banner_cold_vs_warm_dedup_marker_latency` — a latency
assertion. It passes in isolation and it passes in a full run on a quiet machine. Recorded rather
than quietly dropped, because "it passed the second time" is exactly the shape this phase keeps
producing.

---

## Fixed

### CR-01: resolve the CI sync gate against the repository, not the disk

**File:** `tests/layer_registry_selfcheck.rs` · **Commit:** `70691d04`

**Reproduced first.** REAL TREE, clean, `b338c842`: 19 passed, 1 failed, naming ten paths — eight
copies of `profile-authoring-guide.md` under `.claude/worktrees/agent-*/`, one
`.claude/worktrees/agent-ae13967c/.planning/PROJECT.md`, and `.gsd/PROJECT.md`. This host has
**15** agent worktrees under `.claude/worktrees/` and a `.gsd/PROJECT.md`; both trees are
gitignored.

The review's diagnosis is right in all three parts, and the deeper point is the one worth
recording: **round 7 replaced an explicit list with a walk to make discovery "real", and the walk
was less correct than the list it replaced.** Real discovery means enumerating the set the
question is about. This gate asks "can a pull request change this file", so the set is what git
says the repository contains — not what happens to be on disk, and not a hand-written list either.

Both halves now go through one primitive, `tracked_files(pathspec)`, which shells out to
`git ls-files -z` and **fails CLOSED** if git cannot be run or exits non-zero. `-z` so a path with
a quote or a space is not reshaped by git's own quoting into something that silently fails to
match a `ci.yml` clause. `collect_by_extension` is deleted rather than left unused.

Resolution also uses the directory the read site already spells out. The house idiom writes a path
one component per `.join("..")` call, usually on its own line, so the components are visible
across the look-back window but never within one line — the review's suggested same-line fragment
scan would not have reached `.planning/PROJECT.md`. Quoted fragments are collected across the
window in order, `..` is applied, and the **longest trailing suffix that names a tracked file**
wins. `env!("OUT_DIR")` contributes a prefix nothing matches, so
`include_str!(concat!(env!("OUT_DIR"), "/profile-authoring-guide.md"))` falls all the way back to
its basename — which is correct, that site genuinely carries no repository directory. Ambiguity
*within* the winning suffix stays fail-CLOSED.

**PERTURBATIONS.**

| # | Where | Perturbation | Result |
|---|---|---|---|
| P-C | worktree, defect condition staged | `.claude/worktrees/fake-agent/.../profile-authoring-guide.md` + `.gsd/PROJECT.md` created | **PASSES** |
| P-C control | worktree, same condition | the **pre-fix** code | **FAILS**, naming both untracked copies |
| P-A | **REAL TREE** | drop `(^crates/nono-cli/data/)` | **FAILS** naming `crates/nono-cli/data/profile-authoring-guide.md (read at src/config/embedded.rs:39)` |
| P-B | worktree | drop the exact `.planning/PROJECT.md` clause | **FAILS** naming it |
| P-K | **REAL TREE** | drop `(^docs/architecture/)` | **FAILS** naming `docs/architecture/aipc-unix-futures.md` |
| P-L | **REAL TREE** | drop `(^proj/)` as well | **FAILS** naming the SPEC at all three of its read sites |
| P-D | worktree | `ROOT_LOOKBACK` 8 -> 0 (blind the read scan) | **FAILS** on the new classified-set floor |

P-A is the one the brief asked for by name: **round-6 WR-05's genuine counterexample is still
caught**, and now it is named once, as the tracked path, instead of nine times as copies that
cannot be force-included. P-K and P-L show all four force-include clauses are load-bearing — the
gate classifies four distinct repository files, so the new `classified.len() >= 3` floor has
margin rather than being tuned to it.

**Note, not a defect:** P-L prints the SPEC three times, once per read site, because dedup is on
the site's reconstructed path and three sites reconstruct different prefixes. Redundant output,
fail-closed, and it shows every site.

---

### WR-02: make the comment rule a lexer, and the production half code

**File:** `cfg_test_regions.rs` (+ `layer_registry.rs`, `output.rs` call sites) ·
**Commit:** `fc378757`

Comment state was only ENTERED from a line whose trimmed text starts with `/*`, so
`let x = 1; /* NOTE ...` opened no comment and every continuation line of it was ordinary code to
every consumer. The "deliberate limit" paragraph named only the false-drop cost.

**Fixing only the continuation lines would have left the same hole one line up**, and I want that
on the record because it is the difference between the class and a rendering of it.
`ProductionScan::lines` carried RAW lines, so a trailing comment travelled into the production
half attached to its code: `let x = 1; /* DaclAncestorTraverse */` satisfies
`layer_registry.rs`'s daemon gate on the OPENING line, which no continuation-line rule can reach.
That is the same fail-open, in the same file, one token over. So `lines` is now
`Vec<(usize, String)>` holding **code text**, and the module's doc claim ("a comment naming the
thing a gate hunts for would otherwise satisfy the gate") is true for the first time.

Entering a comment mid-line requires knowing where the literals are, so `code_text` lexes `"`,
`'`, `b"`, `r"` and `r#"..."#`. A lifetime is **not** a char literal — reading `'a` as one would
consume to the next quote and swallow any `/*` between them, which is the fail-open direction
again. Nested block comments were already handled and still are. `attribute_span` now strips
comments before counting brackets, so a trailing `// (` can no longer extend an attribute's span
over the item it gates.

**Limits, now stated in both directions.** String state is deliberately NOT carried between lines,
so a `//`- or `/*`-shaped line inside a MULTI-LINE string literal still reads as a comment. That
is the false-DROP direction (loud for a needle-must-be-present gate). Carrying it would let one
mis-lexed quote swallow the rest of a file silently, and unlike `unterminated_block_comment` there
is no cheap check that could see that.

**Measured** with the shipped classifier compiled verbatim via `#[path]` into a standalone driver:
all four scanned files unchanged at this commit — `agent_daemon/launch.rs` `regions=3
skipped=1728 unclosed=[] unterm=None leaked=[]` with the `DaclAncestorTraverse` production site
still at **1466**. The WR-02 fixture goes from keeping `Windows Application event log` in the
production half to dropping both it and the `NOTE` tail of the line that opened the comment.

**PERTURBATION:** restoring the line-leading-only rule fails both new fixtures
(`a_block_comment_opened_mid_line_is_comment_text`,
`a_trailing_comment_is_not_production_text`). Reverted.

---

### WR-01: skip the cfg-test-gated ITEM, whatever kind of item it is

**File:** `cfg_test_regions.rs` · **Commit:** `8bd047fb`

The fall-through comment said "It is not production text either way" and the next statement pushed
it as production text. They agree now, on the code's side: the gated item is skipped whatever kind
of item it is.

Confirmed live with the driver before fixing:

```
agent_daemon/launch.rs  356      pub(crate) const WFP_CONTROL_PIPE_NAME_TESTABLE ...
                        363-365  pub(crate) fn profile_needs_network_scoping_testable ...
output.rs               501      pub(crate) static SESSIONS_ROOT_TEST_LOCK ...
```

The extent rule is the item's own shape over code text: brace-balanced and ending in `;` or `}`
is a one-line region; a header that opens a block ends at the closer at the item's **own indent**
— the rule `mod` has always used, chosen over brace counting because a brace inside a raw string
then closes early in the LOUD direction (a leaked `#[test]`) instead of swallowing the rest of the
file; anything unresolved after 16 lines goes to `unclosed_regions` so it FAILS rather than
picking a silent direction.

A bare `#[cfg(test)] mod foo;` is now a one-line region rather than production text. That also
closes the one-line-module shape WR-04 named: `#[cfg(test)] mod t { #[test] fn q(){} }` is now
skipped at source rather than needing the leak check to catch it.

**Measured, after:** every new region is exactly the gated item —

```
agent_daemon/launch.rs  356..356, 363..365, 1815..1854, 1861..2247, 2254..3554  regions 3->5, skipped 1728->1732
output.rs               501..501, 1531..1818, 1821..2532                        regions 2->3, skipped 1000->1001
main.rs                 156..156 (mod test_env;), 163..163 (mod cfg_test_regions;), 323..1189   regions 1->3
exec_strategy_windows/launch.rs   unchanged, 12 regions / 3346 lines
```

`unclosed=[]`, `unterm=None`, `leaked=[]` on all four; `DaclAncestorTraverse` still at 1466;
WR-08's per-file literal counts unchanged at **429 / 759 / 66 / 26 / 204**.

**PERTURBATION:** restoring the fall-through fails all three new fixtures. Reverted.

**Known, deliberate, loud:** a `static X = Lazy::new(|| { ... });` gated by `#[cfg(test)]` would
have its closer written `});`, which `is_region_closer` rejects, so it would be reported as an
unclosed region and FAIL the build. No such shape exists in the four scanned files. That is the
over-claim-averse direction this module already chose for `mod`, applied consistently rather than
silently narrowed.

---

### WR-04: ask the leak question of every attribute ON the line

**File:** `cfg_test_regions.rs` · **Commit:** `388cf4bc`

`leaked_test_attributes` asked `is_test_attr(l.trim())`, so round 7's widened predicate was
unreachable unless the attribute stood alone on its line. Widening a predicate without widening
its consumer is the same shape one level out.

`attributes_on_line` extracts every `#[...]` with brackets matched, skipping string literals with
the same `literal_len` rule `code_text` uses — so a `]` inside `#[doc = "a]b"]` does not end the
attribute, and a `"#[test]"` written as TEXT is not a leak. Extraction is asserted as a rule over
11 inputs; the leak itself is asserted directly and through a `#[should_panic]` proof.

**Measured:** `leaked=[]` still on all four scanned files, so the widening adds no false positive.

**PERTURBATION:** restoring the line-anchored consumer fails both new fixtures. Reverted.

---

### WR-03: narrow the marker parser to the word, and assert the width relation

**File:** `tests/layer_registry_selfcheck.rs` · **Commit:** `f2d638df`

The detector's doc claimed "strictly wider" and was false on three classes. Fixed in the
**parser**, which is where the asymmetry belongs — widening the detector to match would have made
the blind-file property vacuous by construction, which is the trap round 7 correctly avoided in
the other direction. The rule now lives in one place, `parse_marker_ids`, called by both the
consumer and the self-test instead of each re-spelling `match_indices(" OPEN")`.

`-` counts as a word character there and that is load-bearing rather than incidental: the detector
trims tokens to alphanumerics and `-`, so `OPEN-ish` reaches it whole as `OPEN-ish` and not as
`OPEN`. Admitting `-` as a terminator would have re-opened the same hole one character over — I
found that by working the relation through rather than by testing, and then pinned it.

The one-input width check is now a table of 20 lines asserting `parser fires => detector fires`,
with a **non-vacuity floor on both halves**: a parser that never fires satisfies the implication,
and a detector that has converged on the parser can never report a blind file. The three classes
that broke the claim are pinned individually.

**PERTURBATIONS.** Widening the parser back fails the table naming
`"// WR-14 OPENING the job handle"`. Rewriting the live `WR-10 OPEN` marker as `WR-10 (OPEN)`
fails the consumer naming `layer_registry.rs` blind and listing `error.rs` as the file that did
parse. Adding `OPEN_EXISTING` plus `CR-01` and `CR-99 OPENS` mentions to `attestation.rs` stays
GREEN. All reverted.

---

## Round 6's perturbation table, re-run

Shipped classifier compiled verbatim via `#[path]`; real `agent_daemon/launch.rs`; the gate
verdict is the REAL `cargo test` run, not an emulation. Baseline fields differ from round 6's
because WR-01 changed the classifier — that is the point of reporting them.

| # | Perturbation | Scan | Real daemon gate |
|---|---|---|---|
| baseline | none | `regions=5 skipped=1732 unclosed=[] unterm=None leaked=[] sites=[1466]` | **PASSES** |
| P1 | blank line before `mod attestation_gate_tests {` | **identical to baseline in every field** | **PASSES** |
| P2 | P1 + rename production `layer = "DaclAncestorTraverse"` -> `DaclAncestorWalk` | `regions=5 skipped=1732 sites=[]` | **FAILS** |
| P3 | the rename alone | `regions=5 skipped=1732 sites=[]` | **FAILS** |

P2 and P3 fail with the intended message ("... never names `"DaclAncestorTraverse"` outside a
comment — the row goes completely unattested on `nono agent launch` (CR-02)"). Every perturbation
reverted; `git status --porcelain` clean.

All `#[should_panic]` proofs pass: `WR-02` (unterminated block comment), `WR-03` (unclosed
region), `WR-01/WR-02` (leaked attribute) **twice** — the pre-existing one and the new
line-sharing one — and non-vacuity.

---

## What I checked in my own work before committing

**Two instances found, and both were in my harness rather than in the code.** Both are recorded in
their commit messages rather than quietly re-run.

1. **The WR-03 perturbation P-I PASSED on its first run**, which would have read as "the gate no
   longer catches an adjacency-broken marker". The gate was fine; my substitution was not. The
   marker line carries `WR-10 OPEN` **twice** — once as the marker, once inside a
   ``grep `WR-10 OPEN` `` citation on the same line — and a replace-first-occurrence left the
   second one parsing. Re-run against both occurrences it FAILS correctly. A perturbation that
   cannot distinguish is not evidence; this is the same defect round 7 recorded for its own rename
   harness, in a different disguise.
2. **The WR-04 perturbation's REVERT silently mis-applied.** The revert matched the doc comment
   that QUOTES the old code before it matched the code, so the file came back half-reverted and
   two tests stayed red. Repaired with unique-text edits, not first-occurrence substitution.

Three further things I wrote could each have been the class:

3. **The CR-01 longest-suffix resolution could have narrowed the gate silently** — a site
   resolving to one wrong file that happens to be force-included is a green that proves nothing.
   Tested by dropping each of the four `ci.yml` force-include clauses in turn IN THE REAL TREE
   (P-A, P-B, P-K, P-L): each produces a named failure for its own file, so all four are
   load-bearing and the gate classifies four distinct repository files.
4. **The WR-02 rewrite could have swallowed production text** through a mid-line `/*` inside a
   multi-line string. Measured directly: `unterm=None` and `unclosed=[]` on all four files, kept
   counts moving by exactly the WR-01 item lines and nothing else.
5. **The WR-04 widening could have produced false leaks** from `#[test]` written inside string
   literals or trailing comments. Measured: `leaked=[]` on all four files, and both shapes are
   pinned as fixtures.

---

## Earlier rounds re-confirmed at final HEAD (`f2d638df`)

Sampled against the source and by running the gates, not read from a fix report.

| Fix | Evidence | Verdict |
|---|---|---|
| `is_cfg_test_attr` three-valued rule | `enum Tri` at `:152`, `MAX_CFG_DEPTH` at `:246`; `cfg_test_attr_classification_rule` and `nested_cfg_predicates_compose` pass | **Intact** |
| WR-04 D-28 fail-closed enumeration | `launch.rs:4699` `UNREADABLE DIR`, `:4718` `UNREADABLE ENTRY` | **Intact** |
| WR-07 real-key-space sweep | `launch.rs:4591 marker_content_is_injective_over_the_real_layer_vocabulary` | **Intact** |
| CR-01 digest marker | `output.rs:226` writes only `attestation_downgrade_marker_content(dedup_key)`; `:307` reader compares content | **Intact** |
| `assert_split_is_correct` propagation | 3 consumer call sites (`layer_registry.rs:1469`, `output.rs:2443`, `:2508`), one implementation, no consumer writes its own floor | **Intact** |
| WR-02 (FFI) | `types.rs:218 LayerAttestationFailed = 15`; `nono.h:146`; `lib.rs:211` arm | **Intact** |
| WR-08 per-file literal floor | measured 429 / 759 / 66 / 26 / 204 — unchanged from round 7's final HEAD | **Intact** |
| WR-11 reserved device names | `output.rs:471 RESERVED`, `:482 eq_ignore_ascii_case` | **Intact** |
| CR-03 / RF-13 | `machine_policy.rs:197 RequiredLayersPolicy`, `:268` reader doc | **Intact** |
| WR-16 (`log_target_is_private`) | `cli_bootstrap.rs:101` | **Intact** |
| CR-02 / WR-12 records | `error.rs:506 WR-14 OPEN`, `layer_registry.rs:925 WR-10 OPEN`, both with OPEN ledger rows (`every_open_marker_in_code_has_a_ledger_row` passes under the NARROWED parser) | **Still match the code** |
| Standing rules | no `.unwrap()`/`.expect()` anywhere in `cfg_test_regions.rs`; saturating arithmetic throughout the new scanners; no string `starts_with` on a path | **Clean** |

---

## Verification

| Gate | Where | Result |
|---|---|---|
| `cargo fmt --all -- --check` | REAL TREE | **PASS** |
| Windows `cargo clippy -p nono-sandbox-cli --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | REAL TREE | **PASS** |
| `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | worktree | **PASS** (exit 0), 40m46s |
| `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) | worktree | **PASS** (exit 0), 2m47s |
| `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` | **REAL TREE** | **20 passed, 0 failed** |
| `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` | **REAL TREE** | **10 passed** |
| `cargo test -p nono-sandbox-cli --test layer_force_unavailable` | **REAL TREE** | 0 (platform-filtered) |
| `cargo test -p nono-sandbox-cli --bin nono` | **REAL TREE**, quiet | **1688 passed, 12 failed** |
| `cfg_test_regions` unit tests | REAL TREE | **35 passed** (was 25) |
| `cargo test --workspace --no-fail-fast` | worktree | 20 failed, **all pre-existing** — see below |

**Both cross-target gates ran locally to completion — no PARTIAL-to-CI fallback.** Both were run
because `output.rs` carries cfg-gated blocks and because `layer_registry_selfcheck.rs` compiles
and runs on every target; the new `std::process::Command` call to `git` is platform-neutral and
both gates cover it.

`layer_registry_selfcheck` is **fully green in the real tree**, which is the specific thing round
7 could not have known.

### Regression baseline

| | Passed (`--bin nono`) | Failed |
|---|---|---|
| Phase base `334530af` (round 2) | 1641 | 14 |
| Round-3 HEAD `860d4772` | 1657 | 14 |
| Round-5 HEAD `f7cea97f` | 1669 | 14 |
| Round-7 HEAD `f18fafe2` | 1678 | 14 |
| **This branch `f2d638df`, REAL TREE** | **1688** | **12** |

**Zero regressions; +10 tests over round 7.**

**On 12 versus 14, plainly.** The recorded Windows-host baseline is a list of **14 names**. In the
real tree on a quiet machine, **12** of them fire; the two that did not are
`broker_dispatch_tests::broker_launch_assigns_child_to_job_object` and
`write_deny_low_il_broker_no_pty_tests::...`, which is exactly what round 8 observed. In the
worktree under load all **14** fired. So the honest statement is: the baseline list has 14 names,
12 fire deterministically and 2 are load-sensitive, and no failure outside that list survives a
quiet run. No phase-117 gate is among them.

### Workspace baseline: the recorded figure was 16, the real one is 20

`--workspace --no-fail-fast` shows **20** failures. Round 7 recorded 16. I checked the four extras
against the review base `b338c842` rather than assuming:

| Extra failure | At `b338c842` |
|---|---|
| `resl_nix_async_signal_safety::cr_01_no_format_macro_in_post_fork_child_branch` | **FAILS identically** |
| `env_vars::windows_run_allow_all_network_probe_connects` | **FAILS identically** |
| `env_vars::windows_run_blocks_live_block_net_without_enforcement` | **FAILS identically** |
| `env_vars::windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified` | **FAILS identically** |

All four pre-date this round. Round 7's own correction ("the workspace baseline on this host is
16, not 14") was right in direction and still short: **on this host it is 20**. This is the third
time in this phase a recorded baseline has turned out to be an undercount, and each time the cause
was the same — the number was carried forward rather than re-measured.

---

## Left open, unchanged — recorded operator decisions

1. **CR-02 defect 1** — does the daemon arm consume the registry? The gate deliberately proves "no
   daemon-expected row is unmentioned", and `layer_registry.rs:1410-1423` still says so.
2. **WR-12 + WR-14 are one change.** Both markers say so; the SPEC has a WR-14 row.
3. **WR-10** — accept the lockstep two-binary rename, or keep one `LayerId` covering two kernel
   objects with the divergence documented.
4. **WR-14 is reachable for embedders today** — `nono::attestation::probe_in_job` is public API.

The CR-01 disclosure argument from round 3 stands unmodified.

### Judgement calls that belong to a human

**`.planning/PROJECT.md` still runs the full CI matrix when it changes** (round 7's call,
unchanged here). The alternative remains dropping `tests/adr_aipc_unix_futures.rs`'s assertion on
it.

**`ProductionScan::lines` changed type** from `Vec<(usize, &str)>` to `Vec<(usize, String)>`. That
is a wider change than WR-02 literally asked for, and I made it because fixing only the
continuation lines would have left the identical fail-open on the line that opens the comment. If
that reading of "the class, not a rendering" is wrong, the narrower fix is the mid-line lexing
alone and the type can go back — but the daemon gate is then satisfiable by
`let x = 1; /* DaclAncestorTraverse */`.

**`nono/src/sandbox/windows.rs:2844`** — round 8 resolved this to "no finding, the fixer's decision
to leave it stands". Unchanged.

---

## Commits

| # | Hash | Finding |
|---|---|---|
| 1 | `70691d04` | CR-01 (the BLOCKER; taken first) |
| 2 | `fc378757` | WR-02 |
| 3 | `8bd047fb` | WR-01 |
| 4 | `388cf4bc` | WR-04 |
| 5 | `f2d638df` | WR-03 |

---

_Fixed: 2026-08-14_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 5 (round 9)_
