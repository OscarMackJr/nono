---
slug: broker-trust-gate-path-bypass
status: resolved
trigger: "Security review R-B4: broker Authenticode trust-anchor gate is skipped whenever the running nono.exe path merely CONTAINS the substring \\target\\release\\ (or \\target\\debug\\, /target/release/, /target/debug/). An attacker who places an unsigned nono.exe + sibling nono-shell-broker.exe under any directory whose path contains that substring (e.g. C:\\Users\\victim\\target\\release\\nono.exe) bypasses verify_broker_authenticode entirely and the broker is trusted/spawned."
created: 2026-06-13
updated: 2026-06-13
---

# Debug: broker trust-anchor gate path-substring bypass (R-B4)

## Symptoms

- **Expected:** the Authenticode self-trust-anchor gate (`verify_broker_authenticode`) should only be skipped for *genuine* local Cargo dev builds, never for an arbitrary attacker-chosen install path.
- **Actual:** `is_dev_build_layout()` (`crates/nono-cli/src/exec_strategy_windows/launch.rs:2039`) returns `true` for ANY path containing the substring `\target\release\` / `\target\debug\` / `/target/release/` / `/target/debug/`. The skip is a pure string match on the runtime exe path — not a build-provenance signal.
- **Error/impact:** an unsigned broker under such a path is spawned with no Authenticode verification → trust-anchor bypass on a shared or attacker-writable machine.
- **Timeline:** present since the Phase 32 broker trust gate (D-32-12). Surfaced by the Phase-1 Windows confinement security review (R-B4, HIGH).
- **Reproduction:** place unsigned `nono.exe` + `nono-shell-broker.exe` under `C:\anything\target\release\`; invoke a broker-arm run; observe `tracing::info!(target:"broker_authenticode", "skipping broker Authenticode verify…")` and a spawned unsigned broker.

## Current Focus

- **hypothesis:** The dev-build skip uses a path-substring detector that is satisfiable by attacker-controlled path layout. The gate must key off a signal an attacker outside the build cannot forge.
- **test:** Identify a dev-detection signal that (a) is true for real local builds, (b) is false for any production/attacker install path, and (c) does NOT break the documented `cargo test --release` constraint below.
- **expecting:** A fix that closes the bypass while keeping `cargo test --release` (unsigned broker, no debug_assertions) and normal `cargo run` / `cargo test` dev flows working.
- **next_action:** Confirm root cause in code; design a provenance-based dev-detection that survives the release-test constraint; implement; add regression tests; cross-target clippy.

## CRITICAL CONSTRAINT — read before proposing a fix (root cause already established)

Root cause is **known and confirmed in code** (`launch.rs:2026-2045`). This is NOT a "find the bug" task; it is a "find a fix that does not regress" task.

The review's first-pass suggestion — "just gate on `#[cfg(debug_assertions)]`" — is **WRONG** and must not be applied. The existing doc comment at `launch.rs:2030-2035` explains why the authors deliberately chose the path-substring detector over `#[cfg(debug_assertions)]`:

> `cargo test --release` compiles WITHOUT debug_assertions, so a `#[cfg(debug_assertions)]` gate would falsely apply strict Authenticode checks to release-mode test runs where `nono-shell-broker.exe` is unsigned.

So any fix MUST satisfy ALL of:
1. **Closes the bypass:** an unsigned binary at an attacker-chosen path containing `\target\release\` must NO LONGER skip the gate.
2. **`cargo test --release` still works:** release-mode test runs spawn an unsigned dev broker and must still skip the gate (or otherwise not fail on the unsigned broker).
3. **Normal dev flows work:** `cargo run`, `cargo test` (debug) keep skipping the gate.
4. **Production installs are unaffected:** `Program Files\nono\` etc. continue to ENFORCE the gate (already true today).

### Candidate fix directions for the debugger to evaluate (pick the strongest)

- **(A) Compile-time-baked dev root.** Bake the actual build output dir at compile time (e.g. derive a workspace/target root from `env!("CARGO_MANIFEST_DIR")` or `OUT_DIR`) and require the running exe's *canonicalized* path to live under THAT specific baked root — not any string containing `\target\release\`. An attacker cannot forge the compile-time-baked absolute path of the developer's machine. Must handle that test binaries run from `target\{debug,release}\deps\` while the broker sits in `target\{debug,release}\`.
- **(B) Build-provenance env stamp.** A `build.rs` stamps a compile-time constant (e.g. `option_env!` of a build-id / a value only set during the project's own cargo build) that the runtime checks in addition to the path. Production MSI binaries built by the signed pipeline would either carry the signed identity (gate enforces) or not carry the dev stamp.
- **(C) Combine path + a non-path signal.** Keep the path heuristic but additionally require a signal an attacker-placed copy won't have (e.g. a sibling `.cargo-lock`/workspace marker, or the baked manifest dir). Weakest of the three — evaluate only if A/B are infeasible.

Prefer (A) or (B). Whatever is chosen, add a unit test proving an attacker-style path (`C:\Users\victim\target\release\nono.exe` that is NOT the baked dev root) does NOT count as dev layout, plus a test that the real baked dev root DOES.

## Evidence

- timestamp: 2026-06-13 — `launch.rs:2039` `is_dev_build_layout` is a 4-way `String::contains` on the exe path. Call sites: `launch.rs:1375` (BrokerLaunch / PTY arm) and the no-PTY arm (~`:1695`). Confirmed via security-review subagent + direct read.
- timestamp: 2026-06-13 — doc comment `launch.rs:2030-2035` documents the deliberate rejection of `#[cfg(debug_assertions)]` due to the `cargo test --release` unsigned-broker case.

## Eliminated

- hypothesis: "gate on #[cfg(debug_assertions)]" — ELIMINATED by the documented `cargo test --release` constraint (would break release-mode test runs). Do not re-propose.

## Resolution

root_cause: `is_dev_build_layout()` (launch.rs:2039) skipped the broker Authenticode self-trust-anchor gate based on a 4-way `String::contains` substring match (`\target\release\` etc.) on the *runtime* exe path. An attacker satisfies it by choosing an install path containing that segment (e.g. `C:\Users\victim\target\release\nono.exe`), bypassing `verify_broker_authenticode` and trusting/spawning an unsigned broker (R-B4, HIGH).

fix: Direction (A) — compile-time-baked dev target root.
  - `build.rs` now bakes `NONO_DEV_TARGET_ROOT` = `OUT_DIR.ancestors().nth(4)` (the `<target>` dir for THIS build, honoring `CARGO_TARGET_DIR`) via `cargo:rustc-env`. Fail-closed: if the root can't be derived, it bakes empty and the dev-skip is disabled (gate ENFORCED).
  - `is_dev_build_layout` now delegates to new `is_under_dev_target_root(exe, DEV_TARGET_ROOT)`, which canonicalizes both the baked root and the running exe and requires component-wise `Path::starts_with` (NOT string `starts_with` — that was the bug class). Fail-closed on empty root or any uncanonicalizable path.
  - An attacker cannot reproduce the developer machine's compile-time-baked absolute path, and a copied binary under a different `...\target\release\` is NOT under the baked root → gate enforced.
  - `#[cfg(debug_assertions)]` deliberately avoided: this is a path-provenance signal, so release-mode test binaries (run from `<baked target>\release\deps\`) are still under the baked root and correctly skip the gate. Constraint proven by `current_test_binary_is_dev_build_via_baked_root` passing under `cargo test --release`.

verification:
  - Build: `cargo build -p nono-cli --bin nono` clean (Windows native).
  - Tests (debug AND release): `broker_authenticode_layout_tests` 6/6 pass, including the two required R-B4 regressions (`attacker_lookalike_target_path_is_not_dev_build`, `real_baked_target_root_is_dev_build`), the production-enforce test, two fail-closed tests, and the live-baked-root `cargo test --release` constraint test.
  - Clippy (native Windows): `cargo clippy -p nono-cli --bin nono --all-targets -- -D warnings -D clippy::unwrap_used` clean.
  - Cross-target clippy: PARTIAL — `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin` Rust targets are installed but the C cross-toolchains (`x86_64-linux-gnu-gcc` / `cc`) required by `ring`/`aws-lc-sys` build scripts are NOT, so cross clippy cannot complete from this Windows host. Deferred to live CI per .planning/templates/cross-target-verify-checklist.md. Low risk: the changed runtime code is entirely inside the `#[cfg(target_os = "windows")]`-gated `exec_strategy_windows` module (not compiled on Unix); the only cross-target change is `build.rs`, which uses std-only, unconditional logic.

files_changed:
  - crates/nono-cli/build.rs (bake NONO_DEV_TARGET_ROOT)
  - crates/nono-cli/src/exec_strategy_windows/launch.rs (provenance-based is_dev_build_layout + is_under_dev_target_root, updated call-site comments, rewritten + expanded regression tests)
