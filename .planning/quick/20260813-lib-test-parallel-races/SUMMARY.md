---
type: quick
slug: lib-test-parallel-races
created: 2026-08-13
status: complete
files_modified:
  - crates/nono/src/supervisor/aipc_sdk.rs
  - crates/nono/src/machine_policy.rs
---

# `nono-sandbox --lib` parallel-test flakiness — fixed

Both races were **shared process-global state mutated without mutual exclusion** under `cargo
test`'s default parallelism — the hazard CLAUDE.md's testing rule names, in one case for env vars
and in the other extended to registry keys.

## Race 1 — `NONO_SESSION_TOKEN` / `NONO_SESSION_ID`

`with_test_session_token` saved and restored both vars but held no lock, while **nine** tests called
it concurrently with overlapping values (`testtoken12345678` ×3, `testtoken12345678abc` ×1,
`smokesdktoken1234` ×5). Save/restore cannot help under parallelism — the failure mode is one test
reading another's live value, observed as
`left: "testtoken12345678" / right: "testtoken12345678abc"`.

**Fix:** a poison-tolerant `SESSION_TOKEN_ENV_LOCK` held for the whole mutation window, plus a
`Drop`-based `RestoreEnv` guard.

The `Drop` guard fixes a **second, latent defect** the finding did not name: restoration previously
ran only on the normal path, so a panic inside `f()` skipped it entirely and leaked the test token
to any later reader — including tests outside this module.

## Race 2 — the shared `SOFTWARE\nono-test` parent key

The four `machine_policy` tests use **disjoint** child subkeys, so those never collided. The
collision was that each one also deletes the **shared parent** on teardown. One test's teardown
marks that parent for deletion while another is still working beneath it:

```
Os { code: 1018, "Illegal operation attempted on a registry key that has been marked for deletion." }
```

**Fix:** a poison-tolerant `NONO_TEST_REGKEY_LOCK` taken at the top of all four tests. The parent
cleanup is kept — it is safe now that it cannot overlap another test.

Both locks use `.lock().unwrap_or_else(std::sync::PoisonError::into_inner)`, matching `nono-cli`'s
`lock_log_target_is_private_test()`: a panicking test must not poison the lock and cascade into
unrelated failures for every later taker.

## Verification

- `cargo test -p nono-sandbox --lib --no-fail-fast` × **8 consecutive runs → 843 passed / 0 failed,
  0 runs with failures.**
- `cargo fmt --all -- --check` clean.
- Cross-target clippy `--all-targets`, both Unix targets — see the commit for verdicts.

**On the strength of that evidence:** 8 green runs are supportive but are **not** proof, because the
failure was intermittent — the two known failures appeared in only 2 of the last ~4 full runs. The
load-bearing evidence is structural: the mutation windows are now inside mutexes, so overlap is
impossible by construction rather than improbable. Recorded this way deliberately rather than
claiming the runs settle it.

## Note for future test authors

`crates/nono`'s `--lib` target now has two serialization points. Any new test that mutates
`NONO_SESSION_TOKEN`/`NONO_SESSION_ID`, or creates/deletes keys under `HKCU\SOFTWARE\nono-test`,
must take the corresponding lock. Neither is enforced mechanically — a discovery gate over "tests
that call `std::env::set_var` without holding a lock" would be the durable fix, and is left as a
candidate rather than built here.
