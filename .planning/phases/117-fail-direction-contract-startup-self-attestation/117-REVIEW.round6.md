---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-14T23:40:00Z
depth: standard
round: 6
files_reviewed: 37
files_reviewed_list:
  - .github/workflows/ci.yml
  - bindings/c/include/nono.h
  - bindings/c/src/lib.rs
  - bindings/c/src/types.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/cfg_test_regions.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/network.rs
  - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/query_ext.rs
  - crates/nono-cli/src/telemetry/event.rs
  - crates/nono-cli/src/telemetry/mod.rs
  - crates/nono-cli/src/telemetry/windows.rs
  - crates/nono-cli/tests/common/mod.rs
  - crates/nono-cli/tests/layer_force_unavailable.rs
  - crates/nono-cli/tests/layer_registry_meta_test.rs
  - crates/nono-cli/tests/layer_registry_selfcheck.rs
  - crates/nono-shell-broker/Cargo.toml
  - crates/nono-shell-broker/src/main.rs
  - crates/nono/src/diagnostic/codes.rs
  - crates/nono/src/error.rs
  - crates/nono/src/lib.rs
  - crates/nono/src/machine_policy.rs
  - crates/nono/src/sandbox/windows.rs
  - proj/SPEC-windows-fail-direction-contract.md
findings:
  critical: 0
  warning: 7
  info: 0
  total: 7
status: issues_found
---

# Phase 117: Code Review Report — Round 6

**Reviewed:** 2026-08-14T23:40:00Z
**Depth:** standard (round 6, auditing `62892678..f7cea97f` in the context of `f9ee603^..HEAD`)
**Files Reviewed:** 37
**Status:** issues_found

## Summary

**Verdict on the governing question: the mechanism round 5 targeted is genuinely closed; the
pattern is not fully broken.**

Round 5's central claim is true and I proved it rather than read it. `assert_split_is_correct`
is a real correctness property, every consumer calls it, and the round-4 hole is shut — I
re-derived round 4's perturbations independently, against the real files, and then ran the
decisive one through a real `cargo test` build. There is no BLOCKER this round, no runtime or
security defect, and no regression of any earlier fix I sampled.

But two of round 5's own new artefacts reproduce the phase's signature shape once more, in its
milder "the record/scope is narrower than the claim" form rather than its "a gate a test
assertion can satisfy" form:

- the new CI sync gate (`every_markdown_file_gated_by_a_test_runs_the_code_jobs`) is documented
  as discovering "the Markdown reads in the crate's source and test tree" but scans **five
  hardcoded files**, and there is a **live counterexample outside that list today** —
  `crates/nono-cli/data/profile-authoring-guide.md` is `include_str!`'d into the shipped binary,
  asserted on by `profile_cmd.rs::embedded_guide_contains_no_nono_policy_references`, and a PR
  editing only it still runs zero CI jobs (WR-05);
- WR-06's replacement record still exceeds its mechanism: `output.rs:276-282` and `:311-312` say
  the sweep covers "the WHOLE reachable key space … every one of the 2^13 reachable keys", while
  the sweep enumerates 8192 **synthetic** keys (`Layer00..Layer12`), not the reachable ones
  (subsets of the real `LayerId` `Debug` names) — and injectivity of a hash on one input set does
  not transfer to another (WR-07).

Plus three residual narrownesses in `cfg_test_regions` itself — the single point of failure for
every gate in this phase — one of which is a silent fail-open direction with a shape that already
exists in this crate (WR-01), and one sibling fail-open read that `f7cea97f` left standing three
lines above the one it fixed (WR-04). None is exploitable today; all are the same class this
round exists to close.

### Requirement 2 — round 4's perturbations, re-run independently

I did not trust round 5's table. I compiled `crates/nono-cli/src/cfg_test_regions.rs` verbatim
via `#[path]` into a standalone driver (so the code under test is byte-identical to shipped, not
a replica) and ran the perturbations against the real `agent_daemon/launch.rs`.

| # | Perturbation | Scan result | Emulated gate | Round-5 claim |
|---|---|---|---|---|
| baseline | none | regions=3, skipped=1728, unclosed=[], leaked=[], `DaclAncestorTraverse` production site `[1466]` | PASSES | matches |
| P1 | blank line before `mod attestation_gate_tests {` (line 1861) | regions=3, skipped=1728, unclosed=[], leaked=[] — **identical to baseline** | PASSES | matches |
| P2 | P1 + rename `layer = "DaclAncestorTraverse"` (line 1466) | regions=3, skipped=1728 | **FAILS** — `DaclAncestorTraverse` never named in production half | matches |
| P3 | rename alone | regions=3, skipped=1728 | **FAILS** | matches |

**P2 re-run for real**, not emulated: I applied the perturbation to the working tree and ran
`cargo test -p nono-sandbox-cli --bin nono -- daemon_expected_rows_are_all_named_by_the_daemon_gate`.
It **FAILED** with:

```
DaclAncestorTraverse is expected at (EntryPath::Daemon, None) with an attestable probe
(ConfiguredOnly), but daemon_attest_and_decide's source never names "DaclAncestorTraverse"
outside a comment — the row goes completely unattested on `nono agent launch` (CR-02).
```

The perturbation was reverted with `git checkout --`; `git status --porcelain` afterwards shows
only the pre-existing untracked `117-REVIEW.round4.md`. The tree is clean.

**On P1 staying green — round 5's argument holds, and I can show why rather than accept it.**
The blank-line perturbation produces a scan that is *identical in every field* to the baseline
(same region count, same skipped-line total, same empty `unclosed_regions`, same empty
`leaked_test_attributes`). There is no leak, so there is nothing for the guard to detect; a gate
that failed here would be failing on a difference that does not exist. The guard's independence
is separately demonstrated by round 5's P4 and, in this review, by WR-02 below: I constructed a
shape the classifier *still* mishandles, and `assert_split_is_correct` fired on it. That is the
property that matters for the next evading shape.

### Requirement 1 — `cfg_test_regions` audited directly

Read `scan_production`, `assert_split_is_correct`, `attribute_span`, `is_region_closer`,
`is_cfg_test_attr` and `is_test_attr` line by line, then ran the compiled module against eleven
synthetic shapes and the four real scanned files.

| Question | Method | Result |
|---|---|---|
| Does `assert_split_is_correct` fail for **each** consumer, not one? | Both call sites read (`layer_registry.rs:1469`, `output.rs:2411` for the two-file loop, `output.rs:2476` for the `main.rs` half). No consumer writes its own floor. | **Holds.** Three call sites, one implementation. |
| Is the WR-03 "closer not found" failure reachable, or dead code? | Ran the three `#[should_panic]` proofs live. All three panic with the intended messages (`WR-03`, `WR-01/WR-02`, `non-vacuity`). | **Reachable.** Not dead code. |
| Does the loud failure actually fire on real unclosed regions? | Fixture `mod outer { #[cfg(test)] mod inner {` with no closer → `unclosed_regions=[2]`. | **Holds.** |
| `#[cfg(all(test, target_os = "windows"))]` | 12 of `exec_strategy_windows/launch.rs`'s 12 regions carry it. | **Correct.** |
| `#[cfg(not(test))]`, `#[cfg(all(not(test), unix))]`, multi-line `#[cfg(all(\n not(test),\n unix\n))]` | Synthetic fixtures. | **Correct** — production, kept whole. |
| `#[cfg_attr(test, derive(Debug))]` | Synthetic. | **Correct** — not a test gate. |
| `#[cfg(any(test, feature = "x"))]` | Synthetic. | **WRONG — see WR-01.** Skipped as a test region. |
| Attributes/doc comments/`#[allow(...)]`/blank lines between the cfg attr and `mod` | Synthetic + the real `credential.rs` idiom. | **Correct.** |
| Block comment between the cfg attr and `mod` | Synthetic. | **Partially wrong — see WR-02.** Caught loudly by the leak check today. |
| Nested modules, tab indentation, `} // end` | Synthetic + real (all 3 `agent_daemon/launch.rs` regions are nested inside `mod windows_impl`). | **Correct.** |
| Braces in raw string literals at the module's indent | Synthetic: closes the region early → `#[test]` leaks → `assert_split_is_correct` **fires**. | **Correct direction** (under-claim, loud). |
| Comments containing braces inside a region | Synthetic `// }` at deeper indent. | **Correct.** |
| `mod foo;` vs `mod foo { }` | `main.rs:155` (`test_env`) and `:162` (`cfg_test_regions`) both open no region. | **Correct.** |
| Arithmetic | `raw.len() - raw.trim_start().len()` cannot underflow; `depth` uses `saturating_*`; `skipped_lines()` uses `saturating_sub/add`; `idx += span` with `span >= 1` terminates. | **Sound.** |

### Requirement 5 — earlier rounds re-verified at final HEAD

Sampled directly against the source at `f7cea97f`, not read from the fix report:

| Fix | Evidence | Verdict |
|---|---|---|
| WR-02 (FFI) | `types.rs:218 LayerAttestationFailed = 15`, `:244` conversion arm; `nono.h:147 NONO_DIAGNOSTIC_CODE_LAYER_ATTESTATION_FAILED = 15`; `lib.rs:211` `NonoError::LayerAttestationFailed` arm | **Intact** |
| WR-08 (probe-kind keyed by `LayerId`) | `layer_registry.rs:1529 exactly_one_row_is_confirmed_by_enforcing_component_report` passes | **Intact** |
| WR-11 (reserved device names) | `output.rs:449` `RESERVED` list, `:460` `eq_ignore_ascii_case` rejection, charset restricted to alnum/`-`/`_` so no `CON.txt` form can slip past | **Intact** |
| CR-03 / RF-13 | `machine_policy.rs:197 RequiredLayersPolicy`, `:275 required_layers`, `:683` reader; SPEC row consistent | **Intact** |
| WR-16 (`log_target_is_private`) | `cli_bootstrap.rs:101-174`: all four documented fail-secure early returns present, `Option<Vec<_>>` distinguishes "never ran" from "ran, found nothing", final comparison is component-wise `Path::starts_with` | **Intact** |
| CR-01 digest marker | `output.rs:218-240` writes only `attestation_downgrade_marker_content(dedup_key)` via `create_new(true)`; `:289-294` reader compares content; `.v2` suffix present | **Intact** |
| Standing rules | Ran the shipped classifier over the production half of all 28 `.rs` files in scope and grepped for `.unwrap()` / `.expect(` / `panic!(`. Two hits, both in `telemetry/mod.rs:300-301` inside a `#[cfg(test)] fn poison_for_test` with a documented scoped `#[allow]` — a false positive of my scan, which only skips `#[cfg(test)] mod`. | **0 real violations** |
| Path handling | No `to_string_lossy().starts_with` / `display().to_string().starts_with` on a path in any in-scope production code | **Clean** |

### Gates executed

| Gate | Result |
|---|---|
| `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck --test layer_registry_meta_test` | **19 + 10 passed, 0 failed** |
| `cargo test -p nono-sandbox-cli --bin nono` (24 phase-117 gate tests, incl. all 15 `cfg_test_regions` unit tests, the daemon gate, both SPEC gates, the WR-06 injectivity sweep, the WR-08 literal floor, the D-28 filesystem gate) | **24 passed, 0 failed** |
| WR-08 measured counts printed at HEAD | `output.rs 429, launch.rs 757, attestation.rs 66, attestation_downgrade_event.rs 26, main.rs 204` — exactly the recorded `BASELINE`; floors 214/378/33/13/102 all cleared by 2x |

### On the fixer's judgement call (WR-07 CI scope) — the reasoning holds

The review proposed dropping the blanket `\.md$` from the classifier and force-including `^proj/`.
The fixer declined the first half. **That reasoning is factually correct.** With `\.md$` removed,
the surviving exclusion list is `(^docs/)|(\.mdx$)|(^LICENSE$)|(^\.github/ISSUE_TEMPLATE/)`, so
every `.planning/**/*.md`, `README.md`, `CHANGELOG.md` and `CLAUDE.md` commit sets
`run_code_jobs=true` and fans out the whole matrix (`test` on ubuntu + macos, five Windows lanes,
packaging). `.planning/` is tracked and pushed on this repo, so that is a large recurring cost the
finding did not ask for. Force-including `^proj/` is the proportionate shape, and pairing it with a
discovery gate at the other end is the right architecture.

The execution of that architecture is where WR-05 lands: the discovery half is not actually
discovery. That is a scope defect in the new gate, not a reason to revisit the decision.

### Not re-litigated

Per the brief I did not re-open CR-02's daemon-registry wiring or WR-12's fleet-control plumbing.
I checked both recorded rationales against the code: `layer_registry.rs:1410-1423`'s SCOPE
paragraph still correctly says the gate proves "no daemon-expected row is unmentioned", not "the
registry drives the daemon"; the `WR-10 OPEN` (`layer_registry.rs:925`) and `WR-14 OPEN`
(`error.rs:506`) markers both exist and both have `OPEN` ledger rows (verified by running
`every_open_marker_in_code_has_a_ledger_row`); and `error.rs`'s case-(3) claim is now correctly
scoped and gate-protected. **Those records still match the code.**

I also checked, and then discarded, a candidate finding that Phase 117's Windows-only gates never
execute in CI. They do: `ci.yml:325 windows-layer-fault-injection` runs
`cargo test -p nono-sandbox-cli --features layer-fault-injection -- --test-threads=1` on
`windows-latest`, gated on the same `run_code_jobs` flag WR-07 fixed. Recording the negative
because it is load-bearing for WR-07's value.

---

## Warnings

### WR-01: `is_cfg_test_attr` classifies `#[cfg(any(test, ...))]` as a test gate, silently dropping a production module — the over-claim direction no check can see

**File:** `crates/nono-cli/src/cfg_test_regions.rs:103-109` (doc at `:88-94`, unit test at `:401-414`)

**Issue:** The predicate is

```rust
compact.starts_with("#[cfg(")
    && !compact.contains("not(test")
    && (compact.contains("test)") || compact.contains("test,"))
```

`#[cfg(any(test, feature = "x"))]` compacts to `#[cfg(any(test,feature="x"))]`, contains `test,`,
does not contain `not(test` — so it classifies as a **test gate**. A module gated that way is
compiled into production builds whenever the other disjunct holds, and the classifier deletes its
entire body from the production half. Measured on a fixture:

```
SYN cfg-any-test: regions=[(1, 4)] unclosed=[] leaked=[] kept=[(5, "fn prod() {}")]
```

That is the **over-claim** direction — the one this module's own doc (`:64-75`) calls "the worse
one" — and it is invisible to all three checks in `assert_split_is_correct`: `unclosed_regions` is
empty (the region closed fine), `leaked_test_attributes` is empty (a production module has no
`#[test]`), and `skipped_regions` is *more* non-empty, not less. A gate whose production needle
lived in that module would go green by absence with no signal at all.

This is not a hypothetical attribute shape for this crate — it is written here today:

```rust
// crates/nono-cli/src/session_commands.rs:678
#[cfg(any(test, target_os = "macos", target_os = "windows"))]
fn format_bytes_human(bytes: u64) -> String {
```

It gates a `fn`, in a file no gate scans, so there is no live blast radius. But `#[cfg(not(test))]`
was excluded for exactly that reason — the doc at `:88-94` says so in as many words ("No such
attribute exists in this crate today, so that was latent rather than live; it is excluded here so
it stays that way") — and the sibling shape that *does* exist was not. The unit test at `:401-414`
covers `not(test)` and `all(not(test), ...)` and does not cover `any(test, ...)`.

**Fix:** Require the `test` token to be reachable through `all(`/bare `cfg(` only, and add the
shape to the classification-rule test:

```rust
#[must_use]
pub fn is_cfg_test_attr(trimmed: &str) -> bool {
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    if !compact.starts_with("#[cfg(") {
        return false;
    }
    // `any(test, ..)` and `not(test)` both gate PRODUCTION builds: `any` is
    // satisfied by its other disjuncts, `not` by the absence of `test`.
    // Skipping either drops real source from every gate built on this helper,
    // and that direction is invisible to `assert_split_is_correct`.
    if compact.contains("not(test") || compact.contains("any(test") {
        return false;
    }
    compact.contains("test)") || compact.contains("test,")
}
```

and in `cfg_test_attr_classification_rule`'s `no` list:

```rust
"#[cfg(any(test, feature = \"x\"))]",
"#[cfg(any(test, target_os = \"macos\", target_os = \"windows\"))]",  // session_commands.rs:678
```

---

### WR-02: the pending-attribute window is still narrower than the class it documents — a block comment between the cfg attribute and `mod` un-skips the module

**File:** `crates/nono-cli/src/cfg_test_regions.rs:320-329` (doc at `:44-56`)

**Issue:** Round 4's WR-01 widened the window from three shapes to what the doc now calls the class
("anything that is not the gated item"). The implementation is still an enumeration:

```rust
if t.is_empty() || t.starts_with("//") || t.starts_with("/*") || t.starts_with('*') {
```

A block comment whose interior lines are not `*`-aligned falls outside it. Measured:

```
src:  #[cfg(test)]
      /* hello
         world */
      mod t {
          #[test]
          fn x() {}
      }
      fn prod() {}

SYN block-comment-between: regions=[] unclosed=[] leaked=[4]
```

`world */` is neither empty nor `//`/`/*`/`*`-prefixed, so `pending_test_attr` clears, `mod t {`
opens no region, and the whole module body is classified as production — WR-01's original failure
mode, reopened by a third intervening token, in the module created to make it impossible. Note
also `kept` includes the comment body line itself, so a `/* */` comment naming a gate's needle
would satisfy that gate.

**The guard does catch this today** (`leaked=[4]`), which is why this is a WARNING and not a
BLOCKER, and it is genuine evidence that round 5's structural fix works. But the catch is
conditional on the leaked module carrying a *recognised* test attribute — see WR-03 — and on the
module having any test attribute at all. All 18 regions across the four scanned files have at
least one today (verified), so the safety margin is real but unowned.

**Fix:** Track block-comment depth instead of pattern-matching its lines, or state the narrower
rule honestly. The former is four lines:

```rust
// inside the pending window, before the shape checks
if in_block_comment {
    if t.contains("*/") {
        in_block_comment = false;
    }
    idx += 1;
    continue;
}
if t.starts_with("/*") && !t.contains("*/") {
    in_block_comment = true;
    idx += 1;
    continue;
}
```

Add the non-aligned fixture above to the unit tests alongside the three round-4 shapes.

---

### WR-03: `is_test_attr` calls itself a class and matches two exact forms — a parenthesised test attribute is not a test attribute to it

**File:** `crates/nono-cli/src/cfg_test_regions.rs:111-122`

**Issue:**

```rust
trimmed == "#[test]" || (trimmed.starts_with("#[") && trimmed.ends_with("::test]"))
```

with a doc that says "Deliberately a class, not the one literal: a leaked module brings whatever
test attribute its functions use, and pinning only `#[test]` would make the leak check itself the
narrow needle this review round keeps finding." The parenthesised forms — `#[tokio::test(flavor =
"multi_thread")]`, `#[rstest::test(...)]`, `#[test_case(..)]`, `#[rstest]` — all end in `)]` or are
a bare ident, and none match. Measured:

```
SYN test-attr-with-args: leaked=[4]   # only the plain #[test]; #[tokio::test(flavor = "…")] missed
```

`leaked_test_attributes` is the ONLY check in `assert_split_is_correct` that fires in the
under-claim direction, so this is the width that decides whether WR-02 (and the next unforeseen
window shape) is caught or silent. The crate has 92 plain `#[tokio::test]` and zero parenthesised
forms today, so the gap is latent — the same standard under which `not(test)` was closed
pre-emptively.

**Fix:** match the class by head token rather than by full literal:

```rust
#[must_use]
pub fn is_test_attr(trimmed: &str) -> bool {
    let Some(body) = trimmed
        .strip_prefix("#[")
        .and_then(|b| b.strip_suffix(']'))
    else {
        return false;
    };
    // Drop any argument list: `tokio::test(flavor = "multi_thread")` -> `tokio::test`.
    let head = body.split('(').next().unwrap_or(body).trim();
    head == "test" || head.ends_with("::test")
}
```

and extend `test_attribute_classification_rule`'s `yes` list with
`"#[tokio::test(flavor = \"multi_thread\")]"` and `"#[rstest::test(case(1))]"`.

---

### WR-04: `f7cea97f` closed the fail-open read and left its sibling three lines above it — the enumeration that feeds the D-28 scan still skips unreadable directories silently

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:4603-4615`

**Issue:** The commit correctly replaced `let Ok(bytes) = std::fs::read(file) else { continue; };`
with a fail-closed `match`. The function that *produces* `files` keeps the identical shape, in the
same test, thirty lines up:

```rust
fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;                       // <- unreadable directory silently skipped
    };
    for entry in entries.flatten() {  // <- per-entry Err silently dropped
        ...
    }
}
```

The commit message's own reasoning applies verbatim: "an unscannable file is precisely the one that
could be hiding the plaintext layer set." An unscannable *directory* hides a whole subtree, and
`entries.flatten()` hides individual entries. The new `assert_eq!(files.len(), 1)` catches the case
where the *top-level* session dir is unreadable (0 files → fail), but a nested unreadable
subdirectory leaves `files.len() == 1` and the gate reports a clean D-28 result over a subtree it
never opened.

This is the phase's most security-relevant gate, and priority-4's question — "does any sibling read
in that gate have the same shape?" — has the answer *yes*.

**Fix:** propagate the failure to the caller so it lands in `violations`:

```rust
fn collect_files(
    dir: &std::path::Path,
    out: &mut Vec<std::path::PathBuf>,
    unreadable: &mut Vec<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            unreadable.push(format!("{}: UNREADABLE DIR ({e})", dir.display()));
            return;
        }
    };
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_dir() {
                    collect_files(&path, out, unreadable);
                } else {
                    out.push(path);
                }
            }
            Err(e) => unreadable.push(format!("{}: UNREADABLE ENTRY ({e})", dir.display())),
        }
    }
}
```

and seed `violations` from `unreadable` before the per-file loop, with the same "this guard cannot
prove the file does not name a layer" wording.

---

### WR-05: the new CI sync gate is documented as discovery over the crate's tree and scans five hardcoded files — and a live `.md` outside that list still runs zero CI jobs

**File:** `crates/nono-cli/tests/layer_registry_selfcheck.rs:1597-1623` (claim at `:1592-1596`;
`117-REVIEW-FIX.md` states it "finds Markdown reads in the crate's source and test tree in **both**
house idioms")

**Issue:** The "discovery" is a literal five-element array:

```rust
for rel in [
    "src/main.rs",
    "src/output.rs",
    "tests/layer_registry_selfcheck.rs",
    "tests/layer_registry_meta_test.rs",
    "tests/layer_force_unavailable.rs",
] {
```

out of ~150 source files in the crate. The rule the gate states — "every Markdown file the test
tree READS must be a file that makes CI run the code jobs" — has a live counterexample outside it:

- `crates/nono-cli/src/config/embedded.rs:38-39` does
  `include_str!(concat!(env!("OUT_DIR"), "/profile-authoring-guide.md"))`, whose source is
  `crates/nono-cli/data/profile-authoring-guide.md` (`build.rs:114`);
- `crates/nono-cli/src/profile_cmd.rs:3579-3585`
  (`embedded_guide_contains_no_nono_policy_references`) asserts on its content;
- the file matches `\.md$`, is not under `^proj/`, and is not under `^docs/` — so a pull request
  that edits **only** it runs `run_code_jobs=false`, i.e. **zero jobs**, while changing the shipped
  binary's embedded guide.

That is the same failure WR-07 closed for the SPEC, still open one file over, and the gate built to
prevent exactly this is green. The gate would also have flagged it had `embedded.rs` been in scope
(the line contains `include_str!` and a `"…/profile-authoring-guide.md"` fragment) — but
`candidate_trees = ["proj", "docs", "."]` would then report it `unresolved` rather than
`unguarded`, so the tree list needs widening in the same change.

Second, smaller, in the same gate (`:1691-1696`): the force-include clause is checked as a raw
substring of the whole workflow text — `ci.contains("=~ ^proj/ ]]")` — including YAML comments. A
comment in `ci.yml` mentioning the clause would satisfy it while the clause itself was deleted.
This is the mirror image of the care taken on the Rust side, where comment lines are excluded from
the read scan for precisely this reason.

**Fix:**

```rust
// Discover, do not enumerate. `.rs` under src/ and tests/ — the crate's own
// tree, which is what this gate claims to cover.
fn crate_sources() -> Vec<(String, String)> { /* walk manifest_dir()/src and /tests */ }

// A `.md` under the crate's own data/ tree is embedded into the binary, so
// resolve there too.
let candidate_trees = ["proj", "docs", "crates/nono-cli/data", "."];
```

and strip comment lines from `ci` before the substring check:

```rust
let ci_effective: String = ci
    .lines()
    .filter(|l| !l.trim_start().starts_with('#'))
    .collect::<Vec<_>>()
    .join("\n");
```

Then either force-include `^crates/nono-cli/data/` in `ci.yml`, or move the guide under `proj/`.

---

### WR-06: the new marker parser-correctness check hunts a needle far wider than the class it protects, and its dedup makes a repeated id look like a blind file

**File:** `crates/nono-cli/tests/layer_registry_selfcheck.rs:1136-1189`

**Issue:** Round 5 correctly replaced `!markers.is_empty()` with a parser-correctness property. The
property's trigger is the bare substring ` OPEN`:

```rust
if src.contains(" OPEN") {
    files_with_open_text.push(label);
}
```

while the class being parsed is `XX-NN OPEN`. Two consequences, both producing a *false* build
failure with a misleading message ("that finding's ledger row is unprotected"):

1. Ordinary Windows source trips it. `dwCreationDisposition: OPEN_EXISTING`, `FILE_OPEN`, or the
   English word "open" at a sentence start all contain ` OPEN`; the walk-back then yields an empty
   id, no marker parses, and the file is reported blind. Three of the five scanned files
   (`launch.rs`, `attestation.rs`, `layer_registry.rs`) are Win32-facing and none currently has
   such a constant, but adding a `CreateFileW` call to `launch.rs` would fail this gate for a
   reason unrelated to any deferral record.
2. The marker list dedups by id **globally** (`:1161`), and `files_with_parsed_marker` is keyed on
   `markers.len() > before` (`:1166`). If a second file repeats an id already seen (an ordinary
   thing when one deferral is annotated at both its sites), that file yields no *new* marker, is
   not recorded as having parsed one, and is reported blind.

The direction is fail-closed, so this is a diagnosability and robustness defect rather than a
coverage hole — but the round's whole subject is needles being the right width, and this one is
wrong in the opposite direction from the ones it was fixing.

**Fix:** trigger the property on the marker *shape*, and record parsed markers per file:

```rust
fn line_has_marker_shape(line: &str) -> bool {
    line.match_indices(" OPEN").any(|(pos, _)| {
        let id: String = line[..pos]
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        id.len() >= 4
            && id.chars().take(2).all(|c| c.is_ascii_uppercase())
            && id.chars().nth(2) == Some('-')
            && id[3..].chars().all(|c| c.is_ascii_digit())
    })
}
// files_with_open_text: src.lines().any(line_has_marker_shape)
// files_with_parsed_marker: push when THIS file yielded any parsed id,
//   deduped or not — track a per-file counter, not markers.len().
```

---

### WR-07: WR-06's replacement record still exceeds its mechanism — the sweep enumerates a synthetic vocabulary, not the reachable key space

**File:** `crates/nono-cli/src/output.rs:274-282` and `:303-312`, with the sweep at `:1666-1706`

**Issue:** Round 4's WR-06 was "the record claims a guarantee the mechanism does not provide". The
replacement record says:

> `marker_content_never_names_a_layer_and_stays_injective` enumerates the **WHOLE reachable key
> space** — all 2^13 subsets of a 13-name vocabulary … (`:276-278`)

> every one of the **2^13 reachable keys** is checked to map to a distinct content. (`:311-312`)

What the sweep enumerates (`:1677-1690`) is 2^13 subsets of `["Layer00", …, "Layer12"]` — a
synthetic vocabulary. The reachable key space is 2^13 subsets of the real `LayerId` `Debug` names
(`AppContainerProfile`, `JobObjectContainment`, `WfpEgressFilters`, …), sorted and comma-joined.
These are two disjoint input sets, and injectivity of `DefaultHasher` over one says nothing about
the other — the test's own comment that "a same-length same-shape vocabulary exercises the digest's
input space identically" (`:1673-1675`) is not a property any hash has (the vocabulary is not even
same-length: `Layer00` is 7 bytes, `DaclAncestorTraverse` is 20).

The practical risk is nil — a 128-bit-wide output over 8192 inputs — and the strength framing
correction in the same commit is right and well done. The defect is that the *record* is once more
stronger than the *check*, which is the exact finding being closed, in the fix that closed it.

**Fix:** either state the real claim, or make it true. Making it true costs nothing structural,
because the launch-side sibling already imports the registry — move the sweep there, or feed the
real vocabulary in through a parameter so `output.rs` still never names the type:

```rust
// output.rs — keep the D-28 structural rule, but stop claiming the real space
/// … enumerates all 2^13 subsets of a **synthetic 13-name vocabulary of the
/// same shape production builds**, and asserts every one maps to distinct
/// content. `output.rs` deliberately does not learn the layer identity type
/// (D-28), so this is a property of the digest over a stand-in vocabulary, not
/// over the real `LayerId` names; the launch-side
/// `downgrade_marker_files_never_contain_a_layer_name` is what binds the real
/// vocabulary, and an injectivity sweep over `LayerId::ALL` belongs there.
```

and add the 8192-key sweep over `layer_registry::ALL` to
`exec_strategy_windows/launch.rs`'s `attestation_gate_tests`, where the import is already legal.

---

_Reviewed: 2026-08-14T23:40:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard — round 6 (adversarial audit of `62892678..f7cea97f`)_
