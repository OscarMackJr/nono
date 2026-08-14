---
phase: 117-fail-direction-contract-startup-self-attestation
fixed_at: 2026-08-14T16:40:00Z
review_path: .planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md
iteration: 2
round: 3
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 117: Code Review Fix Report — round 3

**Source review:** `117-REVIEW.md` (round 2, reviewed 2026-08-14T09:40:00Z; 1 Critical + 7
Warning = 8 findings, all in scope)
**Scope:** `critical_warning` — the review reported 0 Info findings, so this is every finding.

> The round-2 fix report is preserved at `117-REVIEW-FIX.round1.md`.

| | Count | Findings |
|---|---|---|
| Fixed | 8 | CR-01, WR-01, WR-02, WR-03, WR-04, WR-05, WR-06, WR-07 |
| Skipped | 0 | — |

7 commits on `gsd-reviewfix/117-r3-1333`, fast-forwarded onto
`milestone/v2.13-carryforward-closeout`. All carry DCO sign-off.

---

## The standing obligation for this round

Round 3 exists because round 2's fix pass created its own blocker: WR-04's fix was evaluated
against one fail direction (operator visibility) and never against the other (disclosure to the
confined child). Every runtime-behaviour change below therefore carries an explicit
**two-direction analysis**, and every guard carries a **perturbation proof** plus a
**non-vacuity** argument that is about *correctness of coverage*, not merely non-emptiness.

Two things worth stating up front, because both were caught by running things rather than by
reading them:

1. **My own first draft of the WR-03 helper was vacuous on the WR-02 mirror.** It took "the rest
   of the text" as the window in which a qualifying negation had to appear, which is fine for a
   one-sentence remediation string and useless on a 2000-character Markdown ledger cell — the
   restored prescriptive SPEC text PASSED because a correction 400 characters later in the same
   cell satisfied it. Found by running the perturbation, not by inspection. Window now bounded.
2. **My own CR-01 test introduced a flaky failure** in a pre-existing latency assertion, which
   `cargo test --workspace` surfaced as 15 failures against the known 14-failure baseline. I
   characterised it (8 runs alone vs. 3 runs paired) rather than declaring it environmental, and
   fixed the contention. Detail under CR-01 below.

---

## Fixed

### CR-01: WR-04's fix persisted un-redacted `LayerId` names to a child-readable path (D-28)

**Files:** `crates/nono-cli/src/output.rs`, `crates/nono-cli/src/exec_strategy_windows/launch.rs`
**Commits:** `15215d37`, plus `f55bd174` (test-contention repair, below)
**Status:** fixed — **requires human confirmation** (this is the round's only runtime-behaviour
change)

Round 2 bought marker content-authority by writing `dedup_key` — the plaintext, comma-joined
`LayerId` set — verbatim into
`%LOCALAPPDATA%\nono\sessions\<id>\attestation-downgrade\<hash>`. The marker content is now a
**128-bit domain-separated `DefaultHasher` digest** (two passes under distinct constant prefixes,
because `DefaultHasher` is fixed-key and an undifferentiated second pass would just repeat the
first). Filename gains a `.v2` suffix so a round-2 plaintext marker is not consulted.

**Two-direction analysis.**

*(a) What does this now expose that it did not before, and to whom — can the confined child
read it on the `Null`, `WriteRestricted` and Low-IL arms?*
**Nothing new, on any arm.** The child *can* read this file on all three (Null: same token;
WriteRestricted: `WRITE_RESTRICTED` applies restricting SIDs to WRITE access checks only; Low-IL:
MIC defaults to `NO_WRITE_UP`, reads up are permitted, and nothing labels the sessions tree). That
reachability is unchanged — what changes is what is *there* to read. The content is now a
deterministic function of the same input that already determines the file's NAME, so an observer
who can invert one can invert the other, and an observer who cannot enumerate the key space
learns nothing from either. **Marginal disclosure relative to the file merely existing under that
name is exactly zero** — which is the pre-phase baseline the review itself endorses ("the pre-fix
zero-byte marker leaked nothing beyond an enumerable 64-bit hash"). Relative to round 2 it
strictly decreases: a child that does not enumerate previously got the names for free.

*(b) What does this now deny that it did not before?*
**Nothing.** The set of `(session, key)` pairs that suppress the banner is identical — the digest
is injective for every input this code can produce, pinned by an explicit injectivity assertion —
so absent / zero-byte / mismatched / unreadable markers still ANNOUNCE. Operator-visible
behaviour is bit-identical; only the on-disk byte string changed.

**On the unforgeability constraint the brief carried forward.** Round 2's stated rationale was
that an offline-computable key over a 13-element enum lets an attacker pre-plant the marker. That
is true, and round 2 **did not fix it** — its own doc says so ("an attacker can enumerate and
pre-plant every valid marker"). I have not traded it away, because there was nothing to trade:
this fix preserves 100% of the authority round 2 actually achieved (collision detection,
zero-byte rejection, tamper rejection) while closing the disclosure hole.

I did **not** achieve unforgeability, and I am saying so rather than implying it. It is not
reachable from here: on the `Null` and `WriteRestricted` arms the confined child runs under the
same user *and the same integrity level* as the supervisor, so there is no on-disk location this
process can read across spawns that that process cannot also write. A keyed MAC needs a secret,
and any secret stored where the next supervisor invocation can read it (dedup must survive across
processes — the hook path re-enters on every tool call) is readable by a same-IL same-user child
too. The doc now states this precisely instead of blaming the 13-element key space. **If an
operator wants unforgeability, it requires a design decision this fix pass cannot take** — e.g.
DACL/label-hardening the sessions subtree (which only helps the Low-IL arms) or moving dedup state
off the filesystem entirely.

**Guards, with perturbation proofs:**

- `launch.rs::downgrade_marker_files_never_contain_a_layer_name` drives the REAL writer over the
  REAL `LayerId::ALL` vocabulary on the `DowngradeDetailChannel::None` arm (the worst case: the
  channel that records nothing anywhere else), then reads back every byte left under
  `sessions_dir()`. Non-vacuity asserted on **both** the needle set (`>= 13` names, from the enum,
  not a hand-written list) and the file set (non-empty, asserted *before* the scan).
  **PERTURBATION:** restoring `f.write_all(dedup_key.as_bytes())` FAILS with all 13 layer names
  listed. Note the reader-side test stayed **GREEN** under that same perturbation — a
  filesystem-level assertion was required, and a unit test of the helper would have shipped the
  defect again.
- `only_an_exact_key_match_suppresses_the_banner` gains case 3b: the plaintext key must NOT be
  accepted content. **PERTURBATION:** widening the reader to also accept `dedup_key` FAILS there.
- `marker_content_never_names_a_layer_and_stays_injective` pins shape, non-disclosure and
  injectivity in `output.rs` **without importing the layer type**, preserving that module's
  structural D-28 property.

Also corrects `print_attestation_downgrade_banner`'s doc, which claimed `dedup_key` was "never
parses or displays, only hashes" in the same edit that made the function persist it verbatim.
That statement is true again.

**Self-inflicted regression, found and fixed (`f55bd174`).** `cargo test --workspace` returned 15
failures against the known 14-failure Windows-host baseline; the extra one was
`attestation_downgrade_banner_cold_vs_warm_dedup_marker_latency`. Measured **alone** on the same
compiled binary, 8 consecutive runs: warm 1.6–15.7ms, all green. Measured **concurrently** with
my new D-28 test — which does `create_dir_all` + `remove_dir_all` under the same sessions root on
another harness thread — one run of three hit **235.68ms**, tripping the 100ms bound. So: harness
filesystem contention I introduced, not a latency regression in the code (the CR-01 change adds
two `DefaultHasher` passes and nothing else). Both tests now take one shared
`SESSIONS_ROOT_TEST_LOCK`, acquired through poisoning. 5/5 paired runs green afterwards, and the
workspace suite is back to exactly 14. I rejected weakening the 100ms bound (it is a real D-24
budget), `#[ignore]` (a gate people learn to ignore is worse than no gate), and redirecting
`LOCALAPPDATA` (CLAUDE.md forbids env mutation in parallel tests — it already causes several of
the 14 baseline failures).

---

### WR-01: the CR-01 gate scanned 1773 lines of test code as production

**Files:** `crates/nono-cli/src/cfg_test_regions.rs` (new), `output.rs`, `layer_registry.rs`,
`main.rs` · **Commit:** `61e4ea40`

The two mirrors of the `#[cfg(test)]`-region classifier were written **in the same fix pass** and
disagreed. Rather than align them, I **eliminated the mirror**: there is now exactly one
implementation, `crate::cfg_test_regions` (test-only), with unit tests for the rule itself, and
both gates call it. A third gate needing the rule must call it too.

Fixed in the shared rule:
- Attributes and doc comments between `#[cfg(test)]` and its `mod` line no longer break the
  region. This is the live defect: six of `launch.rs`'s twelve test modules carry
  `#[allow(clippy::unwrap_used)]` there.
- `#[cfg(not(test))]` was classified as a test gate by **both** copies (it contains the substring
  `test)`), so a `#[cfg(not(test))] mod foo {` — production code — would have been skipped whole.
  Latent, not live; now excluded.

Two doc claims made **true** rather than deleted:
- *"asserted to EXIST in every scanned file"* — each scanned file must now contribute at least one
  **skipped test region**.
- *"The caller asserts non-vacuity per file"* — the floor was a **global sum of 2 across two
  files**. It is now per file.

Secondary from the finding: the arm walk-back now searches production entries only. It previously
searched raw lines, so a comment naming `DowngradeDetailChannel::EventLog` within 12 lines above a
violating string satisfied the gate.

**Non-vacuity is asserted on CORRECTNESS, not emptiness.** `!skipped_regions.is_empty()` would
have stayed green through the actual defect. Instead: **no `#[test]` attribute may appear in any
file's production half** — a leaked module brings its `#[test]`s with it, so this catches the class
directly without restating the classifier's own rule.

**PERTURBATIONS (each fails only with the fix in place):**
1. Restore the pending-flag-clears-on-any-line bug → FAILS: *"41 `#[test]` attribute(s) appear in
   the PRODUCTION half of exec_strategy_windows/launch.rs (first at line 3323)"*. The daemon gate
   stayed GREEN under this same perturbation — which is precisely why the leak check lives in
   `output.rs`'s gate.
2. Drop the `not(test` exclusion → the rule test FAILS on `#[cfg(not(test))]`, and
   `not_test_modules_are_production` FAILS.
3. Break `launch.rs`'s only EventLog site **and** add a second one in `output.rs`, so the OLD
   global `hits >= 2` floor is still satisfied → the per-file floor FAILS: *"matched 0 site(s) in
   exec_strategy_windows/launch.rs (floor 1)"*. This is the exact case the old doc claimed was
   covered.
4. Inject an unconditional event-log mention whose only nearby arm is a **comment** → FAILS with
   `Nearest arm found: ""`. The old raw-line walk-back accepted it.

**Two-direction:** test-only. Denies more (three previously-passing shapes now fail the build);
exposes nothing — no runtime path, output, file or wire format touched.

---

### WR-03 + WR-02 + WR-04 + WR-07: every mirror of the contract record

**Files:** `main.rs`, `crates/nono/src/error.rs`, `tests/layer_registry_selfcheck.rs`,
`proj/SPEC-windows-fail-direction-contract.md` · **Commit:** `84e48ffe`

One commit because these are one defect wearing four hats, and each gate depends on the others'
content — they cannot be landed green in isolation.

**WR-03.** The sibling CR-06 assertion was still the narrow-needle shape, 20 lines above the
function WR-06 fixed: the needle was the command + backtick + a **sentence-final period**, so it
rejected exactly one historical sentence ending while claiming the class "must never be prescribed
as the remedy". It now matches the CLASS (the command substring) and excludes the one legitimate
mention by requiring its NEGATION.

**WR-02.** The sweep for that rule covered `crates/` but not `proj/`. The SPEC's WR-17 ledger row
still recorded, in the present tense, that `MandatoryIntegrityLabel` "gets the real remedy
(`icacls <path> /setintegritylevel Medium`)" — a command `main.rs` carries a regression assertion
against — and that other layers "point at the Windows Application event log", a channel both
`main.rs` and `output.rs` assert is named only as a negation. Row rewritten and marked SUPERSEDED;
RF-13's first sentence corrected for CR-03. Two new gates scan the SPEC **with the same helper**
the rendered-string assertions use, so the mirrors are structurally prevented from diverging.

> The bounded-window bug in that helper (found by perturbation, described at the top of this
> report) is the concrete reason to distrust "both mirrors now assert the class" claims that were
> not perturbed.

**WR-04.** A blank line between the CR-02 and WR-10 rows terminated the D-15 table in GFM, so
WR-10's row rendered as a paragraph of pipe characters — while WR-10's entire skip justification
was that it was "recorded in the SPEC's table". Blank line deleted, and
`every_open_marker_in_code_has_a_ledger_row` makes it structural: **discovery-based** (finds every
`XX-NN OPEN` marker, names none), **GFM-faithful** (rows collected only while contiguous with the
header + delimiter pair, exactly as a renderer parses them), and **collision-aware** — finding IDs
REPEAT across iterations, so the matching row must also be marked OPEN or the Iteration-5 WR-14
row would satisfy the Iteration-6 WR-14 marker.

**WR-07.** The `WR-14 OPEN` record stated the wrong reason. Verified against source:
`probe_in_job` is `pub` behind `pub mod attestation`; both production callers discard the `Err`
(`attestation.rs::classify_probe_outcome` → `Unconfirmed`; the daemon's
`matches!(probe_in_job(process, job), Ok(true))`). So the null handle is not what makes the case
unreachable, and the case **IS** reachable today for FFI/Python/TypeScript embedders, which see
the mis-targeted `ClearStaleLayerResidue { layer: "JobObjectContainment" }` in a shipped build.
Record corrected, given the ledger row it never had, and pinned by a gate that also asserts both
swallow sites still swallow — so if either call site stops discarding, the record's own claim
fails the build.

**PERTURBATIONS (each fails only with the fix in place):**
- Reword the remediation to `` `icacls <path> /setintegritylevel Medium` to clear this. `` — a
  phrasing the OLD needle permitted → FAILS.
- Restore the SPEC's prescriptive WR-17 text → **both** SPEC gates FAIL.
- Re-insert the table-terminating blank line → the ledger gate FAILS naming **both** open markers.
- Delete only the Iteration-6 WR-14 row, leaving the Iteration-5 WR-14 row as a decoy → the ledger
  gate still FAILS on WR-14.
- Restore "requires a null job handle" → the WR-07 record gate FAILS.

**Two-direction:** no runtime behaviour changes. `error.rs` is comment-only (the `remediation()`
match arm is untouched); `main.rs`'s change is inside `#[cfg(test)]`; the SPEC is a document.
Nothing new is exposed, nothing new is denied at runtime — only the build denies more.

---

### WR-05: stale line citations across the registry surface

**Files:** `layer_registry.rs`, `tests/layer_registry_meta_test.rs`,
`tests/layer_force_unavailable.rs`, `tests/layer_registry_selfcheck.rs` · **Commit:** `06be2ae6`

31 raw `file.rs:<line>` citations, none inside a `call_sites` array and therefore outside every
gate. Every one I re-resolved was wrong, matching the reviewer's sample:

| Citation | Claimed | Actually |
|---|---|---|
| `network.rs:1500` | `select_network_backend` | a `Setup` error string; real fn `:1618` |
| `launch.rs:2236` | `verify_broker_authenticode` | real fn `:2666` |
| `launch.rs:2190` | `is_dev_build_layout` | real fn `:2620` |
| `launch.rs:375` | containment assign | real fn `:404` |
| `launch.rs:1620` | the `RestrictedToken` seam | that seam is in `restricted_token.rs` |
| `mod.rs:449/462/486` | guard construction order | doc/comment lines |

All 31 converted to `file.rs::Symbol` form, each resolved by reading the definition rather than
trusting the prose. Two gates close the class, since narrowing is the fail-OPEN direction:

- `no_line_number_citations_remain_in_the_registry_surface` rejects the raw form outright. It
  carries a **detector self-test** (must find both the single-line and range forms; must NOT flag
  `launch.rs::verify_broker_authenticode` or a path-qualified symbol form), so a broken matcher
  cannot report a clean surface.
- `every_registry_surface_symbol_citation_resolves` content-verifies every `file.rs::Symbol`
  anywhere in those files — the rule the SPEC already gets — with a `checked >= 20` floor.

**PERTURBATIONS:** revert one citation to `launch.rs:2134-2137` → the raw-form gate FAILS; rename
a cited symbol → the resolution gate FAILS naming file and symbol.

**Two-direction:** comments, doc comments and test code only. No runtime path, output, file or
wire format changes.

---

### WR-06: the literal extractor silently dropped char-literal lines

**File:** `crates/nono-cli/src/output.rs` · **Commit:** `60352cf9`

`string_literals` documented itself as needing no char-literal handling "because the files it is
pointed at contain neither". `output.rs` is one of those files and contains `else if c == '"' {`
twice — inside `string_literals` itself. Char literals are now consumed with a **lookahead**: a
blind `'` opener would consume from a lifetime (`&'a str`) to the next `'` anywhere on the line,
trading one false negative for a worse one. Unterminated (`\`-continued) literals are still not
pushed, deliberately, so this does not quietly alter what the gate covers. Adds a **per-file
floor** (25) alongside the global 500 — a global floor cannot notice one file going quiet, which
is exactly how this hid.

**PERTURBATION (the decisive one):** injected
`let _wr06_probe = if 'x' == '"' { "collapsed    run here" } else { "" };` — a real collapsed run
on a line that also carries char literals. With the fixed extractor the gate **FAILS**. With the
extractor reverted to the naive version and the **same probe still present**, the gate **PASSES**.
The false negative is demonstrated, not argued. The extractor self-test also FAILS under the naive
version.

**Two-direction:** test-only. Denies more; exposes nothing.

---

### (Not a review finding) Stale warm-path record left by WR-04

**Files:** `output.rs`, SPEC · **Commit:** `860d4772`

Found while auditing the marker surface. WR-04 changed the warm dedup check from `path.exists()`
to `read_to_string` + content comparison, and CR-01 made that content a digest — but three records
still described it as a bare stat: the latency test's doc, the figure it prints, and the SPEC's
D-24 latency table row. The SPEC also carried **74.8µs** as the warm cost, measured before the
content check existed; replaced with the mechanism plus a re-measured range (1.2–20.6ms, this
host, 2026-08-14), with the superseded figure called out rather than quietly deleted.

Exactly the class this review keeps finding: a record that outlived the code it describes, in a
document an operator reads as a contract.

---

## Verification

All gates run **at final HEAD** (`860d4772`).

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | **PASS** |
| Windows `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | **PASS** (exit 0) |
| `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** (exit 0), 15m07s |
| `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) | **PASS** (exit 0) |
| `cargo test -p nono-sandbox-cli --bin nono` | 1657 passed, 14 failed |
| `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` | 18 passed |
| `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` | 10 passed |
| `cargo test --workspace` (at `f55bd174`) | 1657 passed, 14 failed |

Both cross-target gates were run **locally and to completion** — no PARTIAL→CI fallback. This was
required: the change set touches `exec_strategy_windows/` (`#[path]`-mapped to `exec_strategy`)
and `crates/nono/src/error.rs`, which carries `#[cfg(target_os = "linux")]` blocks. They were run
twice: once at `f55bd174` and again at final HEAD after the docs-only commit.

### Regression baseline

The 14 failures are the known Windows-host baseline, **identical by name** to those recorded at
the phase base `334530af` in the round-2 report (`config::tests::*` HOME/USERPROFILE env races,
`protected_paths::tests::*`, `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`,
`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`,
`exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`,
`exec_strategy::launch::broker_dispatch_tests::broker_launch_assigns_child_to_job_object`,
`exec_strategy::launch::write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file`).

| | Passed | Failed |
|---|---|---|
| Phase base `334530af` (recorded round 2) | 1641 | 14 |
| Round-2 HEAD (recorded round 2) | 1646 | 14 |
| This branch | **1657** | **14** |

**Zero regressions; +11 tests over round 2.** The one intermediate 15-failure run is documented
under CR-01 and was closed, not explained away.

---

## Recommend human confirmation

**CR-01 is the only runtime-behaviour change in this round.** Everything else is tests, comments,
assertions or documents. Two things deserve a human read:

1. **The disclosure argument.** The claim is that the marker's content adds *zero* marginal
   disclosure because it is a deterministic function of the same input that already determines the
   filename. That argument depends on the filename remaining a digest of `dedup_key` — if a future
   change makes the filename opaque while leaving the content derived, the content becomes the
   weaker link. Nothing currently gates that coupling.
2. **Unforgeability remains open, by structure not by oversight.** Documented in
   `print_attestation_downgrade_banner`'s doc. If the operator wants a tamper-proof
   "already announced" record, it needs a design decision (hardening the sessions subtree — which
   only helps the Low-IL arms — or moving dedup state off the filesystem). I did not take that
   decision.

---

## Carry-forward (unchanged from round 2, plus one)

1. **CR-02 defect 1** — operator decision: does the daemon arm consume the registry? (Options A/B
   costed in the SPEC ledger.)
2. **WR-12 + WR-14 are one change.** Landing the machine-policy plumbing makes WR-14's
   mis-targeted remediation reachable. Both markers now say so, and the SPEC has a WR-14 row.
3. **WR-10** — operator decision: accept the lockstep two-binary rename, or keep one `LayerId`
   covering two kernel objects with the divergence documented.
4. **NEW — WR-14 is reachable for embedders today.** Not only "when WR-12 lands":
   `nono::attestation::probe_in_job` is public API, and an embedder calling it with a null job
   handle receives the mis-targeted remediation now. This raises WR-14's priority relative to the
   round-2 record.

---

## Commits

| # | Hash | Finding |
|---|---|---|
| 1 | `15215d37` | CR-01 |
| 2 | `61e4ea40` | WR-01 |
| 3 | `60352cf9` | WR-06 |
| 4 | `84e48ffe` | WR-03 + WR-02 + WR-04 + WR-07 |
| 5 | `06be2ae6` | WR-05 |
| 6 | `f55bd174` | CR-01 (test-contention repair) |
| 7 | `860d4772` | stale warm-path record (not a review finding) |

---

_Fixed: 2026-08-14_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2 (round 3)_
