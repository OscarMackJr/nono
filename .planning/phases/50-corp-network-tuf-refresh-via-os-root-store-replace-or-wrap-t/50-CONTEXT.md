# Phase 50: Corp-network TUF refresh via OS root store - Context

**Gathered:** 2026-05-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace the single `nono::trust::TrustedRoot::production()` call inside `crates/nono-cli/src/setup.rs::refresh_trust_root_step` with a nono-local TUF chain-walk that uses `tough` (verification math) plus `ureq` v3 with the `platform-verifier` feature (HTTP transport consulting the OS root store), so that `nono setup --refresh-trust-root` succeeds on Windows hosts behind TLS-inspecting corporate proxies whose enterprise CA is in the Windows root store but not in the Mozilla `webpki-roots` bundle that reqwest 0.12.28 ships with.

The cache file produced (`<nono_home>/.nono/trust-root/trusted_root.json`) must be byte-identical to what `TrustedRoot::production()` would have written, so Phase 32's offline-verify path (`crates/nono/src/trust/bundle.rs:147` `load_production_trusted_root`) remains unchanged.

</domain>

<spec_lock>
## Requirements (locked via SPEC.md)

**6 requirements are locked.** See `50-SPEC.md` for full requirements, boundaries, and acceptance criteria.

Downstream agents MUST read `50-SPEC.md` before planning or implementing. Requirements are not duplicated here.

**In scope (from SPEC.md):**
- Replacing the body of `refresh_trust_root_step` with a new nono-local TUF chain-walk function
- Wiring `tough 0.22.0` + `ureq 3 (platform-verifier)` as the verification + transport stack
- Producing a byte-identical `trusted_root.json` cache file
- 4 hermetic unit/integration tests for the new chain-walk
- 1 HUMAN-UAT scenario for the corp-network proof on Windows
- Documentation update at `docs/cli/development/windows-poc-handoff.mdx` noting that v0.53.x+ corp-network refresh works natively (no `--from-file` required)

**Out of scope (from SPEC.md):**
- Any sigstore-rs HTTP egress other than `--refresh-trust-root` (Rekor inclusion-proof fetches, Fulcio signing, OIDC, freshness-online-probes)
- Linux/macOS-specific user-facing behavior changes (the code path itself is cross-platform per D-50-04, but the user-impact contract is "Windows corp-network failure resolved")
- Removing or modifying the `--from-file` flag
- A live CI integration test with a real corp-network MITM proxy
- Upstream `sigstore-rs` PR adding a `TrustedRoot::with_http_client(...)` seam (Surface (b))
- Hand-rolled TUF signature verification

</spec_lock>

<decisions>
## Implementation Decisions

### Code placement (Area 1)

- **D-50-01:** New sibling module at `crates/nono-cli/src/trust_refresh.rs` holds the chain-walk implementation. `crates/nono-cli/src/setup.rs::refresh_trust_root_step` delegates to it via a single function call. The new module is NOT moved into the `nono` library — `crates/nono` deliberately has no HTTP transport dependencies (P32-CHK-002 / D-32-15 keep `crates/nono` HTTP-free; that invariant is preserved).
- **D-50-02:** Public surface is a single free function: `pub fn refresh_production_trusted_root() -> nono::Result<sigstore_verify::trust_root::TrustedRoot>`. It is a swap-in replacement for `TrustedRoot::production()` — `refresh_trust_root_step` keeps the [X/N] header, the `serde_json::to_string_pretty` serialization, and the `std::fs::write` call exactly as they are at `setup.rs:857-865`. Only the call into sigstore-rs becomes a call into the new function.
- **D-50-03:** Tests live in the same file via `#[cfg(test)] mod tests {}`. No new test directory or integration-test crate is introduced.

### `tough` API surface (Area 2)

- **D-50-04:** `tough::RepositoryLoader` drives the chain walk. nono provides a `struct UreqTransport(ureq::Agent)` that `impl tough::Transport`. Surface (b) (hand-rolled `tough::schema::Signed<Root>` loop) is explicitly REJECTED — security-critical signature math stays in `tough`.
- **D-50-05:** `tough 0.22.0` is promoted from a transitive dep (already in `Cargo.lock` via sigstore-trust-root 0.7.0) to a DIRECT dep of `nono-cli`. Version pin matches the existing lockfile entry to avoid a second copy.
- **D-50-06:** The embedded v14 trust anchor is sourced from `sigstore_trust_root::PRODUCTION_TUF_ROOT` (a `&[u8]` const exported at `sigstore-trust-root-0.7.0/src/tuf.rs:60`). nono does NOT ship its own copy of `tuf_root.json`. This keeps the embedded anchor synced to whatever sigstore-trust-root currently considers production — the same code path that already self-verifies cleanly today.
- **D-50-07:** Cache TUF datastore location for the chain walk's local state is `<nono_home>/.nono/trust-root/tuf-cache/` (sibling of the existing `trusted_root.json` cache file). Created if missing. Best-effort cleanup on failure (mirrors Phase 49 D-49-B2). This is `tough`'s local datastore for `latest_known_time.json` etc. — distinct from the published `trusted_root.json` cache the offline verify path reads.

### Test fixture transport (Area 3)

- **D-50-08:** Tests use an in-memory transport mock — `struct StaticMapTransport(HashMap<String, Vec<u8>>)` implementing `tough::Transport`. No localhost HTTP server. No port allocation. No CI flake surface from sockets. Trait is the same one production code uses, so the integration seam IS exercised.
- **D-50-09:** No real TLS handshake test is added in this phase. ureq+platform-verifier is treated as a trusted dependency. The HUMAN-UAT scenario on a real corp-network Windows host is the dispositive TLS-correctness check (matches the Req-6 acceptance criterion).
- **D-50-10:** Test count = 4 minimum per SPEC Req 5 acceptance: (1) happy-path walk from embedded v14 → fetched v15 → done returns byte-identical TrustedRoot output; (2) bad-signature on v15 surfaces as `NonoError::Setup` with TUF error message; (3) malformed-JSON on v15 surfaces as `NonoError::Setup`; (4) snapshot test asserts byte-identical cache output against a captured baseline.

### Cross-platform code path (Area 4)

- **D-50-11:** Single cross-platform code path. `refresh_trust_root_step` calls `refresh_production_trusted_root()` unconditionally on Linux + macOS + Windows. No `#[cfg(target_os = "windows")]` gate on the call site. ureq+platform-verifier consults the native trust store on every OS (Linux: OpenSSL/ca-certificates; macOS: Security framework; Windows: Crypt32).
- **D-50-12:** "Windows-only" in the SPEC text is interpreted as USER-IMPACT scope, not CODE-GATING scope. The user-impact contract is: "Windows corp-network refresh failure resolved". The code path that delivers it happens to be cross-platform, which is a bonus — Linux corp-CA users who hit the same bug later are auto-covered with zero additional code change. D-21 Windows-invariance is held by zero-behavior-regression on Linux/macOS (refresh still succeeds against tuf-repo-cdn.sigstore.dev), not by zero-file-diff.
- **D-50-13:** CI clippy lanes (per CLAUDE.md cross-target rule) MUST PASS on x86_64-pc-windows-msvc AND x86_64-unknown-linux-gnu AND x86_64-apple-darwin. Cross-target verification REQ is hard, not partial.

### Claude's Discretion

- Error type mapping (NonoError variant choice for tough errors) — researcher/planner picks. Suggested: wrap via `NonoError::Setup(format!("Sigstore TUF refresh failed: {e}"))` to match the existing `refresh_trust_root_step` error message shape.
- ureq Agent configuration knobs (timeout, retry, redirect policy) — planner picks reasonable defaults matching `nono`'s existing network conventions where they exist; otherwise standard `ureq::Agent::config_builder().http_status_as_error(false).build()` shape.
- Doc update granularity for `windows-poc-handoff.mdx` — planner decides whether to do an inline patch or a small rewrite. Either is fine; the acceptance criterion is "describes v0.53.x+ corp-network refresh works natively".

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### SPEC.md (locked requirements)

- `.planning/phases/50-corp-network-tuf-refresh-via-os-root-store-replace-or-wrap-t/50-SPEC.md` — Locked requirements, boundaries, acceptance criteria. MUST read before planning.

### In-tree code that the implementation must integrate with

- `crates/nono-cli/src/setup.rs:828-868` — `refresh_trust_root_step` (current implementation; only the single `TrustedRoot::production()` call body changes).
- `crates/nono-cli/src/setup.rs:888-919` — `from_file_step` (Phase 49; structural mirror for the new step shape — phase-index header, fail-closed cleanup pattern, freshness-gate placement).
- `crates/nono/src/trust/bundle.rs:113-167` — `load_trusted_root`, `load_production_trusted_root`, `check_trusted_root_freshness`. The cache contract these three functions read MUST stay unchanged.
- `crates/nono/src/trust/bundle.rs:32` — `pub use sigstore_verify::trust_root::TrustedRoot;` re-export. Type the new function returns.
- `crates/nono-cli/Cargo.toml:73` — `ureq = { version = "3", features = ["platform-verifier"] }`. Already present; no add needed.

### Upstream crates

- `~/.cargo/registry/src/.../tough-0.22.0/src/lib.rs:173-220` — `RepositoryLoader` builder + `.transport(impl Transport)` seam (line 212).
- `~/.cargo/registry/src/.../tough-0.22.0/src/transport.rs:46-90` — `tough::Transport` trait definition (`Debug + DynClone + Send + Sync` + sync/async fetch).
- `~/.cargo/registry/src/.../sigstore-trust-root-0.7.0/src/tuf.rs:60` — `pub const PRODUCTION_TUF_ROOT: &[u8]`. Direct source for the embedded v14 anchor.

### Project-level conventions

- `CLAUDE.md` § Coding Standards — cross-target clippy MUST/NEVER (`cargo clippy --workspace --target x86_64-unknown-linux-gnu` + `--target x86_64-apple-darwin` from dev host).
- `CLAUDE.md` § Security Considerations — fail-secure on errors, never silently degrade.
- `.planning/templates/cross-target-verify-checklist.md` — cross-target verification protocol Phase 50 must follow.
- `.planning/phases/49-sigstore-trust-root-poc-resilience-from-file-flag-release-as/49-SUMMARY.md` (if present after Phase 49 close) — `from_file_step` design + freshness-gate semantics this phase mirrors.

### Resolved debug sessions (evidence trail)

- `.planning/debug/resolved/sigstore-tuf-fetch-transport.md` — Full root-cause analysis: reqwest 0.12.28 + webpki-roots vs Windows root store. Evidence for surface (a) choice over (b).
- `.planning/debug/resolved/sigstore-trust-root-zero-sigs.md` — Predecessor session (0.6.6 embedded-anchor fix). Useful for understanding the TUF chain-walk verification order this phase preserves.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `ureq::Agent` builder with `platform-verifier` feature (Cargo.toml:73) — already a dep, no add. Construct once via `ureq::Agent::config_builder()...build()` and clone for the Transport impl.
- `sigstore_trust_root::PRODUCTION_TUF_ROOT` const — drop-in source for the embedded v14 anchor; no manual fixture maintenance.
- `serde_json::to_string_pretty` serialization at `setup.rs:857` — preserved verbatim to guarantee byte-identical cache output.
- `crate::config::nono_home_dir()` resolver in `crates/nono-cli/src/config/` — already used by both `refresh_trust_root_step` and `from_file_step`; reused for the TUF datastore directory.

### Established Patterns

- **Phase 49 step shape:** `from_file_step` at `setup.rs:888-919` — print `[X/N] <header>` first, build cache dir, do work, fail-closed cleanup, print success line. The new chain-walk preserves the same step-method skeleton but the WORK between the print and the write moves into `trust_refresh::refresh_production_trusted_root()`.
- **NonoError::Setup wrapping:** Existing pattern at `setup.rs:847-855` wraps upstream errors via `NonoError::Setup(format!("...: {e}"))`. The new module follows the same convention.
- **No `unwrap()`:** Strict `clippy::unwrap_used` deny applies; all error paths use `?` or explicit `.map_err`.
- **`#[cfg(test)] mod tests {}`** colocated with the module — standard nono test placement.
- **Phase 49 D-49-B2 best-effort cleanup:** On any error after creating the TUF datastore directory, best-effort remove partial state (matches the from-file step's pattern of not leaving partial files on disk).

### Integration Points

- `crates/nono-cli/src/setup.rs::refresh_trust_root_step` — single call-site change from `nono::trust::TrustedRoot::production().await` (inside tokio runtime) to `trust_refresh::refresh_production_trusted_root()` (sync — `tough` + `ureq` are sync, no tokio runtime needed). The `tokio::runtime::Builder::new_current_thread()` block at `setup.rs:844-848` is REMOVED — Phase 50 ELIMINATES the only async call in `refresh_trust_root_step`, which simplifies the function meaningfully.
- `crates/nono-cli/src/lib.rs` (or `main.rs` module decls) — add `mod trust_refresh;` declaration.
- `crates/nono-cli/Cargo.toml` — add `tough = "0.22"` to `[dependencies]` (promoting transitive → direct). Possibly also `dyn-clone` if `tough`'s Transport trait requires explicit derive setup — researcher confirms.
- No diff in `crates/nono/` (library) — D-50-01 keeps `crates/nono` HTTP-free; the chain walk lives entirely in `nono-cli`.

</code_context>

<specifics>
## Specific Ideas

- The user explicitly confirmed (round-1 lock) Surface (a) — nono-local TUF walk — is preferred over an upstream sigstore-rs PR. Treat this as locked; do NOT research surface (b) further.
- Phase 49's `--from-file` flag remains a deliberate backstop. The new corp-network path makes it less necessary as a workaround, but it stays available for fully-offline hosts. The doc update at `windows-poc-handoff.mdx` REFRAMES `--from-file` as "for offline hosts" instead of "for corp-network failures".
- The TUF datastore directory (`<nono_home>/.nono/trust-root/tuf-cache/`) is a NEW on-disk artifact this phase introduces. It is `tough`'s local working directory — distinct from the existing `trusted_root.json` cache. Planner notes: this directory may grow with `*.root.json` files; cleanup policy is "let tough manage it; do not externally rotate".

</specifics>

<deferred>
## Deferred Ideas

- **Upstream sigstore-rs PR (Surface (b)):** Adding `TrustedRoot::with_http_client(...)` seam to sigstore-rs so other consumers can pass a native-tls client. Deferred — this phase's surface (a) decision moots it for nono, but it's a community-good contribution that could be opened independently of nono's release cadence. Not on the v2.6 critical path.
- **Online freshness probe with same HTTP client:** Phase 32 D-32-03 documents an unimplemented online freshness-probe path that would also need the OS-root-store HTTP client. Out of scope for Phase 50 (only `--refresh-trust-root` per SPEC). When that probe is implemented (likely v2.7+), it should reuse `trust_refresh::*` helpers from this phase.
- **CI MITM proxy test rig:** Building a CI runner with an installed test CA + MITM proxy to exercise the corp-network code path automatically. Out of scope — round-2 chose Unit + HUMAN-UAT instead.
- **Linux corp-CA UX docs:** If a Linux user hits the analogous bug, document the resolution. Auto-covered by D-50-11 single-path choice; no doc work required preemptively.

</deferred>

---

*Phase: 50-corp-network-tuf-refresh-via-os-root-store-replace-or-wrap-t*
*Context gathered: 2026-05-21*
