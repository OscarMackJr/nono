---
phase: 111-core-carry-resource-cli-verify-release-leapfrog
verified: 2026-08-05T12:00:00Z
status: human_needed
score: 5/5 must-haves verified (all 4 requirement IDs + D-07 safety invariant); 2 unresolved code-review findings require an explicit accept-or-fix decision
overrides_applied: 0
human_verification:
  - test: "Decide disposition of REVIEW.md's CR-01 (Critical): scripts/release-dry-run.ps1's PRE_PUBLISH_REGISTRY_BLOCKED regex matches the literal substring 'no matching package named' anywhere in cargo's dry-run output, unanchored to the specific expected missing package name (nono-sandbox/-proxy/-cli). Confirmed still present verbatim in the live file at HEAD (4ee90c7d) — no follow-up commit has touched scripts/release-dry-run.ps1 since the review landed."
    expected: "Either (a) scope the regex/parse the package name and assert membership in $PublishableCrates before treating it as expected, closing the false-negative risk before this gate is ever used ahead of a real crates.io publish, or (b) explicitly accept the residual risk (record an override / follow-up ticket) since D-07 keeps this milestone prepare-only and no live publish is imminent."
    why_human: "This is a security-relevant judgment call on an irreversible-publish gate (crates.io has no un-publish, only yank) — CLAUDE.md's Fail-Secure principle is implicated, but the actual current run-time behavior (2 correctly-identified pre-publish-blocked crates) is not wrong today; only its precision/robustness against a future genuine packaging defect is in question. A human must decide whether this blocks phase closure or is accepted as a tracked follow-up."
  - test: "Decide disposition of REVIEW.md's WR-01 (MAX_CRYPTO_THREADS raised to an approximate, load-scaling bound and applied uniformly to Linux+macOS despite a macOS-only rationale) and WR-02 (cli.rs's corrected --timeout doc comment describes the macOS-specific kill(-pgrp, SIGKILL)+setpgid(0,0) mechanism as if it also applies to Linux, when Linux actually uses cgroup.kill and neither calls kill(-pgrp) nor depends on setpgid)."
    expected: "Either split MAX_CRYPTO_THREADS per-platform (WR-01's suggested fix) and correct the --timeout doc comment to describe the two distinct kill mechanisms (WR-02's suggested fix), or explicitly accept both as non-blocking follow-ups given neither overstates the underlying security guarantee (both mechanisms are still kernel-enforced and effective)."
    why_human: "WR-02 is user-facing, security-relevant documentation (CORE-02's own scope) that is now factually imprecise in a different way than the defect it replaced — a human should confirm this level of imprecision is acceptable for this phase's CORE-02 exit bar, since the phase's own must-have language (D-06) demanded precision from reading all three enforcement backends live, not from assumption."
---

# Phase 111: Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog Verification Report

**Phase Goal:** The macOS/core carry and resource-CLI alignment land, the whole sync is proven
non-regressing under both cross-target clippy gates, and the tree leapfrogs to a prepare-only
`0.70.0`.
**Verified:** 2026-08-05T12:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | CORE-01: `~/.cache` is granted readwrite in `user_caches_macos`, `MAX_CRYPTO_THREADS == 12`, both backed by new by-value tests, only the one relevant upstream line ported | VERIFIED | `crates/nono-cli/data/policy.json:328-332` array is exactly `["~/Library/Caches","~/Library/Logs","~/.cache"]`; `crates/nono-cli/src/exec_strategy.rs:171` reads `const MAX_CRYPTO_THREADS: usize = 12;`; tests `test_user_caches_macos_includes_dot_cache` (`policy.rs:1743`) and `max_crypto_threads_raised_to_12` (`exec_strategy.rs:4384`) exist with correct assertions; commits `1d0c8eb5`/`74146367` each touch exactly the 2 files the plan named — no unrelated upstream `ca888108` file leaked in |
| 2 | CORE-02: stale "accepted with a warning pending cross-platform follow-up" claim removed from `cli.rs`/`flags.mdx`; ADR-111 settles ADAPT-not-adopt with reasoning; ledger carries the standing-divergence addendum | VERIFIED | `grep -rn "pending cross-platform follow-up" crates/nono-cli/src/cli.rs docs/cli/usage/flags.mdx` → 0 matches; `proj/ADR-111-resource-limits-boundary.md` exists (157 lines), mirrors ADR-108's Context/Decision/Consequences/References shape, contains a dedicated section explaining (not merely asserting) why ADR-86's audit/diagnostics carve-out does not extend to resource-limit enforcement, cites both upstream SHAs verbatim; `108-DIVERGENCE-LEDGER.md:1648-1681` carries `## Phase 111 Standing Divergence Addendum`, names both SHAs, cites the ADR by path, explicitly distinguishes "permanent divergence" from the tool-sandbox cluster's `DEFERRED->v3.7` treatment |
| 3 | VERIFY-01: D-01 still holds (no `pub mod resource;`, no `resource*` file); both cross-target clippy gates ran locally GREEN (no PARTIAL→CI); ADR-86 Windows carve-out unregressed; 24-name baseline reproduced honestly | VERIFIED | `grep -n "^pub mod" crates/nono/src/lib.rs` lists 17 modules, none `resource*`; `ls crates/nono/src \| grep -i resource` → empty; `111-04-VERIFICATION-NOTES.md` records both gates (`cross clippy` linux-gnu, `cargo-zigbuild clippy` apple-darwin, `SDKROOT` unset) exiting 0 with the explicit line "no PARTIAL→CI fallback invoked, no documented runner failure encountered"; `git diff c5c8c5fc..HEAD -- crates/nono-cli/src/exec_strategy_windows/` independently re-confirmed empty (0 lines) at current HEAD; 17-name failing set is a documented, honestly-diffed strict subset of the 24-name baseline, plus a transparent addendum recording a 3-name shared-fixture flake family (`profile_cmd.rs`) discovered by an independent orchestrator sweep, root-caused (not hand-waved), and confirmed untouched by any 108-111 commit |
| 4 | RLS-14: all 6 in-repo crates + both sibling repos report `0.70.0`; `sign-fixture` stays `0.1.0`; gate scripts assert against `0.70.0`/`0.69.0`; `nono-ts`'s path-dep pin corrected | VERIFIED | `cargo metadata` (live run): `nono-sandbox`/`nono-sandbox-cli`/`nono-sandbox-proxy`/`nono-shell-broker`/`nono-fltmgr-client`/`nono-ffi` all `0.70.0`; `sign-fixture` `0.1.0`; `../nono-py/Cargo.toml` + `pyproject.toml` both `0.70.0`; `../nono-ts/Cargo.toml` `version = "0.70.0"` AND path-dep `version = "0.70"` (corrected from `"0.66"`); `../nono-ts/package.json` top-level `0.70.0`; `scripts/gates/release-readiness.ps1:76-77` reads `$targetVersion='0.70.0'`/`$upstreamHighest='0.69.0'`; `grep "0.66" scripts/release-dry-run.ps1 scripts/gates/release-readiness.ps1` → 0 matches in both files |
| 5 | D-07 Safety Invariant: no tag pushed, no live publish executed anywhere in this phase | VERIFIED | `git tag --points-at HEAD` (this repo) → empty; the locally-present `v0.70.0`/`v0.7.0` tags point at unrelated upstream commits (`7bb07d1b`, "chore: release v0.70.0 (#1514)", NOT an ancestor of this repo's HEAD) — these predate this session and are not this fork's tags; `git ls-remote --tags origin` shows no `v0.70.0`/`v0.69.0`; `../nono-py` and `../nono-ts` both show `git tag --points-at HEAD` empty; `grep "cargo publish\|npm publish\|twine upload" scripts/release-dry-run.ps1` shows every invocation carries `--dry-run` / is `twine check` (never `twine upload`) |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/data/policy.json` | `user_caches_macos.allow.readwrite` includes `~/.cache` | VERIFIED | Confirmed live, array exact, order preserved |
| `crates/nono-cli/src/exec_strategy.rs` | `MAX_CRYPTO_THREADS == 12` + doc comment + test | VERIFIED | Confirmed live at line 171 + test at 4384 |
| `crates/nono-cli/src/policy.rs` | New Wave-0 by-value test | VERIFIED | `test_user_caches_macos_includes_dot_cache` at line 1743, asserts all 3 entries |
| `crates/nono-cli/src/cli.rs` | Corrected `--memory`/`--timeout`/`--max-processes` doc comments | VERIFIED (with WR-02 caveat) | Stale substring gone; `--memory` correctly states best-effort+EINVAL caveat (review confirms no overstatement); `--timeout` text is imprecise re: Linux vs macOS mechanism (WR-02, see human_verification) |
| `docs/cli/usage/flags.mdx` | Corrected Resource Limits section | VERIFIED | Stale substring gone (0 matches), `--cpu-percent` entry also corrected beyond plan's literal scope, matching acceptance criteria |
| `proj/ADR-111-resource-limits-boundary.md` | Standalone ADR settling CORE-02's boundary disposition | VERIFIED | Exists, 157 lines, all 5 required sections, both SHAs cited verbatim, explicit ADR-86 non-extension argument |
| `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` | Standing Divergence addendum | VERIFIED | Section present, cross-references ADR by path, distinguishes permanent vs. deferred |
| `.planning/phases/111.../111-04-VERIFICATION-NOTES.md` | Combined 108-111 fork-invariant record | VERIFIED | 380 lines, both gates' raw exit codes/output, D-01/ADR-86 grep-diffs, 24-name baseline table, both binding rebuilds, explicit supersession note re: 110-08, plus an honest orchestrator-observed-flake addendum |
| 6× `Cargo.toml` (workspace crates) + `Cargo.lock` | `0.70.0` | VERIFIED | `cargo metadata` confirms all 6; `sign-fixture` untouched at `0.1.0` |
| `scripts/gates/release-readiness.ps1`, `scripts/release-dry-run.ps1` | Assert `0.70.0`/`0.69.0`, no stale `0.66.x` | VERIFIED (with CR-01 caveat) | 0 stale-version matches; `release-dry-run.ps1`'s pre-publish-block detection was widened post-review-discovery but the resulting regex is unscoped/overly broad per REVIEW.md CR-01 (see human_verification) |
| `../nono-py/Cargo.toml`, `pyproject.toml` | `0.70.0` | VERIFIED | Confirmed live |
| `../nono-ts/Cargo.toml`, `package.json`, 4× `npm/*/package.json` | `0.70.0`, path-dep pin `"0.70"` | VERIFIED | Confirmed live, including the load-bearing path-dep correction |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `policy.json` | `policy.rs::load_embedded_policy` | embedded build.rs data load | WIRED | New test loads embedded policy and asserts on the group's contents |
| `exec_strategy.rs::MAX_CRYPTO_THREADS` | `ThreadingContext::CryptoExpected` arm | constant reference by name | WIRED | Confirmed at lines 918/942, unedited per plan (correct — no consumer-site edit needed) |
| `cli.rs` (corrected doc comments) | `launch_runtime.rs::ResourceLimits` doc comment | condensed-from relationship | WIRED | `--memory` text correctly condenses the source's best-effort/EINVAL caveat; `--timeout` text does NOT correctly condense the source's per-platform mechanism split (WR-02) |
| `ADR-111` | `ADR-86-library-boundary-convergence.md` | explicit non-extension argument | WIRED | Confirmed — dedicated section, cites ADR-86's own stated invariant |
| `108-DIVERGENCE-LEDGER.md` addendum | `ADR-111` | cross-reference by path | WIRED | Confirmed at ledger line 1662 |
| `111-04-VERIFICATION-NOTES.md` | `110-08-VERIFICATION-NOTES.md` | explicit supersession note | WIRED | Confirmed, with concrete reasoning (`has_port_rules()` defect that predated 110-08) |
| `../nono-ts/Cargo.toml` path-dep | `crates/nono/Cargo.toml` `[package] version` | Cargo path-dep version-requirement compatibility | WIRED | `"0.70"` caret-satisfies the actual `0.70.0` package; confirmed no resolution failure (napi build exit 0 per 111-04/111-06 evidence) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CORE-01 | 111-01 | macOS Seatbelt/core carry: `~/.cache` grant + `MAX_CRYPTO_THREADS`=12 | SATISFIED | See Truth #1 |
| CORE-02 | 111-02, 111-03 | Resource-limit CLI surface reconciled with fork's kernel enforcement | SATISFIED | See Truth #2; WR-02 is a residual accuracy gap, not a requirement failure |
| VERIFY-01 | 111-04 | Both cross-target clippy gates + `make ci` GREEN locally; fork-invariant pass | SATISFIED | See Truth #3 |
| RLS-14 | 111-05, 111-06 | 6 in-repo crates + 2 sibling repos leapfrogged to `0.70.0`, prepare-only gate GREEN | SATISFIED | See Truth #4/#5; CR-01 is a residual gate-robustness gap, not a requirement failure |

`.planning/REQUIREMENTS.md` traceability table (lines 170-173) confirms all 4 marked `Complete`, no orphaned requirement IDs found for Phase 111.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `scripts/release-dry-run.ps1` | 97-101 | Unscoped `-match 'no matching package named'` (REVIEW.md CR-01) | 🛑 Critical (per code review, unresolved at HEAD) | Would silently reclassify a genuine unrelated packaging defect as an expected non-blocking state on this irreversible-publish gate. Does not affect any of this phase's stated must-haves today (current 2 blocked crates are correctly the expected ones), but is a real robustness gap in security-relevant release tooling |
| `crates/nono-cli/src/exec_strategy.rs` | 164-171 | `MAX_CRYPTO_THREADS` raised to an approximate, load-scaling bound, applied uniformly to Linux+macOS (REVIEW.md WR-01) | ⚠️ Warning | Loosens the Linux fork-safety tripwire without a Linux-specific justification; not a phase-goal blocker |
| `crates/nono-cli/src/cli.rs` | 2804-2810 | `--timeout` doc comment conflates Linux `cgroup.kill` and macOS `kill(-pgrp,SIGKILL)`+`setpgid` mechanisms (REVIEW.md WR-02) | ⚠️ Warning | Factually imprecise but does not overstate the security guarantee; user-facing security-relevant text |
| `crates/nono-cli/src/exec_strategy.rs` | 4383-4386 | `max_crypto_threads_raised_to_12` is tautological (asserts a constant equals its own literal) (REVIEW.md IN-02) | ℹ️ Info | Documents intent, provides no behavioral regression coverage |
| `crates/nono-cli/data/policy.json` | 328-332 | `~/.cache` grant not subpath-scoped to the specific tools that need it (REVIEW.md IN-01) | ℹ️ Info | Mirrors the existing Linux `user_caches_linux` group's already-broad grant; not a regression |
| `docs/cli/usage/flags.mdx` | 1232-1264 | No `--timeout` subsection exists (pre-existing gap, REVIEW.md IN-03) | ℹ️ Info | Pre-existing, not introduced by this phase |

No `TBD`/`FIXME`/`XXX` unreferenced debt markers found in any file this phase modified.

### Human Verification Required

### 1. CR-01 disposition — unscoped pre-publish-block regex in `scripts/release-dry-run.ps1`

**Test:** Review `scripts/release-dry-run.ps1:97-101`'s `PRE_PUBLISH_REGISTRY_BLOCKED` detection, which now matches the literal substring `'no matching package named'` anywhere in `cargo publish --dry-run`'s output, with no anchor to the specific expected missing package name (`nono-sandbox`/`-proxy`/`-cli`).
**Expected:** A decision — either scope the match (code fix) before this gate is trusted ahead of any real crates.io publish, or explicitly accept the residual risk as a tracked follow-up given the milestone stays prepare-only.
**Why human:** Security-relevant judgment call on an irreversible-publish gate; the current live behavior is correct today, only future-robustness is in question — not mechanically resolvable by grep alone.

### 2. WR-01/WR-02 disposition — `MAX_CRYPTO_THREADS` platform-scoping and `--timeout` doc-comment accuracy

**Test:** Review `exec_strategy.rs:164-171` (uniform Linux+macOS application of a macOS-justified bound) and `cli.rs:2804-2810` (the `--timeout` doc comment's cross-platform mechanism conflation).
**Expected:** A decision — fix now (per REVIEW.md's suggested fixes) or accept as non-blocking follow-ups.
**Why human:** WR-02 is user-facing security documentation that CORE-02 specifically exists to make accurate; a human should confirm the current level of imprecision meets this phase's own bar.

### Gaps Summary

No must-have truth from any of the 6 plans FAILED. All 4 requirement IDs (CORE-01, CORE-02,
VERIFY-01, RLS-14) are independently confirmed against the live codebase — not merely against
SUMMARY.md claims — including the phase's own D-01/D-07/D-09/D-10 locked decisions. The
`111-04-VERIFICATION-NOTES.md` record is unusually rigorous: it re-ran both cross-target clippy
gates locally (no PARTIAL→CI substitution), diffed the pre-existing test-failure baseline
name-by-name rather than re-litigating it, and even recorded a second, independent
orchestrator-observed test sweep that surfaced 3 additional flaky names outside the documented
24-name baseline — which it then root-caused (a shared-fixture race in `profile_cmd.rs`, not a
regression) rather than hand-waving away.

The one open item is `111-REVIEW.md`, the phase's own code-review artifact (committed as the
literal HEAD commit, `4ee90c7d`), which found 1 Critical and 2 Warning-level issues that remain
unresolved in the tree as verified. Neither breaks a stated must-have truth as scoped by the
plans — the release gate genuinely is GREEN today, and the corrected help text genuinely does not
overstate any security guarantee — but a Critical finding sitting unaddressed in a completed
phase's own review, on a script that gates an irreversible crates.io publish, warrants an explicit
human accept-or-fix decision before this phase is considered fully closed out, per the escalation
pattern this verification is built to enforce.

---

*Verified: 2026-08-05T12:00:00Z*
*Verifier: Claude (gsd-verifier)*
