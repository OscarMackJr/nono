---
phase: 49
phase_name: sigstore-trust-root-poc-resilience-from-file-flag-release-as
researched: 2026-05-21
spec_locked_via: 49-SPEC.md
context_locked_via: 49-CONTEXT.md
---

# Phase 49 Research

## Executive Summary

- **Three parallel-safe plans, surfaces disjoint** (per D-49-A1/A2): Plan 49-01 `crates/nono-cli/src/{cli,setup}.rs` + new integration test, Plan 49-02 `.github/workflows/release.yml`, Plan 49-03 `.planning/templates/` + `scripts/` + `docs/cli/development/windows-poc-handoff.mdx`. None depend on each other's runtime artifacts.
- **`check_trusted_root_freshness` is currently PRIVATE** at `crates/nono/src/trust/bundle.rs:247` (no `pub` / no `pub(crate)`). Plan 49-01 MUST factor a `pub` (or at minimum `pub(crate)` re-exported via `crates/nono/src/trust/mod.rs`) helper that BOTH `load_production_trusted_root` and the new `--from-file` path call. SPEC explicitly says "no new schema validator" — but does NOT prohibit visibility widening on an existing helper, so this is in-scope. Minimal change: flip the existing `fn check_trusted_root_freshness` to `pub fn` and add a one-line `pub use` in `trust/mod.rs`. Alternative (factor into a wrapper) is heavier and not needed.
- **Fixture surprise — D-49-D1 expired mutation must ADD a `valid_for.end`, not modify one.** The frozen fixture's tlogs only have `validFor.start`, no `end`. Per `check_trusted_root_freshness` at `bundle.rs:282-283` ("missing end = no expiry asserted; treat as active"), a tlog with no `end` is always fresh. The expired test fixture must INSERT `"end": "1970-01-01T00:00:00Z"` into each tlog's `publicKey.validFor` object. Also note: the JSON uses camelCase (`validFor`, `publicKey`, `rawBytes`) — `serde` lowercases to `valid_for`/`public_key` internally, but raw byte mutation in JSON must target the camelCase keys. There are 2 tlogs in the fixture; both must get the `"end"` field for the freshness check to fail (any active tlog passes the gate).
- **`assert_cmd` pattern is established and uses `tests/common/` infra.** `crates/nono-cli/tests/common/` directory exists with `test_env::{lock_env, EnvVarGuard}` primitives that serialize env-mutating tests under a global mutex. `auto_pull_e2e_linux.rs` shows the canonical pattern. The new `setup_from_file.rs` test file MUST use these primitives — Phase 44 REQ-REVIEW-FU-01 D-44-E6 enforces it (note: also exists a `setup_trust_root.rs` test file already — planner should grep that file for the closest existing pattern before drafting).
- **Release.yml insertion points are tight and well-bounded.** SHA256SUMS aggregation block at `release.yml:315-326` (5 conditional `sha256sum` extensions); `softprops/action-gh-release` `files:` block at lines `334-340` (5 entries). Plan 49-02 adds: (1) a pre-aggregation step that does `cp crates/nono/tests/fixtures/trust-root-frozen.json artifacts/trusted_root.json` + SHA-256 byte-identity assert; (2) a `sha256sum trusted_root.json >> SHA256SUMS.txt` line in the aggregation block; (3) one `artifacts/trusted_root.json` line in the `files:` block.
- **POC handoff doc has TWO stale references to fix, not one.** The "Run once after install" block at `windows-poc-handoff.mdx:166-180` recommends `nono setup --refresh-trust-root` (which is currently broken on stale anchors); the "Known issue" subsection at `windows-poc-handoff.mdx:182-220` documents the `Invoke-WebRequest` workaround AND cites `sigstore-verify 0.6.5` AND links to non-existent `P32-DEFER-005` / `deferred-items.md`. Per Claude's-Discretion section in CONTEXT, planner SHOULD sweep both blocks for consistency (the 166-180 block stays but adds an "or `--from-file <PATH>`" alternative path; the 182-220 block is the primary rewrite).
- **Cross-target clippy is MANDATORY for Plan 49-01.** Both `cli.rs` and `setup.rs` contain `#[cfg(target_os = "windows")]` blocks (5+ in setup.rs alone — verified). `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin` MUST run, or REQ goes PARTIAL with explicit live-CI deferral per `.planning/templates/cross-target-verify-checklist.md`.
- **All decisions are LOCKED**; this is an implementation-bookkeeping phase. Expected open questions: zero.

## REQ-POC-TRUST-01: `--from-file` Flag Implementation Map

### cli.rs SetupArgs surface (lines 2341-2387)

Current shape — verified at `crates/nono-cli/src/cli.rs:2341-2387`:

```rust
#[derive(Parser, Debug)]
#[command(disable_help_flag = true)]
pub struct SetupArgs {
    /// Only verify installation and sandbox support, don't create files
    #[arg(long, help_heading = "OPTIONS")]
    pub check_only: bool,

    // ... 5 Windows-only WFP flags ...

    /// Refresh the cached Sigstore trusted root from https://tuf-repo-cdn.sigstore.dev (per-user, no admin required)
    #[arg(long, help_heading = "OPTIONS")]
    pub refresh_trust_root: bool,

    /// Generate example user profiles in ~/.config/nono/profiles/
    #[arg(long, help_heading = "OPTIONS")]
    pub profiles: bool,

    // ... etc ...
}
```

**Insertion point for `--from-file`:** Immediately AFTER the `refresh_trust_root` flag at line 2370, BEFORE `profiles` at line 2372. New field shape:

```rust
/// Populate the cached Sigstore trusted root from a local JSON file (skips network fetch).
#[arg(long, value_name = "PATH", help_heading = "OPTIONS", conflicts_with = "refresh_trust_root")]
pub from_file: Option<PathBuf>,
```

**Conflicts_with attribute style:** clap v4 derive — uses field-name string `"refresh_trust_root"`. (No existing `conflicts_with` usage was inventoried in the surrounding lines; planner greps the rest of `cli.rs` at plan-open for prior art if any.)

### setup.rs phase-step surface

**Setup struct (`SetupRunner`) at `setup.rs:20-29` — verified field list:**

```rust
pub struct SetupRunner {
    check_only: bool,
    #[cfg(target_os = "windows")] register_wfp_service: bool,
    #[cfg(target_os = "windows")] install_wfp_service: bool,
    #[cfg(target_os = "windows")] install_wfp_driver: bool,
    #[cfg(target_os = "windows")] start_wfp_service: bool,
    #[cfg(target_os = "windows")] start_wfp_driver: bool,
    refresh_trust_root: bool,
    generate_profiles: bool,
    show_shell_integration: bool,
}
```

**Insertion:** Add `from_file: Option<PathBuf>` after `refresh_trust_root`.

**Setup::from_args wiring at `setup.rs:31-49` — verified:**

```rust
impl SetupRunner {
    pub fn new(args: &SetupArgs) -> Self {
        Self {
            check_only: args.check_only,
            // ... 5 Windows-only fields copied ...
            refresh_trust_root: args.refresh_trust_root,
            generate_profiles: args.profiles,
            show_shell_integration: args.shell_integration,
        }
    }
```

**Insertion:** Add `from_file: args.from_file.clone(),` after the `refresh_trust_root` line. (`.clone()` because `args.from_file: Option<PathBuf>` and `args` is `&SetupArgs`.)

**Setup::run branch point at `setup.rs:91-93` — verified:**

```rust
if !self.check_only && self.refresh_trust_root {
    self.refresh_trust_root_step()?;
}
```

**Insertion:** Add a sibling branch BELOW the `refresh_trust_root` branch (clap-mutex guarantees they cannot both be true at the same invocation, so order is cosmetic):

```rust
if !self.check_only {
    if let Some(path) = self.from_file.as_ref() {
        self.from_file_step(path)?;
    }
}
```

(Or fold into a single `if !self.check_only` block alongside `refresh_trust_root` for diff minimality — planner chooses.)

**`refresh_trust_root_step()` shape at `setup.rs:820-860` — verified for mirror pattern:**

```rust
fn refresh_trust_root_step(&self) -> Result<()> {
    let cache_dir = crate::config::nono_home_dir()?
        .join(".nono")
        .join("trust-root");
    std::fs::create_dir_all(&cache_dir).map_err(NonoError::Io)?;

    println!(
        "[{}/{}] Refreshing Sigstore trusted root...",
        self.refresh_trust_root_phase_index(),
        self.total_phases()
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| NonoError::Setup(format!("tokio runtime: {e}")))?;
    let trusted_root = rt
        .block_on(nono::trust::TrustedRoot::production())
        .map_err(|e| NonoError::Setup(format!(
            "Failed to fetch Sigstore trusted root from \
             https://tuf-repo-cdn.sigstore.dev: {e}"
        )))?;

    let json = serde_json::to_string_pretty(&trusted_root)
        .map_err(|e| NonoError::Setup(format!("serialize trusted root: {e}")))?;
    let cache_path = cache_dir.join("trusted_root.json");
    std::fs::write(&cache_path, &json).map_err(NonoError::Io)?;

    println!("  * Sigstore trusted root cached at {}", cache_path.display());
    println!();
    Ok(())
}
```

**New `from_file_step(&self, src: &Path)` skeleton (planner refines):**

```rust
fn from_file_step(&self, src: &Path) -> Result<()> {
    let cache_dir = crate::config::nono_home_dir()?
        .join(".nono")
        .join("trust-root");
    std::fs::create_dir_all(&cache_dir).map_err(NonoError::Io)?;

    println!(
        "[{}/{}] Loading Sigstore trusted root from file...",
        self.refresh_trust_root_phase_index(),  // SAME index (clap-mutex guarantees mutex)
        self.total_phases()
    );

    // Step 1: validate via existing pipeline (NO new schema validator).
    let trusted_root = nono::trust::bundle::load_trusted_root(src)
        .map_err(|e| NonoError::Setup(format!("invalid trusted root at {}: {e}", src.display())))?;

    let cache_path = cache_dir.join("trusted_root.json");

    // Step 2: freshness gate (D-32-03 expiry — REUSED, NOT re-implemented).
    // Requires bumping check_trusted_root_freshness from private → pub in bundle.rs.
    nono::trust::bundle::check_trusted_root_freshness(&trusted_root, &cache_path)
        .map_err(|e| NonoError::Setup(format!("trusted root failed freshness check: {e}")))?;

    // Step 3: byte-identical copy (D-49-B1) with best-effort cleanup (D-49-B2).
    if let Err(e) = std::fs::copy(src, &cache_path) {
        let _ = std::fs::remove_file(&cache_path);
        return Err(NonoError::Io(e));
    }

    println!("  * Sigstore trusted root cached at {}", cache_path.display());
    println!("  * Source: {}", src.display());  // D-49-B3 breadcrumb
    println!();
    Ok(())
}
```

**Phase-index threading:** Per `setup.rs:719-740` (total_phases) and `setup.rs:795-808` (refresh_trust_root_phase_index), the existing arithmetic counts `usize::from(self.refresh_trust_root)`. Per D-49-A2 + clap-mutex, the planner extends the arithmetic to count `usize::from(self.refresh_trust_root || self.from_file.is_some())` — both flags share the same slot. Cosmetic ordering only; planner sweeps the 6 sites (`setup.rs:719, 723, 740, 744, 795, 820`).

### check_trusted_root_freshness accessibility

**VERIFIED:** `crates/nono/src/trust/bundle.rs:247` declares `fn check_trusted_root_freshness(...)` with NO visibility modifier → private to the `bundle` module.

```rust
fn check_trusted_root_freshness(root: &TrustedRoot, cache_path: &std::path::Path) -> Result<()> {
    // ... 60 lines of WR-05 fail-closed format guard + tlog iteration ...
}
```

**Recommendation: flip to `pub fn` and add `pub use` in `crates/nono/src/trust/mod.rs`.**

- **Minimal SPEC-respecting choice.** SPEC.md says "Reuse of the existing `nono::trust::bundle::load_trusted_root` + `check_trusted_root_freshness` validation pipeline for the new flag — no new schema validator, no new code paths in `crates/nono`." Re-exporting an existing private fn is NOT a new code path — it's exposure of an existing one. **In scope.**
- **Alternative (rejected):** Factor a `pub fn validate_trusted_root(path: &Path) -> Result<TrustedRoot>` wrapper that internally calls `load_trusted_root + check_trusted_root_freshness`. This IS technically "new code in `crates/nono`" and could draw a SPEC-out-of-scope flag at review time. Avoid unless reviewer pushback materializes.
- **Resulting signature:** `pub fn check_trusted_root_freshness(root: &TrustedRoot, cache_path: &Path) -> Result<()>` — caller passes the destination cache path (used for the error message's path display), not the source path. Caller (`from_file_step`) computes `cache_path = cache_dir.join("trusted_root.json")` before calling.
- **Verification gate:** `cargo doc --no-deps -p nono` (or rustdoc-warn-on-broken-internal-links) confirms the new pub fn doesn't accidentally drop a `pub(crate)` doc link.

### Integration test pattern (D-49-D2)

**VERIFIED:** `crates/nono-cli/tests/common/` directory exists. `crates/nono-cli/tests/auto_pull_e2e_linux.rs` lines 16-31 confirm the canonical pattern:

```rust
#![cfg(target_os = "linux")]  // for OS-specific; NEW test should be cross-platform — no cfg gate
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::process::Command;
use tempfile::TempDir;

mod common;
use common::test_env::{lock_env, EnvVarGuard};

const NONO_BIN: &str = env!("CARGO_BIN_EXE_nono");
```

**Note:** This file uses raw `std::process::Command` + `env!("CARGO_BIN_EXE_nono")`, NOT `assert_cmd`. CONTEXT.md asserts `assert_cmd` is "already in the workspace via existing tests" — planner should verify at plan-open via `grep -rn 'assert_cmd' crates/nono-cli/tests/ crates/nono-cli/Cargo.toml` before assuming. If `assert_cmd` is not yet a dev-dep, either (a) add it to `crates/nono-cli/Cargo.toml` `[dev-dependencies]`, or (b) use the existing `std::process::Command` + `CARGO_BIN_EXE_nono` pattern (zero new deps).

**Existing related test file:** `crates/nono-cli/tests/setup_trust_root.rs` ALREADY EXISTS (verified in the directory listing). Plan 49-01 SHOULD add the new test cases to that file rather than create a fresh `setup_from_file.rs` — adjacent test cases live together. Planner inventories `setup_trust_root.rs` at plan-open to decide: extend vs new file.

**Per-test scaffold pattern (mirrors auto_pull_e2e_linux.rs):**

```rust
#[test]
fn from_file_happy_path_writes_byte_identical_cache() {
    let _env_lock = lock_env();
    let tmp = TempDir::new().unwrap();
    let _home_guard = EnvVarGuard::set("NONO_TEST_HOME", tmp.path());
    let _xdg_guard = EnvVarGuard::set("XDG_CONFIG_HOME", tmp.path());

    // Copy the frozen fixture to a per-test path (so the input path is well-defined).
    let src = tmp.path().join("input.json");
    std::fs::copy("../nono/tests/fixtures/trust-root-frozen.json", &src).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_nono"))
        .args(["setup", "--from-file"])
        .arg(&src)
        .status()
        .unwrap();
    assert!(status.success());

    let cache = tmp.path().join(".nono/trust-root/trusted_root.json");
    assert_eq!(
        std::fs::read(&src).unwrap(),
        std::fs::read(&cache).unwrap(),
        "cache bytes must equal source bytes",
    );
}
```

**NONO_TEST_HOME wiring:** Verified as the right env var (CONTEXT.md confirms; auto_pull_e2e_linux.rs uses it). The `nono_home_dir()` helper in `crates/nono-cli/src/config/mod.rs` honors `NONO_TEST_HOME` per existing Phase 44 work.

### Fixture mutation surface (D-49-D1)

**Confirmed shape:** `crates/nono/tests/fixtures/trust-root-frozen.json` — 2 tlogs in the `tlogs` array (one rekor at `rekor.sigstore.dev`, one at `log2025-1.rekor.sigstore.dev`).

**Critical surprise:** Both tlogs have ONLY `validFor.start`, no `validFor.end`. The frozen fixture is verified at `bundle.rs:247-305`:

```rust
.and_then(|v| v.end.as_ref())
.map(|end| { /* compare to today */ })
.unwrap_or(true)  // missing end = no expiry asserted; treat as active
```

→ The current fixture's tlogs are ALWAYS fresh. To force `check_trusted_root_freshness` to fail (D-49-D1 expired case), the mutation must ADD `validFor.end` to both tlogs:

```json
"validFor": {
  "start": "2021-01-12T11:53:27Z",
  "end": "1970-01-01T00:00:00Z"   // INSERTED
}
```

**JSON key casing:** The on-disk JSON uses camelCase (`validFor`, `publicKey`, `mediaType`) per the `sigstore_verify` proto-generated serde-renamed names. Mutation logic must target camelCase keys (not snake_case).

**Mutation logic location:** Test helper in `crates/nono-cli/tests/common/` (e.g., `common::fixtures::mutate_trust_root_for_expired_test(src: &Path, dst: &Path)`). Implementation: `serde_json::Value` round-trip, iterate `["tlogs"].as_array_mut()`, inject `["publicKey"]["validFor"]["end"] = json!("1970-01-01T00:00:00Z")` on each tlog. Both tlogs must be mutated (per the freshness gate's `any_active` logic — any one active tlog passes).

**Malformed cases:**
- **Truncation:** Read source bytes, write first 100 bytes to dst → JSON parse fails inside `TrustedRoot::from_file`.
- **Quote-flip:** Read source bytes as `Vec<u8>`, find first `"` byte (likely at offset 1 — `{\n  "mediaType":...`), replace with `'` → distinct parse error class.

## REQ-POC-TRUST-02: Release Asset Bundling Map

### Current `.github/workflows/release.yml:315-340` — verified

```yaml
          find . -name "*.deb" -exec mv {} . \;
          sha256sum *.tar.gz > SHA256SUMS.txt
          if ls *.zip >/dev/null 2>&1; then
            sha256sum *.zip >> SHA256SUMS.txt
          fi
          if ls *.msi >/dev/null 2>&1; then
            sha256sum *.msi >> SHA256SUMS.txt
          fi
          if ls *.exe >/dev/null 2>&1; then
            sha256sum *.exe >> SHA256SUMS.txt
          fi
          cat SHA256SUMS.txt

      - name: Create GitHub Release
        uses: softprops/action-gh-release@153bb8e04406b158c6c84fc1615b65b24149a1fe # v2
        with:
          tag_name: ${{ env.RELEASE_TAG }}
          draft: false
          generate_release_notes: true
          files: |
            artifacts/*.tar.gz
            artifacts/*.zip
            artifacts/*.msi
            artifacts/*.exe
            artifacts/*.deb
            artifacts/SHA256SUMS.txt
```

### Plan 49-02 minimal-diff insertion

**Step 1: New CI step BEFORE the SHA256SUMS aggregation block (insert ABOVE line 315, after the `find -name "*.deb"` mv):**

```yaml
      - name: Bundle Sigstore trusted_root.json as release asset
        run: |
          set -euo pipefail
          SRC=crates/nono/tests/fixtures/trust-root-frozen.json
          DST=artifacts/trusted_root.json
          cp "$SRC" "$DST"
          # Byte-identity assert (D-49-B1 provenance chain).
          SRC_SHA=$(sha256sum "$SRC" | cut -d' ' -f1)
          DST_SHA=$(sha256sum "$DST" | cut -d' ' -f1)
          if [ "$SRC_SHA" != "$DST_SHA" ]; then
            echo "ERROR: trusted_root.json byte-identity assert failed" >&2
            echo "  src ($SRC): $SRC_SHA" >&2
            echo "  dst ($DST): $DST_SHA" >&2
            exit 1
          fi
          echo "trusted_root.json byte-identity verified: $SRC_SHA"
```

**`set -euo pipefail`** is load-bearing (per F-02-04). Bare `if` with `[ ... ]` returns 0 even on syntax errors without `-e`; the pipe `| cut` masks failures without `-o pipefail`.

**Step 2: Extend SHA256SUMS aggregation block at line 326. Insert BEFORE `cat SHA256SUMS.txt`:**

```yaml
          if ls trusted_root.json >/dev/null 2>&1; then
            sha256sum trusted_root.json >> SHA256SUMS.txt
          fi
```

**Step 3: Extend `files:` block at line 340. Insert a new line:**

```yaml
            artifacts/trusted_root.json
```

(Either before or after `artifacts/SHA256SUMS.txt` — placement cosmetic.)

**Working-directory note:** The SHA256SUMS block runs in `artifacts/` (the `find . -name "*.deb"` works on `.` and the `sha256sum *.tar.gz` patterns are unqualified). The new byte-identity step at Plan 49-02 Step 1 runs at repo root (uses `crates/nono/tests/fixtures/...` source path). Planner must reconcile: either set `working-directory: artifacts/` on Step 2 only, OR put the entire `cp + assert` inside the existing block. Recommendation: put it INSIDE the existing `runs-on: ubuntu-latest` step that prepares artifacts, just before SHA256SUMS aggregation, since that block clearly chdirs to artifacts already (the `*.tar.gz` glob pattern would otherwise fail).

## REQ-POC-TRUST-03: Cadence Template + Smoke Script + Docs Rewrite Map

### Existing template structural shape

**Source:** `.planning/templates/cross-target-verify-checklist.md` (78 lines, verified in full).

**Section structure (mirror for `sigstore-rotation-refresh.md`):**

```
# Sigstore Trust-Root Rotation Refresh Checklist

**Read this template before committing a refreshed `crates/nono/tests/fixtures/trust-root-frozen.json`.**

**Source:** Phase 49 (Sigstore TUF root rotation resilience, v2.6) — supersedes P32-DEFER-005.

---

## Scope
[When this checklist applies: trigger sources — Sigstore mailing list, blog, sigstore-rs CI failures]

## Decision Tree
[Step 1: capture command — `curl -L https://raw.githubusercontent.com/sigstore/root-signing/main/repository/trusted_root.json -o /tmp/new.json`]
[Step 2: byte-diff — `diff -u crates/nono/tests/fixtures/trust-root-frozen.json /tmp/new.json | head -50`]
[Step 3: regression test — `cargo test -p nono trust::bundle::load_test_trusted_root_smoke`]
[Step 4: smoke check via the cached-bytes script — `./scripts/verify-trust-root-cached.sh /tmp/new.json` (or `.ps1` on Windows)]
[Step 5: commit-and-tag — `git add crates/nono/tests/fixtures/trust-root-frozen.json && git commit -m "chore(trust-root): refresh frozen fixture..."`]
[Step 6: release-asset gate cross-link — points to .github/workflows/release.yml byte-identity assert step]

## Anti-Patterns (do NOT do)
[Don't refresh without regression test; don't commit a fixture that the smoke script rejects; don't ship without bumping a release tag]

## Enforcement
[Referenced from CLAUDE.md / windows-poc-handoff.mdx Known-issue subsection]
```

### Smoke script signature

**Confirmed `nono setup --from-file <PATH>` invocation shape** (matches Plan 49-01 SetupArgs surface): `nono setup --from-file <PATH>` (no other args required; `NONO_TEST_HOME` controls cache location).

**`nono trust verify` invocation shape** — planner inventories at plan-open via:

```bash
grep -rn 'fn verify\|TrustVerifyArgs\|VerifyArgs' crates/nono-cli/src/cli.rs | head -5
```

D-49-C2 says reuse existing trust-test fixtures — planner inventories at plan-open via:

```bash
ls crates/nono/tests/fixtures/*.sigstore.json crates/nono/tests/fixtures/*.toml 2>/dev/null
```

**`scripts/verify-trust-root-cached.sh` skeleton (~20 lines):**

```bash
#!/usr/bin/env bash
set -euo pipefail
if [ $# -lt 1 ]; then
  echo "usage: $0 <path-to-trusted_root.json>" >&2
  exit 2
fi
CANDIDATE="$1"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export NONO_TEST_HOME="$TMP"
export XDG_CONFIG_HOME="$TMP"
nono setup --from-file "$CANDIDATE"
# Planner fills the verify invocation per the inventoried trust fixtures.
nono trust verify <BUNDLE> <SOURCE>
echo "PASS: $CANDIDATE accepted by setup and used successfully by verify."
```

**`scripts/verify-trust-root-cached.ps1` skeleton:**

```powershell
#Requires -Version 5.1
param([Parameter(Mandatory=$true)][string]$Candidate)
$ErrorActionPreference = 'Stop'
$tmp = New-Item -ItemType Directory -Path "$env:TEMP\nono-trust-smoke-$(Get-Random)" -Force
try {
    $env:NONO_TEST_HOME = $tmp.FullName
    $env:XDG_CONFIG_HOME = $tmp.FullName
    & nono setup --from-file $Candidate
    if ($LASTEXITCODE -ne 0) { throw "nono setup failed" }
    & nono trust verify <BUNDLE> <SOURCE>
    if ($LASTEXITCODE -ne 0) { throw "nono trust verify failed" }
    Write-Host "PASS: $Candidate accepted by setup and used successfully by verify."
} finally {
    Remove-Item -Recurse -Force $tmp.FullName -ErrorAction SilentlyContinue
}
```

**Exit codes:** 0 on success, non-zero on any failure (mirrors checklist tests at acceptance criterion REQ-POC-TRUST-03 (b)). On `.sh` side, `set -e` handles propagation; on `.ps1` side, `$ErrorActionPreference = 'Stop'` + explicit `$LASTEXITCODE` checks for native commands.

### POC handoff doc rewrite source

**Confirmed location:** `docs/cli/development/windows-poc-handoff.mdx` lines 160-225 (verified by reading lines 160-225).

**Stale assertions identified:**

1. **Line 167:** "Run once after install" → `nono setup --refresh-trust-root` (the broken path). Add `--from-file` alternative.
2. **Line 184:** Heading `#### Known issue: Sigstore TUF root rotation (sigstore-verify 0.6.5)` — pinned to `sigstore-verify 0.6.5`. Rewrite to version-agnostic: `#### Known issue: Sigstore TUF root rotation`.
3. **Line 207:** Comment `# Workaround until the fork upgrades to sigstore-verify 0.6.6+ (tracked as / P32-DEFER-005 in .planning/phases/32-sigstore-integration/deferred-items.md).` — the `deferred-items.md` path no longer exists; `P32-DEFER-005` is superseded by Phase 49. Replace with: `# Phase 49 ships --from-file as the supported path; this Invoke-WebRequest workaround remains as a fallback.`
4. **Lines 209-211:** The `Invoke-WebRequest` block referencing `oscarmackjr-twg/nono/281f71ab/...` commit SHA. Replace primary recommendation with `nono setup --from-file <release-asset-url-downloaded-locally>` (recommend `Invoke-WebRequest -OutFile td.json; nono setup --from-file td.json`). Demote the direct-into-cache `Invoke-WebRequest -OutFile $cacheDir\trusted_root.json` to a "if you can't reach the GitHub Releases page" fallback.
5. **Line 218:** `--refresh-trust-root` will start working again once the dep is upgraded` — DELETE; this prose pins the doc to the dep-bump treadmill that Phase 49 exits.

**Rewrite scope:** ~40 lines edited in-place; net delta likely +5 to +15 lines (added `--from-file` recommendation block, removed dep-version pinning prose).

## Validation Architecture

This section is the input to VALIDATION.md (Nyquist Dimension 8).

### Failure modes per REQ

**REQ-POC-TRUST-01:**
- **F-01-01 clap-mutex bypass** — `nono setup --from-file <p> --refresh-trust-root` is accepted at parse time (clap `conflicts_with` missing or mis-spelled).
- **F-01-02 freshness gate bypass** — `--from-file` writes an all-tlog-keys-expired input to cache (e.g., `check_trusted_root_freshness` call omitted or its visibility-widen breaks).
- **F-01-03 schema bypass** — `--from-file` writes a malformed JSON / unparseable TrustedRoot to cache (e.g., `load_trusted_root` call omitted).
- **F-01-04 cache leak on copy failure** — partial-write cache file persists after a mid-copy IO error (D-49-B2 best-effort cleanup omitted).
- **F-01-05 stdout drift** — phase-step output diverges from `--refresh-trust-root` shape (D-49-B3 not honored: missing `Source:` line OR missing `[X/N]` header OR verb not "Loading").
- **F-01-06 cross-target clippy regression** — `cfg(target_os = "windows")` block in cli.rs or setup.rs lints-clean only on Windows host (e.g., `from_file` field is unused under cfg gates and triggers `dead_code`).
- **F-01-07 phase-index off-by-one** — `total_phases()` arithmetic counts `from_file` and `refresh_trust_root` as separate slots, breaking the displayed counter at runtime.
- **F-01-08 freshness fn still private** — researcher forgets to flip `check_trusted_root_freshness` to `pub`, build fails outside `crates/nono`.

**REQ-POC-TRUST-02:**
- **F-02-01 byte-identity drift** — `artifacts/trusted_root.json` differs from `crates/nono/tests/fixtures/trust-root-frozen.json` at release tag's commit (e.g., `cp` mangles line endings on Windows runners).
- **F-02-02 release-asset omission** — `trusted_root.json` not in `softprops/action-gh-release` `files:` glob (the new `artifacts/trusted_root.json` line missing).
- **F-02-03 SHA256SUMS omission** — `trusted_root.json` not in `SHA256SUMS.txt` (the new `sha256sum trusted_root.json >> SHA256SUMS.txt` line missing).
- **F-02-04 CI silent-pass** — assert step exists but `set -euo pipefail` (or equivalent strict-mode) is missing, so a diff silently passes (a `sha256sum | cut` pipe failure masks the failure without `-o pipefail`).
- **F-02-05 working-directory mismatch** — `cp` step runs from repo root but SHA256SUMS aggregation runs from `artifacts/`; paths don't compose.

**REQ-POC-TRUST-03:**
- **F-03-01 template absent** — `.planning/templates/sigstore-rotation-refresh.md` not committed.
- **F-03-02 smoke script absent / non-executable** — `.sh` chmod-bit missing on Unix (`git update-index --chmod=+x scripts/verify-trust-root-cached.sh` forgotten).
- **F-03-03 doc stale cross-references** — `(sigstore-verify 0.6.5)` heading / `P32-DEFER-005` cross-ref / `deferred-items.md` path still present after rewrite.
- **F-03-04 "Run once after install" inconsistency** — doc 166-180 block still recommends `--refresh-trust-root` exclusively while subsection 182-220 recommends `--from-file` as primary.
- **F-03-05 PowerShell script silent-failure** — `.ps1` script doesn't check `$LASTEXITCODE` after native command invocation, exits 0 on a failed `nono setup` call.

### Validation evidence per failure mode

| Failure mode | Validation gate | Command / assertion |
|--------------|-----------------|---------------------|
| F-01-01 | `cargo test -p nono-cli --test setup_trust_root from_file_with_refresh_rejected` | `assert!(!status.success())` + stderr contains `cannot be used with` |
| F-01-02 | `cargo test -p nono-cli --test setup_trust_root from_file_expired_fails_closed` | `assert!(!status.success())` + `!cache_path.exists()` |
| F-01-03 | `cargo test -p nono-cli --test setup_trust_root from_file_malformed_fails_closed` (truncation case) + `from_file_quote_flipped_fails_closed` | same |
| F-01-04 | `cargo test -p nono-cli --test setup_trust_root from_file_missing_path_no_partial_cache` | `!cache_path.exists()` after a failed run |
| F-01-05 | `cargo test -p nono-cli --test setup_trust_root from_file_stdout_matches_refresh_shape` | `String::from_utf8(output.stdout)` contains `[X/N] Loading...` + `Source: <path>` |
| F-01-06 | `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `--target x86_64-apple-darwin` (or PARTIAL per checklist) | exit 0 |
| F-01-07 | `cargo test -p nono-cli --test setup_trust_root from_file_phase_index_matches_refresh` | parses `[X/N]` and asserts `X == self.refresh_trust_root_phase_index()` |
| F-01-08 | `cargo build -p nono-cli` (the import would fail otherwise) | exit 0 |
| F-02-01 | CI step's `sha256sum` comparison (per release.yml insert above) | step exits non-zero on drift |
| F-02-02 | `gh release view <tag> --json assets` + `jq '.assets[].name'` contains `trusted_root.json` | post-release manual check OR CI dry-run with `actions-gh-release` |
| F-02-03 | `gh release download <tag> -p SHA256SUMS.txt && grep trusted_root.json SHA256SUMS.txt` | grep exit 0 |
| F-02-04 | Shellcheck on the new CI step + manual review of `set -euo pipefail` | shellcheck passes |
| F-02-05 | Local dry-run via `act` (if available) OR review of `working-directory:` annotations in release.yml | reviewer signoff |
| F-03-01 | `test -f .planning/templates/sigstore-rotation-refresh.md` | exit 0 |
| F-03-02 | `[[ -x scripts/verify-trust-root-cached.sh ]]` (Unix) + the `.ps1` exists | exit 0 |
| F-03-03 | `grep -E '(sigstore-verify 0\.6\.5|P32-DEFER-005|deferred-items\.md)' docs/cli/development/windows-poc-handoff.mdx` | exit 1 (no matches) |
| F-03-04 | `grep -A 5 'Run once after install' docs/cli/development/windows-poc-handoff.mdx` contains `--from-file` | exit 0 |
| F-03-05 | Manual: run `.ps1` with a corrupt `<PATH>` and verify exit code ≠ 0 | reviewer signoff |

### Verification commands (planner copies into each plan's `<verification_strategy>`)

**Plan 49-01:**
```bash
cargo test -p nono-cli --test setup_trust_root  # or setup_from_file if separate file
cargo test -p nono trust::bundle  # smoke for the visibility-widen
cargo build -p nono-cli --release
cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace -- -D warnings -D clippy::unwrap_used  # Windows host
cargo fmt --all --check
```

**Plan 49-02:**
```bash
# Local dry-run of the new step (manual, since release.yml runs on tags):
SRC=crates/nono/tests/fixtures/trust-root-frozen.json
mkdir -p /tmp/artifacts && cp "$SRC" /tmp/artifacts/trusted_root.json
diff <(sha256sum "$SRC" | cut -d' ' -f1) <(sha256sum /tmp/artifacts/trusted_root.json | cut -d' ' -f1)
shellcheck -s bash .github/workflows/release.yml  # if shellcheck supports yaml embeds; otherwise extract block
yamllint .github/workflows/release.yml
```

**Plan 49-03:**
```bash
test -f .planning/templates/sigstore-rotation-refresh.md
test -x scripts/verify-trust-root-cached.sh
test -f scripts/verify-trust-root-cached.ps1
# Negative-grep stale references:
! grep -E '(sigstore-verify 0\.6\.5|P32-DEFER-005|deferred-items\.md)' docs/cli/development/windows-poc-handoff.mdx
# Positive-grep new recommendation:
grep -E '\-\-from-file' docs/cli/development/windows-poc-handoff.mdx
# Smoke-script self-test (after Plan 49-01 lands):
./scripts/verify-trust-root-cached.sh crates/nono/tests/fixtures/trust-root-frozen.json
```

## Pitfalls & Anti-Patterns (re-confirmation only)

These are LOCKED rejections per CONTEXT.md / SPEC.md — plans MUST NOT re-introduce:

- **DO NOT round-trip-serialize on cache write** (D-49-B1 rejected — breaks release-asset byte-identity provenance chain).
- **DO NOT tmpfile+rename** (D-49-B1/B2 rejected — inconsistent with `--refresh-trust-root`'s single-shot `fs::write`).
- **DO NOT wire smoke script into PR CI** (D-49-C3 rejected — duplicates the existing `load_test_trusted_root_smoke` test for ~30s of every PR run).
- **DO NOT check in dedicated `trust-root-expired.json` / `trust-root-malformed.json`** (D-49-D1 rejected — TempDir mutation is the chosen path).
- **DO NOT bump `sigstore-verify`** (SPEC out-of-scope — Phase 49's entire point is exiting the dep-bump treadmill).
- **DO NOT touch verify path in `crates/nono`** (D-32-15 inheritance — verify is structurally + dynamically offline).
- **DO NOT bundle `trusted_root.json` into Windows MSIs** (SPEC out-of-scope — requires re-spinning MSI on every rotation).
- **DO NOT add `--force` flag or freshness-aware overwrite protection** (SPEC out-of-scope — "last writer wins" overwrite).
- **DO NOT add `jsonschema`-crate-backed validator** (SPEC out-of-scope — `TrustedRoot::from_file` deserialize IS the schema oracle).
- **DO NOT use `.unwrap()` / `.expect()` in production code** (CLAUDE.md `clippy::unwrap_used` enforced; `#[allow(clippy::unwrap_used)]` allowed only in `#[cfg(test)]`).
- **DO NOT silence cross-target lints with `#[allow(dead_code)]`** (cross-target-verify-checklist.md Anti-pattern 2).
- **DO NOT trust `cargo check` as a clippy substitute** (cross-target-verify-checklist.md Anti-pattern 3).

## Project Constraints (from CLAUDE.md)

- **Error handling:** `NonoError` + `?` propagation only; no panics in production paths.
- **Unwrap policy:** `clippy::unwrap_used` enforced workspace-wide; only `#[cfg(test)]` may `#[allow(clippy::unwrap_used)]`.
- **Path security:** Always validate + canonicalize paths at enforcement boundary; never string-compare paths.
- **Fail Secure:** On any validation failure, exit non-zero and do NOT modify cache file. Per CLAUDE.md "Security Considerations" → "Fail Secure" principle.
- **Cross-target clippy MUST run** for any plan touching cfg-gated Unix code (Plan 49-01 qualifies — touches `setup.rs` which has `#[cfg(target_os = "windows")]`).
- **DCO sign-off required** on every commit (`Signed-off-by: ...`).
- **Workspace has 5 crates, not 3** (per memory `project_workspace_crates`) — if Plan 49-01 adds a dev-dep, only `crates/nono-cli/Cargo.toml` is touched; no internal path-dep version pins ripple (no public API additions).
- **No `#[allow(dead_code)]`** — any helper added must be wired into the live code path.

## Open Questions

None — SPEC.md (3 reqs, 0.152 ambiguity) + CONTEXT.md (D-49-A1..D2 all locked) are sufficient. Planner discretion items (clap attribute spelling, phase-index threading, CI step placement) are flagged for plan-author resolution at plan-open with no user-side question.

**One implementation-time surprise to flag to the planner:** The frozen fixture's tlogs lack `validFor.end` fields → the D-49-D1 expired-mutation must ADD them (planner re-reads this RESEARCH.md § "Fixture mutation surface" before drafting the test helper).

## RESEARCH COMPLETE
