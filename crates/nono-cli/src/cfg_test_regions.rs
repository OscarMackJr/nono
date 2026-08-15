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
//! A region is skipped when a `#[cfg(...)]` attribute that gates **test-only**
//! code (see [`is_cfg_test_attr`]) is followed — possibly across
//! **anything that is not the gated item** — by that item. The item may be of
//! any kind (round-8 WR-01): an inline `mod foo {`, a `fn`, a `const`, a
//! `static`, a `use`, a bare `mod foo;` declaration.
//!
//! A block item's region ends at the closing brace at the ITEM's own
//! indentation, so test modules nested inside another module
//! (`agent_daemon/launch.rs`'s live inside `mod windows_impl`) close
//! correctly. A self-contained item — one whose code text is brace-balanced
//! and ends in `;` or `}` — is its own region, which is how a bare
//! `#[cfg(test)] mod test_env;` declaration (`main.rs:156`) is skipped without
//! a closer search: treating that as a BLOCK region start silently truncated
//! the `main.rs` half of a gate to nothing.
//!
//! Comments are dropped as well: prose ABOUT a rule is not an instance of it,
//! and a comment naming the thing a gate hunts for would otherwise satisfy the
//! gate.
//!
//! # Round-6 WR-02: one rule for "is this line code", stated once
//!
//! There used to be TWO comment rules in this module, both enumerations:
//! the whole-scan rule dropped lines whose trimmed text started with `//`
//! (block comments were kept, so a `/* … */` naming a gate's needle satisfied
//! that gate), and the pending-attribute window admitted the four shapes
//! `""` / `//` / `/*` / `*`. A block comment whose body lines are not
//! `*`-aligned fell outside the second one:
//!
//! ```text
//! #[cfg(test)]
//! /* hello
//!    world */
//! mod t { #[test] fn x() {} }
//! ```
//!
//! `world */` is none of those four shapes, so the pending flag cleared, the
//! module opened no region, and its whole body was classified as production —
//! WR-01's failure mode reopened by a third intervening token.
//!
//! Both rules are now the same one, [`code_text`], defined **positively**:
//! code is everything on a line that is not comment.
//! Rust's block comments nest, so the state carried between lines is a depth
//! counter rather than a flag, and it is carried through region bodies too (so
//! a `}` inside a block comment cannot close a region). The pending window no
//! longer has a shape list at all: between an attribute and the item it gates,
//! Rust permits attributes, comments and blank lines and nothing else, so
//! "not a code line" IS the window.
//!
//! A block comment left open at EOF would silently swallow every line after it
//! — the over-claim direction again — so it is recorded in
//! [`ProductionScan::unterminated_block_comment`] and asserted alongside
//! [`ProductionScan::unclosed_regions`].
//!
//! # Round-8 WR-02: the comment rule was still a first-two-bytes test
//!
//! [`code_text`]'s predecessor only entered block-comment state from a line
//! whose TRIMMED text starts with `/*`, and it returned a yes/no rather than
//! text. Two shapes therefore put comment prose into the production half:
//!
//! ```text
//! let x = 1; /* NOTE: this mentions the
//!    Windows Application event log in prose only */
//! ```
//!
//! The comment was never opened, so line 2 — pure prose naming the WR-26
//! gate's exact needle — was ordinary code to every consumer; and line 1 was
//! kept RAW, comment and all, so even closing that gap would have left
//! `let x = 1; /* DaclAncestorTraverse */` able to satisfy the daemon gate.
//! Both directions are invisible to all four checks in
//! [`ProductionScan::assert_split_is_correct`], and the first is **fail-open**
//! for `layer_registry.rs`.
//!
//! The rule is now a lexer over the whole line, and
//! [`ProductionScan::lines`] carries CODE TEXT rather than raw lines. Entering
//! a comment mid-line means knowing where the literals are, so `"`, `'`, `b"`,
//! `r"` and `r#"…"#` are lexed and their contents are code, not comment
//! openers. The one limit retained deliberately — a comment-shaped line inside
//! a MULTI-LINE string literal reads as a comment — is stated on [`code_text`]
//! together with why closing it would be worse.
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

/// Three-valued truth for a `cfg` predicate evaluated with `test` pinned to a
/// known value and **every other atom left unknown**.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tri {
    True,
    False,
    Unknown,
}

/// `all(..)`: false if any operand is false, true only if every one is true.
/// `all()` with no operands is true, as in Rust.
fn tri_all(operands: &[Tri]) -> Tri {
    if operands.contains(&Tri::False) {
        Tri::False
    } else if operands.iter().all(|t| *t == Tri::True) {
        Tri::True
    } else {
        Tri::Unknown
    }
}

/// `any(..)`: true if any operand is true, false only if every one is false.
/// `any()` with no operands is false, as in Rust.
fn tri_any(operands: &[Tri]) -> Tri {
    if operands.contains(&Tri::True) {
        Tri::True
    } else if operands.iter().all(|t| *t == Tri::False) {
        Tri::False
    } else {
        Tri::Unknown
    }
}

fn tri_not(operand: Tri) -> Tri {
    match operand {
        Tri::True => Tri::False,
        Tri::False => Tri::True,
        Tri::Unknown => Tri::Unknown,
    }
}

/// A byte cursor over a whitespace-compacted `cfg` predicate.
struct Cursor<'a> {
    bytes: &'a [u8],
    idx: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.idx).copied()
    }

    fn bump(&mut self) {
        self.idx = self.idx.saturating_add(1);
    }

    /// A `cfg` atom or combinator name: ASCII word characters plus `:` so
    /// path-shaped atoms do not silently truncate.
    fn ident(&mut self) -> Option<&'a str> {
        let start = self.idx;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_' || c == b':')
        {
            self.bump();
        }
        if self.idx == start {
            return None;
        }
        std::str::from_utf8(self.bytes.get(start..self.idx)?).ok()
    }

    /// Consume a `"…"` literal, honouring backslash escapes. Its CONTENT is
    /// never inspected: `feature = "test"` is a feature named `test`, not the
    /// `test` cfg, and must evaluate to [`Tri::Unknown`].
    fn skip_string(&mut self) -> Option<()> {
        if self.peek() != Some(b'"') {
            return None;
        }
        self.bump();
        loop {
            match self.peek() {
                None => return None,
                Some(b'\\') => {
                    self.bump();
                    self.bump();
                }
                Some(b'"') => {
                    self.bump();
                    return Some(());
                }
                Some(_) => self.bump(),
            }
        }
    }
}

/// Guard against a pathological nesting depth; real `cfg` predicates are 1-3
/// deep.
const MAX_CFG_DEPTH: u32 = 32;

/// Evaluate one `cfg` predicate at the cursor with `test` bound to
/// `test_is_on` and every other atom [`Tri::Unknown`].
///
/// `None` means "this evaluator could not parse it", which callers must treat
/// as *not* a test gate — see [`is_cfg_test_attr`]'s fail direction.
fn eval_cfg_predicate(cursor: &mut Cursor, test_is_on: bool, depth: u32) -> Option<Tri> {
    if depth > MAX_CFG_DEPTH {
        return None;
    }
    let name = cursor.ident()?;
    match cursor.peek() {
        // A combinator: `all(..)`, `any(..)`, `not(..)`.
        Some(b'(') => {
            cursor.bump();
            let mut operands: Vec<Tri> = Vec::new();
            if cursor.peek() == Some(b')') {
                cursor.bump();
            } else {
                loop {
                    operands.push(eval_cfg_predicate(
                        cursor,
                        test_is_on,
                        depth.saturating_add(1),
                    )?);
                    match cursor.peek() {
                        Some(b',') => {
                            cursor.bump();
                            // A trailing comma before the closer.
                            if cursor.peek() == Some(b')') {
                                cursor.bump();
                                break;
                            }
                        }
                        Some(b')') => {
                            cursor.bump();
                            break;
                        }
                        _ => return None,
                    }
                }
            }
            match name {
                "all" => Some(tri_all(&operands)),
                "any" => Some(tri_any(&operands)),
                "not" if operands.len() == 1 => Some(tri_not(operands[0])),
                // An unrecognised combinator, or a `not` of the wrong arity,
                // is something this evaluator cannot reason about.
                _ => None,
            }
        }
        // A key/value atom: `target_os = "windows"`, `feature = "x"`.
        Some(b'=') => {
            cursor.bump();
            cursor.skip_string()?;
            Some(Tri::Unknown)
        }
        // A bare atom: `test`, `unix`, `windows`, `doc`.
        _ => Some(if name == "test" {
            if test_is_on {
                Tri::True
            } else {
                Tri::False
            }
        } else {
            Tri::Unknown
        }),
    }
}

/// The text inside `#[cfg( … )]`, or `None` if `compact` is not a `cfg`
/// attribute. Parenthesis matching skips string literals, so
/// `feature = "a)b"` cannot terminate the attribute early.
fn cfg_attribute_body(compact: &str) -> Option<&str> {
    let rest = compact.strip_prefix("#[cfg(")?;
    let bytes = rest.as_bytes();
    let mut depth: usize = 1;
    let mut idx: usize = 0;
    let mut in_string = false;
    while idx < bytes.len() {
        let c = bytes[idx];
        if in_string {
            match c {
                b'\\' => idx = idx.saturating_add(1),
                b'"' => in_string = false,
                _ => {}
            }
        } else {
            match c {
                b'"' => in_string = true,
                b'(' => depth = depth.saturating_add(1),
                b')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        // The attribute must close immediately after the
                        // predicate. Anything further along the line (an
                        // inline `mod foo {`) is not part of it.
                        return if bytes.get(idx.saturating_add(1)) == Some(&b']') {
                            rest.get(..idx)
                        } else {
                            None
                        };
                    }
                }
                _ => {}
            }
        }
        idx = idx.saturating_add(1);
    }
    None
}

/// Is `trimmed` (an already-trimmed, possibly multi-line-joined line) a `cfg`
/// attribute that gates **test-only** code?
///
/// # The rule, stated once
///
/// An attribute gates test-only code exactly when its predicate is FALSE in
/// every build where `test` is off. This is evaluated rather than
/// pattern-matched: the predicate is parsed, `test` is bound to `false`, every
/// other atom is left UNKNOWN, and the attribute is a test gate iff the
/// three-valued result is definitely `False`. That one rule settles `all`,
/// `any`, `not` and every nesting of them without a case per shape:
///
/// | Attribute | With `test = false` | Test gate? |
/// |---|---|---|
/// | `#[cfg(test)]` | `False` | yes |
/// | `#[cfg(all(test, target_os = "windows"))]` | `all(False, ?)` = `False` | yes |
/// | `#[cfg(any(all(test, unix), all(test, windows)))]` | `any(False, False)` = `False` | yes |
/// | `#[cfg(any(test, feature = "x"))]` | `any(False, ?)` = `?` | **no** |
/// | `#[cfg(not(test))]` | `not(False)` = `True` | no |
/// | `#[cfg(all(not(test), unix))]` | `all(True, ?)` = `?` | no |
/// | `#[cfg(target_os = "windows")]` | `?` | no |
///
/// # Round-6 WR-01: why `any(test, …)` is the direction that matters
///
/// The predicate this replaced was three substring tests
/// (`starts_with("#[cfg(")`, `!contains("not(test")`,
/// `contains("test)") || contains("test,")`). `#[cfg(any(test, feature = "x"))]`
/// contains `test,` and not `not(test`, so it classified as a **test gate** —
/// and a module gated that way is compiled into production builds whenever the
/// other disjunct holds, so the classifier deleted a *production* module's
/// entire body from the scan.
///
/// That is the OVER-claim direction, and it is invisible to all three checks in
/// [`ProductionScan::assert_split_is_correct`]: the region closes fine, a
/// production module carries no test attribute to leak, and `skipped_regions`
/// is *more* non-empty rather than less. A gate whose production needle lived in
/// that module would go green by absence with no signal at all. The shape is
/// written in this crate today, at
/// `session_commands.rs`'s `#[cfg(any(test, target_os = "macos", target_os = "windows"))] fn format_bytes_human`.
///
/// # Fail direction on a predicate this cannot parse
///
/// `None` from the evaluator (an unknown combinator, malformed nesting,
/// runaway depth) yields `false` — NOT a test gate. The gated body then stays
/// in the production half, where a leaked `#[test]` is reported loudly by
/// [`ProductionScan::assert_split_is_correct`]. The opposite default would
/// delete source silently.
///
/// # Whitespace is removed before parsing (round-4 WR-01, shape 3)
///
/// A `cfg` attribute may be written across several source lines. Callers join
/// it back together before calling this (see [`scan_production`]), and the join
/// leaves interior spaces (`#[cfg(all( test ))]`). Compacting first makes the
/// predicate independent of how the attribute was formatted.
#[must_use]
pub fn is_cfg_test_attr(trimmed: &str) -> bool {
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    let Some(body) = cfg_attribute_body(&compact) else {
        return false;
    };
    let mut cursor = Cursor {
        bytes: body.as_bytes(),
        idx: 0,
    };
    let Some(without_test) = eval_cfg_predicate(&mut cursor, false, 0) else {
        return false;
    };
    // Trailing text the grammar above did not account for means this is not a
    // predicate we understand. Fail toward "not a test gate".
    if cursor.idx != body.len() {
        return false;
    }
    without_test == Tri::False
}

/// Is `trimmed` a test-function attribute — the thing that must NEVER survive
/// into a production half?
///
/// # The rule, stated once
///
/// An attribute is a test attribute when the **last path segment of its name,
/// with any argument list stripped**, is `test`. That covers `#[test]`,
/// `#[tokio::test]`, `#[async_std::test]`, and — the round-6 WR-03 gap — every
/// PARENTHESISED form: `#[tokio::test(flavor = "multi_thread")]`,
/// `#[rstest::test(case(1))]`, `#[test(harness)]`.
///
/// Two bare-ident harness attributes are matched by NAME as well, because no
/// shape rule can reach them: `#[rstest]` and `#[test_case(…)]` mark test
/// functions while looking like any other attribute. That part of the coverage
/// is an enumeration and is stated as one rather than dressed up as a rule.
///
/// # Why the width of this predicate decides what the whole module can see
///
/// [`ProductionScan::leaked_test_attributes`] is the ONLY check in
/// [`ProductionScan::assert_split_is_correct`] that fires in the under-claim
/// direction, so this is the width that decides whether a classifier gap — like
/// the block-comment window WR-02 closed — is caught or silent. The predicate
/// this replaced was two exact forms (`== "#[test]"` and a `"::test]"` suffix),
/// which no parenthesised attribute matches, under a doc comment that already
/// called itself a class. The crate has 92 plain `#[tokio::test]` and zero
/// parenthesised forms today, so the gap was latent — the same standard under
/// which `not(test)` was closed pre-emptively.
#[must_use]
pub fn is_test_attr(trimmed: &str) -> bool {
    let Some(body) = trimmed.strip_prefix("#[").and_then(|b| b.strip_suffix(']')) else {
        return false;
    };
    // `tokio::test(flavor = "multi_thread")` -> `tokio::test` -> `test`.
    let head = body.split('(').next().unwrap_or(body).trim();
    let last_segment = head.rsplit("::").next().unwrap_or(head);
    last_segment == "test" || matches!(head, "rstest" | "test_case")
}

/// Is `b` a byte that can appear inside a Rust identifier?
///
/// Used to tell the literal PREFIXES `r`/`b`/`br` from the last letter of an
/// ordinary identifier: the `r` in `for` opens no raw string.
fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Byte length of the char literal starting at `start`, or `None` when what is
/// there is not one.
///
/// The case this exists for is the LIFETIME. `fn f<'a>(x: &'a str)` has four
/// `'` on one line and none of them opens a literal; reading the first as one
/// would consume up to the next `'` and, with it, any `/*` in between —
/// silently keeping a comment's continuation lines as code, which is the
/// direction round-8 WR-02 is about.
fn char_literal_len(s: &[u8], start: usize) -> Option<usize> {
    let mut j = start.saturating_add(1);
    if s.get(j) == Some(&b'\\') {
        // `'\n'`, `'\''`, `'\u{1F600}'` — all short, and all closed on the
        // same line.
        j = j.saturating_add(1);
        let limit = j.saturating_add(10).min(s.len());
        while j < limit {
            if s[j] == b'\'' {
                return Some(j.saturating_add(1).saturating_sub(start));
            }
            j = j.saturating_add(1);
        }
        return None;
    }
    let rest = std::str::from_utf8(s.get(j..)?).ok()?;
    let c = rest.chars().next()?;
    let end = j.saturating_add(c.len_utf8());
    (s.get(end) == Some(&b'\'')).then(|| end.saturating_add(1).saturating_sub(start))
}

/// Byte length of the string, byte-string, raw-string or char literal starting
/// at `start` in `line`, or `None` when no literal starts there.
///
/// An UNTERMINATED literal reports the rest of the line. A Rust string may
/// span lines, and consuming the remainder is what keeps a `/*` or `//` inside
/// one from being lexed as a comment opener without carrying string state
/// between lines — see the limits on [`code_text`].
fn literal_len(line: &str, start: usize) -> Option<usize> {
    let s = line.as_bytes();
    let prev_is_ident = start
        .checked_sub(1)
        .and_then(|p| s.get(p))
        .is_some_and(|c| is_ident_byte(*c));
    let mut i = start;
    let mut hashes = 0usize;
    let mut raw = false;
    if !prev_is_ident {
        // `b"…"` / `br#"…"#`
        if s.get(i) == Some(&b'b') && matches!(s.get(i.saturating_add(1)), Some(&b'"' | &b'r')) {
            i = i.saturating_add(1);
        }
        // `r"…"` / `r#"…"#` — the `#` count must be matched by the terminator,
        // which is the whole point of a raw string: `r"C:\x"` has no escapes.
        if s.get(i) == Some(&b'r') {
            let mut j = i.saturating_add(1);
            while s.get(j) == Some(&b'#') {
                hashes = hashes.saturating_add(1);
                j = j.saturating_add(1);
            }
            if s.get(j) == Some(&b'"') {
                raw = true;
                i = j;
            }
        }
    }
    match s.get(i) {
        Some(&b'"') => {
            let mut j = i.saturating_add(1);
            while j < s.len() {
                if raw {
                    if s[j] == b'"' {
                        let mut k = j.saturating_add(1);
                        let mut seen = 0usize;
                        while seen < hashes && s.get(k) == Some(&b'#') {
                            seen = seen.saturating_add(1);
                            k = k.saturating_add(1);
                        }
                        if seen == hashes {
                            return Some(k.saturating_sub(start));
                        }
                    }
                    j = j.saturating_add(1);
                } else {
                    match s[j] {
                        b'\\' => j = j.saturating_add(2),
                        b'"' => return Some(j.saturating_add(1).saturating_sub(start)),
                        _ => j = j.saturating_add(1),
                    }
                }
            }
            Some(s.len().saturating_sub(start))
        }
        // Only when the literal starts exactly here: a `b`/`r` prefix followed
        // by `'` is not a byte-char literal opener at THIS position (`b'x'` is
        // reached one byte later, at the quote itself).
        Some(&b'\'') if i == start => char_literal_len(s, start),
        _ => None,
    }
}

/// The CODE text of `line`, with comments removed, given the block-comment
/// nesting `depth` carried in from the line above. `depth` is updated for the
/// next line.
///
/// This is the module's ONE comment rule (round-6 WR-02, widened by round-8
/// WR-02). It is defined positively — code is everything that is not comment —
/// so every caller that needs it (the whole-file scan, the pending-attribute
/// window, the attribute joiner, the `#[test]`-leak check) asks the same
/// question instead of each carrying its own list of comment renderings.
///
/// Rust's block comments NEST, so the carried state is a depth counter and not
/// a flag.
///
/// # Round-8 WR-02: comment state is entered MID-LINE, and the result is text
///
/// The previous rule only entered block-comment state from a line whose
/// TRIMMED text starts with `/*`. Two things followed, and the doc named only
/// the first:
///
/// 1. false DROP — a comment-shaped line inside a multi-line string literal is
///    treated as a comment. Loud for a needle-must-be-present gate.
/// 2. false KEEP — `let x = 1; /* …` never opened a comment at all, so every
///    continuation line of that comment was ordinary code to every consumer.
///    Comment prose reached the production half, which is **fail-open** for
///    `layer_registry.rs`'s daemon gate (a `/* … DaclAncestorTraverse … */`
///    satisfies it) and a false failure for `output.rs`'s WR-26 gate. No check
///    in [`ProductionScan::assert_split_is_correct`] can see either.
///
/// (2) is closed by lexing the whole line rather than sniffing its first two
/// bytes, and by returning the code TEXT rather than a yes/no: a trailing
/// comment on a code line used to travel into the production half attached to
/// its line, so `let x = 1; /* DaclAncestorTraverse */` satisfied the daemon
/// gate on the opening line even once its continuation lines were dropped.
/// The production half is now code, in the sense this module's doc has claimed
/// since it was written.
///
/// Entering mid-line requires knowing where the literals are, so `"`, `'`,
/// `b"`, `r"` and `r#"…"#` are lexed (see [`literal_len`]) and their contents
/// are code, not comment openers. `'` is only a literal when it CLOSES like
/// one, so a lifetime cannot swallow the rest of a line.
///
/// # Deliberate limits, in both directions
///
/// String state is NOT carried between lines. A string literal left open at
/// end of line consumes the rest of that line, but the next line is lexed from
/// depth `0` again, so a `//`- or `/*`-shaped line INSIDE a multi-line string
/// is read as a comment — limit (1) above, retained deliberately. Carrying
/// string state would let one mis-lexed quote swallow the rest of the file in
/// the silent over-claim direction, and unlike
/// [`ProductionScan::unterminated_block_comment`] there is no cheap check that
/// can see it.
fn code_text(line: &str, depth: &mut usize) -> String {
    let s = line.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;
    // `Some(start)` while collecting code; `None` while inside a comment.
    let mut seg: Option<usize> = (*depth == 0).then_some(0);
    let mut stop = s.len();
    while i < s.len() {
        if *depth > 0 {
            // Inside a block comment nothing else is lexical — not a string,
            // not a char literal. Only `/*` and `*/` matter, and they nest.
            if s[i] == b'*' && s.get(i.saturating_add(1)) == Some(&b'/') {
                *depth = depth.saturating_sub(1);
                i = i.saturating_add(2);
                if *depth == 0 {
                    seg = Some(i);
                }
            } else if s[i] == b'/' && s.get(i.saturating_add(1)) == Some(&b'*') {
                *depth = depth.saturating_add(1);
                i = i.saturating_add(2);
            } else {
                i = i.saturating_add(1);
            }
            continue;
        }
        if s[i] == b'/' && s.get(i.saturating_add(1)) == Some(&b'/') {
            stop = i;
            break;
        }
        if s[i] == b'/' && s.get(i.saturating_add(1)) == Some(&b'*') {
            if let Some(start) = seg.take() {
                out.push_str(line.get(start..i).unwrap_or_default());
            }
            *depth = 1;
            i = i.saturating_add(2);
            continue;
        }
        if matches!(s[i], b'"' | b'\'' | b'r' | b'b') {
            if let Some(len) = literal_len(line, i) {
                i = i.saturating_add(len);
                continue;
            }
        }
        i = i.saturating_add(1);
    }
    if let Some(start) = seg {
        out.push_str(line.get(start..stop).unwrap_or_default());
    }
    out
}

/// The production half of a source file, plus the evidence a caller needs to
/// prove the split actually happened.
pub struct ProductionScan {
    /// Production lines as `(zero-based source index, CODE text)`. Comments —
    /// whole-line, trailing, and block, wherever they open — and
    /// `#[cfg(test)]`-gated items are absent.
    ///
    /// Round-8 WR-02: this used to be the RAW line, so a trailing comment
    /// travelled into the production half attached to its code. That let a
    /// comment naming the thing a gate hunts for satisfy the gate, which is
    /// the failure this module's doc opens by promising to prevent.
    pub lines: Vec<(usize, String)>,
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
    /// Zero-based line at which a block comment was opened and never closed
    /// before end of file. Every line after it was dropped as comment text.
    ///
    /// Same over-claim direction as [`Self::unclosed_regions`] and the same
    /// obligation: callers MUST assert it is `None`. Round-6 WR-02 made block
    /// comments a tracked, nesting-aware state; this is the failure mode that
    /// state introduces, surfaced rather than left silent.
    pub unterminated_block_comment: Option<usize>,
}

impl ProductionScan {
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
    /// The four checks are deliberately ordered worst-direction-first:
    ///
    /// 1. `unterminated_block_comment` — over-claim, everything after the
    ///    unclosed `/*` dropped as comment text.
    /// 2. `unclosed_regions` — over-claim, production text silently unscanned.
    /// 3. `leaked_test_attributes` — under-claim, test assertions able to
    ///    satisfy the caller's gate.
    /// 4. `skipped_regions` non-empty — the zero-region case none of the
    ///    above can see.
    ///
    /// # Panics
    ///
    /// If the production/test split of the scanned file is not correct.
    pub fn assert_split_is_correct(&self, label: &str) {
        assert!(
            self.unterminated_block_comment.is_none(),
            "WR-02: a block comment opened at {label}:{:?} was never closed, so every line \
             after it was dropped as comment text and is silently absent from this gate. \
             Over-claiming satisfies every line-count floor, so nothing else can see it.",
            self.unterminated_block_comment.map(|i| i + 1)
        );
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
        // Comments first (round-8 WR-02): brackets inside a trailing comment
        // are not the attribute's brackets, and counting them extended the
        // span past the attribute — `#[cfg(test)] // opens the tests (see …`
        // would have swallowed the gated item's own line.
        let piece_owned = code_text(lines[idx + span], &mut 0);
        let piece = piece_owned.trim();
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

/// Block-comment nesting carried across lines, plus where the outermost open
/// comment began (for [`ProductionScan::unterminated_block_comment`]).
#[derive(Default)]
struct CommentState {
    depth: usize,
    opened_at: Option<usize>,
}

impl CommentState {
    /// Feed the zero-based line `idx`; returns its CODE text, or `None` when
    /// the line contributes none.
    fn feed(&mut self, line: &str, idx: usize) -> Option<String> {
        let was_open = self.depth > 0;
        let code = code_text(line, &mut self.depth);
        if self.depth == 0 {
            self.opened_at = None;
        } else if !was_open {
            self.opened_at = Some(idx);
        }
        (!code.trim().is_empty()).then_some(code)
    }
}

/// Split `src` into its production half per the rule in this module's doc.
#[must_use]
pub fn scan_production(src: &str) -> ProductionScan {
    let lines: Vec<&str> = src.lines().collect();
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut skipped_regions: Vec<(usize, usize)> = Vec::new();
    let mut unclosed_regions: Vec<usize> = Vec::new();
    let mut idx = 0usize;
    let mut pending_test_attr = false;
    let mut comments = CommentState::default();

    while idx < lines.len() {
        // The ONE comment rule, applied before anything else (round-6 WR-02).
        // Inside a comment nothing is an attribute, an item, or production
        // text — and a non-code line never ENDS a pending attribute window,
        // because between an attribute and the item it gates Rust permits
        // attributes, comments and blank lines and nothing else.
        let Some(code) = comments.feed(lines[idx], idx) else {
            idx += 1;
            continue;
        };

        let t = code.trim();

        // Attributes are handled first and as a unit, because a `cfg`
        // attribute may span several lines (WR-01 shape 3).
        if t.starts_with("#[") {
            let (attr, span) = attribute_span(&lines, idx);
            let end = idx.saturating_add(span).min(lines.len());
            // A production attribute is production text — the `#[test]` leak
            // check depends on those lines being kept. An intervening
            // attribute inside a pending window (`#[allow(clippy::unwrap_used)]`)
            // is consumed and the flag preserved.
            let is_gate = is_cfg_test_attr(&attr);
            let keep = !is_gate && !pending_test_attr;
            if is_gate {
                pending_test_attr = true;
            }
            if keep {
                out.push((idx, code));
            }
            // The attribute's remaining lines go through the SAME carried
            // comment state, so a block comment opened inside a multi-line
            // attribute stays coherent afterwards instead of being skipped
            // unlexed.
            let first_rest = idx.saturating_add(1);
            for (k, line) in lines.iter().enumerate().take(end).skip(first_rest) {
                if let Some(rest) = comments.feed(line, k) {
                    if keep {
                        out.push((k, rest));
                    }
                }
            }
            idx = end;
            continue;
        }

        if pending_test_attr {
            // WR-01/WR-02: the window needs no shape list. Every non-code line
            // was already consumed by `comments.feed` above, and attributes by
            // the branch above that, so reaching here means this line IS the
            // gated item.
            pending_test_attr = false;
            let start = idx;
            let raw = lines[idx];
            let indent = &raw[..raw.len() - raw.trim_start().len()];

            // ROUND-8 WR-01: the gated ITEM is skipped, whatever kind of item
            // it is — not only an inline `mod`.
            //
            // The fall-through this replaces kept a `#[cfg(test)]`-gated
            // `fn`/`const`/`static`/`use` as production text under a comment
            // asserting the opposite ("It is not production text either way"),
            // and kept every line of its body too, because nothing tracked the
            // item's extent. That is live in two of the four scanned files:
            // `agent_daemon/launch.rs`'s `WFP_CONTROL_PIPE_NAME_TESTABLE` and
            // `profile_needs_network_scoping_testable`, and `output.rs`'s
            // `SESSIONS_ROOT_TEST_LOCK`. Both files are scanned by gates that
            // ask "does the PRODUCTION half name X", so a `_testable` helper
            // naming a layer would satisfy the gate on its own — the failure
            // this module's doc opens with, one granularity below the
            // module-level case it already closes. All four checks in
            // `assert_split_is_correct` are blind to it: the region list is
            // non-empty, nothing is unclosed, and a gated `fn` carries no
            // `#[test]` to leak.
            //
            // The extent rule is the item's own SHAPE, resolved over code text:
            //
            //   * balanced and ending in `;` or `}` -> the item is this line
            //     (`const X: &str = "…";`, `use a::b;`, `mod t { … }`);
            //   * a line that opens a block -> the item ends at the closer at
            //     its OWN indent, the same rule `mod` has always used, chosen
            //     over brace counting because a brace inside a raw string
            //     closes early in the LOUD direction (a leaked `#[test]`)
            //     rather than swallowing the rest of the file;
            //   * neither, for up to `MAX_ITEM_HEADER` lines -> a multi-line
            //     header (`pub fn f(\n a: u32,\n) -> bool {`) that has not
            //     resolved yet.
            //
            // An item whose extent cannot be resolved at all is recorded in
            // `unclosed_regions`, so it FAILS loudly instead of picking one of
            // the two silent directions.
            const MAX_ITEM_HEADER: usize = 16;
            let mut header = idx;
            let mut header_text = t.to_string();
            let mut resolved: Option<Option<usize>> = None;
            while header < lines.len() {
                let braces = header_text.matches('{').count();
                let closers = header_text.matches('}').count();
                if braces > closers {
                    // The header opened a block: find the closer at this
                    // item's own indent.
                    resolved = None;
                    break;
                }
                if header_text.ends_with(';') || header_text.ends_with('}') {
                    resolved = Some(Some(header));
                    break;
                }
                if header.saturating_sub(start) >= MAX_ITEM_HEADER {
                    resolved = Some(None);
                    break;
                }
                header = header.saturating_add(1);
                match lines.get(header).and_then(|l| comments.feed(l, header)) {
                    Some(more) => {
                        header_text.push(' ');
                        header_text.push_str(more.trim());
                    }
                    None if header >= lines.len() => {
                        resolved = Some(None);
                        break;
                    }
                    None => {}
                }
            }
            match resolved {
                // Self-contained: the item is `start..=end`.
                Some(Some(end)) => {
                    skipped_regions.push((start, end));
                    idx = end.saturating_add(1);
                    continue;
                }
                // Unresolvable: loud, per WR-03's rule for the same shape.
                Some(None) => {
                    unclosed_regions.push(start);
                    skipped_regions.push((start, lines.len().saturating_sub(1)));
                    continue;
                }
                None => {}
            }

            idx = header.saturating_add(1);
            let mut closed_at = None;
            while idx < lines.len() {
                // Comment state is carried THROUGH the region body too, so
                // it stays coherent afterwards and so a `}` sitting inside
                // a block comment cannot close the region early.
                let has_code = comments.feed(lines[idx], idx).is_some();
                if has_code && is_region_closer(lines[idx], indent) {
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

        out.push((idx, code));
        idx += 1;
    }

    ProductionScan {
        lines: out,
        skipped_regions,
        unclosed_regions,
        unterminated_block_comment: comments.opened_at,
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
            // Round-6 WR-01. `any(test, ..)` is satisfied by its OTHER
            // disjuncts, so the gated item is compiled into production builds
            // and must never be skipped. The second is the real occurrence:
            // `session_commands.rs`'s `fn format_bytes_human`.
            "#[cfg(any(test, feature = \"x\"))]",
            "#[cfg(any(test, target_os = \"macos\", target_os = \"windows\"))]",
            // `not(any(test, ..))` is TRUE in a non-test build where the other
            // disjuncts are also off.
            "#[cfg(not(any(test, feature = \"x\")))]",
            // An `any(test, ..)` nested under `all(..)` is still reachable in
            // production; the rule composes rather than pattern-matching.
            "#[cfg(all(any(test, feature = \"x\"), unix))]",
            // A FEATURE named `test` is not the `test` cfg.
            "#[cfg(feature = \"test\")]",
            // Not `cfg` at all.
            "#[cfg_attr(test, derive(Debug))]",
        ] {
            assert!(
                !is_cfg_test_attr(no),
                "{no:?} must NOT classify as a test gate — it gates code that IS compiled \
                 into production builds, and skipping it deletes real source from every gate \
                 built on this helper. That is the over-claim direction, which \
                 `assert_split_is_correct` cannot see."
            );
        }
    }

    /// Round-6 WR-01: the rule is EVALUATED, so nesting composes rather than
    /// needing a case per shape. `any(all(test, ..), all(test, ..))` is false
    /// in every non-test build and so IS test-only, even though its head is
    /// the `any` that the same round excluded.
    #[test]
    fn nested_cfg_predicates_compose() {
        assert!(
            is_cfg_test_attr("#[cfg(any(all(test, unix), all(test, windows)))]"),
            "every disjunct requires `test`, so this is unreachable in a production build"
        );
        assert!(
            !is_cfg_test_attr("#[cfg(any(all(test, unix), windows))]"),
            "the second disjunct does NOT require `test`, so this IS reachable in a \
             production build"
        );
        assert!(is_cfg_test_attr("#[cfg(all(test, not(miri)))]"));
        assert!(!is_cfg_test_attr("#[cfg(all(not(test), not(miri)))]"));
    }

    /// Round-6 WR-01: an unparseable predicate must fail toward NOT a test
    /// gate, so the gated body stays in the production half where a leaked
    /// `#[test]` is reported loudly. The opposite default deletes source
    /// silently.
    #[test]
    fn an_unparseable_cfg_predicate_is_not_a_test_gate() {
        for malformed in [
            "#[cfg(mystery(test))]",
            "#[cfg(all(test)",
            "#[cfg(not(test, unix))]",
            "#[cfg()]",
        ] {
            assert!(
                !is_cfg_test_attr(malformed),
                "{malformed:?} cannot be evaluated, so it must fail toward the LOUD \
                 direction (kept as production), not the silent one"
            );
        }
    }

    /// Round-6 WR-01 end to end: a module gated by `any(test, ..)` is
    /// PRODUCTION and its body must survive the scan whole.
    ///
    /// This is the direction no check in `assert_split_is_correct` can see —
    /// the region closes fine, a production module carries no test attribute
    /// to leak, and `skipped_regions` grows rather than shrinks — so it has to
    /// be asserted here.
    #[test]
    fn an_any_test_module_is_production_and_is_kept_whole() {
        let src = "\
#[cfg(any(test, target_os = \"macos\", target_os = \"windows\"))]
mod platform_helpers {
    fn format_bytes_human() {}
}
fn production_after() {}
";
        let scan = scan_production(src);
        assert!(
            scan.skipped_regions.is_empty(),
            "`#[cfg(any(test, ..))]` gates code compiled into production builds whenever \
             another disjunct holds; skipping it hides real source from every gate"
        );
        assert!(scan
            .lines
            .iter()
            .any(|(_, l)| l.contains("fn format_bytes_human")));
        assert!(scan
            .lines
            .iter()
            .any(|(_, l)| l.contains("production_after")));
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
        let kept: Vec<&str> = scan.lines.iter().map(|(_, l)| l.as_str()).collect();
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

    /// A bare `#[cfg(test)] mod foo;` declaration is its OWN region — one
    /// line, no closer search. This is `main.rs:156`, and treating it as a
    /// BLOCK region start truncated a gate's whole scan of that file to
    /// nothing, because the search then ran to the first `}` at column zero.
    #[test]
    fn bare_module_declaration_is_its_own_one_line_region() {
        let src = "\
#[cfg(test)]
mod test_env;
fn production_after() {}
";
        let scan = scan_production(src);
        assert_eq!(
            scan.skipped_regions,
            vec![(1, 1)],
            "the declaration is test-only source and is skipped, but it swallows nothing"
        );
        assert!(scan
            .lines
            .iter()
            .any(|(_, l)| l.contains("production_after")));
    }

    /// Round-8 WR-01: a `#[cfg(test)]` attribute gating something that is NOT
    /// an inline module.
    ///
    /// The fall-through this replaces kept the item — and, since nothing
    /// tracked its extent, every line of its body — in the PRODUCTION half,
    /// under a comment claiming the opposite. The shapes below are the two
    /// live ones (`agent_daemon/launch.rs`'s `_testable` wrappers and
    /// `output.rs`'s `SESSIONS_ROOT_TEST_LOCK`) plus the multi-line header
    /// that neither of them exercises.
    ///
    /// Direction: a `_testable` helper naming a layer would satisfy
    /// `layer_registry.rs`'s daemon gate on its own, and all four checks in
    /// `assert_split_is_correct` are blind to it — the region list is
    /// non-empty, nothing is unclosed, and a gated `fn` carries no `#[test]`.
    #[test]
    fn a_cfg_test_gated_non_module_item_is_skipped_whole() {
        let src = "\
fn production_a() {}
#[cfg(test)]
pub(crate) const WFP_CONTROL_PIPE_NAME_TESTABLE: &str = \"DaclAncestorTraverse\";
#[cfg(test)]
pub(crate) static SESSIONS_ROOT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[cfg(test)]
pub(crate) fn profile_needs_network_scoping_testable(profile_name: &str) -> bool {
    profile_needs_network_scoping(profile_name)
}
#[cfg(test)]
pub(crate) fn multi_line_header(
    a: u32,
) -> bool {
    a > 0
}
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
mod one_line_module { #[test] fn q() {} }
fn production_b() {}
";
        let scan = scan_production(src);
        let kept: Vec<&str> = scan.lines.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            kept,
            vec!["fn production_a() {}", "fn production_b() {}"],
            "every `#[cfg(test)]`-gated item is test-only source, whatever kind of item it is"
        );
        assert!(scan.unclosed_regions.is_empty());
        assert!(scan.unterminated_block_comment.is_none());
        assert_eq!(
            scan.skipped_regions.len(),
            6,
            "one region per gated item: {:?}",
            scan.skipped_regions
        );
        // The one-line module is the shape `leaked_test_attributes` could not
        // see either (round-8 WR-04): it opens no block, so no closer search
        // ever ran, and its `#[test]` shared a line with other tokens.
        assert!(
            !kept.iter().any(|l| l.contains("#[test]")),
            "a one-line `#[cfg(test)] mod t {{ #[test] fn q() {{}} }}` must not reach the \
             production half: {kept:?}"
        );
    }

    /// Round-8 WR-01: an item whose extent cannot be resolved must FAIL, not
    /// pick one of the two silent directions — the same rule WR-03 established
    /// for a module whose closer is never found.
    #[test]
    fn an_unresolvable_gated_item_is_reported() {
        let mut src = String::from("#[cfg(test)]\npub(crate) fn never_finishes(\n");
        for i in 0..40 {
            src.push_str(&format!("    arg{i}: u32,\n"));
        }
        let scan = scan_production(&src);
        assert_eq!(
            scan.unclosed_regions,
            vec![1],
            "an item header that never resolves to `;`, `}}` or an open block is recorded so \
             `assert_split_is_correct` fails on it"
        );
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
        for yes in [
            "#[test]",
            "#[tokio::test]",
            "#[rstest::test]",
            // Round-6 WR-03: the PARENTHESISED forms. `leaked_test_attributes`
            // is the only under-claim check there is, so a leaked module whose
            // functions use these was invisible to it.
            "#[tokio::test(flavor = \"multi_thread\")]",
            "#[tokio::test(flavor = \"multi_thread\", worker_threads = 2)]",
            "#[rstest::test(case(1))]",
            "#[async_std::test]",
            "#[test(harness)]",
            // Bare-ident harness attributes, matched by name.
            "#[rstest]",
            "#[test_case(1, 2)]",
        ] {
            assert!(
                is_test_attr(yes),
                "{yes:?} must classify as a test attribute — this predicate's width is what \
                 decides whether a classifier gap is caught or silent"
            );
        }
        for no in [
            "#[cfg(test)]",
            "#[cfg(all(test, target_os = \"windows\"))]",
            "#[allow(clippy::unwrap_used)]",
            "#[should_panic(expected = \"WR-03\")]",
            "#[derive(Debug)]",
            "fn test() {}",
            "// #[test]",
        ] {
            assert!(
                !is_test_attr(no),
                "{no:?} must NOT classify as a test attribute"
            );
        }
    }

    /// Round-6 WR-02: the pending window is defined POSITIVELY, so a block
    /// comment whose body lines are not `*`-aligned no longer un-skips the
    /// module.
    ///
    /// The enumeration this replaced admitted `""`, `//`, `/*` and `*`; the
    /// line `   world */` is none of them, so the pending flag cleared, `mod t
    /// {` opened no region, and the whole body — including the `#[test]` —
    /// was classified as production.
    #[test]
    fn a_non_aligned_block_comment_before_the_mod_does_not_break_the_region() {
        let src = "\
fn production_a() {}
#[cfg(test)]
/* hello
   world */
mod t {
    #[test]
    fn x() {}
}
fn production_b() {}
";
        let scan = scan_production(src);
        assert_eq!(
            scan.skipped_regions.len(),
            1,
            "a block comment between the cfg attribute and the `mod` line must not \
             re-classify the module body as production, whatever its body lines look like"
        );
        assert!(scan.leaked_test_attributes().is_empty());
        assert!(scan.lines.iter().any(|(_, l)| l.contains("production_b")));
        // The comment BODY must not reach the production half either: a
        // `/* … */` naming a gate's needle would otherwise satisfy that gate.
        assert!(
            !scan.lines.iter().any(|(_, l)| l.contains("world")),
            "block-comment text is comment text, exactly as `//` text is"
        );
    }

    /// Round-6 WR-02: block comments NEST in Rust, so the carried state is a
    /// depth counter. A flag would close on the inner `*/` and treat the
    /// remaining comment body as code.
    #[test]
    fn nested_block_comments_are_tracked_by_depth() {
        let src = "\
/* outer
   /* inner */
   still comment */
fn production() {}
";
        let scan = scan_production(src);
        assert_eq!(
            scan.lines
                .iter()
                .map(|(_, l)| l.as_str())
                .collect::<Vec<_>>(),
            vec!["fn production() {}"]
        );
        assert!(scan.unterminated_block_comment.is_none());
    }

    /// Round-8 WR-02: a block comment opened MID-LINE. Comment state used to
    /// be entered only from a line whose trimmed text starts with `/*`, so
    /// this comment was never opened at all and every continuation line of it
    /// was ordinary code to every consumer.
    ///
    /// The needle below is `output.rs`'s WR-26 gate's exact one, and the
    /// direction matters: for `layer_registry.rs`'s daemon gate — which
    /// asserts the production half NAMES a layer — a comment kept as code is
    /// **fail-open**. No check in `assert_split_is_correct` can see it.
    #[test]
    fn a_block_comment_opened_mid_line_is_comment_text() {
        let src = "\
let x = 1; /* NOTE: this mentions the
   Windows Application event log in prose only */
#[cfg(test)]
mod t {
    #[test]
    fn q() {}
}
fn production_b() {}
";
        let scan = scan_production(src);
        assert_eq!(scan.skipped_regions.len(), 1);
        assert!(scan.leaked_test_attributes().is_empty());
        assert!(scan.lines.iter().any(|(_, l)| l.contains("let x = 1")));
        assert!(
            !scan
                .lines
                .iter()
                .any(|(_, l)| l.contains("Windows Application event log")),
            "the continuation line of a mid-line block comment is comment text; keeping it \
             lets prose satisfy a gate that hunts for that needle"
        );
        assert!(
            !scan.lines.iter().any(|(_, l)| l.contains("NOTE")),
            "and so is the tail of the line that OPENED it — the production half is code, not \
             raw lines"
        );
    }

    /// Round-8 WR-02: a trailing comment does not travel into the production
    /// half attached to its code line, in either rendering.
    ///
    /// This is the same class one line up from the fixture above:
    /// `let x = 1; /* DaclAncestorTraverse */` closes on its own line, so no
    /// continuation-line rule can reach it, and the raw line satisfied the
    /// daemon gate by itself.
    #[test]
    fn a_trailing_comment_is_not_production_text() {
        let src = "\
#[cfg(test)]
mod t {
    #[test]
    fn q() {}
}
let a = 1; /* DaclAncestorTraverse */
let b = 2; // DaclPackageSidGrant
let c = \"DaclSessionSidGrant\"; // and this one IS production
";
        let scan = scan_production(src);
        let kept: Vec<&str> = scan.lines.iter().map(|(_, l)| l.as_str()).collect();
        assert!(
            !kept.iter().any(|l| l.contains("DaclAncestorTraverse")),
            "a closed block comment on a code line is comment text: {kept:?}"
        );
        assert!(
            !kept.iter().any(|l| l.contains("DaclPackageSidGrant")),
            "and so is a trailing `//` comment: {kept:?}"
        );
        assert!(
            kept.iter().any(|l| l.contains("DaclSessionSidGrant")),
            "non-vacuity: a STRING LITERAL naming a layer is production text and must survive, \
             or this rule would have made every needle gate vacuous: {kept:?}"
        );
    }

    /// Round-8 WR-02: entering a comment mid-line means knowing where the
    /// literals are. Each of these lines contains a comment OPENER inside a
    /// literal, and none of them opens a comment — if one did, the following
    /// production line would silently vanish from every gate.
    #[test]
    fn comment_openers_inside_literals_open_nothing() {
        for (label, opener) in [
            ("plain string", "let s = \"/* not a comment\";"),
            ("escaped quote", "let s = \"a\\\"/* still not\";"),
            ("raw string", "let s = r\"/* not a comment\";"),
            ("hashed raw string", "let s = r#\"/* \"not\" a comment\"#;"),
            ("byte string", "let s = b\"/* not a comment\";"),
            ("char literal", "let c = '/'; let d = '*';"),
            // A lifetime is not a char literal. Reading `'a` as one would
            // consume to the next `'` and swallow the `/*` after it.
            ("lifetime", "fn f<'a>(x: &'a str) {} // '"),
            ("windows path", "let p = r\"C:\\x\\\"; let q = 1;"),
        ] {
            let src = format!("{opener}\n#[cfg(test)]\nmod t {{\n    #[test]\n    fn q() {{}}\n}}\nfn production_b() {{}}\n");
            let scan = scan_production(&src);
            assert_eq!(
                scan.unterminated_block_comment, None,
                "{label}: {opener:?} opened a block comment that swallowed the rest of the file"
            );
            assert!(
                scan.lines.iter().any(|(_, l)| l.contains("production_b")),
                "{label}: {opener:?} made production text vanish — scan kept {:?}",
                scan.lines
            );
            assert_eq!(
                scan.skipped_regions.len(),
                1,
                "{label}: {opener:?} broke the cfg-test region that follows it"
            );
        }
    }

    /// Round-6 WR-02: an inline `/* label */ value` annotation is CODE. The
    /// shape is written in `exec_strategy_windows/launch.rs` today.
    #[test]
    fn a_line_with_an_inline_block_comment_is_still_code() {
        let src = "\
#[cfg(test)]
mod t {
    fn helper() {}
}
call(/* is_detached */ false, /* has_pty */ true);
";
        let scan = scan_production(src);
        assert!(
            scan.lines.iter().any(|(_, l)| l.contains("call(")),
            "a balanced inline block comment leaves code on the line, so the line is kept"
        );
    }

    /// Round-6 WR-02: a block comment left open at EOF swallows everything
    /// after it. That is the over-claim direction, so it must be REPORTED.
    #[test]
    fn an_unterminated_block_comment_is_reported() {
        let src = "\
#[cfg(test)]
mod t {
    fn helper() {}
}
/* opened and never closed
fn production_that_is_now_invisible() {}
";
        let scan = scan_production(src);
        assert_eq!(scan.unterminated_block_comment, Some(4));
        assert!(!scan
            .lines
            .iter()
            .any(|(_, l)| l.contains("production_that_is_now_invisible")));
    }

    /// `assert_split_is_correct` must FAIL on an unterminated block comment —
    /// the same reason it fails on an unclosed region.
    #[test]
    #[should_panic(expected = "WR-02")]
    fn assert_split_is_correct_rejects_an_unterminated_block_comment() {
        let src = "\
#[cfg(test)]
mod t {
    fn helper() {}
}
/* opened and never closed
fn production_that_is_now_invisible() {}
";
        scan_production(src).assert_split_is_correct("fixture.rs");
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
