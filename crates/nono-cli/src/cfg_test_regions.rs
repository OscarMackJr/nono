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
//! (but NOT `not(test)`) is followed — possibly across **anything that is not
//! the gated item** — by an INLINE `mod foo {`. The region ends at the closing
//! brace at the module's own indentation, so test modules nested inside
//! another module (`agent_daemon/launch.rs`'s live inside `mod windows_impl`)
//! close correctly. A bare `mod foo;` *declaration* opens no region — that is
//! `main.rs:155`'s `#[cfg(test)] mod test_env;`, and treating it as a region
//! start silently truncated the `main.rs` half of a gate to nothing.
//!
//! Line comments are dropped as well: prose ABOUT a rule is not an instance of
//! it, and a comment naming the thing a gate hunts for would otherwise satisfy
//! the gate.
//!
//! # Round-4 WR-01: the window was narrower than the class it documented
//!
//! The first version of this module admitted exactly three intervening shapes
//! (`#[`, `///`, `//!`) and cleared the pending flag on anything else, so the
//! gated `mod` opened no region and the whole module body was classified as
//! production. Three shapes Rust and rustfmt both permit fell outside it:
//!
//! 1. a **blank line** between `#[cfg(test)]` and `mod`;
//! 2. a **plain `//` comment** — this one is not hypothetical, it is already
//!    written in this workspace's own idiom at
//!    `crates/nono-proxy/src/credential.rs::tests` (a `#[cfg(test)]`, then two
//!    `#[allow(..)]`s with a three-line `//` rationale between them, then
//!    `mod tests {`);
//! 3. a **multi-line `cfg` attribute** — `#[cfg(all(` on its own line never
//!    matched [`is_cfg_test_attr`] at all, so the flag was never even set.
//!
//! Measured: one blank line before `agent_daemon/launch.rs`'s
//! `mod attestation_gate_tests {` moved **387 lines of test code** into the
//! production half. The window is now the class ("not the gated item"), and
//! attributes are joined across lines until their brackets balance before
//! classification.
//!
//! # Round-4 WR-03: "closer not found" must FAIL, not widen silently
//!
//! The terminator used to be an exact whole-line string match rebuilt as
//! `" ".repeat(indent)` + `}`. When it never matched, `idx` ran to EOF and the
//! region was recorded as running to the last line **with no signal to the
//! caller** — every line after the module start silently dropped from the
//! production half. That is the OVER-claim direction, and it is the worse one:
//! both consumers' non-vacuity floors are *more* satisfied by over-claiming,
//! so nothing could see it. Ordinary triggers: `} // end of tests` (rustfmt
//! preserves a trailing comment after a brace), tab indentation (byte count
//! rebuilt as spaces can never match its own closer), a bare `}` at the
//! module's indent inside a raw string.
//!
//! Unclosed regions are now recorded in [`ProductionScan::unclosed_regions`],
//! which every consumer MUST assert is empty; the closer is matched against
//! the module line's ORIGINAL leading whitespace (so tabs match) and admits a
//! trailing comment.

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
///
/// # Whitespace is removed before matching (round-4 WR-01, shape 3)
///
/// A `cfg` attribute may be written across several source lines. Callers join
/// it back together before calling this (see [`scan_production`]), and the
/// join leaves interior spaces (`#[cfg(all( test ))]`), which no substring
/// needle written for the single-line form would match. Compacting first makes
/// the predicate independent of how the attribute was formatted.
#[must_use]
pub fn is_cfg_test_attr(trimmed: &str) -> bool {
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    compact.starts_with("#[cfg(")
        && !compact.contains("not(test")
        && (compact.contains("test)") || compact.contains("test,"))
}

/// Is `trimmed` a test-function attribute — the thing that must NEVER survive
/// into a production half?
///
/// Matches `#[test]` and the path-qualified forms (`#[tokio::test]`,
/// `#[rstest::test]`). Deliberately a class, not the one literal: a leaked
/// module brings whatever test attribute its functions use, and pinning only
/// `#[test]` would make the leak check itself the narrow needle this review
/// round keeps finding.
#[must_use]
pub fn is_test_attr(trimmed: &str) -> bool {
    trimmed == "#[test]" || (trimmed.starts_with("#[") && trimmed.ends_with("::test]"))
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
    /// Zero-based `mod` line of every region whose closing brace was never
    /// found — the scan ran to EOF and would otherwise have silently dropped
    /// everything after it from the production half.
    ///
    /// Callers MUST assert this is empty. An over-claimed region satisfies
    /// every non-vacuity floor there is (`skipped_lines() > 0`,
    /// `!skipped_regions.is_empty()`) and is invisible to the `#[test]`-leak
    /// check, which only fires in the under-claim direction (WR-03).
    pub unclosed_regions: Vec<usize>,
}

impl ProductionScan<'_> {
    /// Total source lines dropped as `#[cfg(test)]` module bodies.
    ///
    /// **This is a floor, never a correctness check.** It is monotone in the
    /// WRONG direction: mis-classifying test code as production makes it
    /// smaller but keeps it above zero, so `skipped_lines() > 0` stays green
    /// precisely as coverage degrades. Round-4 WR-02 demonstrated that
    /// mechanically. Pair it with [`Self::leaked_test_attributes`], which is
    /// monotone in the right one.
    #[must_use]
    pub fn skipped_lines(&self) -> usize {
        self.skipped_regions
            .iter()
            .map(|(a, b)| b.saturating_sub(*a).saturating_add(1))
            .sum()
    }

    /// Zero-based lines carrying a test-function attribute that survived into
    /// the production half. **Non-empty means the split is WRONG**, not merely
    /// small.
    ///
    /// This is the correctness property every consumer of this module must
    /// assert. A `#[cfg(test)]` module that leaks brings its `#[test]`s with
    /// it, so this catches the class directly and without restating the
    /// classifier's own rule — and unlike a line-count floor it can only be
    /// satisfied by a split that is right.
    #[must_use]
    pub fn leaked_test_attributes(&self) -> Vec<usize> {
        self.lines
            .iter()
            .filter(|(_, l)| is_test_attr(l.trim()))
            .map(|(i, _)| *i)
            .collect()
    }

    /// Assert the split is correct, and fail loudly with the evidence if it is
    /// not. `label` names the scanned file in the panic message.
    ///
    /// Every consumer calls this instead of writing its own floor. Round 3
    /// propagated the shared *classifier* to both consumers but the
    /// *correctness assertion* to only one; this exists so a third consumer
    /// cannot repeat that.
    ///
    /// The three checks are deliberately ordered worst-direction-first:
    ///
    /// 1. `unclosed_regions` — over-claim, production text silently unscanned.
    /// 2. `leaked_test_attributes` — under-claim, test assertions able to
    ///    satisfy the caller's gate.
    /// 3. `skipped_regions` non-empty — the zero-region case neither of the
    ///    above can see.
    ///
    /// # Panics
    ///
    /// If the production/test split of the scanned file is not correct.
    pub fn assert_split_is_correct(&self, label: &str) {
        assert!(
            self.unclosed_regions.is_empty(),
            "WR-03: {} `#[cfg(test)]` module region(s) in {label} were never closed (first at \
             line {:?}), so the scan ran to EOF and every production line after that point is \
             silently absent from this gate. Over-claiming satisfies every line-count floor, \
             so nothing else can see it. Check for a `}}` closer carrying a trailing comment, \
             tab indentation, or a stray brace inside a raw string.",
            self.unclosed_regions.len(),
            self.unclosed_regions.first().map(|i| i + 1)
        );
        let leaked = self.leaked_test_attributes();
        assert!(
            leaked.is_empty(),
            "WR-01/WR-02: {} test attribute(s) survived into the PRODUCTION half of {label} \
             (first at line {:?}), so at least one `#[cfg(test)]` module leaked into the scan \
             and its assertions can satisfy the caller's gate on their own.",
            leaked.len(),
            leaked.first().map(|i| i + 1)
        );
        assert!(
            !self.skipped_regions.is_empty(),
            "non-vacuity: no `#[cfg(test)]` module region was excluded from {label}, so the \
             caller is scanning the whole file including test assertions that legitimately \
             name whatever it is hunting for. Either the file lost its test modules, or \
             cfg_test_regions no longer recognises them."
        );
    }
}

/// The source lines of one attribute starting at `idx`, joined, plus how many
/// lines it spans.
///
/// An attribute may be written across several lines (`#[cfg(all(` … `))]`).
/// Joining until the brackets balance is what lets [`is_cfg_test_attr`] see it
/// at all — round-4 WR-01 shape (3), where the flag was never set and the
/// whole gated module was classified as production.
fn attribute_span(lines: &[&str], idx: usize) -> (String, usize) {
    // A runaway guard: an unbalanced `#[` (a bracket inside a string literal
    // in a `#[doc = "…"]`, say) must not consume the rest of the file. 16 is
    // far above any real attribute in this crate.
    const MAX_SPAN: usize = 16;

    let mut joined = String::new();
    let mut depth: i32 = 0;
    let mut span = 0usize;
    while idx + span < lines.len() && span < MAX_SPAN {
        let piece = lines[idx + span].trim();
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(piece);
        for c in piece.chars() {
            match c {
                '[' | '(' => depth = depth.saturating_add(1),
                ']' | ')' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        span += 1;
        if depth <= 0 {
            break;
        }
    }
    (joined, span.max(1))
}

/// Does `line` close a region opened by a `mod` line indented with `indent`?
///
/// WR-03: matched against the module line's ORIGINAL leading whitespace rather
/// than a `" ".repeat(len)` reconstruction, so tab-indented modules can close;
/// and a trailing comment after the brace (`} // end of tests`, which rustfmt
/// preserves) still closes.
fn is_region_closer(line: &str, indent: &str) -> bool {
    let Some(rest) = line.strip_prefix(indent) else {
        return false;
    };
    // The closer must be at EXACTLY this indent — a deeper brace is a nested
    // block, not the module's own closer.
    if rest.starts_with(char::is_whitespace) {
        return false;
    }
    let Some(after) = rest.strip_prefix('}') else {
        return false;
    };
    let after = after.trim();
    after.is_empty() || after.starts_with("//") || after.starts_with("/*")
}

/// Split `src` into its production half per the rule in this module's doc.
#[must_use]
pub fn scan_production(src: &str) -> ProductionScan<'_> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out: Vec<(usize, &str)> = Vec::new();
    let mut skipped_regions: Vec<(usize, usize)> = Vec::new();
    let mut unclosed_regions: Vec<usize> = Vec::new();
    let mut idx = 0usize;
    let mut pending_test_attr = false;

    while idx < lines.len() {
        let t = lines[idx].trim();

        // Attributes are handled first and as a unit, because a `cfg`
        // attribute may span several lines (WR-01 shape 3).
        if t.starts_with("#[") {
            let (attr, span) = attribute_span(&lines, idx);
            if is_cfg_test_attr(&attr) {
                pending_test_attr = true;
            } else if !pending_test_attr {
                // A production attribute is production text — the `#[test]`
                // leak check depends on those lines being kept.
                let end = idx.saturating_add(span).min(lines.len());
                for (k, line) in lines.iter().enumerate().take(end).skip(idx) {
                    out.push((k, *line));
                }
            }
            // else: an intervening attribute inside a pending window
            // (`#[allow(clippy::unwrap_used)]`) — consumed, flag preserved.
            idx += span;
            continue;
        }

        if pending_test_attr {
            // WR-01: the window is the CLASS — "anything that is not the gated
            // item". Blank lines, plain `//` comments and block-comment bodies
            // may all sit between a `cfg` attribute and the item it gates;
            // admitting only `#[`/`///`/`//!` is what let one blank line move
            // 387 lines of test code into the production half.
            if t.is_empty() || t.starts_with("//") || t.starts_with("/*") || t.starts_with('*') {
                idx += 1;
                continue;
            }
            pending_test_attr = false;
            // An INLINE `mod foo {` opens a test region; a bare `mod foo;`
            // declaration opens nothing.
            if (t.starts_with("mod ") || t.starts_with("pub mod ")) && t.ends_with('{') {
                let start = idx;
                let raw = lines[idx];
                let indent = &raw[..raw.len() - raw.trim_start().len()];
                idx += 1;
                let mut closed_at = None;
                while idx < lines.len() {
                    if is_region_closer(lines[idx], indent) {
                        closed_at = Some(idx);
                        break;
                    }
                    idx += 1;
                }
                match closed_at {
                    Some(end) => {
                        skipped_regions.push((start, end));
                        idx = end + 1;
                    }
                    None => {
                        // WR-03: record it AND report it. The region is still
                        // pushed so `skipped_lines()` stays meaningful, but
                        // `unclosed_regions` is what the caller must assert
                        // on — this direction is invisible to every floor.
                        unclosed_regions.push(start);
                        skipped_regions.push((start, lines.len().saturating_sub(1)));
                    }
                }
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
        unclosed_regions,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_cfg_test_attr, is_test_attr, scan_production};

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

    /// Round-4 WR-01 shape (1): a BLANK LINE between `#[cfg(test)]` and `mod`.
    ///
    /// This is the perturbation round 4 used to prove the daemon gate could be
    /// driven green by a test assertion: inserting one blank line before
    /// `agent_daemon/launch.rs`'s `mod attestation_gate_tests {` moved 387
    /// lines of test code into the production half, and the gate's
    /// `skipped_lines() > 0` floor stayed satisfied throughout.
    #[test]
    fn a_blank_line_before_the_mod_does_not_break_the_region() {
        let src = "\
fn production_a() {}
#[cfg(test)]

mod some_tests {
    #[test]
    fn inside() {}
}
fn production_b() {}
";
        let scan = scan_production(src);
        assert_eq!(
            scan.skipped_regions.len(),
            1,
            "a blank line between the cfg attribute and the `mod` line must not re-classify \
             the module body as production (round-4 WR-01, shape 1)"
        );
        assert!(
            scan.leaked_test_attributes().is_empty(),
            "the module's `#[test]` must not survive into the production half"
        );
        assert!(scan.lines.iter().any(|(_, l)| l.contains("production_b")));
    }

    /// Round-4 WR-01 shape (2): a PLAIN `//` comment between the cfg attribute
    /// and the `mod` line.
    ///
    /// The fixture is the real occurrence, not a synthetic one: this is the
    /// shape written in `crates/nono-proxy/src/credential.rs`'s test module,
    /// in this codebase's own idiom, today.
    #[test]
    fn a_plain_line_comment_before_the_mod_does_not_break_the_region() {
        let src = "\
fn production_a() {}
#[cfg(test)]
#[allow(clippy::unwrap_used)]
// The env-guard below mutates process env via set_var/remove_var with a
// symmetric Drop restore (the CLAUDE.md-endorsed save/restore test pattern);
// the disallowed_methods ban targets non-test misuse, so scope an allow.
#[allow(clippy::disallowed_methods)]
mod tests {
    #[test]
    fn inside() {}
}
fn production_b() {}
";
        let scan = scan_production(src);
        assert_eq!(
            scan.skipped_regions.len(),
            1,
            "a plain `//` comment between the cfg attribute and the `mod` line must not \
             re-classify the module body as production (round-4 WR-01, shape 2)"
        );
        assert!(scan.leaked_test_attributes().is_empty());
        assert!(scan.lines.iter().any(|(_, l)| l.contains("production_b")));
    }

    /// Round-4 WR-01 shape (3): a MULTI-LINE `cfg` attribute. `#[cfg(all(` on
    /// its own line failed `is_cfg_test_attr` outright, so the pending flag
    /// was never even set.
    #[test]
    fn a_multi_line_cfg_attribute_still_opens_the_region() {
        let src = "\
fn production_a() {}
#[cfg(all(
    test,
    target_os = \"windows\"
))]
#[allow(clippy::unwrap_used)]
mod some_tests {
    #[test]
    fn inside() {}
}
fn production_b() {}
";
        let scan = scan_production(src);
        assert_eq!(
            scan.skipped_regions.len(),
            1,
            "a `cfg` attribute split across source lines must still be recognised (round-4 \
             WR-01, shape 3)"
        );
        assert!(scan.leaked_test_attributes().is_empty());
        assert!(scan.lines.iter().any(|(_, l)| l.contains("production_b")));
    }

    /// A production attribute must stay in the production half — the
    /// `#[test]`-leak check is built out of exactly those lines, so dropping
    /// attributes wholesale would make it permanently vacuous.
    #[test]
    fn production_attributes_are_production_text() {
        let src = "\
#[derive(Debug)]
struct S;
#[test]
fn a_leaked_test() {}
";
        let scan = scan_production(src);
        assert!(scan
            .lines
            .iter()
            .any(|(_, l)| l.contains("#[derive(Debug)]")));
        assert_eq!(
            scan.leaked_test_attributes(),
            vec![2],
            "a `#[test]` outside any cfg-test region IS a leak and must be reported"
        );
    }

    /// The test-attribute class, asserted directly. A leak check pinned to the
    /// single literal `#[test]` would be the narrow needle this review round
    /// keeps finding.
    #[test]
    fn test_attribute_classification_rule() {
        for yes in ["#[test]", "#[tokio::test]", "#[rstest::test]"] {
            assert!(
                is_test_attr(yes),
                "{yes:?} must classify as a test attribute"
            );
        }
        for no in [
            "#[cfg(test)]",
            "#[allow(clippy::unwrap_used)]",
            "fn test() {}",
        ] {
            assert!(
                !is_test_attr(no),
                "{no:?} must NOT classify as a test attribute"
            );
        }
    }

    /// Round-4 WR-03: a closer carrying a trailing comment still closes.
    ///
    /// Without this the region ran to EOF and swallowed `production_b`, with
    /// no signal to the caller — the over-claim direction, which every
    /// line-count floor is *more* satisfied by.
    #[test]
    fn a_closer_with_a_trailing_comment_still_closes_the_region() {
        let src = "\
#[cfg(test)]
mod some_tests {
    fn inside() {}
} // end of some_tests
fn production_b() {}
";
        let scan = scan_production(src);
        assert!(
            scan.unclosed_regions.is_empty(),
            "`}} // end of ...` is an ordinary rustfmt-preserved shape and must close the region"
        );
        // Inclusive of the `mod` line (1) through the commented closer (3).
        assert_eq!(scan.skipped_regions, vec![(1, 3)]);
        assert!(scan.lines.iter().any(|(_, l)| l.contains("production_b")));
    }

    /// Round-4 WR-03: tab-indented nested modules close at their own indent.
    ///
    /// The old closer rebuilt the indent as `" ".repeat(byte_len)`, so a
    /// tab-indented module could never match its own closing brace.
    #[test]
    fn tab_indented_regions_close_at_their_own_indent() {
        let src = "mod outer {\n\t#[cfg(test)]\n\tmod inner {\n\t\tfn t() {}\n\t}\n\tfn production_sibling() {}\n}\n";
        let scan = scan_production(src);
        assert!(
            scan.unclosed_regions.is_empty(),
            "a tab-indented region must close, not run to EOF"
        );
        // Inclusive of the tab-indented `mod` line (2) through its tab-indented
        // closer (4).
        assert_eq!(scan.skipped_regions, vec![(2, 4)]);
        assert!(scan
            .lines
            .iter()
            .any(|(_, l)| l.contains("production_sibling")));
    }

    /// Round-4 WR-03: when the closer genuinely cannot be found, the scan must
    /// SAY SO rather than silently widening the region to EOF.
    #[test]
    fn an_unclosed_region_is_reported_not_swallowed() {
        // The module is opened at indent 4 and never closed at indent 4.
        let src = "\
mod outer {
    #[cfg(test)]
    mod inner {
        fn t() {}
";
        let scan = scan_production(src);
        assert_eq!(
            scan.unclosed_regions,
            vec![2],
            "an unclosed region must be reported: every non-vacuity floor is MORE satisfied \
             by over-claiming, so this is the only signal a caller can act on"
        );
    }

    /// `assert_split_is_correct` must FAIL on an unclosed region — the check
    /// exists precisely because no line-count floor can see this direction.
    #[test]
    #[should_panic(expected = "WR-03")]
    fn assert_split_is_correct_rejects_an_unclosed_region() {
        let src = "\
mod outer {
    #[cfg(test)]
    mod inner {
        fn t() {}
";
        scan_production(src).assert_split_is_correct("fixture.rs");
    }

    /// `assert_split_is_correct` must FAIL on a leaked test attribute.
    #[test]
    #[should_panic(expected = "WR-01/WR-02")]
    fn assert_split_is_correct_rejects_a_leaked_test_attribute() {
        let src = "\
#[cfg(test)]
mod real_region {
    fn helper() {}
}
#[test]
fn leaked() {}
";
        scan_production(src).assert_split_is_correct("fixture.rs");
    }

    /// `assert_split_is_correct` must FAIL when nothing was skipped at all —
    /// the zero-region case neither of the other two checks can see.
    #[test]
    #[should_panic(expected = "non-vacuity")]
    fn assert_split_is_correct_rejects_a_scan_with_no_regions() {
        scan_production("fn only_production() {}\n").assert_split_is_correct("fixture.rs");
    }
}
