---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 36
subsystem: testing
tags: [windows, spec, citation-drift, gap-closure, round-4, cr-04, wr-32, cint-01]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-35's --no-fail-fast baseline, which recorded layer_registry_selfcheck as the
      round's one owned regression"
provides:
  - "A green layer_registry_selfcheck target — unblocking every later round-4 plan whose verify
    asserts suite-level state"
  - "looks_like_a_spec_citation hardened to reject the illustrative placeholder token
    structurally, independent of the document's quoting convention"
  - "layer_registry.rs's own citation-convention prose corrected at BOTH stale sites, with two
    narrow regression guards"
affects: [117-37, 117-39, 117-40, 117-41, 117-42, 117-43, 117-44, 117-45]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "An exclusion a gate depends on is a property of the MATCHER, never a writing convention the
      next author must remember — CR-04 is what the convention-only form costs"
    - "A regression guard for a stale citation asserts absence FILE-WIDE, not at the one site the
      finding named; the site-scoped form is what leaves the second instance"

key-files:
  created: []
  modified:
    - proj/SPEC-windows-fail-direction-contract.md
    - crates/nono-cli/tests/layer_registry_selfcheck.rs
    - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs

key-decisions:
  - "The stale `launch.rs:1237-1278` citation existed at TWO sites, not the one the plan's
    <interfaces> named. Both were fixed and the regression test asserts absence across the whole
    file rather than at a named location."
  - "WR-32's guards are deliberately narrow (two specific doc-defining strings) rather than a
    file-wide ban on raw `\\w+\\.rs:\\d+`, because layer_registry.rs legitimately cites other
    files' analogs in ordinary descriptive prose and a blanket rule would fail on those."
---

# Plan 117-36: CR-04 + WR-32 — Execution Summary

**Executed:** 2026-08-12 · **Wave:** 16 · **Requirement:** CINT-01

## Result

`cargo test -p nono-sandbox-cli --test layer_registry_selfcheck --no-fail-fast` → **14 passed, 0
failed** (baseline was 10 passed / 1 failed). **CR-04 is closed.** The round's one owned regression
from `117-BASELINE-ROUND4.md` is resolved, unblocking every later round-4 plan's suite-level verify.

## Task 1 — SPEC instance fix

Line 296's WR-19 ledger row carried an unquoted illustrative `` `file.rs::Symbol` `` span; wrapped
it in the doubled-quote convention lines 274 and 284 already use.

- `grep -c '`file\.rs::Symbol`-shaped'` → **0** (was 1)
- `grep -c '"file\.rs::Symbol"'` → **3** (274, 284, 296)

## Task 2 — Structural hardening of `looks_like_a_spec_citation` (CR-04)

Added a placeholder-token rejection after the `file_part` character-class check: the last
`/`-separated segment, `.rs` stripped, compared with `eq_ignore_ascii_case("file")`. No real source
file in this workspace is named literally `file.rs`, so such a span is always an example of the
citation FORMAT, never a citation.

New test `illustrative_format_example_is_not_treated_as_a_citation` covers five cases — the four the
plan specified plus a path-qualified positive
(`crates/.../launch.rs::is_dev_build_layout`) confirming the check reads only the last path segment.

### Perturbation proofs (both required by acceptance criteria; both run)

**Proof A — the structural fix, not the quoting fix, is what protects the gate.**
Reverted Task 1 (reintroduced the unquoted span at line 296) with Task 2's check in place:

```
unquoted reintroduced: 1
test every_spec_symbol_citation_resolves_to_a_real_definition ... ok
test result: ok. 1 passed; 0 failed; ... 11 filtered out
```

The gate stays green against the exact text that broke it. A future writer who forgets the quoting
convention cannot reopen CR-04.

**Proof B — the new check is what the previous gate lacked.**
With the SPEC still reverted, disabled ONLY the placeholder check (changed its comparand to
`"__perturbation_disabled__"`):

```
thread 'every_spec_symbol_citation_resolves_to_a_real_definition' panicked at
crates\nono-cli\tests\layer_registry_selfcheck.rs:748:5:
proj/SPEC-windows-fail-direction-contract.md cites a file.rs::Symbol that does not resolve to a
real definition anywhere in the document ...:
"file.rs::Symbol" -> resolved to C:\Users\OMack\Nono\crates\nono-cli\src\exec_strategy_windows\file.rs
                     (unreadable: The system cannot find the file specified. (os error 2))
test result: FAILED. 0 passed; 1 failed
```

CR-04 reproduced exactly on demand. Both perturbations were then reverted and the full target
re-run green.

## Task 3 — WR-32 doc-comment drift

- `LayerRegistryEntry::call_sites` field doc: `` `"file:line"` `` → `` `file.rs::Symbol` citations
  … content-verified against the cited file's real definitions ``.
- `token_arm` prose: `` (`launch.rs:1237-1278`) `` → `` (`launch.rs::select_windows_token_arm`) ``.
  `select_windows_token_arm` confirmed present at `launch.rs:1350` by direct grep before citing it.

Two narrow regression tests added, both reusing `read_layer_registry()`:
`call_sites_field_doc_describes_the_symbol_form` (asserts the symbol form appears in the 400-char
doc window immediately preceding `pub call_sites:`, and that `"file:line"` appears nowhere) and
`token_arm_doc_cites_a_symbol_not_a_raw_line_range`.

Final greps: `"file:line"` → 0, `launch.rs:1237` → 0, `launch.rs::select_windows_token_arm` → 2.

## Deviations

**1. The stale citation existed at two sites, not one.** The plan's `<interfaces>` named only the
`token_arm_names` module doc (line ~547). `ArmExpectancy`'s own doc at line 285 carried the
identical `launch.rs:1237-1278` range and was not mentioned. It surfaced only because the
acceptance criterion demanded a file-wide count of zero rather than a fix at the named site — a
site-scoped fix would have shipped with the second instance intact, which is this phase's recurring
failure mode. Both were fixed, and `token_arm_doc_cites_a_symbol_not_a_raw_line_range` asserts
absence file-wide rather than at a location.

**2. One acceptance criterion's arithmetic is off by construction.** The plan expects
`grep -c "launch.rs::select_windows_token_arm"` to return **1**; it returns **2**, because both
stale sites now cite the symbol. The criterion's intent (the symbol form is present) is satisfied;
its count assumed a single site. Recorded rather than silently reconciled.

**3. rustfmt reflowed two of the new tests** (a long `looks_like_a_spec_citation(..)` argument and
an `unwrap_or_else` closure). `cargo fmt --all` applied, `--check` clean, target re-run green
afterward.

## Verification

| Gate | Command | Result |
|---|---|---|
| Target suite | `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck --no-fail-fast` | **14 passed / 0 failed** |
| Format | `cargo fmt --all -- --check` | clean |
| Cross-target clippy (linux-gnu) | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |
| Cross-target clippy (apple-darwin) | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) | **PASS** — exit 0, 0 warnings, 0 errors |

Both cross-target gates ran locally on this host per CLAUDE.md's MUST rule; neither was deferred to
CI. `layer_registry.rs` is under `exec_strategy_windows/`, which triggers the rule regardless of
whether the file itself contains Unix `cfg` blocks.
