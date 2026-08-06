//! Phase 112 / WR-15 regression: every exit from the Linux supervisor loop
//! must drain the queued network seccomp notifications first.
//!
//! `run_supervisor_loop` is the only consumer of the proxy seccomp-notify fd.
//! Notifications that are still queued when it returns are discarded together
//! with the fd, and with them the end-of-run network denial records that feed
//! the denial diagnostic footer and the audit/session denial record.
//!
//! The CR-03 fix added `drain_pending_network_notifications(...)` to the
//! orphan-reaper exit only; three sibling exits kept returning undrained —
//! including the startup-timeout `SIGKILL` return, i.e. exactly the run whose
//! queued denials are most worth recording. The fix routes every early exit
//! through the local `drained_return!` macro and drains once more before the
//! trailing blocking wait that both `break` paths land on.
//!
//! This is a structural check on the source text rather than a runtime test:
//! driving the loop needs a live Landlock+seccomp supervised child, which is
//! not available on every CI host (and not at all on the Windows dev host).
//! Reading the file as text works everywhere, and it fails loudly the moment a
//! new undrained `return Ok((...))` is added.

use std::path::PathBuf;

const START_SENTINEL: &str = "WR-15-SUPERVISOR-EXITS-START";
const END_SENTINEL: &str = "WR-15-SUPERVISOR-EXITS-END";

fn read_exec_strategy() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir)
        .join("src")
        .join("exec_strategy.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Text between the two sentinels, with line comments stripped so that prose
/// mentioning `return Ok((` cannot false-positive the scan.
fn sentinel_region(src: &str) -> String {
    let start = src.find(START_SENTINEL).unwrap_or_else(|| {
        panic!(
            "expected `{START_SENTINEL}` sentinel in exec_strategy.rs — \
             the Linux `run_supervisor_loop` lost its scoping sentinel; \
             re-place it (Phase 112 WR-15)"
        )
    });
    let end = src.find(END_SENTINEL).unwrap_or_else(|| {
        panic!(
            "expected `{END_SENTINEL}` sentinel in exec_strategy.rs — \
             the Linux `run_supervisor_loop` lost its closing sentinel; \
             re-place it (Phase 112 WR-15)"
        )
    });
    assert!(
        end > start,
        "{END_SENTINEL} must appear after {START_SENTINEL} in source order"
    );

    src[start..end]
        .lines()
        .map(|line| match line.find("//") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// WR-15: no bare `return Ok((` inside the supervisor loop — every early exit
/// goes through `drained_return!`, which drains before returning.
#[test]
fn every_supervisor_loop_exit_drains_network_notifications() {
    let src = read_exec_strategy();
    let region = sentinel_region(&src);

    let bare = region.matches("return Ok((").count();
    assert_eq!(
        bare, 0,
        "WR-15 regression: found {bare} bare `return Ok((` inside the Linux \
         `run_supervisor_loop` body in crates/nono-cli/src/exec_strategy.rs.\n\
         \n\
         Every exit from that loop must first call \
         `drain_pending_network_notifications(...)`, or the network denial \
         records still queued on the proxy seccomp-notify fd are discarded \
         with the fd. Use the local `drained_return!($status)` macro instead \
         of returning directly."
    );

    // The macro must actually be used — an empty loop body would satisfy the
    // check above vacuously.
    let drained = region.matches("drained_return!").count();
    assert!(
        drained >= 4,
        "WR-15 regression: expected at least 4 `drained_return!` exits in the \
         Linux `run_supervisor_loop` (orphan reaper, startup-timeout SIGKILL, \
         normal `Ok(status)`, and `ECHILD`), found {drained}"
    );
}

/// WR-15: the trailing blocking wait that both `break` paths land on is
/// preceded by a drain. Draining *before* the wait also unblocks a child that
/// is parked on a network notification nobody is left to answer.
#[test]
fn supervisor_loop_break_paths_drain_before_the_blocking_wait() {
    let src = read_exec_strategy();
    let region = sentinel_region(&src);

    let tail_wait = region.rfind("wait_for_child(child)?").unwrap_or_else(|| {
        panic!(
            "expected a trailing `wait_for_child(child)?` after the supervisor \
             loop in crates/nono-cli/src/exec_strategy.rs"
        )
    });
    let last_drain = region
        .rfind("drain_pending_network_notifications(")
        .unwrap_or_else(|| {
            panic!(
                "WR-15 regression: no `drain_pending_network_notifications(` call \
                 found inside the Linux `run_supervisor_loop` body"
            )
        });

    assert!(
        last_drain < tail_wait,
        "WR-15 regression: the `break` paths of the Linux `run_supervisor_loop` \
         fall through to a blocking `wait_for_child(child)?` that is not \
         preceded by `drain_pending_network_notifications(...)`.\n\
         \n\
         Drain before the wait, not after: the child may be parked on a \
         network notification that nobody is left to answer, and its queued \
         denials are otherwise discarded with the notify fd."
    );
}
