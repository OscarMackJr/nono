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

/// The `call_sites: &[ ... ]` list bodies in `layer_registry.rs`, as raw
/// source slices.
///
/// # Why the extractors are scoped instead of scanning the whole file
///
/// They used to scan EVERY double-quoted literal in `layer_registry.rs`, on
/// the stated assumption that "this file's `call_sites: &[...]` arrays are
/// the only place double-quoted string literals containing a real citation
/// appear". That assumption held only by luck, and stopped holding the
/// moment this phase's fix pass added assertion messages to the file's own
/// `#[cfg(test)]` module that legitimately name symbols in prose
/// (`attestation.rs::classify_row`, and a `"file.rs::Symbol"` shape
/// example). Those were scraped as citations and "resolved" to paths like
/// `exec_strategy_windows/ProbeKind`, failing the build for a citation
/// nobody wrote.
///
/// Scoping to the `call_sites` lists is what this test always meant. The
/// narrowing is the fail-OPEN direction if it ever goes quiet, so
/// `registry_call_sites_exist` asserts a floor on both the number of lists
/// found and the number of citations in them.
fn call_sites_regions(src: &str) -> Vec<&str> {
    const MARKER: &str = "call_sites: &[";
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(start) = rest.find(MARKER) {
        let after = &rest[start + MARKER.len()..];
        let Some(end) = after.find(']') else {
            break;
        };
        out.push(&after[..end]);
        rest = &after[end + 1..];
    }
    out
}

/// Extract every `"file:line"`-shaped double-quoted Rust string literal from
/// `layer_registry.rs`'s `call_sites` lists. A plain substring scan, not a
/// full parser.
///
/// Requiring the character immediately after the final `:` to be an ASCII
/// digit excludes shape-illustrating examples while keeping every real
/// citation (all of which end in `<digits>` or `<digits>-<digits>`).
fn extract_call_site_citations(src: &str) -> Vec<String> {
    let mut citations = Vec::new();
    for region in call_sites_regions(src) {
        citations.extend(extract_call_site_citations_in(region));
    }
    citations
}

fn extract_call_site_citations_in(src: &str) -> Vec<String> {
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
/// literal from `layer_registry.rs`'s `call_sites` lists (Phase 117 gap
/// closure, NR3-08). A citation is symbol-form if it contains the literal
/// substring `".rs::"` — the double-colon distinguishes it from the
/// line-form `".rs:<digit>"` citations `extract_call_site_citations` finds.
///
/// Scoped via [`call_sites_regions`]: see that function for why a
/// whole-file scan was wrong.
fn extract_symbol_citations(src: &str) -> Vec<String> {
    let mut citations = Vec::new();
    for region in call_sites_regions(src) {
        citations.extend(extract_symbol_citations_in(region));
    }
    citations
}

fn extract_symbol_citations_in(src: &str) -> Vec<String> {
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

    // Non-vacuity floors for the region-scoped scan. Narrowing the scan from
    // "every literal in the file" to "the call_sites lists" is the fail-OPEN
    // direction if the marker ever stops matching (a rustfmt change, a field
    // rename), so pin both the number of lists found and the number of
    // citations inside them. Two of the thirteen rows carry `call_sites: &[]`
    // deliberately (DaclSessionSidGrant, MinifilterAbsence), so the citation
    // floor mirrors layer_registry.rs's own `every_call_site_string_is_symbol_form`.
    let regions = call_sites_regions(&src).len();
    assert!(
        regions >= 13,
        "only {regions} `call_sites: &[` list(s) found in layer_registry.rs, but every \
         LayerRegistryEntry declares one. The region marker has stopped matching, so this \
         test is now scanning almost nothing."
    );
    assert!(
        citations.len() + symbol_citations.len() >= 20,
        "only {} call_sites citation(s) extracted from {regions} list(s) — the scan went \
         quiet. A citation gate that resolves nothing passes for the wrong reason.",
        citations.len() + symbol_citations.len()
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

/// Phase 117 Plan 32 (WR-19 gap closure): does `span` — a single Markdown
/// backtick-delimited span pulled from the SPEC document's text — have the
/// `word(/word)*.rs::Word(::Word)?` shape of a real `file.rs::Symbol`
/// citation, as opposed to:
/// - an illustrative example of the citation FORMAT itself, which this
///   document always wraps in an extra pair of literal double quotes (e.g.
///   `` `"file.rs::Symbol"` ``) — the same convention
///   `extract_call_site_citations`'s doc comment above already establishes
///   for the line-form illustrative example `` `"file:line"` ``; or
/// - an English-prose mention with trailing call-parens (`` `apply()` ``,
///   not a `file.rs::Symbol` citation at all — a real citation names a bare
///   symbol, never `Symbol()`).
///
/// `span` qualifies only if the file part (everything up to and including
/// `.rs`) consists of `/`-separated path segments of identifier/hyphen/dot
/// characters only, AND the symbol part (everything after `.rs::`) is one
/// or two `::`-separated identifier segments (`Word` or `Word::Word`) with
/// no other characters — either malformed half (a stray quote, unmatched
/// parens, more than one `::` in the symbol) disqualifies the whole span.
fn looks_like_a_spec_citation(span: &str) -> bool {
    let Some(idx) = span.find(".rs::") else {
        return false;
    };
    let file_part = &span[..idx + 3];
    let symbol_part = &span[idx + 5..];

    let file_ok = !file_part.is_empty()
        && file_part.split('/').all(|seg| {
            !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        });

    // Phase 117 Plan 36 (CR-04): reject the illustrative placeholder token
    // STRUCTURALLY, independent of the document's quoting convention. No real
    // source file in this workspace is ever named literally `file.rs`, so a
    // span whose file part's last path segment is `file` is always an example
    // of the citation FORMAT, never a citation.
    //
    // Before this check, the only thing excluding the illustrative examples was
    // the extra pair of literal double quotes the document wraps them in (which
    // fails the character class above). Nothing enforced that convention, so
    // Plan 117-34 — a later wave in the SAME round that built this gate — wrote
    // an UNQUOTED one, and the gate then tried to resolve `file.rs::Symbol` as a
    // real definition and failed the build (CR-04). Case-insensitive, so a
    // future writer capitalizing the placeholder cannot reopen it either.
    let last_segment = file_part.rsplit('/').next().unwrap_or(file_part);
    if last_segment
        .strip_suffix(".rs")
        .unwrap_or(last_segment)
        .eq_ignore_ascii_case("file")
    {
        return false;
    }

    let symbol_segments: Vec<&str> = symbol_part.split("::").collect();
    let symbol_ok = !symbol_part.is_empty()
        && symbol_segments.len() <= 2
        && symbol_segments.iter().all(|seg| {
            !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        });

    file_ok && symbol_ok
}

/// Extract every backtick-delimited span from `text` that satisfies
/// `looks_like_a_spec_citation` — the SPEC-document equivalent of
/// `extract_symbol_citations`'s registry-source-text scan, generalized to
/// scan Markdown backtick spans instead of Rust double-quoted string
/// literals. Phase 117 Plan 32 (WR-19): the only prior drift gate,
/// `spec_matches_registry`, compared `LayerId` NAMES, never citations, so
/// nothing in the test suite could see a citation drift anywhere in this
/// document — this scans the WHOLE document text, not only the Layer
/// registry table, so a stale citation in the Manual verification section
/// or the discrepancy ledger fails the build too.
fn extract_spec_citations(text: &str) -> Vec<String> {
    let mut citations = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after_open = &rest[start + 1..];
        let Some(end) = after_open.find('`') else {
            break;
        };
        let literal = &after_open[..end];
        if looks_like_a_spec_citation(literal) {
            citations.push(literal.to_string());
        }
        rest = &after_open[end + 1..];
    }
    citations
}

/// Segment `layer_registry.rs`'s source text into one slice per
/// `LayerRegistryEntry`, split on `"id: LayerId::"` occurrences (each of the
/// 13 entries has exactly one), and extract each entry's OWN
/// `call_sites: &[...]` array content from ITS OWN narrowed slice — not the
/// whole file — so a citation belonging to one row is never attributed to
/// another. Returns `(LayerId name, this entry's own citations)` pairs, in
/// declaration order. Reuses `extract_symbol_citations` /
/// `extract_call_site_citations` on the narrowed `call_sites: &[...]` slice
/// rather than duplicating their per-literal parsing logic.
fn registry_citations_by_layer_id(registry_src: &str) -> Vec<(String, Vec<String>)> {
    let marker = "id: LayerId::";
    let mut entries = Vec::new();
    let mut rest = registry_src;
    while let Some(start) = rest.find(marker) {
        let after_marker = &rest[start + marker.len()..];
        let name_end = after_marker
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after_marker.len());
        let name = after_marker[..name_end].to_string();

        let entry_slice_end = rest[start + marker.len()..]
            .find(marker)
            .map_or(rest.len(), |next_rel| start + marker.len() + next_rel);
        let entry_slice = &rest[start..entry_slice_end];

        let citations = match entry_slice.find("call_sites: &[") {
            Some(cs_start) => {
                let after_open = &entry_slice[cs_start + "call_sites: &[".len()..];
                let array_body = after_open
                    .find(']')
                    .map_or("", |cs_end| &after_open[..cs_end]);
                // `array_body` is ALREADY a `call_sites` list body, so use
                // the region-level extractors directly — the top-level
                // `extract_*_citations` wrappers re-scope by searching for
                // `call_sites: &[`, which does not occur inside a body.
                let mut c = extract_symbol_citations_in(array_body);
                c.extend(extract_call_site_citations_in(array_body));
                c
            }
            None => Vec::new(),
        };

        entries.push((name, citations));
        rest = &rest[entry_slice_end..];
    }
    entries
}

/// Parse the SPEC's `## Layer registry` Markdown table: locate the table by
/// its header row, then for each data row (skipping the `|---|...`
/// separator), split on `|` and read the `LayerId` name cell and the
/// "Enforcing call site(s)" cell. `str::split('|')` on a row that starts and
/// ends with `|` produces an empty leading element, so the row's own first
/// DATA cell is `cells[1]` (the backtick-wrapped `LayerId` name) and its
/// third DATA cell is `cells[3]` (the citations column) — matching the
/// table's `| LayerId | Name | Enforcing call site(s) | Expected on |
/// Outcome | Probe |` column order. Returns `(LayerId name, this row's own
/// citation-shaped backtick spans)` pairs, in table order. Terminates at the
/// first line that does not start with `|` — no hardcoded row count.
fn spec_layer_registry_table_citations(spec_text: &str) -> Vec<(String, Vec<String>)> {
    let header_marker = "| `LayerId` | Name | Enforcing call site(s) |";
    let header_start = spec_text.find(header_marker).unwrap_or_else(|| {
        panic!(
            "expected to find the Layer registry table header `{header_marker}` in the SPEC — \
             the table was renamed, reformatted, or removed; update this test's marker to match"
        )
    });
    let after_header = &spec_text[header_start..];

    let mut lines = after_header.lines();
    lines.next(); // the header row itself (already matched by header_marker)
    lines.next(); // the `|---|---|...` separator row

    let mut rows = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            break;
        }
        let cells: Vec<&str> = trimmed.split('|').collect();
        let (Some(name_cell), Some(call_sites_cell)) = (cells.get(1), cells.get(3)) else {
            continue;
        };
        let name = name_cell.trim().trim_matches('`').to_string();
        if name.is_empty() {
            continue;
        }
        rows.push((name, extract_spec_citations(call_sites_cell)));
    }
    rows
}

/// CINT-01/WR-19: the call-site drift gate `spec_matches_registry` could
/// never be — it compared `LayerId` NAMES only. For every `LayerId` row,
/// asserts the SPEC's `## Layer registry` table's "Enforcing call site(s)"
/// cell equals, AS A SET, that row's own `call_sites` array in
/// `layer_registry.rs` — a stale, added, or removed citation in either the
/// SPEC or the registry fails this test, naming the specific `LayerId`.
/// `DaclSessionSidGrant` and `MinifilterAbsence` (RF-03/D-33: `call_sites`
/// is empty per the shipped tree) are not special-cased — an empty registry
/// set is asserted equal to the SPEC cell's own (also empty) parsed set,
/// same as every other row.
#[test]
fn spec_call_site_cells_match_registry_call_sites() {
    let registry_src = read_layer_registry();
    let spec_text = read_spec();

    let registry_map = registry_citations_by_layer_id(&registry_src);
    let spec_map = spec_layer_registry_table_citations(&spec_text);

    assert!(
        registry_map.len() >= 13,
        "expected at least 13 entries parsed from layer_registry.rs's REGISTRY_ENTRIES (one per \
         `id: LayerId::` occurrence), found {}: {registry_map:?} — the segmentation marker may \
         have gone stale",
        registry_map.len()
    );
    assert_eq!(
        spec_map.len(),
        registry_map.len(),
        "SPEC Layer registry table row count ({}) does not match the entry count ({}) parsed \
         from layer_registry.rs — a row was added or removed in only one of the two places",
        spec_map.len(),
        registry_map.len()
    );

    let mut mismatches = Vec::new();
    for (name, registry_citations) in &registry_map {
        let Some((_, spec_citations)) = spec_map.iter().find(|(n, _)| n == name) else {
            mismatches.push(format!(
                "{name}: present in layer_registry.rs but has no row in the SPEC's Layer \
                 registry table"
            ));
            continue;
        };

        let mut registry_set: Vec<&String> = registry_citations.iter().collect();
        registry_set.sort();
        let mut spec_set: Vec<&String> = spec_citations.iter().collect();
        spec_set.sort();

        if registry_set != spec_set {
            mismatches.push(format!(
                "{name}: registry call_sites {registry_citations:?} does not match the SPEC's \
                 Enforcing call site(s) cell {spec_citations:?}"
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "SPEC Layer registry table citations have drifted from layer_registry.rs's own \
         call_sites (WR-19's structural fix — the prior spec_matches_registry test compared \
         LayerId names only and could never have caught this):\n{}",
        mismatches.join("\n")
    );
}

/// CINT-01/WR-19: every `` `file.rs::Symbol` ``-shaped citation ANYWHERE in
/// the SPEC document's text — not only inside the Layer registry table —
/// resolves to a real definition in the file it names. Proves this test
/// would have caught WR-19's stale
/// `` `BrokerAuthenticodeTrustGate` `` Manual-verification citation (fixed
/// by this same plan's Task 1) before it was fixed; see the perturbation
/// proof recorded in this plan's SUMMARY.md.
#[test]
fn every_spec_symbol_citation_resolves_to_a_real_definition() {
    let spec_text = read_spec();
    let citations = extract_spec_citations(&spec_text);

    // Non-vacuity floor (Phase 115 V-01 lesson): the Layer registry table
    // alone contributes ~31 citations, plus several more from the Manual
    // verification section and the discrepancy ledger — a floor, not an
    // exact count, so a future citation addition or removal does not force
    // this test to be edited every time.
    assert!(
        citations.len() >= 30,
        "expected at least 30 file.rs::Symbol-shaped citations across the whole SPEC document \
         (the Layer registry table alone has ~31) — found {}: extraction may be broken: \
         {citations:?}",
        citations.len()
    );

    let mut missing = Vec::new();
    for citation in &citations {
        let (file_part, symbol) = split_symbol_citation(citation);
        let resolved = resolve_file_part(file_part);
        match std::fs::read_to_string(&resolved) {
            Ok(content) if content_defines_symbol(&content, symbol) => {}
            Ok(_) => missing.push(format!(
                "{citation:?} -> resolved to {}, but its content does not define {symbol:?} at \
                 a real definition site (only a comment, string, or macro literal mention, if \
                 any)",
                resolved.display()
            )),
            Err(e) => missing.push(format!(
                "{citation:?} -> resolved to {} (unreadable: {e})",
                resolved.display()
            )),
        }
    }

    assert!(
        missing.is_empty(),
        "proj/SPEC-windows-fail-direction-contract.md cites a file.rs::Symbol that does not \
         resolve to a real definition anywhere in the document (Manual verification section, \
         discrepancy ledger, or any other section — not only the Layer registry table):\n{}",
        missing.join("\n")
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

/// Phase 117 Plan 36 (CR-04 regression): the illustrative `file.rs::Symbol`
/// placeholder — the example of the citation FORMAT this document uses to
/// describe its own convention — must never be treated as a real citation,
/// whether or not the writer remembered the extra pair of literal double
/// quotes.
///
/// CR-04: Plan 117-32 built `every_spec_symbol_citation_resolves_to_a_real_definition`
/// and relied on the quoting convention alone to exclude the illustrative
/// examples. Plan 117-34 — a LATER WAVE IN THE SAME ROUND — then wrote an
/// unquoted one into the WR-19 ledger row, and the gate failed the build
/// trying to resolve `file.rs` as a real source file. The exclusion is now a
/// property of the matcher, not a convention a future writer must remember.
#[test]
fn illustrative_format_example_is_not_treated_as_a_citation() {
    assert!(
        !looks_like_a_spec_citation("file.rs::Symbol"),
        "the bare unquoted placeholder `file.rs::Symbol` is an example of the citation \
         FORMAT, not a citation — this exact span failing to be excluded is CR-04"
    );
    assert!(
        !looks_like_a_spec_citation("\"file.rs::Symbol\""),
        "the quoted-convention form must remain excluded too — Plan 36 strengthens the \
         existing rule, it does not replace it"
    );
    assert!(
        !looks_like_a_spec_citation("File.rs::Symbol"),
        "placeholder rejection is case-insensitive: a writer capitalizing the placeholder \
         must not reopen CR-04"
    );
    assert!(
        looks_like_a_spec_citation("launch.rs::is_dev_build_layout"),
        "a REAL citation naming a real file must still qualify — the new exclusion must \
         reject only the literal placeholder token `file`, never a real file name"
    );
    assert!(
        looks_like_a_spec_citation(
            "crates/nono-cli/src/exec_strategy_windows/launch.rs::is_dev_build_layout"
        ),
        "a real path-qualified citation must still qualify — the placeholder check reads \
         only the LAST `/`-separated segment, so intermediate path segments are irrelevant"
    );
}

/// Phase 117 Plan 36 (WR-32): `layer_registry.rs`'s OWN prose describing the
/// citation convention must use the current `file.rs::Symbol` form, not the
/// pre-WR-08 `"file:line"` form.
///
/// This is deliberately NARROW rather than a file-wide ban on `\w+\.rs:\d+`:
/// `layer_registry.rs` legitimately cites other files' analogs in ordinary
/// descriptive prose elsewhere in its own module docs (e.g. `network.rs:1500`,
/// `crates/nono/src/undo/types.rs:333`), and a blanket rule would fail on
/// those. WR-32's finding is specifically about the two doc comments that
/// DEFINE the convention — those are what these two tests pin.
#[test]
fn call_sites_field_doc_describes_the_symbol_form() {
    let src = read_layer_registry();

    assert!(
        !src.contains("\"file:line\""),
        "layer_registry.rs still describes its call_sites citations as `\"file:line\"` — \
         that convention was replaced by the `file.rs::Symbol` form in WR-08/NR3-08, and \
         every real call_sites entry in this file has used the symbol form since (WR-32)"
    );

    let field_pos = src.find("pub call_sites:").unwrap_or_else(|| {
        panic!("layer_registry.rs no longer declares a `pub call_sites:` field")
    });
    let doc_window = &src[field_pos.saturating_sub(400)..field_pos];
    assert!(
        doc_window.contains("file.rs::Symbol"),
        "the doc comment immediately preceding `pub call_sites:` must describe the current \
         `file.rs::Symbol` citation form (WR-32) — window was:\n{doc_window}"
    );
}

/// Phase 117 Plan 36 (WR-32): the `token_arm` prose must cite a SYMBOL, not a
/// raw line range. There were TWO such citations, not one — `ArmExpectancy`'s
/// doc and `token_arm_names`' module doc both carried `launch.rs:1237-1278`.
/// The second was not named in this plan's `<interfaces>` and was found only
/// because the acceptance criterion demanded a zero count file-wide, which is
/// why this test asserts absence across the whole file rather than at one site.
#[test]
fn token_arm_doc_cites_a_symbol_not_a_raw_line_range() {
    let src = read_layer_registry();

    assert!(
        !src.contains("launch.rs:1237"),
        "layer_registry.rs still carries the stale raw line-range citation \
         `launch.rs:1237-1278` for the WindowsTokenArm variants; the real declaration is \
         `launch.rs::select_windows_token_arm` (WR-32)"
    );
    assert!(
        src.contains("launch.rs::select_windows_token_arm"),
        "the token_arm prose must cite `launch.rs::select_windows_token_arm` by symbol so \
         it survives line drift (WR-32)"
    );
}

/// Phase 117 review WR-04: every `OPEN` marker in the tree must have a row in
/// the SPEC's review-fix discrepancy ledger — and that row must actually BE a
/// row of that table.
///
/// # Why this exists
///
/// WR-10 was consciously skipped on the sole basis that the conflation was
/// "recorded explicitly in the SPEC's ... table (D-15)". The recorded
/// mitigation was the entire justification for not fixing a live
/// naming/claim-precision defect in a cross-binary wire contract — and the
/// record was not in the table it was claimed to be in: a stray blank line sat
/// between the CR-02 row and the WR-10 row, and in GitHub-Flavored Markdown a
/// blank line TERMINATES a table. The WR-10 row rendered as a literal
/// paragraph of pipe characters.
///
/// So the parse here is deliberately GFM-faithful: rows are collected only
/// while they are contiguous with the header + delimiter pair. A blank line
/// anywhere in the table drops every row after it, which is exactly what a
/// reader's Markdown renderer does, and exactly what this gate must notice.
///
/// # The ID-collision hazard, handled explicitly
///
/// Finding identifiers REPEAT across review iterations: there is a `WR-14
/// (Iteration 5, ...)` row AND a `WR-14 (Iteration 6, ...)` row. Requiring
/// merely "some row whose id cell starts with `WR-14`" would therefore be
/// satisfiable by the wrong row — the exact "green for the wrong reason" shape
/// this phase keeps producing. The matching row must ALSO be marked `OPEN`,
/// which no closed row of an earlier iteration is.
#[test]
fn every_open_marker_in_code_has_a_ledger_row() {
    // Rows of the SPEC's review-fix ledger, as raw lines, parsed the way a GFM
    // renderer parses them: header, delimiter, then contiguous `|` lines.
    fn ledger_rows(spec: &str) -> Vec<String> {
        let lines: Vec<&str> = spec.lines().collect();
        let header = lines
            .iter()
            .position(|l| l.trim_start().starts_with("| # | What was wrong |"))
            .unwrap_or_else(|| {
                panic!(
                    "the SPEC's review-fix ledger header was not found — the table this gate \
                     exists to protect has been renamed or removed"
                )
            });
        assert!(
            lines
                .get(header + 1)
                .is_some_and(|l| l.trim_start().starts_with("|---")),
            "the ledger header is not followed by a delimiter row, so it is not a table at all"
        );
        let mut rows = Vec::new();
        for line in lines.iter().skip(header + 2) {
            if !line.trim_start().starts_with('|') {
                break;
            }
            rows.push((*line).to_string());
        }
        rows
    }

    // Source files that may carry an `OPEN` marker. Deliberately explicit: a
    // glob would silently start scanning generated or vendored trees.
    fn marker_sources() -> Vec<(&'static str, PathBuf)> {
        vec![
            (
                "crates/nono/src/error.rs",
                workspace_root().join("crates/nono/src/error.rs"),
            ),
            (
                "crates/nono-cli/src/exec_strategy_windows/layer_registry.rs",
                manifest_dir().join("src/exec_strategy_windows/layer_registry.rs"),
            ),
            (
                "crates/nono-cli/src/exec_strategy_windows/launch.rs",
                manifest_dir().join("src/exec_strategy_windows/launch.rs"),
            ),
            (
                "crates/nono-cli/src/exec_strategy_windows/attestation.rs",
                manifest_dir().join("src/exec_strategy_windows/attestation.rs"),
            ),
            (
                "crates/nono-cli/src/output.rs",
                manifest_dir().join("src/output.rs"),
            ),
        ]
    }

    let rows = ledger_rows(&read_spec());
    assert!(
        rows.len() >= 40,
        "non-vacuity: the review-fix ledger parsed to only {} row(s). It carried 60+ before \
         this gate was written, so a low count means the table was truncated — by a blank \
         line, a section split, or any other renderer-visible break — and every row after the \
         break is no longer part of the table a reader sees.",
        rows.len()
    );

    // Discovery: find every `XX-NN OPEN` marker rather than naming the two
    // that exist today, so a third marker added later is covered without this
    // test being touched.
    let mut markers: Vec<(String, String)> = Vec::new();
    for (label, path) in marker_sources() {
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        for (n, line) in src.lines().enumerate() {
            let Some(pos) = line.find(" OPEN") else {
                continue;
            };
            // Walk back over the identifier immediately preceding ` OPEN`.
            let head = &line[..pos];
            let mut id: Vec<char> = head
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            id.reverse();
            let id: String = id.into_iter().collect();
            // Shape: two uppercase letters, a dash, then digits (CR-01, WR-14).
            let looks_like_finding_id = id.len() >= 4
                && id.chars().take(2).all(|c| c.is_ascii_uppercase())
                && id.chars().nth(2) == Some('-')
                && id[3..].chars().all(|c| c.is_ascii_digit());
            if looks_like_finding_id {
                markers.push((id, format!("{label}:{}", n + 1)));
            }
        }
    }

    assert!(
        !markers.is_empty(),
        "non-vacuity: no `XX-NN OPEN` marker was found in any scanned source. Either every \
         deferred finding has been closed (delete this gate deliberately if so) or the marker \
         convention changed and this scan is now blind."
    );

    let mut missing = Vec::new();
    for (id, site) in &markers {
        let prefix = format!("| {id} ");
        let matched = rows
            .iter()
            .any(|r| r.starts_with(&prefix) && r.contains("OPEN"));
        if !matched {
            missing.push(format!("{id} (marked OPEN at {site})"));
        }
    }

    assert!(
        missing.is_empty(),
        "WR-04: {} finding(s) are marked OPEN in code but have no OPEN row in the SPEC's \
         review-fix discrepancy ledger. A deferral whose entire justification is that it is \
         recorded in the SPEC must actually be recorded there, as a row of that table:\n  {}\n\
         (Ledger rows parsed: {}.)",
        missing.len(),
        missing.join("\n  "),
        rows.len()
    );
}

/// Phase 117 review WR-07: the `WR-14 OPEN` record must state the REAL reason
/// `probe_in_job`'s failure is unreachable through nono's own binaries.
///
/// The block previously said case (3) "requires a null job handle no
/// production caller passes". That is a claim about the callee's INPUT, and it
/// is not what makes the case unreachable: both production callers DISCARD the
/// `Err`, so the variant could not reach `remediation()` even with a null
/// handle. An implementer reading the old wording would hunt for a null-handle
/// guard at the call site and conclude the case had become reachable when it
/// had not.
///
/// This pins both halves — the corrected record, and the two swallow sites the
/// record depends on. If either call site stops discarding, the record becomes
/// false and this fails, which is the whole point: the defect class this phase
/// keeps re-producing is a record that outlives the code it describes.
#[test]
fn the_wr14_open_record_matches_the_actual_swallow_sites() {
    let error_rs = std::fs::read_to_string(workspace_root().join("crates/nono/src/error.rs"))
        .expect("read crates/nono/src/error.rs");

    for needle in [
        "classify_probe_outcome",
        "agent_daemon/launch.rs",
        "Ok(true)",
        "FFI",
    ] {
        assert!(
            error_rs.contains(needle),
            "WR-07: the `WR-14 OPEN` record in crates/nono/src/error.rs no longer names \
             {needle:?}. The record must state that BOTH production callers discard the Err \
             (that is what makes the case unreachable through nono's own binaries), and that \
             the case IS reachable for FFI/embedder callers today."
        );
    }
    assert!(
        !error_rs.contains("requires a null job handle"),
        "WR-07: the incorrect reason (a null job handle no production caller passes) is back \
         in the WR-14 OPEN record. Both production callers swallow the Err, so the job handle \
         is not what makes the case unreachable."
    );

    // The two swallow sites the record depends on, asserted against the real
    // source rather than trusted.
    let cli_attestation =
        std::fs::read_to_string(manifest_dir().join("src/exec_strategy_windows/attestation.rs"))
            .expect("read exec_strategy_windows/attestation.rs");
    assert!(
        cli_attestation.contains("classify_probe_outcome(probe_in_job("),
        "WR-07: nono-cli no longer wraps `probe_in_job` in `classify_probe_outcome`, so the \
         WR-14 OPEN record's unreachability reasoning no longer describes this call site"
    );
    assert!(
        cli_attestation.contains("Err(_) => LayerAttestationStatus::Unconfirmed"),
        "WR-07: `classify_probe_outcome` no longer discards the Err — `probe_in_job`'s \
         LayerAttestationFailed can now reach `remediation()` through nono-cli, and the \
         WR-14 OPEN record must be updated in the same change"
    );

    let daemon = std::fs::read_to_string(manifest_dir().join("src/agent_daemon/launch.rs"))
        .expect("read agent_daemon/launch.rs");
    assert!(
        daemon.contains("matches!(probe_in_job(process, job), Ok(true))"),
        "WR-07: nono-agentd no longer discards `probe_in_job`'s Err via matches!(.., \
         Ok(true)), so the WR-14 OPEN record's unreachability reasoning no longer describes \
         this call site"
    );
}
