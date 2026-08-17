//! Phase 118 Plan 06 (D-27): cross-crate wire-contract name-match discovery
//! test — build-time half of D-27's completeness guard.
//!
//! `nono-shell-broker` structurally cannot import `layer_registry.rs` (it
//! depends only on `nono` core — see `Cargo.toml`), so it has no static
//! per-`LayerId` table of its own for a scan to compare against
//! `layer_registry.rs`'s cells (D-27 explicitly rejected giving it one — a
//! third, drift-prone copy of registry knowledge). What a build-time
//! cross-crate test on the broker CAN meaningfully prove — and what this
//! file implements — is that the broker's source genuinely reads BOTH
//! wire-contract env-var names `crates/nono-cli`'s `attestation.rs` module
//! actually defines, by their exact CURRENT literal string values, catching
//! a rename/typo drift between the two crates. This test deliberately does
//! NOT attempt to re-verify per-`LayerId` wire-completeness — Task 1's
//! partition test (`attestation.rs`'s `broker_wire_contract_tests` module)
//! already proves that, against the real compiled registry data, more
//! reliably than a text scan could (see `118-06-PLAN.md`'s `<interfaces>`
//! block, "On Task 3's design").
//!
//! House discovery-based scan idiom (`crates/nono-cli/tests/
//! layer_registry_selfcheck.rs`): `env!("CARGO_MANIFEST_DIR")` +
//! `std::fs::read_to_string`, no `regex`, no `include_str!` (both source
//! files are read fresh on every run — "a test that names its targets is
//! blind by construction", Phase 115 V-01). Test 1 below never hardcodes
//! either wire-contract env var's string VALUE as a literal anywhere in
//! this file — both are extracted from `attestation.rs`'s source text at
//! test-run time. It does not even hardcode the two
//! `pub(crate) const ..._ENV_VAR` Rust IDENTIFIER names (those are
//! compile-time symbols, not wire content, so the discipline does not
//! strictly require avoiding them — but the scan below discovers every
//! `..._ENV_VAR` const generically, without naming any of them, for the
//! strongest form of this guarantee). Test 2's stand-in strings ARE
//! literal, by design — see that test's own doc comment for why.

use std::path::PathBuf;

/// `crates/nono-shell-broker` — the crate this integration test belongs to.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The sibling `nono-cli` crate's manifest dir
/// (`crates/nono-shell-broker/../nono-cli`), reached via a relative path
/// from this crate's own manifest dir — `nono-shell-broker` has no
/// dependency on `nono-cli` (separate binary, no shared `[lib]` target), so
/// this is a TEST-TIME source-text read only, never a compilation
/// dependency.
fn nono_cli_dir() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap_or_else(|| {
            panic!(
                "CARGO_MANIFEST_DIR {} has no parent",
                manifest_dir().display()
            )
        })
        .join("nono-cli")
}

fn read_attestation_rs() -> String {
    let path = nono_cli_dir()
        .join("src")
        .join("exec_strategy_windows")
        .join("attestation.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn read_broker_main_rs() -> String {
    let path = manifest_dir().join("src").join("main.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Discovers every `pub(crate) const <NAME>_ENV_VAR: &str = "<VALUE>";`
/// declaration in `src` and returns the extracted `<VALUE>` strings, in
/// declaration order. Generic over the const's NAME (does not hardcode
/// `BROKER_REQUIRED_LAYERS_ENV_VAR`/`BROKER_NOT_APPLICABLE_LAYERS_ENV_VAR`
/// either) — any current or future `..._ENV_VAR` wire-contract constant in
/// `attestation.rs` is discovered automatically.
fn extract_env_var_const_values(src: &str) -> Vec<String> {
    const MARKER: &str = "pub(crate) const ";
    let mut out = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = src[search_from..].find(MARKER) {
        let decl_start = search_from + rel;
        let after_marker = &src[decl_start + MARKER.len()..];
        let Some(colon_pos) = after_marker.find(':') else {
            search_from = decl_start + MARKER.len();
            continue;
        };
        let name = after_marker[..colon_pos].trim();
        // Only advance past this declaration's name+colon for the next
        // search iteration, regardless of whether it matches — otherwise a
        // non-matching `pub(crate) const` would infinite-loop this scan.
        search_from = decl_start + MARKER.len() + colon_pos;
        if !name.ends_with("_ENV_VAR") {
            continue;
        }
        // Find this declaration's line (bounded by the next `;`) so the
        // quoted value extracted below cannot accidentally reach into a
        // LATER declaration's string.
        let decl_tail = &src[search_from..];
        let Some(semi_pos) = decl_tail.find(';') else {
            continue;
        };
        let decl_line = &decl_tail[..semi_pos];
        let Some(q1) = decl_line.find('"') else {
            continue;
        };
        let Some(q2_rel) = decl_line[q1 + 1..].find('"') else {
            continue;
        };
        let value = &decl_line[q1 + 1..q1 + 1 + q2_rel];
        out.push(value.to_string());
    }
    out
}

/// Returns `true` iff every value in `values` appears in `src` as the
/// literal string argument to an `std::env::var(...)` call.
fn all_values_read_via_env_var_call(values: &[String], src: &str) -> bool {
    values
        .iter()
        .all(|v| src.contains(&format!("std::env::var(\"{v}\")")))
}

/// Test 1 (discovery-based, positive): every wire-contract `..._ENV_VAR`
/// constant `attestation.rs` currently defines is read, by its exact
/// literal string value, via an `std::env::var(...)` call somewhere in
/// `nono-shell-broker`'s `main.rs`.
#[test]
fn broker_reads_every_attestation_rs_wire_contract_env_var_by_its_current_literal_value() {
    let attestation_src = read_attestation_rs();
    let values = extract_env_var_const_values(&attestation_src);

    // Non-vacuity guard (house discipline): a scanner bug that silently
    // found zero declarations would otherwise let this test pass trivially.
    assert!(
        !values.is_empty(),
        "expected at least one `pub(crate) const ..._ENV_VAR` declaration in attestation.rs — \
         found none; the scan itself is broken (or attestation.rs's declaration shape changed)"
    );
    assert_eq!(
        values.len(),
        2,
        "expected exactly 2 wire-contract env-var constants (D-27's required + \
         not-applicable channels), found {values:?} — either the scan mis-parsed the source, \
         or a third wire-contract channel was added without updating this test's expectation"
    );

    let broker_src = read_broker_main_rs();
    for value in &values {
        assert!(
            broker_src.contains(&format!("std::env::var(\"{value}\")")),
            "attestation.rs defines a wire-contract env var {value:?} that nono-shell-broker's \
             main.rs never reads via `std::env::var({value:?})` — this is exactly the \
             producer/consumer name-drift class this test exists to catch"
        );
    }
}

/// Test 2 (perturbation proof): using SYNTHETIC in-test string literals
/// standing in for the two source files (never a real file mutation),
/// rename one of the two env-var values in the "attestation.rs" stand-in
/// text without updating the "main.rs" stand-in text, and confirm the same
/// matching logic (`extract_env_var_const_values` +
/// `all_values_read_via_env_var_call`) reports the mismatch.
#[test]
fn perturbation_proof_matcher_detects_a_name_drift_between_stand_in_sources() {
    let attestation_stand_in = "\
        pub(crate) const BROKER_REQUIRED_LAYERS_ENV_VAR: &str = \"NONO_BROKER_REQUIRED_LAYERS\";\n\
        pub(crate) const BROKER_NOT_APPLICABLE_LAYERS_ENV_VAR: &str = \"NONO_BROKER_NOT_APPLICABLE_LAYERS\";\n";
    let broker_stand_in_matching = "\
        let required_layers_raw = std::env::var(\"NONO_BROKER_REQUIRED_LAYERS\").map_err(|_| ());\n\
        let not_applicable_raw = std::env::var(\"NONO_BROKER_NOT_APPLICABLE_LAYERS\").unwrap_or_default();\n";

    // Baseline: the matcher agrees with itself on a genuinely-matching pair
    // — proves the matching logic isn't ALWAYS failing (a vacuous-failure
    // guard on the perturbation proof itself).
    let baseline_values = extract_env_var_const_values(attestation_stand_in);
    assert_eq!(
        baseline_values.len(),
        2,
        "stand-in extraction must find both consts"
    );
    assert!(
        all_values_read_via_env_var_call(&baseline_values, broker_stand_in_matching),
        "the matcher must report a MATCH on a genuinely-matching stand-in pair"
    );

    // Perturbation: rename ONE value in the "attestation.rs" stand-in
    // WITHOUT updating the "main.rs" stand-in — a genuine name-drift
    // between the two crates.
    let attestation_stand_in_renamed = attestation_stand_in.replace(
        "NONO_BROKER_NOT_APPLICABLE_LAYERS",
        "NONO_BROKER_NOT_APPLICABLE_LAYERS_V2",
    );
    let renamed_values = extract_env_var_const_values(&attestation_stand_in_renamed);
    assert_eq!(
        renamed_values.len(),
        2,
        "renaming a value must not change how many consts are found"
    );
    assert!(
        !all_values_read_via_env_var_call(&renamed_values, broker_stand_in_matching),
        "the matcher must detect a name drift between the two stand-in sources — this is the \
         perturbation proof that the real Test 1 above is not vacuously passing"
    );
}
