---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-15T05:20:00Z
depth: standard
round: 8
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
  critical: 1
  warning: 4
  info: 0
  total: 5
status: issues_found
---

# Phase 117: Code Review Report — Round 8

**Reviewed:** 2026-08-15T05:20:00Z
**Depth:** standard (round 8, auditing `88a58936..f18fafe2` at HEAD `548d2113`)
**Files Reviewed:** 37
**Status:** issues_found

## Summary

**Round 7's substantive fixes are real and I proved every one of them by perturbation rather
than by reading. Round 7 also shipped a test that fails, right now, in this repository.**

Every claim in `117-REVIEW-FIX.md` that I could discriminate, I discriminated:

- the rewritten three-valued `is_cfg_test_attr` is correct on all 45 predicates I threw at it,
  including the ones the fixer singled out, and fails in the safe direction on every malformed
  input I could construct;
- `unterminated_block_comment` is reachable, fires on a real fixture, and `assert_split_is_correct`
  panics on it (`#[should_panic]` proof runs);
- round 6's perturbation table reproduces field-for-field (baseline `regions=3 skipped=1728
  sites=[1466]`, P1 identical, P2/P3 FAIL);
- WR-04's fail-closed enumeration works — I applied the decisive nested-unreadable-path
  perturbation to the real tree and the D-28 gate failed naming the subtree while
  `assert_eq!(files.len(), 1)` sailed past it, exactly as predicted;
- WR-07's real-vocabulary sweep is genuinely discriminating — the 103-character truncation
  perturbation fails the real sweep and leaves the synthetic one green;
- the two CI judgement calls are as narrow as claimed (`docs/architecture/` is 7 of 77 Markdown
  files under `docs/`; `.planning/PROJECT.md` is a quoted `==` literal, 1 of 910);
- the corrected 16-vs-14 workspace baseline is right, and I can name the mechanism.

But **`every_markdown_file_gated_by_a_test_runs_the_code_jobs` FAILS at HEAD** — 19 passed, 1
failed, reproduced twice on an otherwise clean working tree. The gate WR-05 replaced resolved
Markdown basenames against three named trees; its own comment said *"Explicit rather than a walk:
a walk would reach `.planning/` and `target/`"*. Round 7 deleted that comment and shipped the
walk, which now descends into every gitignored copy of the repository under the workspace root
and demands that `ci.yml` force-include paths that do not exist in the repository. The fix report
records this target as **20 passed**; that measurement was taken in an isolated temp worktree
(`Temp\sv-117-reviewfix-*`) where the defect structurally cannot appear. That is CR-01, and it is
the round's own signature failure — a guard whose scope was widened past the thing it is
guarding — committed by the fix that widened it.

Three further findings are in `cfg_test_regions.rs`, still the single point of failure for every
gate in the phase. Two of them (WR-01, WR-02) are **live in the scanned files today**, not
latent: `#[cfg(test)]`-gated items that are not inline modules land in the production half, and
a block comment opened mid-line has its continuation lines kept as production text. Both are
invisible to all four checks in `assert_split_is_correct`. WR-03 shows the new marker detector is
not "strictly wider" than the parser, as its doc asserts.

### What I ran, and how

I reproduced round 6's method rather than inheriting its results. `crates/nono-cli/src/cfg_test_regions.rs`
was compiled **verbatim** via `#[path]` into a standalone driver, so the code under test is
byte-identical to what ships, and run against the real scanned files and against synthetic
fixtures. Real-tree perturbations were applied with a script, run through `cargo test`, and
reverted with `git checkout --`; `git status --porcelain` at the end of this review shows only the
pre-existing untracked `117-REVIEW.round6.md`.

#### Priority 1 — `is_cfg_test_attr`, evaluated directly (45 inputs)

| Input | Result | Verdict |
|---|---|---|
| `#[cfg(test)]` | test gate | correct |
| `#[cfg(all(test, target_os = "windows"))]` | test gate | correct |
| `#[cfg(not(test))]` | not | correct |
| `#[cfg(any(test, feature = "x"))]` | **not** | correct — round-6 WR-01 closed |
| `#[cfg(any(all(test,unix), all(test,windows)))]` | **test gate** | correct — the fixer's claim holds |
| `#[cfg(all(not(test), unix))]` | not | correct |
| `#[cfg(feature = "test")]`, `#[cfg(all(feature = "test", unix))]`, `#[cfg(target_os = "test")]` | not | correct — the string literal's content is never inspected |
| `#[cfg(not(not(test)))]` / `#[cfg(not(not(not(test))))]` | test gate / not | correct |
| `#[cfg(all(all(all(all(test)))))]`, `#[cfg(any(any(any(any(test)))))]` | test gate | correct |
| 40-deep `all(`-nest around `test` | **not** | correct — `MAX_CFG_DEPTH` trips, fails toward the LOUD direction |
| `#[cfg(all(test)`, `#[cfg(test`, `#[cfg(`, `#[cfg()]`, `#[cfg(mystery(test))]`, `#[cfg(not(test, unix))]`, `#[cfg(all(test, feature = "abc))]` (unterminated string) | not | correct — every malformed shape fails to "keep as production" |
| `#[cfg(feature = "a)b")]`, `#[cfg(all(test, feature = "a)b"))]` | not / test gate | correct — paren-matching skips string literals |
| `#[cfg(all(test, feature = "a\"b"))]` | test gate | correct — escapes honoured |
| `#[cfg(all( test , unix ))]`, `#[cfg( test )]` | test gate | correct — whitespace-compacted first |
| `#[cfg_attr(test, derive(Debug))]` | not | correct |

`#[cfg(any())]` classifies as a test gate. That is **not** a defect: `any()` is false in every
build, so the gated item is never compiled at all and skipping it hides nothing from any gate.

**Fail direction on the unknown is right in every case I could construct.** `None` from the
evaluator, trailing text the grammar did not consume, and depth overrun all yield `false`, so the
body stays in the production half where a leaked `#[test]` is loud. I could not find an input that
made a production module disappear silently.

#### Priority 2 — the comment rule

`unterminated_block_comment` is reachable and loud: fixture `/* opened and never closed` →
`unterminated_block_comment=Some(4)`, and `assert_split_is_correct` panics with `WR-02` (the
`#[should_panic]` proof runs and passes). Nested `/* a /* b */ still */` is tracked by depth.
`}` inside a block comment inside a region no longer closes it early. `"*/"` inside a string on a
code line does not underflow the depth counter. `"/* not a comment"` at the start of a line is
(correctly, per the documented limit) treated as a comment — a false drop, in the loud direction
for the daemon gate.

The one shape that is wrong is a block comment opened **mid-line** (WR-02 below), and it is wrong
in the other direction.

#### Priority 3 — round 6's perturbation table, re-derived

Compiled shipped classifier, real `agent_daemon/launch.rs`:

| # | Perturbation | Scan | Emulated gate |
|---|---|---|---|
| baseline | none | `regions=3 skipped=1728 unclosed=[] unterm=None leaked=[] sites=[1466]` | PASSES |
| P1 | blank line before `mod attestation_gate_tests {` | **identical in every field** | PASSES |
| P2 | P1 + rename production `layer = "DaclAncestorTraverse"` → `DaclAncestorWalk` | `regions=3 skipped=1728 sites=[]` | **FAILS** |
| P3 | rename alone | `regions=3 skipped=1728 sites=[]` | **FAILS** |
| P4 (mine) | non-`*`-aligned block comment between the cfg attr and `mod` | `regions=3 skipped=1728 leaked=[]` | PASSES — round-7 WR-02 fix confirmed on the real file |

All four `#[should_panic]` proofs pass live (`WR-02`, `WR-03`, `WR-01/WR-02`, `non-vacuity`), and
all 25 `cfg_test_regions` unit tests pass.

#### Priority 4 — WR-04 and the class sweep

The decisive perturbation, run for real: a second `collect_files` against
`session_dir.join("nested-that-does-not-exist")`. `assert_eq!(files.len(), 1)` **still passed**
(confirming it is structurally blind), and the gate then **FAILED**:

```
CR-01/D-28: 1 violation(s) ... nested-that-does-not-exist: UNREADABLE DIR
(The system cannot find the path specified. (os error 3)) — this guard cannot prove the
subtree below it does not name a layer ...
```

Reverted; tree clean.

**On the unresolved item, `crates/nono/src/sandbox/windows.rs:2844` — leaving it is defensible,
and I will say why rather than repeat "not established".** `resolved_program.file_name()
.and_then(to_str).unwrap_or_default()` can only produce `""` when the path has no final component
or the name is not UTF-8. Neither case can equal any arm of the `match` (`"powershell.exe"`,
`"cmd.exe"`, …), so the fallback cannot cause a *listed* interpreter to skip its argument
validator: to be skipped, the name would have to match one of the literals, which requires it to
be valid UTF-8 in the first place. `validate_absolute_path_args` also runs unconditionally before
the match, so the baseline check is not bypassed. The residual weakness in that function is
name-based dispatch generally (8.3 short names, copies of `powershell.exe` under another name) —
pre-existing, out of this phase's change set, and unaffected by the `unwrap_or_default`. **No
finding; the fixer's decision to leave it stands.**

#### Priority 6 — the CI classifier

Discovery **is** genuinely dynamic (`collect_by_extension(manifest_dir()/src, "rs", ..)` +
`/tests`, `rust_sources.len() >= 100` floor) — and that is precisely what breaks it on the
resolution side (CR-01). Both judgement calls are as narrow as claimed:

- `(^docs/architecture/)` covers **7** files; `docs/` holds **77** `.md`/`.mdx`. Not `^docs/` in
  disguise. `force_include_covers` walks ancestors but the extracted region deliberately starts
  after the exclusion test's `]]`, so the exclusion list's own `(^docs/)` cannot satisfy it — I
  read `ci_force_include_region` and confirmed the cut.
- `[[ "${file}" == ".planning/PROJECT.md" ]]` — RHS is quoted, so bash compares literally, not as
  a glob. One file out of **910** under `.planning/`. Not `^\.planning/` in disguise.

#### Priority 7 — regressions

| Fix | Evidence at HEAD `548d2113` | Verdict |
|---|---|---|
| WR-02 (FFI) | `types.rs:218 LayerAttestationFailed = 15`, `:244` conversion arm, `:267` round-trip test; `nono.h:138`; `lib.rs:211` `NonoError::LayerAttestationFailed` arm | **Intact** |
| WR-08 (per-file literal floor) | `cargo test --bin nono` green on the floor gate; counts unchanged | **Intact** |
| WR-11 (reserved device names) | `output.rs:471 RESERVED`, `:482 eq_ignore_ascii_case` rejection | **Intact** |
| CR-03 / RF-13 | `machine_policy.rs:197 RequiredLayersPolicy`, `:275 required_layers`, `:683` reader doc | **Intact** |
| WR-16 (`log_target_is_private`) | `cli_bootstrap.rs:101` + fail-secure early returns | **Intact** |
| CR-01 digest marker | `output.rs` writes only `attestation_downgrade_marker_content`; now additionally bound by the real-vocabulary sweep, which I perturbed and watched fail | **Intact and strengthened** |
| round-5 `assert_split_is_correct` propagation | three call sites (`layer_registry.rs:1469`, `output.rs:2443`, `output.rs:2508`), one implementation, no consumer writes its own floor | **Intact** |
| CR-02 / WR-12 rationales (not re-litigated) | `layer_registry.rs:1410-1423` SCOPE paragraph still says "no daemon-expected row is unmentioned"; `error.rs:506 WR-14 OPEN` and `layer_registry.rs:925 WR-10 OPEN` both present with OPEN ledger rows (`every_open_marker_in_code_has_a_ledger_row` passes) | **Records still match the code** |
| Standing rules on the round-7 delta | no `.unwrap()`/`.expect()` in `cfg_test_regions.rs`; the other two production files' hunks are test-module and doc only; `cargo fmt --all -- --check` exit 0 | **Clean** |

`cargo test -p nono-sandbox-cli --bin nono`: **1680 passed, 12 failed** — every failure in the
recorded Windows-host baseline list (6 `config::tests` env races, 3 `protected_paths::tests`,
`profile_cmd`, `audit_session`, `labels_guard`). The two broker tests from the recorded list
happened not to fail this run; no phase-117 gate failed. `layer_registry_meta_test` 10/10,
`layer_force_unavailable` 0 (platform-filtered).

#### The 16-vs-14 baseline correction — verified, with the mechanism

`audit_attestation::audit_verify_reports_signed_attestation_with_pinned_public_key` and
`::rollback_signed_session_verifies_from_audit_dir_bundle` fail at HEAD with
`nono: Command execution failed: /bin/pwd: cannot find binary path`. The test hardcodes
`"/bin/pwd"` (`crates/nono-cli/tests/audit_attestation.rs:147` and `:209`) — a Unix path, on a
Windows host. No commit in `f9ee603^..HEAD` touches that file. **The correction is right: this
host's workspace baseline is 16, not 14, and the two extras are structurally host-related, not a
round-7 regression.**

---

## Critical Issues

### CR-01: `every_markdown_file_gated_by_a_test_runs_the_code_jobs` fails at HEAD — the discovery walk resolves reads against gitignored copies of the repository

**File:** `crates/nono-cli/tests/layer_registry_selfcheck.rs:1732-1764` (walk at `:1793-1824`,
fragment extraction at `:1693-1717`)

**Issue:** Run at HEAD `548d2113` on an otherwise clean tree (`git status --porcelain` shows only
the untracked round-6 review), twice, with no perturbation applied:

```
test result: FAILED. 19 passed; 1 failed

WR-05/WR-07: 10 Markdown file(s) are read and asserted on by the crate tree but are NOT
force-included by `.github/workflows/ci.yml`'s `changes` classifier ...
  .claude/worktrees/agent-a0e20603/crates/nono-cli/data/profile-authoring-guide.md (read at src/config/embedded.rs:39)
  ... 7 more worktree copies ...
  .claude/worktrees/agent-ae13967c/.planning/PROJECT.md (read at tests/adr_aipc_unix_futures.rs:118)
  .gsd/PROJECT.md (read at tests/adr_aipc_unix_futures.rs:118)
```

Three compounding causes, all introduced by `a8211eb4`:

1. **The resolution walk is over the filesystem, not the repository.**
   `collect_by_extension(&workspace_root(), "md", ..)` (`:1733`) prunes only
   `target | node_modules | dist | .git` (`:1816`). `.claude/` and `.gsd/` are **gitignored**
   (`.gitignore:13`, `.gitignore:68`) and both are present on this host — `.claude/worktrees/`
   holds 16 stale agent worktrees, each a full repository copy, all dated April–June 2026, i.e.
   they predate the phase. Every copy of a guarded `.md` becomes a candidate.
2. **Resolution is by BASENAME**, discarding path information the read site already spells out.
   `adr_aipc_unix_futures.rs:113-118` builds the path as
   `manifest_dir()/".."/".."/".planning"/"PROJECT.md"` — the directory is right there, in the same
   expression — but `:1712` reduces the fragment to `PROJECT.md` and then asks the whole workspace
   who might mean it.
3. **Ambiguity is resolved fail-CLOSED over that candidate set** (`:1756-1763`, "EVERY location
   the basename could denote must be force-included"). Since the candidates are paths that do not
   exist in the repository, no `ci.yml` clause can ever satisfy the assertion. The failure is
   unfixable from the workflow side.

The code this replaced got it right and said so. From `88a58936`:

```rust
// Candidate trees to resolve a basename in. Explicit rather than a walk: a
// walk would reach `.planning/` and `target/`, and the question being
// asked is only "which top-level tree does this contract document live in".
let candidate_trees = ["proj", "docs", "."];
```

That comment was deleted and the walk it warned against was shipped.

**Why the fix report records the opposite.** `117-REVIEW-FIX.md` verifies this target as
"**20 passed** (was 19)". The environment shows the round-7 work was done in
`C:\Users\OMack\AppData\Local\Temp\sv-117-reviewfix-*` worktrees — clean checkouts with no
`.claude/` or `.gsd/`, where a filesystem walk and a repository walk coincide. The gate passes
there and fails here, deterministically, and nothing in the gate makes that dependence visible.
A guard that only holds when the developer's workspace contains nothing but tracked files is not
a guard; it is the same "green because of where it was measured" shape this phase keeps
producing.

On a fresh CI clone it would pass, so this is a broken developer/`make ci` build rather than a
broken pipeline — but it is deterministic, it is in the phase's own test surface, and it is
recorded as passing.

**Fix:** ask git what the repository contains, and use the path the read site already gives you.

```rust
// Resolution set = TRACKED files, not whatever is on disk. `.claude/`,
// `.gsd/` and any sibling worktree are not part of this repository, and a
// path that cannot appear in a pull request's changed-file list can never be
// force-included by `ci.yml` — so treating one as a candidate makes the
// assertion unsatisfiable rather than strict.
let out = std::process::Command::new("git")
    .args(["-C", &workspace_root().display().to_string(), "ls-files", "*.md"])
    .output()
    .unwrap_or_else(|e| panic!("WR-05: `git ls-files` failed ({e}); this gate's \
         resolution set IS the repository, so it must FAIL rather than fall back to a \
         filesystem walk"));
assert!(out.status.success(), "WR-05: `git ls-files` exited {:?}", out.status);
let repo_markdown: Vec<String> = String::from_utf8_lossy(&out.stdout)
    .lines().map(str::to_string).collect();
```

and prefer the directory the read site names before falling back to the basename:

```rust
// `.join("..").join("..").join(".planning").join("PROJECT.md")` already says
// which file it means. Collect the quoted fragments on the line and use the
// longest trailing path suffix that matches a tracked file; only fall back to
// the bare basename when the site genuinely carries no directory.
```

Keep the fail-closed ambiguity rule — it is the right direction — but apply it to a candidate set
that can actually be satisfied.

---

## Warnings

### WR-01: a `#[cfg(test)]` attribute that gates something other than an inline `mod` puts test-only source into the PRODUCTION half — live in two of the four scanned files, and the code contradicts its own comment

**File:** `crates/nono-cli/src/cfg_test_regions.rs:753-799`

**Issue:** In the pending-attribute branch, an item that is not `mod foo {` falls through with
this comment:

```rust
// Fall through: the cfg-test attribute gated something that is not
// an inline module (a bare declaration, a `use`, a fn). It is not
// production text either way, but it opens no region.
```

The next statement is `out.push((idx, lines[idx]));` — it **is** treated as production text, and
so is every line of its body, because nothing tracks the item's extent. Measured against the real
files with the shipped classifier:

```
agent_daemon/launch.rs
  line 356 in PRODUCTION half? true   pub(crate) const WFP_CONTROL_PIPE_NAME_TESTABLE: &str = WFP_CONTROL_PIPE;
  line 363 in PRODUCTION half? true   pub(crate) fn profile_needs_network_scoping_testable(profile_name: &str) -> bool {
  line 364 in PRODUCTION half? true       profile_needs_network_scoping(profile_name)
  line 365 in PRODUCTION half? true   }
output.rs
  line 501 in PRODUCTION half? true   pub(crate) static SESSIONS_ROOT_TEST_LOCK: std::sync::Mutex<()> = ...
```

`agent_daemon/launch.rs` is the file the CR-02 daemon gate scans, and `output.rs` is one of the
two the WR-26 gate scans. Both gates ask "does the PRODUCTION half name X", so a `#[cfg(test)]`
helper naming a layer would satisfy the gate — which is verbatim the failure mode this module's
doc opens with ("a gate a test assertion can satisfy proves nothing"), one granularity down from
the module-level case it does close. Synthetic proof:

```
#[cfg(test)]
pub(crate) fn helper_testable() -> &'static str { "DaclAncestorTraverse" }   <- 3 lines, all kept
...
  regions=[(5, 8)] unclosed=[] unterm=None leaked=[]
  needle in production half? true
```

**All four checks in `assert_split_is_correct` are blind to it**: the region list is non-empty
(the real `mod tests` still closed), nothing is unclosed, no block comment is open, and a
`#[cfg(test)] fn` carries no `#[test]` to leak. This is the over-claim direction the module calls
"the worse one", and it is not latent — it is written in two of the four scanned files today.

**Fix:** either exclude the gated item, or state honestly that only `mod` gating is handled.
Excluding it is a brace-balance walk over the item, but the cheap and complete version is to treat
the fall-through as non-production:

```rust
// A `#[cfg(test)]` that gates a fn/const/static/use is test-only source. It
// is NOT production text — dropping it is the loud direction (a gate hunting
// a needle fails), keeping it is the silent one (a `_testable` helper can
// satisfy the gate). Drop the item, not just its attribute.
if t.ends_with('{') {
    // Skip to the matching closer at this item's own indent, exactly as the
    // `mod` arm does, and record it in `skipped_regions`.
} else {
    idx += 1;               // single-line item: `const X: &str = ...;`
    continue;
}
```

and add a fixture asserting a `#[cfg(test)] fn helper_testable` body does **not** reach
`scan.lines`, plus the two real sites (`agent_daemon/launch.rs`'s `_testable` wrappers,
`output.rs`'s `SESSIONS_ROOT_TEST_LOCK`) as regression anchors. Correct the fall-through comment
either way — right now it asserts the opposite of what the code does.

---

### WR-02: `line_has_code` only enters block-comment state from a line-leading `/*`, so a comment opened mid-line has its body kept as production text — and the "deliberate limit" paragraph names only the other direction

**File:** `crates/nono-cli/src/cfg_test_regions.rs:461-505` (doc at `:63-70` and `:452-460`)

**Issue:** The module doc states the rule as total — "a line contributes code unless everything on
it is whitespace or comment" — and the limit paragraph declares one cost: *"at the cost of one
accepted false drop: a comment-shaped line inside a multi-line raw string literal is treated as a
comment."* There is a second cost, in the opposite and more dangerous direction, and it is not
stated. Measured with the shipped classifier:

```
src:  let x = 1; /* NOTE: this mentions the
         Windows Application event log in prose only */
      #[cfg(test)]
      mod t { #[test] fn q() {} }

regions=[(3, 6)] unterm=None leaked=[]
  kept 1: let x = 1; /* NOTE: this mentions the
  kept 2: Windows Application event log in prose only */
```

Line 2 is pure comment prose, it names the WR-26 gate's exact needle, and it is in the production
half. Because `line_has_code` returns `true` before touching the depth counter whenever the
trimmed line does not *start* with `/*` (`:468-469`), the comment is never opened, so its
continuation lines are ordinary code to every consumer.

Direction depends on the consumer, and one of them is fail-open:

- `layer_registry.rs:1468`'s daemon gate asserts the production half **names** a layer. A
  `/* ... DaclAncestorTraverse ... */` comment opened mid-line satisfies it. **Fail-open** — the
  precise failure the module's own doc says comments are dropped to prevent ("a comment naming the
  thing a gate hunts for would otherwise satisfy the gate", `:39-41`).
- `output.rs:2443`'s WR-26 gate asserts every needle site sits in an `EventLog` arm, so the same
  shape produces a false failure. Loud, but wrong.

`unterminated_block_comment` does not help: the comment here is closed, just never opened.

No such shape exists in the four scanned files today — I checked all four for
`opens > closes && !starts_with("/*")` and found none — so this is latent, exactly like the
`not(test)` and parenthesised-`::test` gaps that were closed pre-emptively under this same
standard.

**Fix:** either handle a mid-line open, or state the limit in both directions. Handling it without
string lexing is not possible in general, so the honest minimum is:

```rust
/// # Deliberate limit, in BOTH directions
///
/// Block-comment state is only ENTERED from a line whose trimmed text starts
/// with `/*`. Two costs follow, and they point opposite ways:
///
/// 1. false DROP — a comment-shaped line inside a multi-line raw string is
///    treated as a comment (loud for a needle-must-be-present gate);
/// 2. false KEEP — a block comment opened MID-LINE (`let x = 1; /* ...`) has
///    its continuation lines classified as CODE, so comment prose reaches the
///    production half. That direction is fail-OPEN for `layer_registry.rs`'s
///    daemon gate, and no check in `assert_split_is_correct` can see it.
```

with a fixture pinning the measured behaviour, so the next widening of this rule starts from a
true statement. Closing (2) properly needs a `*/`-terminated scan from the mid-line `/*` with
string-literal awareness; if that is judged too much, the record above is the minimum.

---

### WR-03: the new marker detector is documented as "strictly wider" than the parser and is not — three demonstrated classes where the parser fires and the detector is blind

**File:** `crates/nono-cli/tests/layer_registry_selfcheck.rs:1853-1873` (self-test at
`:1878-1925`, consumer at `:1163-1183`)

**Issue:** The doc asserts a general property:

> It ignores adjacency, so **it stays strictly wider than** [`finding_id_before`]

I copied both functions verbatim into a driver and ran them side by side:

| line | detector | parser |
|---|---|---|
| `// WR-14 OPEN` | true | `["WR-14"]` |
| `// WR-14 (OPEN)` | true | `[]` — the intended catch, works |
| `// WR-14 OPENING the job handle` | **false** | `["WR-14"]` |
| `// CR-99 OPENS a handle` | **false** | `["CR-99"]` |
| `// WR-14 OPEN_EXISTING` | **false** | `["WR-14"]` |
| `dwCreationDisposition: OPEN_EXISTING,` | false | `[]` — the intended rejection, works |
| `// ABC-12 OPEN`, `// WR-14a OPEN`, `// 117-WR-14 OPEN` | false | `[]` |

The parser still triggers on the bare substring `" OPEN"` (`:1170`), which is the *same* over-wide
needle round 6's WR-06 raised — fixed in the detector, left in the parser. Consequences:

1. **The claim is false**, and the self-test only exercises it on one input
   (`"// WR-14 (OPEN)"`, `:1909-1917`). That is an instance asserted as a rule — the shape
   `117-REVIEW-FIX.md` says this round exists to stop ("calling an enumeration a rule is the thing
   this round exists to stop").
2. **A false ledger demand is reachable.** `// CR-99 OPENS a handle` parses to marker id `CR-99`,
   which then must have an `OPEN` row in the SPEC ledger or the build fails on a line that is not
   a deferral marker at all. Fail-closed, so a false failure rather than a hole — but it is
   diagnostically misleading in exactly the way WR-06 objected to.
3. Rows 8-10 are invisible to **both**, so a marker written `// ABC-12 OPEN` is unprotected with
   no blind-file report. Under-claim.

Only the two genuine markers exist today (`error.rs:506`, `layer_registry.rs:925`), so nothing is
broken now.

**Fix:** narrow the parser to the class it parses and keep the detector where it is — the
independence argument is right, the asymmetry just has to run the correct way:

```rust
// The parser's trigger must be the marker WORD, not the substring: `OPENING`
// and `OPEN_EXISTING` are not `OPEN`. Narrowing HERE preserves the detector's
// strict width, which is the property the blind-file check rests on.
for (pos, _) in line.match_indices(" OPEN") {
    let after = line.get(pos + " OPEN".len()..).unwrap_or("");
    if after.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
        continue;
    }
    ...
}
```

and turn the one-input width check into a rule over a table:

```rust
for line in [/* every row of the table above */] {
    assert!(
        line_carries_marker_tokens(line) || parse(line).is_empty(),
        "{line:?}: the parser recognised a marker the detector cannot see, so the \
         blind-file property has a hole exactly where it claims none"
    );
}
```

---

### WR-04: `leaked_test_attributes` is line-anchored, so a test attribute sharing a line with anything else is invisible to the ONLY under-claim check

**File:** `crates/nono-cli/src/cfg_test_regions.rs:562-568`, with `is_test_attr` at `:430-438`

**Issue:** Round 7 correctly widened `is_test_attr` from two exact literals to a class
(argument lists stripped, last path segment `test`, plus two named harness idents) — I verified all
ten `yes` and seven `no` cases pass. But the *consumer* still asks the question only of whole
lines:

```rust
.filter(|(_, l)| is_test_attr(l.trim()))
```

so the predicate's new width is only reachable when the attribute is alone on its line. Measured:

```
#[test] fn x() {}                    -> is_test_attr = false
#[cfg_attr(miri, ignore)] #[test]    -> is_test_attr = false
#[allow(unused)] // #[test]          -> is_test_attr = false
```

and end to end, the shape that matters:

```
src:  #[cfg(test)]
      mod t { #[test] fn q(){} }

regions=[] unclosed=[] unterm=None leaked=[]
kept: ["mod t { #[test] fn q(){} }"]
```

A one-line `#[cfg(test)] mod t { ... }` opens **no region** (`is_cfg_test_attr` matches the
attribute, `attribute_span` consumes the whole line, and the `mod` never reaches the region
opener), its body lands in the production half, and `leaked_test_attributes` is **empty** — the
one check that fires in the under-claim direction cannot see it. WR-03's fix was justified on
exactly this reasoning ("this is the width that decides whether a classifier gap is caught or
silent"); the same reasoning applies one level out, to where the width is consumed.

Not present in the four scanned files (I checked every line containing `#[test]` or `#[cfg(test)]`
that is not exactly that literal — two hits, both inside prose comments). Latent, at the same
standard under which `not(test)` and `#[tokio::test(..)]` were closed pre-emptively.

**Fix:** scan within the line rather than anchoring to it, and make the classifier's own
one-line-module blind spot a fixture:

```rust
pub fn leaked_test_attributes(&self) -> Vec<usize> {
    self.lines
        .iter()
        // A production line may carry an attribute alongside other tokens
        // (`#[cfg_attr(miri, ignore)] #[test]`, a single-line module body).
        // `leaked_test_attributes` is the only under-claim check there is, so
        // it must ask the question of every attribute ON the line, not only of
        // lines that ARE one.
        .filter(|(_, l)| split_attributes(l).iter().any(|a| is_test_attr(a)))
        .map(|(i, _)| *i)
        .collect()
}
```

plus a `#[should_panic]` proof on the one-line `#[cfg(test)] mod t { #[test] fn q(){} }` fixture.

---

## Not re-litigated

Per the brief I did not re-open CR-02's daemon-registry wiring or WR-12's fleet-control plumbing.
I checked both recorded rationales against the code and **both still match**:
`layer_registry.rs:1410-1423`'s SCOPE paragraph still claims only "no daemon-expected row is
unmentioned"; `error.rs:506`'s `WR-14 OPEN` and `layer_registry.rs:925`'s `WR-10 OPEN` both exist
and both have OPEN ledger rows (`every_open_marker_in_code_has_a_ledger_row` passes at HEAD).

---

_Reviewed: 2026-08-15T05:20:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard — round 8 (adversarial audit of `88a58936..f18fafe2`)_
