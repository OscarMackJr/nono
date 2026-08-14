---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-14T09:40:00Z
depth: standard
round: 2
files_reviewed: 36
files_reviewed_list:
  - .github/workflows/ci.yml
  - bindings/c/include/nono.h
  - bindings/c/src/lib.rs
  - bindings/c/src/types.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/agent_daemon/launch.rs
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
  critical: 1
  warning: 7
  info: 0
  total: 8
status: issues_found
---

# Phase 117: Code Review Report — Round 2 (post-fix re-review)

**Reviewed:** 2026-08-14T09:40:00Z
**Depth:** standard
**Files Reviewed:** 36
**Status:** issues_found

## Summary

I independently re-derived every claimed fix rather than reading the fix report's
assertions. Most of them hold up, and several hold up *better* than the report claims:

- **CR-03 (registry discard)** — verified. `read_required_layers` carries the parsed names;
  `is_unconfigured()` (`machine_policy.rs:331`) and `validate()` (`:358`) both provably ignore
  the field, so the runtime change is behaviour-neutral today and the gap is now at the single
  consumer, as claimed.
- **WR-08 (probe-kind-generic evidence)** — verified fail-closed. `match entry.id { WfpEgressFilters => …, _ => false }`
  strictly narrows what can confirm; `exactly_one_row_is_confirmed_by_enforcing_component_report`
  is an equality assertion, so it cannot go vacuous.
- **CR-02 (daemon drift gate)** — verified non-vacuous by re-running the gate's own logic against
  `agent_daemon/launch.rs`: it skips 1722 lines in 3 `#[cfg(test)]` module bodies and finds each of
  the 5 daemon-expected rows at exactly one production site (`:1411,:1422,:1431,:1446,:1466`).
  Renaming any one of them fails the gate. `checked >= 5` matches the registry's real count of 5
  exactly (I enumerated the `EntryPath::Daemon` cells: 6 rows carry one, `MinifilterAbsence` is
  excluded by `ProbeKind::NotApplicable`).
- **WR-01 (collapsed literals)** — verified. Re-ran the gate's predicate: 1417 literals scanned
  (floor 500), 0 offenders. A workspace-wide sweep for the same shape returns only legitimately
  column-aligned fixtures.
- **WR-02 (FFI)** — verified. `nono.h`'s new block is byte-consistent with cbindgen's style for
  every other doc-commented variant in the file (`/**` block, same indent, appended after
  `CANCELLED = 14`, `OTHER = 99` unmoved), so it is generated, not hand-edited. `map_error`
  (`bindings/c/src/lib.rs:211`) already had an explicit `LayerAttestationFailed` arm, so no second
  FFI gap.
- **7a8fcd36 (citation-scan narrowing)** — verified *no* real coverage was lost: the whole-file scan
  found 26 symbol citations, the region-scoped scan finds 23, and the 3 dropped are exactly the
  three false positives (`.rs::`, `file.rs::Symbol`, and the WR-08 assertion prose).

The defects are concentrated in two places:

1. **The WR-04 fix introduced a new D-28 violation.** Making the dedup marker
   content-authoritative meant writing the un-redacted, comma-joined `LayerId` set to a plain
   file under `%LOCALAPPDATA%\nono\sessions\…` — a path the confined child can read on three of
   the four token arms. The fix report reasoned only about *visibility to the operator* ("fail
   direction is toward MORE visibility") and did not consider *disclosure to the confined
   process*, which is the property D-28 exists to protect and which WR-16 hardened with
   fail-secure path validation one phase earlier.

2. **The class sweeps stopped at code and did not reach the contract document or the sibling
   assertion two lines away.** WR-03 claims "every remaining `/setintegritylevel Medium` mention
   in the tree is corrective or negated" — the SPEC's own D-15 ledger still prescribes it as "the
   real remedy". WR-06 claims "both mirrors now assert the CLASS" — the sibling CR-06 assertion in
   the *same test function's module* is still the exact narrow-needle shape WR-06 was raised to
   eliminate. And the CR-01 `production()` helper still fails to exclude 6 of `launch.rs`'s test
   modules for a reason its own doc says it fixed.

I also confirmed, by replicating the gate logic line-for-line, that the CR-01 gate is **not**
vacuous today (`hits` = exactly 1 in `output.rs` + 1 in `launch.rs` = the floor of 2;
`main_mentions` = exactly 1). Its residual defects are structural, not live.

Per the review brief I did not re-litigate CR-02's operator-wiring decision, WR-12, or WR-14's
substance. WR-14's *recorded rationale* is wrong on one point and is reported below as WR-07.

## Critical Issues

### CR-01: WR-04's fix persists un-redacted downgraded `LayerId` names to a path the confined child can read (D-28 violation)

**File:** `crates/nono-cli/src/output.rs:187-212` (writer), `:241-244` (reader), with
`crates/nono/src/state_paths.rs:89` and `:60-66`

**Issue:** Before the fix the dedup marker was a **zero-byte** file whose *name* was a 64-bit
`DefaultHasher` digest of the downgraded-layer set. The fix now writes the key verbatim:

```rust
f.write_all(dedup_key.as_bytes())          // output.rs:199
```

`dedup_key` is the sorted, comma-joined `Debug`-format `LayerId` list built at
`launch.rs:1598-1602`. The destination is
`%LOCALAPPDATA%\nono\sessions\<session_id>\attestation-downgrade\<hash>` — an ordinary user file
created with `OpenOptions::new().write(true).create_new(true)`, with no DACL hardening and no
mandatory label.

D-28 is a locked decision that this codebase enforces aggressively everywhere else:

- `launch.rs:1605-1640` gates all three `ProceedDowngraded` emission sites on
  `log_target_is_private()` so layer names never reach a channel the child shares.
- `cli_bootstrap.rs`'s `log_target_is_private()` was hardened in WR-16 to canonicalize the
  `--log-file` path and component-wise `Path::starts_with`-check it against this launch's own
  granted `CapabilitySet` paths, failing secure on any error — specifically so a log file inside
  the granted workspace cannot leak the names.
- `downgrade_detail_pointer`'s `Stderr` arm (`launch.rs:1496-1502`) exists solely to say
  "layer names are withheld".
- `attestation_downgrade_event.rs:84,108` and `output.rs:109` all repeat the rule.
- The SPEC even carries a standing manual-verification item (`etw-applog-child-readability`)
  because nobody has proven the Application event log is *not* child-readable.

The marker file is subject to **none** of that. Reachability from the child:

| Token arm | Can the child read `%LOCALAPPDATA%\nono\sessions\…`? |
|---|---|
| `Null` | Yes — same user token, ordinary DACL |
| `WriteRestricted` | Yes — `WRITE_RESTRICTED` applies restricting SIDs to **write** access checks only |
| `LowIlPrimary` / `BrokerLaunchNoPty` (Low-IL) | Yes — MIC's default policy is `NO_WRITE_UP`; reads up are permitted unless `NO_READ_UP` is set, and nothing labels the sessions tree |
| AppContainer grandchild | No — package SID not granted there |

So on the `Stderr` and `None` detail channels — the two cases where the code deliberately
withholds or cannot record the names anywhere else — the marker file becomes the *only* durable,
plaintext record of exactly which confinement layers failed to confirm, sitting where the
untrusted process can read it. That is precisely the reconnaissance signal D-28 was written to
deny, and it is a regression: the pre-fix zero-byte marker leaked nothing beyond an enumerable
64-bit hash.

The fix report's own risk note for WR-04 ("Fail direction is toward MORE visibility") reasons only
about the operator-facing direction and never evaluates the disclosure direction.

**Fix:** Do not store the plaintext key. Keep the marker content-authoritative without
disclosing the set — e.g. store a keyed digest whose key is not derivable by the child, or store a
value that only distinguishes collisions without naming layers:

```rust
// output.rs — writer
use std::hash::{DefaultHasher, Hash, Hasher};
// A second, independent digest under a per-session random salt held only in
// the supervisor's session state. Distinguishes a genuine repeat from a
// 64-bit filename collision, without naming any LayerId on a channel the
// confined child can read (D-28).
let mut h = DefaultHasher::new();
session_secret.hash(&mut h);      // supervisor-only, never granted to the child
dedup_key.hash(&mut h);
f.write_all(format!("{:016x}", h.finish()).as_bytes())?;
```

and mirror it in `marker_says_already_announced`. If a per-session secret is unwanted, revert to
the zero-byte marker and accept the collision case (announce-on-collision can instead be obtained
by including the *length* of the layer set in the filename, which discloses nothing).

Either way, add a D-28 regression test in the same shape as
`launch.rs:4005`'s discovery-based withholding scan: assert that no file written under
`sessions_dir()` on the downgrade path contains any `LayerId`'s `{id:?}` text.

Also update `print_attestation_downgrade_banner`'s doc (`output.rs:75-96`), which currently says
`dedup_key` is "an opaque `&str` this function never parses or displays, only hashes" — after this
fix the function *does* persist it verbatim, so the doc is false in the same edit that made it so.

## Warnings

### WR-01: The CR-01 gate still scans 1773 lines of `launch.rs`'s test modules as production, and its doc asserts two guarantees the code does not provide

**File:** `crates/nono-cli/src/output.rs:1856-1955` (`production`), `:1995-2001` (the `hits` floor)

**Issue:** The fix report states that `production()` "now skips cfg-test-gated inline modules by
brace region, which has neither failure mode." I re-ran the helper's exact logic against the three
scanned files. It does not:

```
output.rs : production entries 1114 / 2039, skipped regions [(1372,1507), (1510,2038)]   OK
launch.rs : production entries 3632 / 5889, skipped regions [(2741,2769),(2772,2903),
            (2906,3017),(3020,3123),(3126,3283),(5799,5888)]
main.rs   : production entries  189 /  889, skipped regions [(316,888)]                  OK
```

Six of `launch.rs`'s twelve test modules are **not** skipped — `job_hardening_tests` (`:3289`),
`attestation_gate_tests` (`:3377`), `detached_stdio_tests` (`:4574`), `broker_dispatch_tests`
(`:4673`), `env_filter_tests` (`:5281`), `write_deny_low_il_broker_no_pty_tests` (`:5544`) —
because each carries `#[allow(clippy::unwrap_used)]` between the `#[cfg(all(test, …))]` attribute
and the `mod` line, and the loop clears `pending_test_attr` on any intervening line
(`output.rs:1913-1929`). **1773 production entries** come from inside those modules. The daemon
gate written in the same fix pass (`layer_registry.rs:1428-1432`) *does* handle this
(`if t.starts_with("#[") || t.starts_with("///") { continue; }`) — the two mirrors of the same
helper disagree.

Two further problems in the same block:

1. **The doc states an assertion that does not exist.** `output.rs:1859-1860`: "The marker is now a
   top-level `#[cfg(test)]` line, **which is asserted to EXIST in every scanned file**." There is
   no such assertion. `:1900-1901`: "**The caller asserts non-vacuity per file**; that is what
   caught the truncation above, and it is the only thing that can." The caller asserts
   `hits >= 2` — a **global sum across two files** (`:1957`, `:1995`). Today each file contributes
   exactly 1, so the floor is tight; but nothing prevents `launch.rs` going to 0 while `output.rs`
   drifts to 2, and the doc tells the next maintainer that case is covered. This is the same
   "the record misstates the mechanism" shape that made the original CR-01 vacuous.
2. **`is_cfg_test_attr` (`:1881`) would classify `#[cfg(not(test))]` as a test gate** (it matches
   the substring `test)`), so a `#[cfg(not(test))] mod foo {` — a *production* module — would be
   silently skipped in its entirety. No such attribute exists in the three scanned files today, so
   this is latent, not live.

Secondary: the arm walk-back (`:1969-1977`) searches **raw** lines including comments, so a
comment naming `DowngradeDetailChannel::EventLog` within 12 lines above a violating string would
satisfy the gate.

**Fix:**
```rust
// output.rs — align with layer_registry.rs's daemon gate, and tighten the attr test
fn is_cfg_test_attr(t: &str) -> bool {
    t.starts_with("#[cfg(") && !t.contains("not(test")
        && (t.contains("test)") || t.contains("test,"))
}
// …inside the loop, before clearing the pending flag:
if pending_test_attr && (t.starts_with("#[") || t.starts_with("///")) {
    idx += 1;
    continue;                       // attributes/docs may sit between cfg and item
}
```
and replace the global floor with a per-file one so the doc becomes true:
```rust
let mut per_file: Vec<(&str, usize)> = Vec::new();
// …push (label, hits_in_this_file) per iteration…
for (label, n) in &per_file {
    assert!(*n >= 1, "CR-01: the WR-26 class gate matched {n} site(s) in {label}; \
        each scanned file must contribute at least one known EventLog-arm site, or the \
        needle has drifted out of that file's production text");
}
```
Additionally assert that each scanned file actually contained at least one skipped test region,
so a marker change cannot silently turn the "production half only" claim into a whole-file scan.

---

### WR-02: The SPEC's D-15 ledger still prescribes the remedy WR-03 removed from the code, and still names the channel WR-27 forbade

**File:** `proj/SPEC-windows-fail-direction-contract.md:294` (WR-17 row); secondary `:262` (RF-13 row)

**Issue:** WR-03's fix report claims: "Swept the class: every remaining `/setintegritylevel
Medium` mention in the tree is corrective or negated, none prescriptive." The sweep covered
`crates/` but not `proj/`. The SPEC's WR-17 ledger row states, in the present tense, as the
recorded resolution:

> `render_error_for_operator` now branches on the failed layer name: `MandatoryIntegrityLabel`
> gets the real remedy (`icacls <path> /setintegritylevel Medium`); every other layer points at
> the Windows Application event log instead of the always-wrong check-only command.

Both halves are now false **and** both are the exact operator-harming guidance this phase later
eliminated:

- `icacls /setintegritylevel Medium` was verified on-host to write a Medium label and re-trigger
  the identical abort (CR-06); `main.rs:363-367` carries a regression assertion forbidding it.
- Pointing other layers at the Windows Application event log is what WR-27 removed; both
  `main.rs:427-445` and `output.rs:2003-2036` now *assert* that the event log is named only as an
  explicit negation.

So the contract document — the one an operator or the next planner reads — prescribes a command
the code proves wrong and a channel the code's own tests forbid. This is precisely the "two
mirrored sites carry contradictory guidance for one condition" defect WR-03 was raised to close,
relocated one document over. It also self-certifies with a stale re-run receipt
(`grep -n "\"MandatoryIntegrityLabel\" =>" … (2026-08-11) → line 262`), which reads as verified.

Secondary, same class and same document: the RF-13 row (`:262`) still says "The reader now
DETECTS a configured sub-key and emits a loud warning" — after CR-03 the reader also **carries**
the value into `RequiredLayersPolicy.required`. Benign in direction, but it is the row WR-12's
skip rationale leans on.

**Fix:** Amend the WR-17 row's Resolution column to record what the code does *now* (label-removal
via `SetNamedSecurityInfoW(.., LABEL_SECURITY_INFORMATION, .., <empty ACL>)`; non-label layers
pointed at `-vv --log-file`, with the event log named only as a negation), with a forward
reference to the CR-06/WR-27 rows that superseded it. Update RF-13's first sentence to
"reads the sub-key into `RequiredLayersPolicy.required` and warns that no consumer enforces it".

Then close the class rather than the instance: `layer_registry_selfcheck.rs` already
content-verifies every `file.rs::Symbol` citation in the whole SPEC — add a sibling gate asserting
the SPEC contains **no** occurrence of `/setintegritylevel Medium` that is not immediately
qualified as non-remedial, mirroring `main.rs`'s own regression assertion.

---

### WR-03: The sibling CR-06 assertion is still the narrow-needle shape WR-06 was raised to eliminate

**File:** `crates/nono-cli/src/main.rs:362-367`

**Issue:** WR-06's fix rewrote `render_error_for_operator_names_a_reachable_channel_for_non_label_layers`
to "assert the CLASS, not one verb" and mirrored the rule in `output.rs`. The assertion **20 lines
above it, in the same test module, guarding the same rendered string** was not touched:

```rust
assert!(
    !lines[1].contains("/setintegritylevel Medium`."),
    "the Medium-label command must never be prescribed as the remedy (CR-06): {}",
    lines[1]
);
```

The needle is the literal `` /setintegritylevel Medium`. `` — command, closing backtick, **and a
sentence-final period**. It rejects exactly one historical sentence ending. `` `icacls <path>
/setintegritylevel Medium` to clear this ``, `` run `icacls /setintegritylevel Medium` ``, or any
phrasing not ending the sentence right there evades it entirely. The class the assertion names
("must never be prescribed as the remedy") is far wider than the predicate. The current text at
`main.rs:295` is a legitimate *negated* mention ("`icacls /setintegritylevel Medium` writes a
Medium label and re-triggers this same abort") — structurally identical to the event-log negation
at `:305` that WR-06 handled correctly by requiring the negation rather than narrowing the needle.

This is the phase's signature failure mode surviving in the file the fix pass edited, in the
function directly above the one it fixed.

**Fix:** Apply WR-06's own pattern:
```rust
const MEDIUM_LABEL_CMD: &str = "/setintegritylevel Medium";
let normalised = lines[1].split_whitespace().collect::<Vec<_>>().join(" ");
let mut mentions = 0usize;
for (pos, _) in normalised.match_indices(MEDIUM_LABEL_CMD) {
    mentions += 1;
    let following = &normalised[pos + MEDIUM_LABEL_CMD.len()..];
    assert!(
        following.contains("writes a Medium label") || following.contains("re-triggers"),
        "CR-06: the Medium-label command is mentioned other than as an explicit warning \
         that it does NOT clear the condition: {}",
        lines[1]
    );
}
assert!(mentions >= 1, "CR-06 non-vacuity: the remediation no longer warns about \
    `/setintegritylevel Medium` at all — restore the warning or retire this guard");
```

---

### WR-04: The D-15 ledger table is broken by a stray blank line, so WR-10's record is not a row of that table

**File:** `proj/SPEC-windows-fail-direction-contract.md:299-301`

**Issue:** The two rows the fix pass appended are separated by a blank line:

```
299: | CR-02 (Iteration 6, code-review fix pass) | … |
300:
301: | WR-10 (Iteration 6, code-review fix pass) | **OPEN — recorded, not fixed.** … |
```

In GitHub-Flavored Markdown a blank line terminates the table. Line 301 has no preceding header +
delimiter pair, so it renders as a literal paragraph of pipe characters, not as a ledger row.

This matters specifically because WR-10 was **consciously skipped** on the sole basis that "the
review explicitly sanctions this alternative (record the conflation explicitly in the SPEC's …
table (D-15))". The recorded mitigation is the entire justification for not fixing a live
naming/claim-precision defect in a cross-binary wire contract — and the record is not in the table
it is claimed to be in. (The `WR-10 OPEN` marker on `layer_registry.rs:924` is intact and
greppable, and `every_spec_symbol_citation_resolves_to_a_real_definition` still validates the row's
`nono-shell-broker/src/main.rs::broker_resume_gate` citation, which does resolve — `main.rs:369`.)

**Fix:** Delete line 300 so the WR-10 row joins the table. Then make it structural — the SPEC
already has parsing gates; add one that asserts the discrepancy ledger contains a row for every
`OPEN` marker in the tree:

```rust
#[test]
fn every_open_marker_in_code_has_a_d15_ledger_row() {
    let ledger = d15_ledger_rows(&read_spec());          // parse rows of THAT table
    for marker in grep_open_markers(&workspace_sources()) {  // e.g. "WR-10 OPEN", "WR-14 OPEN"
        assert!(ledger.iter().any(|r| r.starts_with(&format!("| {marker} "))),
            "{marker} is marked OPEN in code but has no row in the SPEC's D-15 \
             \"Contract vs. code discrepancies\" ledger");
    }
}
```

---

### WR-05: Stale line citations survive in `layer_registry.rs`'s module doc and in `layer_registry_meta_test.rs`, covered by no gate

**Files:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:114-124` (and 25 line
citations file-wide); `crates/nono-cli/tests/layer_registry_meta_test.rs:175`

**Issue:** NR3-08 converted every `call_sites` array entry to line-drift-immune `"file.rs::Symbol"`
form, and WR-19 swept the SPEC for the same class. Neither sweep reached (a) `layer_registry.rs`'s
own module doc comment, or (b) the reason strings in `layer_registry_meta_test.rs`'s
`MANUALLY_VERIFIED` list. Both carry raw line citations, and both are outside every citation gate:
`registry_call_sites_exist` now scans **only** `call_sites: &[` regions (commit `7a8fcd36`), and
`every_spec_symbol_citation_resolves_to_a_real_definition` scans only the SPEC.

`layer_registry.rs` carries **25** `file.rs:<line>` citations, **zero** of them inside a
`call_sites` array. I checked six at random; all six are wrong:

| Citation | Claimed target | Actual content at that line |
|---|---|---|
| `launch.rs:2236` (`:117`) | `verify_broker_authenticode` definition | inside `CreateProcessW`/`process_info` block; real definition is `:2666` |
| `launch.rs:1505-1506` (`:118`) | `verify_broker_authenticode` call site | `downgrade_detail_pointer`'s `None` arm; real call sites `:1906`, `:2235` |
| `launch.rs:2190` / `:2194` (`:119`,`:121`) | `is_dev_build_layout` | `InitializeProcThreadAttributeList` teardown; real definition `:2620` |
| `launch.rs:375` (`:78`) | — | middle of an unrelated error-message literal |
| `mod.rs:484` (`:794`) | `prepare_live_windows_launch` | inside `wfp_composition_report`'s doc block |
| `agent_daemon/launch.rs:959` (`:824`) | daemon job creation | an orphaned-process comment |
| `crates/nono/src/sandbox/windows.rs:2247` (`:101`) | — | a bare `)));` |

`layer_registry_meta_test.rs:175` is the same defect with an extra edge: it carries the *exact*
citation pair the SPEC's WR-19 row declared stale and fixed —
`` (`launch.rs:2190`,2194`) `` — with mismatched backticks. Two mirrors of one fact; one was fixed,
the other was not.

None of this is fix-pass-introduced (I diffed `launch.rs:1503-1510` and `:1828-1830` at the phase
base `334530af` and both citations were already stale there), but it is live drift in files this
phase declares authoritative, and the WR-05/`7a8fcd36` narrowing means no gate will ever see it.

**Fix:** Convert all 25 to `"file.rs::Symbol"` form and extend the content-verifier past the
`call_sites` regions. The safest shape, since narrowing is the fail-open direction, is to
content-verify every `file.rs::Symbol` citation *anywhere* in `layer_registry.rs`,
`layer_registry_meta_test.rs` and `layer_force_unavailable.rs` (the SPEC gate already does exactly
this for the SPEC), and to add a gate that **rejects** any `file.rs:<digits>` citation in those
files outright:

```rust
#[test]
fn no_line_number_citations_remain_in_the_registry_surface() {
    for (label, src) in registry_surface_sources() {
        for (n, line) in src.lines().enumerate() {
            assert!(!LINE_CITATION_RE.is_match(line),
                "{label}:{}: raw \"file.rs:<line>\" citation — these drift silently \
                 (NR3-08/WR-19); use \"file.rs::Symbol\" form", n + 1);
        }
    }
}
```

---

### WR-06: `no_downgrade_surface_literal_has_a_collapsed_continuation`'s extractor silently drops literals on lines containing a `'"'` char literal

**File:** `crates/nono-cli/src/output.rs:1744-1766`

**Issue:** `string_literals` is documented as "deliberately naive (no raw-string or char-literal
handling): the files it is pointed at contain neither on the lines that matter". That is not true
of `output.rs`, which the scan includes and which contains `else if c == '"' {` twice — inside
`string_literals` itself (`:1755`, `:1763`). On those lines the extractor opens a string at the
char literal's quote and never closes it, so the entire rest of the line is discarded rather than
scanned. The doc's stated safety argument — "a false positive here fails the build loudly rather
than silently passing" — describes the wrong direction: this is a false *negative* (silently
scanning less), which is exactly the failure mode the `checked >= 500` floor exists to catch and
is too coarse to catch (1417 scanned today).

Low impact today (the dropped content is the extractor's own body), but it is a self-referential
blind spot in an anti-vacuity gate.

**Fix:** Skip char literals in the scanner, or exclude the extractor's own definition region:
```rust
} else if c == '\'' {
    in_char = true;              // consume up to the matching unescaped '
}
```
and tighten the floor to a per-file minimum so a single file going quiet is visible.

---

### WR-07: The `WR-14 OPEN` record misstates why `probe_in_job`'s failure is unreachable, and understates the FFI/embedder surface

**File:** `crates/nono/src/error.rs:506-529`

**Issue:** The brief permits re-reporting a skipped finding when "the recorded rationale is
actually wrong". The block states:

> All three are UNREACHABLE in a shipped build today: … (3) requires a null job handle no
> production caller passes …

That is a claim about the **callee's input**, and it is not what makes the case unreachable. Both
production callers pass a real handle, yes — but neither would surface the error even if they did
not, because both **swallow** it:

- `attestation.rs:312` → `classify_probe_outcome(probe_in_job(...))`, whose `Err(_)` arm
  (`:265`) maps to `LayerAttestationStatus::Unconfirmed`. The `NonoError` is dropped on the floor.
- `agent_daemon/launch.rs:1419` → `matches!(probe_in_job(process, job), Ok(true))`. Same.

So `probe_in_job`'s `LayerAttestationFailed` can never reach `remediation()` **through nono's own
binaries at all**, regardless of the job handle. Getting this backwards matters for the co-fix the
block mandates ("THIS ARM MUST BE FIXED IN THE SAME CHANGE" as WR-12): an implementer reading it
will look for a null-handle guard at the call site rather than noticing the two swallow points, and
will conclude the case became reachable when it did not.

Conversely the record **understates** one surface: `probe_in_job` is `pub` in `crates/nono`
(`attestation.rs:274`) and is therefore reachable by every FFI/Python/TypeScript embedder, none of
which swallow it. An embedder that calls it with a null job today gets
`ClearStaleLayerResidue { layer: "JobObjectContainment" }` — the mis-targeted remediation, in a
shipped build, right now. "Unreachable in a shipped build" is true only of nono's own binaries.

**Fix:** Correct the block:
```
//   3. `crate::attestation::probe_in_job`'s null-job refusal. Unreachable through
//      nono's own binaries not because no caller passes a null handle, but because
//      BOTH production callers discard the Err:
//        - nono-cli  `attestation.rs::classify_probe_outcome` -> Unconfirmed
//        - nono-agentd `agent_daemon/launch.rs` -> matches!(.., Ok(true))
//      It IS reachable for FFI/embedder callers, which see the mis-targeted
//      remediation today.
```

---

_Reviewed: 2026-08-14T09:40:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard — round 2 (re-review of fix pass `334530af..HEAD`)_
