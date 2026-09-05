# Deferred Items — Quick Task 260904-wkv

## Pre-existing `cargo clippy --workspace` failure in `receipt_sink.rs` (out of scope)

**Found during:** Task 3's full verification gate.

**Issue:** `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`
fails with two `dead_code` errors (escalated to errors by `-D warnings`) in
`crates/nono-cli/src/receipt_sink.rs:315` (`ReceiptWriter.session_id` field never read) and
`crates/nono-cli/src/receipt_sink.rs:399,405` (`ReceiptWriter::session_id()` and
`ReceiptWriter::file_path()` methods never used), in both the `nono` and `nono-agentd` binary
targets.

**Root-cause isolation:** `cargo clippy -p nono-sandbox-cli --all-targets --all-features -- -D
warnings -D clippy::unwrap_used`, run with none of this quick task's changes reachable from
`nono-cli` (nothing in `nono-cli` imports or calls `session_credential`'s new pub items), fails
identically. This confirms the failure is pre-existing on the `milestone/v2.13-carryforward-closeout`
branch, entirely unrelated to this task's diff.

**Why not fixed:** `crates/nono-cli/src/receipt_sink.rs` is on this quick task's explicit
"FILES YOU MUST NOT MODIFY" list (it belongs to Phase 118, currently PAUSED at a human-verify
gate). Per the SCOPE BOUNDARY rule, out-of-scope pre-existing failures in files outside this
task's diff are not auto-fixed; per the hard gate's explicit file prohibition, this file cannot
be touched regardless.

**Disposition:** Not fixed. Recorded here for Phase 118's resumption — likely `session_id()`/
`file_path()` are consumed by a not-yet-landed Plan 118 task (Phase 118 is mid-implementation,
paused per `.planning/STATE.md`).

**Scoped verification that this task's own code is clean:**
`cargo clippy -p nono-sandbox --all-targets --all-features -- -D warnings -D clippy::unwrap_used`
exits 0 (see `260904-wkv-SUMMARY.md` for verbatim output).
