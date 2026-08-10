// Phase 117 review CR-10: the `feature = "layer-fault-injection"` gate is
// REMOVED. These three tests read `layer_registry.rs`,
// `layer_force_unavailable.rs` and the SPEC as TEXT — they touch no
// fault-injection seam and never spawn anything, so the feature was
// gratuitous. With it, the D-32 discovery gate ran in no build, no `make`
// target and no CI job: adding a 14th `LayerId` failed nothing, which
// directly falsified the SPEC's "or that test fails the build" claim.
//
// The `target_os = "windows"` gate stays: `layer_registry::all_entries()` is
// Windows-only and the SPEC documents a Windows-only contract.
#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]
//! Phase 117 Plan 12 (CINT-03/D-32): the discovery-based meta-test that makes
//! "a contract entry with no such test is not satisfied" mechanically true.
//!
//! Three tests, all discovery-based (Phase 115 V-01 lesson: "a test that
//! names its targets is blind by construction" — none of these hardcode the
//! 13 `LayerId` names themselves; all three read `LayerId::ALL` fresh out of
//! `layer_registry.rs`'s source text on every run, following
//! `layer_registry_selfcheck.rs`'s own established `CARGO_MANIFEST_DIR` +
//! `std::fs::read_to_string` house pattern — no `include_str!`, no `regex`):
//!
//! - `every_registry_row_has_a_test`: every `LayerId` NOT on the
//!   `MANUALLY_VERIFIED` allow-list below must have a
//!   `force_unavailable_<snake_case_name>` function in
//!   `layer_force_unavailable.rs`'s source text. Adding a 14th `LayerId`
//!   variant without a corresponding test (and without adding it to
//!   `MANUALLY_VERIFIED` with a reason) fails this test.
//! - `host_gated_rows_are_loud`: every `MANUALLY_VERIFIED` entry's reason
//!   string is non-empty AND the row's name appears in the SPEC's
//!   manual-verification section — D-31's "never a silent skip" mechanically
//!   enforced.
//! - `security_assumptions_are_loud`: the SAME loudness check for
//!   `MANUAL_SECURITY_ASSUMPTIONS`, a separate, non-`LayerId` list for
//!   cross-cutting assumptions like the ETW/AppLog child-readability item
//!   (Blocker-3, checker pass 2).

use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap_or_else(|| {
            panic!(
                "CARGO_MANIFEST_DIR {} has no parent",
                manifest_dir().display()
            )
        })
        .parent()
        .unwrap_or_else(|| {
            panic!(
                "CARGO_MANIFEST_DIR {} has no grandparent (expected crates/nono-cli under a \
                 workspace root)",
                manifest_dir().display()
            )
        })
        .to_path_buf()
}

fn read_layer_registry() -> String {
    let path = manifest_dir()
        .join("src")
        .join("exec_strategy_windows")
        .join("layer_registry.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn read_force_unavailable_tests() -> String {
    let path = manifest_dir()
        .join("tests")
        .join("layer_force_unavailable.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn read_spec() -> String {
    let path = workspace_root()
        .join("proj")
        .join("SPEC-windows-fail-direction-contract.md");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Extracts the identifier list inside `layer_registry.rs`'s
/// `pub(crate) const ALL: &[LayerId] = &[ ... ];` block, mirroring
/// `layer_registry_selfcheck.rs::extract_all_layer_id_names` exactly (kept
/// as a separate copy per that file's own house convention: each
/// `tests/*.rs` file is a SEPARATE compilation unit and cannot import from a
/// sibling integration-test file).
fn extract_all_layer_id_names(src: &str) -> Vec<String> {
    let marker = "const ALL: &[LayerId] = &[";
    let start = src.find(marker).unwrap_or_else(|| {
        panic!(
            "expected to find `{marker}` in layer_registry.rs — the ALL const was renamed, \
             removed, or reformatted; update this test's marker to match"
        )
    });
    let after_marker = &src[start + marker.len()..];
    let end = after_marker
        .find("];")
        .unwrap_or_else(|| panic!("expected a closing `];` after `{marker}` in layer_registry.rs"));
    let body = &after_marker[..end];

    body.split(',')
        .filter_map(|entry| {
            let trimmed = entry.trim();
            trimmed
                .strip_prefix("LayerId::")
                .map(|name| name.trim().to_string())
        })
        .filter(|name| !name.is_empty())
        .collect()
}

/// `PascalCase` -> `snake_case`, matching the naming convention
/// `layer_force_unavailable.rs`'s function names use
/// (`force_unavailable_<snake_case_name>`). Inserts `_` before every
/// uppercase ASCII letter that is not the first character. None of the 13
/// current `LayerId` names contain a back-to-back-acronym run (e.g. no
/// "SID" in all-caps — always "Sid"), so this simple rule is exact for the
/// names this file discovers; it is re-derived from source on every run
/// rather than hardcoded, so a future name with a different shape would
/// simply produce a different (still-searched-for) string, not a silent
/// false pass.
fn pascal_to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// D-31: `LayerId` rows with no automatable `force_unavailable_*` test in
/// `layer_force_unavailable.rs`, each with a named, cited reason. Finalized
/// by this plan against what Plans 04/06/07 actually shipped and what this
/// plan's own execution empirically found (see `layer_force_unavailable.rs`'s
/// module doc comment for the full `RestrictedToken`/`JobObjectContainment`
/// investigation writeup).
const MANUALLY_VERIFIED: &[(&str, &str)] = &[
    (
        "DaclSessionSidGrant",
        "Phase 117 review CR-05: this layer does not exist in the shipped tree. The only \
         production construction of `AppliedDaclGrantsGuard` is passed `config.package_sid`, \
         not `config.session_sid`, and the synthetic per-session restricting SID is granted on \
         no DACL anywhere — so the row's expectancy is empty and there is nothing to force \
         unavailable. Manual verification is therefore a CODE READ, not a run: confirm \
         `grep -rn \"session_sid\" crates/nono-cli/src` still shows no DACL grant of \
         `config.session_sid`. If that ever changes, the registry expectancy must be restored \
         in the same commit. See the RF-03 open operator decision in \
         `proj/SPEC-windows-fail-direction-contract.md`.",
    ),
    (
        "WfpEgressFilters",
        "Requires a live, elevated nono-wfp-service and a non-elevated daemon session \
         (per-SID WFP is daemon-path only, `nono agent launch`, not direct `nono run`) — not \
         reproducible on an ordinary, non-elevated dev/CI host. Manual steps: install/start \
         nono-wfp-service as admin, run a confined daemon session from a non-elevated shell \
         with the WFP layer forced unavailable, assert the contracted Abort outcome.",
    ),
    (
        "MinifilterAbsence",
        "Structurally untestable: no minifilter driver exists in this tree (ADR-65 stands). \
         The row documents a deliberate structural absence, not a probeable mechanism — there \
         is nothing to force unavailable. Record as absent, citing ADR-65, per Phase 116 D-08's \
         structurally-blocked row form.",
    ),
    (
        "FirewallRulesEgress",
        "No force-unavailable seam was shipped for this row (Plans 04/06/07 covered the WFP \
         and CLI-side token/label/DACL/JobObject/AppContainer layers only). Forcing \
         `run_netsh_firewall` to fail closed would need either live `netsh` manipulation or a \
         new production seam, both out of this plan's file-scoped remit (test files + SPEC \
         only). Manual steps: temporarily block `netsh advfirewall` (e.g. via a conflicting \
         firewall rule or a restricted execution policy) and confirm `nono run` with the \
         FirewallRules backend selected aborts with the partial-rule-rollback behavior \
         documented in `network.rs:1567-1586`.",
    ),
    (
        "BrokerAuthenticodeTrustGate",
        "The gate is skipped entirely under `is_dev_build_layout()` (`launch.rs:2190`,2194`) — \
         active only in a signed, production (non-dev-layout) install. An ordinary dev/CI host \
         building from `cargo build` cannot exercise it. Manual steps: from a signed release \
         install outside `target/...`, stage a `nono-shell-broker.exe` signed by a different \
         identity than `nono.exe` and confirm the broker-arm spawn refuses \
         (`broker_authenticode.rs::broker_signature_mismatch_refuses_spawn` already covers the \
         underlying `verify_broker_authenticode` logic directly; this item is the live-install \
         end-to-end round-trip).",
    ),
    (
        "InterpreterCoverageGate",
        "A pre-flight, build-time-of-the-launch-plan check (`probe: ProbeKind::NotApplicable`) \
         — not a D-21 post-spawn attestation, so it has no force-unavailable seam to arm. The \
         fail-closed mechanism is already proven at the library-unit level by \
         `crates/nono/src/sandbox/windows.rs::validate_launch_paths_refuses_uncovered_interpreter`. \
         A full CLI-level round-trip additionally requires reconstructing \
         `resolve_interpreter_paths`'s shebang/PATH interpreter-resolution shape end-to-end, \
         deferred as a follow-up rather than staged un-reviewed in this plan.",
    ),
    (
        "AppContainerProfile",
        "The real AppContainer-confined child is spawned INSIDE a separate \
         `nono-shell-broker.exe` process (Blocker-1) — nono-cli's own gate never attests this \
         layer. Driving the `BrokerLaunchNoPty` arm externally requires a real console session \
         (broker spawn fails with GLE=87 under git-bash/MSYS, per project memory \
         `feedback_windows_supervised_needs_real_console.md`) and was found, during this plan's \
         own execution, to be console-fragile even from a PowerShell-wrapped `cargo test` \
         harness. The underlying seam has real, passing in-crate coverage \
         (`nono-shell-broker/src/main.rs::run_fails_when_app_container_forced_unavailable`, \
         Plan 07, 24/24 passing). Manual steps: from a real (non-git-bash) PowerShell console, \
         run `nono run --profile claude-code` with `NONO_FORCE_UNAVAILABLE_APP_CONTAINER=1` set \
         and confirm the broker-arm spawn refuses.",
    ),
    (
        "DaclAncestorTraverse",
        "Shares `dacl_guard.rs`'s ONE `DACL_GRANT_FORCE_UNAVAILABLE` flag with \
         `DaclSessionSidGrant`/`DaclPackageSidGrant`, but `AppliedAncestorTraverseGuard::\
         snapshot_and_apply` is constructed STRICTLY AFTER `AppliedDaclGrantsGuard::\
         snapshot_and_apply` in `prepare_live_windows_launch` (`mod.rs:449` then `mod.rs:462`) \
         — arming the shared flag always aborts the launch at the FIRST guard via its `?`, so \
         this row's own apply function is never reached by any external, black-box subprocess \
         test. Direct evidence is the in-crate unit test in `dacl_guard.rs`'s `#[cfg(test)]` \
         module (117-06: '8 feature-gated regression tests proving each hook short-circuits \
         before its real OS call'), which calls `AppliedAncestorTraverseGuard::snapshot_and_apply` \
         directly.",
    ),
    (
        "DaclAncestorReadAttrs",
        "Identical reasoning to `DaclAncestorTraverse` immediately above: shares the same \
         shared flag, and `AppliedAncestorReadAttributesGuard::snapshot_and_apply_targets` is \
         constructed even later in `prepare_live_windows_launch` (`mod.rs:486`), strictly after \
         `applied_dacls`'s `?` would already have returned. Direct evidence is the in-crate \
         unit test in `dacl_guard.rs`'s `#[cfg(test)]` module (117-06).",
    ),
    (
        "RestrictedToken",
        "The seam is checked LATE — inside `spawn_windows_child` (`launch.rs:1620`), AFTER the \
         Windows Supervised-strategy session file + capability-pipe event loop has already \
         started (unlike the three rows this plan automates, which all abort during \
         `prepare_live_windows_launch`, before that machinery starts). Empirically, on this \
         plan's development host, killing/unwinding a piped-stdio `nono.exe` child spawned from \
         a `cargo test` harness process AFTER that event loop starts reproducibly stalls the \
         child's own teardown — even across a bounded-wait-and-kill retry loop (4 attempts x \
         45s). This is a host/harness characteristic, not a defect: two independent, isolated \
         single-invocation reproductions via PowerShell's `Start-Process` (outside `cargo \
         test`'s own subprocess management) completed correctly in well under a second each, \
         producing the exact expected diagnostic (`Startup self-attestation failed for layer \
         RestrictedToken: forced unavailable by test seam`). Manual steps: from a real \
         PowerShell console, run `target\\debug\\nono.exe run -- cmd /c echo hello` with \
         `NONO_FORCE_UNAVAILABLE_RESTRICTED_TOKEN=1` set and confirm the diagnostic above.",
    ),
    (
        "JobObjectContainment",
        "Same late-checked, event-loop-already-started class as `RestrictedToken` immediately \
         above (`apply_process_handle_to_containment`, `launch.rs:404-422`, called from within \
         `spawn_windows_child` after the session/capability-pipe machinery is up) and the same \
         empirically-observed host/harness teardown-stall characteristic. Manual steps: from a \
         real PowerShell console, run `target\\debug\\nono.exe run -- cmd /c echo hello` with \
         `NONO_FORCE_UNAVAILABLE_JOB_OBJECT=1` set and confirm the `JobObjectContainment` \
         diagnostic.",
    ),
];

/// Blocker-3 fix (checker pass 2): a SEPARATE, non-`LayerId` list for
/// cross-cutting security assumptions that are not tied to one registry row
/// — the concrete pickup Plan 09's flagged ETW/AppLog assumption needed and
/// did not get in the prior pass.
const MANUAL_SECURITY_ASSUMPTIONS: &[(&str, &str)] = &[(
    "etw-applog-child-readability",
    "verify via wevtutil gl Application that the channelAccess SDDL does not grant read to a \
     low-integrity/AppContainer-reachable SID before trusting SecurityEvent.downgraded_layers \
     as operator-only — see telemetry/event.rs doc comment",
)];

/// D-32: for every `LayerId` NOT on `MANUALLY_VERIFIED`, a
/// `force_unavailable_<snake_case_name>` function must exist in
/// `layer_force_unavailable.rs`'s source text. Discovery-based: this test
/// reads `LayerId::ALL` fresh from `layer_registry.rs` on every run — it
/// does NOT hardcode the 13 current names. Adding a 14th `LayerId` variant
/// without a corresponding test function (or a `MANUALLY_VERIFIED` entry)
/// fails here.
#[test]
fn every_registry_row_has_a_test() {
    let registry_src = read_layer_registry();
    let variant_names = extract_all_layer_id_names(&registry_src);
    assert!(
        variant_names.len() >= 13,
        "expected at least 13 LayerId variants (the 117-01 inventory), found {}: {variant_names:?}",
        variant_names.len()
    );

    let test_src = read_force_unavailable_tests();
    let manual_names: Vec<&str> = MANUALLY_VERIFIED.iter().map(|(name, _)| *name).collect();

    let mut missing = Vec::new();
    for name in &variant_names {
        if manual_names.contains(&name.as_str()) {
            continue;
        }
        let expected_fn = format!("fn force_unavailable_{}", pascal_to_snake_case(name));
        if !test_src.contains(&expected_fn) {
            missing.push(format!("{name} (expected `{expected_fn}`)"));
        }
    }

    assert!(
        missing.is_empty(),
        "the following LayerId row(s) have neither a force_unavailable_* test in \
         layer_force_unavailable.rs nor a MANUALLY_VERIFIED entry in this file — CINT-03: \
         \"a contract entry with no such test is not satisfied\":\n{}",
        missing.join("\n")
    );
}

/// Phase 117 review WR-03: locate the SPEC's manual-verification section and
/// return only the text FROM that heading onward.
///
/// The whole point of `host_gated_rows_are_loud` is "the row is documented in
/// the manual-verification section". Searching the whole document cannot
/// check that: every `LayerId` name already appears in the SPEC's registry
/// table, so the assertion was true for any row that exists at all and the
/// test could not detect the precise failure it advertises — a row added to
/// `MANUALLY_VERIFIED` with no manual-verification entry.
fn manual_verification_section(spec: &str) -> &str {
    let lower = spec.to_ascii_lowercase();
    let idx = lower
        .match_indices('\n')
        .map(|(i, _)| i + 1)
        .chain(std::iter::once(0))
        .filter(|&start| lower[start..].starts_with("## "))
        .find(|&start| {
            let line_end = lower[start..].find('\n').map_or(lower.len(), |e| start + e);
            let heading = &lower[start..line_end];
            heading.contains("manual") || heading.contains("host-gated")
        })
        .expect(
            "proj/SPEC-windows-fail-direction-contract.md has no `## ` heading containing \
             \"manual\" or \"host-gated\" — D-31 requires a loud, named manual-verification home",
        );
    &spec[idx..]
}

/// D-31: every `MANUALLY_VERIFIED` row must be LOUD — a non-empty reason,
/// AND the row's name must appear in the SPEC's manual-verification
/// SECTION, not merely somewhere in the document and not merely in this test
/// file. This is what makes a host-gated row visible to a reader of the
/// SPEC, not just to a reader of this Rust source.
#[test]
fn host_gated_rows_are_loud() {
    let spec = read_spec();
    // WR-03: scoped to the section, not the whole document.
    let section = manual_verification_section(&spec);

    let mut missing = Vec::new();
    for (name, reason) in MANUALLY_VERIFIED {
        assert!(
            !reason.trim().is_empty(),
            "MANUALLY_VERIFIED entry {name:?} has an empty reason string — D-31 requires a \
             named reason, never a bare skip"
        );
        if !section.contains(name) {
            missing.push(*name);
        }
    }

    assert!(
        missing.is_empty(),
        "the following MANUALLY_VERIFIED LayerId row(s) do not appear in the \
         manual-verification SECTION of proj/SPEC-windows-fail-direction-contract.md — a \
         host-gated row must be documented there, not only in the registry table above it or \
         in this test file's source (D-31 \"loud, never silent\"):\n{missing:?}"
    );
}

/// WR-03 non-vacuity guard: the scoping helper must actually exclude the
/// registry table, where every `LayerId` name appears. If
/// `manual_verification_section` regressed to returning the whole document,
/// this fails.
#[test]
fn manual_verification_section_excludes_the_registry_table() {
    let spec = read_spec();
    let section = manual_verification_section(&spec);
    assert!(
        section.len() < spec.len(),
        "the manual-verification section must be a strict suffix of the SPEC, not the whole \
         document — otherwise host_gated_rows_are_loud cannot fail"
    );
    assert!(
        !section.contains("## Layer registry"),
        "the manual-verification section must start AFTER the registry table, which names \
         every LayerId and would make the loudness assertion vacuous"
    );
}

/// Blocker-3: the SAME loudness check as `host_gated_rows_are_loud`, for
/// `MANUAL_SECURITY_ASSUMPTIONS` — cross-cutting assumptions distinct from
/// per-`LayerId` rows.
#[test]
fn security_assumptions_are_loud() {
    let spec_full = read_spec();
    // WR-03: same scoping as `host_gated_rows_are_loud`.
    let spec = manual_verification_section(&spec_full);
    for (name, reason) in MANUAL_SECURITY_ASSUMPTIONS {
        assert!(
            !reason.trim().is_empty(),
            "MANUAL_SECURITY_ASSUMPTIONS entry {name:?} has an empty reason string"
        );
        assert!(
            spec.contains(name),
            "MANUAL_SECURITY_ASSUMPTIONS entry {name:?} does not appear in \
             proj/SPEC-windows-fail-direction-contract.md — Blocker-3 requires this cross-\
             cutting assumption to have a named, cross-referenced home in the SPEC"
        );
    }
    assert!(
        spec.contains("wevtutil gl Application"),
        "expected the exact wevtutil gl Application verification command in the SPEC's manual \
         verification section (Blocker-3 fix)"
    );
}

/// Sanity check on `pascal_to_snake_case`, so a bug in the converter itself
/// cannot produce a false-negative `every_registry_row_has_a_test` pass.
#[test]
fn pascal_to_snake_case_matches_expected_shapes() {
    assert_eq!(pascal_to_snake_case("RestrictedToken"), "restricted_token");
    assert_eq!(
        pascal_to_snake_case("MandatoryIntegrityLabel"),
        "mandatory_integrity_label"
    );
    assert_eq!(
        pascal_to_snake_case("DaclAncestorReadAttrs"),
        "dacl_ancestor_read_attrs"
    );
    assert_eq!(
        pascal_to_snake_case("AppContainerProfile"),
        "app_container_profile"
    );
    assert_eq!(
        pascal_to_snake_case("WfpEgressFilters"),
        "wfp_egress_filters"
    );
}
