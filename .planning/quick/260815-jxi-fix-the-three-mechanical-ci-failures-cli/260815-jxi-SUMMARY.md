# Quick Task 260815-jxi: Fix Three Mechanical CI Failures Summary

Fixed the three mechanical failures from CI run 31888431118 — a 1.97-only clippy lint on
redundant borrows in `format!` args, a stale `-p nono-cli` package selector left behind by
the Phase 102 rename, and an undocumented `--allow-http2` run flag.

**Branch:** `milestone/v2.13-carryforward-closeout` (real tree, no worktree)
**Base:** `18405fc0c8e6bbdaf9d9ecaa6d578d705781d11f`
**Head:** `90b36fc0`

## Commits

| Commit | Type | Subject |
|--------|------|---------|
| `8397e5b2` | fix | drop redundant borrows in format! args |
| `b5d3b1c6` | fix | use the renamed nono-sandbox-cli package selector |
| `90b36fc0` | docs | document the --allow-http2 run flag |

Each commit carries the explicit DCO trailer `Signed-off-by: Oscar Mack Jr
<oscar.mack.jr@gmail.com>` (written by hand — `git commit -s` would have stamped
`oscarmackjr-twg`).

## Fix 1 — clippy `useless-borrows-in-formatting`

`crates/nono-proxy/src/server.rs:157` — `&*self.token` → `*self.token`. The deref is kept
(it unwraps `Zeroizing<String>`); only the redundant borrow is dropped, because `format!`
already takes its arguments by reference.

**Class coverage — two additional sites fixed.** A workspace-wide scan of every `&*` in
`*.rs` (51 occurrences) found exactly three in format-macro argument position; the other
48 are legitimate raw-pointer derefs in FFI/Windows code. The two extra sites are:

- `crates/nono-cli/src/proxy_command.rs:394`
- `crates/nono-cli/src/proxy_command.rs:396`

Both are `Zeroizing<String>` in `eprintln!` args — the identical shape. **Why CI reported
only one:** `crates/nono-cli/Cargo.toml:52` shows `nono-cli` depends on `nono-proxy`, so
under `-D warnings` clippy aborted at `nono-proxy` and never linted `nono-cli`. Had these
been left, the next CI run would have failed on the same lint one crate later. Fixing the
class was the point of the pass.

**Scope note / deviation from the verification checklist.** The instructions asked me to
"confirm the `nono-proxy` change is the ONLY production-code line touched." That is
**not** true as delivered: `proxy_command.rs` is also production code and I changed 2 lines
in it. This is a deliberate, flagged deviation — it follows directly from the same
section's instruction to "fix the CLASS, not just the reported line," and the checklist
bullet was written before the extra sites were known. Total production diff is 3 lines
across 2 files.

**Behavioural equivalence.** All three are pure expression changes. `&*token` and `*token`
both resolve to `<String as Display>::fmt`; `format_args!` borrows its arguments, so
`*token` is a place expression that is never moved. Rendered output is byte-identical, and
`Zeroizing` handling is unchanged. `cargo test -p nono-sandbox-proxy` is fully green
(307 + 6 passed, 0 failed).

**What local verification does and does not prove.** Local clippy is **1.95.0**
(`clippy 0.1.95 (59807616e1 2026-04-14)`); CI's `dtolnay/rust-toolchain@stable` resolves to
**1.97.0**. The lint does not exist in 1.95, so the local green run proves only that the
change **does not regress** on 1.95. It does **not** prove the 1.97 lint is satisfied —
only the next CI run can establish that. `rustup toolchain list` shows only
`stable-x86_64-pc-windows-msvc` and `stable-x86_64-unknown-linux-gnu`, both at 1.95; no
newer toolchain was available and, per instructions, none was installed for this task.

## Fix 2 — stale package selector

`tests/run_integration_tests.sh:31` — `-p nono-cli` → `-p nono-sandbox-cli`.

Live workspace package names via `cargo metadata --no-deps`:
`nono-sandbox`, `nono-sandbox-cli`, `nono-sandbox-proxy`, `nono-shell-broker`,
`nono-fltmgr-client`, `nono-ffi`, `sign-fixture`. (Note: the briefing's list also named
`nono` and `nono-agentd`; neither is a workspace *package* — `nono` is the lib name pinned
on the `nono-sandbox` package, and `nono-agentd` is a bin target, not a package. This did
not affect the fix, since `nono-sandbox-cli` was correct either way.)

`test-trust-overrides` confirmed a real feature of that package
(`crates/nono-cli/Cargo.toml:44`).

**Perturbation proof** — the check distinguishes pass from fail rather than merely passing:

```
cargo tree -p nono-sandbox-cli --features test-trust-overrides --depth 0
  → nono-sandbox-cli v0.70.0 (resolves)
cargo tree -p nono-cli --features test-trust-overrides --depth 0
  → error: cannot specify features for packages outside of workspace
```

The old name reproduces the exact CI error string; the new name resolves.

A `grep` for `nono-cli` / `nono-proxy` / `nono_cli` across the whole script returned that
one line only — no other stale selectors in this file.

**Scope held.** Not touched, per instructions: `.github/workflows/release.yml` (deliberately
`if: false`, guarded by `scripts/verify-release-yml-publish-selectors.ps1`, rewritten by
Phase 105 PUB-02), `phase-45-resl-native-host.yml`, `phase-46-uat-backlog.yml`, and the
cosmetic stale advice in `scripts/*.sh` / `sign-poc-local.ps1`.

## Fix 3 — undocumented `--allow-http2`

Added a `#### \`--allow-http2\`` entry to `docs/cli/usage/flags.mdx` in the Network Control
section, immediately after `--proxy-ca-validity` and before `--listen-port`, matching the
file's existing entry format (heading → prose → fenced bash example → `<Note>`).

**Semantics were read out of the code, not inferred from the flag name:**

- `crates/nono-cli/src/cli.rs:2197-2204` — declared on `RunArgs`, `help_heading = "OPTIONS"`
- `crates/nono-cli/src/sandbox_prepare.rs:811` —
  `allow_http2_requested = args.allow_http2 || profile_allow_http2` (additive OR)
- `crates/nono-cli/src/proxy_runtime.rs:236` — feeds `ProxyConfig::enable_h2`
- `crates/nono-proxy/src/pool.rs:210` — `enable_h2` selects HTTP/2 ALPN when building the
  **upstream** pooled client; `crates/nono-proxy/src/config.rs:141` defaults it to `false`

So the documented behaviour is: enables HTTP/2 ALPN on the proxy's pooled connections to
upstream servers, off by default. The `<Note>` records two facts a name-derived description
would have missed — it affects upstream connections only and is **not** a capability grant,
and because the merge is a pure OR (`grep` for `no-allow-http2` / `no_allow_http2` returns
zero hits) there is no way to turn it back off for a profile that opts in.

`git check-ignore -v docs/cli/usage/flags.mdx` exits 1 (not ignored), so plain `git add`
worked; staging was confirmed via `git status --short` showing `M ` in the index column
before committing.

## Verification

| Check | Result |
|-------|--------|
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | PASS (1.95 — see caveat above) |
| `cargo fmt --all -- --check` | exit 0 |
| `bash .github/scripts/check-cli-doc-flags.sh` | exit 0 ("parity check passed") |
| `bash -n tests/run_integration_tests.sh` | exit 0 |
| `-p` selector names a real workspace package | confirmed via `cargo metadata`, plus perturbation proof |
| `cargo test -p nono-sandbox-proxy` | 307 + 6 passed, 0 failed |
| `cargo test -p nono-sandbox-cli --bin nono` | 1688 passed / 12 failed / 2 ignored — see below |

### The baseline count moved: 1688 passed, not 1694

The **12 failures match the documented baseline exactly**, by name:

- 6 `config::tests` env races — `nono_home_dir_falls_through_when_unset`,
  `nono_home_dir_rejects_non_absolute_override`, `nono_home_dir_returns_override_when_set`,
  `test_validated_home_falls_back_to_userprofile`,
  `test_validated_home_ignores_non_absolute_home_when_userprofile_exists`,
  `user_state_dir_uses_localappdata_on_windows`
- 3 `protected_paths::tests` — `blocks_child_directory_capability`,
  `blocks_parent_directory_capability`,
  `requested_path_blocks_nonexistent_child_under_protected_root`
- `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`
- `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`
- the host-blocked WR-20 pin —
  `exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`
  (fails loudly on missing `SeRestorePrivilege`, D-31)

The **passed** count is 1688, not the documented 1694. I did not round that off — I
measured whether my change caused it:

```
cargo test -p nono-sandbox-cli --bin nono -- --list | grep -c ": test$"
  at HEAD (with fix)                          → 1702
  with proxy_command.rs reverted to base      → 1702
```

The test population is **identical** with and without the change, so the delta is not mine.
Arithmetic confirms the documented figure is stale rather than the run being short:
1688 + 12 + 2 ignored = 1702 total, whereas 1694 + 12 implies a 1706-test population that
does not exist at base commit `18405fc0`. The 1694 figure predates the recent Phase 117
round-8/9 commits (`f2d638df`, `388cf4bc`, …), which changed the test population. **The
correct post-task baseline for this branch is 1688 passed / 12 failed / 2 ignored.**

The revert used a targeted `git checkout <base> -- <single file>` and was restored with
`git checkout HEAD -- <single file>`; no blanket reset, no `git clean`, no `git stash`.
Working tree is clean and the fix is intact at `proxy_command.rs:394,396`.

### Not run / out of scope

- **1.97 clippy** — no such toolchain on the host; not installed, per instructions.
- **Full `make ci` / `make test`** — `make` is not installed on this host; the individual
  gates (clippy, fmt, the two doc/script checkers, the two test suites) were run directly.
- **Cross-target clippy (linux-gnu / apple-darwin)** — not required here. The
  CLAUDE.md rule triggers on cfg-gated Unix code; none of the three changed source files
  contains a `cfg(target_os)` block, none is under `exec_strategy/` or `bindings/c/src/`,
  and the diff is 3 lines of platform-neutral formatting plus a doc and a shell script.
- **`tests/run_integration_tests.sh` end-to-end** — not executed; it drives a full
  `--release` build and a live integration harness. The selector was validated by
  resolution plus perturbation proof rather than by running the suite.
- **libdbus / keyring backend / cross-compile job** — untouched, per instructions. No
  `libdbus-1-dev` was added anywhere.

## Deviations from Plan

**1. [Rule 2 — missing critical functionality] Extended the clippy fix to 2 more sites**
- **Found during:** Fix 1 class scan
- **Issue:** `crates/nono-cli/src/proxy_command.rs:394,396` carry the identical
  `&*`-in-format-arg shape. CI never reported them because `-D warnings` aborted at the
  upstream `nono-proxy` crate first.
- **Fix:** Same `&*x` → `*x` change; included in commit `8397e5b2`.
- **Why:** The task explicitly directed fixing the class. Leaving them would have
  guaranteed the next CI run failed on the same lint, defeating the task's purpose.
- **Consequence:** The verification bullet "confirm the `nono-proxy` change is the ONLY
  production-code line touched" is knowingly not satisfied. Flagged above rather than
  quietly dropped.

**2. [Documentation] Corrected baseline figure**
- The documented `cargo test -p nono-sandbox-cli --bin nono` baseline of 1694 passed is
  stale. Measured, proven not to be caused by this change, and restated as 1688/12/2.

No other deviations. No architectural changes, no auth gates, no blockers.

## Known Stubs

None. No placeholder values, TODO markers, or unwired data paths were introduced — the
diff is 3 lines of expression cleanup, 1 shell-script token, and 12 lines of prose.

## Threat Flags

None. No new network endpoints, auth paths, file-access patterns, or schema changes. The
one security-adjacent surface touched is credential rendering: `Zeroizing<String>` token
printing in `server.rs` / `proxy_command.rs`, whose zeroization semantics and output bytes
are unchanged (the wrapper is still derefed for `Display`, never cloned or leaked into a
new owned `String`).

## Self-Check: PASSED

Files verified present:
- `crates/nono-proxy/src/server.rs` — FOUND
- `crates/nono-cli/src/proxy_command.rs` — FOUND
- `tests/run_integration_tests.sh` — FOUND
- `docs/cli/usage/flags.mdx` — FOUND

Commits verified in `git log`:
- `8397e5b2` — FOUND
- `b5d3b1c6` — FOUND
- `90b36fc0` — FOUND

Working tree clean at `90b36fc0`; not pushed (orchestrator handles pushes).
`.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md` untouched; no SDK
state or roadmap writers were run.
