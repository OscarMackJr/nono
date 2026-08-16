//! D-14 (half 1): a discovery-based type-allowlist source scan over
//! `EnforcementReceipt`'s field types (`crates/nono/src/receipt.rs`).
//!
//! This is a mechanical, compile-time-adjacent enforcement of D-05's
//! content-free claim: the scan parses the struct fresh, on every run,
//! never hardcoding its shape (house pattern — see
//! `crates/nono-cli/tests/layer_registry_selfcheck.rs`, no `regex`, no
//! `include_str!` for the file being scanned). A field of a
//! non-allowlisted type (`PathBuf`, a path-shaped `String`, a raw
//! `Vec<u8>`, etc.) fails this scan — the D-14 SC2 mechanism this test
//! exists to be.
//!
//! **The type-FIELD-parsing mechanism here is new** (`118-PATTERNS.md`):
//! every existing house scan matches `fn`/enum-variant names, none parses
//! struct field types. Line-by-line only, no `regex`.
//!
//! Three behaviors, each a `#[test]`:
//! 1. Positive — the CURRENT `EnforcementReceipt` shape passes.
//! 2. Perturbation proof (negative) — a synthetic struct body with a
//!    `PathBuf` field and a mis-named `String` field fails, via the SAME
//!    parsing/classification function, fed an in-test string literal (no
//!    real-file mutation).
//! 3. Converse proof (guards against vacuous failure) — a synthetic struct
//!    body containing only allowlisted-type fields passes, via the same
//!    function.
//!
//! Plus a discovery-failure guard: the parser panics loudly (never
//! silently returns an empty `Vec`) if it cannot locate the struct
//! definition at all — the Phase 115 V-01 lesson ("a test that names its
//! targets is blind by construction" cuts both ways: a scan that silently
//! passes when it finds nothing is the same defect).

use std::path::PathBuf;

/// `crates/nono-cli` — the crate this integration test belongs to.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The workspace root (`crates/nono-cli/../..`), same derivation as
/// `layer_registry_selfcheck.rs::workspace_root`.
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

/// Reads `crates/nono/src/receipt.rs` fresh on every test run — never
/// `include_str!`, which would compile the scanned text into the test
/// binary and defeat "discovers, never assumes".
fn read_receipt_source() -> String {
    let path = workspace_root()
        .join("crates")
        .join("nono")
        .join("src")
        .join("receipt.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Extracts `(field_name, type_text)` pairs from the brace-delimited body
/// of `pub struct <struct_name> { ... }` within `source`.
///
/// Line-by-line string parsing only — no `regex`. Locates the struct's
/// opening brace by literal substring match, then walks forward counting
/// brace depth to find the matching close, so field types containing
/// angle-bracket generics (`Vec<LayerReceiptRow>`, `Option<&'static str>`)
/// never confuse the boundary (angle brackets are not counted; only `{`/`}`
/// are, and no field in this struct's shape contains a nested `{`/`}`).
///
/// # Panics
///
/// Panics with a descriptive message — never silently returns an empty
/// `Vec` — if `struct_name`'s definition cannot be located in `source`, or
/// if the struct's braces are unbalanced. A renamed, removed, or
/// reformatted struct must fail the build loudly, not pass vacuously.
fn field_types_in_struct(source: &str, struct_name: &str) -> Vec<(String, String)> {
    let marker = format!("pub struct {struct_name} {{");
    let start = source.find(&marker).unwrap_or_else(|| {
        panic!(
            "could not locate `{marker}` in the scanned source — struct renamed, removed, or reformatted?"
        )
    });
    let body_start = start + marker.len();
    let bytes = source.as_bytes();
    let mut depth: i32 = 1;
    let mut idx = body_start;
    let mut body_end = None;
    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    body_end = Some(idx);
                    break;
                }
            }
            _ => {}
        }
        idx += 1;
    }
    let body_end = body_end.unwrap_or_else(|| {
        panic!("unbalanced braces while scanning for the end of `{struct_name}`'s body")
    });
    let body = &source[body_start..body_end];

    let mut fields = Vec::new();
    for raw_line in body.lines() {
        let line = raw_line.trim();
        let Some(rest) = line.strip_prefix("pub ") else {
            continue;
        };
        let Some(colon_idx) = rest.find(':') else {
            continue;
        };
        let field_name = rest[..colon_idx].trim().to_string();
        let type_text = rest[colon_idx + 1..].trim().trim_end_matches(',').trim();
        fields.push((field_name, type_text.to_string()));
    }
    fields
}

/// The exact field types `EnforcementReceipt` uses today, plus the one named
/// exception below (`session_id: String`). Anything else fails.
///
/// `entry_path`/`token_arm` were originally `&'static str` /
/// `Option<&'static str>` (Plan 118-01's first cut); an operator-decided
/// corrective retyped them as the core [`EntryPath`]/[`TokenArm`] enums —
/// content-free by construction, same promotion pattern as `LayerId` — so
/// `&'static str` and `Option<&'static str>` are deliberately NOT in this
/// list any more. A field typed `&'static str` on `EnforcementReceipt`
/// today would be a regression back to the pre-corrective shape and must
/// fail this scan (see `perturbation_proof_rejects_a_pathbuf_field_and_a_misnamed_string_field`,
/// which asserts exactly that).
const ALLOWED_TYPES: &[&str] = &[
    "u16",
    "u32",
    "EntryPath",
    "Option<TokenArm>",
    "SessionOutcome",
    "Vec<LayerReceiptRow>",
];

/// The single named exception D-05 permits: an opaque, per-session,
/// never-path-derived identifier stored as an owned `String` rather than
/// `&'static str`, because it is generated per-session, not statically
/// known. No OTHER field named anything else may be typed `String`.
const SESSION_ID_EXCEPTION_FIELD: &str = "session_id";
const SESSION_ID_EXCEPTION_TYPE: &str = "String";

/// Classifies `fields` against [`ALLOWED_TYPES`] plus the named
/// `session_id` exception, returning the violating `(field_name,
/// type_text)` pairs. An empty return means the scan passes.
///
/// A field passes if EITHER its type is in [`ALLOWED_TYPES`] verbatim, OR
/// it is named exactly [`SESSION_ID_EXCEPTION_FIELD`] AND typed exactly
/// [`SESSION_ID_EXCEPTION_TYPE`] — the one D-05-sanctioned owned-`String`
/// identity field. Any field named something else but typed `String` (or
/// any other non-allowlisted type, including `PathBuf`) is a violation.
fn classify_fields(fields: &[(String, String)]) -> Vec<(String, String)> {
    fields
        .iter()
        .filter(|(name, type_text)| {
            let is_session_id_exception =
                name == SESSION_ID_EXCEPTION_FIELD && type_text == SESSION_ID_EXCEPTION_TYPE;
            let is_allowlisted = ALLOWED_TYPES.contains(&type_text.as_str());
            !(is_session_id_exception || is_allowlisted)
        })
        .cloned()
        .collect()
}

#[test]
fn enforcement_receipt_current_shape_passes_the_type_allowlist_scan() {
    let source = read_receipt_source();
    let fields = field_types_in_struct(&source, "EnforcementReceipt");
    assert!(
        !fields.is_empty(),
        "parser found zero fields on the real EnforcementReceipt struct — discovery failure, not a real pass"
    );
    let violations = classify_fields(&fields);
    assert!(
        violations.is_empty(),
        "EnforcementReceipt has non-allowlisted field types: {violations:?}"
    );
}

#[test]
fn perturbation_proof_rejects_a_pathbuf_field_and_a_misnamed_string_field() {
    let synthetic = "pub struct EnforcementReceipt {\n\
                      \x20\x20\x20\x20pub schema_version: u16,\n\
                      \x20\x20\x20\x20pub workspace: String,\n\
                      \x20\x20\x20\x20pub log_path: PathBuf,\n\
                      \x20\x20\x20\x20pub entry_path: &'static str,\n\
                      }\n";
    let fields = field_types_in_struct(synthetic, "EnforcementReceipt");
    let violations = classify_fields(&fields);
    assert!(
        !violations.is_empty(),
        "perturbation proof failed: the scan did not flag a PathBuf field / a mis-named String field"
    );
    let violating_names: Vec<&str> = violations.iter().map(|(name, _)| name.as_str()).collect();
    assert!(
        violating_names.contains(&"workspace"),
        "scan must flag `workspace: String` (not the allowlisted `session_id` exception): {violations:?}"
    );
    assert!(
        violating_names.contains(&"log_path"),
        "scan must flag `log_path: PathBuf`: {violations:?}"
    );
    assert!(
        violating_names.contains(&"entry_path"),
        "scan must flag `entry_path: &'static str` — the pre-corrective shape is no longer \
         allowlisted, so a regression back to it must fail: {violations:?}"
    );
}

#[test]
fn allows_a_synthetic_struct_containing_only_allowlisted_types() {
    let synthetic = "pub struct EnforcementReceipt {\n\
                      \x20\x20\x20\x20pub schema_version: u16,\n\
                      \x20\x20\x20\x20pub session_id: String,\n\
                      \x20\x20\x20\x20pub pid: u32,\n\
                      \x20\x20\x20\x20pub entry_path: EntryPath,\n\
                      \x20\x20\x20\x20pub token_arm: Option<TokenArm>,\n\
                      \x20\x20\x20\x20pub outcome: SessionOutcome,\n\
                      \x20\x20\x20\x20pub layers: Vec<LayerReceiptRow>,\n\
                      }\n";
    let fields = field_types_in_struct(synthetic, "EnforcementReceipt");
    assert_eq!(
        fields.len(),
        7,
        "parser did not find all 7 synthetic fields: {fields:?}"
    );
    let violations = classify_fields(&fields);
    assert!(
        violations.is_empty(),
        "converse proof failed: an allowlisted-only synthetic struct was flagged: {violations:?}"
    );
}

#[test]
#[should_panic(expected = "could not locate")]
fn panics_loudly_when_the_struct_cannot_be_located() {
    let synthetic = "pub struct SomethingElse {\n    pub x: u16,\n}\n";
    let _ = field_types_in_struct(synthetic, "EnforcementReceipt");
}
