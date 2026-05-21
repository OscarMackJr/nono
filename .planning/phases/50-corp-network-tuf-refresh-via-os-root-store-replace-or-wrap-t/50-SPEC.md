# Phase 50: Corp-network TUF refresh via OS root store — Specification

**Created:** 2026-05-21
**Ambiguity score:** 0.17
**Requirements:** 6 locked

## Goal

`nono setup --refresh-trust-root` succeeds on a Windows host behind a TLS-inspecting corporate proxy (whose interceptor CA is in the Windows root store but not in the Mozilla `webpki-roots` bundle), by replacing the single sigstore-rs `TrustedRoot::production()` call with a nono-local TUF chain-walk that uses an HTTP client consulting the Windows certificate store.

## Background

`crates/nono-cli/src/setup.rs:828-868` (`refresh_trust_root_step`) is currently a single call into `nono::trust::TrustedRoot::production()` (re-exported from `sigstore_verify::trust_root::TrustedRoot::production`, see `crates/nono/src/trust/bundle.rs:32`). That call transitively pulls `sigstore-trust-root 0.7.0` → `tough 0.22.0` → `reqwest 0.12.28`, which is compiled with `hyper-rustls 0.27.9` + `webpki-roots` and does **not** consult the Windows certificate store. The same lockfile pins `reqwest 0.13.3` with `rustls-platform-verifier 0.7.0` for unrelated paths, isolating the trust-store mismatch to the sigstore-rs 0.12 reqwest chain — confirmed by `grep -rn 'reqwest' crates/` returning zero in-tree matches (nono builds zero custom HTTP clients; trust behavior is 100% upstream-determined).

The triggering failure is documented in `.planning/debug/resolved/sigstore-tuf-fetch-transport.md`: after Phase 49's binary rebuild made the embedded v14 anchor self-verify cleanly, the next TUF step (fetch `15.root.json` from `tuf-repo-cdn.sigstore.dev`) fails with `Transport 'other' error ... error sending request for url` on the same POC user's corporate network. PowerShell's `Invoke-WebRequest` (SChannel-backed) succeeds on the same URL from the same shell — narrowing the failure to nono's HTTP client trust-store configuration.

Phase 49's `from_file_step` (`crates/nono-cli/src/setup.rs:888-919`) is the validated operational escape hatch and covers the user-side gap today. The cache file `<nono_home>/.nono/trust-root/trusted_root.json` is read offline-only by `load_production_trusted_root` (`crates/nono/src/trust/bundle.rs:147-167`) for `nono trust verify` and related flows, so any new write path must produce a byte-identical cache file.

Required deps are already present:
- `ureq = { version = "3", features = ["platform-verifier"] }` at `crates/nono-cli/Cargo.toml:73` — HTTP transport that honors the Windows certificate store.
- `tough 0.22.0` at `Cargo.lock` (transitive via sigstore-rs) — TUF chain-walk + signature-threshold verification.

## Requirements

1. **Nono-local TUF chain-walk replaces upstream call.** The refresh-trust-root code path must perform the TUF root-chain walk locally instead of delegating to sigstore-rs.
   - Current: `refresh_trust_root_step` calls `nono::trust::TrustedRoot::production()` (single line; goes through sigstore-rs's reqwest+webpki-roots client).
   - Target: `refresh_trust_root_step` invokes a new nono-internal function that performs the TUF chain walk using the `tough` crate (already in `Cargo.lock`) for verification + a `ureq` v3 client with the `platform-verifier` feature for HTTP transport. The new function produces the same `TrustedRoot` value the upstream call would have produced.
   - Acceptance: `grep -rn 'TrustedRoot::production()' crates/nono-cli/src/` returns zero matches after this phase lands; the new chain-walk function exists and is invoked exactly once from `refresh_trust_root_step`.

2. **HTTP client consults the Windows certificate store.** The new TUF chain-walk's HTTP transport must trust enterprise CAs deployed to the Windows root store via GPO/MDM.
   - Current: reqwest 0.12.28 pulled by sigstore-rs uses bundled Mozilla `webpki-roots`; cannot see enterprise CAs.
   - Target: HTTP requests in the new chain-walk use `ureq` v3 with `platform-verifier` enabled, which on Windows delegates to `rustls-platform-verifier` → Crypt32 native trust validation against `HKLM\SOFTWARE\Microsoft\SystemCertificates\ROOT`.
   - Acceptance: Code review confirms the new HTTP client is constructed via `ureq::Agent` with the `platform-verifier` feature path. No `reqwest::Client` is built in the new code. Unit test asserts construction does not panic and the agent honors a custom-CA test setup (see Req 5).

3. **TUF verification correctness preserved.** The new chain-walk performs the same signature-threshold-of-N verification at every step that sigstore-rs's call did.
   - Current: sigstore-trust-root 0.7.0 + tough 0.22.0 do full TUF spec compliance — step 5.2 embedded-anchor self-verify + step 5.3 chain walk where each `N+1.root.json` is verified against the prior root's keys + threshold check before being trusted.
   - Target: nono's chain-walk uses `tough::RepositoryLoader` (or equivalent `tough` v0.22 API) so the verification logic is identical; nono provides only the HTTP transport, never the verification math.
   - Acceptance: Unit test exercises a hermetic localhost TUF repo where v15.root.json has an intentionally invalid signature against v14's keys, and confirms the new chain-walk rejects it with a `NonoError::Setup` containing a TUF-verification error message. Hand-rolled signature verification is forbidden in the diff (grep guard: no new code in `crates/nono*/src/**` calls `verify_role` outside the `tough` crate path).

4. **Cache file byte-identical to `TrustedRoot::production()` output.** The persisted `trusted_root.json` must be loadable by the existing offline path without any code changes there.
   - Current: `refresh_trust_root_step` at `setup.rs:857-860` writes `serde_json::to_string_pretty(&trusted_root)` to `<nono_home>/.nono/trust-root/trusted_root.json`.
   - Target: The new chain-walk produces a `sigstore_verify::trust_root::TrustedRoot` Rust value (same type sigstore-rs returns), serialized via the exact same `serde_json::to_string_pretty` call, written to the same cache path. The function `load_production_trusted_root` in `crates/nono/src/trust/bundle.rs:147` continues to read it unchanged.
   - Acceptance: Snapshot test compares the new chain-walk's output bytes against a captured baseline produced by the upstream `TrustedRoot::production()` call (against the same hermetic TUF repo); byte-identical match required. `TrustedRoot::from_file(&cache_path)` succeeds on the new output.

5. **Hermetic unit-test coverage for the new chain-walk.** The new code path must have unit tests that don't require a real corp proxy.
   - Current: No tests cover the refresh-trust-root path; the existing test surface is limited to the offline `load_production_trusted_root` reader.
   - Target: A new unit/integration test module exercises the chain-walk against a localhost test fixture that serves canned TUF metadata (`14.root.json`, `15.root.json`, etc.) over plain HTTP — TLS-trust correctness is asserted by the `platform-verifier` feature flag being present, not by a live MITM. Test count ≥ 4 covering: (a) successful happy-path walk from v14 → v15 → done; (b) failed signature verification at chain step 5.3 surfaces as `NonoError::Setup`; (c) malformed JSON in fetched root surfaces as `NonoError::Setup`; (d) byte-identical cache output snapshot.
   - Acceptance: `cargo test -p nono-cli refresh_trust_root::tests` PASS with ≥ 4 test functions in the new module. CI green on Windows + Linux + macOS targets (the hermetic tests run on all platforms; the platform-verifier behavioral distinction is a runtime trust-store concern, not a compile-time test concern).

6. **HUMAN-UAT scenario for corp-network proof.** One human-UAT scenario captures the real corp-network success criterion.
   - Current: No human-UAT artifact exists for Phase 50.
   - Target: `.planning/phases/50-.../50-HUMAN-UAT.md` contains ONE scenario: "Run `nono setup --refresh-trust-root` on a Windows host behind a TLS-inspecting corporate proxy whose enterprise CA is in the Windows root store. Expected: step [3/5] exits 0 and writes `trusted_root.json` to the cache. Expected stderr: zero transport errors." Pass criterion is binary; the UAT result lands in the phase's VERIFICATION.md.
   - Acceptance: HUMAN-UAT file exists, contains the single scenario with explicit expected output, and one user run on a corp-network host produces a pass entry in VERIFICATION.md before the phase closes.

## Boundaries

**In scope:**
- Replacing the body of `refresh_trust_root_step` with a new nono-local TUF chain-walk function
- Wiring `tough 0.22.0` + `ureq 3 (platform-verifier)` as the verification + transport stack
- Producing a byte-identical `trusted_root.json` cache file
- 4 hermetic unit/integration tests for the new chain-walk
- 1 HUMAN-UAT scenario for the corp-network proof on Windows
- Documentation update at `docs/cli/development/windows-poc-handoff.mdx` noting that v0.53.x+ corp-network refresh works natively (no `--from-file` required)

**Out of scope:**
- Any sigstore-rs HTTP egress other than `--refresh-trust-root` (Rekor inclusion-proof fetches, Fulcio signing, OIDC, freshness-online-probes) — Phase 32 D-32-01 keeps `nono trust verify` offline; this phase scope is "Only `--refresh-trust-root`" per round-1 user lock.
- Linux/macOS behavior changes — round-1 user lock is "Windows-only". `tough` + `ureq+platform-verifier` happen to be cross-platform, but Linux/macOS file diff must be zero (D-21 Windows-invariance). Linux/macOS behavior must not regress; an empty file-diff on those targets is the contractual guarantee, not feature parity.
- Removing or modifying the `--from-file` flag — Phase 49's escape hatch stays in place as a backstop for users without network access at all.
- A live CI integration test with a real corp-network MITM proxy — round-2 user lock chose "Unit + HUMAN-UAT scenario" over "Mock corp-network in CI".
- Upstream `sigstore-rs` PR adding a `TrustedRoot::with_http_client(...)` seam (Surface (b)) — round-1 user lock chose Surface (a) (nono-local TUF walk) instead. Out of scope by elimination.
- Hand-rolled TUF signature verification — Req 3 explicitly forbids this; `tough` does the math.

## Constraints

- **D-21 Windows-invariance**: Linux/macOS source files must have zero diff outside Cargo.toml/lock and tests. The refresh path's new function is `#[cfg(target_os = "windows")]`-gated on the Windows-active behavior; on Linux/macOS the existing `TrustedRoot::production()` call MAY remain in place OR the new code path MAY be used (cross-platform-safe), but no behavioral regression on Linux/macOS is permitted.
- **Cache contract preservation**: `<nono_home>/.nono/trust-root/trusted_root.json` byte format must remain consumable by `load_production_trusted_root` (`crates/nono/src/trust/bundle.rs:147`) with no diff to that function or the freshness-check path.
- **No new transitive sigstore deps**: The new code must not pull additional sigstore-rs crates beyond what's already in `Cargo.lock`. `tough 0.22.0` and `ureq 3` are already there; `rustls-platform-verifier 0.7.0` is already there (via reqwest 0.13.3). No new top-level Cargo dependency adds are required.
- **TUF security**: Hand-rolled signature verification is forbidden. All signature math must flow through `tough`'s public APIs.
- **CLAUDE.md unwrap policy**: Strict `#![deny(clippy::unwrap_used)]` continues to apply; new code uses `?` propagation.

## Acceptance Criteria

- [ ] `grep -rn 'TrustedRoot::production()' crates/nono-cli/src/` returns zero matches
- [ ] New chain-walk function exists in `crates/nono-cli/src/` and is invoked exactly once from `refresh_trust_root_step`
- [ ] HTTP transport in the new code is `ureq::Agent` with `platform-verifier`; no `reqwest::Client::builder()` calls in the diff
- [ ] All TUF signature verification routes through `tough` (no hand-rolled `verify_role` in diff)
- [ ] Snapshot test confirms byte-identical cache output vs. `TrustedRoot::production()` baseline against a hermetic test TUF repo
- [ ] ≥ 4 unit/integration tests covering happy-path + signature-rejection + malformed-JSON + byte-identical-snapshot; all PASS on Windows, Linux, macOS CI lanes
- [ ] `TrustedRoot::from_file(&cache_path)` succeeds on the new cache file (Phase 32 D-32-01 offline-verify path unaffected)
- [ ] `50-HUMAN-UAT.md` contains the corp-network scenario with explicit expected output
- [ ] One human-UAT run on a Windows corp-network host produces a pass entry in `50-VERIFICATION.md` before phase closes
- [ ] Zero file diff under `crates/**/*.rs` on Linux/macOS-only paths (D-21 Windows-invariance: cross-target diff equals empty for non-Windows sources)
- [ ] `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` PASS on x86_64-pc-windows-msvc AND x86_64-unknown-linux-gnu AND x86_64-apple-darwin (per CLAUDE.md cross-target rule)
- [ ] `docs/cli/development/windows-poc-handoff.mdx` updated to note native corp-network success in v0.53.x+; `--from-file` reframed as "for hosts with no outbound network" rather than "for corp-network failures"

## Ambiguity Report

| Dimension          | Score | Min  | Status | Notes                                                                                |
|--------------------|-------|------|--------|--------------------------------------------------------------------------------------|
| Goal Clarity       | 0.85  | 0.75 | ✓      | One specific call replaced; outcome ("corp-network refresh succeeds") is measurable. |
| Boundary Clarity   | 0.80  | 0.70 | ✓      | Round-1 locks: refresh-only, surface (a), Windows-only. Out-of-scope list explicit.  |
| Constraint Clarity | 0.85  | 0.65 | ✓      | D-21 invariance + cache contract + no-hand-rolled-TUF + dep-set frozen.              |
| Acceptance Criteria| 0.80  | 0.70 | ✓      | 11 binary checkboxes; snapshot + cross-target clippy + HUMAN-UAT all explicit.       |
| **Ambiguity**      | 0.17  | ≤0.20| ✓      | Two rounds; surface chosen + verification path chosen + cache contract locked.       |

Status: ✓ = met minimum, ⚠ = below minimum (planner treats as assumption)

## Interview Log

| Round | Perspective                  | Question summary                                | Decision locked                                                                          |
|-------|------------------------------|-------------------------------------------------|------------------------------------------------------------------------------------------|
| 1     | Researcher                   | Scope of network calls to fix?                  | Only `--refresh-trust-root` (Phase 32 D-32-01 keeps verify offline)                      |
| 1     | Researcher                   | Implementation surface — (a) local / (b) upstream PR / defer? | Lock surface (a): nono-local TUF walk with `ureq + platform-verifier`             |
| 1     | Researcher                   | OS coverage — Windows-only or cross-platform?   | Windows-only (D-21 invariance; cross-target diff guards Linux/macOS files)               |
| 2     | Boundary Keeper              | TUF security correctness — `tough` or hand-roll? | Reuse `tough 0.22.0` (already in Cargo.lock); nono provides only HTTP transport          |
| 2     | Boundary Keeper              | Cache format compatibility?                     | Byte-identical to current `TrustedRoot::production()` serialization (snapshot test gate) |
| 2     | Failure Analyst              | CI vs HUMAN-UAT for corp-network proof?         | Unit (hermetic) + ONE HUMAN-UAT scenario (no live MITM in CI)                            |

---

*Phase: 50-corp-network-tuf-refresh-via-os-root-store-replace-or-wrap-t*
*Spec created: 2026-05-21*
*Next step: /gsd-discuss-phase 50 — implementation decisions (tough API surface, module placement, test fixture shape, etc.)*
