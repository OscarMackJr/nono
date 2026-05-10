# Broker.exe Authenticode Self-Trust-Anchor

**Status:** Accepted
**Date:** 2026-05-10
**Phase:** 32 (v2.3 Sigstore Integration & Broker Trust)
**Decision IDs:** D-32-11, D-32-12, D-32-13, D-32-14
**Related ADR:** [Sigstore TUF Root Cache + Verify-Is-Offline Invariant](sigstore-tuf-cache.md) (the other Phase 32 deliverable; documents where the Sigstore primitives that this ADR's threat model contrasts against come from)

## Context

Phase 31 introduced the Windows broker dispatch arm (`WindowsTokenArm::BrokerLaunch` in `exec_strategy_windows/launch.rs:1246-1438`) so `nono.exe` can spawn `nono-shell-broker.exe` as a Low-Integrity-Level child for Job Object containment. Phase 31 deliberately landed the dispatch path WITHOUT signature verification on the broker binary — the Phase 31 plan called the verification gate out as a Phase 32 follow-up so the broker code path could ship while the trust-anchor question was decided separately.

Without a verification gate, an attacker who can write to the install directory (already admin on a typical Windows machine) can replace `nono-shell-broker.exe` with a hostile binary. The hostile broker would inherit `nono.exe`'s privilege boundary at the moment of spawn, defeating the supervisor-led security model: `nono.exe` is not the sandbox; the broker IS the sandbox enforcement point on Windows. A compromised broker accepts Low-IL handles from `nono.exe` and can do anything `CreateProcessAsUserW` lets it do with the parent's authority.

Three trust-anchor options were considered before Phase 32 wave 0:

- **(a) Baked-in expected publisher CN constant** — hardcode the project's release-pipeline subject (e.g., `"Always Further LLC"`) as a Rust constant; verify the broker's certificate chain matches that constant at every dispatch.
- **(b) Config-file trust anchor** — ship a `trust-anchors.json` next to the binary listing accepted publisher subjects; verify against the config at every dispatch.
- **(c) Sigstore bundle alongside the binary** — package a `nono-shell-broker.exe.sigstore` bundle in the MSI; verify via `sigstore-verify` at every dispatch.

All three have a chicken-and-egg problem: the trust source itself becomes a target. Option (a) ships in the MSI alongside `nono.exe`, so any tamper that swaps `nono-shell-broker.exe` can also patch the constant in `nono.exe`. Option (b) is even more obvious: the config IS the trust source, so the attacker rewrites it. Option (c) shifts the problem one level — the Sigstore bundle has to be verified against a TUF root that lives somewhere; whatever pinned root we bake in becomes the new attack surface, and the existing Phase 32 cached-root design (see [sigstore-tuf-cache.md](sigstore-tuf-cache.md)) is geared at end-user verify, not internal binary trust.

### Goals

- Verify `nono-shell-broker.exe`'s Authenticode signature on every broker dispatch (no cache).
- Fail-closed on verify failure: no spawn, no escape-hatch flag, no env-var override.
- No new FFI surface; reuse Phase 28's `query_authenticode_status` chain-walker (`crates/nono-cli/src/exec_strategy_windows/...`) unchanged.
- Survive `cargo test --release` cleanly (Pitfall 6: `#[cfg(debug_assertions)]` would falsely trigger production-mode strict checks against an unsigned dev broker).
- Preserve the supervisor-led security model: trust decisions live in `nono.exe`, not in config files or broker-side state.

### Non-goals

- This ADR explicitly does NOT commit to:
  - Verifying signatures on every binary `nono.exe` invokes (only the broker, because only the broker inherits the Low-IL token boundary).
  - Caching the verification result. D-32-14 makes the check unconditional on every dispatch — caching would create a TOCTOU window between cache write and broker swap.
  - Cross-platform extension. Linux and macOS have different supervisor models (Phase 25 aIPC; Plan 22 audit attestation respectively); this ADR is Windows-only.
  - Migrating to Sigstore-verified broker bundles. Out of scope per D-32-13 self-bootstrapping rationale.

## Decision Table

| Option | Trust Source | Bootstrap | Verdict |
|--------|-------------|-----------|---------|
| (a) Baked-in publisher CN constant | Rust source string | Compiled into `nono.exe` | Rejected: tamper of `nono.exe` patches the constant; ships+ages with each release. |
| (b) Config-file trust anchor | `trust-anchors.json` next to binary | Loaded at dispatch | Rejected: config IS the trust source; tamper rewrites it. |
| (c) Sigstore bundle alongside binary | `nono-shell-broker.exe.sigstore` | Verified via TUF root | Rejected: shifts the problem one level. The pinned TUF root becomes the new attack surface; uses Phase 32's user-verify infrastructure for an internal trust decision. |
| **(d, chosen) Self-introspection self-trust-anchor — `nono.exe` extracts ITS OWN Authenticode signature, requires broker to match** | Whoever signed `nono.exe` | OS Authenticode chain-of-trust on the running process | **Accepted** |

Option (d) is self-bootstrapping: if `nono.exe` is running on Windows, its Authenticode signature was already trusted by the OS via CodeIntegrity at load time. We don't need to bake in WHO signed `nono.exe` — we read it from the live process. The broker must match that same identity (subject AND thumbprint).

## Decision

The self-trust-anchor decision: at every entry into the `WindowsTokenArm::BrokerLaunch` arm of `exec_strategy_windows::launch::run`, after the broker-not-found check and BEFORE handle-inheritance setup, `nono.exe` invokes the new gate `verify_broker_authenticode(nono_exe, broker_path)`:

1. Resolve `nono_exe` via `std::env::current_exe()` and canonicalize.
2. Call Phase 28's `query_authenticode_status(nono_exe)` to extract the running process's `(subject, thumbprint)` pair.
3. Call `query_authenticode_status(broker_path)` against the broker.
4. Both must return `Valid`. If either is `Unsigned`, `Tampered`, `Revoked`, `ChainNotTrusted`, or any other non-`Valid` status, return `NonoError::TrustVerification` with a message naming the binary and the failure mode.
5. The `(subject, thumbprint)` pairs MUST match exactly. Subject is compared byte-equal (no normalization, no canonicalization — Authenticode's subject is already the canonical form). Thumbprint is hex-equal (case-insensitive comparison; the underlying bytes are case-irrelevant).
6. On match: emit a `tracing::debug!(target: "broker_authenticode", subject, thumbprint, "verified")` line and proceed to spawn.
7. On mismatch or any failure: bubble `NonoError::TrustVerification` up; the broker is NOT spawned.

There is NO escape-hatch flag, NO env-var override. D-32-12 is explicit: fail-closed without a bypass.

### Skip Mechanism (dev-build only)

The dev-build broker is unsigned; without a skip, every `cargo run`-style local invocation of `nono shell` would fail-closed. The skip is a runtime install-layout substring detector applied to `current_exe()`'s path:

- The path contains `\target\debug\` OR `\target\release\` OR `/target/debug/` OR `/target/release/` → dev layout, skip the gate (tracing-info that we did so).
- Otherwise → production layout, run the gate unconditionally.

We deliberately do NOT use `#[cfg(debug_assertions)]`. `cargo test --release` compiles WITHOUT `debug_assertions`, so a `#[cfg(debug_assertions)]` gate would fail-closed against the unsigned dev broker under release-mode test runs — a false positive that masks real strict-mode regressions. Pitfall 6 from `32-RESEARCH.md` covers this in detail.

The detector is `is_dev_build_layout(path: &Path) -> bool` in `exec_strategy_windows/launch.rs`. Test coverage: `broker_authenticode_layout_tests::is_dev_build_layout_detection` exercises 8 boundary cases including `target/debug` substring matches inside non-dev paths (rejected) and absolute production paths under `Program Files` (rejected).

### Diagnostic Surface

`nono setup --check-only` reports `nono.exe`'s self-Authenticode subject + thumbprint as two lines in its summary, regardless of whether `nono.exe` is signed (P32-CHK-003). This gives operators a way to confirm what trust anchor the broker gate is using before they hit a fail-closed at runtime. If the lines show `unsigned` for a release build, the operator knows the gate would fail-closed at the first broker dispatch.

## Consequences

### Positive

- **No baked-in constants.** The trust anchor is whatever signs `nono.exe`. Publisher-cert rotation handled naturally: both binaries roll together via the Phase 31 Plan 04 release pipeline (one signing call per artifact, same identity).
- **Self-bootstrapping.** No chicken-and-egg: if `nono.exe` is running on Windows, the OS already trusts its signature via CodeIntegrity. We simply read what's there.
- **Reuses Phase 28 chain-walker primitives unchanged.** REQ-AUDC invariant preserved — Phase 28's `query_authenticode_status`, `parse_signer_subject`, `parse_thumbprint` are byte-identical pre/post Phase 32.
- **~30 lines of glue at the dispatch site.** No new FFI; small surface area; easy to audit.
- **Diagnostic visibility.** `nono setup --check-only` surfaces the live trust anchor; tracing-debug on every successful gate; tracing-info on every dev-skip activation.
- **Naturally extends to other supervisor-spawned binaries** if Phase 33+ adds them. The pattern is "same signer as me," reusable for any trusted child.

### Negative

- **A compromised release pipeline that signs both binaries with the same hostile cert defeats this.** Acceptable: the threat model (CONTEXT.md § Threat Boundaries) accepts that a compromised release pipeline is out of scope for the dispatch gate. The gate prevents post-release tamper, not signing-key compromise. Defense-in-depth is the Sigstore pipeline (Phase 32 Plan 03 keyless signing) for end-user release artifact verification.
- **TOCTOU window between `query_authenticode_status(broker)` and `CreateProcessW(broker)`.** Acceptable: writing to the install directory between the check and the spawn requires admin-equivalent access; an attacker with that already controls the system.
- **Dev-build broker is unsigned, requiring a skip mechanism.** Mitigated by the install-layout detector documented above; test-covered.
- **Verification cost on every broker spawn.** Acceptable: `query_authenticode_status` is a few WinTrust API calls; broker spawns are not in the hot path; D-32-14 explicitly forbids caching to prevent TOCTOU.
- **Self-introspection via `current_exe()` can return symlinks on some Windows configurations.** The implementation canonicalizes via `std::fs::canonicalize` before passing to WinTrust. The broker path is canonicalized identically.

## References

### Internal

- `.planning/phases/32-sigstore-integration/32-CONTEXT.md` (D-32-11 Authenticode mechanism choice; D-32-12 fail-closed no-escape-hatch; D-32-13 self-trust-anchor; D-32-14 verify-on-every-dispatch)
- `.planning/phases/32-sigstore-integration/32-RESEARCH.md` (Pattern 5 chain-walker reuse; Pitfall 5 TOCTOU acceptance; Pitfall 6 dev-build skip mechanism rationale)
- `.planning/phases/32-sigstore-integration/32-04-SUMMARY.md` (implementation: gate insertion, `is_dev_build_layout` boundary tests, tempdir staging strategy for fail-closed test paths)
- `.planning/phases/31-broker-process-architecture-shell-01/31-04-SUMMARY.md` (Phase 31 release pipeline contract: both binaries signed by same identity in `scripts/sign-windows-artifacts.ps1`)

### Source code

- `crates/nono-cli/src/exec_strategy_windows/launch.rs` § `WindowsTokenArm::BrokerLaunch` (line range 1246-1438) — dispatch arm with the Phase 32 gate insertion at the broker-found-but-not-yet-spawned point.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs::verify_broker_authenticode` — `pub(crate)` seam exposed for integration tests (`broker_authenticode.rs`); not part of the public library API.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs::is_dev_build_layout` — install-layout substring detector; pure function with `broker_authenticode_layout_tests` covering boundary cases.
- `crates/nono-cli/src/setup.rs::print_self_authenticode_status` — `--check-only` diagnostic line.
- `crates/nono-cli/tests/broker_authenticode.rs` — 6-test integration suite covering self-extraction, valid match, mismatch, unsigned-release, dev-skip-does-not-bypass-release, every-dispatch-revalidates.
- Phase 28 chain-walker (search `query_authenticode_status`, `parse_signer_subject`, `parse_thumbprint`) — reused unchanged.

### Related ADRs

- [Sigstore TUF Root Cache + Verify-Is-Offline Invariant](sigstore-tuf-cache.md) — the Phase 32 ADR for the OTHER novel decision (cached-root design for end-user keyless verify). The two are parallel: the TUF cache governs `nono trust verify` against external Sigstore-signed artifacts; this ADR governs `nono.exe`'s internal trust of its own broker child. They share Phase 28's Authenticode primitives but otherwise operate independently.
- `docs/architecture/audit-bundle-target.md` (Phase 27.2 ADR convention reference; this ADR mirrors its structure).
