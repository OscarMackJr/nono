---
type: quick
slug: lib-test-parallel-races
created: 2026-08-13
---

# Fix `nono-sandbox --lib` parallel-test flakiness

Two distinct process-global races make `-p nono-sandbox --lib` intermittently red. A **different**
test failed in each of the last two full-workspace runs, which is why this is worth fixing on its
own merits: an intermittently-red target is the one thing that can mask a real regression in a
baseline comparison (see `117-REGRESSION-ROUND4.md`).

## Race 1 — `NONO_SESSION_TOKEN` / `NONO_SESSION_ID`

`supervisor::aipc_sdk::tests::with_test_session_token` (`aipc_sdk.rs:702`) saves and restores both
env vars, but has **no mutual exclusion**. Nine tests call it concurrently
(`:741 :772 :810 :845 :974 :1036 :1080 :1142 :1213`) with overlapping values —
`"testtoken12345678"` (×3) vs `"testtoken12345678abc"` (×1), and `"smokesdktoken1234"` (×5).

Observed: `left: "testtoken12345678"` / `right: "testtoken12345678abc"` — one test read another's
value. Save/restore cannot help under parallelism; only serialization can.

Secondary defect: if `f()` panics, restoration is skipped entirely, leaking a token to any later
test that reads it.

## Race 2 — the shared `SOFTWARE\nono-test` parent key

Four `machine_policy::tests` use **disjoint** HKCU subkeys (`machine_policy_test`,
`wrong_type_test`, `sentinel_only_test`, `configured_test`) — those do not collide. But each one
also deletes the **shared parent** on teardown (`:1383 :1430 :1500 :1538`):

```rust
let _ = hkcu.delete_subkey(r"SOFTWARE\nono-test");
```

So test A's teardown marks the parent for deletion while test B is mid-flight under it →
`Os { code: 1018, "Illegal operation attempted on a registry key that has been marked for deletion" }`.

## Approach

Serialize each group on a poison-tolerant process mutex, matching the repo's existing
`lock_log_target_is_private_test()` idiom in `cli_bootstrap.rs`
(`.lock().unwrap_or_else(std::sync::PoisonError::into_inner)`) — a panicking test must not poison
the lock and cascade into unrelated failures.

Additionally make the env restore panic-safe via a `Drop` guard, so a panic inside `f()` cannot
leak a token.

## Done when

- Both races are serialized; `cargo test -p nono-sandbox --lib` is green across repeated runs.
- The mutexes tolerate poisoning.
- Env restoration survives a panic in the closure.
- Cross-target clippy passes on both Unix targets (this is `crates/nono`, and `aipc_sdk.rs` /
  `machine_policy.rs` are cfg-gated).
