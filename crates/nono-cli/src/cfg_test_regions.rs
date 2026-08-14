//! Shared `#[cfg(test)]`-region classifier for this crate's source-text
//! drift gates (Phase 117 review WR-01).
//!
//! # Why this module exists
//!
//! Several gates in this crate assert things about *production* source text —
//! that an operator-facing needle sits inside a particular match arm, that a
//! registry row is named by a hand-written mirror in another binary. Every one
//! of them must exclude `#[cfg(test)]` module bodies, because a test module
//! legitimately asserts ON the very strings the gate is hunting, and a gate a
//! test assertion can satisfy proves nothing.
//!
//! WR-01 is what happens when that classifier is written twice. Two mirrors of
//! this predicate shipped **in the same fix pass** and disagreed:
//!
//! - `layer_registry.rs`'s daemon gate skipped attributes and doc comments
//!   sitting between the `#[cfg(test)]` attribute and its `mod` line.
//! - `output.rs`'s WR-26 gate cleared its pending flag on any intervening
//!   line, so six of `launch.rs`'s twelve test modules — every one carrying
//!   `#[allow(clippy::unwrap_used)]` between the two — were **not** skipped,
//!   and **1773 lines of test code were scanned as production**.
//!
//! So there is exactly one implementation here, and both gates call it. A
//! future third gate that needs the same rule must call it too rather than
//! grow a third mirror.
//!
//! # The rule
//!
//! A region is skipped when a `#[cfg(...)]` attribute that gates on `test`
//! (but NOT `not(test)`) is followed — possibly across further attributes and
//! doc comments — by an INLINE `mod foo {`. The region ends at the closing
//! brace at the module's own indentation, so test modules nested inside
//! another module (`agent_daemon/launch.rs`'s live inside `mod windows_impl`)
//! close correctly. A bare `mod foo;` *declaration* opens no region — that is
//! `main.rs:155`'s `#[cfg(test)] mod test_env;`, and treating it as a region
//! start silently truncated the `main.rs` half of a gate to nothing.
//!
//! Line comments are dropped as well: prose ABOUT a rule is not an instance of
//! it, and a comment naming the thing a gate hunts for would otherwise satisfy
//! the gate.

/// Is `trimmed` (an already-trimmed line) a `cfg` attribute that gates on
/// `test`?
///
/// Covers `#[cfg(test)]` and `#[cfg(all(test, target_os = "..."))]`, both of
/// which gate test modules in this crate.
///
/// # Why `not(test` is excluded
///
/// `#[cfg(not(test))]` contains the substring `test)` and so was classified as
/// a test gate by both pre-WR-01 mirrors. A `#[cfg(not(test))] mod foo {` is
/// *production* code — it exists only in non-test builds — and would have been
/// skipped in its entirety. No such attribute exists in this crate today, so
/// that was latent rather than live; it is excluded here so it stays that way.
#[must_use]
pub fn is_cfg_test_attr(trimmed: &str) -> bool {
    trimmed.starts_with("#[cfg(")
        && !trimmed.contains("not(test")
        && (trimmed.contains("test)") || trimmed.contains("test,"))
}

/// The production half of a source file, plus the evidence a caller needs to
/// prove the split actually happened.
pub struct ProductionScan<'a> {
    /// Production lines as `(zero-based source index, raw line)`. Comments and
    /// `#[cfg(test)]` module bodies are absent.
    pub lines: Vec<(usize, &'a str)>,
    /// Skipped `#[cfg(test)]` module regions as `(first_line, last_line)`,
    /// zero-based and inclusive of the `mod` line.
    pub skipped_regions: Vec<(usize, usize)>,
}

impl ProductionScan<'_> {
    /// Total source lines dropped as `#[cfg(test)]` module bodies.
    #[must_use]
    pub fn skipped_lines(&self) -> usize {
        self.skipped_regions
            .iter()
            .map(|(a, b)| b.saturating_sub(*a).saturating_add(1))
            .sum()
    }
}

/// Split `src` into its production half per the rule in this module's doc.
#[must_use]
pub fn scan_production(src: &str) -> ProductionScan<'_> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out: Vec<(usize, &str)> = Vec::new();
    let mut skipped_regions: Vec<(usize, usize)> = Vec::new();
    let mut idx = 0usize;
    let mut pending_test_attr = false;

    while idx < lines.len() {
        let t = lines[idx].trim();

        if is_cfg_test_attr(t) {
            pending_test_attr = true;
            idx += 1;
            continue;
        }
        if pending_test_attr {
            // Further attributes and doc comments may sit between the cfg
            // attribute and the item it gates. This is the exact line WR-01
            // turned on: `#[allow(clippy::unwrap_used)]` between
            // `#[cfg(all(test, ...))]` and `mod ... {`.
            if t.starts_with("#[") || t.starts_with("///") || t.starts_with("//!") {
                idx += 1;
                continue;
            }
            pending_test_attr = false;
            // An INLINE `mod foo {` opens a test region; a bare `mod foo;`
            // declaration opens nothing.
            if (t.starts_with("mod ") || t.starts_with("pub mod ")) && t.ends_with('{') {
                let start = idx;
                let indent = lines[idx].len() - lines[idx].trim_start().len();
                let closer = format!("{}}}", " ".repeat(indent));
                idx += 1;
                while idx < lines.len() && lines[idx] != closer {
                    idx += 1;
                }
                // `idx` is the closer (or EOF). Record the region inclusive of
                // both, then step past it.
                skipped_regions.push((start, idx.min(lines.len().saturating_sub(1))));
                idx += 1;
                continue;
            }
            // Fall through: the cfg-test attribute gated something that is not
            // an inline module (a bare declaration, a `use`, a fn). It is not
            // production text either way, but it opens no region.
        }

        if t.starts_with("//") {
            idx += 1;
            continue;
        }
        out.push((idx, lines[idx]));
        idx += 1;
    }

    ProductionScan {
        lines: out,
        skipped_regions,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_cfg_test_attr, scan_production};

    /// The classification rule itself, asserted directly rather than through a
    /// consumer. WR-01's whole mechanism was two consumers disagreeing about
    /// this predicate while both of their own gates stayed green.
    #[test]
    fn cfg_test_attr_classification_rule() {
        for yes in [
            "#[cfg(test)]",
            "#[cfg(all(test, target_os = \"windows\"))]",
            "#[cfg(all(test, feature = \"x\"))]",
        ] {
            assert!(
                is_cfg_test_attr(yes),
                "{yes:?} must classify as a test gate"
            );
        }
        for no in [
            "#[cfg(not(test))]",
            "#[cfg(all(not(test), target_os = \"windows\"))]",
            "#[cfg(target_os = \"windows\")]",
            "#[allow(clippy::unwrap_used)]",
            "mod tests {",
        ] {
            assert!(
                !is_cfg_test_attr(no),
                "{no:?} must NOT classify as a test gate — `#[cfg(not(test))]` gates \
                 PRODUCTION code, and treating it as a test gate skips a whole production \
                 module"
            );
        }
    }

    /// The exact shape WR-01 found live in `launch.rs`: an intervening
    /// `#[allow(...)]` between the cfg attribute and the `mod` line.
    #[test]
    fn intervening_attributes_do_not_break_the_region() {
        let src = "\
fn production_a() {}
#[cfg(all(test, target_os = \"windows\"))]
#[allow(clippy::unwrap_used)]
mod some_tests {
    fn inside_a_test() {}
}
fn production_b() {}
";
        let scan = scan_production(src);
        let kept: Vec<&str> = scan.lines.iter().map(|(_, l)| *l).collect();
        assert_eq!(
            kept,
            vec!["fn production_a() {}", "fn production_b() {}"],
            "an `#[allow(...)]` between the cfg attribute and the `mod` line must not \
             re-classify the module body as production (WR-01)"
        );
        assert_eq!(scan.skipped_regions.len(), 1);
        // `mod some_tests {` .. `}` inclusive — the gating attributes above it
        // are dropped as attributes, not counted as region body.
        assert_eq!(scan.skipped_lines(), 3);
    }

    /// A `#[cfg(not(test))]` module is production and must be kept whole.
    #[test]
    fn not_test_modules_are_production() {
        let src = "\
#[cfg(not(test))]
mod real_impl {
    fn shipped() {}
}
";
        let scan = scan_production(src);
        assert!(
            scan.skipped_regions.is_empty(),
            "`#[cfg(not(test))]` gates production code; skipping it hides real source from \
             every gate built on this helper"
        );
        assert!(scan.lines.iter().any(|(_, l)| l.contains("fn shipped")));
    }

    /// A bare `#[cfg(test)] mod foo;` declaration opens no region — this is
    /// `main.rs:155`, and treating it as a region start truncated a gate's
    /// whole scan of that file to nothing.
    #[test]
    fn bare_module_declaration_opens_no_region() {
        let src = "\
#[cfg(test)]
mod test_env;
fn production_after() {}
";
        let scan = scan_production(src);
        assert!(scan.skipped_regions.is_empty());
        assert!(scan
            .lines
            .iter()
            .any(|(_, l)| l.contains("production_after")));
    }

    /// Nested test modules close at their own indentation, not at column 0 —
    /// `agent_daemon/launch.rs`'s live inside `mod windows_impl`.
    #[test]
    fn nested_test_modules_close_at_their_own_indent() {
        let src = "\
mod outer {
    #[cfg(test)]
    mod inner_tests {
        fn t() {}
    }
    fn production_sibling() {}
}
";
        let scan = scan_production(src);
        assert_eq!(scan.skipped_regions.len(), 1);
        assert!(
            scan.lines
                .iter()
                .any(|(_, l)| l.contains("production_sibling")),
            "a nested test region must not swallow the rest of its parent module"
        );
        assert!(!scan.lines.iter().any(|(_, l)| l.contains("fn t()")));
    }

    /// Comments are dropped: prose about a rule is not an instance of it.
    #[test]
    fn comments_are_not_production_text() {
        let src = "\
// mentions DowngradeDetailChannel::EventLog in prose
fn real() {}
";
        let scan = scan_production(src);
        assert_eq!(scan.lines.len(), 1);
        assert!(scan.lines[0].1.contains("fn real"));
    }
}
