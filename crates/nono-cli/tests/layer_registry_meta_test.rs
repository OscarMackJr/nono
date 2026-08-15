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

mod common;

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
        "MinifilterAbsence",
        "Structurally untestable: no minifilter driver exists in this tree (ADR-65 stands). \
         The row documents a deliberate structural absence, not a probeable mechanism — there \
         is nothing to force unavailable. Record as absent, citing ADR-65, per Phase 116 D-08's \
         structurally-blocked row form.",
    ),
    (
        "BrokerAuthenticodeTrustGate",
        "The gate is skipped entirely under `launch.rs::is_dev_build_layout` — \
         active only in a signed, production (non-dev-layout) install. An ordinary dev/CI host \
         building from `cargo build` cannot exercise it. Manual steps: from a signed release \
         install outside `target/...`, stage a `nono-shell-broker.exe` signed by a different \
         identity than `nono.exe` and confirm the broker-arm spawn refuses \
         (`broker_authenticode.rs::broker_signature_mismatch_refuses_spawn` already covers the \
         underlying `verify_broker_authenticode` logic directly; this item is the live-install \
         end-to-end round-trip).",
    ),
];

/// Phase 117 Plan 18 (SC3 gap closure): `LayerId` rows that have a real,
/// ordinary-host-runnable force-unavailable or negative test SOMEWHERE in
/// the tree, but not under `layer_force_unavailable.rs`'s
/// `fn force_unavailable_<snake>` external-subprocess-spawn convention —
/// that convention is specifically for the black-box, whole-process
/// force-unavailable shape; these 8 rows instead have a direct, in-process
/// unit test that calls the guarded function or the real attestation gate
/// directly. Each entry is (LayerId name, workspace-root-relative file
/// path, function name); `also_automated_entries_are_non_vacuous` below
/// existence-checks every entry's file+function on every run, so a renamed
/// or removed test function fails the build rather than silently continuing
/// to count as coverage (T-117-18-01).
const ALSO_AUTOMATED: &[(&str, &str, &str)] = &[
    (
        "RestrictedToken",
        "crates/nono-cli/src/exec_strategy_windows/restricted_token.rs",
        "create_restricted_token_with_sid_fails_when_forced_unavailable",
    ),
    (
        "JobObjectContainment",
        "crates/nono-cli/src/exec_strategy_windows/launch.rs",
        "apply_process_handle_to_containment_fails_when_forced_unavailable",
    ),
    (
        "DaclAncestorTraverse",
        "crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs",
        "ancestor_traverse_snapshot_and_apply_fails_when_forced_unavailable",
    ),
    (
        "DaclAncestorReadAttrs",
        "crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs",
        "ancestor_read_attributes_snapshot_and_apply_fails_when_forced_unavailable",
    ),
    (
        "WfpEgressFilters",
        "crates/nono-cli/src/exec_strategy_windows/launch.rs",
        "wfp_row_can_be_selected_and_unconfirmed_from_a_real_guard_value",
    ),
    (
        "FirewallRulesEgress",
        "crates/nono-cli/src/exec_strategy_windows/launch.rs",
        "firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate",
    ),
    (
        "AppContainerProfile",
        "crates/nono-shell-broker/src/main.rs",
        "run_fails_when_app_container_forced_unavailable",
    ),
    (
        "InterpreterCoverageGate",
        "crates/nono/src/sandbox/windows.rs",
        "validate_launch_paths_refuses_uncovered_interpreter",
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

/// Word-boundary-exact `fn {name}` search: plain `str::contains` would
/// false-POSITIVE if `{name}` is a prefix of a different, unrelated function
/// (e.g. renaming `foo` to `foo_v2` still contains the substring `fn foo`).
/// Confirmed empirically during this plan's own execution: renaming
/// `firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate` to
/// `..._RENAMED` still satisfied a plain substring check, which would have
/// made `also_automated_entries_are_non_vacuous` vacuous — the exact class
/// of bug T-117-18-01 exists to prevent. Requires the character
/// immediately after the name to be neither an identifier character nor
/// `!` (so `fn foo` matches `fn foo(` and `fn foo<T>` but not `fn foo2`,
/// `fn foobar`, or a macro-shaped `fn foo!` false positive — Phase 117
/// Plan 24 / WR-10 closes this documented-but-previously-unimplemented `!`
/// exclusion).
///
/// Also requires (WR-10) that the match sit at a real definition line: the
/// trimmed text from the start of that line up to the match must be empty
/// or consist entirely of qualifier-keyword words (`pub`, `pub(crate)`,
/// `pub(super)`, `async`, `unsafe`, `const`, or any word starting with
/// `pub(`) — identical in shape to `layer_registry_selfcheck.rs`'s
/// `content_defines_symbol` — so a `fn {name}` spelled out inside a doc
/// comment or a string/macro literal (e.g. a `#[doc = "... fn foo(...) \
/// ..."]` attribute) cannot satisfy this check. A rejected match does not
/// return `false` immediately — scanning continues so a later genuine
/// definition in the same file is still found.
///
/// Phase 117 Plan 31 (WR-13): the trailing-boundary check above is now
/// `common::is_ident_boundary` — the SAME predicate
/// `layer_registry_selfcheck.rs::content_defines_symbol` uses, extracted to
/// `tests/common/mod.rs` so the two matchers cannot drift apart again the
/// way they did between Plan 24 (this file) and the still-unguarded
/// `content_defines_symbol` (closed by this same plan).
fn contains_fn_exact(src: &str, fn_name: &str) -> bool {
    let needle = format!("fn {fn_name}");
    let mut search_start = 0;
    while let Some(rel_idx) = src[search_start..].find(&needle) {
        let match_start = search_start + rel_idx;
        let after = match_start + needle.len();
        let boundary_ok = common::is_ident_boundary(src[after..].chars().next());
        let line_start = src[..match_start]
            .rfind('\n')
            .map_or(0, |newline_pos| newline_pos + 1);
        let prefix = src[line_start..match_start].trim();
        let prefix_ok = prefix.is_empty()
            || prefix.split_whitespace().all(|word| {
                matches!(
                    word,
                    "pub" | "pub(crate)" | "pub(super)" | "async" | "unsafe" | "const"
                ) || word.starts_with("pub(")
            });
        if boundary_ok && prefix_ok {
            return true;
        }
        search_start = match_start + 1;
    }
    false
}

/// D-32: for every `LayerId` NOT on `MANUALLY_VERIFIED`, either a
/// `force_unavailable_<snake_case_name>` function must exist in
/// `layer_force_unavailable.rs`'s source text, OR the row must have a valid
/// `ALSO_AUTOMATED` entry whose cited file (read fresh from disk) contains
/// its cited function (Plan 18 broadening — see `ALSO_AUTOMATED`'s doc
/// comment). Discovery-based: this test reads `LayerId::ALL` fresh from
/// `layer_registry.rs` on every run — it does NOT hardcode the 13 current
/// names. Adding a 14th `LayerId` variant without a corresponding test
/// function (or a `MANUALLY_VERIFIED`/`ALSO_AUTOMATED` entry) fails here.
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
    let workspace_root = workspace_root();

    let mut missing = Vec::new();
    for name in &variant_names {
        if manual_names.contains(&name.as_str()) {
            continue;
        }
        let force_unavailable_fn = pascal_to_snake_case(name);
        if contains_fn_exact(
            &test_src,
            &format!("force_unavailable_{force_unavailable_fn}"),
        ) {
            continue;
        }
        let expected_fn = format!("fn force_unavailable_{force_unavailable_fn}");
        // Broadened discovery (Plan 18, SC3 gap closure): a row not covered
        // by the force_unavailable_* convention may still be covered by an
        // ALSO_AUTOMATED entry — a real, in-process test living outside
        // layer_force_unavailable.rs's external-subprocess convention. Read
        // the cited file FRESH on every run (not cached) so a stale
        // citation is caught here, not silently trusted.
        if let Some((_, file_path, fn_name)) = ALSO_AUTOMATED.iter().find(|(n, _, _)| n == name) {
            let full_path = workspace_root.join(file_path);
            let expected_fn = format!("fn {fn_name}");
            match std::fs::read_to_string(&full_path) {
                Ok(src) if contains_fn_exact(&src, fn_name) => continue,
                Ok(_) => missing.push(format!(
                    "{name} (ALSO_AUTOMATED entry exists but `{expected_fn}` was not found in \
                     {})",
                    full_path.display()
                )),
                Err(e) => missing.push(format!(
                    "{name} (ALSO_AUTOMATED entry cites {} which failed to read: {e})",
                    full_path.display()
                )),
            }
            continue;
        }
        missing.push(format!(
            "{name} (expected `{expected_fn}` in layer_force_unavailable.rs, and no \
             ALSO_AUTOMATED entry)"
        ));
    }

    assert!(
        missing.is_empty(),
        "the following LayerId row(s) have no force_unavailable_* test in \
         layer_force_unavailable.rs, no valid ALSO_AUTOMATED entry, and no MANUALLY_VERIFIED \
         entry in this file — CINT-03: \"a contract entry with no such test is not \
         satisfied\":\n{}",
        missing.join("\n")
    );
}

/// Plan 18 (T-117-18-01): every `ALSO_AUTOMATED` entry's cited file must
/// exist and must contain its cited function name, checked fresh from disk
/// on every run. Without this test, `ALSO_AUTOMATED` would be a second,
/// unchecked `MANUALLY_VERIFIED`-shaped escape hatch — a renamed or removed
/// test function would silently keep passing `every_registry_row_has_a_test`
/// on stale trust instead of failing the build.
#[test]
fn also_automated_entries_are_non_vacuous() {
    let workspace_root = workspace_root();
    let mut failures = Vec::new();

    for (layer_name, file_path, fn_name) in ALSO_AUTOMATED {
        let full_path = workspace_root.join(file_path);
        match std::fs::read_to_string(&full_path) {
            Ok(src) => {
                let expected_fn = format!("fn {fn_name}");
                if !contains_fn_exact(&src, fn_name) {
                    failures.push(format!(
                        "{layer_name}: {} does not contain `{expected_fn}` — the ALSO_AUTOMATED \
                         citation is stale (function renamed or removed)",
                        full_path.display()
                    ));
                }
            }
            Err(e) => failures.push(format!(
                "{layer_name}: failed to read cited file {}: {e}",
                full_path.display()
            )),
        }
    }

    assert!(
        failures.is_empty(),
        "ALSO_AUTOMATED entries with a stale or unreadable citation (fix the citation or the \
         source file, do not silently drop coverage):\n{}",
        failures.join("\n")
    );
}

/// Phase 117 review WR-07: the three coverage buckets must account for every
/// `LayerId`, exactly once, and the arithmetic must be checked by a machine.
///
/// `layer_force_unavailable.rs`'s module doc claimed "this file automates the
/// 3 ..." and "The remaining 10 rows are on `MANUALLY_VERIFIED`". Both
/// numbers were wrong: only two `#[test] fn force_unavailable_*` functions
/// exist (117-44's WR-05 removed a duplicate), and the real split is
/// 2 + 8 `ALSO_AUTOMATED` + 3 `MANUALLY_VERIFIED` = 13. A reader auditing
/// coverage from that doc went looking for a test that does not exist.
///
/// Prose drifts silently; this cannot. It also enforces DISJOINTNESS, which
/// the arithmetic alone would not: a row on both lists would otherwise let a
/// genuinely-uncovered row hide inside a correct-looking total.
#[test]
fn coverage_split_accounts_for_every_layer_id() {
    let registry_src = read_layer_registry();
    let all = extract_all_layer_id_names(&registry_src);

    // Count DEFINITIONS, not mentions: the module doc names the convention
    // repeatedly, and counting those would inflate the total exactly when the
    // functions went missing.
    let force_src = read_force_unavailable_tests();
    let automated = force_src
        .lines()
        .filter(|l| l.trim_start().starts_with("fn force_unavailable_"))
        .count();

    let also: Vec<&str> = ALSO_AUTOMATED.iter().map(|(n, _, _)| *n).collect();
    let manual: Vec<&str> = MANUALLY_VERIFIED.iter().map(|(n, _)| *n).collect();

    for name in &also {
        assert!(
            !manual.contains(name),
            "{name} is on BOTH ALSO_AUTOMATED and MANUALLY_VERIFIED. Double-counting a row \
             lets a genuinely-uncovered row hide inside a correct-looking total (WR-07)."
        );
    }

    assert_eq!(
        automated + also.len() + manual.len(),
        all.len(),
        "WR-07: the coverage buckets do not account for every LayerId exactly once. \
         layer_force_unavailable.rs defines {automated} `fn force_unavailable_*` test(s), \
         ALSO_AUTOMATED has {}, MANUALLY_VERIFIED has {}, and LayerId::ALL has {}. Update \
         the buckets AND layer_force_unavailable.rs's module doc, which states this split in \
         prose.",
        also.len(),
        manual.len(),
        all.len()
    );
}

/// The two coverage-list identifiers, each paired with the row names actually
/// on it.
///
/// Both halves come from the const itself — `stringify!` for the identifier
/// the prose has to spell, and the const's own entries for the membership —
/// in ONE expression, so renaming either const breaks this file's compilation
/// instead of silently un-scoping
/// [`module_doc_assigns_each_claimed_row_to_the_list_it_is_on`] (a gate that
/// scans for a string nobody writes any more passes for the wrong reason).
fn coverage_lists() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        (
            stringify!(MANUALLY_VERIFIED),
            MANUALLY_VERIFIED.iter().map(|(name, _)| *name).collect(),
        ),
        (
            stringify!(ALSO_AUTOMATED),
            ALSO_AUTOMATED.iter().map(|(name, _, _)| *name).collect(),
        ),
    ]
}

/// The module-doc (`//!`) region of `src`, one entry per doc line with the
/// `//!` prefix stripped. A line that is exactly `//!` yields an empty entry,
/// which is what separates paragraphs in [`doc_paragraphs`].
fn module_doc_lines(src: &str) -> Vec<&str> {
    src.lines()
        .filter_map(|line| line.trim_start().strip_prefix("//!"))
        .collect()
}

/// Group [`module_doc_lines`] into paragraphs on blank `//!` lines, joining
/// each paragraph's lines with a single space. Joining is what lets the
/// sentence splitter below see a sentence that was hard-wrapped across
/// several doc lines as one string.
fn doc_paragraphs(doc_lines: &[&str]) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in doc_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(trimmed);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }
    paragraphs
}

/// Abbreviations whose internal `.` must not end a sentence.
///
/// Deliberately short and explicit. An UNLISTED abbreviation over-splits a
/// sentence, and over-splitting can only move a row mention out of reach of
/// the list identifier that precedes it — i.e. into the UNVERIFIED count,
/// where the equality pin in
/// [`module_doc_assigns_each_claimed_row_to_the_list_it_is_on`] fails loudly.
/// It can never turn a wrong claim into a passing one.
const SENTENCE_ABBREVIATIONS: &[&str] = &["e.g", "i.e"];

/// Split one joined paragraph into sentences.
///
/// A `.` terminates a sentence only when the character after it is whitespace
/// or the paragraph ends there — which already excludes `mod.rs`, `nono.exe`,
/// a `file.rs::Symbol` citation and a trailing `.)`/`."` — and only when the
/// word ending at that `.` is not in [`SENTENCE_ABBREVIATIONS`]. The end of
/// the paragraph is always a sentence end too, so a heading paragraph with no
/// terminal period is still one sentence rather than being dropped.
fn sentences(paragraph: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (idx, ch) in paragraph.char_indices() {
        if ch != '.' {
            continue;
        }
        let terminates = paragraph[idx + 1..]
            .chars()
            .next()
            .is_none_or(char::is_whitespace);
        if !terminates {
            continue;
        }
        let mut word: Vec<char> = paragraph[..idx]
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '.')
            .collect();
        word.reverse();
        let word: String = word.into_iter().collect();
        if SENTENCE_ABBREVIATIONS.contains(&word.as_str()) {
            continue;
        }
        let sentence = paragraph[start..=idx].trim();
        if !sentence.is_empty() {
            out.push(sentence);
        }
        start = idx + 1;
    }
    let tail = paragraph[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// Byte offsets of every occurrence of `word` in `haystack` that sits at
/// identifier boundaries on BOTH sides.
///
/// The trailing boundary is `common::is_ident_boundary` — the same predicate
/// `contains_fn_exact` above and `layer_registry_selfcheck.rs`'s
/// `content_defines_symbol` use, so this file grows no second boundary rule.
/// The leading side applies the same truth table minus the `!` case, which
/// cannot precede an identifier.
fn word_occurrences(haystack: &str, word: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut search_start = 0usize;
    while let Some(rel_pos) = haystack[search_start..].find(word) {
        let pos = search_start + rel_pos;
        let before_ok = haystack[..pos]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let after_ok = common::is_ident_boundary(haystack[pos + word.len()..].chars().next());
        if before_ok && after_ok {
            out.push(pos);
        }
        search_start = pos + 1;
    }
    out
}

/// Phase 117 SC4 gap closure: `layer_force_unavailable.rs`'s module doc
/// DECLARES, in prose, which coverage list each `LayerId` row is on. This
/// gate checks that declaration against the lists themselves.
///
/// `coverage_split_accounts_for_every_layer_id` above enforces the ARITHMETIC
/// (2 + 8 + 3 = 13) and DISJOINTNESS, and is structurally blind to WHICH list
/// the prose names — which is precisely how that doc came to state that two
/// `ALSO_AUTOMATED` rows were on `MANUALLY_VERIFIED`, sending an auditor
/// hunting for manual reproduction steps that were not those rows' coverage
/// of record. That is the same class Iteration 4's WR-07 fixed once in this
/// same file.
///
/// # The association rule
///
/// > Within the module-doc (`//!`) region, for each occurrence of a `LayerId`
/// > name at identifier boundaries, look BACKWARDS within the SAME SENTENCE
/// > for the nearest coverage-list identifier. If one is found, that is a
/// > CLAIM and the row must actually be on that list. If the sentence names no
/// > coverage list before the row, the mention makes no assignment claim: it
/// > is UNVERIFIED, and counted.
///
/// It is written once, here, and implemented once, below. This phase has
/// already shipped two mirrors of one rule with contradictory content that
/// both passed every gate; a second variant of this rule anywhere is a defect.
///
/// # Why SENTENCE-scoped and nearest-preceding, not paragraph-scoped
///
/// A paragraph-scoped rule would have to skip any paragraph naming both
/// lists — and the `Of the 13 LayerId rows` paragraph, which carries 11 of the
/// doc's 18 row mentions and ALL of its primary per-row assignment claims,
/// names both. Such a rule would check 2 mentions out of 18 and be blind in
/// exactly the region WR-07 already had to correct once. Worse, it would hand
/// an author a silent escape hatch: adding the second list name to a paragraph
/// makes a failing assertion disappear. Under the sentence-scoped
/// nearest-preceding rule there is no ambiguity case and therefore no escape
/// hatch — naming both lists in one sentence binds each row to whichever
/// identifier precedes it, and moving a row name away from its list name
/// shows up in the unverified-mention pin below.
///
/// # Scope: the `//!` region only
///
/// The claim being checked is a MODULE-DOC claim about the coverage split.
/// Widening the scan to the whole file would sweep in the per-test `///` doc
/// comments, which describe one test each and make no split claim, plus the
/// test bodies' own assertion strings.
///
/// # Residual, stated rather than hidden
///
/// This gate verifies per-row list assignments made in sentences that name a
/// list. It does NOT verify the row mentions in the "reasons below cover rows
/// on BOTH lists" sentence (which deliberately enumerates rows from both lists
/// in one breath and makes no per-row assignment), the two in the
/// late-checked-seams section HEADING, or the one inside the quoted
/// `LayerAttestationFailed` diagnostic. Those make no assignment claim; they
/// are held CONSTANT by the equality pin below, not checked. Task 3's SC4
/// disposition in `117-VERIFICATION.md` states the same limit, so the record
/// does not overclaim this gate's reach.
#[test]
fn module_doc_assigns_each_claimed_row_to_the_list_it_is_on() {
    let registry_src = read_layer_registry();
    let row_names = extract_all_layer_id_names(&registry_src);
    assert!(
        row_names.len() >= 13,
        "expected at least 13 LayerId variants (the 117-01 inventory), found {}: {row_names:?}",
        row_names.len()
    );

    let lists = coverage_lists();
    let doc = read_force_unavailable_tests();
    let doc_lines = module_doc_lines(&doc);
    let paragraphs = doc_paragraphs(&doc_lines);

    let mut wrong: Vec<String> = Vec::new();
    let mut verified: Vec<String> = Vec::new();
    let mut unverified: Vec<String> = Vec::new();

    for paragraph in &paragraphs {
        for sentence in sentences(paragraph) {
            let mut list_positions: Vec<(usize, &str)> = Vec::new();
            for (list_name, _) in &lists {
                for pos in word_occurrences(sentence, list_name) {
                    list_positions.push((pos, list_name));
                }
            }
            for row in &row_names {
                for pos in word_occurrences(sentence, row) {
                    let Some((_, claimed)) = list_positions
                        .iter()
                        .filter(|(list_pos, _)| *list_pos < pos)
                        .max_by_key(|(list_pos, _)| *list_pos)
                    else {
                        unverified.push(format!("{row} — in sentence: {sentence:?}"));
                        continue;
                    };
                    let actual: Vec<&str> = lists
                        .iter()
                        .filter(|(_, members)| members.contains(&row.as_str()))
                        .map(|(list_name, _)| *list_name)
                        .collect();
                    if actual.contains(claimed) {
                        verified.push(format!("{row} -> {claimed}"));
                    } else {
                        let real = if actual.is_empty() {
                            "NEITHER coverage list (it is covered by this file's own \
                             `fn force_unavailable_*` convention, or by nothing at all)"
                                .to_string()
                        } else {
                            actual.join(" and ")
                        };
                        wrong.push(format!(
                            "`{row}` is claimed to be on `{claimed}`, but it is on {real}\n    \
                             offending sentence: {sentence:?}"
                        ));
                    }
                }
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "the module doc of layer_force_unavailable.rs assigns {} row(s) to a coverage list they \
         are not on. A coverage declaration that names the wrong list sends an auditor hunting \
         for a test that does not exist (Iteration 4's WR-07, same file):\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );

    // TWO PINS. Narrowing the scan is the fail-OPEN direction here, so both
    // directions are pinned, following this file's existing floor idiom.
    assert!(
        verified.len() >= 5,
        "only {} module-doc list-assignment claim(s) were verifiable, expected at least 5: the \
         three rows the coverage-split sentence assigns to one list, plus the two rows the \
         late-checked-seams paragraph assigns to the other. Claims found: {verified:?}. A DROP \
         means a claim was reworded OUT of this gate's reach — the list identifier moved to a \
         different sentence, or the row names were replaced by an anaphor such as \"both rows\" \
         — which is exactly how the defect this gate closes survived unnoticed. Put the row back \
         in a sentence that names its list.",
        verified.len()
    );
    assert_eq!(
        unverified.len(),
        13,
        "the number of module-doc row mentions this gate cannot verify changed from 13 to {}. \
         These mentions sit in sentences that name no coverage list before them, so they are \
         UNVERIFIED — not merely unclaimed. An INCREASE means a new row mention was written \
         where the gate cannot check it; a DECREASE means a mention was deleted, or a real claim \
         was demoted into unverifiable prose. Either put the row in a sentence that names its \
         list (preferred), or move this pin deliberately and say why. Unverified mentions \
         found:\n  {}",
        unverified.len(),
        unverified.join("\n  ")
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

/// WR-10: a `fn {name}` spelled out only inside a doc comment must not
/// satisfy `contains_fn_exact` — it requires a real definition-line prefix.
#[test]
fn contains_fn_exact_rejects_doc_comment_mention() {
    let src = "/// see `fn probe_target(...)`\n";
    assert!(
        !contains_fn_exact(src, "probe_target"),
        "a doc-comment mention of `fn probe_target` must not satisfy contains_fn_exact"
    );
}

/// WR-10: the documented `!` boundary exclusion — `fn probe_target!` is a
/// macro-shaped false positive, not a real function definition.
#[test]
fn contains_fn_exact_rejects_bang_suffix() {
    let src = "fn probe_target! ";
    assert!(
        !contains_fn_exact(src, "probe_target"),
        "`fn probe_target!` must not satisfy contains_fn_exact — the trailing `!` boundary \
         exclusion is documented but was previously unimplemented (WR-10)"
    );
}

/// WR-10: a rejected match (here, a doc-comment mention) must not short-
/// circuit the scan — a later genuine definition in the same file must
/// still be found.
#[test]
fn contains_fn_exact_accepts_real_definition_after_rejecting_a_false_positive() {
    let src = "/// see `fn probe_target(...)`\npub(crate) fn probe_target() {}\n";
    assert!(
        contains_fn_exact(src, "probe_target"),
        "a real `pub(crate) fn probe_target(` definition later in the file must still be found \
         after an earlier doc-comment mention is rejected"
    );
}
