# Sigstore TUF Root Cache + Verify-Is-Offline Invariant

**Status:** Accepted
**Date:** 2026-05-10
**Phase:** 32 (v2.3 Sigstore Integration & Broker Trust)
**Decision IDs:** D-32-01, D-32-02, D-32-03, D-32-04, D-32-05, D-32-06, D-32-15
**Related ADR:** [Broker.exe Authenticode Self-Trust-Anchor](broker-trust-anchor.md) (the other Phase 32 deliverable; documents the Windows broker-binary trust gate that runs alongside the keyless verify path documented here)

## Context

`crates/nono/src/trust/bundle.rs::load_production_trusted_root` was originally `pub async fn` calling `sigstore_verify::trust_root::TrustedRoot::production().await`, which fetches and TUF-verifies the live Sigstore root metadata from `https://tuf-repo-cdn.sigstore.dev` on every invocation. Three problems surfaced together during quick task `260509-s9m`:

1. **The `production()` call fails on `sigstore-verify 0.6.5`.** The pinned bundled root metadata cannot meet the threshold-of-3 signature check against the current published TUF root: `Tuf("TUF repository load failed: Failed to verify trusted root metadata: Signature threshold of 3 not met for role root (0 valid signatures)")`. This blocks every keyless verify code path on Windows (and elsewhere), since both the original 2 unit tests at `bundle.rs:877` and `bundle.rs:914` were calling `production()` and both were failing.
2. **Every keyless `nono trust verify` becomes a Sigstore-uptime dependency AND a privacy leak.** Inline TUF on every verify means every verify phones home (timing + IP exposure) and every verify breaks if `tuf-repo-cdn.sigstore.dev` is down or rate-limiting.
3. **The verify call path was async, so the 6 caller sites in `nono-cli/` were each wrapping it in `rt.block_on(...)`** to bridge sync CLI command runners. That's a Tokio runtime per verify per command — wasteful and a source of nested-runtime panics if any caller is itself inside a runtime.

The fix had to land all three at once: cache the verified TUF root somewhere durable, drop the inline network call from the verify path, and rip out the async wrapper at every caller. A naive "just call `production()` once at process start and cache in memory" doesn't work because the call still fails — we need a captured-once-good fixture for tests AND a per-user cache that the user explicitly hydrates with `nono setup --refresh-trust-root`.

### Goals

- Move the network call out of the verify path. Verify NEVER does inline network — the **verify-is-offline invariant**.
- Cache the verified `TrustedRoot` JSON at a per-user durable path, refreshed via an explicit operator action (`nono setup --refresh-trust-root`).
- Fail-closed when the cache is missing OR expired, with a recovery hint that names the exact command (D-32-03 / D-32-05).
- Hermetic test coverage via a frozen TUF root fixture (D-32-02 / D-32-06 — pinned indefinitely; no rotation job).
- Honor `NONO_TEST_HOME` (Phase 27.1 D-27.1-08 test seam).
- No new dependencies in `crates/nono` (D-32-15 / D-19 invariant; specifically: NO `dirs` crate — home directory resolved via `std::env`).

### Non-goals

- This ADR explicitly does NOT commit to:
  - Periodic auto-refresh of the cache. The user runs `nono setup --refresh-trust-root` manually; D-32-04 keeps `sigstore-verify` and `sigstore-sign` pinned at 0.6.5 with no version bump in this phase.
  - Migrating release-artifact signing to keyless. Out of scope per D-32-10; recorded as a v2.4+ deferred item by Plan 32-05.
  - Rotating the frozen test fixture on a CI schedule. D-32-06 is explicit: pinned indefinitely; the fixture's purpose is to provide a syntactically-valid root for hermetic tests, not to track upstream Sigstore drift.
  - Touching `crates/nono`'s dependency surface beyond the documented D-19 deviations enumerated in D-32-15.

## Decision Table

| Option | Verify Path | Refresh Mechanism | Verdict |
|--------|-------------|-------------------|---------|
| **A (chosen) — Cache + sync verify, explicit refresh** | Sync read of `<nono_home_dir()>/.nono/trust-root/trusted_root.json` | `nono setup --refresh-trust-root` | **Accepted** |
| B — Inline network on every verify | `TrustedRoot::production().await` | None | Rejected: status quo. Currently broken (signature threshold), Sigstore-uptime dependency, privacy leak. |
| C — Bake the TUF root into the binary | Const `&'static [u8]` from `include_bytes!` | New release per Sigstore root rotation | Rejected: Sigstore TUF root rotates more frequently than `nono` releases; baking it in turns every Sigstore key rotation into a `nono` release event. |
| D — Auto-refresh on a timer / first-verify-of-the-day | Cache read, but trigger `production()` if cache > N hours old | Background refresh | Rejected: re-introduces an implicit network dependency on the verify path; D-32-03 explicit "verify NEVER does inline network." |

Option A finishes the cache-and-explicit-refresh design that the existing `bundle.rs::load_trusted_root` (sync, takes `Path`) was already half-building. The Phase 32 work is to make `load_production_trusted_root` reuse that same sync path against the cached file.

## Decision

`crates/nono/src/trust/bundle.rs::load_production_trusted_root` is rewritten as a sync function that reads from `<nono_home_dir()>/.nono/trust-root/trusted_root.json`:

1. **Signature change (D-32-15 enumerated D-19 deviation #1):** `pub async fn load_production_trusted_root() -> Result<TrustedRoot>` becomes `pub fn load_production_trusted_root() -> Result<TrustedRoot>`. The function's doc-comment explicitly names the upstream-divergence per Pitfall 4. `tests/integration/test_upstream_drift.sh:257` carries the `# intentional fork: Phase 32 D-32-01` annotation so future drift-audit runs don't flag it.
2. **Cache path resolution:** new helper `home_dir_from_env()` resolves the home directory using `std::env` only (Windows: `USERPROFILE` then `HOMEDRIVE`+`HOMEPATH`; Unix: `HOME`). NO `dirs` crate is added to `crates/nono`'s `Cargo.toml` — D-32-15 / P32-CHK-002 enforces this.
3. **First-run UX (D-32-05):** missing cache file → `NonoError::TrustPolicy` with text containing both `Sigstore trusted root not initialized` AND `nono setup --refresh-trust-root (requires network)`. The `--check-only` summary line surfaces `Trust root cache: NOT INITIALIZED` for the same situation.
4. **Expiry detection (D-32-03):** the cache file's tlogs are inspected for `valid_for.end`. The file is considered expired when no active tlog has `end > now`. On expiry → `NonoError::TrustVerification` with text containing both `expired` and `nono setup --refresh-trust-root`. The `--check-only` summary line surfaces `Trust root cache: STALE` and includes the recovery hint literally (P32-CHK-012 acceptance criterion).
5. **Date comparison strategy (D-19 invariant — no `chrono` dep):** Howard Hinnant's civil-from-days algorithm in `current_date_iso_prefix_for_secs(secs: u64) -> String` produces an ISO-8601 date prefix; expiry uses string-prefix comparison (`"YYYY-MM-DD" <= &valid_for_end[..10]`). The 4-value regression test pins `1970-01-01`, `2000-02-29` (leap year), `2024-01-01`, and `2026-05-09` to known epoch-second values (P32-CHK-013).
6. **Pitfall 3 caveat:** an expired RETIRED tlog is NORMAL — old log keys are kept for historical bundle verification. Only the absence of any current key triggers fail-closed. Implemented as `any_active = tlogs.iter().any(|tlog| ...)`.
7. **Caller-site cascade:** all 6 sites in `crates/nono-cli/` that previously wrapped the call in `rt.block_on(...)` drop the Tokio runtime construction. Plan 02 Task 1's commit `ee1ae16c` made this a single mechanical change per site.

`nono setup --refresh-trust-root` (added by Plan 32-02 Task 2) is the explicit refresh path:

- Constructs a one-shot `tokio::runtime::Builder::new_current_thread()` runtime, calls `TrustedRoot::production().await`, serializes via `serde_json::to_string_pretty`, writes to the cache path with `0o600`-equivalent permissions on Unix.
- No admin required (Pitfall 7). Per-user cache at `<nono_home_dir()>/.nono/trust-root/trusted_root.json`.
- Fail-closed on fetch failure with the underlying TUF error surfaced.
- The `setup_refresh_trust_root_writes_cache` integration test is `#[ignore]`'d as "requires network access to https://tuf-repo-cdn.sigstore.dev (manual operator verification)" — the live `production()` call cannot be cleanly httpmock'd within the scope of Phase 32 (sigstore-rs's TUF client is not surface-pluggable in 0.6.5), so cache-state coverage is delivered via the hermetic `setup_check_only_reports_*_cache` companion tests instead.

### Frozen Test Fixture (D-32-02 / D-32-06)

`crates/nono/tests/fixtures/trust-root-frozen.json` is a captured-once-good copy of a syntactically-valid `TrustedRoot` JSON:

- **Source:** `sigstore/root-signing@main targets/trusted_root.json` (canonical authoritative TUF target). Fetched directly because the in-tree pinned root in `sigstore-verify 0.6.5` fails verification with the threshold-of-3 issue described in Context above.
- **Shape:** 6787 bytes; mediaType `application/vnd.dev.sigstore.trustedroot+json;version=0.1`; 2 Fulcio CAs; 2 Rekor tlogs; 1 timestamp authority; 2 ctlogs.
- **Loader:** `crates/nono/src/trust/mod.rs::load_test_trusted_root()` is `#[cfg(test)] pub(crate)` (D-32-15 enumerated D-19 deviation #2). It resolves the fixture path via `env!("CARGO_MANIFEST_DIR")` so tests work regardless of cwd at run time.
- **Pinning policy (D-32-06):** indefinite. No CI rotation job. The fixture's purpose is hermetic deserialization, not tracking upstream Sigstore changes. If Sigstore breaks deserializer compat in a future protobuf-specs version, the fixture is recaptured then; until then it stays.
- **Production code path NEVER reads this file.** Plan 02's `load_production_trusted_root` rewrite reads from `<nono_home_dir()>/.nono/trust-root/`, not from `tests/fixtures/`. The fixture is library-test-internal.

## Consequences

### Positive

- **Verify is offline.** Verify-is-offline invariant tested both structurally (source-grep of the verify path for `.await` / `reqwest` / `tokio::net` / `Runtime::new` / `.block_on`) AND dynamically (`verify_path_uses_no_async_network_io` runs `verify_bundle_with_digest` on a non-runtime `std::thread` to prove sync-only execution). No Sigstore-uptime DoS vector for end-user verify.
- **Hermetic CI.** The frozen fixture lets the entire Phase 32 test surface run without network access (D-32-07). 14 new hermetic tests pass; 2 originally-failing tests (`load_production_trusted_root_succeeds`, `verify_bundle_with_invalid_digest`) now pass via the fixture-backed test seam.
- **Per-user, no admin.** Refresh writes to `<nono_home_dir()>/.nono/trust-root/`. Per-user-MSI install convention preserved; multi-user Windows machines work without admin elevation.
- **Honors `NONO_TEST_HOME`.** Phase 27.1 D-27.1-08 test seam continues to work: tests setting `NONO_TEST_HOME` get isolated cache paths.
- **`crates/nono` dependency surface unchanged.** D-32-15 P32-CHK-002 holds: no `dirs` crate; `home_dir_from_env()` uses `std::env` only.
- **Diagnostic surface.** `nono setup --check-only` reports `Trust root cache: NOT INITIALIZED | STALE | OK <date>` so operators can see status without running a verify and parsing an error.

### Negative

- **First-run UX requires the operator to run `nono setup --refresh-trust-root` once.** D-32-05 fails-closed with a clear recovery hint, but it IS an extra step compared to "verify just works on first install." Acceptable because the alternative (status quo) was "verify is broken on first install AND every subsequent install due to upstream drift." Documented in the Windows POC handoff cookbook (Plan 32-05 Task 3).
- **Library API surface change.** `pub async fn` → `pub fn` is a semver-major break for any direct consumer of `nono::trust::load_production_trusted_root` outside the workspace. D-32-15 enumerates this as a deliberate D-19 deviation; the `bundle.rs` doc-comment names it explicitly; the upstream-drift script annotation prevents future audit alarms.
- **Refresh requires the broken-on-this-version `production()` call to actually succeed.** If upstream stays drifted, end-user `nono setup --refresh-trust-root` fails fail-closed with the underlying TUF error — same failure mode as the inline call, just at refresh time instead of verify time. Mitigated by the documented operator workaround (manual download from `sigstore/root-signing` if `production()` continues to fail; recorded in the Phase 32 deferred-items entry for v2.4+ TUF integration).
- **Two D-19 deviations in one phase.** D-32-15 enumerates and bounds them: (1) the `load_production_trusted_root` signature change; (2) the `load_test_trusted_root()` test-only helper. The `tests/integration/test_upstream_drift.sh:257` annotation prevents drift-audit false positives.

## References

### Internal

- `.planning/phases/32-sigstore-integration/32-CONTEXT.md` (D-32-01 cached-root design; D-32-02 frozen fixture; D-32-03 expired-cache fail-closed; D-32-04 sigstore-verify/sign 0.6.5 pin; D-32-05 first-run UX; D-32-06 indefinite pin; D-32-15 D-19 deviation enumeration)
- `.planning/phases/32-sigstore-integration/32-RESEARCH.md` (Pattern 1 cache-write; Pattern 2 sync verify; Pattern 3 frozen fixture; Pitfalls 2, 3, 4, 7)
- `.planning/phases/32-sigstore-integration/32-02-SUMMARY.md` (implementation: sync rewrite, 6 caller-site cascade, two-test bundle.rs migration, civil-from-days algorithm)
- `.planning/phases/32-sigstore-integration/32-01-SUMMARY.md` (test substrate: frozen fixture capture, `load_test_trusted_root()` helper, scaffold creation)
- `.planning/quick/260509-s9m-SUMMARY.md` (the upstream-drift failure that triggered this phase; the 2 originally-failing tests at `bundle.rs:877` and `bundle.rs:914`)

### Source code

- `crates/nono/src/trust/bundle.rs::load_production_trusted_root` — sync cache-reader; doc-comment names the upstream-divergence per Pitfall 4.
- `crates/nono/src/trust/bundle.rs::home_dir_from_env` — `std::env`-based home resolver (no `dirs` crate per D-32-15 / P32-CHK-002).
- `crates/nono/src/trust/bundle.rs::current_date_iso_prefix_for_secs` — Howard Hinnant civil-from-days; pinned by `current_date_iso_prefix_*` regression tests.
- `crates/nono/src/trust/mod.rs::load_test_trusted_root` — `#[cfg(test)] pub(crate)` library-test seam (D-32-15 #2).
- `crates/nono/tests/fixtures/trust-root-frozen.json` — 6787-byte frozen fixture (D-32-06).
- `crates/nono-cli/src/setup.rs::refresh_trust_root_step` — one-shot Tokio runtime; per-user cache writer.
- `crates/nono-cli/src/setup.rs::print_trust_root_status` — `--check-only` cache-status reporter.
- `crates/nono-cli/tests/setup_trust_root.rs`, `keyless_offline_invariant.rs` — hermetic integration coverage.
- `tests/integration/test_upstream_drift.sh:257` — `# intentional fork: Phase 32 D-32-01` annotation prevents drift-audit false positive.

### Related ADRs

- [Broker.exe Authenticode Self-Trust-Anchor](broker-trust-anchor.md) — the Phase 32 ADR for the OTHER novel decision (Windows broker-binary self-trust anchor). The two are parallel: this ADR governs `nono trust verify` against external Sigstore-signed artifacts; the broker ADR governs `nono.exe`'s internal trust of its own broker child. They share Phase 28's Authenticode primitives indirectly (the broker ADR; this ADR uses Sigstore's Rekor + Fulcio chain) but otherwise operate independently.
- `docs/architecture/audit-bundle-target.md` (Phase 27.2 ADR convention reference; this ADR mirrors its structure).
