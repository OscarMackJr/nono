---
created: 2026-08-16T00:06:16.850Z
title: Three CI gates that outlived their scope — libdbus, the Phase 102 rename tail, and the floating toolchain
area: tooling
files:
  - crates/nono-cli/Cargo.toml:38,42,154
  - .github/workflows/ci.yml (cross-compile job; "Verify binary does not link libdbus (Linux)")
  - .github/workflows/phase-45-resl-native-host.yml:72,105
  - .github/workflows/phase-46-uat-backlog.yml:84,91,97,105,111,154,159,166,171
  - scripts/diagnose-macos-browser-open.sh:28
  - scripts/probe-claude-open-path.sh:33
  - scripts/sign-poc-local.ps1:171
  - CLAUDE.md (Coding Standards → cross-target verification)
---

## Problem

Surfaced by the 2026-08-15 CI triage — runs `31888431118` and `31896098661` on
`milestone/v2.13-carryforward-closeout`, **the first CI runs ever executed against Phases
115/116/117** (the branch had been 687 commits ahead of origin, whose tip predated the entire
milestone). Three findings, all diagnosed with evidence, none fixed.

They are grouped because they share one theme: **each is a gate that was correct when written and
silently stopped covering reality.** A target set that missed a toolchain axis; a package set that
outlived a rename; a portability assertion nobody had ever run. Fixing the three instances without
addressing the pattern leaves the pattern.

### Finding 1 — libdbus in the Linux release binary (the design decision)

Both Linux cross-compile gates are red, from **one cause, not two**:
- `aarch64-unknown-linux-gnu` builds (via `cross`), then fails an explicit gate:
  *"target/aarch64-unknown-linux-gnu/release/nono links libdbus — the release binary is not portable"*.
- `x86_64-unknown-linux-gnu` fails **earlier**, in `libdbus-sys`'s build script, because the
  cross-compile job installs only `pkg-config`, not `libdbus-1-dev`.

Provenance: `crates/nono-cli/Cargo.toml:38` `default = ["system-keyring"]` → `:42`
`system-keyring = ["dep:keyring", ...]` → `:154` on Linux `keyring = { version = "3", features =
["sync-secret-service"] }` → pulls `libdbus-sys`. **`libdbus-sys` has been in `Cargo.lock` since
Initial Commit** — longstanding, NOT drift from recent work. Never observed because CI had never run.

**SCOPING NOTE, VERIFIED — do not skip this.** The portability gate's condition is
`if: runner.os == 'Linux'`, so it runs on BOTH Linux targets. **Adding `libdbus-1-dev` is therefore
NOT a fix** — it would let x86_64 build and then fail the same gate aarch64 already fails. The
package's absence is plausibly a deliberate forcing function (a libdbus dependency fails the Linux
build immediately). Do not reach for "install the dev package".

**The real question is a product decision: what does nono promise for Linux credential storage?**
This has genuine security surface — it decides where Linux credentials live.

### Finding 2 — the Phase 102 rename tail

Phase 102 renamed packages to `nono-sandbox` / `nono-sandbox-cli` / `nono-sandbox-proxy`. Stale
pre-rename selectors survive. **Already surveyed and dispositioned — verify, do not re-survey:**

| Site | Disposition |
|---|---|
| `tests/run_integration_tests.sh:31` | **ALREADY FIXED** 2026-08-15, commit `b5d3b1c6`. Was breaking the Integration Tests job. |
| `.github/workflows/release.yml:521,535` | **DELIBERATE — LEAVE ALONE.** `publish-crates` is neutralized (`if: false`) with a comment stating Phase 105 (PUB-02) rewrites it with correct names plus dependency-ordered, index-visibility-polled publish logic. `scripts/verify-release-yml-publish-selectors.ps1` guards it. **Belongs to Phase 105, not here.** |
| `phase-45-resl-native-host.yml` (2), `phase-46-uat-backlog.yml` (9) | **LIVE STALE** — would fail if triggered. Decide per workflow: fix the selectors if still wanted, or retire the workflow explicitly rather than leaving one that cannot run. |
| `scripts/diagnose-macos-browser-open.sh:28`, `scripts/probe-claude-open-path.sh:33`, `scripts/sign-poc-local.ps1:171` | Stale advice in echo/error strings ("Build it first with: `cargo build -p nono-cli`"). Cosmetic, but user-facing exactly when a script has already failed. |

### Finding 3 — CI toolchain floats while the dev host trails

CI uses `dtolnay/rust-toolchain@stable`, which resolved to **1.97.0** on 2026-08-15. This dev host
is **1.95.0**. A clippy lint existing only in 1.97 (`useless_borrows_in_formatting`) failed CI on
all three hosts while local `cargo clippy` was clean — **local verification was structurally
incapable of catching it.** Three sites were fixed 2026-08-15 (commit `8397e5b2`), but the class
recurs on every clippy release.

CLAUDE.md's cross-target rule pins a TARGET SET, not a toolchain version, so it does not and cannot
cover this. The rule is currently silent on toolchain version, which is why this was a surprise
rather than a caught regression.

## Solution

Three decisions and one pattern fix. Sized as a design pass, not a patch.

**1. libdbus / Linux credential storage — DECIDE, then implement.** Candidate options, none
pre-selected: drop `system-keyring` from Linux default features; switch to a keyring v3 backend
that avoids libdbus (**verify what v3 actually ships** — do not assume a rustls-based
secret-service or linux-native variant exists before checking); or change what the portability gate
asserts, if the portability promise itself is what is wrong. Whichever is chosen, the outcome must
state plainly where Linux credentials live and what the release binary links.

**2. Rename tail — clear the live sites per the table above,** honouring the release.yml carve-out.

**3. Toolchain — pin CI to an explicit Rust version, or bump-and-hold local to match `stable`,**
or at minimum add a CI step that reports the resolved toolchain so drift is visible. Then say
which, in CLAUDE.md's verification section.

**4. THE PATTERN, and the reason these are one item.** Nothing mechanically prevents the next
rename leaving the same tail, or the next toolchain bump landing as a surprise. Consider a gate
asserting every `-p <name>` / `--package <name>` selector across `.github/`, `scripts/`, `tests/`
and `Makefile` resolves to a real workspace package per `cargo metadata`, with a documented
allowlist for the deliberately-neutralized release.yml lines. That converts a recurring manual
sweep into a build failure. **Whatever shape this takes, at least one success criterion should be
about the gates staying true — not merely about these three instances being closed.**

## Addendum — 2026-08-16, CI run 31916670497 (post-fix verdict)

The three mechanical fixes landed: `Clippy (windows-latest)`, `Clippy (macos-latest)` and
`Docs Checks` all flipped GREEN. Two new data points, both reinforcing this item rather than
changing it:

**3b — a THIRD toolchain-drift instance, and the worst-positioned one.**
`Clippy (ubuntu-latest)` still fails, on a *different* lint from the one fixed:
```
error: this block may be rewritten with the `?` operator
  --> crates/nono-cli/src/learn.rs:1385   (clippy::question_mark, rust-1.97.0)
```
It sits inside `fn extract_path_from_syscall` (`:1377`), gated `#[cfg(target_os = "linux")]` at
`:1376` — which is why windows/macos clippy pass: they never compile it. **`git log` shows that
code is from Initial Commit (2026-02-09), untouched since** — no commit skipped a cross-target
check, and CLAUDE.md's rule would not have fired anyway because nothing in this milestone touched
the file. It is newly-linted by 1.97.
**This is invisible to local verification on TWO independent axes at once** — wrong toolchain
(1.95 vs 1.97) AND wrong target (Linux-gated, so a Windows-host clippy never compiles it).
CLAUDE.md's rule addresses only the second. Strongest evidence yet for pinning the toolchain: fix
the lint if you like, but the lint is the symptom.

**NEW, adjacent — the Integration Tests suite has been failing invisibly.**
The `-p nono-sandbox-cli` fix (commit `b5d3b1c6`) worked: the `cannot specify features for packages
outside of workspace` error is gone and the script now actually executes. It reports **16 suites
passed, 5 FAILED** — e.g. `Trust CLI` fails 1 of 11 on a real assertion (`trust list --json`
verification of a `SKILLS.md` fixture). These are genuine content failures that were masked for as
long as the script was unrunnable. **Not yet triaged.** Same shape as the fail-open CI gate: fixing
the blocker revealed what it was hiding. Scope this separately — it is not part of the libdbus /
rename / toolchain trio, but it was found by the same sweep and has the same cause-of-invisibility.

**Also still red and NOT yet triaged at all** (out of scope for this item, recorded so they are not
mistaken for covered): `Test (ubuntu-latest)`, `Test (macos-latest)`, `Windows Build`,
`Windows Smoke`, `Windows Integration`, `Windows Regression`, `Windows Packaging`.

## Evidence already on disk — do not re-derive

- `.planning/debug/ci-windows-117-failures.md` — the sibling Windows triage (19 failures, 3 proven
  causes, since fixed and CI-verified: 22 failures → 3, the 3 being the pre-existing
  `protected_paths` baseline).
- CI runs `31888431118` and `31896098661`, readable via `gh run view --job <id> --log`.
- Two distinct local test baselines, both correct for their own invocation — do NOT reconcile them
  into one number: `--bin nono` is 1688 passed / 12 failed / 2 ignored (population 1702);
  `--bin nono --features layer-fault-injection` is 1694 / 12 / 2 (population 1708). The 6-test
  delta is the feature-gated seam tests.

## Constraints

- **v3.5 is open in parallel.** Never overwrite `.planning/REQUIREMENTS.md` or
  `.planning/ROADMAP.md` — append only. No SDK state/roadmap writers; they have previously deleted
  tracked milestone blocks on this project. `.planning/STATE.md` is hand-maintained.
- Commits need an explicit `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` trailer
  (`git commit -s` stamps the wrong name on this host).
