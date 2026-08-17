// Phase 118 Plan 03, Task 2 (D-14 half 2 — the behavioral sentinel round-trip).
//
// This file's own frontmatter path is `crates/nono-cli/tests/
// receipt_sentinel_roundtrip.rs`, but the REAL sentinel round-trip test
// lives inline, as a `#[cfg(test)] mod tests` block inside
// `crates/nono-cli/src/exec_strategy_windows/attestation.rs`
// (`sentinel_seeded_session_sid_never_leaks_into_the_serialized_receipt`),
// not in this file.
//
// Why: `nono-cli` has no `[lib]` target (see
// `crates/nono-cli/tests/layer_force_unavailable.rs`'s module doc), so a
// `tests/*.rs` integration test — a SEPARATE compilation unit — cannot call
// `census_from_entries` or `build_enforcement_receipt`, both `pub(crate)`
// items private to the `nono` binary crate. The plan's own `<interfaces>`
// block anticipated this and pre-authorized the inline `#[cfg(test)]`
// placement as the recommended default, with this file kept only as a
// documentation shim so the plan's frontmatter path still resolves to
// something real — not an empty, silently-vacuous test target.
//
// This one signpost test is a discovery-based, non-vacuous proof that the
// real test has not silently disappeared: it reads attestation.rs's source
// text fresh on every run and confirms the sentinel test function still
// exists there, by name, with a real `#[test]` attribute immediately above
// it (not merely mentioned in a doc comment or string).

use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_real_sentinel_round_trip_test_lives_in_attestation_rs() {
    let path = manifest_dir()
        .join("src")
        .join("exec_strategy_windows")
        .join("attestation.rs");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let fn_marker = "fn sentinel_seeded_session_sid_never_leaks_into_the_serialized_receipt";
    let fn_pos = src.find(fn_marker).unwrap_or_else(|| {
        panic!(
            "expected to find `{fn_marker}` in attestation.rs — the D-14 sentinel round-trip \
             test was renamed, removed, or moved; update this signpost (or this plan's \
             SUMMARY.md, if the placement changed) to match"
        )
    });

    // The nearest preceding non-blank, non-doc-comment line above the `fn`
    // must be `#[test]` — proving this is a real, attribute-gated test
    // function, not a mention inside a doc comment or a string literal.
    let preceding = &src[..fn_pos];
    let attr_line = preceding
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    assert_eq!(
        attr_line.trim(),
        "#[test]",
        "expected `#[test]` immediately above `{fn_marker}` in attestation.rs — found {:?}. A \
         mention in a doc comment or elsewhere would satisfy a plain substring search but is \
         not a real test.",
        attr_line.trim()
    );

    assert!(
        src.contains("SENTINEL-TOKEN-7f3a"),
        "expected the sentinel literal substring \"SENTINEL-TOKEN-7f3a\" in attestation.rs — \
         proves the real test seeds an actual sentinel value, not a vacuous check"
    );
}
