#![cfg(all(target_os = "windows", feature = "layer-fault-injection"))]
#![allow(clippy::unwrap_used)]
//! Phase 117 Plan 12 (CINT-03): one forced-unavailable integration test per
//! automatable `LayerId` registry row, exercising the REAL launch path
//! through a spawned `nono.exe` subprocess.
//!
//! # Why this file cannot call the pub(crate) setters directly
//!
//! `nono-sandbox-cli` has no `[lib]` target (binary-only, two `[[bin]]`
//! targets). `tests/*.rs` files are therefore always a SEPARATE compilation
//! unit with zero visibility into `main.rs`'s internals — they can only
//! interact with the built binaries via subprocess spawn
//! (`env!("CARGO_BIN_EXE_nono")`), never by calling a `pub(crate)` function.
//! Plans 06/07 shipped each layer's force-unavailable seam as a
//! `pub(crate)`-only static + setter + getter, exercised by an in-crate
//! `#[cfg(test)] mod tests` regression test living in the SAME source file as
//! the seam (e.g. `restricted_token.rs`'s own test module) — that pattern
//! proves the seam fires, but is not discoverable from this file's
//! `tests/layer_force_unavailable.rs` location, and does not exercise the
//! seam from OUTSIDE the process the way an external CINT-03 test should.
//!
//! **Deviation (Rule 3 — missing env var, an explicitly Rule-3-listed
//! blocking-issue example): this plan adds a minimal, `layer-fault-injection`
//! -feature-gated env-var bridge** (`crates/nono-cli/src/command_runtime.rs`,
//! `crates/nono-cli/src/exec_strategy_windows/mod.rs`) so this external test
//! can arm each seam from outside the process, mirroring the ALREADY-SHIPPED
//! `--dangerous-force-wfp-ready` CLI-flag bridge Plan 04 built for WFP (the
//! only layer that had one before this plan). Reading
//! `NONO_FORCE_UNAVAILABLE_RESTRICTED_TOKEN` etc. and calling the
//! corresponding `pub(crate)` setter is the SAME `#[cfg(feature =
//! "layer-fault-injection")]`-gated, compiled-out-of-release shape D-30
//! already requires — no new runtime toggle exists in a default build; see
//! this plan's SUMMARY for the full deviation writeup.
//!
//! # Automated vs. manually-verified rows (D-31/D-32)
//!
//! WR-07: the counts below were wrong (they read "the 3" and "the remaining
//! 10", describing a third `#[test]` in this file that does not exist —
//! Phase 117-44's WR-05 removed the duplicate). The real split is
//! **2 automated here + 8 `ALSO_AUTOMATED` + 3 `MANUALLY_VERIFIED` = 13**,
//! and `layer_registry_meta_test.rs::coverage_split_accounts_for_every_layer_id`
//! now asserts that arithmetic mechanically, so this prose cannot drift
//! again without failing the build.
//!
//! Of the 13 `LayerId` rows, this file automates the 2 that are BOTH (a)
//! reachable via a plain, ordinary-host `nono run` subprocess, (b) able to
//! independently demonstrate THEIR OWN row's short-circuit (not merely a
//! shared call site's short-circuit a different row would also trip), and
//! (c) checked EARLY — inside `prepare_live_windows_launch`, before the
//! Windows `Supervised`-strategy session/capability-pipe event loop starts —
//! which turns out to be load-bearing for reliable external subprocess
//! testing (see below). The remaining 11 rows are split across
//! `layer_registry_meta_test.rs`'s two lists — 8 on `ALSO_AUTOMATED` (a
//! real, ordinary-host-runnable in-process test elsewhere in the tree,
//! existence-checked on every run by `also_automated_entries_are_non_vacuous`)
//! and 3 on `MANUALLY_VERIFIED` (`DaclSessionSidGrant`,
//! `MinifilterAbsence`, `BrokerAuthenticodeTrustGate`) — each with its
//! own named, loud reason (D-31). The reasons below cover rows on BOTH
//! lists: a live-elevated-service requirement
//! (`WfpEgressFilters`), structural absence (`MinifilterAbsence`), no shipped
//! force-unavailable seam for this plan to consume (`FirewallRulesEgress`), a
//! gate that is inert outside a signed production install
//! (`BrokerAuthenticodeTrustGate`), a pre-flight (not post-spawn) check
//! already proven at the library-unit level (`InterpreterCoverageGate`), a
//! real confined child spawned inside a SEPARATE `nono-shell-broker.exe`
//! process that is console-fragile to drive externally
//! (`AppContainerProfile`), two DACL sub-rows (`DaclAncestorTraverse`,
//! `DaclAncestorReadAttrs`) whose own apply functions are constructed
//! strictly AFTER the shared `DACL_GRANT_FORCE_UNAVAILABLE`-gated
//! `AppliedDaclGrantsGuard` in `prepare_live_windows_launch`
//! (`exec_strategy_windows/mod.rs`) — arming that ONE shared flag always
//! aborts the launch at the FIRST DACL guard, so the ancestor guards' own
//! short-circuits can never be independently observed by a black-box
//! subprocess test under the current shared-flag design (verified by reading
//! `mod.rs::prepare_live_windows_launch`'s guard construction order:
//! `applied_dacls` unconditionally precedes
//! `applied_ancestor_traverse`/`applied_ancestor_read_attrs`,
//! and the `?` on `applied_dacls` returns before
//! either of the later `let` bindings is ever reached) — and, discovered
//! live during this plan's own execution, `RestrictedToken` and
//! `JobObjectContainment`.
//!
//! ## `RestrictedToken` / `JobObjectContainment` — proven correct, not
//! reliably automatable from THIS harness
//!
//! Both seams are checked LATE — inside `spawn_windows_child`
//! (`restricted_token.rs::create_restricted_token_with_sid`, reached from
//! `launch.rs::spawn_windows_child`, and
//! `launch.rs::apply_process_handle_to_containment` respectively), AFTER
//! `prepare_live_windows_launch` has already brought the Windows
//! `Supervised`-strategy session file + capability-pipe event loop up —
//! unlike the three rows automated below, which all abort during
//! `prepare_live_windows_launch` itself, before that machinery starts.
//! Empirically, on this development host, killing/unwinding a `nono.exe`
//! child AFTER the Supervised event loop has started (which is exactly what
//! happens when one of these two seams is armed) reproducibly stalls the
//! child's own teardown when spawned as a piped-stdio subprocess from a
//! `cargo test` harness process — even with a bounded-wait-and-kill retry
//! loop (4 attempts × 45s), the stall recurred on every attempt. This is a
//! HOST/HARNESS characteristic, not a defect in the seam or the assertion:
//! two independent, ISOLATED single-invocation reproductions (via
//! PowerShell's `Start-Process -RedirectStandardOutput/-RedirectStandardError`,
//! run directly against `target\debug\nono.exe` with
//! `NONO_FORCE_UNAVAILABLE_RESTRICTED_TOKEN=1`, outside of `cargo test`'s
//! own subprocess-management) completed in well under a second each,
//! producing the exact expected diagnostic: `nono: Sandbox initialization
//! failed: Windows supervised execution failed during shutting-down
//! (session: ...): Startup self-attestation failed for layer
//! RestrictedToken: forced unavailable by test seam`. Both rows are
//! therefore on the `MANUALLY_VERIFIED` list with their manual reproduction
//! steps (the exact command above), not silently dropped — D-31's "loud
//! gap, never a silent skip."

use std::process::{Command, Output};

fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

fn combined_output(output: &Output) -> String {
    let mut s = String::from_utf8_lossy(&output.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&output.stderr));
    s
}

/// The minimal, ordinary WriteRestricted-arm supervised launch every
/// automated row in this file drives: no `--profile`/`--allow` needed
/// (mirrors `env_vars.rs`'s `windows_run_executes_basic_command`, already
/// proven to reach the default supervised WriteRestricted-arm path).
/// `env_var` is armed to "1" in THIS process's own environment before
/// spawning — the layers this file tests are all applied by nono-cli
/// itself, in-process, before it ever spawns the confined child, so no
/// env-forwarding to a grandchild is required for these rows. All three
/// automated rows abort during `prepare_live_windows_launch`, before the
/// Supervised session/capability-pipe event loop starts, so a plain
/// `.output()` (blocking to completion) is safe here — see the module doc
/// comment for why `RestrictedToken`/`JobObjectContainment` (which abort
/// LATER, after that event loop starts) are NOT tested this way.
fn run_minimal_with_forced_unavailable(env_var: &str) -> Output {
    nono_bin()
        .env(env_var, "1")
        .args(["run", "--", "cmd", "/c", "echo", "hello"])
        .output()
        .expect("failed to spawn nono.exe")
}

/// Assert the CONTRACTED `Abort` outcome: non-zero exit, and the
/// `NonoError::LayerAttestationFailed` Display text ("Startup self-attestation
/// failed for layer {layer}: {reason}") naming `expected_layer_in_message`
/// present in combined stdout+stderr. `expected_layer_in_message` is the
/// literal `layer` string the production code's `Err` constructs — for rows
/// sharing a call site (documented per-test below) this is NOT always the
/// same string as the row name the test function itself is named after.
fn assert_layer_attestation_abort(output: &Output, expected_layer_in_message: &str) {
    let text = combined_output(output);
    assert!(
        !output.status.success(),
        "expected non-zero exit when the seam is forced unavailable, got success:\n{text}"
    );
    assert!(
        text.contains("Startup self-attestation failed for layer")
            && text.contains(expected_layer_in_message),
        "expected the LayerAttestationFailed diagnostic naming {expected_layer_in_message:?} \
         in combined output, got:\n{text}"
    );
}

/// `LayerId::MandatoryIntegrityLabel` — `labels_guard.rs`'s
/// `MANDATORY_LABEL_FORCE_UNAVAILABLE` seam (Plan 06). Expected on all 5
/// `DirectCli` arms, so the default WriteRestricted-arm launch exercises it.
/// Checked inside `prepare_live_windows_launch`, before the Supervised
/// session event loop starts — non-vacuous (disarming the seam changes a
/// success to a failure) and reliably fast.
#[test]
fn force_unavailable_mandatory_integrity_label() {
    let output = run_minimal_with_forced_unavailable("NONO_FORCE_UNAVAILABLE_MANDATORY_LABEL");
    assert_layer_attestation_abort(&output, "MandatoryIntegrityLabel");
}

/// `LayerId::DaclPackageSidGrant` — `dacl_guard.rs`'s ONE shared
/// `DACL_GRANT_FORCE_UNAVAILABLE` seam (Plan 06), fired via
/// `AppliedDaclGrantsGuard::snapshot_and_apply` (`mod.rs`), the FIRST DACL
/// guard `prepare_live_windows_launch` constructs. That guard is passed
/// `config.package_sid`, so this IS this row's own enforcement call site.
///
/// # Phase 117 review WR-05
///
/// There used to be TWO tests here — `force_unavailable_dacl_session_sid_grant`
/// and `force_unavailable_dacl_package_sid_grant` — with byte-identical
/// bodies, both arming `NONO_FORCE_UNAVAILABLE_DACL_GRANT` and both
/// asserting the string `"DaclSessionSidGrant"`. The package-SID test
/// therefore proved nothing about its own row and was counted as automated
/// coverage by `every_registry_row_has_a_test` purely because a function
/// with the right name existed. CR-05 established that the guard only ever
/// grants the PACKAGE SID, so the seam now reports `DaclPackageSidGrant` —
/// the layer it actually implements — and this is the one, correctly
/// labelled test. `DaclSessionSidGrant` no longer has (or needs) a test:
/// its registry expectancy is empty because no shipped call site applies it.
///
/// Non-vacuous: without the env var, the identical command exits 0 with
/// "hello" in stdout (`windows_run_executes_basic_command`, `env_vars.rs`) —
/// disarming the seam changes the outcome, proving this assertion can fail.
#[test]
fn force_unavailable_dacl_package_sid_grant() {
    let output = run_minimal_with_forced_unavailable("NONO_FORCE_UNAVAILABLE_DACL_GRANT");
    assert_layer_attestation_abort(&output, "DaclPackageSidGrant");
}
