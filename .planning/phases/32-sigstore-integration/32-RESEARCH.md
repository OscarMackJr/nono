# Phase 32: Sigstore Integration - Research

**Researched:** 2026-05-09
**Domain:** TUF cached-root refresh + keyless Sigstore CLI hardening + broker Authenticode self-trust-anchor
**Confidence:** HIGH for sigstore-rs API surface (verified by reading 0.6.5 source on disk); HIGH for Phase 28 chain-walker reuse (verified by reading `exec_identity_windows.rs`); HIGH for D-19 / cookbook / setup conventions (verified by reading existing code)

## Summary

Phase 32 closes four discrete trust gaps identified by quick task `260509-s9m`: (1) `TrustedRoot::production().await` fails on Windows because sigstore-rs 0.6.5's bundled TUF root is stale and the runtime fetch can't meet the threshold-of-3 signature requirement against current Sigstore TUF metadata; (2) the keyless CLI flow (`nono trust sign --keyless` + `nono trust verify --keyless`) has zero hermetic test coverage; (3) the keyless verify path constructs `VerificationPolicy::default()` with no identity / issuer pinning — a permissive default that contradicts CLAUDE.md's fail-secure principle; (4) Phase 31's broker dispatch arm (`launch.rs:1262`) spawns `nono-shell-broker.exe` with no signature check, so a malicious sibling broker swap goes undetected.

The cleanest path forward is to treat the cached `TrustedRoot` as a serializable artifact (sigstore-rs already implements `Serialize + Deserialize` on the `TrustedRoot` struct via `serde`, and exposes both `from_json(&str)` and `from_file(path)` constructors — verified at `sigstore-trust-root-0.6.5/src/trusted_root.rs:174-185`). `nono setup --refresh-trust-root` calls the existing `TrustedRoot::production().await`, persists the result as JSON under `<nono_home_dir()>/.nono/trust-root/trusted_root.json`, and `bundle.rs::load_production_trusted_root` is rewritten to read that file synchronously via `TrustedRoot::from_file`. The frozen test fixture is a captured-once-good copy of the same JSON payload checked into the repo. Mock Fulcio/Rekor for the keyless integration test is achievable by injecting a `SigningContext::with_config(SigningConfig { fulcio_url, rekor_url, .. })` pointing at a localhost test server (sigstore-sign 0.6.5's `SigningContext` exposes `with_config` — verified at `sigstore-sign-0.6.5/src/sign.rs:148`). Broker Authenticode verify reuses Phase 28's `query_authenticode_status` function unchanged — `nono.exe` calls it twice (once on `current_exe()`, once on the broker), then compares `signer_subject` + `thumbprint` for equality before `CreateProcessW`.

**Primary recommendation:** Wave order is **C → A → B → H** (CONTEXT discretion). C (cached-root design) lands first as a foundational `crates/nono/src/trust/bundle.rs` rewrite plus `nono-cli/src/setup.rs` extension. A is then a single fixture-migration commit on top of C. B (keyless hardening) and H (broker verify) are independent and can run in parallel after C.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| TUF root fetch via network | CLI (`nono setup --refresh-trust-root` in `nono-cli/src/setup.rs`) | — | Network I/O is policy; library stays offline-only per D-32-15 (no library API regression) |
| TUF root cache I/O (read for verify) | Library (`nono/src/trust/bundle.rs::load_production_trusted_root`) | — | Verify-side primitive; CLI calls library, not vice versa |
| Frozen test-fixture loading | Library (`nono/src/trust/mod.rs::load_test_trusted_root` `#[cfg(test)]`) | — | Test seam owned by library since 2 failing tests live in `bundle.rs` test module |
| Keyless sign | CLI (`nono-cli/src/trust_cmd.rs::sign_file_keyless`) | — | Already there; no library change |
| Keyless verify identity matching | CLI (`nono-cli/src/trust_cmd.rs::verify_single_file`) | Library (`nono/src/trust/bundle.rs::verify_bundle*`) | Library does sigstore-verify call; CLI does regex matching of SAN against `--identity` (sigstore-verify's `policy.identity` is exact-equality only — see Pitfall 1) |
| Mock Fulcio/Rekor test server | CLI test-only (`nono-cli/tests/keyless_sign.rs`) | — | Hermetic; SigningContext::with_config injection point |
| Broker Authenticode verify | CLI (`nono-cli/src/exec_strategy_windows/launch.rs::WindowsTokenArm::BrokerLaunch`) | CLI primitive (`nono-cli/src/exec_identity_windows.rs::query_authenticode_status`) | Phase 28's chain walker is already in CLI; reuse not in library |
| nono.exe self-introspection | CLI (broker dispatch site calls `query_authenticode_status(current_exe())`) | — | Trust anchor is `nono.exe`'s own Authenticode signature — see Pitfall 5 |

## User Constraints

> Copied verbatim from `32-CONTEXT.md` so the planner can verify compliance without re-reading.

### Locked Decisions (16 total — D-32-01 through D-32-16)

**TUF root refresh & test unblock (areas A + C):**
- **D-32-01:** TUF trusted root cached at `<nono_home_dir()>/.nono/trust-root/`; refreshed by `nono setup --refresh-trust-root` (per-user, no admin); fetches from `https://tuf-repo-cdn.sigstore.dev`; setup fails fail-closed if fetch fails; verify runs offline against cache. `bundle.rs::load_production_trusted_root` rewritten to read cache (NOT call `TrustedRoot::production()` directly).
- **D-32-02:** Frozen TUF fixture at `crates/nono/tests/fixtures/trust-root-frozen.json` (or equivalent); new `load_test_trusted_root()` helper in trust module test scope; the 2 failing unit tests use the fixture.
- **D-32-03:** Cached root expired with no network → fail-closed `NonoError::TrustVerification` with recovery hint: "Sigstore trusted root expired YYYY-MM-DD; run `nono setup --refresh-trust-root` (requires network)." Verify NEVER does inline network — verify-is-offline invariant.
- **D-32-04:** sigstore-verify + sigstore-sign stay pinned at 0.6.5. No version bump as part of Phase 32.
- **D-32-05:** First-run UX (cache never initialized) → fail-closed `NonoError::TrustPolicy("Sigstore trusted root not initialized; run `nono setup --refresh-trust-root` (requires network).")`. Symmetric with D-32-03.
- **D-32-06:** Frozen fixture pinned indefinitely; no CI rotation job.

**Public-good keyless CLI hardening (area B):**
- **D-32-07:** Keyless test coverage uses mock Fulcio/Rekor (NOT live infra). New `crates/nono-cli/tests/keyless_sign.rs`. No `#[cfg(feature = "online-tests")]` lane.
- **D-32-08:** `nono trust verify --keyless` requires explicit `--issuer` (OIDC issuer URL; exact match) AND `--identity` (regex pattern matching SAN/OIDC identity claim). Fails-closed if either flag missing.
- **D-32-09:** Keyless signing stays CI-only. No interactive browser OAuth. Local devs use `--keyref`. Phase 32 only polishes the existing `discover_oidc_token` error-message wording.
- **D-32-10:** Audit `.github/workflows/release.yml` signing posture; recommendation defaults to "keep current and document." Migration to keyless explicitly out of scope.

**Broker.exe Authenticode verification (area H):**
- **D-32-11:** Broker verification mechanism is Authenticode (NOT Sigstore), reusing Phase 28's chain walker (`parse_signer_subject` + `parse_thumbprint`).
- **D-32-12:** Broker Authenticode verify failure → fail-closed `NonoError::TrustVerification`. No escape-hatch flag, no env-var override. Dev-build skip mechanism is planner's discretion (NOT a runtime override).
- **D-32-13:** Trust anchor is `nono.exe`'s own subject + thumbprint — self-bootstrapping. Both binaries signed by Phase 31 Plan 04 release pipeline with the SAME identity. `nono.exe` extracts ITS OWN signature at launch, requires broker's signature to match.
- **D-32-14:** Verify on every broker dispatch. No cache.

**Cross-cutting:**
- **D-32-15:** D-19 invariant carries forward. ONLY two intentional `crates/nono/` changes: (1) `bundle.rs::load_production_trusted_root` rewrite + 2 failing-test fixture migration; (2) new `load_test_trusted_root()` helper.
- **D-32-16:** Phase 27.2's audit-attestation surface untouched.

### Claude's Discretion

- Exact `--refresh-trust-root` argument shape and integration with `--check-only` (whether `--check-only` reports cached-trust-root staleness alongside WFP service status).
- Exact error-message wording for D-32-03 / D-32-05 / D-32-09 / D-32-12 (so long as the message names the problem AND the recovery command).
- Mock Fulcio/Rekor implementation shape (sigstore-sign test machinery wrapper vs. minimal local mock vs. recorded HTTP fixture). The contract is "deterministic, hermetic, no network in CI."
- Exact policy file format for the baked-in `trust-policy.json` template (D-32-10).
- Dev-build broker-verification skip mechanism (D-32-12) — `#[cfg(debug_assertions)]` vs install-layout detector vs presence-of-Authenticode check. Researcher recommendation: install-layout detector — see Pitfall 6.
- Order of waves. Researcher recommendation: C → A → B + H (parallel after C).

### Deferred Ideas (OUT OF SCOPE)

- Interactive browser OAuth for keyless signing (cosign-style local-developer flow).
- Migrating `release.yml` to keyless signing.
- sigstore-rs version bump (0.6.5 → upstream-current).
- Cosign compatibility / Sigstore Bundle v0.3+ adoption beyond what 0.6.5 ships.
- Broker-trust caching (D-32-14 picks no-cache).
- Verifying `nono.exe` itself at launch.
- `#[cfg(feature = "online-tests")]` lane for live Sigstore smoke.
- CI rotation job for the frozen TUF fixture.
- `--block-net` Job Object fallback for unprivileged installs (separate Sigstore-unrelated concern).
- Centralized `trust-policy.json` under `<install_dir>` (D-32-10 alternative).

## Phase Requirements

ROADMAP.md § Phase 32 currently lists "Goal/Requirements: TBD". CONTEXT.md is the de facto contract until plans land — researcher does NOT invent REQ-IDs. Each D-32-XX decision below maps to a research-derived planner item:

| D-32-XX | Research Item | Section |
|---------|---------------|---------|
| D-32-01 | TUF cache JSON serialization round-trip via `TrustedRoot::from_file` / `serde_json` | Standard Stack, Code Examples |
| D-32-02 | Frozen-fixture format = same JSON shape as the cached file | Standard Stack |
| D-32-03 | Expiry detection from `TrustedRoot.tlogs[*].public_key.valid_for.end` ISO-8601 strings | Common Pitfalls |
| D-32-04 | Stay on sigstore-verify/sigstore-sign 0.6.5 (already pinned) | Standard Stack |
| D-32-05 | First-run detection = file absent vs file present | Code Examples |
| D-32-06 | Fixture stability — no rotation job | (informational) |
| D-32-07 | Mock Fulcio/Rekor via `SigningContext::with_config(SigningConfig { fulcio_url, rekor_url, .. })` | Standard Stack, Code Examples |
| D-32-08 | Identity regex matching is OUR job (sigstore-verify does exact equality only) | Common Pitfalls #1 |
| D-32-09 | Polish existing `discover_oidc_token` error message | Code Examples |
| D-32-10 | Read `.github/workflows/release.yml` signing posture; document | (informational) |
| D-32-11 | Reuse `query_authenticode_status` — already in `exec_identity_windows.rs` (Phase 28) | Reusable Assets |
| D-32-12 | Dev-build skip mechanism = install-layout detector | Common Pitfalls #6 |
| D-32-13 | `current_exe()` Authenticode self-introspection — already supported by `query_authenticode_status` (just pass current_exe() path) | Code Examples |
| D-32-14 | No cache — straight-line call site at `launch.rs:1262`+ | Architecture Patterns |
| D-32-15 | Library API change documented in `bundle.rs` doc-comment | Common Pitfalls #4 |
| D-32-16 | Audit-attestation untouched — researcher confirmed `audit_attestation.rs` test surface independent of the keyless paths | (informational) |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `sigstore-verify` | 0.6.5 (PINNED per D-32-04) | Bundle verification, TrustedRoot loading, VerificationPolicy | Already in `crates/nono/Cargo.toml`. No bump. [VERIFIED: `crates/nono/Cargo.toml:38`] |
| `sigstore-sign` | 0.6.5 (PINNED per D-32-04) | Keyless signing context + Signer | Already in `crates/nono-cli/Cargo.toml`. [VERIFIED: `crates/nono-cli/Cargo.toml:59`] |
| `sigstore-trust-root` | 0.6.5 (transitive, exposed as `sigstore_verify::trust_root`) | `TrustedRoot` struct (Serialize + Deserialize via serde), `TrustedRoot::from_json` / `from_file` / `production` (async) | Re-exported by sigstore-verify at `lib.rs:39` `pub use sigstore_trust_root as trust_root;`. [VERIFIED: `sigstore-verify-0.6.5/src/lib.rs:39`] |
| `serde_json` | workspace | TUF root cache serialization (writing the cached `TrustedRoot` to disk) | Already a workspace dep |
| `tokio` | 1 (already used in CLI for keyless flow) | Async runtime for the ONE-SHOT TUF fetch in `nono setup --refresh-trust-root` | The fetch is async (`TrustedRoot::production().await`); verify stays sync after the cache is hydrated |

### Supporting (test-only)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tempfile` | workspace | Per-test ephemeral `<NONO_TEST_HOME>` for keyless integration test isolation | Already used by `audit_attestation.rs` integration tests — same pattern |
| `httpmock` or `wiremock` | NEW (planner picks one) | Mock Fulcio + Rekor HTTP servers for hermetic keyless sign integration test | Recommendation: `httpmock` (sync-friendly, simpler API) — see Code Examples |
| `tokio-test` or built-in `#[tokio::test]` | (already used) | Async test harness for the TUF refresh path | The 2 failing tests at `bundle.rs:877` + `:914` already use `#[tokio::test]` — pattern preserved |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom JSON cache file | `TrustedRoot::from_tuf(TufConfig::production().offline())` (sigstore-rs's own offline/cache mode) | sigstore-rs's offline mode reads from its OWN cache directory (`directories::ProjectDirs::from("dev", "sigstore", "sigstore-rust")` — verified at `sigstore-trust-root-0.6.5/src/tuf.rs:427`). That's NOT under `<NONO_TEST_HOME>`, so D-27.1's test-home seam won't apply, AND the cache layout is opaque (TUF datastore subtree). **Reject** — explicit `<nono_home_dir()>/.nono/trust-root/trusted_root.json` is simpler, test-seam-friendly, and matches the per-user-MSI invariant in CONTEXT § Integration Points. |
| Caching TUF metadata bytes (root.json, timestamp.json, snapshot.json, targets.json) | Caching the derived `TrustedRoot` JSON | TUF-metadata caching forces verify to re-do the TUF protocol (which requires network for freshness validation). The derived `TrustedRoot` cache is the verified output — verify-is-offline preserved. **Pick derived** per D-32-01. |
| `httpmock` for Fulcio/Rekor mock | Custom Tokio HTTP listener | `httpmock` is a stable crate with mock-server-as-fixture support; custom listener doubles the maintenance surface. **Pick `httpmock`** unless planner finds a workspace-precedent that says otherwise. |
| `#[cfg(debug_assertions)]` skip for dev-build broker | Install-layout detector (path under `target/debug` or `target/release`?) | `#[cfg(debug_assertions)]` is compile-time — but `cargo test --release` (which Phase 31's `broker_launch_assigns_child_to_job_object` uses) compiles WITHOUT debug_assertions, so a release-build dev test would still try to verify and fail. Install-layout detector survives both `cargo test` and `cargo test --release` cleanly. **Pick install-layout detector** (see Pitfall 6). |

**Installation:**
```bash
# No new crate adds for production code (D-32-04)
# Dev-deps add (in crates/nono-cli/Cargo.toml [dev-dependencies]):
cargo add --dev --package nono-cli httpmock@0.7
# (planner verifies version against npm-equivalent: https://crates.io/crates/httpmock)
```

**Version verification:** `cargo info sigstore-verify@0.6.5` confirms version available; `Cargo.lock` line 2982 confirms `sigstore-trust-root` resolves at 0.6.5. [VERIFIED: `Cargo.lock:2982,2990`]

## Architecture Patterns

### System Architecture Diagram

```
nono setup --refresh-trust-root          (one-shot, online; per-user, no admin)
    └─> sigstore_verify::trust_root::TrustedRoot::production().await        # network call
        └─> serde_json::to_string_pretty(&trusted_root)                     # serialize
            └─> std::fs::write(<nono_home_dir()>/.nono/trust-root/trusted_root.json)
                                                                            ↓
                                                            [cached forever, refreshed only on next setup --refresh]


nono trust verify --keyless --issuer X --identity REGEX <bundle>            (every run, offline)
    └─> bundle.rs::load_production_trusted_root()                           # NEW BEHAVIOR
        ├─> std::fs::read_to_string(<nono_home_dir()>/.nono/trust-root/trusted_root.json)
        │   └─> NonoError::TrustPolicy("not initialized; run nono setup --refresh-trust-root")  # D-32-05
        └─> sigstore_verify::trust_root::TrustedRoot::from_json(&json)
            └─> check valid_for.end < now                                   # D-32-03 expiry gate
                └─> NonoError::TrustVerification("expired YYYY-MM-DD; run ...")
    └─> sigstore_verify::verify(artifact, bundle, VerificationPolicy::default(), &trusted_root)
        └─> bundle.rs::extract_signer_identity(...)                         # NEW: regex match SAN against --identity
            └─> regex check (regress crate, already a nono dep)             # D-32-08 our-side regex
                └─> issuer == --issuer (exact match — sigstore-verify's contract)


nono shell ...                                                              (every dispatch, no cache; D-32-14)
    └─> WindowsTokenArm::BrokerLaunch (launch.rs:1246+)
        ├─> let nono_subject_thumbprint = query_authenticode_status(current_exe())?       # NEW
        │   └─> match Valid { signer_subject, thumbprint } else fail-closed (D-32-12)
        ├─> let broker_subject_thumbprint = query_authenticode_status(broker_path)?       # NEW
        │   └─> match Valid { signer_subject, thumbprint } else fail-closed (D-32-12)
        ├─> if broker_subject != nono_subject || broker_thumbprint != nono_thumbprint:
        │   └─> NonoError::TrustVerification("broker.exe Authenticode mismatch...")
        └─> CreateProcessW(broker_path, ...)                                # unchanged
```

### Recommended Project Structure
```
crates/nono/src/trust/
├── bundle.rs           # MODIFIED: load_production_trusted_root rewrite (D-32-15 #1)
└── mod.rs              # MODIFIED: load_test_trusted_root() #[cfg(test)] helper (D-32-15 #2)
crates/nono/tests/fixtures/
└── trust-root-frozen.json  # NEW: D-32-02 frozen fixture (captured-once-good)

crates/nono-cli/src/
├── setup.rs            # MODIFIED: SetupRunner::refresh_trust_root() impl (D-32-01)
├── cli.rs              # MODIFIED: SetupArgs adds pub refresh_trust_root: bool (D-32-01); TrustVerifyArgs adds pub issuer: Option<String>, pub identity: Option<String> (D-32-08)
├── trust_cmd.rs        # MODIFIED: run_verify() threads --issuer / --identity into VerificationPolicy + post-verify SAN regex (D-32-08); discover_oidc_token error wording polish (D-32-09)
└── exec_strategy_windows/launch.rs  # MODIFIED: BrokerLaunch arm wraps Authenticode self-trust-anchor verify around CreateProcessW (D-32-13/14)
crates/nono-cli/tests/
└── keyless_sign.rs     # NEW: hermetic keyless sign + verify roundtrip via httpmock'd Fulcio + Rekor (D-32-07)

docs/architecture/
└── broker-trust-anchor.md      # NEW (suggested): ADR for D-32-13 self-trust-anchor pattern
docs/cli/development/windows-poc-handoff.mdx
                                # MODIFIED: cookbook gets `nono setup --refresh-trust-root` prereq line + --issuer / --identity examples (per CONTEXT § Specifics)
```

### Pattern 1: TUF cache write (online, in setup) → JSON file
**What:** `nono setup --refresh-trust-root` calls `TrustedRoot::production().await`, serializes the result to JSON via `serde_json::to_string_pretty`, writes to a fixed path under `nono_home_dir()`.
**When to use:** Once per user per Sigstore trust-material rotation event (or whenever the user wants to refresh).
**Example:**
```rust
// In crates/nono-cli/src/setup.rs (NEW METHOD on SetupRunner)
// Source: sigstore-trust-root-0.6.5/src/trusted_root.rs:174-185 + tuf.rs:454
fn refresh_trust_root(&self) -> Result<()> {
    let cache_dir = crate::config::nono_home_dir()?
        .join(".nono")
        .join("trust-root");
    std::fs::create_dir_all(&cache_dir).map_err(NonoError::Io)?;

    // ONE-SHOT tokio runtime — the rest of `nono setup` is sync.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| NonoError::Setup(format!("tokio runtime: {e}")))?;
    let trusted_root = rt.block_on(
        nono::trust::TrustedRoot::production()
    ).map_err(|e| NonoError::Setup(format!(
        "Failed to fetch Sigstore trusted root from https://tuf-repo-cdn.sigstore.dev: {e}"
    )))?;

    let json = serde_json::to_string_pretty(&trusted_root)
        .map_err(|e| NonoError::Setup(format!("serialize trusted root: {e}")))?;
    let cache_path = cache_dir.join("trusted_root.json");
    std::fs::write(&cache_path, json).map_err(NonoError::Io)?;

    println!("  * Sigstore trusted root cached at {}", cache_path.display());
    Ok(())
}
```

### Pattern 2: TUF cache read (offline, in verify) → struct
**What:** `bundle.rs::load_production_trusted_root` is rewritten to read the cache file synchronously and check `valid_for.end` for expiry.
**When to use:** Every keyless verify path (`trust_cmd.rs::verify_single_file` keyless arm and `verify_multi_subject_file` keyless arm both call this).
**Example:**
```rust
// In crates/nono/src/trust/bundle.rs (REWRITTEN — D-32-15 #1)
// NOTE: signature changes from `pub async fn load_production_trusted_root() -> Result<TrustedRoot>`
//       to `pub fn load_production_trusted_root() -> Result<TrustedRoot>` (no async).
//       This is a deliberate API surface change documented in the doc-comment.
//       Callers in trust_cmd.rs:912, trust_cmd.rs:1032, trust_scan.rs:255, trust_scan.rs:760,
//       trust_intercept.rs:376 currently wrap in `rt.block_on(...)` — those wrappers come out.
//
// Source: sigstore-trust-root-0.6.5/src/trusted_root.rs:181 (from_file is sync)
pub fn load_production_trusted_root() -> Result<TrustedRoot> {
    let cache_path = nono_home_dir()?
        .join(".nono")
        .join("trust-root")
        .join("trusted_root.json");

    if !cache_path.exists() {
        return Err(NonoError::TrustPolicy(
            "Sigstore trusted root not initialized; run \
             `nono setup --refresh-trust-root` (requires network).".to_string()
        ));
    }

    let trusted_root = TrustedRoot::from_file(&cache_path)
        .map_err(|e| NonoError::TrustPolicy(format!(
            "failed to load cached trusted root from {}: {e}",
            cache_path.display()
        )))?;

    // D-32-03: expiry gate — fail-closed if cached root has expired.
    check_trusted_root_freshness(&trusted_root)?;

    Ok(trusted_root)
}

/// Returns Err(TrustVerification) if all transparency log keys have expired.
/// A fresh trusted root has at least one tlog whose valid_for.end is in the future.
fn check_trusted_root_freshness(root: &TrustedRoot) -> Result<()> {
    // TrustedRoot.tlogs[*].public_key.valid_for: Option<ValidityPeriod>
    // ValidityPeriod.end: Option<String> (ISO 8601)
    // [VERIFIED: sigstore-trust-root-0.6.5/src/trusted_root.rs:160-172]
    //
    // Implementation: at least one tlog's end (parsed via chrono::DateTime) is > now.
    // If ALL tlogs are expired, the cache is stale.
    // (Library doesn't depend on chrono — parse manually or add chrono.)
    // ...
}
```

> Note for the planner: `nono` library does NOT currently have `chrono` as a dep ([VERIFIED: `crates/nono/Cargo.toml:15-26`]). Three options for parsing ISO 8601: (a) add `chrono` to `nono` (a library API surface widening — counter-D-19), (b) string-prefix-compare `"2026-05-09" < end` after stripping time (lossy but fail-closed), (c) parse with `time` crate, (d) leave expiry detection in the CLI. **Researcher recommends option (b)** for D-19 byte-identicality compliance — date-prefix string comparison is fail-closed correct for the "end is at least N days from today" question and adds zero dependencies.

### Pattern 3: Frozen test fixture
**What:** `crates/nono/tests/fixtures/trust-root-frozen.json` is a captured-once-good copy of a known-valid `TrustedRoot` JSON. The 2 failing tests use it via the new `load_test_trusted_root()` helper.
**When to use:** Test-only; never read by production code paths.
**Example:**
```rust
// In crates/nono/src/trust/mod.rs (NEW HELPER — D-32-15 #2)
#[cfg(test)]
pub(crate) fn load_test_trusted_root() -> crate::Result<crate::trust::TrustedRoot> {
    // Use CARGO_MANIFEST_DIR so the path resolves regardless of cwd at test time.
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("trust-root-frozen.json");
    crate::trust::bundle::load_trusted_root(&path)
}

// In crates/nono/src/trust/bundle.rs (TEST MIGRATION)
#[tokio::test]   // remains tokio::test for harness consistency, but body is now sync
async fn load_production_trusted_root_succeeds() {
    // OLD: let root = load_production_trusted_root().await;
    // NEW: load the frozen fixture; this test now exercises the test seam,
    //      not the production cache (which doesn't exist in CI).
    let root = crate::trust::load_test_trusted_root();
    assert!(root.is_ok(), "frozen fixture must load: {:?}", root.err());
}

#[tokio::test]
async fn verify_bundle_with_invalid_digest() {
    let json = make_public_key_bundle_json("key");
    let bundle = Bundle::from_json(&json).unwrap();
    let root = crate::trust::load_test_trusted_root().unwrap();    // CHANGED
    let policy = VerificationPolicy::default();
    let result = verify_bundle_with_digest("not-hex!", &bundle, &root, &policy, Path::new("test"));
    assert!(result.is_err());  // unchanged: invalid hex still fails
}
```

> **How to capture the frozen fixture:** On a developer machine with network access, run a one-time helper that calls `TrustedRoot::production().await`, serializes via `serde_json::to_string_pretty`, and writes to `crates/nono/tests/fixtures/trust-root-frozen.json`. Commit the file. The fixture is human-readable JSON; `git diff` against future updates is meaningful. D-32-06: pinned indefinitely; no rotation job. Estimated size: ~30-100 KB (5 Fulcio CAs + 4 Rekor logs + 4 TSA chains).

### Pattern 4: Mock Fulcio/Rekor for keyless integration test
**What:** Spin up `httpmock` listeners on random localhost ports; build `SigningConfig { fulcio_url: <mock_fulcio_url>, rekor_url: <mock_rekor_url>, .. }`; pass to `SigningContext::with_config(...)`; the resulting `Signer` issues against the mock. The test bundle then verifies with the frozen fixture (which contains the same mock CA cert in `certificate_authorities`).
**When to use:** `crates/nono-cli/tests/keyless_sign.rs` (NEW). Hermetic; runs in CI without Sigstore infra.
**Example:**
```rust
// In crates/nono-cli/tests/keyless_sign.rs (NEW)
//
// Source: sigstore-sign-0.6.5/src/sign.rs:148 (SigningContext::with_config) +
//         sigstore-sign-0.6.5/src/sign.rs:25-49 (SigningConfig fields)
#[tokio::test]
async fn keyless_sign_then_verify_roundtrip() {
    // 1. Mock Fulcio that returns a locally-generated cert chained to a test CA.
    let fulcio_mock = httpmock::MockServer::start();
    fulcio_mock.mock(|when, then| {
        when.method("POST").path("/api/v2/signingCert");
        then.status(200).body(test_fulcio_response_bytes());
    });

    // 2. Mock Rekor that returns a fake tlog entry signed by the same test CA.
    let rekor_mock = httpmock::MockServer::start();
    rekor_mock.mock(|when, then| {
        when.method("POST").path("/api/v1/log/entries");
        then.status(201).body(test_rekor_entry_bytes());
    });

    // 3. SigningContext::with_config injection (the test seam).
    let config = sigstore_sign::SigningConfig {
        fulcio_url: fulcio_mock.url(""),
        rekor_url: rekor_mock.url(""),
        tsa_url: None,
        signing_scheme: sigstore_sign::SigningScheme::EcdsaP256Sha256,
        rekor_api_version: sigstore_sign::RekorApiVersion::V1,
    };
    let context = sigstore_sign::SigningContext::with_config(config);
    // ... rest of the keyless sign flow ...

    // 4. Verify via the frozen fixture (which carries the matching test CA pubkey)
    //    — the bundle must round-trip through the existing verify path.
}
```

> **The mock infra is the trickiest part.** `test_fulcio_response_bytes()` must return a valid X.509 cert chain with the right Fulcio OID extensions (1.3.6.1.4.1.57264.1.12 etc — see `bundle.rs:51-67`). One viable simplification: the planner generates these fixtures ONCE on a Sigstore staging instance using the real `sigstore_sign` flow, captures the wire bytes, and replays them deterministically. This is the same "captured-once-good" pattern as the frozen TUF root.

### Pattern 5: Authenticode self-trust-anchor for broker
**What:** At the broker dispatch site (`launch.rs:1262+`), call `query_authenticode_status` once on `current_exe()` and once on the broker; require equal `signer_subject` AND equal `thumbprint`.
**When to use:** Every broker dispatch (D-32-14: no cache).
**Example:**
```rust
// In crates/nono-cli/src/exec_strategy_windows/launch.rs::WindowsTokenArm::BrokerLaunch
// (insert AFTER `let broker_path = exe_dir.join("nono-shell-broker.exe");` at line 1262)
//
// Source: crates/nono-cli/src/exec_identity_windows.rs:119 (query_authenticode_status)
if !is_dev_build_layout(&nono_exe) {     // see Pitfall 6 for is_dev_build_layout
    use crate::exec_identity::AuthenticodeStatus;

    let nono_status = crate::exec_identity_windows::query_authenticode_status(&nono_exe)?;
    let broker_status = crate::exec_identity_windows::query_authenticode_status(&broker_path)?;

    let (nono_subject, nono_thumbprint) = match nono_status {
        AuthenticodeStatus::Valid { signer_subject, thumbprint } => (signer_subject, thumbprint),
        other => return Err(NonoError::TrustVerification {
            path: nono_exe.display().to_string(),
            reason: format!(
                "nono.exe Authenticode status is {other:?} (expected Valid). \
                 Self-trust-anchor unavailable; refusing to spawn broker."
            ),
        }),
    };
    let (broker_subject, broker_thumbprint) = match broker_status {
        AuthenticodeStatus::Valid { signer_subject, thumbprint } => (signer_subject, thumbprint),
        other => return Err(NonoError::TrustVerification {
            path: broker_path.display().to_string(),
            reason: format!(
                "broker.exe Authenticode status is {other:?} (expected Valid)."
            ),
        }),
    };
    if nono_subject != broker_subject || nono_thumbprint != broker_thumbprint {
        return Err(NonoError::TrustVerification {
            path: broker_path.display().to_string(),
            reason: format!(
                "broker.exe Authenticode signature does not match nono.exe — \
                 expected subject `{nono_subject}` thumbprint `{nono_thumbprint}`, \
                 got subject `{broker_subject}` thumbprint `{broker_thumbprint}`. Refusing to spawn."
            ),
        });
    }
}
```

### Anti-Patterns to Avoid

- **Don't put TUF refresh networking in verify path.** D-32-03's "verify is offline" invariant is structural. The only network call in Phase 32 lives in `nono setup --refresh-trust-root`. Anything else creates a denial-of-service vector (verify hangs when TUF is down) AND a privacy leak (every `nono trust verify` phones home).
- **Don't inject the broker-trust skip via env var.** D-32-12 explicitly: "no escape hatch (no `NONO_BROKER_VERIFY=off` env var, no flag)." A skip flag IS an attacker-controlled override.
- **Don't reuse sigstore-rs's TUF cache directory.** `directories::ProjectDirs::from("dev", "sigstore", "sigstore-rust")` at `sigstore-trust-root-0.6.5/src/tuf.rs:427` is opaque (TUF datastore subtree), NOT under `<nono_home_dir()>`, so D-27.1's `NONO_TEST_HOME` test seam doesn't reach it. Use the explicit `<nono_home_dir()>/.nono/trust-root/trusted_root.json` location.
- **Don't add `chrono` to `crates/nono/Cargo.toml`.** D-19 invariant — library API stays minimal. Use string-prefix date comparison for ISO 8601 expiry checks (or move freshness logic to CLI if string-prefix is too lossy).
- **Don't add a regex-matched `--identity` to sigstore-verify's policy.identity.** sigstore-verify does exact string equality (verified at `sigstore-verify-0.6.5/src/verify.rs:277`). The regex match must happen in OUR code AFTER `extract_signer_identity`. See Pitfall 1.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TUF protocol metadata fetch | Custom `tough` wrapper | `sigstore_verify::trust_root::TrustedRoot::production()` | sigstore-rs already wraps `tough` (TUF Rust client v0.21.0); shipping our own duplicates the bug surface AND the upstream-divergence problem we accepted in PR #777/#778. |
| TUF root JSON serialization | Custom JSON encoder | `serde_json::to_string_pretty(&trusted_root)` | `TrustedRoot` derives `Serialize` (verified at `sigstore-trust-root-0.6.5/src/trusted_root.rs:18`); round-trip works out of the box. |
| Authenticode chain walking | New WTHelper FFI | `crates/nono-cli/src/exec_identity_windows.rs::query_authenticode_status` (Phase 28) | Already implemented, fail-closed-correct, has SAFETY comments, `WinTrustCloseGuard` handles state cleanup. Just call it twice. |
| OIDC ambient detection | Custom env-var sniffing | `sigstore_sign::oidc::IdentityToken::detect_ambient()` (already used at `trust_cmd.rs:660`) | Phase 32 changes ONLY the error-message wording on the None branch (D-32-09). |
| HTTP mock server | Custom Tokio listener | `httpmock` crate (NEW dev-dep) OR planner alternative | Standard ecosystem tool; ~3KB of test code instead of ~300. |
| ISO 8601 date parsing | New parser | String-prefix comparison `"2026-05-09" < &valid_for_end[..10]` | D-19 invariant (no chrono in `crates/nono/`). Lossy but fail-closed for the "is end-date in the past?" question. |
| Identity regex matching | New regex crate dep | `regress` crate (already a `nono` dep at `Cargo.toml:26`) | Don't add another regex crate when `regress` is already pulled in for OIDC issuer pinning (`signing.rs:86`). |

**Key insight:** Phase 32 is overwhelmingly an integration phase — almost every primitive it needs already exists somewhere. The largest net-new code is the mock-Fulcio/Rekor test fixture (Pattern 4); everything else is a 5-30 line glue change.

## Runtime State Inventory

This is NOT a rename / refactor / migration phase. No stored data, OS-registered state, or build artifacts carry the old name. The only state Phase 32 introduces is the new TUF cache file at `<nono_home_dir()>/.nono/trust-root/trusted_root.json`, which is creatable from scratch via `nono setup --refresh-trust-root`. No data migration needed.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — TUF cache is brand-new at a brand-new path | None |
| Live service config | None — `nono setup --refresh-trust-root` is per-user, no service registration | None |
| OS-registered state | None — no Windows Task Scheduler / launchd / systemd entries touched | None |
| Secrets/env vars | None — no new env-var conventions; existing `NONO_TEST_HOME` is honored via `nono_home_dir()` | None |
| Build artifacts | None — all changes are source-level; release pipeline (Phase 31 Plan 04) already signs both binaries | None |

**Nothing found in any category** — verified by reading the entirety of `setup.rs`, `trust_cmd.rs`, `bundle.rs`, `launch.rs::BrokerLaunch`. Phase 32 is purely additive code + one rewrite of one library function.

## Common Pitfalls

### Pitfall 1: sigstore-verify's `policy.identity` is exact-equality, NOT regex
**What goes wrong:** Planner reads CONTEXT D-32-08 ("`--identity` accepts a regex pattern") and threads `--identity` straight into `VerificationPolicy::with_identity(...)`. At test time, the test passes a regex `^https://github\.com/.*/release\.yml@.*$`, sigstore-verify compares it for literal equality against the SAN string `https://github.com/always-further/nono/.github/workflows/release.yml@refs/tags/v2.4.0`, and verify fails with "identity mismatch."
**Why it happens:** `VerificationPolicy.identity` is `Option<String>`; sigstore-verify line 277 does `actual_identity == expected_identity`. [VERIFIED: `sigstore-verify-0.6.5/src/verify.rs:275-291`]
**How to avoid:** Leave `VerificationPolicy::default()` (no identity / issuer pin in the sigstore-verify policy), then AFTER `verify` succeeds, call `bundle.rs::extract_signer_identity` to pull the SAN, and run `regress::Regex::new(&user_identity_pattern)?.find(san_str).is_some()` for the regex match. Use the existing `regress` crate (already a `nono` dep). Issuer is exact-match (D-32-08 says "OIDC issuer URL"), so issuer CAN go through `policy.issuer`.
**Warning signs:** A test that passes a regex like `.*` and gets "identity mismatch: expected `.*`, got `https://...`" instead of success.

### Pitfall 2: `TrustedRoot::production().await` is the FAILING call — don't accidentally call it from verify path
**What goes wrong:** Planner rewrites `bundle.rs::load_production_trusted_root` to read from cache, but a downstream reviewer notices the function is now sync and asks "why was it async?" — someone "helpfully" reverts the signature back to async by re-adding `.await` AND `tokio::runtime::Builder` blocking around the file read. That works but reintroduces a tokio-runtime requirement on the verify path (which is no longer warranted) AND makes the function look like it might do network I/O.
**Why it happens:** The ONLY reason `load_production_trusted_root` was async in v2.3 was that `TrustedRoot::production()` is async. After D-32-01, the function reads from a file (sync). Keeping it async is dead async.
**How to avoid:** Document the API change in the function's doc-comment (D-32-15 explicitly calls out documenting it). The 5 call sites in CLI (`trust_cmd.rs:912`, `trust_cmd.rs:1032`, `trust_scan.rs:255`, `trust_scan.rs:760`, `trust_intercept.rs:376`) all currently wrap in `rt.block_on(...)` — those wrappers come out as part of the migration. CHANGELOG entry for the library API change.
**Warning signs:** Pre-merge: `cargo build` complains about `unused tokio runtime`. Post-merge: a future test sets `cargo nextest --threads=1` and finds the verify path mysteriously hits TUF.

### Pitfall 3: TUF cache expiry detection has an "all logs expired" interpretation question
**What goes wrong:** Planner writes `check_trusted_root_freshness` to fail when ANY tlog has expired. But Sigstore's TUF root contains both ACTIVE and HISTORICAL transparency-log keys (so old bundles signed under retired keys can still be verified). An expired retired-key tlog is normal; failing on its presence breaks verification of legitimate older bundles.
**Why it happens:** `valid_for.end` on a tlog is the END of that key's validity for SIGNING new entries. Expired keys are kept for verifying historical entries.
**How to avoid:** `check_trusted_root_freshness` succeeds if AT LEAST ONE active tlog has `valid_for.end > now`, NOT if all do. Test: emit the in-tree fixture with one expired and one current tlog and assert success. Better: the freshness check is on the cache FILE's mtime relative to a configurable max-age (e.g., "cache > 30 days old"), independent of any ValidityPeriod gating, since we want to nudge users to refresh even if the keys are still valid.
**Warning signs:** Verifying a bundle from 2024 (signed under a retired Rekor key) suddenly fails after the freshness gate is added.

### Pitfall 4: The library API change in `bundle.rs::load_production_trusted_root` IS counter-D-19; document it
**What goes wrong:** A future Phase 33+ runs the upstream-drift script (`tests/integration/test_upstream_drift.sh:257` — verified, the script literally checks `'load_production_trusted_root'` as one of its drift sentinels) and flags the rewritten function as upstream-divergent, asking "is this an intentional fork divergence or did we miss an upstream update?" Without documentation, the answer is unclear and the function gets reverted.
**Why it happens:** D-19 says `crates/nono/` stays byte-identical to upstream EXCEPT for documented intentional changes. D-32-15 enumerates this rewrite as one of the two intentional changes. The drift-script entry needs a comment OR the function's doc-comment needs an explicit upstream-divergence note.
**How to avoid:** (a) Multi-line doc-comment on the rewritten function explicitly says: "Diverges from upstream `sigstore_verify::TrustedRoot::production()`-style flow per Phase 32 D-32-01: this fork caches the trusted root explicitly under `<nono_home_dir()>/.nono/trust-root/` to keep verify offline. See `.planning/phases/32-sigstore-integration/32-CONTEXT.md`." (b) Same comment block adds a CHANGELOG entry under `crates/nono/CHANGELOG.md` (or wherever the project keeps its changelog). (c) Update `tests/integration/test_upstream_drift.sh:257` to add a same-line `# intentional fork: Phase 32 D-32-01` comment.
**Warning signs:** A future "refresh stack onto upstream" quick task asks "why is `load_production_trusted_root` async upstream and sync in fork?"

### Pitfall 5: `query_authenticode_status` on `current_exe()` is supported, but reentrancy needs verification
**What goes wrong:** Planner writes `query_authenticode_status(&std::env::current_exe()?)` and assumes `WinVerifyTrust(WTD_CHOICE_FILE)` reads the file from disk in process-isolated mode. In practice it works (the kernel reads the PE on disk; the running image is mapped, not exclusive-locked), but a paranoid reviewer asks "is there a reentrancy concern when the verifier is the verifying binary?"
**Why it happens:** `WinVerifyTrust` opens the file via `WTD_CHOICE_FILE` (verified at `exec_identity_windows.rs:152`); on Windows, executable file shares are read-allowed even while the file is mapped as the running image. Phase 28's existing tests verify against `C:\Windows\System32\notepad.exe`, NOT against the running test binary, so there's no in-tree precedent for self-introspection.
**How to avoid:** Add a Phase 32 unit test `current_exe_authenticode_self_introspection_succeeds` (Windows-only, gated `#[cfg(target_os = "windows")]`) that runs in the test binary and asserts `query_authenticode_status(&std::env::current_exe()?).is_ok()`. On a developer Windows machine the test binary is unsigned (returns `AuthenticodeStatus::Unsigned`); on a CI release runner it's signed (returns `Valid`). Either is OK — the test asserts NO error, NOT a specific status. This hedges against any future windows-sys regression that breaks self-handle reads.
**Warning signs:** Production Windows MSI hits "WinVerifyTrust returned hresult=0x..." when called on its own path.

### Pitfall 6: Dev-build broker skip — `#[cfg(debug_assertions)]` doesn't cover `cargo test --release`
**What goes wrong:** Planner picks `#[cfg(debug_assertions)]` to skip the broker Authenticode check in dev. Phase 31's existing test `broker_launch_assigns_child_to_job_object` (at `launch.rs:2247`) requires the broker built via `cargo build --release`. A developer running `cargo test --release` to validate a code change finds the broker-verify gate fires (release mode = no debug_assertions = full verify) against an unsigned dev-built broker, fails, and the test errors out before it can assert Job Object containment.
**Why it happens:** `debug_assertions` is set ONLY in debug profile. Release profile (including `cargo test --release`) compiles them out.
**How to avoid:** Use an install-layout detector. Production-installed `nono.exe` lives under `Program Files\nono\` (machine-MSI) or `LocalAppData\Programs\nono\` (user-MSI); dev builds live under `target\(debug|release)\`. The detector is a single string-contains check on `current_exe().to_string_lossy()`:
```rust
fn is_dev_build_layout(nono_exe_path: &Path) -> bool {
    let s = nono_exe_path.to_string_lossy();
    s.contains(r"\target\debug\") || s.contains(r"\target\release\")
        || s.contains("/target/debug/") || s.contains("/target/release/")
}
```
This is ~5 lines, runs at every dispatch, has zero allocation cost, and is byte-identical between `cargo build` and `cargo build --release`. The detector explicitly LOGS at info-level when it activates so production runs (which never hit it) are loud about any unexpected activation.
**Warning signs:** A field-test on a real install hits "broker.exe Authenticode signature does not match" because the install-layout detector incorrectly thought it was a dev build.

### Pitfall 7: `nono_home_dir()` returns the user's NONO_TEST_HOME-aware home, not the install dir
**What goes wrong:** Planner reads "cache directory" and writes the cache under `<install_dir>\nono\trust-root\` (machine-scope, requires admin). D-32-01 explicitly says per-user, no admin.
**Why it happens:** Cookbook precedent for `nono setup --install-wfp-service` IS admin-required (because it registers a Windows service), so by analogy the planner thinks `--refresh-trust-root` is too. It's not.
**How to avoid:** Trust-root cache lives under `crate::config::nono_home_dir()?.join(".nono").join("trust-root")` — same pattern as audit / rollback / hooks (verified at `nono-cli/src/audit_session.rs:36`, `hooks.rs:73`, `rollback_session.rs:40`). NO admin. `nono setup --refresh-trust-root` does NOT call `is_admin_process()`.

## Code Examples

(See Architecture Patterns above — Patterns 1-5 each include full code examples sourced from the verified upstream code.)

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `TrustedRoot::production().await` at every verify path | Cached `<nono_home_dir()>/.nono/trust-root/trusted_root.json` + `from_file` | Phase 32 (this phase, 2026-05) | Verify becomes offline; no Sigstore-uptime CI dependency |
| `VerificationPolicy::default()` (no identity / issuer pinning) | `--issuer` exact match + `--identity` regex match | Phase 32 D-32-08 | Keyless verify becomes fail-closed-by-default |
| `nono shell` spawns `nono-shell-broker.exe` with no signature check | Authenticode self-trust-anchor verify before `CreateProcessW` | Phase 32 D-32-13/14 | Closes the broker-binary trust loop opened by Phase 31 |
| Bundled-with-sigstore-rs TUF root that's stale on Windows | User-refreshable cache | Phase 32 D-32-01 | Restores keyless-verify functional parity on Windows |

**Deprecated/outdated:**
- `pub async fn load_production_trusted_root() -> Result<TrustedRoot>` (becomes `pub fn` — sync). 5 callers update to drop the `rt.block_on(...)` wrapper. Documented in `bundle.rs` doc-comment per D-32-15.
- The 2 failing unit tests' direct call to `load_production_trusted_root().await`. Replaced with `load_test_trusted_root()` that reads the frozen fixture.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `httpmock` is the right crate for the mock Fulcio/Rekor; alternatives `wiremock` and `mockito` are equivalent in capability | Standard Stack (Supporting) | Planner picks differently; minor — the mock-server crate is replaceable |
| A2 | The frozen TUF fixture is ~30-100 KB; `git diff` against future updates is meaningful | Pattern 3 | If actually 1+ MB, planner adds it as a binary fixture or compresses |
| A3 | String-prefix date comparison is sufficient for D-32-03 expiry detection (vs. `chrono`) | Pattern 2, Pitfall 3 | If tlog `valid_for.end` field is missing or has unexpected format, fail-closed is correct (the fixture stays loaded; verify still works against an in-tree-known-good root). User-visible: stale-but-not-yet-expired root keeps working — see Pitfall 3 "the freshness check should be on the cache FILE's mtime, not just on tlog ValidityPeriods" |
| A4 | The install-layout detector (`\target\debug\` / `\target\release\` substring on `current_exe()`) is sufficient to distinguish dev from production for D-32-12 | Pitfall 6 | If a customer happens to have a path containing `\target\release\` for non-cargo reasons (unlikely — string is literal `\target\release\` not just `\release\`), they get a dev-build skip in production. Mitigation: log at info-level when detector activates so unexpected hits show in logs. |
| A5 | `query_authenticode_status(&std::env::current_exe()?)` returns successfully on a self-introspecting process; no reentrancy issue | Pattern 5, Pitfall 5 | If WinVerifyTrust holds a file lock that conflicts with the running image's file mapping, the call fails. Pre-Phase-32 unit test `current_exe_authenticode_self_introspection_succeeds` validates this assumption — see Pitfall 5. |
| A6 | The keyless integration test mock Fulcio cert can be captured-once-good and replayed deterministically (analogous to the frozen TUF fixture) | Pattern 4 | If the mock cert's validity window expires (which it will), the test starts failing on a future date. Mitigation: use `VerificationPolicy::default()` with `verify_certificate_chain` skipped (the `skip_certificate_chain()` builder method, verified at `verify.rs:94`) for the mock test specifically, OR generate the mock cert ON-DEMAND in the test using `aws_lc_rs` (already a CLI dep at `Cargo.toml:54`) with a fresh validity window. The latter is cleaner. |
| A7 | The `regress` crate's regex semantics match operator expectations for `--identity` patterns (e.g. `^https://github\.com/.*/release\.yml@refs/tags/v.*$`) | Pitfall 1 | If `regress` rejects `\.` or `^`/`$` semantics differ, tests catch it. Already used in `signing.rs:86::validate_oidc_issuer` — same pattern. |

## Open Questions

1. **What's the exact freshness semantics for the cached trusted root?**
   - **What we know:** D-32-03 says "expired YYYY-MM-DD; run `nono setup --refresh-trust-root`." `TrustedRoot.tlogs[*].public_key.valid_for.end` carries an ISO 8601 string per-key.
   - **What's unclear:** Should expiry mean "all tlog keys are past their `valid_for.end`" (very rare; might never trigger) OR "the cache file is older than N days" (more useful nudge, but requires picking N)?
   - **Recommendation:** Two-layer check. (a) Hard fail-closed when no active tlog key has `valid_for.end > now` (covers actual cryptographic expiry). (b) Soft warn (stderr log, NOT error) when cache mtime > 30 days old. Both layers are user-actionable via the same recovery command. Planner can keep just (a) for first cut; (b) is a pure-additive follow-up.

2. **Should `nono setup --check-only` report cached-trust-root staleness?**
   - **What we know:** CLAUDE-discretion per CONTEXT § Claude's Discretion.
   - **What's unclear:** What format? Same as WFP service status (one-line OK/MISSING/STALE)?
   - **Recommendation:** Yes. New line in `print_check_only_summary()` at `setup.rs:818+`: "Trust root cache: <path> (refreshed YYYY-MM-DD; valid until YYYY-MM-DD)" or "Trust root cache: NOT INITIALIZED (run `nono setup --refresh-trust-root`)". Matches the existing one-liner WFP-status convention.

3. **What's the exact format for the baked-in `trust-policy.json` D-32-10 template?**
   - **What we know:** Phase 32 D-32-10 says "match the existing `crates/nono/src/trust/policy.rs` format unless that format can't express keyless identity constraints."
   - **What's unclear:** The existing `Publisher` struct has `issuer / repository / workflow / ref_pattern` fields ([VERIFIED: `bundle.rs:1076-1084` test fixture]); does it have a `--identity` regex pattern field too? Checking `trust/types.rs::Publisher` is a planner step.
   - **Recommendation:** The existing format already covers `workflow + ref_pattern`, which IS the GitHub Actions identity claim. No format extension needed — the planner just builds a populated `trust-policy.json` template.

4. **Mock Fulcio cert generation: capture-once or regenerate-on-demand?**
   - **What we know:** A6 above.
   - **What's unclear:** Whether `aws_lc_rs` can produce a cert with the right Fulcio v2 OID extensions (1.3.6.1.4.1.57264.1.12 etc).
   - **Recommendation:** Capture-once-good for first cut. If the mock cert's validity window forces a fixture rotation, escalate to on-demand generation in a follow-up.

## Environment Availability

> Phase 32 mostly depends on tools that are already validated by quick task `260509-s9m` (which verified Windows MSVC build cleanly).

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Build | ✓ (verified by 260509-s9m) | rustc 1.95.0 (workspace MSRV is 1.77) | — |
| `cargo build --workspace --release` | All test runs | ✓ | 0.37.1 builds clean | — |
| Windows host | Phase 32 H broker-verify tests | ✓ (current host is Windows 11) | — | Linux/macOS: tests gated `#[cfg(target_os = "windows")]` |
| `tuf-repo-cdn.sigstore.dev` reachability | `nono setup --refresh-trust-root` (manual operator step, Phase 32 doesn't run this in CI) | Test-machine-dependent | — | Frozen fixture for tests |
| Sigstore-signed test fixture (real Fulcio cert from `always-further/test-sk-prov`) | Existing `bundle.rs:1037` test (unchanged by Phase 32) | ✓ (committed in-tree) | 2026-02-21 | — |
| `httpmock` crate | Mock Fulcio/Rekor in keyless integration test | ✗ (NOT in `Cargo.toml` yet) | New `dev-dep` pull | None — required to ship D-32-07 |
| `target/x86_64-pc-windows-msvc/release/nono-shell-broker.exe` | Phase 31 broker-dispatch tests + manual Phase 32 H smoke | ✓ when Phase 31 Plan 04 release pipeline runs | — | Existing tests already SKIP cleanly when missing |

**Missing dependencies with no fallback:**
- `httpmock` (or planner-chosen alternative). Resolved by `cargo add --dev`.

**Missing dependencies with fallback:**
- Network access to `tuf-repo-cdn.sigstore.dev`: `nono setup --refresh-trust-root` requires it; tests use the frozen fixture (no network).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner + `#[tokio::test]` |
| Config file | `Cargo.toml` workspaces; per-crate `[dev-dependencies]` |
| Quick run command | `cargo test -p nono trust::bundle::tests::load_production_trusted_root_succeeds -- --exact` (single test); `cargo test -p nono trust::bundle::tests` (full bundle test module) |
| Full suite command | `make ci` (runs `cargo build` + `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` + `cargo fmt --all -- --check` + `cargo test --workspace`) |

### Phase Requirements → Test Map
| D-32-XX | Behavior | Test Type | Automated Command | File Exists? |
|---------|----------|-----------|-------------------|-------------|
| D-32-01 | Cache write-then-read round-trip | unit | `cargo test -p nono trust::bundle::tests::cache_round_trip -- --exact` | ❌ Wave 0 (new test in `bundle.rs` test mod) |
| D-32-01 | `nono setup --refresh-trust-root` writes the cache file at the expected path | integration | `cargo test -p nono-cli --test setup_trust_root setup_writes_cache -- --exact` | ❌ Wave 0 (new integration test file `tests/setup_trust_root.rs`) |
| D-32-02 | Frozen TUF fixture loads syntactically | unit | `cargo test -p nono trust::bundle::tests::load_production_trusted_root_succeeds -- --exact` | ✓ Test exists at `bundle.rs:877`; will pass once fixture migration lands |
| D-32-02 | `verify_bundle_with_invalid_digest` runs against frozen fixture | unit | `cargo test -p nono trust::bundle::tests::verify_bundle_with_invalid_digest -- --exact` | ✓ Test exists at `bundle.rs:914`; same migration |
| D-32-03 | Verify path makes NO network call (offline invariant) | integration | `cargo test -p nono-cli --test keyless_offline_invariant verify_does_not_phone_home -- --exact` | ❌ Wave 0 (new test file or section; uses `httpmock` + asserts the mock receives ZERO requests during verify) |
| D-32-03 | Cached root past `valid_for.end` → fail-closed `TrustVerification` | unit | `cargo test -p nono trust::bundle::tests::expired_cache_fails_closed -- --exact` | ❌ Wave 0 (new test) |
| D-32-04 | sigstore-verify version is 0.6.5 | static (lock-file check) | `grep -E '^version = "0\.6\.5"' Cargo.lock | wc -l` (within `[[package]] name = "sigstore-verify"` block) | ✓ Existing |
| D-32-05 | First-run (cache absent) → fail-closed `TrustPolicy` with recovery hint | unit | `cargo test -p nono trust::bundle::tests::missing_cache_fails_closed -- --exact` | ❌ Wave 0 (new test) |
| D-32-06 | Frozen fixture file exists at expected path and parses | unit | `cargo test -p nono trust::bundle::tests::frozen_fixture_loads -- --exact` | ❌ Wave 0 (new helper test ensuring `crates/nono/tests/fixtures/trust-root-frozen.json` is committed) |
| D-32-07 | Mock Fulcio + Rekor produce a Bundle that the existing verify path accepts | integration | `cargo test -p nono-cli --test keyless_sign keyless_sign_then_verify_roundtrip -- --exact` | ❌ Wave 0 (new test file) |
| D-32-08 | `nono trust verify --keyless` without `--issuer` fails fast | integration | `cargo test -p nono-cli --test trust_verify_args missing_issuer_fails_closed -- --exact` | ❌ Wave 0 |
| D-32-08 | `nono trust verify --keyless` without `--identity` fails fast | integration | `cargo test -p nono-cli --test trust_verify_args missing_identity_fails_closed -- --exact` | ❌ Wave 0 |
| D-32-08 | `--identity REGEX` matches via `regress::Regex` | unit | `cargo test -p nono-cli trust_cmd::tests::identity_regex_matches -- --exact` | ❌ Wave 0 (new test in `trust_cmd.rs` test mod) |
| D-32-08 | SAN-mismatch via `--identity REGEX` (no match) → fail-closed | integration | `cargo test -p nono-cli --test keyless_sign san_mismatch_fails_closed -- --exact` | ❌ Wave 0 |
| D-32-09 | `discover_oidc_token` error message names `--keyref` | unit | `cargo test -p nono-cli trust_cmd::tests::oidc_error_suggests_keyref -- --exact` | ❌ Wave 0 |
| D-32-11 | Phase 28 `query_authenticode_status` is callable from broker-dispatch site (Windows-only) | unit | `cargo test -p nono-cli --test exec_identity_windows current_exe_authenticode_self_introspection_succeeds -- --exact` | ❌ Wave 0 (new test) |
| D-32-12 | Authenticode mismatch between `nono.exe` and `broker.exe` → broker dispatch fails fast | integration | `cargo test -p nono-cli --test broker_authenticode broker_subject_mismatch_refuses_spawn -- --exact` | ❌ Wave 0 (new test, Windows-only, uses two pre-signed test artifacts with DIFFERENT subjects — see Wave 0 Gaps) |
| D-32-12 | Authenticode-`Unsigned` broker → fail-closed | integration | `cargo test -p nono-cli --test broker_authenticode unsigned_broker_refuses_spawn -- --exact` | ❌ Wave 0 |
| D-32-12 | Dev-build install-layout detector skips broker verify | unit | `cargo test -p nono-cli exec_strategy_windows::launch::tests::is_dev_build_layout_detection -- --exact` | ❌ Wave 0 |
| D-32-13 | Authenticode-match between two same-signer binaries → broker dispatch succeeds | integration | `cargo test -p nono-cli --test broker_authenticode same_signer_succeeds -- --exact` | ❌ Wave 0 (uses Phase 31 release-pipeline-built broker as the same-signer fixture) |
| D-32-14 | No cache: every dispatch calls `query_authenticode_status` afresh | integration | `cargo test -p nono-cli --test broker_authenticode every_dispatch_re_verifies -- --exact` | ❌ Wave 0 (uses a counting wrapper around `query_authenticode_status` in test mode) |
| D-32-15 | `crates/nono/` byte-identicality maintained except documented changes | static | `git diff main -- crates/nono/src/ | grep -E '^[+-]' | grep -vE '^[+-]{3}'` (manual review per ADR template) | (manual; Phase 28 test_upstream_drift.sh covers similar) |

### Sampling Rate
- **Per task commit:** `cargo test -p nono trust::bundle::tests` (~6s) + `cargo test -p nono-cli trust_cmd::tests` (~5s) — fast feedback during development
- **Per wave merge:** `cargo test -p nono` + `cargo test -p nono-cli --test trust_verify_args --test keyless_sign --test broker_authenticode --test exec_identity_windows`
- **Phase gate:** `make ci` must pass on Windows host before `/gsd-verify-work`. The existing `audit_attestation` integration suite (4 tests) MUST stay green per D-32-16.

### Wave 0 Gaps
- [ ] `crates/nono/tests/fixtures/trust-root-frozen.json` — captured-once-good frozen TUF fixture (~30-100 KB JSON; covers D-32-02, D-32-06)
- [ ] `crates/nono/src/trust/mod.rs` — new `#[cfg(test)] pub(crate) fn load_test_trusted_root() -> Result<TrustedRoot>` helper (D-32-15 #2)
- [ ] `crates/nono-cli/tests/keyless_sign.rs` — new integration test file with mock Fulcio + Rekor (D-32-07)
- [ ] `crates/nono-cli/tests/setup_trust_root.rs` — new integration test for `nono setup --refresh-trust-root` (D-32-01)
- [ ] `crates/nono-cli/tests/broker_authenticode.rs` — new Windows-only integration test for D-32-11..14 (uses Phase 31 release pipeline broker artifact)
- [ ] `crates/nono-cli/tests/keyless_offline_invariant.rs` — new integration test asserting verify-is-offline (D-32-03 invariant)
- [ ] `crates/nono-cli/tests/trust_verify_args.rs` — new integration test for `--issuer` / `--identity` flag fail-closed (D-32-08)
- [ ] Two pre-signed test artifacts with DIFFERENT publisher subjects (for D-32-12 mismatch test). One option: ship a 2KB stub binary signed by a self-generated test CA, plus a second stub signed by a different test CA, both committed to `crates/nono-cli/tests/fixtures/`. (Alternative: gen at test-time with `signtool` if present; SKIPs cleanly otherwise.)
- [ ] Framework install: `cargo add --dev --package nono-cli httpmock@0.7` (or planner-chosen mock-server crate)

## Security Domain

`security_enforcement` is enabled (the project's whole reason for existing). All four work streams are security-critical.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (OIDC token discovery; broker subject verification) | sigstore-sign 0.6.5 + existing `validate_oidc_issuer` (`signing.rs:86`) for issuer pinning; Phase 28 chain walker for broker subject |
| V3 Session Management | yes (broker dispatch is per-session; no caching of trust state per D-32-14) | No new code — D-32-14 explicitly says no cache, no race window |
| V4 Access Control | yes (verify is the access control for keyless-signed content) | sigstore-verify 0.6.5 + new SAN regex post-check |
| V5 Input Validation | yes (`--identity` regex input; cached TUF root JSON; bundle JSON; Authenticode subject string) | `regress::Regex::new(...)` for the regex (already used at `signing.rs:86`); existing `sanitize_for_terminal` already strips control chars from Authenticode subject (`exec_identity_windows.rs:312`) — Phase 32 reuses it via `query_authenticode_status` directly |
| V6 Cryptography | yes (TUF root signature verification; bundle DSSE signature; Authenticode WinTrust chain) | All delegated to upstream: `sigstore-verify` 0.6.5 + `WinVerifyTrust` Win32 API. Never hand-roll crypto. |
| V8 Data Protection | yes (cached trust root must not be writable by attackers) | Cache is under `<nono_home_dir()>/.nono/` which is per-user; standard Unix file perms / Windows ACL inheritance applies. NOT under `Program Files\` (which would require admin to refresh — D-32-01 explicitly per-user). |
| V14 Configuration | yes (no escape-hatch flags; D-32-12 forbids env-var override) | Defense-in-depth: even if an attacker controls env vars, broker-verify gate stays on. |

### Known Threat Patterns for {Sigstore + Windows broker dispatch}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stale cached TUF root used to verify an attacker-signed bundle | Spoofing | Hard expiry gate (D-32-03) on `valid_for.end`; user-actionable recovery message names the exact command to refresh |
| First-run verify with no cache (cold-start) silently allowing | Tampering | Hard fail-closed (D-32-05) — no permissive default |
| OIDC issuer impersonation (e.g., `https://gitlab.com.evil.example`) | Spoofing | Reuse existing `validate_oidc_issuer` (`signing.rs:86`) which uses URL component parsing, not string comparison |
| `--identity REGEX` allows over-broad pattern (e.g., `.*`) that matches any signer | Tampering | Documentation; cookbook `--identity` examples (per CONTEXT § Specifics) lock down the canonical pattern shape `^https://github\.com/<org>/<repo>/\.github/workflows/<file>@refs/tags/v.*$` |
| Broker swap: attacker drops a sibling `nono-shell-broker.exe` signed by a different publisher | Spoofing | D-32-13 self-trust-anchor: subject + thumbprint must match `nono.exe`'s own |
| Broker swap with same publisher (e.g., compromised Microsoft cert): match passes | Spoofing | Out of scope; relies on Authenticode chain-of-trust + Microsoft's revocation pipeline (D-32-14: verify on every dispatch ensures revoked certs are caught at next launch — Authenticode chain check fails when CRL/revocation triggers, returning `InvalidSignature`) |
| Authenticode chain check failure on `nono.exe` itself (e.g., disk corruption) → broker spawn refused | Denial of Service | Acceptable trade-off per CLAUDE.md fail-secure principle; recovery is "reinstall nono" |
| TOCTOU between `query_authenticode_status(broker)` and `CreateProcessW(broker)` (attacker swaps the file in the gap) | Tampering | The PE file is mapped readonly at `CreateProcessW` time; if the attacker has write access to `Program Files\nono\` they already have admin and can do worse. Acceptable trade-off; no easy structural mitigation. |
| Cached trust-root file modified by malware (escalating malicious sigs to "trusted") | Tampering | The cache file is per-user (NOT machine-wide). Compromising it requires user-level access; if attacker has user-level access, they can already MITM the user's `nono trust verify` invocations. Defense-in-depth: cache lives under `<nono_home_dir()>` which Phase 27.1 D-27.1-08 already plumbs through `NONO_TEST_HOME` for test isolation. |
| Mock Fulcio/Rekor cert leaks into production verify path | Tampering | Mock cert lives in `crates/nono-cli/tests/fixtures/` — `[dev-dependencies]` only. Frozen production fixture in `crates/nono/tests/fixtures/` is signed by Sigstore production CAs, NOT by the mock CA. Cross-contamination would require shipping test fixtures into a release artifact, which the release pipeline doesn't do. |

## Sources

### Primary (HIGH confidence)
- `sigstore-trust-root-0.6.5/src/trusted_root.rs` (lines 1-185) — verified locally; `TrustedRoot` derives `Serialize, Deserialize`; `from_json` and `from_file` are sync constructors; `production()` is the only async constructor.
- `sigstore-trust-root-0.6.5/src/tuf.rs` (full file) — verified locally; embedded production root constants, `TufConfig::production()`, `TufConfig::offline()`, sigstore-rs's own cache directory layout.
- `sigstore-sign-0.6.5/src/sign.rs` (lines 1-230) — verified locally; `SigningContext::with_config(SigningConfig)` is the test-injection seam; `SigningConfig` has `fulcio_url`/`rekor_url` as `String` fields directly.
- `sigstore-verify-0.6.5/src/verify.rs` (lines 1-150, 270-310) — verified locally; `VerificationPolicy.identity` is `Option<String>` with EXACT-EQUALITY semantics (line 277); `VerificationPolicy::skip_certificate_chain()` exists for testing.
- `crates/nono/src/trust/bundle.rs` (full file) — read end-to-end; current state of all rewrite targets.
- `crates/nono/src/trust/mod.rs` (full file) — read; current re-exports surface.
- `crates/nono-cli/src/trust_cmd.rs` lines 1-1050 — read; current keyless flow + verify flow + `discover_oidc_token` shape.
- `crates/nono-cli/src/setup.rs` (full file) — read; current SetupRunner shape.
- `crates/nono-cli/src/exec_identity_windows.rs` (lines 1-313) — read; Phase 28 chain-walker primitives confirmed callable on `current_exe()`.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` lines 1240-1450, 2168-2350 — read; broker dispatch site at `:1262`+ (NOT `:2173+` as CONTEXT cites — that's the test module).
- `crates/nono-cli/src/cli.rs` lines 1985-2710 — read; current `SetupArgs` and `TrustVerifyArgs` definitions confirm `--issuer` / `--identity` / `--refresh-trust-root` need to be ADDED.
- `crates/nono-cli/Cargo.toml` (full file) — read; existing `test-trust-overrides` feature flag at line 36 + sigstore-sign 0.6.5 pin at line 59.
- `crates/nono/Cargo.toml` (full file) — read; sigstore-verify 0.6.5 pin at line 38; no chrono dep.
- `Cargo.lock` lines 2982-3000 — read; sigstore-trust-root 0.6.5 confirmed.
- `.planning/quick/260509-s9m-verify-that-the-sigstore-functionality-i/260509-s9m-SUMMARY.md` (full file) — read; root-cause "0 valid signatures of required 3" confirmed.
- `.planning/phases/31-broker-process-architecture-shell-01/31-04-SUMMARY.md` lines 1-80 — read; Phase 31 Plan 04 release pipeline confirmed signs both binaries with the SAME identity.
- `.planning/phases/28-authenticode-chain-walker-subject-extraction/28-01-AUDC-PLAN.md` lines 1-80 — read; Phase 28 chain-walker primitives confirmed available + tested.
- `.planning/REQUIREMENTS.md` lines 250-290 — read; REQ-AUDC-01..03 acceptance criteria establish the chain-walker contract.
- `docs/cli/development/windows-poc-handoff.mdx` lines 155-200 — read; existing cookbook block-net prereq pattern.

### Secondary (MEDIUM confidence)
- Sigstore TUF repository `https://tuf-repo-cdn.sigstore.dev/` is the source of truth — confirmed in CONTEXT § Canonical References AND in `sigstore-trust-root-0.6.5/src/tuf.rs:48`.
- `tough` crate version 0.21.0 (TUF protocol Rust client used by sigstore-rs) — confirmed in Cargo.lock.

### Tertiary (LOW confidence)
- None. All claims in this research were verified against either local source code or Cargo.lock.

## Project Constraints (from CLAUDE.md)

- **No `.unwrap()` / `.expect()` in production code** — `clippy::unwrap_used` enforced. Phase 32 unsafe-block additions (none planned beyond Phase 28's existing) MUST follow the SAFETY-comment pattern (`exec_identity_windows.rs:120-200`).
- **Fail-secure on any error** — D-32-03 / D-32-05 / D-32-09 / D-32-12 all comply. NEVER fall back to a permissive default.
- **Path security: validate/canonicalize before applying capabilities** — `<nono_home_dir()>/.nono/trust-root/` is constructed via `PathBuf::join`, which avoids string-based path manipulation; `nono_home_dir()` is already path-canonicalized per Phase 27.1.
- **Tests modifying env vars must save/restore (parallel test execution)** — Phase 32's keyless integration test uses the same `setup_isolated_home()` + `EnvVarGuard::set_all` pattern as `audit_attestation.rs:38-80` (verified). New test files MUST follow this convention.
- **No `#[allow(dead_code)]` to mask unused code** — If `load_test_trusted_root()` is unused after the 2 failing-test migration, remove it. (It WILL be used; both failing tests reference it.)
- **D-19 invariant** — `crates/nono/` stays byte-identical except for the explicitly-documented changes enumerated in D-32-15. The drift script at `tests/integration/test_upstream_drift.sh:257` already monitors `load_production_trusted_root` — see Pitfall 4 for documentation strategy.
- **Commit DCO sign-off required** — `Signed-off-by: Name <email>` on every commit.

## Metadata

**Confidence breakdown:**
- Standard stack & library APIs: **HIGH** — verified against on-disk crate source
- Architecture patterns: **HIGH** — patterns lifted directly from existing code or 1:1 mapped to upstream API surface
- Pitfalls: **HIGH** for #1 (sigstore-verify exact-equality), **MEDIUM** for #6 (install-layout detector — based on assumption A4); rest **HIGH**
- Validation architecture: **HIGH** — every D-32-XX maps to a concrete test command
- Phase 28 chain-walker reuse: **HIGH** — existing primitives confirmed callable, sanitize_for_terminal already integrated
- Cookbook integration: **HIGH** — existing block-net prereq pattern provides 1:1 template

**Research date:** 2026-05-09
**Valid until:** 2026-06-09 (~30 days; sigstore-rs ecosystem is fast-moving but Phase 32 stays on a pinned 0.6.5 so external drift is bounded)
