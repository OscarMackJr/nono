#![cfg_attr(target_os = "windows", allow(dead_code))]

//! CLI output styling for nono
//!
//! All colors are drawn from the active theme via `theme::current()`.

use crate::command_display::format_command_line;
#[cfg(target_os = "windows")]
use crate::state_paths;
use crate::theme::{self, badge, fg, Rgb};
use colored::Colorize;
use nono::{AccessMode, CapabilitySet, NetworkMode, NonoError, Result};
use std::ffi::{OsStr, OsString};
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Dark foreground for badge text (works on both light and dark bg colors)
const BADGE_FG_DARK: Rgb = Rgb(30, 30, 46);
/// Print a thin horizontal rule using overlay color
fn rule() {
    let t = theme::current();
    eprintln!("  {}", theme::fg(&"\u{2500}".repeat(52), t.overlay));
}

// ---------------------------------------------------------------------------
// Banner
// ---------------------------------------------------------------------------

/// Print the nono banner
pub fn print_banner(silent: bool) {
    if silent {
        return;
    }

    let t = theme::current();
    let version = env!("CARGO_PKG_VERSION");

    eprintln!();
    eprintln!(
        "  {} {}",
        theme::fg("nono", t.brand).bold(),
        theme::fg(&format!("v{version}"), t.subtext),
    );
}

// ---------------------------------------------------------------------------
// Startup self-attestation downgrade banner (Phase 117 CINT-02, D-27)
// ---------------------------------------------------------------------------

/// Print a coarse, arm-independent notice that a startup self-attestation
/// pass could not fully confirm one or more confinement layers (D-27's
/// human-visible channel; the other two channels — a typed
/// `NonoDiagnosticCode` and a `SecurityEventLayer` event — are Plans 02/09's
/// other artifacts, wired at the Plan 10/11 gate points).
///
/// Deliberately takes no `silent` parameter — a downgraded confinement claim
/// must reach the operator unconditionally (Warning-8, checker pass 2).
/// `--silent` suppresses routine/informational output (the version banner,
/// capability summaries); it must NOT suppress a security-relevant notice
/// that a confinement guarantee was not fully confirmed.
///
/// Deduplicates per session, not per spawn — the FIRST occurrence of a given
/// `dedup_key` for a given `session_id` always prints; only an identical
/// repeat within the same session is suppressed, closing the per-tool-call
/// noise the hook path (`claude_code_hook.rs` re-enters the direct spawn
/// path on every tool call) would otherwise produce (Item-2, checker pass
/// 3). This is a content-addressed marker, not a silence switch — a
/// marker-write failure defaults to printing again next time, never to
/// silently suppressing more than intended.
///
/// # WR-04: the dedup marker is a hint, not a security boundary
///
/// State this honestly, because "unconditional" above is a strong claim.
/// The marker lives under the user's own sessions directory, so ANY process
/// running as this user can write it. Since WR-04 the marker's CONTENT is
/// authoritative — the stored bytes must equal
/// `attestation_downgrade_marker_content(dedup_key)` — which closes the two
/// failure modes that were not adversarial at all: a 64-bit `DefaultHasher`
/// collision between two different downgraded-layer-sets silently
/// suppressing the second, and a zero-byte file of the right name
/// suppressing everything. A stale, empty, collided or unreadable marker
/// prints rather than suppresses.
///
/// It does NOT make suppression impossible for a same-user process, and no
/// marker scheme reachable from here could. On the `Null` and
/// `WriteRestricted` token arms the confined child runs under the same user
/// AND the same integrity level as this supervisor, so there is no on-disk
/// location this process can read across spawns that that process cannot
/// also write; an unforgeable marker would need a secret those arms
/// structurally cannot keep. That is accepted, not overlooked. Such a
/// process already holds strictly greater capability over this launch (it
/// can edit nono's config, or shadow `nono` on `PATH`), so the marker is not
/// the weakest link; and the audit event and the `tracing::warn!` on the
/// same path are independent channels it does not control. "Unconditional"
/// above means "not gated on `--silent`, and not suppressed by any nono code
/// path", not "tamper-proof".
///
/// # CR-01: content-authority must not become DISCLOSURE (D-28)
///
/// The first WR-04 fix bought that authority by writing `dedup_key` — the
/// plaintext, comma-joined `LayerId` set — verbatim into this same
/// child-readable file. That was a regression, not a trade worth making: on
/// the `Stderr` and `None` detail channels every other channel deliberately
/// WITHHOLDS those names, so the marker became the only durable plaintext
/// record of exactly which confinement layers failed to confirm, sitting
/// where the untrusted process can read it — precisely the reconnaissance
/// signal D-28 exists to deny.
///
/// The content is therefore a domain-separated digest of the key, never the
/// key. This keeps the entire authority gain at ZERO marginal disclosure:
/// the content is a deterministic function of the same input that already
/// determines the file's NAME, so it reveals nothing the file's existence
/// under that name did not already reveal.
///
/// `dedup_key` is an opaque `&str` this function never parses, displays or
/// persists — only hashes. The caller (which already has the layer identity
/// type in scope) computes the sorted, comma-joined dedup key string and
/// passes it in; this module deliberately never imports that type
/// (mechanically verifiable — see the acceptance criterion this doc comment
/// must not itself trip).
///
/// This wording is identical on every entry path/token arm — deliberately
/// not special-cased per-arm, because verifying per-arm console-sharing
/// safety (whether the confined child shares this console) has not been
/// independently verified — see `117-VALIDATION.md`'s D-31 list. No
/// per-layer identifier or specific mechanism name ever appears in this
/// function's output (D-28); that detail is reserved for the operator's
/// diagnostic and audit-event channels.
///
/// `#[cfg(target_os = "windows")]`: CINT-02 is a Windows-only self-attestation
/// pass (D-09); mirrors `format_scope_status`'s Linux-only gating shape
/// (`crates/nono-cli/src/output.rs`, this file) rather than compiling a
/// perpetually-uncalled function on platforms with no attestation caller.
#[cfg(target_os = "windows")]
pub fn print_attestation_downgrade_banner(
    downgraded_count: usize,
    session_id: Option<&str>,
    dedup_key: &str,
    channel: crate::exec_strategy::attestation_downgrade_event::DowngradeDetailChannel,
) {
    if downgraded_count == 0 {
        return;
    }

    let marker_path = session_id.and_then(|id| attestation_downgrade_marker_path(id, dedup_key));

    if let Some(path) = &marker_path {
        if marker_says_already_announced(path, dedup_key) {
            return;
        }
    }

    // WR-26 (Phase 117-44): name a destination ONLY when it received the
    // record. One match, no `_` arm — a future variant is a compile error, not
    // a silently wrong pointer. The dedup marker below deliberately does NOT
    // key on `channel`: the same downgraded layer-set on the same session is
    // still a repeat, whatever channel carried it.
    use crate::exec_strategy::attestation_downgrade_event::DowngradeDetailChannel;
    let where_to_look = match channel {
        DowngradeDetailChannel::EventLog => {
            "layer detail is in the Windows Application event log (source `nono`, event id \
             10011); re-run with --log-file <path> to capture it locally"
        }
        // The operator's own private log already holds the specific names.
        DowngradeDetailChannel::PrivateLogFile => {
            "layer detail was written to your --log-file target"
        }
        DowngradeDetailChannel::Stderr => {
            "the `nono` event source is not registered, so the record went to this console with \
             layer names withheld; run `nono setup` to register it, or re-run with --log-file \
             <path>"
        }
        DowngradeDetailChannel::None => {
            "layer detail could not be recorded on any channel — re-run with --log-file <path> \
             to capture it locally"
        }
    };

    let t = theme::current();
    eprintln!(
        "  {}",
        theme::fg(
            &format!(
                "{downgraded_count} confinement layer(s) could not be fully confirmed at \
                 startup — {where_to_look}"
            ),
            attestation_downgrade_color(downgraded_count, t),
        )
    );

    // Best-effort "already announced" marker (Item-2 dedup). Any I/O failure
    // here is logged and otherwise ignored — it can only cause an extra
    // repeat print next time, never a suppressed first occurrence, and it
    // must never prevent or undo the print that already happened above.
    if let Some(path) = marker_path {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::debug!(
                    "attestation-downgrade dedup marker directory create failed \
                     (non-fatal, banner will re-print next time): {e}"
                );
                return;
            }
        }
        // WR-04: the reader compares CONTENT, not just the hashed filename,
        // so a zero-byte or colliding marker announces instead of
        // suppressing. CR-01: the content written is a digest, never the
        // plaintext key — this file is readable by the confined child on
        // three of the four token arms, and the layer names are exactly what
        // D-28 withholds from every channel that child can observe.
        // `create_new` so a pre-existing file is never clobbered — if one is
        // there with different content, this launch has already announced,
        // and overwriting it would only hide that fact from the next launch.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                use std::io::Write as _;
                if let Err(e) =
                    f.write_all(attestation_downgrade_marker_content(dedup_key).as_bytes())
                {
                    tracing::debug!(
                        "attestation-downgrade dedup marker write failed \
                         (non-fatal, banner will re-print next time): {e}"
                    );
                }
            }
            Err(e) => {
                tracing::debug!(
                    "attestation-downgrade dedup marker create failed \
                     (non-fatal, banner will re-print next time): {e}"
                );
            }
        }
    }
}

/// Has this exact `dedup_key` already been announced this session?
///
/// # WR-04: marker PRESENCE is not authoritative — its CONTENT is
///
/// The marker filename is a 64-bit `DefaultHasher` digest of `dedup_key`.
/// `DefaultHasher::new()` is documented to use fixed keys, so the mapping is
/// deterministic and offline-computable, and the key space is a sorted
/// comma-join drawn from a 13-element enum (at most 2^13 sets). Returning
/// `path.exists()` therefore meant:
///
/// - a 64-bit collision between two DIFFERENT downgraded-layer-sets in one
///   session silently suppressed the second, genuinely different one; and
/// - a zero-byte file of the right name, from any source, suppressed the
///   announcement entirely — for a channel whose own doc calls it
///   unconditional.
///
/// Requiring the stored CONTENT to match makes an absent, empty, unreadable,
/// collided or tampered marker ANNOUNCE rather than suppress: every failure
/// mode resolves toward more visibility, never less. It also makes a
/// `DefaultHasher` change across a toolchain bump merely re-print once
/// instead of silently aliasing two different sets.
///
/// # CR-01: the compared value is a digest, not the key
///
/// The comparison is against `attestation_downgrade_marker_content(key)`,
/// not against `key` itself, so no `LayerId` name is ever written to this
/// child-readable path (D-28). The set of `(session, key)` pairs that
/// suppress is IDENTICAL either way — the digest is injective for every
/// input this code can produce, up to a 128-bit collision — so this closes
/// the disclosure direction without giving back any of WR-04's authority.
///
/// See [`print_attestation_downgrade_banner`]'s doc for what this does NOT
/// close (a same-user process can still pre-plant a correct marker, and on
/// the same-IL token arms no scheme could prevent that) and why that is
/// accepted.
#[cfg(target_os = "windows")]
fn marker_says_already_announced(path: &std::path::Path, dedup_key: &str) -> bool {
    matches!(
        std::fs::read_to_string(path),
        Ok(existing) if existing == attestation_downgrade_marker_content(dedup_key)
    )
}

/// The bytes stored INSIDE the dedup marker for `dedup_key` (CR-01, D-28).
///
/// A 128-bit domain-separated `DefaultHasher` digest, rendered as 32 lowercase
/// hex characters. Two independent 64-bit passes, each seeded with a distinct
/// constant, are concatenated: `DefaultHasher` is fixed-key, so hashing the
/// same key twice without domain separation would just repeat one value.
///
/// # Why a digest and not the key
///
/// This file sits under `%LOCALAPPDATA%\nono\sessions\…` with an ordinary
/// DACL and no mandatory label. The confined child can read it on the `Null`
/// arm (same token), the `WriteRestricted` arm (`WRITE_RESTRICTED` applies
/// restricting SIDs to WRITE checks only) and the Low-IL arms (MIC's default
/// policy is `NO_WRITE_UP`; reads up are permitted). Writing the plaintext
/// `LayerId` set there would hand the untrusted process the exact list of
/// confinement layers that failed to confirm — the reconnaissance signal
/// D-28 exists to deny, and one that the `Stderr` and `None` detail channels
/// deliberately withhold from every other surface.
///
/// # Why this discloses nothing the marker did not already disclose
///
/// The marker's FILENAME is already a deterministic 64-bit digest of the same
/// `dedup_key`. Its content here is another deterministic function of that
/// same input, so any observer that can invert one can invert the other, and
/// an observer that cannot enumerate the key space learns nothing from
/// either. The marginal disclosure of this content, relative to the file
/// merely existing under that name, is exactly zero.
///
/// This is NOT a keyed MAC and is not claimed to be one: see
/// [`print_attestation_downgrade_banner`]'s doc for why unforgeability is
/// unreachable on the same-user, same-IL token arms.
#[cfg(target_os = "windows")]
fn attestation_downgrade_marker_content(dedup_key: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};

    fn digest(domain: &str, key: &str) -> u64 {
        let mut h = DefaultHasher::new();
        domain.hash(&mut h);
        key.hash(&mut h);
        h.finish()
    }

    let hi = digest("nono/attestation-downgrade/marker-content/v1/hi", dedup_key);
    let lo = digest("nono/attestation-downgrade/marker-content/v1/lo", dedup_key);
    format!("{hi:016x}{lo:016x}")
}

/// Compute the per-session, content-addressed dedup marker path for
/// [`print_attestation_downgrade_banner`], or `None` when the session
/// directory root cannot be resolved (defaults to "not yet announced" —
/// erring toward MORE visibility, never less, per this function's own
/// doc comment).
///
/// `dedup_key` is hashed via `std::hash::DefaultHasher` (std-only, no new
/// dependency) rather than stored verbatim — this function, like its
/// caller, never inspects or displays the key's contents.
///
/// # CR-01: the `.v2` filename suffix
///
/// Marker CONTENT changed from the plaintext key to a digest
/// (`attestation_downgrade_marker_content`). A marker left behind by a build
/// that wrote the plaintext form would never match again, and `create_new`
/// means it would never be replaced either — so the banner would re-print on
/// every tool call for the rest of that session. That is the SAFE direction
/// (more visibility), but it is still a needless regression, and the suffix
/// removes it: the two formats simply occupy different names. Stale v1
/// markers are inert leftovers in a directory that is already best-effort.
///
/// # Phase 117 review WR-11: `session_id` is validated before it is joined
///
/// The returned path is passed straight to `create_dir_all` + `write`, so
/// joining an unvalidated `&str` here is a write-boundary path-traversal
/// surface: on Windows a value containing `..`, a separator, or a
/// drive-relative prefix (`C:`) escapes the sessions root and creates
/// directories/files elsewhere. Today's callers pass an internally-generated
/// id, so this is defence in depth — but CLAUDE.md's "validate and
/// canonicalize all paths" rule applies at the boundary, not at the caller.
///
/// Rejection (returning `None`) errs toward MORE visibility, never less:
/// `None` means "no dedup marker", which makes the banner re-print rather
/// than be suppressed.
#[cfg(target_os = "windows")]
fn attestation_downgrade_marker_path(
    session_id: &str,
    dedup_key: &str,
) -> Option<std::path::PathBuf> {
    use std::hash::{DefaultHasher, Hash, Hasher};

    if !session_id_is_safe_path_component(session_id) {
        tracing::debug!(
            "attestation-downgrade dedup: session id is not a safe single path component, \
             defaulting to always-print"
        );
        return None;
    }

    let mut hasher = DefaultHasher::new();
    dedup_key.hash(&mut hasher);
    let key_hex = format!("{:016x}.v2", hasher.finish());

    match state_paths::sessions_dir() {
        Ok(dir) => Some(
            dir.join(session_id)
                .join("attestation-downgrade")
                .join(key_hex),
        ),
        Err(e) => {
            tracing::debug!(
                "attestation-downgrade dedup: sessions_dir unavailable, \
                 defaulting to always-print: {e}"
            );
            None
        }
    }
}

/// WR-11: is `session_id` a single, inert path component safe to `join` onto
/// the sessions root?
///
/// Allow-list, not deny-list: ASCII alphanumerics plus `-` and `_`. That
/// covers every id `nono` generates (uuid-simple / hex session tokens) and
/// structurally excludes separators, `..`, drive prefixes, ADS colons,
/// trailing dots/spaces, and every non-ASCII homoglyph trick — none of which
/// can appear at all.
///
/// # WR-11: bare reserved device names need their own exclusion
///
/// The allow-list does NOT exclude them: `CON`, `PRN`, `AUX`, `NUL`,
/// `COM0`..`COM9` and `LPT0`..`LPT9` are pure ASCII alphanumerics and passed.
/// Today's impact would be benign in direction (the later `create_dir_all`
/// fails, no marker is written, and the banner re-prints — the fail-safe
/// side), but the doc asserted a guarantee the predicate did not provide,
/// which is how the next consumer of this helper gets bitten. Rejecting them
/// here keeps the doc and the code saying the same thing, and rejection is
/// itself the safe direction: `None` means "no dedup marker", i.e. the
/// banner prints MORE, never less.
///
/// `COM0`/`LPT0` are included: they are reserved in the Win32 namespace even
/// though the `COM1`..`COM9` form is the one usually listed. Comparison is
/// ASCII-case-insensitive because the Win32 device namespace is.
#[cfg(target_os = "windows")]
fn session_id_is_safe_path_component(session_id: &str) -> bool {
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
        "LPT9",
    ];

    !session_id.is_empty()
        && session_id.len() <= 128
        && session_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !RESERVED.iter().any(|r| session_id.eq_ignore_ascii_case(r))
}

/// Serializes the tests that drive the REAL downgrade-banner writer against
/// the REAL sessions root (`%LOCALAPPDATA%\nono\sessions`).
///
/// Two such tests now exist in this binary — `output.rs`'s cold/warm latency
/// measurement and `launch.rs`'s CR-01/D-28 disclosure scan — and they live in
/// different modules, so cargo's harness runs them on different threads. Adding
/// the second one made the first flaky: measured alone the warm path is
/// 1.6-15.7ms across 8 runs, but concurrently with a `create_dir_all` +
/// `remove_dir_all` under the same root it was observed at 235ms, tripping the
/// 100ms sanity bound. That is filesystem contention in the test harness, not a
/// latency regression in the code under test — but a flaky gate is a gate
/// people learn to ignore, so it is removed rather than explained.
///
/// Takes the lock through poisoning (`into_inner`): a panic in one of these
/// tests must not convert every other run into a second, unrelated failure.
#[cfg(all(test, target_os = "windows"))]
pub(crate) static SESSIONS_ROOT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Color for [`print_attestation_downgrade_banner`]'s line — yellow/warning
/// when `downgraded_count > 0`, else the neutral subtext color (mirrors
/// `scope_status_color`'s shape, `crates/nono-cli/src/output.rs:503`).
#[cfg(target_os = "windows")]
#[must_use]
pub fn attestation_downgrade_color(downgraded_count: usize, t: &theme::Theme) -> Rgb {
    if downgraded_count > 0 {
        t.yellow
    } else {
        t.subtext
    }
}

// ---------------------------------------------------------------------------
// Capabilities
// ---------------------------------------------------------------------------

/// Print the capability summary
///
/// When `verbose` is 0, only user-specified capabilities are shown (CLI flags
/// and profile filesystem entries). System paths and group-resolved paths are
/// hidden to reduce noise. Use `-v` to show all capabilities.
pub fn print_capabilities(
    caps: &CapabilitySet,
    blocked_grants: &[(std::path::PathBuf, Option<String>)],
    verbose: u8,
    silent: bool,
    // When true, a proxy will start even though caps may show Blocked or AllowAll.
    // Upstream 72bcfd66 (#1225): display yellow "proxy" or "proxy (strict)" rather
    // than "outbound blocked" / "outbound allowed" when a proxy is pending.
    proxy_pending: bool,
) {
    if silent {
        return;
    }

    let t = theme::current();

    eprintln!("  {}", theme::fg("Capabilities:", t.subtext).bold());
    rule();

    // Filesystem capabilities
    let fs_caps = caps.fs_capabilities();
    if !fs_caps.is_empty() {
        let (user_caps, other_count) = if verbose > 0 {
            (fs_caps.to_vec(), 0)
        } else {
            let user: Vec<_> = fs_caps
                .iter()
                .filter(|c| c.source.is_user_intent())
                .cloned()
                .collect();
            let hidden = fs_caps.len() - user.len();
            (user, hidden)
        };

        for cap in &user_caps {
            let kind = if cap.is_file { "file" } else { "dir" };
            let access_badge = format_access_badge(&cap.access);

            if verbose > 0 {
                let source_str = format!("{}", cap.source);
                eprintln!(
                    "  {} {} {}",
                    access_badge,
                    theme::fg(&cap.resolved.display().to_string(), t.text),
                    theme::fg(&format!("({kind}) [{source_str}]"), t.subtext),
                );
            } else {
                eprintln!(
                    "  {} {} {}",
                    access_badge,
                    theme::fg(&cap.resolved.display().to_string(), t.text),
                    theme::fg(&format!("({kind})"), t.subtext),
                );
            }
        }

        if other_count > 0 {
            eprintln!(
                "       {}",
                theme::fg(
                    &format!("+ {other_count} system/group paths (-v to show)"),
                    t.subtext
                )
            );
        }
    }

    // Protected paths kept blocked despite a user grant (macOS deny groups).
    // Folded into one row by default so a broad grant (e.g. ~/Library) that
    // overlaps several deny groups does not produce a wall of warnings.
    print_blocked_grants(blocked_grants, verbose, t);

    // AF_UNIX socket capabilities (issue #685 / #696)
    let unix_caps = caps.unix_socket_capabilities();
    if !unix_caps.is_empty() {
        let (user_caps, hidden_count) = if verbose > 0 {
            (unix_caps.to_vec(), 0)
        } else {
            let user: Vec<_> = unix_caps
                .iter()
                .filter(|c| c.source.is_user_intent())
                .cloned()
                .collect();
            let hidden = unix_caps.len() - user.len();
            (user, hidden)
        };

        for cap in &user_caps {
            let mode_badge = format_unix_socket_mode_badge(cap.mode);
            let scope_suffix = match cap.scope {
                nono::SocketScope::File => "",
                nono::SocketScope::DirChildren => "  (directory grant — direct child sockets only)",
                nono::SocketScope::DirSubtree => "  (subtree grant — recursive socket paths)",
            };
            if verbose > 0 {
                let source_str = format!("{}", cap.source);
                eprintln!(
                    "  {} {} {}{}",
                    mode_badge,
                    theme::fg(&cap.resolved.display().to_string(), t.text),
                    theme::fg(&format!("[{source_str}]"), t.subtext),
                    theme::fg(scope_suffix, t.subtext),
                );
            } else {
                eprintln!(
                    "  {} {}{}",
                    mode_badge,
                    theme::fg(&cap.resolved.display().to_string(), t.text),
                    theme::fg(scope_suffix, t.subtext),
                );
            }
        }

        if hidden_count > 0 {
            eprintln!(
                "       {}",
                theme::fg(
                    &format!("+ {hidden_count} system/group unix sockets (-v to show)"),
                    t.subtext
                )
            );
        }
    }

    // Network status
    match caps.network_mode() {
        NetworkMode::Blocked => {
            if proxy_pending {
                // Profile set network.block but CLI added proxy flags — proxy
                // will start in strict_filter mode and owns the network mode.
                eprintln!(
                    "  {} {}",
                    theme::badge(" net ", t.yellow, BADGE_FG_DARK),
                    theme::fg("proxy (strict)", t.subtext),
                );
            } else {
                eprintln!(
                    "  {} {}",
                    theme::badge(" net ", t.red, BADGE_FG_DARK),
                    theme::fg("outbound blocked", t.subtext),
                );
            }
        }
        NetworkMode::ProxyOnly { port, bind_ports } => {
            let port_str = if *port == 0 {
                String::new()
            } else {
                format!(" localhost:{port}")
            };
            if bind_ports.is_empty() {
                eprintln!(
                    "  {} {}",
                    theme::badge(" net ", t.yellow, BADGE_FG_DARK),
                    theme::fg(&format!("proxy{port_str}"), t.subtext),
                );
            } else {
                let ports_str: Vec<String> = bind_ports.iter().map(|p| p.to_string()).collect();
                let bind_info = format!(", bind: {}", ports_str.join(", "));
                eprintln!(
                    "  {} {}",
                    theme::badge(" net ", t.yellow, BADGE_FG_DARK),
                    theme::fg(&format!("proxy{port_str}{bind_info}"), t.subtext,),
                );
            }
        }
        NetworkMode::AllowAll => {
            if proxy_pending {
                eprintln!(
                    "  {} {}",
                    theme::badge(" net ", t.yellow, BADGE_FG_DARK),
                    theme::fg("proxy", t.subtext),
                );
            } else {
                eprintln!(
                    "  {} {}",
                    theme::badge(" net ", t.green, BADGE_FG_DARK),
                    theme::fg("outbound allowed", t.subtext),
                );
            }
        }
    }
    if !caps.localhost_ports().is_empty() || !caps.localhost_port_ranges().is_empty() {
        let mut parts: Vec<String> = caps
            .localhost_ports()
            .iter()
            .map(|p| p.to_string())
            .collect();
        for &(start, end) in caps.localhost_port_ranges() {
            parts.push(format!("{start}..={end}"));
        }
        eprintln!(
            "  {} {}",
            theme::badge(" ipc ", t.teal, BADGE_FG_DARK),
            theme::fg(&format!("localhost:{}", parts.join(", ")), t.subtext,),
        );
    }

    rule();
    eprintln!();
}

/// Format an access mode as a fixed-width colored badge
/// Render the paths that a deny group keeps blocked despite a user grant.
///
/// Collapsed by default to a single row (a broad grant such as `~/Library`
/// overlaps many deny groups and would otherwise emit one warning per path).
/// `-v` expands to the full paths grouped by the deny rule that blocks them,
/// with the `--bypass-protection` escape hatch shown once.
fn print_blocked_grants(
    blocked: &[(std::path::PathBuf, Option<String>)],
    verbose: u8,
    t: &theme::Theme,
) {
    if blocked.is_empty() {
        return;
    }

    let badge = theme::badge("deny ", t.yellow, BADGE_FG_DARK);

    if verbose == 0 {
        let n = blocked.len();
        let noun = if n == 1 { "path" } else { "paths" };
        eprintln!(
            "  {} {}",
            badge,
            theme::fg(
                &format!("{n} sensitive {noun} kept blocked inside your grants (-v to show)"),
                t.subtext,
            ),
        );
        return;
    }

    eprintln!(
        "  {} {}",
        badge,
        theme::fg("sensitive paths kept blocked despite your grants:", t.text),
    );

    // Group by the deny rule that blocks each path, preserving first-seen order.
    let mut groups: Vec<(String, Vec<&std::path::Path>)> = Vec::new();
    for (path, group) in blocked {
        let group_name = group.as_deref().unwrap_or("a deny rule");
        match groups.iter_mut().find(|(name, _)| name == group_name) {
            Some((_, paths)) => paths.push(path.as_path()),
            None => groups.push((group_name.to_string(), vec![path.as_path()])),
        }
    }

    for (name, paths) in &groups {
        eprintln!("       {}", theme::fg(name, t.subtext));
        for path in paths {
            eprintln!("         {}", theme::fg(&path.to_string_lossy(), t.text));
        }
    }

    eprintln!(
        "       {}",
        theme::fg(
            "use --bypass-protection <path> to allow a specific path",
            t.subtext,
        ),
    );
}

fn format_access_badge(access: &AccessMode) -> String {
    let t = theme::current();
    match access {
        AccessMode::Read => theme::badge("  r  ", t.green, BADGE_FG_DARK),
        AccessMode::Write => theme::badge("  w  ", t.yellow, BADGE_FG_DARK),
        AccessMode::ReadWrite => theme::badge(" r+w ", t.brand, BADGE_FG_DARK),
    }
}

fn format_unix_socket_mode_badge(mode: nono::UnixSocketMode) -> String {
    let t = theme::current();
    match mode {
        nono::UnixSocketMode::Connect => theme::badge("sock ", t.green, BADGE_FG_DARK),
        nono::UnixSocketMode::ConnectBind => theme::badge("sock+", t.brand, BADGE_FG_DARK),
    }
}

/// Format an access mode as inline colored text (for prompts)
fn format_access_inline(access: &AccessMode) -> colored::ColoredString {
    let t = theme::current();
    match access {
        AccessMode::Read => theme::fg("read", t.green),
        AccessMode::Write => theme::fg("write", t.yellow),
        AccessMode::ReadWrite => theme::fg("read+write", t.brand),
    }
}

// ---------------------------------------------------------------------------
// Kernel / ABI
// ---------------------------------------------------------------------------

/// Print Landlock ABI information (Linux only).
///
/// Shows the detected ABI version and available features. When features
/// are degraded (ABI < V5), displays which features are unavailable.
#[cfg(target_os = "linux")]
pub fn print_abi_info(silent: bool) {
    if silent {
        return;
    }
    let t = theme::current();
    match nono::Sandbox::detect_abi() {
        Ok(detected) => {
            type AbiFeatureCheck = (&'static str, fn(&nono::DetectedAbi) -> bool);
            const ALL_FEATURES: &[AbiFeatureCheck] = &[
                ("Refer", nono::DetectedAbi::has_refer),
                ("Truncate", nono::DetectedAbi::has_truncate),
                ("TCP filtering", nono::DetectedAbi::has_network),
                ("IoctlDev", nono::DetectedAbi::has_ioctl_dev),
                ("Scoping", nono::DetectedAbi::has_scoping),
            ];

            let missing: Vec<&str> = ALL_FEATURES
                .iter()
                .filter(|(_, check)| !check(&detected))
                .map(|(name, _)| *name)
                .collect();
            let is_wsl2 = nono::sandbox::is_wsl2();

            if missing.is_empty() && !is_wsl2 {
                return;
            }

            eprintln!(
                "  {} {}",
                badge(" kernel ", t.yellow, BADGE_FG_DARK),
                fg(&detected.to_string(), t.text),
            );

            let hint = if is_wsl2 {
                let pad = " ".repeat(10);
                let mut wsl2_missing: Vec<&str> = Vec::new();
                if !detected.has_network() {
                    wsl2_missing.push("per-port filtering");
                }
                if !detected.has_ioctl_dev() {
                    wsl2_missing.push("device ioctl");
                }
                if !detected.has_scoping() {
                    wsl2_missing.push("process scoping");
                }
                wsl2_missing.push("capability elevation (seccomp notify)");
                format!(
                    "degraded: {} unavailable on WSL2\n\
                     {pad}(block-all network via --block-net still works)\n\
                     {pad}details: https://nono.sh/docs/cli/internals/wsl2",
                    wsl2_missing.join(", "),
                )
            } else {
                format!(
                    "degraded: {} (upgrade kernel for full support)",
                    missing.join(", "),
                )
            };
            eprintln!("          {}", fg(&hint, t.yellow));
        }
        Err(e) => {
            eprintln!(
                "  {} {}",
                badge(" kernel ", t.red, BADGE_FG_DARK),
                fg(&format!("Landlock detection failed: {e}"), t.red),
            );
        }
    }
}

/// Print the Landlock scope policy derived from the current capabilities.
#[cfg(target_os = "linux")]
pub fn print_landlock_scope_policy(caps: &CapabilitySet, verbose: u8, silent: bool) {
    if silent || verbose == 0 {
        return;
    }

    let t = theme::current();
    match nono::landlock_scope_policy(caps) {
        Ok(policy) => {
            eprintln!(
                "  {} {}",
                badge(" scope ", t.blue, BADGE_FG_DARK),
                fg(
                    &format!("Landlock {} detected", policy.abi_version),
                    t.subtext,
                )
            );
            eprintln!(
                "          {} {}",
                fg("signal:", t.subtext),
                fg(
                    &format_scope_status(
                        policy.signal_requested,
                        policy.signal_enforced,
                        policy.scoping_supported,
                    ),
                    scope_status_color(
                        policy.signal_requested,
                        policy.signal_enforced,
                        policy.scoping_supported,
                        t,
                    ),
                )
            );
            eprintln!(
                "          {} {}",
                fg("abstract-unix-socket:", t.subtext),
                fg(
                    &format_scope_status(
                        policy.abstract_unix_socket_requested,
                        policy.abstract_unix_socket_enforced,
                        policy.scoping_supported,
                    ),
                    scope_status_color(
                        policy.abstract_unix_socket_requested,
                        policy.abstract_unix_socket_enforced,
                        policy.scoping_supported,
                        t,
                    ),
                )
            );
        }
        Err(err) => {
            eprintln!(
                "  {} {}",
                badge(" scope ", t.red, BADGE_FG_DARK),
                fg(&format!("Landlock scope policy unavailable: {err}"), t.red),
            );
        }
    }
}

#[cfg(target_os = "linux")]
fn format_scope_status(requested: bool, enforced: bool, supported: bool) -> String {
    match (requested, enforced, supported) {
        (true, true, _) => "requested, enforced".to_string(),
        (true, false, false) => "requested, unsupported by detected ABI".to_string(),
        (true, false, true) => "requested, not enforced".to_string(),
        (false, _, true) => "not requested".to_string(),
        (false, _, false) => "not requested; detected ABI has no scope support".to_string(),
    }
}

#[cfg(target_os = "linux")]
fn scope_status_color(requested: bool, enforced: bool, supported: bool, t: &theme::Theme) -> Rgb {
    match (requested, enforced, supported) {
        (true, true, _) => t.green,
        (true, false, _) => t.yellow,
        (false, _, _) => t.subtext,
    }
}

// ---------------------------------------------------------------------------
// Status messages
// ---------------------------------------------------------------------------

/// Print supervised mode status
pub fn print_supervised_info(silent: bool, rollback: bool, proxy_active: bool) {
    if silent || (!rollback && !proxy_active) {
        return;
    }
    let t = theme::current();
    let mut features = Vec::new();
    if rollback {
        features.push("snapshots");
    }
    if proxy_active {
        features.push("proxy");
    }
    features.push("supervisor");

    eprintln!(
        "  {} {}",
        fg("mode", t.subtext),
        fg(&format!("supervised ({})", features.join(", ")), t.subtext),
    );
}

/// Print a minimal status line before handing off to the sandboxed child.
pub fn print_applying_sandbox(silent: bool) {
    if silent {
        return;
    }
    let t = theme::current();
    eprintln!("  {}", fg("Applying sandbox...", t.subtext));
    eprintln!();
}

/// Print a styled warning message to stderr
pub fn print_warning(message: &str) {
    let t = theme::current();
    eprintln!("  {} {}", fg("warning:", t.red).bold(), fg(message, t.text),);
}

/// Print proxy credential warnings collected at startup.
pub fn print_proxy_diagnostics(diagnostics: &[nono_proxy::ProxyDiagnostic]) {
    if diagnostics.is_empty() {
        return;
    }

    let t = theme::current();
    eprintln!();
    eprintln!(
        "  {}",
        theme::fg("Proxy credential warnings:", t.red).bold(),
    );
    for diagnostic in diagnostics {
        let code = diagnostic.code.as_str();
        eprintln!(
            "  {} /{} — {}",
            theme::fg(code, t.subtext),
            diagnostic.route_prefix,
            fg(&diagnostic.message, t.text),
        );
        if let Some(hint) = &diagnostic.hint {
            eprintln!("    {}", theme::fg(hint, t.subtext));
        } else if let Some(action) = proxy_diagnostic_action(&diagnostic.code) {
            eprintln!("    {}", theme::fg(action, t.subtext));
        }
    }
}

fn proxy_diagnostic_action(code: &nono_proxy::ProxyDiagnosticCode) -> Option<&'static str> {
    use nono_proxy::ProxyDiagnosticCode;
    match code {
        ProxyDiagnosticCode::CredentialNotFound => Some(
            "Configure a valid credential reference for this route, or use an explicit upstream credential.",
        ),
        ProxyDiagnosticCode::CredentialUnavailable => Some(
            "Unlock the system keychain or authenticate with your credential provider (e.g. `op signin`).",
        ),
        ProxyDiagnosticCode::OAuthClientIdUnavailable
        | ProxyDiagnosticCode::OAuthClientSecretUnavailable => {
            Some("Provide OAuth client credentials via env/keystore configuration for this route.")
        }
        ProxyDiagnosticCode::OAuthTokenExchangeFailed => {
            Some("Verify OAuth client credentials and provider availability, then retry.")
        }
        _ => None,
    }
}

/// Format startup-blocked lines for writing to /dev/tty or stderr.
/// Returns a Vec of lines ready to write (without trailing newline).
pub fn format_startup_blocked(
    program: &str,
    timeout_secs: u64,
    has_output: bool,
    recommended_profile: Option<&str>,
) -> Vec<String> {
    let t = theme::current();
    let label = fg("blocked:", t.yellow).bold().to_string();
    let reason = if has_output {
        format!(
            "`{}` has not become interactive after {} seconds.",
            program, timeout_secs
        )
    } else {
        format!(
            "`{}` produced no terminal output after {} seconds.",
            program, timeout_secs
        )
    };
    let mut lines = vec![
        format!("  {} {}", label, fg(&reason, t.text)),
        format!(
            "  {}",
            fg(
                "Terminating process — re-run with -v to inspect denied paths.",
                t.subtext
            )
        ),
    ];
    if let Some(profile) = recommended_profile {
        lines.push(format!(
            "  {} nono run --profile {} -- {}",
            fg("Try:", t.green).bold(),
            profile,
            program,
        ));
    }
    lines
}

/// Print a styled diagnostic footer emitted by the core diagnostic formatter.
pub fn print_diagnostic_footer(footer: &str) {
    let rendered = render_diagnostic_footer(footer);
    print_terminal_block(&rendered, true);
}

/// Print skipped CLI path grants in a user-facing format.
pub fn print_skipped_requested_paths(paths: &[String], silent: bool) {
    if silent || paths.is_empty() {
        return;
    }

    let t = theme::current();
    eprintln!(
        "  {} {}",
        fg("warning:", t.red).bold(),
        fg(
            "some requested sandbox grants were skipped because the path does not exist:",
            t.text,
        ),
    );
    for path in paths {
        eprintln!("           {}", fg(path, t.subtext));
    }
    eprintln!();
}

fn render_diagnostic_footer(footer: &str) -> String {
    let t = theme::current();
    footer
        .lines()
        .enumerate()
        .map(|(idx, line)| render_diagnostic_line(idx, line, t))
        .collect::<Vec<_>>()
        .join("\n")
}

fn print_terminal_block(message: &str, leading_blank_line: bool) {
    let mut stderr = std::io::stderr();
    if stderr.is_terminal() {
        let normalized = normalize_terminal_line_endings(message);
        if leading_blank_line {
            let _ = write!(stderr, "\r\n");
        }
        let _ = write!(stderr, "\r{}\r\n", normalized);
    } else {
        if leading_blank_line {
            let _ = writeln!(stderr);
        }
        let _ = writeln!(stderr, "{}", message);
    }
}

fn normalize_terminal_line_endings(message: &str) -> String {
    message.replace('\n', "\r\n")
}

fn render_diagnostic_line(idx: usize, line: &str, t: &theme::Theme) -> String {
    let line = sanitize_terminal_output(line);
    if line.is_empty() {
        return String::new();
    }

    if idx == 0 && line == "nono diagnostic" {
        return format!("{}", fg("NONO DIAGNOSTIC", t.red).bold());
    }

    if idx == 1 && line.chars().all(|c| c == '\u{2500}') {
        return format!("{}", fg(&"\u{2500}".repeat(24), t.red));
    }

    if line.starts_with("The command failed") {
        return format!("{}", fg(&line, t.red).bold());
    }

    if line.starts_with("The command succeeded") {
        return format!("{}", fg(&line, t.yellow).bold());
    }

    if !line.starts_with(' ') && line.ends_with(':') {
        let color = match line.as_str() {
            "Likely sandbox denial:" | "Missing path:" => t.red,
            "Sandbox policy:" => t.brand,
            _ => t.text,
        };
        return format!("{}", fg(&line, color).bold());
    }

    if let Some(rest) = line.strip_prefix("  Try: ") {
        return format!(
            "  {} {}",
            fg("Try:", t.green).bold(),
            fg(rest, t.text).bold()
        );
    }

    if let Some(rest) = line.strip_prefix("  Why: ") {
        return format!("  {} {}", fg("Why:", t.blue).bold(), fg(rest, t.text));
    }

    if let Some(rest) = line.strip_prefix("  Learn: ") {
        return format!("  {} {}", fg("Learn:", t.teal).bold(), fg(rest, t.text));
    }

    if let Some(rest) = line.strip_prefix("  Re-use ") {
        return format!("  {}", fg(&format!("Re-use {rest}"), t.subtext));
    }

    if line == "  Allowed paths:" {
        return format!("  {}", fg("Allowed paths:", t.subtext).bold());
    }

    if let Some(rest) = line.strip_prefix("  Network: ") {
        let color = if rest.contains("blocked") {
            t.red
        } else if rest.contains("allowed") {
            t.green
        } else {
            t.blue
        };
        return format!("  {} {}", fg("Network:", t.subtext).bold(), fg(rest, color));
    }

    if line.starts_with("  /") || line.starts_with("  ~/") {
        let content = line.trim_start();
        return if let Some(idx) = content.rfind(" (") {
            format!(
                "  {} {}",
                fg(&content[..idx], t.text).bold(),
                &content[idx + 1..],
            )
        } else {
            format!("  {}", fg(content, t.text).bold())
        };
    }

    if line.starts_with("    + ") {
        return format!("    {}", fg(line.trim_start(), t.subtext));
    }

    if line.starts_with("    ") {
        return format!("    {}", fg(line.trim_start(), t.text));
    }

    line
}

/// Print dry run message
pub fn print_dry_run(
    program: &OsStr,
    cmd_args: &[OsString],
    redaction_policy: &nono::ScrubPolicy,
    silent: bool,
) {
    if silent {
        return;
    }
    let t = theme::current();
    let command_line = dry_run_command_line(program, cmd_args, redaction_policy);

    eprintln!(
        "  {} {}",
        fg("dry-run", t.yellow).bold(),
        fg(
            "sandbox would be applied with above capabilities",
            t.subtext
        ),
    );
    eprintln!("  {} {}", fg("$", t.subtext), fg(&command_line, t.text));
}

fn dry_run_command_line(
    program: &OsStr,
    cmd_args: &[OsString],
    redaction_policy: &nono::ScrubPolicy,
) -> String {
    let mut command = Vec::with_capacity(1 + cmd_args.len());
    command.push(program.to_string_lossy().into_owned());
    command.extend(
        cmd_args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned()),
    );

    format_command_line(&nono::scrub_argv_with_policy(&command, redaction_policy))
}

// ---------------------------------------------------------------------------
// Rollback / Snapshots
// ---------------------------------------------------------------------------

/// Print rollback tracking status during session start
pub fn print_rollback_tracking(paths: &[std::path::PathBuf], silent: bool) {
    if silent {
        return;
    }
    let t = theme::current();
    let display_paths = if paths.len() <= 3 { paths } else { &paths[..2] };
    for path in display_paths {
        eprintln!(
            "  {} {}",
            badge(" snap ", t.surface, t.subtext),
            fg(&path.display().to_string(), t.subtext),
        );
    }
    if paths.len() > 3 {
        eprintln!(
            "         {}",
            fg(&format!("+ {} more paths", paths.len() - 2), t.subtext),
        );
    }
}

/// Print post-exit summary of changes detected by the rollback system
pub fn print_rollback_session_summary(changes: &[nono::undo::Change], silent: bool) {
    if silent || changes.is_empty() {
        return;
    }

    let t = theme::current();

    let created = changes
        .iter()
        .filter(|c| c.change_type == nono::undo::ChangeType::Created)
        .count();
    let modified = changes
        .iter()
        .filter(|c| c.change_type == nono::undo::ChangeType::Modified)
        .count();
    let deleted = changes
        .iter()
        .filter(|c| c.change_type == nono::undo::ChangeType::Deleted)
        .count();

    let mut parts = Vec::new();
    if created > 0 {
        parts.push(format!("{}", fg(&format!("{created} created"), t.green)));
    }
    if modified > 0 {
        parts.push(format!("{}", fg(&format!("{modified} modified"), t.yellow)));
    }
    if deleted > 0 {
        parts.push(format!("{}", fg(&format!("{deleted} deleted"), t.red)));
    }

    eprintln!();
    eprintln!(
        "  {} {} files changed ({})",
        fg("nono", t.brand).bold(),
        changes.len(),
        parts.join(", "),
    );
}

// ---------------------------------------------------------------------------
// Update notification
// ---------------------------------------------------------------------------

/// Detect how nono was installed based on the binary's path.
fn detect_install_command() -> &'static str {
    let exe = match std::env::current_exe().and_then(|p| p.canonicalize()) {
        Ok(p) => p,
        Err(_) => return "cargo install nono-cli",
    };
    let path = exe.to_string_lossy();

    // Homebrew (macOS Intel or Apple Silicon)
    if path.contains("/opt/homebrew/") || path.contains("/usr/local/Cellar/") {
        return "brew upgrade nono";
    }

    // Cargo
    if path.contains("/.cargo/bin/") {
        return "cargo install nono-cli";
    }

    // Linux system package manager
    if path.starts_with("/usr/bin/") || path.starts_with("/usr/local/bin/") {
        if Path::new("/usr/bin/apt").exists() {
            return "sudo apt update && sudo apt upgrade nono";
        }
        if Path::new("/usr/bin/dnf").exists() {
            return "sudo dnf upgrade nono";
        }
        // Fallback for other system installs
        return "upgrade nono via your package manager";
    }

    "cargo install nono-cli"
}

/// Strip ANSI escape sequences and non-printable characters from a string.
///
/// Prevents terminal injection from a compromised update server.
fn sanitize_terminal_output(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ESC and the entire escape sequence
            if let Some(next) = chars.next() {
                if next == '[' {
                    // CSI sequence: skip until a letter is found
                    for seq_char in chars.by_ref() {
                        if seq_char.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                // OSC, other sequences: already consumed the next char, continue
            }
        } else if c.is_control() && c != '\n' {
            // Strip control characters (except newline)
        } else {
            result.push(c);
        }
    }
    result
}

/// Print update notification if a newer version is available
pub fn print_update_notification(info: &crate::update_check::UpdateInfo, silent: bool) {
    if silent {
        return;
    }

    let t = theme::current();
    let version = sanitize_terminal_output(&info.latest_version);
    let install_cmd = detect_install_command();
    eprintln!(
        "  {} {} {} {}",
        fg("update", t.yellow).bold(),
        fg(&version, t.green).bold(),
        fg("available", t.subtext),
        fg(
            &format!("(current: {})", env!("CARGO_PKG_VERSION")),
            t.subtext,
        ),
    );
    if let Some(ref msg) = info.message {
        let safe_msg = sanitize_terminal_output(msg);
        eprintln!("  {}", fg(&safe_msg, t.subtext));
    }
    eprintln!("  {} {}", fg("$", t.subtext), fg(install_cmd, t.text));
    if let Some(ref url) = info.release_url {
        let safe_url = sanitize_terminal_output(url);
        eprintln!("  {}", fg(&safe_url, t.blue));
    }
    eprintln!();
}

// ---------------------------------------------------------------------------
// Interactive prompts
// ---------------------------------------------------------------------------

/// Prompt the user to confirm sharing the current working directory.
///
/// Returns `Ok(true)` if user confirms, `Ok(false)` if user declines.
/// Returns `Ok(false)` with a hint if stdin is not a TTY.
pub fn prompt_cwd_sharing(cwd: &Path, access: &AccessMode) -> Result<bool> {
    let t = theme::current();
    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        eprintln!(
            "  {}",
            fg(
                "Skipping CWD prompt (non-interactive). Use --allow-cwd to include working directory.",
                t.subtext,
            ),
        );
        return Ok(false);
    }

    let access_colored = format_access_inline(access);

    eprintln!(
        "  Share {} with {} access?",
        fg(&cwd.display().to_string(), t.text).bold(),
        access_colored,
    );
    eprintln!("  {}", fg("use --allow-cwd to skip this prompt", t.subtext),);
    eprint!("  {} ", fg("[y/N]", t.text).bold());
    std::io::stderr().flush().ok();

    let mut input = String::new();
    stdin.lock().read_line(&mut input).map_err(NonoError::Io)?;

    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

pub fn print_profile_hint(program: &str, profile: &str, silent: bool) {
    if silent {
        return;
    }

    let t = theme::current();
    eprintln!(
        "  {}",
        fg(
            &format!(
                "Hint: `{program}` usually needs the built-in `{profile}` profile for its state and auth paths."
            ),
            t.yellow,
        )
    );
    eprintln!(
        "  {}",
        fg(
            &format!("Try: nono run --profile {profile} -- {program}"),
            t.subtext,
        )
    );
    eprintln!();
}

/// Phase 117 review WR-11 non-vacuity: the session-id validator must reject
/// every traversal shape, and the marker-path helper must return `None`
/// (always-print) rather than a path outside the sessions root.
#[cfg(all(test, target_os = "windows"))]
mod attestation_marker_path_tests {
    use super::{
        attestation_downgrade_marker_content, attestation_downgrade_marker_path,
        marker_says_already_announced, session_id_is_safe_path_component,
    };

    /// WR-04: the four ways a marker can be present-but-not-an-announcement
    /// must all resolve to "announce". Only an exact content match suppresses.
    #[test]
    fn only_an_exact_key_match_suppresses_the_banner() {
        let dir = std::env::temp_dir().join(format!(
            "nono-wr04-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("marker");
        let key = "DaclAncestorTraverse,WfpEgressFilters";

        // 1. Absent.
        assert!(
            !marker_says_already_announced(&path, key),
            "an absent marker must not suppress"
        );

        // 2. Zero-byte — the pre-plantable shape the old `path.exists()`
        //    check trusted, and the one that made the 'unconditional' channel
        //    silenceable.
        std::fs::write(&path, []).expect("write empty");
        assert!(
            !marker_says_already_announced(&path, key),
            "a zero-byte marker must not suppress (WR-04): presence alone was the whole \
             suppression condition before this fix"
        );

        // 3. Different content — the 64-bit-collision case, where a
        //    genuinely different downgraded-layer-set hashes to this name.
        std::fs::write(&path, b"SomeOtherLayerSet").expect("write other");
        assert!(
            !marker_says_already_announced(&path, key),
            "a marker holding a DIFFERENT key must not suppress — that is a hash collision \
             between two different downgraded-layer-sets, not a repeat"
        );

        // 3b. CR-01 control: the PLAINTEXT key is no longer the stored form,
        //     so a marker holding it must NOT suppress. This is the pin that
        //     stops the plaintext write being quietly restored — reverting
        //     the writer alone would leave every other case in this test
        //     green while re-opening the D-28 disclosure.
        std::fs::write(&path, key.as_bytes()).expect("write plaintext key");
        assert!(
            !marker_says_already_announced(&path, key),
            "the plaintext dedup key must not be an acceptable marker content (CR-01): the \
             stored form is a digest, because this file is readable by the confined child"
        );

        // 4. Exact match — the one genuine repeat.
        std::fs::write(&path, attestation_downgrade_marker_content(key).as_bytes())
            .expect("write digest");
        assert!(
            marker_says_already_announced(&path, key),
            "an exact content match IS a genuine repeat and must suppress, or the dedup does \
             nothing and the hook path re-prints on every tool call"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// CR-01/D-28: whatever the marker stores, it must not NAME the layers.
    ///
    /// The property is stated over the content function directly rather than
    /// over one sample string, because the defect this pins was introduced by
    /// changing what the writer passes to `write_all`. Two halves:
    ///
    /// 1. *Non-disclosure* — for a key naming every layer this codebase can
    ///    downgrade, no fragment of any layer name survives into the content.
    ///    The needles are read from the caller's own vocabulary
    ///    (`LayerId::ALL`, via the launch-side sibling test) plus the literal
    ///    key text here, so this cannot pass by testing a name that no longer
    ///    exists.
    /// 2. *Injectivity* — distinct keys must still produce distinct content,
    ///    or the digest would silently re-open the collision hole WR-04
    ///    closed. Checked exhaustively over a spanning sample of key shapes.
    #[test]
    fn marker_content_never_names_a_layer_and_stays_injective() {
        // Deliberately built from key TEXT, not from an imported enum:
        // `output.rs` must not learn the layer identity type (D-28's
        // structural half). The launch-side sibling
        // `downgrade_marker_files_never_contain_a_layer_name` drives the same
        // property from the real `LayerId::ALL` vocabulary end to end.
        let keys = [
            "",
            "DaclAncestorTraverse",
            "WfpEgressFilters",
            "DaclAncestorTraverse,WfpEgressFilters",
            "WfpEgressFilters,DaclAncestorTraverse",
            "JobObjectContainment,MandatoryIntegrityLabel,RestrictedToken",
        ];

        for key in keys {
            let content = attestation_downgrade_marker_content(key);
            assert_eq!(
                content.len(),
                32,
                "marker content must be a fixed-width 128-bit hex digest, got {content:?}"
            );
            assert!(
                content.chars().all(|c| c.is_ascii_hexdigit()),
                "marker content must be hex only — anything else is a channel for key text: \
                 {content:?}"
            );
            // Non-disclosure: no comma-separated fragment of the key may
            // appear in the content, in any casing.
            for fragment in key.split(',').filter(|f| !f.is_empty()) {
                assert!(
                    !content
                        .to_ascii_lowercase()
                        .contains(&fragment.to_ascii_lowercase()),
                    "CR-01/D-28: marker content {content:?} contains the layer name \
                     {fragment:?}. This file is readable by the confined child on the Null, \
                     WriteRestricted and Low-IL arms; the layer set must never be written \
                     there in plaintext."
                );
            }
        }

        // Injectivity over the sample, including the order-swapped pair —
        // production sorts before joining, but the content function must not
        // be the thing relying on that.
        for (i, a) in keys.iter().enumerate() {
            for b in keys.iter().skip(i + 1) {
                assert_ne!(
                    attestation_downgrade_marker_content(a),
                    attestation_downgrade_marker_content(b),
                    "distinct dedup keys {a:?} and {b:?} collide in marker content — that \
                     re-opens the WR-04 collision hole the content check exists to close"
                );
            }
        }
    }

    #[test]
    fn accepts_the_generated_session_id_shapes() {
        assert!(session_id_is_safe_path_component(
            "0f8b2c1d4e5a6b7c8d9e0f1a2b3c4d5e"
        ));
        assert!(session_id_is_safe_path_component(
            "117-10-gate-test_session"
        ));
    }

    #[test]
    fn rejects_traversal_and_separator_shapes() {
        for hostile in [
            "",
            "..",
            "../..",
            r"..\..",
            "a/b",
            r"a\b",
            "C:",
            r"C:\Windows",
            r"\\server\share",
            "name:stream",
            "trailing.",
            "trailing ",
            "sess\u{00ad}id",
        ] {
            assert!(
                !session_id_is_safe_path_component(hostile),
                "{hostile:?} must be rejected as a session id"
            );
            assert!(
                attestation_downgrade_marker_path(hostile, "k").is_none(),
                "{hostile:?} must yield no marker path (always-print), never an escaped one"
            );
        }
    }

    /// WR-11: bare Windows reserved device names are pure ASCII
    /// alphanumerics, so the allow-list alone accepted every one of them
    /// while the doc claimed they were "structurally excluded". Checks the
    /// whole class (all 24 names, both cases) rather than the one or two a
    /// hand-written list would remember.
    #[test]
    fn rejects_bare_reserved_device_names() {
        for base in [
            "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM5", "COM9", "LPT0", "LPT1", "LPT5",
            "LPT9",
        ] {
            for variant in [base.to_string(), base.to_lowercase(), {
                let mut s = base.to_lowercase();
                s.replace_range(0..1, &base[0..1]);
                s
            }] {
                assert!(
                    !session_id_is_safe_path_component(&variant),
                    "{variant:?} is a Win32 reserved device name and must be rejected as a \
                     session id (WR-11); the Win32 device namespace is case-insensitive"
                );
                assert!(
                    attestation_downgrade_marker_path(&variant, "k").is_none(),
                    "{variant:?} must yield no marker path (always-print)"
                );
            }
        }

        // Control: reserved-name PREFIXES and suffixed forms are ordinary
        // components and must still be accepted, or the exclusion has
        // over-reached into rejecting legitimate ids.
        for benign in ["CONSOLE", "NULL", "COM10", "LPT10", "con-1", "aux_x"] {
            assert!(
                session_id_is_safe_path_component(benign),
                "{benign:?} is not a reserved device name and must still be accepted — an \
                 over-broad exclusion silently disables dedup for legitimate sessions"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::theme;
    use super::{
        dry_run_command_line, format_unix_socket_mode_badge, normalize_terminal_line_endings,
        print_blocked_grants, print_capabilities, print_profile_hint, render_diagnostic_footer,
    };
    use nono::{CapabilitySet, UnixSocketMode};
    use std::ffi::{OsStr, OsString};
    use tempfile::tempdir;

    #[test]
    fn normalize_terminal_line_endings_uses_crlf() {
        assert_eq!(
            normalize_terminal_line_endings("line one\nline two"),
            "line one\r\nline two"
        );
    }

    #[test]
    fn render_diagnostic_footer_preserves_line_structure() {
        let footer = "nono diagnostic\n────────\nThe command failed.\n  Learn: nono learn";
        let rendered = render_diagnostic_footer(footer);
        assert_eq!(rendered.lines().count(), 4);
    }

    #[test]
    fn render_diagnostic_footer_splits_path_on_last_paren_group() {
        // Path contains " (" in the directory name — rfind ensures we split on
        // the *last* parenthesised group (the access type), not the one
        // embedded in the path.
        let footer = "  /home/user/my (project)/file (read)";
        let rendered = render_diagnostic_footer(footer);
        assert!(
            rendered.contains("/home/user/my (project)/file"),
            "path with embedded parens should be preserved: {rendered}"
        );
        assert!(
            rendered.contains("(read)"),
            "access type should be preserved: {rendered}"
        );
    }

    #[test]
    fn print_profile_hint_is_noop_when_silent() {
        print_profile_hint("claude", "claude-code", true);
    }

    #[test]
    fn dry_run_command_line_redacts_default_secrets() {
        let line = dry_run_command_line(
            OsStr::new("curl"),
            &[
                OsString::from("--token"),
                OsString::from("real-token"),
                OsString::from("https://example.com/api?token=real-secret"),
            ],
            &nono::ScrubPolicy::secure_default(),
        );

        assert!(line.contains("[REDACTED]"));
        assert!(!line.contains("real-token"));
        assert!(!line.contains("real-secret"));
    }

    #[test]
    fn dry_run_command_line_uses_configured_redaction_policy() {
        let mut redactions = nono::ScrubPolicy::secure_default();
        redactions.add_flag("--private-token");

        let line = dry_run_command_line(
            OsStr::new("curl"),
            &[OsString::from("--private-token=private-secret")],
            &redactions,
        );

        assert_eq!(line, "curl '--private-token=[REDACTED]'");
        assert!(!line.contains("private-secret"));
    }

    #[test]
    fn unix_socket_mode_badges_are_fixed_width_and_distinct() {
        let connect = format_unix_socket_mode_badge(UnixSocketMode::Connect);
        let bind = format_unix_socket_mode_badge(UnixSocketMode::ConnectBind);
        // Same rendered-width contract as format_access_badge (5 chars).
        // We can't `strip_ansi` cleanly here, so check the printable payload
        // is present rather than the raw length.
        assert!(connect.contains("sock "));
        assert!(bind.contains("sock+"));
        assert_ne!(connect, bind);
    }

    #[test]
    fn print_capabilities_with_unix_socket_does_not_panic() {
        // Smoke test: constructing a CapabilitySet with both connect and
        // connect+bind unix socket grants (one file, one directory) and
        // rendering it must not panic. Silent=true keeps stderr quiet in
        // test output. Dry-run-style `verbose=1` path is also exercised.
        let dir = tempdir().expect("tempdir");
        let sock = dir.path().join("a.sock");
        std::fs::write(&sock, b"").expect("create socket stub");

        let caps = CapabilitySet::new()
            .allow_unix_socket(&sock, UnixSocketMode::Connect)
            .expect("connect grant")
            .allow_unix_socket_dir(dir.path(), UnixSocketMode::ConnectBind)
            .expect("bind dir grant");

        print_capabilities(&caps, &[], 0, true, false);
        print_capabilities(&caps, &[], 1, true, false);
    }

    #[test]
    fn print_blocked_grants_collapsed_and_verbose_do_not_panic() {
        // Blocked grants render as one folded row by default and expand under
        // -v; both paths (and the empty case) must render without panicking.
        let t = theme::current();
        let blocked = vec![
            (
                std::path::PathBuf::from("/Users/x/Library/Application Support/Google/Chrome"),
                Some("deny_browser_data_macos".to_string()),
            ),
            (
                std::path::PathBuf::from("/Users/x/Library/Application Support/1Password"),
                Some("deny_keychains_macos".to_string()),
            ),
            (
                std::path::PathBuf::from("/Users/x/Library/Application Support/Unknown"),
                None,
            ),
        ];

        print_blocked_grants(&blocked, 0, t);
        print_blocked_grants(&blocked, 1, t);
        print_blocked_grants(&[], 0, t);
    }

    /// Phase 117-12, Item-2 (checker pass 3): measures
    /// `print_attestation_downgrade_banner`'s per-session dedup marker cost
    /// COLD (marker absent — first occurrence, does the `create_dir_all` +
    /// `write`) versus WARM (marker present — repeat occurrence, only the
    /// `path.exists()` stat) separately, per the plan's explicit instruction
    /// that these two figures must not be folded into one blended number —
    /// the hook path (`claude_code_hook.rs`) re-enters this exact call on
    /// every tool call, so the warm case is the steady-state cost for a
    /// long-running session. Uses a unique, process-scoped `session_id` so
    /// this test never collides with real session state and cleans up its
    /// own marker directory afterward.
    #[cfg(target_os = "windows")]
    #[test]
    fn attestation_downgrade_banner_cold_vs_warm_dedup_marker_latency() {
        use super::{attestation_downgrade_marker_path, print_attestation_downgrade_banner};

        // Serialize against the other test that writes under the real sessions
        // root; see SESSIONS_ROOT_TEST_LOCK's doc for the measured contention.
        let _guard = super::SESSIONS_ROOT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let session_id = format!(
            "test-latency-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        );
        let dedup_key = "TestLayer";

        let cold_start = std::time::Instant::now();
        print_attestation_downgrade_banner(
            1,
            Some(&session_id),
            dedup_key,
            crate::exec_strategy::attestation_downgrade_event::DowngradeDetailChannel::EventLog,
        );
        let cold_elapsed = cold_start.elapsed();

        let warm_start = std::time::Instant::now();
        print_attestation_downgrade_banner(
            1,
            Some(&session_id),
            dedup_key,
            crate::exec_strategy::attestation_downgrade_event::DowngradeDetailChannel::EventLog,
        );
        let warm_elapsed = warm_start.elapsed();

        eprintln!(
            "D-24/Item-2 measured downgrade-banner dedup marker cost: cold (first occurrence, \
             create_dir_all + write) = {cold_elapsed:?}; warm (repeat, read + digest compare) \
             = {warm_elapsed:?}"
        );

        // Best-effort cleanup: remove the marker this test created so repeat
        // runs do not accumulate stale directories under the real sessions
        // root. Non-fatal if the session-dir resolution differs.
        if let Some(marker) = attestation_downgrade_marker_path(&session_id, dedup_key) {
            if let Some(session_dir) = marker.parent().and_then(|p| p.parent()) {
                let _ = std::fs::remove_dir_all(session_dir);
            }
        }

        assert!(
            cold_elapsed < std::time::Duration::from_millis(250),
            "cold dedup marker write took {cold_elapsed:?}, exceeding the generous 250ms \
             sanity bound"
        );
        assert!(
            warm_elapsed < std::time::Duration::from_millis(100),
            "warm dedup marker stat took {warm_elapsed:?}, exceeding the generous 100ms \
             sanity bound"
        );
    }

    /// Every `.rs` file on the D-27 downgrade surface, as
    /// `(label, source)` — the files whose string literals reach an operator
    /// (or assert on text that does).
    const DOWNGRADE_SURFACE: &[(&str, &str)] = &[
        ("output.rs", include_str!("output.rs")),
        (
            "exec_strategy_windows/launch.rs",
            include_str!("exec_strategy_windows/launch.rs"),
        ),
        (
            "exec_strategy_windows/attestation.rs",
            include_str!("exec_strategy_windows/attestation.rs"),
        ),
        (
            "exec_strategy_windows/attestation_downgrade_event.rs",
            include_str!("exec_strategy_windows/attestation_downgrade_event.rs"),
        ),
        ("main.rs", include_str!("main.rs")),
    ];

    /// Length in chars of a complete Rust char literal starting at `i`, or
    /// `None` if this `'` opens something else (a lifetime, a stray quote).
    ///
    /// WR-06: the lookahead matters. Blindly treating `'` as a char-literal
    /// opener would consume from a lifetime (`&'a str`) to the next `'`
    /// anywhere on the line, silently swallowing any string literal in
    /// between — trading one false negative for a worse one.
    fn char_literal_len(chars: &[char], i: usize) -> Option<usize> {
        if chars.get(i) != Some(&'\'') {
            return None;
        }
        let mut j = i + 1;
        if chars.get(j) == Some(&'\\') {
            j += 1;
            if chars.get(j) == Some(&'u') {
                // '\u{XXXX}' — scan to the closing quote.
                while j < chars.len() && chars[j] != '\'' {
                    j += 1;
                }
            } else {
                j += 1;
            }
        } else {
            j += 1;
        }
        (chars.get(j) == Some(&'\'')).then(|| j - i + 1)
    }

    /// Extract the *contents* of every double-quoted string literal on `line`.
    ///
    /// No raw-string handling: the files it is pointed at contain none on the
    /// lines that matter, and a false positive there fails the build loudly
    /// rather than silently passing — the correct direction for a gate whose
    /// whole purpose is to stop a silent corruption.
    ///
    /// # WR-06: char literals ARE handled, because the doc's excuse was false
    ///
    /// The previous version documented itself as having no char-literal
    /// handling on the grounds that the scanned files contain none. `output.rs`
    /// is one of the scanned files and contains `else if c == '"' {` twice —
    /// inside this very function. On those lines the extractor opened a string
    /// at the char literal's quote and never closed it, so the entire rest of
    /// the line was DISCARDED rather than scanned. That is a false NEGATIVE
    /// (silently scanning less), the opposite of the direction the doc claimed,
    /// and it is precisely what the `checked` floor exists to catch and is far
    /// too coarse to catch — 1417 literals scanned made a handful of dropped
    /// lines invisible. Hence also the per-file floors at the call site.
    fn string_literals(line: &str) -> Vec<String> {
        let chars: Vec<char> = line.chars().collect();
        let mut out = Vec::new();
        let mut i = 0usize;
        while i < chars.len() {
            if chars[i] == '\'' {
                if let Some(len) = char_literal_len(&chars, i) {
                    i += len;
                    continue;
                }
                i += 1;
                continue;
            }
            if chars[i] != '"' {
                i += 1;
                continue;
            }
            i += 1;
            let mut cur = String::new();
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    // Keep the escaped character, drop the backslash — the
                    // pre-WR-06 behaviour, preserved deliberately.
                    cur.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    i += 1;
                    closed = true;
                    break;
                }
                cur.push(chars[i]);
                i += 1;
            }
            // Only a CLOSED literal is a literal. A `\`-continued literal ends
            // the line unterminated; scanning its partial content would change
            // what this gate covers, which is a separate decision from WR-06.
            if closed {
                out.push(cur);
            }
        }
        out
    }

    /// WR-06 self-test: the extractor must not lose a string literal that
    /// shares a line with a char literal.
    ///
    /// This is the exact shape that was live in `output.rs` — `else if c ==
    /// '"' {` — plus the lifetime case, which a naive char-literal fix breaks
    /// in the same (silent, scan-less) direction.
    #[test]
    fn string_literal_extraction_survives_char_literals_and_lifetimes() {
        assert_eq!(
            string_literals(r#"} else if c == '"' { let s = "kept"; }"#),
            vec!["kept".to_string()],
            "a `'\"'` char literal must not open a string and swallow the rest of the line"
        );
        assert_eq!(
            string_literals(r#"fn f<'a>(x: &'a str) { let s = "also kept"; }"#),
            vec!["also kept".to_string()],
            "a lifetime must not be treated as an unterminated char literal"
        );
        assert_eq!(
            string_literals(r#"let c = '\''; let s = "still kept";"#),
            vec!["still kept".to_string()],
            "an escaped-quote char literal must be consumed whole"
        );
        assert_eq!(
            string_literals(r#"let s = "a\"b";"#),
            vec!["a\"b".to_string()],
            "an escaped quote inside a string must not close it"
        );
    }

    /// WR-01 class gate: no string literal on the D-27 downgrade surface may
    /// carry a *collapsed line continuation*.
    ///
    /// A bad automated edit replaced `\`-continuations with the literal
    /// newline plus its indentation, embedding runs of 14-22 spaces
    /// mid-sentence in operator-facing text. That is not cosmetic: it is the
    /// mechanism that made `every_operator_detail_pointer_is_conditional`
    /// (CR-01) cover zero sites in `launch.rs`, because the needle it
    /// searches for had been split by such a run. Fixing the twelve literals
    /// without this gate would leave the next automated edit free to
    /// reintroduce them — and the class gate above silently vacuous again.
    ///
    /// The predicate is "a run of 4+ spaces with a non-space character before
    /// it", which admits leading-indentation literals (`"       {}"`, of
    /// which `output.rs` has several by design) and rejects mid-sentence
    /// runs.
    #[test]
    fn no_downgrade_surface_literal_has_a_collapsed_continuation() {
        let mut checked = 0usize;
        let mut per_file: Vec<(&str, usize)> = Vec::new();
        let mut offenders: Vec<String> = Vec::new();
        for (label, src) in DOWNGRADE_SURFACE {
            let before = checked;
            for (idx, line) in src.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for lit in string_literals(line) {
                    checked += 1;
                    let bytes: Vec<char> = lit.chars().collect();
                    let mut i = 0usize;
                    while i < bytes.len() {
                        if bytes[i] == ' ' {
                            let start = i;
                            while i < bytes.len() && bytes[i] == ' ' {
                                i += 1;
                            }
                            if i - start >= 4 && start > 0 && bytes[start - 1] != ' ' {
                                offenders.push(format!(
                                    "{label}:{}: {} consecutive spaces mid-literal",
                                    idx + 1,
                                    i - start
                                ));
                                break;
                            }
                        } else {
                            i += 1;
                        }
                    }
                }
            }
            per_file.push((label, checked - before));
        }
        assert!(
            offenders.is_empty(),
            "WR-01: {} string literal(s) on the D-27 downgrade surface carry a collapsed \
             line continuation (a `\\`-continuation replaced by literal spaces). These render \
             verbatim to the operator and can split a class gate's needle:\n  {}",
            offenders.len(),
            offenders.join("\n  ")
        );
        assert!(
            checked >= 500,
            "WR-01: only {checked} string literal(s) scanned — the extractor went vacuous \
             (a gate that scans nothing is the defect this phase exists to eliminate)"
        );
        // WR-06: a global floor of 500 is far too coarse to notice one file
        // going quiet — the char-literal bug dropped whole lines inside a
        // 1417-literal scan and nothing moved. Each file on the surface must
        // contribute its own non-trivial share.
        const PER_FILE_FLOOR: usize = 25;
        for (label, n) in &per_file {
            assert!(
                *n >= PER_FILE_FLOOR,
                "WR-06: only {n} string literal(s) scanned in {label} (floor \
                 {PER_FILE_FLOOR}); the extractor has gone quiet for this file. Per-file \
                 counts: {per_file:?}"
            );
        }
    }

    /// WR-26 class gate (Phase 117-44): no operator-facing string may name a
    /// detail destination unconditionally.
    ///
    /// Fixing only the two sites WR-26 named would repeat this phase's defining
    /// failure — three consecutive rounds each closed the enumerated sites and
    /// not the class. This scans every production string literal in the files
    /// that render downgrade guidance and requires each mention of the
    /// Application event log to sit inside a `DowngradeDetailChannel::EventLog`
    /// arm, i.e. to be reached only when that channel actually received the
    /// record.
    ///
    /// `main.rs` is included because WR-27 is the same class on the abort path:
    /// there the event log receives NOTHING, so it must never be named as a
    /// place to look at all.
    ///
    /// # CR-01: why the production split matters
    ///
    /// The first version of this gate covered **zero** sites in `launch.rs`,
    /// for two independent reasons, and so could relabel but never deny:
    ///
    /// 1. It ended the production half at `l.trim() != "mod tests {"`.
    ///    `launch.rs` has no such line — its twelve test modules are named
    ///    `attestation_gate_tests`, `job_hardening_tests`, ... — so the scan
    ///    silently ran over the whole 5862-line file, the opposite of the
    ///    stated invariant. `output.rs`'s first test module is not named
    ///    `tests` either.
    /// 2. The needle was matched against raw source lines, so a `\`-continued
    ///    literal that straddles two lines — or, as WR-01 found, one whose
    ///    continuation had collapsed into a run of literal spaces — was
    ///    invisible. Lines are still continuation-joined and
    ///    whitespace-normalised before matching.
    ///
    /// # WR-01: the split is no longer implemented here
    ///
    /// The second version of the split lived in this function as a private
    /// helper, and it disagreed with the mirror written in the same fix pass
    /// for `layer_registry.rs`'s daemon gate: it cleared its pending-attribute
    /// flag on any intervening line, so the six `launch.rs` test modules that
    /// carry `#[allow(clippy::unwrap_used)]` between `#[cfg(all(test, ...))]`
    /// and `mod` were scanned as production — **1773 lines of test code**.
    /// There is now exactly one implementation, in
    /// `crate::cfg_test_regions`, with its own unit tests for the rule; both
    /// gates call it. See that module's doc for the rule itself.
    ///
    /// # Non-vacuity is asserted PER FILE
    ///
    /// The previous floor was `hits >= 2` summed across two files, while the
    /// doc claimed a per-file guarantee. Each file contributed exactly 1, so
    /// the floor happened to be tight — but nothing stopped `launch.rs` going
    /// to 0 while `output.rs` drifted to 2, and the record told the next
    /// maintainer that case was covered. The floor is now per file, and each
    /// file must also have contributed at least one SKIPPED test region, so a
    /// marker change cannot silently turn "production half only" into a
    /// whole-file scan.
    #[test]
    fn every_operator_detail_pointer_is_conditional() {
        use crate::cfg_test_regions::scan_production;

        const OUTPUT_SRC: &str = include_str!("output.rs");
        const LAUNCH_SRC: &str = include_str!("exec_strategy_windows/launch.rs");
        const MAIN_SRC: &str = include_str!("main.rs");

        const NEEDLE: &str = "Windows Application event log";

        /// Production entries of `src` as `(first_line_index, normalised_text)`,
        /// with `\`-continued literals rejoined so a needle split across
        /// source lines is still seen.
        ///
        /// Returns the `ProductionScan` alongside, so the caller can assert
        /// the test-region split actually happened for this file.
        fn production(
            src: &str,
        ) -> (
            Vec<(usize, String)>,
            crate::cfg_test_regions::ProductionScan<'_>,
        ) {
            let scan = scan_production(src);
            let mut out: Vec<(usize, String)> = Vec::new();
            let mut i = 0usize;
            while i < scan.lines.len() {
                let (start, first) = scan.lines[i];
                let mut merged = first.to_string();
                // Continuation lines are contiguous in the source AND kept by
                // the scan (they are neither comments nor cfg attributes), so
                // requiring index contiguity here cannot re-admit skipped text.
                while merged.trim_end().ends_with('\\')
                    && i + 1 < scan.lines.len()
                    && scan.lines[i + 1].0 == scan.lines[i].0 + 1
                {
                    merged.truncate(merged.trim_end().len() - 1);
                    i += 1;
                    merged.push_str(scan.lines[i].1);
                }
                out.push((
                    start,
                    merged.split_whitespace().collect::<Vec<_>>().join(" "),
                ));
                i += 1;
            }
            (out, scan)
        }

        for (label, src, floor) in [
            ("output.rs", OUTPUT_SRC, 1usize),
            ("exec_strategy_windows/launch.rs", LAUNCH_SRC, 1usize),
        ] {
            let (entries, scan) = production(src);
            assert!(
                !scan.skipped_regions.is_empty(),
                "WR-01: no `#[cfg(test)]` module region was excluded from {label}, so this \
                 gate is scanning the whole file including test assertions that legitimately \
                 name the event log. Either the file lost its test modules, or \
                 cfg_test_regions no longer recognises them."
            );
            // The split must be CORRECT, not merely non-empty. `!scan
            // .skipped_regions.is_empty()` would have stayed green through
            // WR-01's actual defect, where 6 of launch.rs's 12 test modules
            // leaked into the production half because an `#[allow(...)]` sat
            // between the cfg attribute and the `mod` line. A leaked module
            // brings its `#[test]` attributes with it, so this catches the
            // class directly and without restating the classifier's own rule.
            let leaked: Vec<usize> = entries
                .iter()
                .filter(|(_, l)| l.trim() == "#[test]")
                .map(|(i, _)| i + 1)
                .collect();
            assert!(
                leaked.is_empty(),
                "WR-01: {} `#[test]` attribute(s) appear in the PRODUCTION half of {label} \
                 (first at line {:?}), so at least one `#[cfg(test)]` module leaked into the \
                 scan and its assertions can satisfy this gate.",
                leaked.len(),
                leaked.first()
            );

            let mut hits = 0usize;
            for (idx, line) in &entries {
                if !line.contains(NEEDLE) {
                    continue;
                }
                hits += 1;
                // Walk back to the nearest DowngradeDetailChannel arm, over
                // PRODUCTION entries only. Searching raw lines would let a
                // comment naming the arm within 12 lines above satisfy the
                // gate — prose about the rule is not an instance of it.
                let arm = entries
                    .iter()
                    .filter(|(j, _)| j <= idx)
                    .rev()
                    .take(12)
                    .find_map(|(_, l)| {
                        l.find("DowngradeDetailChannel::")
                            .map(|i| l[i + "DowngradeDetailChannel::".len()..].to_string())
                    })
                    .unwrap_or_default();
                assert!(
                    arm.starts_with("EventLog"),
                    "WR-26: {label}:{} names the Windows Application event log but is not \
                     inside a `DowngradeDetailChannel::EventLog` arm, so it can be rendered \
                     when that channel received nothing. Nearest arm found: {arm:?}
line: {}",
                    idx + 1,
                    line.trim()
                );
            }

            // Non-vacuity (CR-01/WR-01), PER FILE. `output.rs`'s
            // downgrade-banner EventLog arm and `launch.rs`'s
            // `downgrade_detail_pointer` EventLog arm are each a known site;
            // a file dropping to zero means the needle has drifted out of
            // that file's production text, which is indistinguishable from
            // "the class is clean" without this check.
            assert!(
                hits >= floor,
                "CR-01/WR-01: the WR-26 class gate matched {hits} site(s) in {label} (floor \
                 {floor}). Each scanned file must contribute at least one known \
                 DowngradeDetailChannel::EventLog arm site, or the needle has drifted out of \
                 that file's production text and the gate can relabel but never deny."
            );
        }

        // WR-27: on the abort path no record is ever written to the event log,
        // so main.rs must not name it as a destination at all.
        //
        // WR-06: the predicate is the CLASS — "mentions the event log" — not
        // one verb. The previous needle was the literal `see the Windows
        // Application event log`, which `check the ...`, `in the ...`, `look
        // in the ...` or any other phrasing evaded, so the guard could
        // relabel but never deny. `main.rs`'s one legitimate mention is a
        // NEGATION ("No Windows Application event log record is written on
        // this path"); that is excluded by requiring the negation, not by
        // narrowing the needle.
        let mut main_mentions = 0usize;
        let (main_entries, main_scan) = production(MAIN_SRC);
        assert!(
            !main_scan.skipped_regions.is_empty(),
            "WR-01: no `#[cfg(test)]` module region was excluded from main.rs — its test \
             modules assert ON the rendered remediation text, so scanning them would let a \
             test satisfy this gate"
        );
        for (idx, line) in main_entries {
            for (pos, _) in line.match_indices(NEEDLE) {
                main_mentions += 1;
                let preceding = &line[..pos];
                assert!(
                    preceding.ends_with("No ") || preceding.ends_with("no "),
                    "WR-27/WR-06: main.rs:{} mentions the Windows Application event log \
                     other than as an explicit negation. No record is written there on the \
                     abort path — every LayerAttestationFailed construction site is a plain \
                     `return Err(..)` — so it must never be named as a place to look.
line: {line}",
                    idx + 1,
                );
            }
        }
        assert!(
            main_mentions >= 1,
            "WR-06 non-vacuity: main.rs no longer mentions the Windows Application event log \
             at all, so this guard now checks nothing. `render_error_for_operator`'s `_` arm \
             is expected to carry the explicit negation that tells the operator NOT to look \
             there; restore it, or retire this guard deliberately."
        );
    }
}
