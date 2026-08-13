# Phase 117 — Gap-Closure Round 4 Regression Verdict

**Verdict: round 4 introduced ZERO regressions, and CR-04's target stayed closed.**

**Run:** `cargo test --workspace --no-fail-fast`
**HEAD:** `99d8f977` (`fix(117-44): close WR-26 and WR-22 — one seam for both`) — all ten round-4 commits landed
**Captured:** 2026-08-13, finished 13:53:07Z
**Compared against:** [`117-BASELINE-ROUND4.md`](117-BASELINE-ROUND4.md) (HEAD `9ef519cf`, pre-fix)
**Cargo exit code:** 101 (`error: 5 targets failed`)

---

## Target-by-target comparison

| Target | Baseline | After round 4 | Verdict |
|---|---|---|---|
| `-p nono-sandbox-cli --bin nono` | 12 failed | 12 failed | **identical names** — 11 documented HOME/env-race + WR-20's host-blocked test |
| `-p nono-sandbox-cli --test audit_attestation` | 2 failed | 2 failed | unchanged — tests hardcode `/bin/pwd`, unresolvable on Windows |
| `-p nono-sandbox-cli --test env_vars` | 3 failed | 3 failed | unchanged — 1 host-environmental + the 2 UNCLASSIFIED |
| `-p nono-sandbox-cli --test layer_registry_selfcheck` | **1 failed (CR-04)** | **absent** | **CLOSED by 117-36 and still closed** |
| `-p nono-sandbox-cli --test resl_nix_async_signal_safety` | 1 failed | 1 failed | unchanged — red since ~Phase 25 |
| `-p nono-sandbox --lib` | passed | 1 failed | **flake, not a regression** — see below |
| `-p nono-sandbox-cli --bin nono-agentd` | passed | passed | **no regression** from 117-45's `daemon_attest_and_decide` arity change |

The one target that left the failing set is the one round 4 owned. Nothing entered it.

### The 12 `--bin nono` failures are name-for-name identical

```
audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty
config::tests::nono_home_dir_falls_through_when_unset
config::tests::nono_home_dir_rejects_non_absolute_override
config::tests::nono_home_dir_returns_override_when_set
config::tests::test_validated_home_falls_back_to_userprofile
config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists
config::tests::user_state_dir_uses_localappdata_on_windows
exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap
profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name
protected_paths::tests::blocks_child_directory_capability
protected_paths::tests::blocks_parent_directory_capability
protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root
```

Count and membership match the baseline exactly. This matters because 117-39 and 117-44 both changed
code exercised by this target (`main.rs`'s `render_error_for_operator`, `labels_guard.rs`'s coverage
arithmetic, `output.rs`'s banner signature, the emission block in `launch.rs`).

---

## The `--lib` failure is not a regression

```
---- machine_policy::tests::windows_wrong_reg_type_returns_policy_load_failed stdout ----
thread '...' panicked at crates\nono\src\machine_policy.rs:1399:14:
called `Result::unwrap()` on an `Err` value: Os { code: 1018, kind: Uncategorized,
message: "Illegal operation attempted on a registry key that has been marked for deletion." }
```

Established three independent ways:

1. **Round 4 never touched the file.**
   `git log --oneline 537ae106^..HEAD -- crates/nono/src/machine_policy.rs` → empty. The only
   `crates/nono` change in the round was `sandbox/windows.rs` (117-43, WR-25/WR-34).
2. **The error is a parallel-resource race**, not a logic failure — a shared HKLM test key deleted
   under a concurrent reader.
3. **It does not reproduce in isolation.** Two consecutive re-runs of the target:
   `test result: ok. 843 passed; 0 failed` (both times).

---

## Operator item, updated: `nono-sandbox --lib` has TWO distinct races

A **different** test failed in each of the last two full-workspace runs, so this target is
intermittently red under parallel execution:

| Run | Failing test | Shared resource |
|---|---|---|
| Wave-16 checkpoint (`415e7f4d`) | `supervisor::aipc_sdk::tests::windows_loopback_tests::helper_stamps_session_token_from_env` | `NONO_SESSION_TOKEN` env var — four tests set it, two with overlapping values (`testtoken12345678` vs `…abc`) |
| Round-4 final (`99d8f977`) | `machine_policy::tests::windows_wrong_reg_type_returns_policy_load_failed` | an HKLM test key, deleted while another test held it |

Both are the hazard CLAUDE.md's testing rule names ("Rust runs unit tests in parallel within the
same process, so an unrestored env var causes flaky failures in unrelated tests"), extended from
environment variables to registry keys.

**Why this is worth fixing on its own merits:** it is the one thing that can mask a real regression
in a future baseline comparison. A reviewer seeing `--lib` red has to re-run in isolation to decide
whether it matters — which is exactly the manual judgement call a green suite is supposed to remove.

---

## Run-cost note

`env_vars` took **2479s** (~41 min) this run against 1977s (~33 min) at baseline — it spawns real
supervised children and dominates wall clock. Budget **~45 minutes** for a full
`--no-fail-fast` workspace run on this host, not the ~40 recorded in the baseline.

---

_Raw capture retained for this session at_
`…/scratchpad/round4-final-regression.txt` _(scratchpad is session-scoped and not durable; the
verdict above is the record)._
