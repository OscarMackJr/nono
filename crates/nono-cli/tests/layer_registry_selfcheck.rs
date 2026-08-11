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

mod common;

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

/// Phase 117 Plan 31 (WR-13): is `word` present in `haystack` as a whole
/// identifier — not as a substring of a longer one? Used to check whether an
/// `impl` line's type name is the exact `ty` being resolved for
/// (`impl FirewallRulesNetworkBackend` must not satisfy a search for
/// `NetworkBackend`). Reuses `common::is_ident_boundary` on both sides of the
/// match: the character before must not be an identifier character either
/// (checked directly, since `is_ident_boundary`'s truth table — "not
/// alphanumeric/`_`/`!`" — is symmetric and applies the same going backward
/// as forward, minus the `!` case which cannot precede an identifier).
fn line_contains_word(haystack: &str, word: &str) -> bool {
    let mut search_start = 0usize;
    while let Some(rel_pos) = haystack[search_start..].find(word) {
        let match_start = search_start + rel_pos;
        let before_ok = haystack[..match_start]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let after = match_start + word.len();
        let after_ok = common::is_ident_boundary(haystack[after..].chars().next());
        if before_ok && after_ok {
            return true;
        }
        search_start = match_start + 1;
    }
    false
}

/// Phase 117 Plan 31 (WR-13): for a `Type::method` citation, does the
/// nearest preceding `impl` block — scanning backward line-by-line from
/// `match_pos` — name `ty`? Walks `content[..match_pos]`'s lines in reverse
/// order (this naturally includes the partial line containing `match_pos`
/// itself, so a single-line `impl Foo { fn bar() {} }` shape resolves
/// correctly too, since the trimmed partial line up to the match already
/// starts with `"impl "`). The first line whose trimmed text starts with
/// `"impl "` decides the answer; if no such line exists before `match_pos`,
/// there is no enclosing impl to resolve against and the citation is
/// rejected.
fn nearest_preceding_impl_names(content: &str, match_pos: usize, ty: &str) -> bool {
    let before = &content[..match_pos];
    for line in before.lines().rev() {
        if let Some(rest) = line.trim_start().strip_prefix("impl ") {
            return line_contains_word(rest, ty);
        }
    }
    false
}

/// Phase 117 Plan 24 (WR-08), hardened by Plan 31 (WR-13): does `content`
/// define `symbol` at a real definition site, not merely mention it
/// somewhere (a doc comment, a string/macro literal, a commented-out line,
/// or — the WR-13 gap — a differently-named function/method that happens to
/// share `symbol`'s name as a PREFIX)?
///
/// `symbol` is split on its LAST `::` into `method` (searched for) and,
/// where present, `ty` — the type qualifier of a `Type::method` citation
/// (`None` for a bare symbol). Taking only the second `rsplit` segment as
/// `ty` does not support a longer qualifier chain (`mod::Type::method`);
/// none of this codebase's current citations need one, but a future one
/// that does would need this resolution extended, not silently mis-resolved.
///
/// Searches for both `"fn {method}"` and `"impl {method}"` needles. For each
/// occurrence:
/// - the PREFIX (trimmed text from the start of that line up to the match)
///   must be empty or consist entirely of qualifier-keyword words (`pub`,
///   `pub(crate)`, `pub(super)`, `async`, `unsafe`, `const`, or any word
///   starting with `pub(`) — unchanged from Plan 24;
/// - the SUFFIX (the character immediately after the needle) must be a
///   valid identifier boundary per `common::is_ident_boundary` — the WR-13
///   fix: without this, `fn create_restricted_token_with_sid_v2` satisfies a
///   search for `create_restricted_token_with_sid`, since the needle is only
///   a PREFIX of the real (renamed) identifier;
/// - when `ty.is_some()`, the match must additionally sit inside the
///   nearest preceding `impl ty` block (`nearest_preceding_impl_names`) —
///   the WR-13 fix for `Type::method` citations: without it,
///   `WfpNetworkBackend::install` and `FirewallRulesNetworkBackend::install`
///   both reduce to the bare needle `fn install` and become
///   indistinguishable.
///
/// A rejected match at any of these three gates is skipped, not treated as
/// a failure — scanning continues so a later genuine match in the same file
/// is still found.
fn content_defines_symbol(content: &str, symbol: &str) -> bool {
    let mut parts = symbol.rsplit("::");
    let method = parts.next().unwrap_or(symbol);
    let ty = parts.next();
    let needles = [format!("fn {method}"), format!("impl {method}")];

    for needle in &needles {
        let mut search_start = 0usize;
        while let Some(rel_pos) = content[search_start..].find(needle.as_str()) {
            let match_pos = search_start + rel_pos;
            let after = match_pos + needle.len();
            let line_start = content[..match_pos]
                .rfind('\n')
                .map_or(0, |newline_pos| newline_pos + 1);
            let prefix = content[line_start..match_pos].trim();
            let is_definition_prefix = prefix.is_empty()
                || prefix.split_whitespace().all(|word| {
                    matches!(
                        word,
                        "pub" | "pub(crate)" | "pub(super)" | "async" | "unsafe" | "const"
                    ) || word.starts_with("pub(")
                });
            let boundary_ok = common::is_ident_boundary(content[after..].chars().next());
            let impl_ok = match ty {
                Some(ty) => nearest_preceding_impl_names(content, match_pos, ty),
                None => true,
            };
            if is_definition_prefix && boundary_ok && impl_ok {
                return true;
            }
            // Not a real definition line, not a trailing boundary, or not
            // inside the right impl block — keep scanning past this
            // rejected match instead of giving up on the whole needle.
            search_start = match_pos + 1;
        }
    }

    false
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
    let symbol_citations = extract_symbol_citations(&src);

    // Phase 117 Plan 24 converted every remaining raw "file:line" citation
    // to symbol form (SC1 / CINT-01's 9-of-13 gap), so `citations` alone is
    // now expected to be empty — the extraction-is-broken sanity check must
    // look at the combined total instead of raw-form citations specifically.
    assert!(
        !citations.is_empty() || !symbol_citations.is_empty(),
        "expected at least one call_sites citation (\"file:line\" or \"file.rs::Symbol\") in \
         layer_registry.rs — found zero of either shape; the extraction scan may be broken, or \
         the registry lost its citations"
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
    let mut missing_symbols = Vec::new();
    for citation in &symbol_citations {
        let (file_part, symbol) = split_symbol_citation(citation);
        let resolved = resolve_file_part(file_part);
        match std::fs::read_to_string(&resolved) {
            Ok(content) if content_defines_symbol(&content, symbol) => {}
            Ok(_) => missing_symbols.push(format!(
                "{citation:?} -> resolved to {}, but its content does not define {symbol:?} at a \
                 real definition site (only a comment, string, or macro literal mention, if any)",
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

/// WR-08: a doc-comment mention of a symbol must not satisfy
/// `content_defines_symbol` — only a real `fn`/`impl` definition line does.
#[test]
fn content_defines_symbol_rejects_a_doc_comment_mention() {
    let src = "/// see `fn foo(...)` for details\n";
    assert!(
        !content_defines_symbol(src, "foo"),
        "a doc-comment mention of `fn foo` must not satisfy content_defines_symbol"
    );
}

/// WR-08: a real, qualifier-prefixed definition line must satisfy
/// `content_defines_symbol`.
#[test]
fn content_defines_symbol_accepts_a_real_definition() {
    let src = "pub(crate) fn foo(x: u32) -> u32 {\n    x\n}\n";
    assert!(
        content_defines_symbol(src, "foo"),
        "a real `pub(crate) fn foo(` definition line must satisfy content_defines_symbol"
    );
}

/// WR-13: a prefix-preserving rename must NOT satisfy `content_defines_symbol`
/// for the original name — the regression this plan closes, confirmed
/// empirically against `nono-shell-broker/src/main.rs` in the perturbation
/// proof recorded in this plan's SUMMARY.md (`::run` vs.
/// `run_fails_when_app_container_forced_unavailable`).
#[test]
fn content_defines_symbol_rejects_a_prefix_preserving_rename() {
    let src = "pub(crate) fn create_restricted_token_with_sid_v2(x: u32) -> u32 {\n    x\n}\n";
    assert!(
        !content_defines_symbol(src, "create_restricted_token_with_sid"),
        "`fn create_restricted_token_with_sid_v2` (a prefix-preserving rename) must not satisfy \
         content_defines_symbol(\"create_restricted_token_with_sid\") — WR-13"
    );
}

/// WR-13: two same-named methods in different `impl` blocks in the same file
/// must be distinguishable by their `Type::method` qualifier — modeled on
/// the real `WfpNetworkBackend::install` / `FirewallRulesNetworkBackend::
/// install` pair the review cited.
#[test]
fn content_defines_symbol_distinguishes_same_named_methods_in_different_impls() {
    let wrong_impl_only =
        "impl FirewallRulesNetworkBackend {\n    pub(crate) fn install(&self) {}\n}\n";
    assert!(
        !content_defines_symbol(wrong_impl_only, "WfpNetworkBackend::install"),
        "a `fn install` defined only inside `impl FirewallRulesNetworkBackend` must not satisfy \
         the qualified citation `WfpNetworkBackend::install` — WR-13"
    );

    let both_impls = "impl FirewallRulesNetworkBackend {\n    pub(crate) fn install(&self) {}\n}\n\
                       \nimpl WfpNetworkBackend {\n    pub(crate) fn install(&self) {}\n}\n";
    assert!(
        content_defines_symbol(both_impls, "WfpNetworkBackend::install"),
        "the correct `impl WfpNetworkBackend {{ fn install }}` must satisfy \
         `WfpNetworkBackend::install` even when a same-named method exists in a different impl \
         in the same file — WR-13"
    );
}

/// WR-13 positive case: an ordinary `Type::method` citation against its real
/// definition must succeed — the hardening must not turn into a false
/// negative for the legitimate, common shape.
#[test]
fn content_defines_symbol_accepts_a_qualified_type_method_citation() {
    let src = "impl RealType {\n    pub(crate) fn real_method() {}\n}\n";
    assert!(
        content_defines_symbol(src, "RealType::real_method"),
        "a real `impl RealType {{ pub(crate) fn real_method() {{}} }}` must satisfy the \
         qualified citation `RealType::real_method`"
    );
}
