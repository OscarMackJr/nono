---
phase: 49
phase_name: sigstore-trust-root-poc-resilience-from-file-flag-release-as
gathered: 2026-05-21
status: Ready for planning
requirements_locked_via: 49-SPEC.md (3 requirements — REQ-POC-TRUST-01, REQ-POC-TRUST-02, REQ-POC-TRUST-03; ambiguity 0.152)
---

# Phase 49: Sigstore trust-root POC resilience - Context

**Gathered:** 2026-05-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 49 delivers a structural fix to the recurring `nono setup --refresh-trust-root` failure caused by stale embedded TUF anchors in `sigstore-verify`. The fix gives POC users a code path that does NOT depend on the upstream-embedded anchor: a `--from-file <PATH>` CLI flag (REQ-POC-TRUST-01), a `trusted_root.json` release asset (REQ-POC-TRUST-02), and a maintainer-cadence template + cross-platform smoke script (REQ-POC-TRUST-03).

The phase exploits the existing D-32-15 verify-is-offline invariant: `nono trust verify` already reads the cache via `nono::trust::bundle::load_production_trusted_root()` using plain `TrustedRoot::from_file` deserialization (no `sigstore_verify` TUF code on the verify path). A side-channel populate of `<nono_home>/.nono/trust-root/trusted_root.json` is therefore sufficient to unblock verify — `--from-file` exploits exactly that structural property.

**Three plans, parallel-safe wave (per D-49-A1 / D-49-A2):**

- **Plan 49-01** — `feat(49-01): nono setup --from-file flag for trusted_root.json` — touches `crates/nono-cli/src/cli.rs` (clap arg + conflicts_with `--refresh-trust-root`), `crates/nono-cli/src/setup.rs` (new branch + helper that wraps `nono::trust::bundle::load_trusted_root` + `check_trusted_root_freshness` for validation, then `std::fs::copy` for byte-identity cache write), `crates/nono-cli/tests/setup_from_file.rs` (NEW integration test via `assert_cmd`).
- **Plan 49-02** — `chore(49-02): ship trusted_root.json as release asset` — touches `.github/workflows/release.yml` (new step: SHA-256 byte-identity assert between `crates/nono/tests/fixtures/trust-root-frozen.json` and `artifacts/trusted_root.json`; extend `softprops/action-gh-release` `files:` glob; extend `SHA256SUMS.txt` aggregation).
- **Plan 49-03** — `docs(49-03): sigstore rotation cadence + smoke script + POC handoff rewrite` — touches `.planning/templates/sigstore-rotation-refresh.md` (NEW), `scripts/verify-trust-root-cached.sh` (NEW), `scripts/verify-trust-root-cached.ps1` (NEW), `docs/cli/development/windows-poc-handoff.mdx` (rewrite of the "Known issue: Sigstore TUF root rotation" subsection).

**Inherited baseline:** Phase 49 inherits Phase 45 close SHA as the v2.6 quiet-baseline anchor for the baseline-aware CI gate (per D-44-E1 carry-forward). Surfaces are disjoint from Phases 44 / 45 / 46 / 47 / 48 (per ROADMAP Phase 49 entry), so the wave is independently schedulable any time before milestone close.

</domain>

<spec_lock>
## Requirements (locked via SPEC.md)

**3 requirements are locked.** See `49-SPEC.md` for full requirements, boundaries, and acceptance criteria.

Downstream agents MUST read `49-SPEC.md` before planning or implementing. Requirements are not duplicated here.

**In scope (from SPEC.md):**
- New CLI flag `--from-file <PATH>` on the `nono setup` subcommand (`crates/nono-cli/src/cli.rs` + `crates/nono-cli/src/setup.rs`).
- Reuse of the existing `nono::trust::bundle::load_trusted_root` + `check_trusted_root_freshness` validation pipeline for the new flag — no new schema validator, no new code paths in `crates/nono`.
- Clap-level `conflicts_with` between `--from-file` and `--refresh-trust-root`.
- Release-workflow change to copy `crates/nono/tests/fixtures/trust-root-frozen.json` to `artifacts/trusted_root.json` verbatim, with a CI-asserted SHA-256 byte-identity gate.
- Addition of `trusted_root.json` to the existing `SHA256SUMS.txt` and the `softprops/action-gh-release` `files:` glob.
- `.planning/templates/sigstore-rotation-refresh.md` maintainer-cadence template.
- `scripts/verify-trust-root-cached.sh` (or sibling `.ps1` for Windows) cross-platform cached-bytes verify smoke script.
- Rewrite of the `Known issue: Sigstore TUF root rotation` subsection in `docs/cli/development/windows-poc-handoff.mdx`.
- Unit + integration test coverage for the new flag (happy path, expired fixture, malformed input, missing path, clap-mutex collision).

**Out of scope (from SPEC.md):**
- Bumping `sigstore-verify` — the entire point of Phase 49 is to exit the dep-bump treadmill, not run another lap.
- Adding a `jsonschema`-crate-backed schema validator — reuse the existing `TrustedRoot::from_file` deserialize as the schema oracle.
- Content-hash-pinning `--from-file` to a known-good SHA-256 list baked into the binary — couples the flag to release cadence.
- A `--force` flag or freshness-aware overwrite protection — simple "last writer wins" overwrite.
- A separate cache path for user-supplied roots (`trusted_root.user.json`) — cache-path collision keeps the verify-path lookup unchanged.
- Predictive rotation tooling / "fetch next root before it rotates" automation.
- An automated `scripts/refresh-trust-root-fixture.sh` harness that does capture + diff + commit on the maintainer's behalf — deferred to v2.7 if maintainer cadence proves error-prone.
- Bundling `trusted_root.json` into the Windows MSIs — sliding a JSON inside requires re-spinning the MSI on every rotation.
- Authenticode/Sigstore-signing the `trusted_root.json` release asset itself — integrity is covered by `SHA256SUMS.txt`.
- Touching the verify path in `crates/nono` — D-32-15 verify-is-offline invariant is inherited, not modified.
- Modifying `crates/nono-shell-broker/` or `*_windows.rs` files.
- Hot-reload / SIGHUP-style cache refresh while `nono trust verify` is running.
- Cross-binding lockstep with `../nono-py/` + `../nono-ts/` — the new flag is `nono-cli`-only.

</spec_lock>

<decisions>
## Implementation Decisions

### Plan slicing & parallelism (Area A — discussed)

- **D-49-A1: Three plans, one per REQ.** 49-01 `--from-file` flag (REQ-POC-TRUST-01) → `crates/nono-cli/src/{cli,setup}.rs` + integration test. 49-02 release-asset bundling (REQ-POC-TRUST-02) → `.github/workflows/release.yml` + SHA256SUMS extension. 49-03 cadence template + smoke script + docs rewrite (REQ-POC-TRUST-03) → `.planning/templates/sigstore-rotation-refresh.md` + `scripts/verify-trust-root-cached.{sh,ps1}` + `docs/cli/development/windows-poc-handoff.mdx`. Mirrors the established v2.6 cadence (Phase 44 D-44-A1 + Phase 45 D-45-A1 "one plan per REQ"); per-plan SUMMARY + per-REQ closure ergonomics. **User explicitly chose** option (a) "3 plans, one per REQ" over (b) "2 plans: code+CI bundled vs docs+template" and (c) "1 mega-plan".

- **D-49-A2: All 3 plans parallel-safe in a single wave.** Surfaces disjoint: 49-01 = `crates/nono-cli/`, 49-02 = `.github/workflows/release.yml`, 49-03 = `.planning/templates/` + `scripts/` + `docs/cli/development/`. None of the three depend on each other's artifacts to be testable: 49-01 tests work against the existing `crates/nono/tests/fixtures/trust-root-frozen.json`; 49-02's CI gate runs against the existing fixture; 49-03's smoke script wraps the SPEC'd `--from-file` shape and skips the live-CLI smoke at plan-execute time if 49-01 hasn't landed (planner scaffolds the script against the spec; live-CLI smoke runs once 49-01 close lands). **User explicitly chose** option (a) "all 3 parallel-safe" over (b) "49-01 first then 49-02/49-03 parallel" and (c) "49-02 first then 49-01/49-03 parallel".

### Cache-write semantics for `--from-file` (Area B — discussed)

- **D-49-B1: Verbatim byte-copy of validated input to the cache path.** After `nono::trust::bundle::load_trusted_root(<PATH>)` + `check_trusted_root_freshness(...)` both succeed, the implementation does `std::fs::copy(<PATH>, <cache_path>)` (or equivalent raw-bytes read+write). The cache file is **byte-identical** to the input — critical for REQ-POC-TRUST-02's release-asset story: a POC user who downloads `trusted_root.json` from a GitHub Release and runs `--from-file` ends up with the same bytes on disk that CI asserted SHA-256-byte-identity for at release time. Provenance preserved end-to-end. **Rejected** option (b) "validated round-trip via `serde_json::to_string_pretty`" (the current `--refresh-trust-root` path at `setup.rs:849-852`) — re-serialization changes whitespace and potentially reorders map keys, breaking the release-asset provenance chain. **Rejected** option (c) "atomic tmpfile + rename" — introduces an inconsistency with `--refresh-trust-root` which does single-shot `fs::write` at `setup.rs:852`.

- **D-49-B2: Best-effort delete on copy failure.** Wrap the `std::fs::copy` in a guard: on `Err`, attempt `let _ = std::fs::remove_file(<cache_path>);` (swallow inner error), then propagate the original `NonoError::Io`. Cache is either fully written (success) or absent (failure). Honors SPEC.md REQ-POC-TRUST-01 acceptance criterion (c) "malformed input does NOT create or modify the cache file" extended to mid-copy IO failures. **Rejected** option (b) (reconsider tmpfile+rename — same reason as D-49-B1) and option (c) "punt: trust the FS" (violates SPEC acceptance contract).

- **D-49-B3: Stdout mirrors `--refresh-trust-root` shape + adds a `Source:` breadcrumb.** Print pattern (verb tweaked to "Loading"):
  ```
  [{phase_index}/{total_phases}] Loading Sigstore trusted root from file...
    * Sigstore trusted root cached at <cache_path>
    * Source: <abs_path_of_input>
  ```
  The first two lines mirror `setup.rs:826-857` exactly (with "Loading" verb in place of "Refreshing"); the third line is new and gives POC users a visible breadcrumb to which fixture they used. Aids debugging when a POC user runs `--from-file` once and later forgets the input path. **Rejected** option (b) "mirror verbatim, no Source: line" (no breadcrumb) and option (c) "silent on success" (breaks symmetry with the existing `--refresh-trust-root` output).

### Smoke script flavor + location (Area C — discussed)

- **D-49-C1: Matched `.sh` + `.ps1` pair at `scripts/verify-trust-root-cached.{sh,ps1}`.** Both ~20 lines, both take a `<PATH>` arg, both run `NONO_TEST_HOME=<tmp> nono setup --from-file <PATH>` followed by `nono trust verify <known-good-bundle> <known-good-source>`, both exit 0 on success / non-zero on any failure. First-class Windows POC UX (matches the POC-handoff doc's Windows audience); maintainer-cadence template recommends the appropriate one per platform. **Rejected** option (b) "bash-only with Git Bash note" (POC-handoff doc's Windows audience may not have Git Bash; mismatches the fork's Windows-first POC posture) and option (c) "cargo test harness" (couples smoke to Rust toolchain; less obvious as a 'try this at home' artifact).

- **D-49-C2: Smoke inputs reuse existing trust-test fixtures.** The `<known-good-bundle>` and `<known-good-source>` are sourced from the existing `crates/nono/tests/fixtures/` layout (planner inventories at plan-open via `grep -rl "trust" crates/nono/tests/fixtures/ + ls crates/nono/tests/fixtures/*.sigstore.json *.toml`). Zero new fixtures to maintain through rotations. Coupling risk: if a future phase renames/relocates those fixtures, both the test suite and the smoke script touch — accepted because the rename event is rare. **Rejected** option (b) "ship dedicated smoke-test fixtures alongside the script" (2 more fixtures to maintain through every Sigstore rotation) and option (c) "take both as CLI args" (defeats the point of a smoke script).

- **D-49-C3: Smoke script is maintainer-only; not wired into PR CI.** Referenced from `.planning/templates/sigstore-rotation-refresh.md` as the pre-commit maintainer gate during a Sigstore rotation. PR CI continues to rely on the existing `cargo test -p nono trust::bundle::load_test_trusted_root_smoke` regression (which already exercises the cached path end-to-end). Avoids duplicate-gate cost: the existing test catches the same class of regression in PR CI for free. **Rejected** option (b) "wire into PR CI as a separate lane" (~30s added to every PR CI run for an overlapping check) and option (c) "wire into release workflow only" (regression slips past PR CI and surfaces at release time).

### Test fixture strategy for expired/malformed inputs (Area D — discussed)

- **D-49-D1: Runtime helper that mutates frozen fixture in a TempDir.** A test-only helper (co-located in the new `crates/nono-cli/tests/setup_from_file.rs` integration test, or in a `crates/nono-cli/tests/common/mod.rs` module if the existing test suite already uses that pattern — planner inventories at plan-open) reads `crates/nono/tests/fixtures/trust-root-frozen.json`, mutates it in a per-test `tempfile::TempDir` per case:
  - **Expired:** set all tlog `valid_for.end` fields to `"1970-01-01T00:00:00Z"` (forces `check_trusted_root_freshness` failure per the WR-05 fail-closed format guard at `bundle.rs:277-281`).
  - **Malformed-truncation:** truncate to 100 bytes (forces `TrustedRoot::from_file` deserialize failure).
  - **Malformed-quote-flip:** flip a single byte in a JSON string-quote position (forces JSON parse failure with a distinct error class from truncation).
  - **Missing-path:** point `--from-file` at a `TempDir`-relative path that doesn't exist.

  ONE fixture to maintain through Sigstore rotations; mutation logic is explicit and inspectable in the test file. **Rejected** option (b) "check in dedicated `trust-root-expired.json` + `trust-root-malformed.json`" (2 more fixtures to refresh every rotation — `expired` could accidentally become 'currently valid' if a rotation pushes the freshness window back) and option (c) "synthesize entirely in-test via `serde_json::json!{...}` literals" (hand-constructed TrustedRoot literal is brittle to upstream schema changes).

- **D-49-D2: Integration test at `crates/nono-cli/tests/setup_from_file.rs` via `assert_cmd`.** New integration test file alongside the existing `crates/nono-cli/tests/` suite. Uses `assert_cmd::Command::cargo_bin("nono")` to invoke the binary end-to-end with `--from-file <path>`. `NONO_TEST_HOME` points at a per-test `tempfile::TempDir`. Matches the pattern already used by `crates/nono-cli/tests/{auto_pull_e2e_linux,resl_nix_linux}.rs` (per Phase 44 CONTEXT.md inventory). `assert_cmd` is already in the workspace via existing tests — zero new deps. Covers all 5 acceptance cases from REQ-POC-TRUST-01: happy path, expired, malformed (truncation), malformed (quote-flip), missing path, and the clap-mutex case (`--from-file <p> --refresh-trust-root` rejected at clap-parse time). **Rejected** option (b) "unit-test in setup.rs via factored helper" (doesn't exercise the clap-mutex check; net more test surface) and option (c) "mix: unit + integration" (split test files for one feature; harder to find).

### Claude's Discretion

Areas the planner / researcher decide at plan-open with no user-side question:

- **`setup.rs` phase-index threading.** The current code uses `refresh_trust_root_phase_index()` and `total_phases()` arithmetic (`setup.rs:719,723,740,744,795,820`); the planner threads `--from-file` through the same index/total functions. Cosmetic ordering only — no user-facing decision.
- **Exact clap attribute spelling for `conflicts_with`.** clap v4 supports `conflicts_with = "refresh_trust_root"` (field-name string) or `conflicts_with = SetupArgs::REFRESH_TRUST_ROOT` (enum). Planner picks whichever matches the surrounding `crates/nono-cli/src/cli.rs` style.
- **Per-plan commit shape.** Plan 49-01 is likely a single atomic `feat(49-01):` commit because the CLI + setup + test changes are tightly coupled. Plan 49-02 is a single atomic `chore(49-02):` commit because the release.yml edits are co-located. Plan 49-03 may be 1-3 commits (template / smoke scripts / docs rewrite as separate commits if the reviewer wants per-file scope, single atomic if `docs(49-03):` is cleaner). Planner chooses.
- **CI step placement in `release.yml`.** The byte-identity assert can live before the existing artifact-assembly steps or as a sibling to the SHA256SUMS computation. Planner picks the placement that minimizes diff against the existing `.github/workflows/release.yml:325-340` block.
- **POC-handoff docs rewrite scope.** The minimum scope is the `Known issue: Sigstore TUF root rotation` subsection at `docs/cli/development/windows-poc-handoff.mdx:182-220`. The planner may discover adjacent subsections that also reference `--refresh-trust-root` (e.g., the "Run once after install" block at lines 166-180) and update those for consistency. Planner sweeps as discovered, scope-creep-aware: anything beyond the trust-root narrative gets deferred.
- **Cross-target clippy scope.** Plan 49-01 touches `crates/nono-cli/src/{cli,setup.rs}` which contain `#[cfg(target_os = "windows")]` and `#[cfg(not(any(...)))]` blocks. Cross-target clippy on `x86_64-unknown-linux-gnu` AND `x86_64-apple-darwin` MUST run per CLAUDE.md MUST/NEVER rule + `.planning/templates/cross-target-verify-checklist.md`. Planner runs both targets at verification time; PARTIAL allowed only if cross-toolchain unavailable.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 49 artifacts
- `.planning/phases/49-sigstore-trust-root-poc-resilience-from-file-flag-release-as/49-SPEC.md` — 3 locked requirements with current/target/acceptance triplets; 11-item out-of-scope list; 16-item acceptance criteria checkbox. **LOCKED — MUST read before planning.**
- `.planning/ROADMAP.md` § "Phase 49: Sigstore trust-root POC resilience" — 5 success criteria; phase numbering; "parallel-safe with 44–48" assertion; mid-milestone-addition trigger context.
- `.planning/REQUIREMENTS.md` § "v2 Requirements (Deferred)" — references `P32-DEFER-005` (note: superseded by Phase 49's structural fix per ROADMAP entry).

### Trust-root code path (sigstore integration)
- `crates/nono-cli/src/setup.rs:820-860` — current `refresh_trust_root_step()` implementation; reference for the new `--from-file` step's phase-index threading and stdout shape.
- `crates/nono-cli/src/cli.rs:2341-2385` — `SetupArgs` struct with all current flags including `refresh_trust_root`; reference for the new `--from-file` clap attribute placement.
- `crates/nono/src/trust/bundle.rs:113-167` — `load_trusted_root` + `load_production_trusted_root` (production cache reader the verify path uses); confirms the validation pipeline `--from-file` reuses.
- `crates/nono/src/trust/bundle.rs:247-305` — `check_trusted_root_freshness` (D-32-03 expiry gate via WR-05 fail-closed ISO-8601 format guard); the freshness gate `--from-file` reuses.
- `crates/nono/src/trust/mod.rs:68-80` — `#[cfg(test)] load_test_trusted_root()` and its anchor at `tests/fixtures/trust-root-frozen.json`; the canonical source-of-truth fixture.
- `crates/nono/tests/fixtures/trust-root-frozen.json` — the 126-line fixture that gets verbatim-copied to the release asset (REQ-POC-TRUST-02) and mutated in TempDirs for test cases (D-49-D1).

### CI / release packaging
- `.github/workflows/release.yml:325-340` — current `softprops/action-gh-release` step with `files:` glob (`*.tar.gz/.zip/.msi/.exe/.deb/SHA256SUMS.txt`); the surface Plan 49-02 extends.
- `.github/workflows/release.yml:325-326` — `SHA256SUMS.txt` aggregation step; Plan 49-02 adds `trusted_root.json` to the aggregation input.

### POC handoff documentation
- `docs/cli/development/windows-poc-handoff.mdx:166-180` — "Run once after install" block; may need consistency edits (per Claude's Discretion above).
- `docs/cli/development/windows-poc-handoff.mdx:182-220` — `Known issue: Sigstore TUF root rotation (sigstore-verify 0.6.5)` subsection; Plan 49-03 rewrites this. Contains the broken `P32-DEFER-005` / `deferred-items.md` cross-reference that gets removed.

### Phase 32 trust-root architecture (decisions the phase inherits)
- `.planning/phases/32-sigstore-integration/32-CONTEXT.md` — D-32-01 (cached `trusted_root.json` path), D-32-03 (freshness gate), D-32-05 (first-run UX recovery message), D-32-15 (verify-is-offline invariant) — all inherited unchanged.
- `tests/integration/test_upstream_drift.sh:257` — `# intentional fork: Phase 32 D-32-01` annotation; may need an additional Phase 49 reference per SPEC.md acceptance (f).

### Test infrastructure references
- `crates/nono-cli/tests/auto_pull_e2e_linux.rs` — `assert_cmd::Command::cargo_bin("nono")` pattern reference for D-49-D2.
- `crates/nono-cli/tests/resl_nix_linux.rs` — second `assert_cmd` pattern reference.
- Phase 44 CONTEXT.md § Code Insights — `assert_cmd` workspace inventory.

### Cross-target enforcement
- `.planning/templates/cross-target-verify-checklist.md` — MUST/NEVER rule for cfg-gated Unix code. Plan 49-01 touches `crates/nono-cli/src/{cli,setup}.rs` which contain `#[cfg(target_os = "windows")]` and `#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]` blocks.
- `CLAUDE.md` § "Coding Standards" → "Cross-target clippy verification" bullet — the codified MUST/NEVER rule.

### Project decisions / memory (apply across the phase)
- `CLAUDE.md` § "Security Considerations" → "Fail Secure" principle — applies to `--from-file` error handling (D-49-B2).
- `CLAUDE.md` § "Coding Standards" → "Unwrap Policy" — `clippy::unwrap_used` enforced; `--from-file` helper uses `?` propagation, no `.unwrap()` / `.expect()`.
- `CLAUDE.md` § "Coding Standards" → "Lazy use of dead code" — any helper added for `--from-file` must be wired into the live code path, not gated behind `#[cfg(test)]` if it's a production primitive.
- Memory `project_workspace_crates` — workspace has 5 crates; only Plan 49-01 may touch any `Cargo.toml` and only if a new dep is required (none anticipated — `assert_cmd` + `tempfile` already in `crates/nono-cli`).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`nono::trust::bundle::load_trusted_root(<path>)`** (`crates/nono/src/trust/bundle.rs:113-116`) — public function that wraps `TrustedRoot::from_file` with a `NonoError::TrustPolicy` error class. Exact validator `--from-file` reuses; zero new code in `crates/nono`.
- **`nono::trust::bundle::check_trusted_root_freshness(&trusted_root, &cache_path)`** (`crates/nono/src/trust/bundle.rs:247-305`) — pub(crate) freshness gate with WR-05 fail-closed ISO-8601 format guard. Currently called inside `load_production_trusted_root`; needs to be made callable from the new `--from-file` path (planner determines: re-export as `pub`, or factor the freshness-check into a `pub` helper that both `load_production_trusted_root` and `--from-file` call).
- **`crates/nono/tests/fixtures/trust-root-frozen.json`** — 126-line frozen TUF root fixture. Source-of-truth for the release asset (Plan 49-02) AND the mutation base for test fixtures (Plan 49-01 D-49-D1).
- **`SetupArgs` struct** (`crates/nono-cli/src/cli.rs:2341-2385`) — clap derive with `help_heading = "OPTIONS"` convention; new `--from-file` flag follows the same shape with `conflicts_with = "refresh_trust_root"` (or the equivalent clap-v4 spelling).
- **`assert_cmd` + `tempfile`** — already in `crates/nono-cli` dev-deps (per Phase 44 CONTEXT.md inventory); zero new deps for Plan 49-01 tests.
- **`softprops/action-gh-release@153bb8e0...` step in release.yml** — already pinned to v2; Plan 49-02 extends its `files:` glob, not its dependency.

### Established Patterns
- **`refresh_trust_root_step()` shape** (`setup.rs:820-860`) — header print + `std::fs::create_dir_all(&cache_dir)` + validation block + `std::fs::write(&cache_path, ...)` + footer print. `--from-file` mirrors this shape (header tweak, validation block points at user `<PATH>`, write is `std::fs::copy` per D-49-B1, footer adds `Source:` line per D-49-B3).
- **`refresh_trust_root_phase_index()` and `total_phases()` arithmetic** (`setup.rs:719,723,740,744,795`) — counts active phases based on `bool` flags. `--from-file` slots into the same arithmetic (planner threads it through; cosmetic ordering only).
- **Clap `conflicts_with`** — clap v4 derive supports `#[arg(long, conflicts_with = "other_field")]`. Used elsewhere in the codebase (planner greps for `conflicts_with` to inventory existing call sites for style consistency).
- **Integration tests via `assert_cmd::Command::cargo_bin("nono")`** — pattern reference at `crates/nono-cli/tests/auto_pull_e2e_linux.rs` and `crates/nono-cli/tests/resl_nix_linux.rs`. Planner copies the imports + per-test `TempDir` + `NONO_TEST_HOME` env shape.
- **Maintainer-cadence template shape** — `.planning/templates/cross-target-verify-checklist.md` (78 lines, sections: Scope / Decision Tree / Cross-Toolchain Setup / PARTIAL Disposition / Anti-Patterns / Enforcement) and `.planning/templates/upstream-sync-quick.md` (planner reads both at plan-open). New `sigstore-rotation-refresh.md` mirrors the same structural shape: trigger sources / capture command / byte-diff / regression check / commit-and-tag / release-asset gate cross-link.

### Integration Points
- **`SetupArgs` → `Setup` runtime** (`crates/nono-cli/src/setup.rs:24-43`) — the `Setup::from_args` constructor (`setup.rs:43`) copies `args.refresh_trust_root` into `self.refresh_trust_root`. New `from_file: Option<PathBuf>` field follows the same wiring.
- **`Setup::run` entry** (`crates/nono-cli/src/setup.rs:91`) — branch point where `if !self.check_only && self.refresh_trust_root { self.refresh_trust_root_step()? }`. New branch added for `self.from_file.is_some()`.
- **`load_production_trusted_root` callsite in verify path** (`crates/nono/src/trust/bundle.rs:147`) — UNCHANGED. The cache file `--from-file` writes is read by this function later, completing the side-channel populate.
- **`SHA256SUMS.txt` aggregation in `release.yml`** — Plan 49-02 extends the aggregation input by one file (`trusted_root.json`).

</code_context>

<specifics>
## Specific Ideas

- **POC-handoff doc Windows-first posture preserved.** D-49-C1's `.sh` + `.ps1` pair is specifically motivated by the POC-handoff doc's Windows audience — POC users follow that doc on Windows hosts, and `bash scripts/...` (per the rejected option) imposes a Git Bash dependency the doc otherwise doesn't require.
- **Release-asset SHA-256 byte-identity is the load-bearing CI gate.** D-49-B1's byte-copy semantics + Plan 49-02's SHA-256 assert form an end-to-end provenance chain: what the maintainer commits to `crates/nono/tests/fixtures/trust-root-frozen.json` is what CI uploads to the GitHub Release, and what a POC user downloads and `--from-file`s is byte-identical to what's on disk. Breaking any link in this chain (e.g., changing D-49-B1 to round-trip serialize) silently breaks the provenance story without a CI signal.
- **Inherit don't extend Phase 32's verify-is-offline invariant.** D-32-15 says the verify path is structurally + dynamically offline. Phase 49 explicitly does NOT touch verify (per SPEC.md out-of-scope); the verify side-effect of `--from-file` happens at setup time only. This preserves the invariant by construction.

</specifics>

<deferred>
## Deferred Ideas

- **Automated `scripts/refresh-trust-root-fixture.sh` harness** that does capture + diff + commit on the maintainer's behalf. Open as a v2.7 backlog item if maintainer cadence proves error-prone in practice. SPEC.md out-of-scope.
- **`--force` flag or freshness-aware overwrite protection on `--from-file`.** Round-1 spec-phase decision was simple "last writer wins" overwrite. Reconsider as a v2.7 follow-up if a POC user accidentally overwrites a working cache with a stale drop.
- **Bundling `trusted_root.json` into the Windows MSIs.** Requires re-spinning the MSI on every Sigstore rotation; breaks the "rotate fixture independently of binary" invariant. Release-asset bundling (Plan 49-02) is sufficient. Reconsider only if a future requirement says POC users cannot reach github.com/releases.
- **Authenticode/Sigstore-signing the `trusted_root.json` release asset itself.** Redundant — the file is already a Sigstore-signed artifact (it's the trust anchor); fork-side signing requires key custody. Integrity covered by `SHA256SUMS.txt` per existing release-integrity gate.
- **Cross-binding lockstep with `../nono-py/` + `../nono-ts/` siblings.** The new flag is `nono-cli`-only; Python/TS bindings expose `nono` library directly and inherit verify-is-offline without change. Reconsider only if a future binding consumer wants programmatic `--from-file` parity (not anticipated).
- **Phase 44 follow-up todos (`44-class-d-validator-preflight-investigation.md`, `44-validate-restore-target-fd-relative-hardening.md`).** Surfaced by `todo.match-phase 49` with score 0.6 (keyword-only). Both are about Class D Landlock validator + restore-target TOCTOU — unrelated to Sigstore trust-root. Not folded. Carry-forward target: Phase 46+ Linux-host phase per Phase 44 CONTEXT § Deferred Ideas.

</deferred>

---

*Phase: 49-sigstore-trust-root-poc-resilience-from-file-flag-release-as*
*Context gathered: 2026-05-21*
