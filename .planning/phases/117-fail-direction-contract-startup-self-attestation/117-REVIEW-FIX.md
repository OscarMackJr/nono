---
phase: 117-fail-direction-contract-startup-self-attestation
fixed_at: 2026-08-15T02:10:00Z
review_path: .planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md
iteration: 4
round: 7
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 117: Code Review Fix Report — round 7

**Source review:** `117-REVIEW.md` (round 6, reviewed 2026-08-14T23:40:00Z; 0 Critical + 7
Warning = 7 findings, all in scope)
**Scope:** `critical_warning` — the review reported 0 Info findings, so this is every finding.

> Round-5's fix report is preserved at `117-REVIEW-FIX.round5.md`; round 3's at
> `117-REVIEW-FIX.round3.md`; round 1's at `117-REVIEW-FIX.round1.md`.

| | Count | Findings |
|---|---|---|
| Fixed | 7 | WR-01, WR-02, WR-03, WR-04, WR-05, WR-06, WR-07 |
| Skipped | 0 | — |

5 commits on `gsd-reviewfix/117-r7-1166`, fast-forwarded onto
`milestone/v2.13-carryforward-closeout`. All carry DCO sign-off.

**One finding has runtime security consequence (WR-04) and it went first.** The other six are
guard, record and CI-configuration defects. No production behaviour changed this round: the only
non-test production edits are documentation and one `fn` -> `pub(crate)`.

---

## Requirement 5 (first, because it is the regression question)

Round 5's fix to `ProductionScan::assert_split_is_correct` is **not regressed**. I re-ran round
6's own method rather than trusting either report: the SHIPPED `cfg_test_regions.rs` was compiled
verbatim via `#[path]` into a standalone driver (byte-identical to what ships, not a replica) and
run against the real `agent_daemon/launch.rs`.

| # | Perturbation | Scan result | Emulated gate | Round-6 expectation |
|---|---|---|---|---|
| baseline | none | regions=3, skipped=1728, unclosed=[], unterminated_block=None, leaked=[], `DaclAncestorTraverse` production site `[1466]` | **PASSES** | matches |
| P1 | blank line before `mod attestation_gate_tests {` (line 1861) | **identical to baseline in every field** | **PASSES** | matches |
| P2 | P1 + rename production `layer = "DaclAncestorTraverse"` (line 1466) | regions=3, skipped=1728, sites `[]` | **FAILS** | matches |
| P3 | rename alone | regions=3, skipped=1728, sites `[]` | **FAILS** | matches |

`assert_split_is_correct` PASSES in all four, correctly: none of these perturbations produces a
leak, and P2/P3 are caught by the gate's own needle, which is the intended division of labour.

**P2 re-run for real**, not emulated: applied to the working tree and run through
`cargo test -p nono-sandbox-cli --bin nono -- daemon_expected_rows_are_all_named_by_the_daemon_gate`.
It **FAILED** with the intended message:

```
DaclAncestorTraverse is expected at (EntryPath::Daemon, None) with an attestable probe
(ConfiguredOnly), but daemon_attest_and_decide's source never names "DaclAncestorTraverse"
outside a comment — the row goes completely unattested on `nono agent launch` (CR-02).
```

The three (now four) `#[should_panic]` proofs still pass. Every perturbation was reverted;
`git status --porcelain` is empty at final HEAD.

> **Method note.** My first emulation renamed the site to `DaclAncestorTraverseRENAMED`, which
> still CONTAINS the needle, so P2 and P3 both reported PASSES. That was my harness being wrong,
> not the guard. Re-run with a genuine rename (`DaclAncestorWalk`) the table above is what came
> out. Recording it because a perturbation that cannot distinguish is the same defect this round
> is about, one level out.

---

## Fixed

### WR-04: fail closed on the enumeration that feeds the D-28 scan

**File:** `exec_strategy_windows/launch.rs` · **Commit:** `d79f831b`

The only finding with runtime security consequence, so it went first. Round 5 fixed
`let Ok(bytes) = fs::read(..) else { continue }` and left the identical shape in the function that
produces `files`, thirty lines up. `collect_files` now reports unreadable directories AND
unreadable entries to the caller, and `violations` is seeded from them before the per-file loop.

**PERTURBATION (the decisive one):** a second `collect_files` call against a nonexistent nested
path leaves `files.len() == 1`, so the round-5 `assert_eq!(files.len(), 1)` **still PASSES** —
confirming the review's claim that it is structurally blind to a nested unreadable subtree — and
the gate then **FAILS** naming the subtree. Reverted.

The summary assertion's message named only the plaintext class; it now names both disqualifying
classes, because "this file was never scanned" is not "this file is clean".

#### The full class sweep (priority 1's second half)

Every `let Ok(..) = <fs op> else`, `.flatten()`, `.ok()`, `unwrap_or_default()` and
`filter_map(Result::ok)` in the 27 in-scope files, with the question "if this fails, does the
guard or the sandbox end up claiming more than it checked?".

| Site | Shape | Verdict |
|---|---|---|
| `ex_win/launch.rs:4604` `let Ok(entries) = read_dir else return` | fail-OPEN | **FIXED** |
| `ex_win/launch.rs:4607` `entries.flatten()` | fail-OPEN | **FIXED** |
| `selfcheck.rs:1614` `if let Ok(src) = read_to_string` (CI sync gate's file list) | fail-OPEN — silently narrows the discovery | **FIXED** in WR-05 |
| `ex_win/launch.rs` D-28 per-file `match fs::read` | fail-closed (round 5) | intact |
| `cli_bootstrap.rs:126` `let Ok(canonical) = canonicalize else return false` | fail-SECURE — "not private" closes the D-28 detail gate | kept |
| `cli_bootstrap.rs:357` `.ok().flatten()` on the user config THEME | not security-relevant (UI colour) | kept |
| `cli_bootstrap.rs:393` `telemetry_config.unwrap_or_default()` | not a filesystem read | kept |
| `ex_win/mod.rs:1275` `fs::read(exe).ok()?` in `read_distlib_shebang` | fail-SECURE — documented DIAGNOSTIC ASSIST ONLY, never auto-grants (D-07/T-71-13); `None` = no candidate | kept |
| `ex_win/launch.rs:628` `file_name()…unwrap_or_default()` | not a read; an unknown program name falls to the pass-through arm of `prepare_runtime_hardened_args`, which adds no interpreter hardening rather than removing any | kept |
| `ex_win/launch.rs:1131` `let Ok(output) = icacls … else return false` | fail-SECURE — the directory is not treated as safely writable | kept |
| `network.rs:140/145` `entries.flatten()`, `let Ok(file_type) else continue` in `copy_program_siblings` | a skipped entry is NOT staged into the sandbox dir — restrictive direction (an availability risk, not a confinement one) | kept |
| `network.rs:293` `if let Ok(entries) = read_dir` in `cleanup_stale_network_enforcement_artifacts` | best-effort cleanup; failing to clean LEAVES firewall BLOCK rules in place | kept |
| `network.rs:1568` `let Ok(config) = current_wfp_probe_config() else` | fail-SECURE — returns a "probe failed" report and grants nothing | kept |
| `output.rs:291` `matches!(read_to_string(..), Ok(e) if ..)` in `marker_says_already_announced` | fail-SECURE and documented — every abnormal state resolves to ANNOUNCE | kept |
| `output.rs:~2432` `.unwrap_or_default()` on the WR-26 arm walk-back | fail-CLOSED — an empty arm fails `starts_with("EventLog")` and trips the assert | kept |
| `query_ext.rs:416` `let Ok(glob) = Glob::new(&r.path) else return false` | fail-SECURE — an unparseable endpoint rule NARROWS the allow set | kept |
| `meta_test.rs:354,397`; `selfcheck.rs:424,807,1519` `match read_to_string { Err => violation }` | fail-closed | kept |
| `selfcheck.rs:1137,1240,1328,1342,1376,1599`; `labels_guard.rs:1401,1441` `.expect` / `unwrap_or_else(panic)` | fail-closed | kept |
| `agent_daemon/launch.rs:1037,1067,1482,1486`; `ex_win/launch.rs:3783` `if let Ok(x) = ….lock()` | mutex poisoning, not a filesystem or security read — outside the class | noted |
| `nono/src/sandbox/windows.rs:2844` `file_name()…unwrap_or_default()` | pre-existing and outside the phase's change set; it selects which interpreter's ARG validator runs, not whether a capability is granted. **Not changed, and I did not establish a verdict on it** — flagged here rather than silently cleared | noted |

Two fail-open sites found, both fixed. No other site in the class was in the wrong direction.

---

### WR-01 + WR-02 + WR-03: state each classifier rule once, by class

**File:** `cfg_test_regions.rs` · **Commit:** `21b2246a`

Three narrownesses in the module that is the single point of failure for every gate in the phase.
Each is now ONE rule in ONE place, per the brief.

**WR-01 — the rule is EVALUATED, not pattern-matched.** `#[cfg(any(test, feature = "x"))]`
contains `test,` and not `not(test`, so it classified as a test gate and the classifier deleted a
PRODUCTION module's body. The predicate is now parsed into a tree, `test` is bound to `false`,
every other atom is left UNKNOWN, and the attribute is a test gate iff the three-valued result is
definitely `False` — i.e. "the code cannot exist in a non-test build". One rule settles `all`,
`any`, `not` and every nesting:

| Attribute | With `test = false` | Test gate? |
|---|---|---|
| `#[cfg(test)]` | `False` | yes |
| `#[cfg(all(test, target_os = "windows"))]` | `all(False, ?)` = `False` | yes |
| `#[cfg(any(all(test, unix), all(test, windows)))]` | `any(False, False)` = `False` | **yes** |
| `#[cfg(any(test, feature = "x"))]` | `any(False, ?)` = `?` | **no** |
| `#[cfg(not(test))]` | `True` | no |
| `#[cfg(all(any(test, feature = "x"), unix))]` | `?` | no |

The third row is why this had to be a rule and not a list: an `any`-headed predicate CAN be
test-only. An unparseable predicate fails toward NOT a test gate, so the body stays in the
production half where a leak is loud; the opposite default deletes source silently.

The live shape is `session_commands.rs`'s
`#[cfg(any(test, target_os = "macos", target_os = "windows"))] fn format_bytes_human`, and it is
in the rule test as a fixture. There is also an end-to-end test that an `any(test, ..)` module
survives the scan whole — that direction is invisible to all three checks in
`assert_split_is_correct`, so it has to be asserted directly.

**WR-02 — one comment rule, defined positively.** There were TWO comment rules and both were
enumerations: the whole-file scan dropped lines starting with `//` (block comments were kept as
production text, so a `/* … */` naming a gate's needle satisfied that gate), and the pending
window admitted `""` / `//` / `/*` / `*`. Both are now `line_has_code`: a line contributes code
unless everything on it is whitespace or comment. Block comments nest, so the carried state is a
depth counter, and it is carried through region bodies too — so a `}` inside a block comment can
no longer close a region. **The pending window now has no shape list at all**, because between an
attribute and the item it gates Rust permits attributes, comments and blank lines and nothing
else, so "not a code line" IS the window.

The one limit is stated rather than hidden: block-comment state is only ENTERED from a line whose
trimmed text starts with `/*`, never mid-line, which keeps this free of string- and char-literal
lexing at the cost of one accepted false drop (a comment-shaped line inside a multi-line raw
string). The pre-existing `//` rule has always had exactly that limit; this extends it
symmetrically rather than adding a class.

A block comment left open at EOF would swallow the rest of the file — the over-claim direction
again — so it is recorded in the new `unterminated_block_comment` and asserted FIRST in
`assert_split_is_correct`, with a fourth `#[should_panic]` proof.

**WR-03 — the leak check matches the class.** `is_test_attr` was two exact forms under a doc
comment that already called itself a class, so `#[tokio::test(flavor = "multi_thread")]` was not a
test attribute to it. Since `leaked_test_attributes` is the ONLY under-claim check there is, that
width decides whether a classifier gap like WR-02's is caught or silent. The rule is now "last
path segment, argument list stripped, is `test`". Two bare-ident harness attributes (`#[rstest]`,
`#[test_case]`) are matched by NAME and the doc says so — no shape rule can reach them, and
calling an enumeration a rule is the thing this round exists to stop.

25 unit tests, up from 15. Measured at that commit: WR-08's per-file literal counts unchanged at
429/757/66/26/204, i.e. the widened comment rule moved no production text on any scanned file.

**Two-direction:** test-only. Denies more; exposes nothing.

---

### WR-05: make the CI sync gate's discovery actually discover

**Files:** `tests/layer_registry_selfcheck.rs`, `.github/workflows/ci.yml` · **Commit:** `a8211eb4`

The "discovery" was five hardcoded files out of ~170. It now walks every `.rs` under the crate's
`src/` and `tests/` and **fails CLOSED** on any enumeration error — a discovery gate's coverage IS
its walk, which is the WR-04 shape one commit earlier.

A `"….md"` fragment counts as a repo read when a repo-root marker appears within 8 lines above
it. That window is what separates a real read from the far more common
`tempdir.path().join("CLAUDE.md")` fixture WRITE, and what reaches the house idiom where the root
is bound a few lines up. Basenames resolve by walking the workspace instead of guessing three
trees; ambiguity is fail-CLOSED (every location the name could denote must be covered).

Making discovery real found **three** files, not one:

| File | Read by | Was |
|---|---|---|
| `crates/nono-cli/data/profile-authoring-guide.md` | `config/embedded.rs` (`include_str!` into the shipped binary), asserted by `profile_cmd.rs` | the review's cited counterexample |
| `docs/architecture/aipc-unix-futures.md` | `tests/adr_aipc_unix_futures.rs` | **newly surfaced** |
| `.planning/PROJECT.md` | `tests/adr_aipc_unix_futures.rs` | **newly surfaced** |

The old three-tree guess could not even see the last two — it would have reported them
"unresolved", a narrower claim wearing a different message. All three are force-included.
`.planning/PROJECT.md` is force-included as an EXACT PATH rather than as a `^\.planning/` tree,
because that tree is written by every planning command and only one file in it is gated by a test;
this preserves the cost reasoning round 6 upheld. The coverage rule accepts either an ancestor
`^dir/` prefix or the file's own quoted path.

**I caught my own instance of the class here, and only because I ran the perturbation.** The
review asked for comment lines to be stripped from the workflow text. I did that AND scoped the
needle to the classifier loop body — and with both new tree clauses deleted the gate **still
PASSED**. The loop body also holds the docs-only EXCLUSION list (`(^docs/)|…`) and the
`run_docs_checks` test (`(^crates/nono-cli/src/cli\.rs$)`), so `^docs/` and `^crates/nono-cli/`
were both findable as substrings of clauses that force-include nothing — a gate satisfied by the
very list it exists to override. The region now starts AFTER the negated exclusion test's `]]`.

**PERTURBATIONS:** delete the two new tree clauses -> **FAILS**, naming both files with their real
read sites. Move a clause into a YAML comment only -> **FAILS**. Delete the exact-path
`.planning/PROJECT.md` clause -> **FAILS** naming it. All reverted.

---

### WR-06: trigger the marker property on the class, and count per file

**File:** `tests/layer_registry_selfcheck.rs` · **Commit:** `04bf35ca`

The trigger was the bare substring ` OPEN` while the class is `XX-NN OPEN`; three of the five
scanned files are Win32-facing, so an `OPEN_EXISTING` added to any of them would report that file
"blind" with a message pointing at an unrelated deferral record.

**The obvious fix is the wrong one and I want that on the record.** The review's suggested
`line_has_marker_shape` restates the parser's own rule in the trigger. Do that and trigger and
parse agree on every input, and the blind-file property can never fire again — vacuous by
construction, which is this round's subject. So the detector is deliberately INDEPENDENT: it
tokenises the whole line (`OPEN` whole word AND a finding-id token anywhere), while the parser
requires ADJACENCY. Because they differ only in adjacency, rewriting a marker as `WR-14 (OPEN)` or
`WR-14 — OPEN` still trips the detector and blinds the parser — exactly the failure the property
exists to catch — while `dwCreationDisposition: OPEN_EXISTING` trips neither.

The per-file bookkeeping was also wrong: a file counted as having parsed a marker only when the
GLOBAL list grew, and that list dedups by id, so a second file repeating an id — ordinary when one
deferral is annotated at both its sites — was reported blind. Now a per-file counter.

Adds the detector self-test the sibling CI gate has and this one lacked, including an explicit
assertion that the detector stays strictly wider than the parser.

**PERTURBATIONS:** rewrite the live `WR-10 OPEN` marker as `WR-10 (OPEN)` -> **FAILS** naming
`layer_registry.rs` and listing the files that did parse. Add `OPEN_EXISTING` plus a `CR-01`
mention to `attestation.rs` -> stays **GREEN** (the old trigger would have failed). Repeat
`WR-14 OPEN` in a second scanned file -> stays **GREEN** (the old counter would have called it
blind). All reverted.

---

### WR-07: make the injectivity claim true over the real key space

**Files:** `output.rs`, `exec_strategy_windows/launch.rs` · **Commit:** `f18fafe2`

The record said the sweep covers "the WHOLE reachable key space … every one of the 2^13 reachable
keys" while it enumerates 2^13 subsets of a SYNTHETIC vocabulary. Those are disjoint input sets,
and the test's own comment that "a same-length same-shape vocabulary exercises the digest's input
space identically" is not a property any hash has — nor is it same-length (`Layer00` is 7 bytes,
`DaclAncestorTraverse` is 20).

The brief says not to restate the claim into something weaker than the guard needs, so **both**
halves were done:

- the records now say exactly what the synthetic sweep proves — injectivity over 8192 keys of
  production SHAPE — and name where the real claim lives;
- `attestation_gate_tests::marker_content_is_injective_over_the_real_layer_vocabulary` runs the
  same 2^13 sweep over `layer_registry::ALL`, on the side of the D-28 boundary where importing the
  registry is already legal.

D-28's structural half is preserved: `output.rs` still never names the layer identity type. Only
the digest helper becomes `pub(crate)` — the vocabulary travels to the digest, not the type to
`output.rs`. Non-vacuity is asserted three ways (real vocabulary >= 13 names, exactly `2^len` keys
enumerated, distinct-content count equal to that), plus an upper bound of 20 names so a future
enum growth is a deliberate decision rather than a silently slow test.

**PERTURBATION (the discriminating one):** truncate the digest input to the key's first **103**
characters — the exact length of the longest SYNTHETIC key, so that sweep is untouched and still
**PASSES**, while the real-vocabulary sweep **FAILS**:

```
WR-07: marker-content collision between
"AppContainerProfile,BrokerAuthenticodeTrustGate,DaclAncestorReadAttrs,DaclAncestorTraverse,DaclPackageSidGrant"
and "…,DaclSessionSidGrant" -> 72edda644eedb9a1c110caf7751243b8
```

That gap is precisely the coverage the old record claimed and did not have. Reverted.

---

## Priority 6: what I checked in my own work before committing

Three of the things I wrote could each have been the class I was fixing. Two were, and I found
them by perturbation rather than by reading:

1. **The WR-05 workflow needle** was satisfied by the exclusion list it exists to override.
   Caught by P-A; re-scoped. Described above.
2. **My WR-05 basename-to-tree resolution** was initially statement-scoped, which silently dropped
   two real repo reads whose path root is bound in a separate statement. Caught by running the
   discovery against the real tree and reading the output rather than the count; widened to an
   8-line look-back and re-measured at 4/8/12/16 to confirm the result is stable rather than
   tuned.
3. **The WR-06 detector** was NOT made a copy of the parser, for the reason given above; the
   non-vacuity of that choice is itself asserted in the self-test.

And one harness error: the requirement-5 rename perturbation initially produced a string that
still contained the needle. Recorded in the perturbation section rather than quietly re-run.

---

## Priority 8: earlier rounds re-confirmed at final HEAD (`f18fafe2`)

Sampled against the source, not read from a fix report.

| Fix | Evidence at final HEAD | Verdict |
|---|---|---|
| WR-02 (FFI) | `types.rs:218 LayerAttestationFailed = 15`, `:244` conversion arm; `nono.h:146 NONO_DIAGNOSTIC_CODE_LAYER_ATTESTATION_FAILED = 15`; `lib.rs:211` `NonoError::LayerAttestationFailed` arm | **Intact** |
| WR-08 (probe-kind keyed by `LayerId`) | `exec_strategy::layer_registry::tests` 10/10 pass, incl. `exactly_one_row_is_confirmed_by_enforcing_component_report` | **Intact** |
| WR-11 (reserved device names) | `output.rs:471` `RESERVED` list, `:482` `eq_ignore_ascii_case` rejection | **Intact** |
| CR-03 / RF-13 | `machine_policy.rs:197 RequiredLayersPolicy`, `:716 read_required_layers`, `:763` reader; SPEC `:262` row still says "reads the sub-key into `RequiredLayersPolicy.required`" | **Intact and consistent** |
| WR-16 (`log_target_is_private`) | `cli_bootstrap.rs:101`, all four fail-secure early returns present | **Intact** |
| CR-01 digest marker | `output.rs:226` writes only `attestation_downgrade_marker_content(dedup_key)`; `:307` reader compares content; `:427` `.v2` suffix | **Intact**, and now additionally bound by the real-vocabulary injectivity sweep |
| CR-02 rationale | `layer_registry.rs:1410-1423` SCOPE paragraph still says "no daemon-expected row is unmentioned", not "the registry drives the daemon" | **Still matches the code** |
| WR-12 / WR-14 rationale | `error.rs:506` `WR-14 OPEN`, `layer_registry.rs:925` `WR-10 OPEN`; both have OPEN ledger rows (`every_open_marker_in_code_has_a_ledger_row` passes, now under the widened detector) | **Still matches the code** |

CR-02 and WR-12 remain **open operator decisions** and were not re-litigated.

---

## Verification

All gates at final HEAD (`f18fafe2`) unless noted.

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | **PASS** |
| Windows `cargo clippy -p nono-sandbox-cli --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | **PASS** (exit 0) |
| `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) | **PASS** (exit 0), 2m23s |
| `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** (exit 0), 15m17s |
| `cargo test -p nono-sandbox-cli --bin nono` | 1678 passed, **14 failed** (baseline) |
| `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` | **20 passed** (was 19) |
| `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` | **10 passed** |
| `cfg_test_regions` unit tests | **25 passed** (was 15) |
| `cargo test --workspace --no-fail-fast` | 1678 + 844 + 97 + 51 + 40 + 25 + 18 + smaller targets passed; 14 + 2 failed (both pre-existing — see below) |
| WR-08 measured counts at HEAD | `output.rs 429, launch.rs 759, attestation.rs 66, attestation_downgrade_event.rs 26, main.rs 204` |

**Both cross-target gates were run locally and to completion — no PARTIAL-to-CI fallback.** Both
were run because `output.rs` and `exec_strategy_windows/launch.rs` carry cfg-gated blocks and
because the new `layer_registry_selfcheck.rs` walk compiles and runs on every target.

**On `launch.rs` 757 -> 759.** The WR-08 extractor counts literals over the WHOLE file (it skips
only `//` lines), so the +2 is exactly the two closed single-line literals the WR-07 sweep added
(`"{id:?}"` and the `","` join separator); its multi-line assert messages are backslash-continued
and are not counted. No production text moved — the count was still 757 at the commit that
widened the comment rule.

### Regression baseline

The 14 `--bin nono` failures are the known Windows-host baseline, **identical by name** to the
list recorded in rounds 3 and 5 (`config::tests::*` HOME/USERPROFILE env races x6,
`protected_paths::tests::*` x3, `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`,
`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`,
`exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`,
`exec_strategy::launch::broker_dispatch_tests::broker_launch_assigns_child_to_job_object`,
`exec_strategy::launch::write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file`).

| | Passed (`--bin nono`) | Failed |
|---|---|---|
| Phase base `334530af` (recorded round 2) | 1641 | 14 |
| Round-3 HEAD `860d4772` | 1657 | 14 |
| Round-5 HEAD `f7cea97f` | 1669 | 14 |
| **This branch `f18fafe2`** | **1678** | **14** |

**Zero regressions; +9 tests over round 5.**

**Two additional workspace failures I did not inherit a record for**, and which I checked rather
than assumed: `audit_attestation::audit_verify_reports_signed_attestation_with_pinned_public_key`
and `audit_attestation::rollback_signed_session_verifies_from_audit_dir_bundle`. I ran that test
target at the review base `88a58936` **in the same worktree** and both fail there identically.
They are pre-existing and worktree-environment-sensitive, not a regression from this round.
Round 5's report recorded 14 workspace failures and did not list these two, so the honest
statement is that the workspace baseline on this host is 16, not 14, and round 5's figure was
incomplete — the same shape as the "real figure is 11, not 4" correction already on record for
this host's `-p nono-cli` baseline.

---

## Left open, unchanged

1. **CR-02 defect 1** — operator decision: does the daemon arm consume the registry? The gate
   deliberately proves "no daemon-expected row is unmentioned", and its doc says so.
2. **WR-12 + WR-14 are one change.** Both markers say so; the SPEC has a WR-14 row.
3. **WR-10** — operator decision: accept the lockstep two-binary rename, or keep one `LayerId`
   covering two kernel objects with the divergence documented.
4. **WR-14 is reachable for embedders today** — `nono::attestation::probe_in_job` is public API.

The CR-01 disclosure argument from round 3 stands unmodified: the filename/content coupling is
still ungated, and unforgeability is still structurally out of reach on the same-user, same-IL
token arms.

### Judgement calls that belong to a human, not to this fix pass

**`.planning/PROJECT.md` now runs the full CI matrix when it changes.** That is a single-file
force-include, not the `^\.planning/` tree, so the cost is bounded — but PROJECT.md is touched by
planning commands, so it is a real recurring cost that nobody asked for. The alternative is to
drop `tests/adr_aipc_unix_futures.rs`'s assertion on it. Either is defensible; the gate will keep
whichever is chosen honest.

**`docs/architecture/` is now force-included.** Cheap, but it is a widening of the round-5
decision the brief told me not to revisit. I read that instruction as "do not re-open `^proj/`
versus dropping the blanket `\.md$`", which I did not; force-including two further specific trees
is the discovery half doing its job. Flagging it in case that reading is wrong.

**`nono/src/sandbox/windows.rs:2844`** is the one item in the WR-04 class sweep I did not resolve
to a verdict. It is pre-existing and outside the phase's change set, so I left it alone rather
than change unreviewed sandbox code late in a fix round.

---

## Commits

| # | Hash | Finding |
|---|---|---|
| 1 | `d79f831b` | WR-04 (the fail-open; taken first) |
| 2 | `21b2246a` | WR-01 + WR-02 + WR-03 (one module, three rules) |
| 3 | `a8211eb4` | WR-05 |
| 4 | `04bf35ca` | WR-06 |
| 5 | `f18fafe2` | WR-07 |

---

_Fixed: 2026-08-15_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 4 (round 7)_
