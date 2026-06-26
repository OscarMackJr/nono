# Cross-Target Clippy Verification Checklist

**Read this template before flipping any plan-touching-cfg-gated-Unix-code REQ to VERIFIED.**

**Source:** Phase 25 CR-A regression lesson (memory `feedback_clippy_cross_target`) + Phase 41 Plans 41-09 / 41-10 (twice mis-verified on Windows-host-only evidence). **Updated Phase 96 (2026-06-26):** both cross-target gates are now **provably local-runnable on this Windows dev host** (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`); the auto-default-to-PARTIAL→CI is **retired per-gate** (evidence-based, per Phase 96 D-07).

---

## Scope

This checklist applies to every plan that touches:
- Files containing `#[cfg(target_os = "linux")]` or `#[cfg(target_os = "macos")]` blocks
- Files containing `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks
- Files under `crates/nono-cli/src/exec_strategy/` (Unix supervisor code)
- Files under `bindings/c/src/` (FFI code consumed by macOS / Linux runtimes)
- Any file re-exported via Unix-side modules in `crate::exec_strategy` (the non-Windows file path)

Does NOT apply to:
- Pure Windows-only files (e.g. anything under `crates/nono-cli/src/exec_strategy_windows/` that has NO Unix counterpart)
- Pure documentation changes
- Pure build-tooling changes (Cargo.toml, build.rs) that don't change Rust source

## Decision Tree

**Question 1:** Does the plan touch any in-scope file (per § Scope above)?
- **No** → cross-target verification not required. Proceed with standard verification.
- **Yes** → continue to Question 2.

**Question 2 (linux-gnu — MUST run locally):** Run the canonical containerized gate on the dev host:
```bash
cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
```
This is the canonical linux-gnu invocation. `cross clippy` runs `cargo clippy` *inside* the pinned Linux container (real `x86_64-linux-gnu-gcc`, so the C-linking crates `aws-lc-sys` / `ring` link cleanly), exercising the real `#[cfg(target_os = "linux")]` branches with the same lints. The bare `cargo clippy --workspace --target x86_64-unknown-linux-gnu ...` is **NOT** independently runnable on this host (no native gnu linker) and must not be presented as the runnable command — use the `cross clippy` form above.

- **Clean exit (0)** → linux-gnu is satisfied; the REQ may flip to VERIFIED at the codebase level for the Linux gate. Proceed to Question 3.
- **Errors reported** → REQ must be marked PARTIAL or GAPS_FOUND. Errors must be **closed structurally** (cfg-gates, visibility, recovered fork invariants — NEVER `#[allow]` silencing) before flipping to VERIFIED. The first local run of this gate (Phase 96 Plan 01) surfaced *hard compile errors* in dropped fork invariants — that is the gate's whole value; fix, don't defer.
- **PARTIAL is allowed ONLY on a *documented* cross/Docker failure** — an image-pull failure or a genuine Docker engine capability gap, captured verbatim in the verification record. A **stopped Docker daemon is NOT such a failure** (start it: `docker info` must report a `Server Version`), and **"toolchain absent" is NOT such a failure** (cross 0.2.5 + Docker are present on this host). The auto-default-to-PARTIAL for linux-gnu is **retired**.

**Question 3 (apple-darwin — MUST run locally):** Run the canonical zig-linked gate on the dev host:
```bash
cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```
This is the canonical apple-darwin invocation. **Use the direct-binary `cargo-zigbuild clippy …` form, NOT `cargo zigbuild clippy …`** — the cargo external-subcommand form passes `zigbuild` as argv[1], which collides with the binary's own `zigbuild` subcommand and mis-parses `clippy` as a stray arg. cargo-zigbuild 0.23.0 exposes `clippy` as a first-class subcommand on the binary itself, so the direct-binary form above is the working invocation. `SDKROOT` **MUST stay UNSET** (the gate passed clean with it unset in Phase 96 Plan 02; setting it would cross the proprietary-macOS-SDK licensing line and is unnecessary).

- **Clean exit (0)** → apple-darwin is satisfied; the REQ may flip to VERIFIED at the codebase level for the macOS gate.
- **Errors reported** → close errors structurally first (same rule as Q2 — no `#[allow]` silencing).
- **PARTIAL is allowed ONLY on a *documented* zig / cargo-zigbuild failure** (e.g. a captured macOS-SDK / framework / Mach-O link signature such as `'<x>.h' file not found`, `framework not found for '-framework ...'`, `unable to find Darwin SDK`, or an `aws-lc-sys` emulation/link mismatch), recorded verbatim. A **missing-tool or absent-toolchain state is NOT such a failure** (install `zig` + `cargo-zigbuild` per § Cross-Toolchain Setup). The auto-default-to-PARTIAL for apple-darwin is **retired**.

**NEVER:** Flip a Unix-touching REQ to VERIFIED based solely on `cargo check --workspace` from a Windows host. `cargo check` does not run clippy, does not enforce `-D warnings`, and does not exercise the Unix-cfg-gated code paths that CI's Linux/macOS clippy lanes do. Windows-host `cargo clippy` (no `--target`, or `--target x86_64-pc-windows-*`) is also NOT a substitute — it only exercises the Windows cfg branches and is structurally blind to `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` drift.

## Cross-Toolchain Setup (one-time)

Both rustup std targets are already added on this host (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`); `rustup target add` is no longer the primary setup step. The real setup is the per-gate runner below.

### linux-gnu runner — Docker + `cross`

Requires `cross` 0.2.5 + a running Docker Linux engine (both installed on this host).

```bash
# 1. Confirm the Docker engine is up (a stopped daemon is an operator precondition, not a gate failure):
docker info 2>&1 | grep "Server Version"      # must print e.g. "Server Version: 29.5.3"

# 2. Run the gate:
cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
```

Pinned cross image (recorded Phase 96 Plan 01 — copy this to reproduce the exact base):
```
ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5@sha256:9e5b39c09874bc1816c675ed11afca2c2ed6cee0c4ed2b3c1d5763c346c9ae3f
```
The `Cross.toml` `[target.x86_64-unknown-linux-gnu]` pre-build (`libdbus-1-dev` + `pkg-config`) runs as a `RUN` layer on top of that base image (cached after the first run).

### apple-darwin runner — `zig` + `cargo-zigbuild`

Requires `zig` 0.16.0 + `cargo-zigbuild` 0.23.0 (host installs; from official orgs `ziglang` / `rust-cross`).

```bash
# 1. Install (one-time):
winget install --id zig.zig          # -> zig 0.16.0; shim under %LOCALAPPDATA%\Microsoft\WinGet\Links
cargo install --locked cargo-zigbuild # -> cargo-zigbuild 0.23.0; lands in %USERPROFILE%\.cargo\bin
# Ensure both dirs are on PATH (a fresh shell picks up the winget Links dir automatically).

# 2. Run the gate (SDKROOT MUST stay UNSET; use the direct-binary form):
cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

zig's bundled macOS C target support satisfies the `aws-lc-sys` / `ring` build-dep C probes without a macOS SDK, so the gate runs clean with `SDKROOT` unset and no SDK extraction.

## PARTIAL Disposition (FALLBACK — only when a runner is genuinely unavailable)

PARTIAL→CI is no longer the default for either gate; it is the **fallback** for a *documented* runner failure (a captured Docker/cross failure for linux-gnu, or a captured zig/cargo-zigbuild failure for apple-darwin — see Q2 / Q3). A stopped daemon, a missing-but-installable tool, or "I didn't run it" do NOT qualify. When a runner genuinely fails on a documented signature, the verifier MUST:

1. Mark the related REQ as **PARTIAL** (not VERIFIED) at the codebase level.
2. Capture the **verbatim failure signature** (image-pull error, Docker capability gap, or the macOS-SDK/framework/Mach-O link error) in the verification record — prose acknowledgement alone does not qualify.
3. Add a `human_verification_truths` entry referencing the specific live-CI lane that compensates (e.g., "GH Actions Linux Clippy lane on the head SHA reports no -Dwarnings errors").
4. Set the overall verification status to `human_needed` (not `passed`).
5. Document the SKIPPED reason in the verification report's § "Codebase Evidence" section using this exact prose:

   > Cross-target clippy gate SKIPPED on Windows dev host due to a documented runner failure for x86_64-{unknown-linux-gnu | apple-darwin}: <verbatim failure signature>. The live GH Actions {Linux Clippy | macOS Clippy} lane on the head SHA is the decisive signal per .planning/templates/cross-target-verify-checklist.md. REQ marked PARTIAL pending CI confirmation.

Do NOT flip the REQ to VERIFIED until the live CI lane reports green on the head SHA.

## Anti-Patterns (do NOT do)

- **Anti-pattern 1:** "Documented as load-bearing risk; flipped to VERIFIED anyway" — this is what happened in Phase 41 (twice). Acknowledging the risk in prose does not discharge it. The REQ must be PARTIAL until the local gate runs clean (or CI confirms, in the documented-failure fallback).
- **Anti-pattern 2:** Adding `#[allow(dead_code)]` or `#[allow(clippy::unwrap_used)]` to silence cross-target lints. This violates REQ-CI-01 SC#4 (no raw allows) AND CLAUDE.md § Unwrap Policy. Use cfg-gates, visibility changes, or structural code changes instead.
- **Anti-pattern 3:** Running `cargo check` and assuming it covers clippy. It does not. `cargo check` does not run clippy.
- **Anti-pattern 4:** Running `cargo clippy --workspace` (no `--target`) on Windows host and assuming it covers Linux/macOS. It does NOT — the host-target clippy only exercises Windows cfg branches.
- **Anti-pattern 5:** Defaulting a gate to PARTIAL→CI because Docker was stopped, a tool was not yet installed, or the gate was simply not run. PARTIAL→CI is the fallback for a *documented runner failure only* (Phase 96 D-07) — start the daemon / install the tool and run the gate.
- **Anti-pattern 6:** Invoking apple-darwin as `cargo zigbuild clippy …` (the cargo external-subcommand form). It mis-parses `clippy` and silently degrades to a plain `zigbuild`/`check`. Use the direct-binary `cargo-zigbuild clippy …` form.

## Enforcement

This checklist is referenced from:
- CLAUDE.md § "Coding Standards" → bullet "Cross-target clippy verification" (a one-line pointer here — this file is the single source of truth for setup + invocation)
- Future close-gate verifications via `/gsd-verify-phase` (verifier reads this file before flipping cfg-gated-Unix-touching REQs)

Established 2026-05-16 by Phase 41 Plan 41-10 Task 5 (REQ-CI-03 closure response to twice-mis-verified REQ-CI-01). Rewritten 2026-06-26 by Phase 96 Plan 03 (XTGT-04) to retire the auto-PARTIAL default per-gate after both gates were proven local-runnable (linux-gnu RECORD: `96-01-XTGT-LINUX-GNU-RECORD.md`; apple-darwin RECORD: `96-02-XTGT-APPLE-DARWIN-RECORD.md`).
