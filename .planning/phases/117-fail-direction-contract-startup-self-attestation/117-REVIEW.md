---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-14T18:20:00Z
depth: standard
round: 4
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
  warning: 8
  info: 0
  total: 8
status: issues_found
---

# Phase 117: Code Review Report — Round 4

**Reviewed:** 2026-08-14T18:20:00Z
**Depth:** standard (round 4 re-review of `e6438f96..860d4772`, in the context of `f9ee603^..HEAD`)
**Files Reviewed:** 37
**Status:** issues_found

## Summary

**Verdict on the governing question: round 3 half-broke the pattern.** The runtime fix (CR-01)
is genuinely correct in both directions — I re-derived it independently and could not make it
fail. But the *guard* half of round 3 reproduced the pattern twice more, and I can demonstrate
both mechanically rather than argue them:

1. The new shared classifier `cfg_test_regions::scan_production` implements a rule strictly
   narrower than the class it documents, and **the one consumer that did not receive a
   correctness check can be driven green by a test assertion with a single blank line** — the
   exact defect that consumer's own comment says it exists to close.
2. Two of the round's new SPEC/record gates match a needle narrower than the class their own
   doc-comments state, so they can relabel but never deny.

No BLOCKER. I found no runtime correctness or security defect: no `.unwrap()`/`.expect()` in any
production half of any changed file, no string-`starts_with` path handling, checked arithmetic in
the new helper, and D-28's fail-secure `log_target_is_private()` gating intact.

### What I verified, and how

Everything below was re-derived by executing the logic against the real files, not by reading the
fix report's claims.

| Claim under test | Method | Result |
|---|---|---|
| CR-01: plaintext layer set gone from disk | Read the writer (`output.rs:218-240`); only `attestation_downgrade_marker_content(dedup_key)` is written. Filename is a `DefaultHasher` digest + `.v2`. | **Holds.** No `LayerId` text reaches the marker on any path. |
| CR-01: announce/suppress fails toward ANNOUNCE | Traced every abnormal state through `marker_says_already_announced` (`output.rs:280-285`). Absent / unreadable / permission-denied / directory-where-file-expected / non-UTF-8 → `read_to_string` `Err` → `false`. Zero-byte / truncated / mismatched / v1-plaintext → content compare fails → `false`. `session_id` rejected or `sessions_dir()` unavailable → `None` → no marker consulted. | **Holds in all 8 states.** |
| CR-01: domain separation real | `digest(domain, key)` hashes `domain` then `key`; `impl Hash for str` is prefix-free (`write_str` appends `0xff`), and `DefaultHasher::new()` is fixed-key, so `hi`/`lo` are genuinely distinct. | **Holds.** (Strength claim overstated — see WR-06.) |
| WR-01 perturbation ("41 `#[test]` leak, first at 3323") | Reimplemented `scan_production` exactly and ran both the fixed and the pre-fix variant over the real `launch.rs`. | **Reproduced exactly**: fixed → 12 regions, 0 leaked `#[test]`; naive → 6 regions, 41 leaked, first at line 3323. |
| Classifier correctness on all four scanned files | Ran the replica over `output.rs`, `main.rs`, `exec_strategy_windows/launch.rs`, `agent_daemon/launch.rs`. | **Correct today.** 2/1/12/3 regions, 0 leaked `#[test]` in every file; both bare `#[cfg(test)] mod foo;` declarations (`main.rs:155`, `:162`) correctly open no region. |
| WR-26 class gate non-vacuity, per file | Replicated `production()` + continuation join + 12-entry arm walk-back. | **Non-vacuous and correct**: `output.rs:165` hit, arm `EventLog => {`; `launch.rs:1488` hit, arm `EventLog => {`; `main.rs:310` mention preceded by `(No `. hits = 1/1, floor 1/1. |
| WR-06 literal extractor + floors | Replicated `char_literal_len` + `string_literals` + the collapsed-run predicate over all five surface files. | **Passes, non-vacuous**: 1457 literals, 0 offenders. Per-file 421/757/66/26/187 (floor 25 — see WR-08). Hand-traced the report's decisive perturbation; the naive extractor does swallow the probe. |
| WR-05 citation conversion | Replicated both new gates. | **0 raw `file.rs:<line>` citations remain** on the registry surface; 38 symbol citations extracted (floor 20), 28 distinct — I resolved every one of the 28 by grepping for the definition. All 28 resolve. |
| WR-04 ledger gate | Parsed the SPEC the way the gate does. | **Correct.** Header at `:248`, delimiter `:249`, 52 contiguous rows to `:301`, no blank line inside. Exactly 2 `XX-NN OPEN` markers exist tree-wide (`error.rs:506`, `layer_registry.rs:925`); both have OPEN rows. |
| WR-03 medium-label helper | Replicated `medium_label_command_mentions` over the SPEC and the rendered remediation. | **Passes**: 1 SPEC mention (`:294`), qualified within 200 chars; rendered remediation qualified immediately. |
| Round-1/2 fixes not regressed | Re-checked WR-02 (FFI `LayerAttestationFailed = 15` in `types.rs`/`nono.h`/`lib.rs:211`), WR-08 (`classify_row` keyed by `LayerId`, `_ => false`), WR-11 (reserved device names), CR-03 (RF-13 row now matches `machine_policy`), WR-16 (`log_target_is_private` fail-secure, all 6 `capability.rs` citations still resolve). | **All intact.** |
| Standing rules | Ran the classifier over 19 changed source files and grepped the *production* half only. | **0** `.unwrap()` / `.expect()` / `unwrap_or_else(\|\| panic` in production code. |

Per the brief I did not re-litigate CR-02's daemon-wiring operator decision or WR-12's fleet-control
plumbing. I checked both recorded rationales against the code: RF-13's row now correctly says the
reader carries the value into `RequiredLayersPolicy.required` (matching CR-03), and the
`WR-10 OPEN` / `WR-14 OPEN` markers both exist with matching SPEC rows. Those records are correct.

## Warnings

### WR-01: `cfg_test_regions`'s rule is narrower than the class it documents — a blank line or a plain `//` comment un-skips an entire test module

**File:** `crates/nono-cli/src/cfg_test_regions.rs:101-130` (and the rule stated at `:27-40`)

**Issue:** The pending-attribute window admits exactly three intervening shapes:

```rust
if t.starts_with("#[") || t.starts_with("///") || t.starts_with("//!") {
```

Any other line clears `pending_test_attr`, and the following `mod foo {` then opens no region —
so the whole module body is classified as production. Three shapes that Rust and rustfmt both
permit fall outside that window:

1. **A blank line** between `#[cfg(test)]` and `mod`.
2. **A plain `//` comment** (not `///`, not `//!`) between them.
3. **A multi-line `cfg` attribute** — `#[cfg(all(` on its own line fails `is_cfg_test_attr`
   entirely, so `pending_test_attr` is never even set.

This is not hypothetical styling. Shape (2) already exists in this workspace, written in this
codebase's own idiom:

```rust
// crates/nono-proxy/src/credential.rs:596
#[cfg(test)]
#[allow(clippy::unwrap_used)]
// The env-guard below mutates process env via set_var/remove_var with a symmetric
// Drop restore (the CLAUDE.md-endorsed save/restore test pattern); the
// disallowed_methods ban targets non-test misuse, so scope an allow to the tests.
#[allow(clippy::disallowed_methods)]
mod tests {
```

Measured impact when the shape lands on a scanned file — one blank line inserted before
`agent_daemon/launch.rs`'s `mod attestation_gate_tests {`:

```
BASELINE           regions=3  skipped=1728  production sites: DaclAncestorTraverse [1466]
BLANKLINE-PERTURB  regions=2  skipped=1341  production sites: DaclAncestorTraverse [1466, 2225]
```

387 lines of test code cross into the production half. This is WR-01's own failure mode reopened
by a different intervening token, in the module that was created specifically to make it
impossible. The module's unit tests cover the `#[allow(...)]` case, the `not(test)` case, the
bare-declaration case and the nesting case — none of the three above.

The `#[test]`-leak assertion in `output.rs` catches this for its three files. `layer_registry.rs`'s
gate does not (WR-02, below), and any future third consumer inherits the same gap.

**Fix:** Widen the window to the class ("anything that is not the gated item"), and make the
attribute test see continuation lines:

```rust
// cfg_test_regions.rs — inside the pending window
if t.is_empty() || t.starts_with("#[") || t.starts_with("//") || t.starts_with("*/") {
    idx += 1;
    continue;
}
```

and treat an unterminated `#[cfg(` line as an attribute continuation (accumulate until the
bracket balances) before classifying. Add all three shapes to the module's unit tests, each with
the `nono-proxy/src/credential.rs:596` shape as the fixture for case (2) so the test cites a real
occurrence rather than a synthetic one.

---

### WR-02: the daemon gate's non-vacuity check is monotone in the wrong direction, so a partial split leaves it green — provably

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:1449-1458`

**Issue:** WR-01's fix eliminated the duplicate classifier but propagated the *correctness* check
to only one of the two consumers. `output.rs` got the real one — "no `#[test]` attribute may
appear in any file's production half" (`output.rs:2271-2283`). `layer_registry.rs` kept:

```rust
let skipped_test_lines = scan.skipped_lines();
assert!(skipped_test_lines > 0, "CR-02 non-vacuity: no `#[cfg(test)]` module body was excluded …");
```

`skipped_lines() > 0` is satisfied *more* the more the classifier skips and cannot distinguish
"all three test regions skipped" from "two of three skipped". It is the exact predicate whose
weakness the round-3 report itself names ("`!skipped_regions.is_empty()` would have stayed green
through the actual defect") — fixed in one mirror, left in the other, in the same commit.

Demonstrated end to end. With the WR-01 shape present (one blank line before
`mod attestation_gate_tests {`), I then renamed the real production site the gate exists to pin:

```
BLANKLINE+RENAME   regions=2  skipped=1341   (> 0, so the non-vacuity assert still PASSES)
    DaclAncestorTraverse  production sites: [2225]      <- line 1466 renamed; 2225 is a TEST assertion
```

The gate stays **GREEN** while the production arm it guards has been renamed away — satisfied by
`attestation_gate_tests`'s own assertion. That is verbatim the failure this gate's own comment
records as the reason the test-module exclusion exists:

> renaming the production `layer = "DaclAncestorTraverse"` left it GREEN, because
> `agent_daemon/launch.rs`'s own `attestation_gate_tests` asserts on that same string. A gate that
> a test assertion can satisfy is the defect one step over.

**Fix:** Give this gate the same correctness check `output.rs` has, and lift it into the shared
module so a third consumer cannot skip it:

```rust
// cfg_test_regions.rs
impl ProductionScan<'_> {
    /// Zero-based lines of any `#[test]` attribute left in the production
    /// half. Non-empty means the split is WRONG, not merely small.
    #[must_use]
    pub fn leaked_test_attributes(&self) -> Vec<usize> {
        self.lines.iter().filter(|(_, l)| l.trim() == "#[test]").map(|(i, _)| *i).collect()
    }
}

// layer_registry.rs — replace the `skipped_test_lines > 0` assert
let leaked = scan.leaked_test_attributes();
assert!(
    leaked.is_empty(),
    "CR-02: {} `#[test]` attribute(s) survived into the production half of \
     agent_daemon/launch.rs (first at line {:?}) — a test module leaked, and its assertions \
     can satisfy this gate",
    leaked.len(),
    leaked.first().map(|i| i + 1)
);
```

Keep the `skipped_test_lines > 0` floor as well; it catches the *zero*-region case the leak check
does not.

---

### WR-03: `scan_production` silently converts "closing brace not found" into a region that runs to EOF, and neither consumer's non-vacuity check can see it

**File:** `crates/nono-cli/src/cfg_test_regions.rs:113-126`

**Issue:** The region terminator is an exact whole-line string match:

```rust
let indent = lines[idx].len() - lines[idx].trim_start().len();
let closer = format!("{}}}", " ".repeat(indent));
idx += 1;
while idx < lines.len() && lines[idx] != closer { idx += 1; }
skipped_regions.push((start, idx.min(lines.len().saturating_sub(1))));
```

When the closer is never found, `idx == lines.len()` and the region is recorded as running to the
last line — **with no signal to the caller that it was never closed**. Everything after the module
start is dropped from the production half. That is the over-claim direction, i.e. production text
silently unscanned, which is the worse of the two failure directions.

Three realistic triggers, none currently present but none prevented:

- `} // end of tests` on the closing line (rustfmt preserves trailing comments after a brace).
- Tab indentation: `indent` counts *bytes* stripped, then `" ".repeat(indent)` rebuilds them as
  spaces, so a tab-indented nested module can never match its own closer.
- A line that is exactly `}` at the module's indent inside a raw string or a `/* */` block inside
  the test module (ends the region early — the opposite direction, caught only in `output.rs`).

Neither consumer can notice. `skipped_lines() > 0` and `!skipped_regions.is_empty()` are both
*more* satisfied by over-claiming; the `#[test]`-leak check only fires on under-claim. Measured:
changing `launch.rs:2769` from `}` to `} // end of detached_token_gate_tests` merges two regions
(12 → 11), and every assertion in both gates stays green.

Today's blast radius is zero only by layout accident — I checked, and the four scanned files have
essentially no production code after their first test region (`output.rs` 1 line, `main.rs` 0,
`exec_strategy_windows/launch.rs` 11 separator lines, `agent_daemon/launch.rs` 4). That is not a
property anyone is maintaining.

**Fix:** Make "unclosed" representable and fail loudly:

```rust
// cfg_test_regions.rs
pub struct ProductionScan<'a> {
    pub lines: Vec<(usize, &'a str)>,
    pub skipped_regions: Vec<(usize, usize)>,
    /// Regions whose closing brace was never found — the scan ran to EOF and
    /// silently dropped everything after `start`. Callers MUST assert this is
    /// empty; an over-claimed region satisfies every non-vacuity floor.
    pub unclosed_regions: Vec<usize>,
}
```

populate it when `idx >= lines.len()`, and assert `unclosed_regions.is_empty()` in both consumers.
Compute `indent` from `chars().take_while(|c| c.is_whitespace())` and rebuild the closer from the
original leading whitespace slice rather than from `" ".repeat`, so tabs match.

---

### WR-04: the new SPEC event-log gate matches two verbs while its own doc states the class — it can relabel but never deny

**File:** `crates/nono-cli/src/main.rs:465-487`
(`the_spec_ledger_does_not_point_abort_path_layers_at_the_event_log`)

**Issue:** The doc comment states the rule as a class:

> The predicate is "no ledger row asserts that non-label layers are **POINTED AT** the event log".

The predicate implemented is two present-tense verb phrases:

```rust
n.contains("points at the Windows Application event log")
    || n.contains("point at the Windows Application event log")
```

`pointed at`, `pointing at`, `directs the operator to`, `refers … to`, `names … as the place to
look`, `see the …` — every one of them evades it. This is the *identical* complaint round 2 raised
as WR-06 against `!contains("see the Windows Application event log")`, and round 3 fixed that one
correctly (`main.rs:583-600`: count **all** mentions of the class needle, require the negation) —
then wrote the rejected narrow-needle shape into the new SPEC mirror **in the same commit**
(`84e48ffe`). The perturbation the fix report cites ("restore the SPEC's prescriptive WR-17 text →
both SPEC gates FAIL") only exercises the one historical phrasing the needle was written from.

The sibling gate 30 lines above (`the_spec_never_prescribes_the_medium_label_command`) does it
right — class needle plus an attached-qualifier window. Two gates, same commit, same document,
opposite patterns.

**Fix:** Use the sibling's own shape:

```rust
const EVENT_LOG: &str = "Windows Application event log";
const WINDOW: usize = 160;
for (idx, line) in SPEC.lines().enumerate().filter(|(_, l)| l.trim_start().starts_with('|')) {
    let n = line.split_whitespace().collect::<Vec<_>>().join(" ");
    for (pos, _) in n.match_indices(EVENT_LOG) {
        let before = &n[pos.saturating_sub(WINDOW)..pos];
        let after_end = (pos + EVENT_LOG.len() + WINDOW).min(n.len());
        let ctx = format!("{before}{}", &n[pos + EVENT_LOG.len()..after_end]);
        assert!(
            ctx.contains("named ONLY as an explicit negation")
                || ctx.contains("downgrade")     // the path that really does write there
                || ctx.contains("No ") || ctx.contains("no "),
            "WR-02: SPEC:{} names the event log in a ledger row without qualifying it as \
             the downgrade-path destination or as an explicit negation:\n{line}", idx + 1);
    }
}
```

and add a **detector self-test** — the WR-05 raw-citation gate has one and it is why that gate can
be trusted; this one has none.

---

### WR-05: the WR-14-record gate searches its needles over all of `error.rs`, not the record — one of the four already matches unrelated lines

**File:** `crates/nono-cli/tests/layer_registry_selfcheck.rs:1157-1183`
(`the_wr14_open_record_matches_the_actual_swallow_sites`)

**Issue:** The gate's stated job is that "the `WR-14 OPEN` record must state the REAL reason". Its
implementation is four whole-file `contains` checks:

```rust
for needle in ["classify_probe_outcome", "agent_daemon/launch.rs", "Ok(true)", "FFI"] {
    assert!(error_rs.contains(needle), "…the `WR-14 OPEN` record … no longer names {needle:?}…");
}
```

`"FFI"` already appears on four lines of `crates/nono/src/error.rs` that have nothing to do with
the WR-14 record — `:10`, `:274`, `:275`, `:295` — all pre-existing doc comments about the C FFI
surface. So **deleting the FFI-reachability sentence from the WR-14 record, which is the single
correction WR-07 asked for, would not fail this gate.** The other three needles are block-unique
today by luck, not by construction: the gate goes vacuous the moment any of them is written
elsewhere in the file, which for `agent_daemon/launch.rs` and `classify_probe_outcome` is an
ordinary thing to do in a doc comment.

Same shape as WR-01/WR-04: the predicate's *scope* is wider than the thing the message claims it
protects.

**Fix:** Extract the block and assert within it:

```rust
let block = {
    let start = error_rs.find("⚠ WR-14 OPEN").expect(
        "the WR-14 OPEN marker is gone from error.rs — either the finding was closed \
         (delete this gate deliberately) or the marker convention changed");
    let end = error_rs[start..].find("=> Some(NonoRemediation::ClearStaleLayerResidue")
        .map_or(error_rs.len(), |o| start + o);
    &error_rs[start..end]
};
for needle in ["classify_probe_outcome", "agent_daemon/launch.rs", "Ok(true)", "FFI"] {
    assert!(block.contains(needle), "…");
}
assert!(!block.contains("requires a null job handle"), "…");
```

---

### WR-06: the CR-01 injectivity claim is asserted over 6 keys and recorded as covering the whole key space

**File:** `crates/nono-cli/src/output.rs:1560-1631`
(`marker_content_never_names_a_layer_and_stays_injective`), with `:266-273`

**Issue:** Three records state a property the gate does not establish:

- `117-REVIEW-FIX.md`: "the digest is injective for every input this code can produce, **pinned by
  an explicit injectivity assertion**".
- `output.rs:271-272`: "the digest is injective for every input this code can produce, up to a
  128-bit collision".
- The test's own doc: "Checked **exhaustively** over a **spanning sample** of key shapes" — which
  is self-contradictory.

The assertion covers 6 hand-written keys. The production key space is the sorted comma-join of
subsets of a 13-element enum, i.e. up to 2^13 = 8192 distinct inputs, all of which are cheaply
enumerable. This is the same "the record claims a guarantee the mechanism does not provide" class
that WR-01 (round 2) and WR-05 (round 3) were both raised against, in round 3's own new gate.

Practical risk is low (a simultaneous 64-bit filename *and* 128-bit content collision), and the
digest strength framing is also loose — two `DefaultHasher` passes over the same fixed-key
SipHash-1-3 permutation with only a domain prefix is not "128-bit" against a chosen-input
adversary, though it is fine against accidental collision, which is all WR-04 needed. The defect
is the record, not the byte string.

**Fix:** Either make the claim true or state the real one. Exhaustive is cheap and needs no
`LayerId` import (which `output.rs` must not have):

```rust
// Synthetic 13-name vocabulary of the same shape production builds. 2^13
// subsets, sorted comma-joined — the whole reachable key space.
let vocab: Vec<String> = (0..13).map(|i| format!("Layer{i:02}")).collect();
let mut seen = std::collections::HashMap::new();
for mask in 0u16..(1 << 13) {
    let key = vocab.iter().enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, n)| n.as_str()).collect::<Vec<_>>().join(",");
    let content = attestation_downgrade_marker_content(&key);
    if let Some(prev) = seen.insert(content.clone(), key.clone()) {
        panic!("marker-content collision between {prev:?} and {key:?} -> {content}");
    }
}
```

and reword `output.rs:271-272` and the test doc to say what is actually checked.

---

### WR-07: the SPEC gates this round added do not run in CI for the change class they guard

**File:** `.github/workflows/ci.yml:46-49` (the `changes` classifier), with `:107`, `:456`

**Issue:** Round 3's stated closure for WR-02 is structural:

> Two new gates scan the SPEC **with the same helper** the rendered-string assertions use, so the
> mirrors are structurally prevented from diverging.

Those gates (`the_spec_never_prescribes_the_medium_label_command`,
`the_spec_ledger_does_not_point_abort_path_layers_at_the_event_log` in `--bin nono`;
`every_open_marker_in_code_has_a_ledger_row`, `every_spec_symbol_citation_resolves…` in
`layer_registry_selfcheck`) all run inside the `test` job, which is gated on:

```yaml
needs: changes
if: ${{ … && needs.changes.outputs.run_code_jobs == 'true' }}
```

and `run_code_jobs` is set to `false` when every changed file matches
`(^docs/)|(\.md$)|(\.mdx$)|(^LICENSE$)|(^\.github/ISSUE_TEMPLATE/)`. The contract document is
`proj/SPEC-windows-fail-direction-contract.md` — a `.md` file. `run_docs_checks` does not cover
`proj/` either.

So a pull request that edits **only** the SPEC runs zero jobs. Every one of the three SPEC defects
this phase actually found is a SPEC-only edit: restoring the prescriptive WR-17 remedy (WR-02),
re-inserting the table-terminating blank line (WR-04), deleting an `OPEN` ledger row. The gates
built to catch exactly those would not execute. The classifier itself is pre-existing and
correctly fail-open in every other respect (empty diff, missing base SHA, failed `git diff` all
leave both flags `true`); the new reliance on it is what this phase introduced.

**Fix:** Exclude the contract documents from the docs-only skip, so a SPEC edit runs the code jobs
that gate it:

```yaml
if [[ ! "${file}" =~ (^docs/)|(\.mdx$)|(^LICENSE$)|(^\.github/ISSUE_TEMPLATE/) ]] \
   || [[ "${file}" =~ ^proj/ ]]; then
  run_code_jobs=true
fi
```

(dropping the blanket `\.md$` in favour of an explicit docs-tree match, and force-including
`proj/`). Note this list must stay in sync with every `include_str!`/`read_to_string` of a
Markdown file in the test tree — worth a comment at both ends.

---

### WR-08: the new per-file literal floor has one literal of headroom on the smallest file

**File:** `crates/nono-cli/src/output.rs:2139-2147`

**Issue:** `PER_FILE_FLOOR: usize = 25` is a good idea (a global floor of 500 genuinely cannot see
one file going quiet). But measured against the real files today:

```
output.rs                                              421
exec_strategy_windows/launch.rs                        757
exec_strategy_windows/attestation.rs                    66
exec_strategy_windows/attestation_downgrade_event.rs    26   <- floor 25
main.rs                                                187
```

`attestation_downgrade_event.rs` clears the floor by one. Deleting two string literals from that
file — an ordinary refactor — trips a gate whose message reads "the extractor has gone quiet for
this file", pointing the next maintainer at the extractor rather than at their own edit. The
failure direction is loud, not silent, so this is a calibration/diagnosability problem rather than
a coverage hole, but a floor that a two-line edit crosses is not measuring what it claims.

**Fix:** Make the floor proportional and self-describing, e.g. a per-file floor of
`max(10, previous_count / 2)` recorded as a named constant per file, or simply lower
`attestation_downgrade_event.rs`'s floor to 10 with a comment stating it is a small file and the
signal there is "went to zero", not "went below 25".

---

_Reviewed: 2026-08-14T18:20:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard — round 4 (adversarial re-review of `e6438f96..860d4772`)_
