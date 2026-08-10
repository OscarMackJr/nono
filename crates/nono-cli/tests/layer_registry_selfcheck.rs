//! Phase 117 Plan 03 (CINT-01, D-01/D-32): keeps the code-resident Windows
//! fail-direction layer registry
//! (`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs`) and the
//! human-readable contract (`proj/SPEC-windows-fail-direction-contract.md`)
//! honest against each other and against the tree they cite.
//!
//! Source-scan based (no `regex`, no `include_str!` — the house pattern is
//! `env!("CARGO_MANIFEST_DIR")` + `std::fs::read_to_string`, confirmed by
//! `crates/nono-cli/tests/resl_supervisor_drain.rs`):
//!
//! - `registry_call_sites_exist`: every `"file:line"` string literal cited
//!   in `layer_registry.rs`'s `call_sites` arrays names a file that actually
//!   exists in this tree. Catches stale citations as the code moves. Phase
//!   117 gap closure (NR3-08) extended this test: every `"file.rs::Symbol"`
//!   citation additionally must resolve to a file whose CONTENT contains the
//!   cited symbol — content-verified, not merely existence-verified, so a
//!   renamed or removed enforcing function fails the build instead of
//!   silently invalidating the citation.
//! - `spec_matches_registry`: every `LayerId` variant named in the
//!   registry's `ALL` const also appears, by name, somewhere in the SPEC
//!   document's text. This is the drift gate D-01 requires: a `LayerId`
//!   variant added to the registry without a corresponding SPEC row fails
//!   this test — the registry wins, and drift fails a build, not a review.
//!
//! All tests are discovery-based (Phase 115 V-01 lesson: "a test that names
//! its targets is blind by construction") — neither hardcodes the list of 13
//! `LayerId` variants; both parse it fresh out of `layer_registry.rs` on
//! every run.

use std::path::PathBuf;

/// `crates/nono-cli` — the crate this integration test belongs to.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The workspace root (`crates/nono-cli/../..`), needed because several
/// `call_sites` citations in `layer_registry.rs` are workspace-root-relative
/// (e.g. `"crates/nono/src/sandbox/windows.rs:2247"`) or relative to a
/// sibling crate (e.g. `"nono-shell-broker/src/main.rs:322-336"`), not all
/// relative to `exec_strategy_windows/` itself.
fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR {} has no parent", manifest_dir().display()))
        .parent()
        .unwrap_or_else(|| {
            panic!(
                "CARGO_MANIFEST_DIR {} has no grandparent (expected crates/nono-cli under a workspace root)",
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

fn read_spec() -> String {
    let path = workspace_root()
        .join("proj")
        .join("SPEC-windows-fail-direction-contract.md");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Extract every `"file:line"`-shaped double-quoted Rust string literal from
/// `layer_registry.rs`'s source text. A plain substring scan, not a full
/// parser: this file's `call_sites: &[...]` arrays are the only place
/// double-quoted string literals containing a real `.rs:<digit>` citation
/// appear. Doc-comment call site citations normally use backtick code
/// spans, e.g. `` `network.rs:1730` `` (no double quotes at all), but one
/// doc comment illustrates the *shape* with a literal, non-numeric example
/// (`` `"file:line"` ``) — backticks wrapping an actual double-quoted
/// string. Requiring the character immediately after the final `:` to be an
/// ASCII digit excludes that illustrative example while keeping every real
/// citation (all of which end in `<digits>` or `<digits>-<digits>`).
fn extract_call_site_citations(src: &str) -> Vec<String> {
    let mut citations = Vec::new();
    let mut rest = src;
    while let Some(start) = rest.find('"') {
        let after_open = &rest[start + 1..];
        let Some(end) = after_open.find('"') else {
            break;
        };
        let literal = &after_open[..end];
        if literal.contains(".rs:") {
            let looks_like_a_real_citation = literal
                .rsplit_once(':')
                .is_some_and(|(_file, line)| line.starts_with(|c: char| c.is_ascii_digit()));
            if looks_like_a_real_citation {
                citations.push(literal.to_string());
            }
        }
        rest = &after_open[end + 1..];
    }
    citations
}

/// Resolve a bare `file_part` (e.g. `"mod.rs"`, `"crates/nono/src/sandbox/
/// windows.rs"`, `"nono-shell-broker/src/main.rs"`,
/// `"agent_daemon/launch.rs"`) to the workspace-relative source file it
/// names, applying the path conventions `layer_registry.rs`'s own citations
/// use (documented on `LayerRegistryEntry::call_sites`): a bare filename is
/// relative to `exec_strategy_windows/`; a `"crates/..."`-prefixed part is
/// workspace-root-relative; a `"nono-shell-broker/..."` part is relative to
/// `crates/`; an `"agent_daemon/..."` part is relative to
/// `crates/nono-cli/src/`. Shared by both the line-form
/// (`resolve_citation_path`) and symbol-form (`registry_call_sites_exist`)
/// citation resolvers so the four-way prefix match is not duplicated.
fn resolve_file_part(file_part: &str) -> PathBuf {
    if let Some(rest) = file_part.strip_prefix("crates/") {
        workspace_root().join("crates").join(rest)
    } else if let Some(rest) = file_part.strip_prefix("nono-shell-broker/") {
        workspace_root()
            .join("crates")
            .join("nono-shell-broker")
            .join(rest)
    } else if let Some(rest) = file_part.strip_prefix("agent_daemon/") {
        manifest_dir().join("src").join("agent_daemon").join(rest)
    } else {
        manifest_dir()
            .join("src")
            .join("exec_strategy_windows")
            .join(file_part)
    }
}

/// Resolve a `"file:line"` or `"file:line-line"` citation string to the
/// workspace-relative source file it names.
fn resolve_citation_path(citation: &str) -> PathBuf {
    let (file_part, _line_part) = citation
        .rsplit_once(':')
        .unwrap_or_else(|| panic!("citation {citation:?} has no ':' separating file from line"));
    resolve_file_part(file_part)
}

/// Extract every `"file.rs::Symbol"`-shaped double-quoted Rust string
/// literal from `layer_registry.rs`'s source text (Phase 117 gap closure,
/// NR3-08). A citation is symbol-form if it contains the literal substring
/// `".rs::"` — the double-colon distinguishes it from the line-form
/// `".rs:<digit>"` citations `extract_call_site_citations` finds. Same
/// plain-substring-scan shape as that function, not a full parser.
fn extract_symbol_citations(src: &str) -> Vec<String> {
    let mut citations = Vec::new();
    let mut rest = src;
    while let Some(start) = rest.find('"') {
        let after_open = &rest[start + 1..];
        let Some(end) = after_open.find('"') else {
            break;
        };
        let literal = &after_open[..end];
        if literal.contains(".rs::") {
            citations.push(literal.to_string());
        }
        rest = &after_open[end + 1..];
    }
    citations
}

/// Split a `"file.rs::Symbol"` citation into its file and symbol halves at
/// the FIRST `"::"` — not the last, since the symbol half may itself
/// contain further `::` for `Type::method` notation (e.g.
/// `"dacl_guard.rs::AppliedDaclGrantsGuard::snapshot_and_apply"` must split
/// into `("dacl_guard.rs", "AppliedDaclGrantsGuard::snapshot_and_apply")`,
/// not split again on the method's own `::`).
fn split_symbol_citation(citation: &str) -> (&str, &str) {
    citation.split_once("::").unwrap_or_else(|| {
        panic!("symbol citation {citation:?} has no '::' separating file from symbol")
    })
}

/// CINT-01: every `call_sites` citation in the registry names a file that
/// actually exists in this tree.
///
/// // TODO(117-12): this asserts file EXISTENCE, the hard gate — it does not
/// // verify the cited line NUMBER still points at the right code. Line-drift
/// // is a softer, known risk this test (and the D-32 meta-test) does not
/// // fully close; a file can still exist while a citation's line number has
/// // gone stale under it.
#[test]
fn registry_call_sites_exist() {
    let src = read_layer_registry();
    let citations = extract_call_site_citations(&src);

    assert!(
        !citations.is_empty(),
        "expected at least one \"file:line\" call_sites citation in layer_registry.rs — \
         found zero; the extraction scan may be broken, or the registry lost its citations"
    );

    let mut missing = Vec::new();
    for citation in &citations {
        let resolved = resolve_citation_path(citation);
        if !resolved.is_file() {
            missing.push(format!(
                "{citation:?} -> resolved to {}",
                resolved.display()
            ));
        }
    }

    assert!(
        missing.is_empty(),
        "layer_registry.rs cites call_sites naming files that do not exist in this tree \
         (stale citation — the code moved and the registry row was not updated):\n{}",
        missing.join("\n")
    );

    // Phase 117 gap closure (NR3-08): symbol-form ("file.rs::Symbol")
    // citations must resolve to a file whose CONTENT contains the cited
    // symbol — content-verified, not merely existence-verified, so a
    // renamed or removed enforcing function fails the build instead of
    // silently invalidating the citation.
    let symbol_citations = extract_symbol_citations(&src);
    let mut missing_symbols = Vec::new();
    for citation in &symbol_citations {
        let (file_part, symbol) = split_symbol_citation(citation);
        let resolved = resolve_file_part(file_part);
        match std::fs::read_to_string(&resolved) {
            Ok(content) if content.contains(symbol) => {}
            Ok(_) => missing_symbols.push(format!(
                "{citation:?} -> resolved to {}, but its content does not contain {symbol:?}",
                resolved.display()
            )),
            Err(e) => missing_symbols.push(format!(
                "{citation:?} -> resolved to {} (unreadable: {e})",
                resolved.display()
            )),
        }
    }

    assert!(
        missing_symbols.is_empty(),
        "layer_registry.rs cites symbol-form call_sites naming a symbol that no longer \
         exists in the cited file (renamed or removed — the citation is stale):\n{}",
        missing_symbols.join("\n")
    );
}

/// Non-vacuity proof (Phase 117 gap closure, NR3-08): after Task 2's
/// conversion, at least 8 symbol-form citations are discoverable in
/// `layer_registry.rs`'s source text — proves `extract_symbol_citations`
/// actually finds real data, not zero matches passing vacuously. A floor,
/// not an exact count, so a future symbol-form citation does not force this
/// test to be edited every time one is added.
#[test]
fn symbol_citation_extraction_finds_the_eight_converted_citations() {
    let src = read_layer_registry();
    let symbol_citations = extract_symbol_citations(&src);
    assert!(
        symbol_citations.len() >= 8,
        "expected at least 8 symbol-form (\"file.rs::Symbol\") call_sites citations in \
         layer_registry.rs (NR3-08 converted 8) — found {}: {symbol_citations:?}",
        symbol_citations.len()
    );
}

/// Extract the identifier list inside `layer_registry.rs`'s
/// `pub(crate) const ALL: &[LayerId] = &[ ... ];` block — the flat,
/// declaration-order list of every current `LayerId` variant name.
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

/// CINT-01/D-01: the drift gate. Every `LayerId` variant listed in the
/// registry's `ALL` const must appear, by name, somewhere in the SPEC
/// document's text — a variant added to the registry without a
/// corresponding SPEC row fails here, not in a human review.
#[test]
fn spec_matches_registry() {
    let registry_src = read_layer_registry();
    let spec_text = read_spec();

    let variant_names = extract_all_layer_id_names(&registry_src);

    assert!(
        variant_names.len() >= 13,
        "expected at least 13 LayerId variants in layer_registry.rs's ALL const \
         (the 13-row Phase 117 Plan 01 inventory), found {}: {variant_names:?}",
        variant_names.len()
    );

    let mut missing = Vec::new();
    for name in &variant_names {
        if !spec_text.contains(name.as_str()) {
            missing.push(name.clone());
        }
    }

    assert!(
        missing.is_empty(),
        "proj/SPEC-windows-fail-direction-contract.md is missing a row for the following \
         LayerId variant(s) present in layer_registry.rs's ALL const: {missing:?}\n\
         \n\
         D-01: the registry is the source of truth — every registry row must have a \
         corresponding SPEC row, or the SPEC has drifted."
    );
}

/// Sanity check on the extraction helper itself: confirms
/// `extract_call_site_citations` does not also pick up backtick code-span
/// citations from doc comments (which would make `registry_call_sites_exist`
/// pass vacuously against text that was never a real `call_sites` entry).
#[test]
fn call_site_extraction_ignores_backtick_doc_comment_citations() {
    let sample = r#"
        //! See `network.rs:9999` for background (this is a doc comment, not data).
        //! `"file:line"` citations of the enforcing call site(s).
        LayerRegistryEntry {
            call_sites: &["restricted_token.rs:55", "launch.rs:1403"],
            ..
        }
    "#;
    let citations = extract_call_site_citations(sample);
    assert_eq!(
        citations,
        vec![
            "restricted_token.rs:55".to_string(),
            "launch.rs:1403".to_string()
        ],
        "extraction picked up a backtick doc-comment citation, or missed a real \
         double-quoted call_sites entry: {citations:?}"
    );
}
