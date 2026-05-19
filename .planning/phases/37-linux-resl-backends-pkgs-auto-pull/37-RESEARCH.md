# Phase 37: Linux RESL backends + PKGS auto-pull - Research

**Researched:** 2026-05-19
**Domain:** Linux cgroup v2 verification on GitHub Actions Ubuntu 24.04 runners + cargo-install-style auto-pull for registry profiles with Sigstore keyless e2e
**Confidence:** HIGH (most surfaces verified against in-tree code; 1 confirmed gap requiring a string-fix sub-plan; 2 LOW-confidence items flagged for user validation)

## Summary

Phase 37 closes a 3-year Linux silent-no-op for `--memory` / `--cpu-percent` / `--max-processes` by **verifying** code that already exists in `crates/nono-cli/src/exec_strategy/supervisor_linux.rs::cgroup` on a real Linux host (GitHub Actions `ubuntu-24.04`), and ships two missing micro-features: a typed `NonoError::UnsupportedKernelFeature` variant for cgroup-v1 hosts and a `--no-auto-pull` flag for registry-profile resolution. The PKGS-04 e2e test exercises the Plan 26-02 `is_registry_ref` / `load_registry_profile` plumbing end-to-end against a CI-time keyless-signed ephemeral pack served by a multi-endpoint extension of the std-only TCP server fixture pattern.

**Three confirmed findings shape planner scope:**

1. **`nono inspect` Limits-block strings DO NOT match the LOCKED acceptance strings** (verified at `crates/nono-cli/src/session_commands.rs:546-565`). Current emission is `cpu: {pct}% (hard cap)` / `memory: {bytes_human} (job-wide)` / `procs: {N} (active)`. LOCKED strings are `cpu_percent: 25 (cgroup v2 cpu.max 25000 100000)` / `memory: 100M (cgroup v2 memory.max)` / `max_processes: 5 (cgroup v2 pids.max)`. A string-fix sub-plan IS required and is NOT optional. Per success criteria #1-3 (LOCKED in ROADMAP) and REQ-RESL-NIX-01..03 acceptance #2, these strings are part of phase-close.
2. **The current `NonoError::UnsupportedPlatform("cgroup_v2: ...")` detection site at `supervisor_linux.rs:881-886, 889-893, 896-900, 957-962, 967-971` is the exact replacement target for D-08.** Five distinct call sites currently emit `UnsupportedPlatform`; D-08 says only fire `UnsupportedKernelFeature` when the user passes a resource flag, meaning the detection-point split must distinguish "detection failure during a resource-bearing run" from "detection failure during a non-resource run". Today, `CgroupSession::detect()` is called unconditionally inside `CgroupSession::new()`, which is only invoked when `limits.has_any_limit()` is true — so the existing call shape ALREADY only fires on resource-flag invocations. The variant swap is mechanical; no extra dispatch logic needed.
3. **GitHub Actions `ubuntu-24.04` runs systemd 255.4 as PID 1 with cgroup v2 default.** `loginctl enable-linger <runner-user>` + `machinectl shell <runner-user>@.host` is the canonical incantation per the systemd CGROUP_DELEGATION.md docs and `runc` cgroup-v2 docs, but **default Ubuntu user delegation is `memory pids` ONLY — `cpu` is NOT delegated by default**. Phase 37 MUST install a systemd drop-in (`/etc/systemd/system/user@.service.d/delegate.conf`) before running the cpu-related test, or `CgroupSession::apply_limits` will fail writing `cpu.max`.

**Primary recommendation:** Plan 6 sub-plans organized as Waves 1 (parallel) → 2 (parallel) → 3 (gate):

- **Plan 37-01** (Wave 1): `NonoError::UnsupportedKernelFeature` variant + FFI map + 5-site detection swap in `supervisor_linux.rs::cgroup`. No CI dependency. ~15 min.
- **Plan 37-02** (Wave 1): `--no-auto-pull` flag + `NONO_NO_AUTO_PULL` env + `ProfileResolverArgs` flatten + `ResolveContext` parameter threaded into `profile/mod.rs::load_profile`. No CI dependency. ~25 min.
- **Plan 37-03** (Wave 1): `nono inspect` Limits-block string-fix sub-plan — rewrite the 4 `println!` arms in `session_commands.rs::run_inspect` (and Windows mirror in `session_commands_windows.rs`) to match LOCKED success-criteria strings. Includes platform-aware format strings (the LOCKED strings reference `cgroup v2 cpu.max`; on Windows these would be wrong, so the format must be platform-aware OR the test must be Linux-gated and the inspect output platform-aware). ~25 min.
- **Plan 37-04** (Wave 2): `.github/workflows/phase-37-linux-resl.yml` — new workflow file with `runs-on: ubuntu-24.04`, two jobs (`resl-nix` + `pkgs-auto-pull`), `loginctl enable-linger` + cgroup `cpu` delegate drop-in setup, registered as required check. Depends on 37-01 + 37-03 for the variant + strings to land first. ~35 min.
- **Plan 37-05** (Wave 2): `crates/nono-cli/tests/auto_pull_e2e_linux.rs` new integration test + extension of std-only TCP server to multi-endpoint. Includes CI-time keyless sign-blob step that produces the ephemeral signed pack. Depends on 37-02 for `--no-auto-pull` (acceptance #4). ~45 min.
- **Plan 37-06** (Wave 3 gate): TUF-trust-root flake triage — the 2 pre-existing `load_production_trusted_root_succeeds` + `verify_bundle_with_invalid_digest` failures from Plan 26-02 SUMMARY. Either: (a) absorb as a Phase 37 fix-pass commit if root cause is sigstore-rs version drift since 2026-05-09, or (b) bypass via test-only trust root if root cause is upstream Sigstore TUF data and not fixable in-tree, with a follow-up sub-plan deferred to v2.6. Researcher recommends triage commit before declaring 37-05 green. ~10-30 min depending on (a) vs (b).

If TUF flakes turn out to be unfixable in-tree (Plan 37-06 path (b)), Plan 37-05 fixture strategy MUST flip to a test-only trust root via the `NONO_TEST_HOME` seam (Phase 27.1) — and the `production trust root` posture of D-15 becomes a follow-up for v2.6 with the real production-registry monitoring test instead.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** New dedicated `.github/workflows/phase-37-linux-resl.yml` workflow file (NOT bolted onto `ci.yml`). Pins `runs-on: ubuntu-24.04` per REQ-RESL-NIX-01..03 acceptance #4 (current `ci.yml` jobs use `ubuntu-latest`). Cleanest blast-radius isolation; allows independent re-runs.

**D-02:** cgroup-v2 user-delegation is set up via `loginctl enable-linger <runner-user>` + `machinectl shell <runner-user>@.host` so the tests run under a real systemd-user-session, exercising the unprivileged user-delegated cgroup code path. **Not** running tests as root (would mask delegation bugs and bypass the actual production code path).

**D-03:** Workflow is a **required check on PRs to main** (blocking merge on red). Matches REQ-RESL-NIX-01 acceptance #4 + the v2.5 baseline-aware gate posture inherited from Phase 41 close.

**D-04:** Two-job split, **not** matrix-per-backend: `resl-nix` job runs `cargo test -p nono-cli --test resl_nix_linux --test resl_nix_async_signal_safety`; `pkgs-auto-pull` job runs the new `auto_pull_e2e_linux` integration test. Reflects the two distinct REQ families (RESL-NIX vs PKGS) and isolates the signed-artifact fixture path from the cgroup path.

**D-05:** **Net-new variant** `NonoError::UnsupportedKernelFeature { feature: String, hint: String }` added to `crates/nono/src/error.rs`. Distinct from existing `UnsupportedPlatform(String)` (whole-platform missing) and `NotSupportedOnPlatform { feature }` (feature missing on this OS); new variant is for "this OS supports the feature, but the kernel is misconfigured". Matches REQ-RESL-NIX-01 acceptance #3 verbatim.

**D-06:** FFI mapping: `UnsupportedKernelFeature { .. } => NonoErrorCode::ErrUnsupportedPlatform` in `bindings/c/src/lib.rs` (reuses the existing FFI code; FFI consumers distinguish via `nono_last_error()` message string). Mirrors the Phase 25-01 precedent for `NotSupportedOnPlatform`. **No new FFI error code** added.

**D-07:** Hint text on cgroup-v1 host (single line, minimal): `"cgroup v2 required; boot with systemd.unified_cgroup_hierarchy=1 or cgroup_no_v1=all"`. No docs link; no diagnostic shell-command sub-line; intentionally short to fit one terminal row of `nono`'s diagnostic-footer output.

**D-08:** Detection point: **at sandbox setup, pre-fork, per resource flag**. Only fire `UnsupportedKernelFeature` when the user actually passes `--memory` / `--cpu-percent` / `--max-processes` on a cgroup-v1 host. Other `nono` invocations on a v1 host still work (e.g., pure `--allow` grants without resource limits). Matches v2.3 Plan 25-01 fail-fast precedent (the existing `UnsupportedPlatform("cgroup_v2: ...")` site is the replacement target).

**D-09:** Scope: only `nono run` + `nono wrap`. These are the two subcommands where profile resolution happens implicitly during user execution. `nono pull` (direct install) intentionally does **not** get the flag — it's an explicit-install command where opt-out makes no sense.

**D-10:** Env var counterpart: `NONO_NO_AUTO_PULL=1` is honored. CLI flag takes precedence over env var (clap default behavior). Mirrors the existing `NONO_LOG` / `NONO_NO_UPDATE_CHECK` / `NONO_UPDATE_URL` convention (per CLAUDE.md § Configuration).

**D-11:** Fallback behavior: existing `profile not found` error string verbatim (matches REQ-PKGS-04 acceptance #4 literal wording), **plus** a `DiagnosticFormatter` footer line indicating `--no-auto-pull` is set so users can self-diagnose without a separate error variant.

**D-12:** Structural placement: new `ProfileResolverArgs` struct in `cli.rs` with `no_auto_pull: bool`, flattened into both `RunArgs` and `WrapArgs` via `#[clap(flatten)]`. Threaded into `profile/mod.rs::load_profile` via a new `ResolveContext` parameter (not a thread-local, not a global). Sets up future profile-resolver options to slot in without re-plumbing the same path.

**D-13:** Signed fixture pack is **generated + signed at CI time** using `sigstore-sign` keyless with the GitHub Actions OIDC token (the same flow Phase 32 sigstore-integration shipped). Hermetic; tests verify the same crypto path real users hit. Avoids check-in TTL / Rekor staleness problems.

**D-14:** HTTP surface uses the **std-only single-shot TCP server pattern** Phase 26-02 already established in `registry_client::tests` (50 LOC, no new dev-deps). Extended to serve a multi-endpoint mock registry (bundle.json + manifest.json + artifact). NO `mockito` dev-dep (Phase 26-02 deliberately avoided it under portable-subset; Phase 37 holds the line).

**D-15:** Trust root: **production Sigstore trust root** + GitHub Actions OIDC issuer pin (`https://token.actions.githubusercontent.com`). Most realistic; exercises the same verification path production users hit. **Prerequisite:** the 2 pre-existing `load_production_trusted_root_succeeds` / `verify_bundle_with_invalid_digest` TUF flakes documented in Plan 26-02 SUMMARY must be addressed (or confirmed environmental + not blocking) before this test can be green; researcher should investigate and plan accordingly.

**D-16:** Test placement: new `crates/nono-cli/tests/auto_pull_e2e_linux.rs` integration test (Linux-gated via `#[cfg(target_os = "linux")]`). Mirrors the existing `resl_nix_linux.rs` pattern. Invokes the `nono` binary via the integration-test harness; covers REQ-PKGS-04 acceptance #1 (happy path), #2 (unknown-name fail-closed), #3 (signature-failure abort), #4 (`--no-auto-pull` fallback).

### Claude's Discretion

1. Researcher decides whether the 2 pre-existing TUF-trust-root test flakes need their own sub-plan or can be absorbed as a Phase 37 fix-pass commit (see D-15 prerequisite).
2. Planner decides whether `nono inspect` Limits-block string drift (success criteria #1–3 exact strings) gets a Phase 37 plan or a follow-up — depends on what the existing code emits today.
3. Researcher confirms whether Ubuntu 24.04's default systemd-user-session provides cgroup-v2 delegation out-of-the-box such that `loginctl enable-linger` is sufficient, or whether additional cgroup-delegation config is required (D-02 implementation detail).
4. Planner decides whether the `phase-37-linux-resl.yml` workflow gets a path-filter so it only fires on Linux-touching PRs, or always runs. Memory `feedback_clippy_cross_target` argues for always-on; CI minute budget may argue for path-filter.

### Deferred Ideas (OUT OF SCOPE)

- **macOS `setrlimit` portion of Plan 25-01** — Already deferred at v2.5 scoping (REQUIREMENTS.md explicit). Existing `supervisor_macos.rs` code is kept on disk; macOS host UAT belongs in v2.6+.
- **Phase 38 REQ-AAHX-HOST-01 native re-validation** — Depends on Phase 37 native UAT; pre-deferred to v2.6 per REQUIREMENTS.md.
- **Mockito dev-dep** — Phase 26-02 deliberately avoided; D-14 holds the line.
- **Net-new FFI error code `ErrUnsupportedKernel`** — D-06 chose to reuse `ErrUnsupportedPlatform`.
- **Real `registry.nono.sh` as e2e source** — D-13 chose ephemeral CI-signed pack instead.
- **Path-filtered workflow trigger** — D-01 didn't lock whether `phase-37-linux-resl.yml` always runs or path-filters to Linux-touching PRs.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-RESL-NIX-01 | Linux cgroup v2 memory cap (`--memory`) via `memory.max` + `cgroup.kill`. Acceptance: kernel-OOM-kills 200MB allocation when cap is 100M; `nono inspect` shows `memory: 100M (cgroup v2 memory.max)`; fail-closed on v1 hosts with `UnsupportedKernelFeature` and `cgroup_no_v1` hint; GitHub Actions Linux runner verifies. | Existing `CgroupSession::apply_limits` writes `memory.max` already (`supervisor_linux.rs:1063-1071`). Existing `resl_nix_linux.rs::linux_memory_limit_oom_kills_child` test exercises this. Needs (a) string fix in `session_commands.rs::run_inspect` (current `memory: {human} (job-wide)` ≠ LOCKED `memory: 100M (cgroup v2 memory.max)`), (b) D-05 error variant swap, (c) Plan 37-04 CI workflow. |
| REQ-RESL-NIX-02 | Linux cgroup v2 CPU cap (`--cpu-percent`) via `cpu.max <quota> <period>`. Acceptance: `yes >/dev/null` averages ~25% CPU; `nono inspect` shows `cpu_percent: 25 (cgroup v2 cpu.max 25000 100000)`; fail-closed on v1; GitHub Actions verifies. | Existing `CgroupSession::apply_limits` writes `cpu.max` with `quota = percent * 100000 / 100` format (`supervisor_linux.rs:1072-1089`). Existing `resl_nix_linux.rs` has no CPU-percent test today — Plan 37-04 MUST add one OR confirm the existing 4 tests are sufficient. Needs string fix + variant swap + `cpu` controller delegated via systemd drop-in (default Ubuntu user delegation does NOT include cpu controller — see Common Pitfalls below). |
| REQ-RESL-NIX-03 | Linux cgroup v2 process count cap (`--max-processes`) via `pids.max`. Acceptance: 5-process cap contains a fork bomb; `nono inspect` shows `max_processes: 5 (cgroup v2 pids.max)`; fail-closed on v1; GitHub Actions verifies. | Existing `CgroupSession::apply_limits` writes `pids.max` (`supervisor_linux.rs:1090-1098`). Existing `resl_nix_linux.rs::linux_max_processes_blocks_eleventh_fork` covers the fork-bomb case at `--max-processes 10`. Needs string fix + variant swap. |
| REQ-PKGS-04 | `load_registry_profile` auto-pull on `--profile` reference; `--no-auto-pull` flag falls back to legacy `profile not found`. Acceptance: registry profile auto-pulls + verifies + runs; unknown name fails closed; signature-failure aborts; `--no-auto-pull` falls back; GitHub Actions verifies. | Existing `is_registry_ref` + `load_registry_profile` shipped in Plan 26-02 (`profile/mod.rs:2179-2330`). Auto-pull dispatcher routes to `package_cmd::run_pull` at L2240-2246. Needs (a) `ProfileResolverArgs` + `--no-auto-pull` plumbing (D-12), (b) `auto_pull_e2e_linux.rs` test (D-16), (c) D-13 keyless CI signing step, (d) D-14 multi-endpoint mock server extension. |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **No `.unwrap()` / `.expect()`** in production code (`clippy::unwrap_used` enforced). `#[allow(clippy::unwrap_used)]` permitted ONLY in test modules.
- **No `#[allow(dead_code)]`** — orphan code must be deleted or wired ("lazy use of dead code" rule).
- **`NonoError` propagation via `?`** — never panic on expected error conditions.
- **Path security:** path-component comparison only, never string `starts_with`. Validate + canonicalize at the enforcement boundary. Existing `cgroup` module already enforces this at `supervisor_linux.rs:903-934`.
- **Tests that touch `HOME` / `TMPDIR` / `XDG_CONFIG_HOME`** must save and restore the original via the EnvGuard / RAII save-restore pattern (Phase 35-02 precedent). Rust tests run in parallel within the same process; unrestored env vars cause flaky failures in unrelated tests.
- **Cross-target clippy required for cfg-gated Unix code** — `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` MUST run before close per memory `feedback_clippy_cross_target`. Windows-host workspace clippy alone is insufficient.
- **Commits must include DCO sign-off** (`Signed-off-by: Name <email>`).
- **GSD Workflow Enforcement:** all repo edits go through a GSD command; direct edits without GSD are forbidden unless user explicitly bypasses.
- **Workspace touches 5 crates** (per memory `project_workspace_crates`): `nono`, `nono-cli`, `nono-proxy`, `nono-shell-broker`, `nono-ffi` (`bindings/c/`). The error variant addition in Plan 37-01 touches `nono` (error.rs) + `nono-ffi` (bindings/c/src/lib.rs) — 2 crates. If any of `nono-py` / `nono-ts` external bindings have an exhaustive match on `NonoError` strings (not just FFI codes), Plan 37-01 may also need to touch those repos — see Integration Points below.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| cgroup v2 detection + setup | CLI supervisor (parent process, pre-fork) | — | All resource enforcement is kernel-mediated. Detection runs in `supervisor_linux.rs::cgroup::CgroupSession::new()` which is called from the parent process before fork; the child only places itself in the already-configured cgroup via the pre_exec hook. |
| `memory.max` / `cpu.max` / `pids.max` enforcement | Linux kernel (cgroup v2 controllers) | CLI supervisor (writes the values + writes `cgroup.kill` for timeout) | Kernel-enforced — `nono` only configures the cgroup. The kernel kills via OOM or rejects `fork()` via `EAGAIN`. |
| Async-signal-safe child placement | Forked child, post-fork pre-exec | — | Only raw `libc::write()` / `libc::open()` / `libc::close()` post-fork; allocation forbidden (`place_self_in_cgroup_raw` precedent). |
| `UnsupportedKernelFeature` detection point | CLI supervisor (parent, pre-fork, per resource-flag invocation) | Library (`nono::NonoError` enum) | The variant lives in `crates/nono/`; the dispatch decision (which variant to construct) lives in the CLI supervisor — only the CLI knows which flags the user passed. |
| FFI error mapping | `bindings/c/` (nono-ffi crate) | — | `map_error` in `lib.rs` exhaustively matches every `NonoError` variant; adding a new variant without updating this match fails workspace `cargo check`. Verified Phase 25-01 lesson. |
| `--no-auto-pull` flag parsing | `nono-cli` (clap derive) | — | Pure CLI plumbing — flattens into existing `RunArgs` + `WrapArgs`. No library change. |
| Auto-pull dispatch decision | `nono-cli` (profile/mod.rs::load_profile) | `nono-cli` (package_cmd::run_pull for the actual fetch) | Decision lives at the dispatch boundary; the fetch reuses existing Plan 26-02 infrastructure. |
| Sigstore keyless signing at CI time | GitHub Actions workflow (calls `cargo run -p sigstore-sign --example sign_blob`) | — | The CI workflow generates the fixture pack at job start; the test verifies it through the same code path real users hit. |
| Multi-endpoint mock registry server | `nono-cli` test code (std-only TCP) | — | Std-only TCP server pattern from Phase 26-02 extends to dispatch on request path (`/api/v1/packages/...` for pull-response JSON vs the artifact URL). No new dev-dep. |
| Production Sigstore trust root verification | `nono::trust` module (existing in `crates/nono/`) | `nono-cli` (load_production_trusted_root call site) | The same code path real users hit. D-15 prerequisite: 2 pre-existing flakes from Plan 26-02 SUMMARY MUST be triaged first. |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `landlock` | 0.4 (existing) | Linux LSM bindings for filesystem capability enforcement (Phase 37 doesn't touch this directly, but the cgroup setup happens in the same `supervisor_linux.rs` file) | Already in `crates/nono/Cargo.toml`; pure-Rust safe wrapper around the kernel ABI |
| `nix` | 0.27+ (existing) | POSIX bindings used in `supervisor_linux.rs::cgroup` for `libc::write` / `libc::open` / `libc::close` calls in the async-signal-safe `place_self_in_cgroup_raw` | Already in `crates/nono-cli/Cargo.toml` with the `resource` feature for `setrlimit`. Cgroup code uses `nix::libc` re-exports. |
| `clap` | 4.x (existing) | CLI argument parsing; `#[clap(flatten)]` is the standard pattern for sharing arg groups between subcommands | Already in workspace; `RunArgs` + `WrapArgs` already use `flatten` for `SandboxArgs`. D-12 `ProfileResolverArgs` follows the same pattern. |
| `ureq` | 3.x (existing) | HTTP client for registry calls — Plan 26-02 already uses `Agent::config_builder` with 4 timeout knobs and `with_config().limit()` body cap | Already in `nono-cli/Cargo.toml`. Streaming download to `tempfile::TempDir` is the established pattern. |
| `tempfile` | 3 (existing) | Per-pull staging directory with RAII Drop cleanup | Already in `nono-cli/Cargo.toml`. Used by `VerifiedDownloads::_tempdir`. |
| `serde_json` | 1 (existing) | Pack manifest + bundle.json + pull-response JSON serialization | Already in workspace. |
| `sigstore` (`sigstore-verify` / `sigstore-sign`) | (existing, see Cargo.lock) | Sigstore attestation + verification | Already in `crates/nono/Cargo.toml`. D-13 uses `sigstore-sign` at CI time via `cargo run -p sigstore-sign --example sign_blob` per upstream's documented pattern. |
| `thiserror` | 1 (existing) | `NonoError` derive macros | Already in `crates/nono/Cargo.toml`. New variant follows existing `#[error(...)]` pattern. |
| `tracing` | 0.1 (existing) | Structured logging — used for cgroup `Drop` warnings | Already in workspace. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `sha2` | (existing in test deps) | SHA-256 for the std-only TCP server fixture | Already used in `registry_client::tests` |
| `serde` | 1 (existing) | Serialization for any new test fixtures | Already in workspace |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Extend std-only TCP server to multi-endpoint | Add `mockito` dev-dep | Phase 26-02 deliberately avoided `mockito`; D-14 holds the line. Multi-endpoint extension is ~30-50 additional LOC matching on the parsed request URI. |
| `cargo run -p sigstore-sign --example sign_blob` at CI time | Pre-signed fixture checked into the repo | Pre-signed fixtures have Rekor TTL and trust-root rotation problems; D-13 chose CI-time keyless to avoid them. |
| Net-new `NonoErrorCode::ErrUnsupportedKernel` FFI code | Reuse `ErrUnsupportedPlatform` (D-06 choice) | D-06 chose reuse; future phase can add the new code without breaking ABI if external bindings demand programmatic distinction. |
| `nono inspect` platform-aware emission with feature-flagged strings | Single platform-agnostic emission with the cgroup-v2 substring on all platforms | Linux-only success criteria; Windows + macOS need different format (or "n/a" suffix). Plan 37-03 must decide between cfg-gated strings vs always-emit-cgroup-v2-text-regardless. Recommendation: cfg-gated, per CLAUDE.md "explicit over implicit". |

### Installation

All dependencies are ALREADY in the workspace `Cargo.toml`. **No new runtime or dev-deps are added by Phase 37.**

**Version verification (verified during research, 2026-05-19):**
- `nix` family / `landlock` / `ureq` / `tempfile` / `serde_json` / `sigstore-*` / `thiserror` / `tracing` versions are pinned in workspace `Cargo.toml`. No new crates needed; D-14 explicitly avoids `mockito`.

## Architecture Patterns

### System Architecture Diagram

```
                  ┌─────────────────────────────────────────────────────┐
                  │              GitHub Actions ubuntu-24.04            │
                  │  (systemd 255.4 as PID 1; cgroup v2 default)        │
                  │                                                     │
                  │   .github/workflows/phase-37-linux-resl.yml (NEW)   │
                  │                                                     │
                  │   ┌─────────────────────┐    ┌──────────────────┐   │
                  │   │   Setup steps:      │    │  Sigstore Fulcio │   │
                  │   │   apt deps,         │───►│  (OIDC issuer:   │   │
                  │   │   dbus-user-session,│    │  token.actions.  │   │
                  │   │   loginctl enable-  │    │  githubuser-     │   │
                  │   │   linger,           │    │  content.com)    │   │
                  │   │   cpu-controller    │    └────────┬─────────┘   │
                  │   │   delegate.conf     │             │             │
                  │   └──────────┬──────────┘             │             │
                  │              │                        ▼             │
                  │              │              CI-time `sigstore-sign` │
                  │              │              ──► ephemeral signed    │
                  │              │                  policy pack         │
                  │              │                                      │
                  │              ▼                                      │
                  │   ┌───────────────────┐     ┌──────────────────┐    │
                  │   │ Job 1: resl-nix   │     │ Job 2: pkgs-auto │    │
                  │   │  machinectl shell │     │  -pull           │    │
                  │   │  runner@.host →   │     │  runs nono run   │    │
                  │   │  cargo test       │     │  --profile X     │    │
                  │   │  resl_nix_linux + │     │  against multi-  │    │
                  │   │  async_signal_    │     │  endpoint mock   │    │
                  │   │  safety           │     │  TCP server      │    │
                  │   └─────────┬─────────┘     └────────┬─────────┘    │
                  └─────────────┼────────────────────────┼──────────────┘
                                │                        │
                                ▼                        ▼
   ┌──────────────────────────────────────────────────────────────────────────┐
   │                            nono binary                                   │
   │                                                                          │
   │  nono run --memory 100M -- <cmd>          nono run --profile X -- <cmd>  │
   │                  │                                       │               │
   │                  ▼                                       ▼               │
   │  ┌──────────────────────────────┐         ┌────────────────────────────┐ │
   │  │  RunArgs (cli.rs)            │         │  RunArgs (cli.rs)          │ │
   │  │   memory: Some(100M)         │         │   ProfileResolverArgs      │ │
   │  │   ResourceLimits.has_any()   │         │     no_auto_pull: bool     │ │
   │  └──────────────┬───────────────┘         └─────────────┬──────────────┘ │
   │                 │                                       │                │
   │                 ▼                                       ▼                │
   │  ┌──────────────────────────────┐         ┌────────────────────────────┐ │
   │  │  exec_strategy.rs            │         │  profile/mod.rs::          │ │
   │  │   apply_resource_limits_unix │         │   load_profile             │ │
   │  │                              │         │    + ResolveContext        │ │
   │  │                              │         │                            │ │
   │  │   ┌──────────────────────┐   │         │   is_registry_ref(name)?  ─┼─┐
   │  │   │ CgroupSession::new() │   │         │   ──► load_registry_      │ │
   │  │   │  detect() →          │   │         │       profile()           │ │
   │  │   │  ┌────────────────┐  │   │         │                            │ │
   │  │   │  │ cgroup v2 OK?  │  │   │         │   if no_auto_pull &&      │ │
   │  │   │  │ ├─ YES: setup  │  │   │         │     not installed:        │ │
   │  │   │  │ └─ NO:         │  │   │         │       Err(ProfileNot-     │ │
   │  │   │  │   UnsupportedKernel-  │         │       Found) + Diag-      │ │
   │  │   │  │   Feature {feat,hint} │         │       Formatter footer    │ │
   │  │   │  └────────────────┘  │   │         └────────────────────────────┘ │
   │  │   │                      │   │                                       │
   │  │   │  apply_limits():     │   │              package_cmd::run_pull() ─┼─┐
   │  │   │   write memory.max,  │   │                                       │ │
   │  │   │   cpu.max, pids.max  │   │                                       │ │
   │  │   │  install_pre_exec()  │   │                                       │ │
   │  │   └──────────┬───────────┘   │                                       │ │
   │  │              │ fork() + exec │                                       │ │
   │  │              ▼               │                                       │ │
   │  │   ┌──────────────────────┐   │                                       │ │
   │  │   │ Child (post-fork):   │   │                                       │ │
   │  │   │ place_self_in_       │   │                                       │ │
   │  │   │ cgroup_raw()         │   │                                       │ │
   │  │   │ (async-signal-safe)  │   │                                       │ │
   │  │   └──────────────────────┘   │                                       │ │
   │  └──────────────────────────────┘                                       │ │
   │                                                                         │ │
   │                                                                         ▼ ▼
   │                                                ┌─────────────────────────────┐
   │                                                │ registry_client.rs          │
   │                                                │ NONO_REGISTRY env override  │
   │                                                │ → multi-endpoint mock TCP   │
   │                                                │   server in test            │
   │                                                │ → real registry.nono.sh in  │
   │                                                │   production                │
   │                                                └─────────────────────────────┘
   └──────────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
.github/workflows/
└── phase-37-linux-resl.yml          # NEW (D-01); two-job workflow

crates/nono/src/
└── error.rs                          # +1 variant (D-05)

bindings/c/src/
└── lib.rs                            # +1 match arm (D-06)

crates/nono-cli/src/
├── cli.rs                            # +ProfileResolverArgs (D-12)
├── profile/mod.rs                    # +ResolveContext threading (D-12)
├── session_commands.rs               # rewrite run_inspect Limits strings (success criteria #1-3)
├── session_commands_windows.rs       # mirror string changes if applicable (platform-aware)
└── exec_strategy/
    └── supervisor_linux.rs           # swap 5 UnsupportedPlatform("cgroup_v2: ...") → UnsupportedKernelFeature (D-08)

crates/nono-cli/tests/
└── auto_pull_e2e_linux.rs           # NEW (D-16); Linux-gated; multi-endpoint mock server + run_nono harness
```

### Pattern 1: Adding a `NonoError` variant — 3-surface touch

**What:** When a new `NonoError` variant lands in `crates/nono/src/error.rs`, the FFI exhaustive `match` in `bindings/c/src/lib.rs::map_error` must also gain a match arm — otherwise `cargo check --workspace` fails non-exhaustive.

**When to use:** Whenever Phase 37 Plan 37-01 adds `UnsupportedKernelFeature` (verified via Phase 25-01 SUMMARY § Deviations #1, which lists this as the exact lesson learned when `NotSupportedOnPlatform` was added).

**Example:**

```rust
// crates/nono/src/error.rs (additions)
#[derive(Error, Debug)]
pub enum NonoError {
    // ... existing variants ...

    /// A kernel feature required for a specific operation is not configured
    /// on this Linux host. Distinct from `UnsupportedPlatform` (platform-wide
    /// detection failure) and `NotSupportedOnPlatform` (feature missing on
    /// this OS by design — e.g., `--cpu-percent` on macOS).
    ///
    /// This variant is for "OS supports it; kernel is misconfigured":
    /// cgroup v2 not enabled because the host booted with cgroup v1.
    ///
    /// # Field convention
    /// - `feature`: stable machine-readable id (e.g., `"cgroup_v2"`)
    /// - `hint`: single-line operator-actionable remediation
    #[error("Kernel feature not supported: {feature} ({hint})")]
    UnsupportedKernelFeature { feature: String, hint: String },
}

// bindings/c/src/lib.rs (additions to match in map_error)
// Phase 37 D-06: kernel-misconfigured feature; reuses ErrUnsupportedPlatform
// so FFI consumers see the same code as UnsupportedPlatform but with a
// structured feature+hint field in the message string.
nono::NonoError::UnsupportedKernelFeature { .. } => NonoErrorCode::ErrUnsupportedPlatform,
```

### Pattern 2: Detection-point swap in `supervisor_linux.rs::cgroup`

**What:** Replace the 5 existing `NonoError::UnsupportedPlatform("cgroup_v2: ...")` constructions in the cgroup submodule with `NonoError::UnsupportedKernelFeature` ONLY at the detection point (`detect_from_str` + `detect()`). The other 3 sites at lines 881-893 / 957-971 should ALSO swap because they represent the same semantic ("cgroup v2 detection failed") — D-08 says "pre-fork, per resource flag", and that condition is satisfied because `CgroupSession::new` (and therefore `detect()`) is only called when `limits.has_any_limit()` is true.

**When to use:** Plan 37-01 mechanical refactor; the same hint text per D-07 across all 5 sites.

**Example:**

```rust
// supervisor_linux.rs::cgroup::CgroupSession::detect_from_str
return Err(NonoError::UnsupportedKernelFeature {
    feature: "cgroup_v2".into(),
    hint: "cgroup v2 required; boot with systemd.unified_cgroup_hierarchy=1 or cgroup_no_v1=all".into(),
});
```

The 5 sites in current code (`supervisor_linux.rs`):
1. Line 880-884: empty `/proc/self/cgroup`
2. Line 889-893: multi-line (v1/hybrid)
3. Line 896-900: missing `0::` prefix
4. Line 930-933: path traversal in cgroup path (KEEP as `UnsupportedPlatform` — this is an attacker-controlled `/proc` content, not a kernel-config issue. Per CLAUDE.md "fail closed", these stay as the existing variant to preserve distinction.)
5. Line 957-961, 966-968, 969-971: `read_to_string("/proc/self/cgroup")` failed / delegated path is not a directory / not accessible

**Recommendation:** sites 1, 2, 3, 5 → `UnsupportedKernelFeature`; site 4 (path-traversal guard) → keep as `UnsupportedPlatform` since the hint text would be misleading (the user can't fix attacker-controlled `/proc` content by changing boot flags). Document this distinction in the Plan 37-01 commit body.

### Pattern 3: `ProfileResolverArgs` flatten threading

**What:** New struct flattened into `RunArgs` and `WrapArgs`, then threaded into `profile/mod.rs::load_profile` via a `ResolveContext` parameter.

**Example:**

```rust
// crates/nono-cli/src/cli.rs (additions)
#[derive(Parser, Debug, Clone)]
pub struct ProfileResolverArgs {
    /// Disable cargo-install-style auto-pull when --profile references a
    /// registry pack not yet installed locally. Falls back to the legacy
    /// "profile not found" error.
    #[arg(long, env = "NONO_NO_AUTO_PULL", help_heading = "PROFILE")]
    pub no_auto_pull: bool,
}

// In RunArgs:
#[command(flatten)]
pub profile_resolver: ProfileResolverArgs,

// In WrapArgs:
#[command(flatten)]
pub profile_resolver: ProfileResolverArgs,

// crates/nono-cli/src/profile/mod.rs (additions)
#[derive(Debug, Clone, Default)]
pub struct ResolveContext {
    pub no_auto_pull: bool,
}

pub fn load_profile_with_context(
    name_or_path: &str,
    ctx: &ResolveContext,
) -> Result<Profile> {
    if is_registry_ref(name_or_path) {
        if ctx.no_auto_pull {
            // D-11: fall back to ProfileNotFound; supervisor's DiagnosticFormatter
            // adds a footer noting --no-auto-pull is set.
            return Err(NonoError::ProfileNotFound(name_or_path.to_string()));
        }
        return load_registry_profile(name_or_path);
    }
    // ... existing logic ...
}

// Keep the existing `load_profile()` as a thin wrapper:
pub fn load_profile(name_or_path: &str) -> Result<Profile> {
    load_profile_with_context(name_or_path, &ResolveContext::default())
}
```

### Pattern 4: Multi-endpoint std-only TCP server extension

**What:** Extend the existing `spawn_one_shot_server` in `registry_client::tests` (which serves a single body to a single request) into a loop that handles multiple requests dispatching on URL path. The exact same patterns (Read request, write CRLF response) — just keep the listener alive in a loop and route to different bodies.

**Example sketch (D-14):**

```rust
fn spawn_multi_endpoint_server(
    routes: HashMap<String, Vec<u8>>,  // path → body
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let url = format!("http://{}", addr);

    let handle = thread::spawn(move || {
        for accept in listener.incoming().take(routes.len() * 3) {
            // Accept up to N*3 requests so retries don't kill the test
            let Ok(mut stream) = accept else { return };
            // ... read request line ...
            // Parse path from "GET /path HTTP/1.1"
            let path = parse_path(&request_line);
            let body = routes.get(&path).cloned().unwrap_or_else(|| b"404".to_vec());
            let status = if routes.contains_key(&path) { 200 } else { 404 };
            // ... write response ...
        }
    });
    (url, handle)
}
```

### Pattern 5: GitHub Actions cgroup v2 + cpu delegation drop-in

**What:** Default Ubuntu user `user@<uid>.service` delegates `memory pids` only — NOT `cpu`. To run REQ-RESL-NIX-02's `--cpu-percent 25` test on the runner, install a drop-in BEFORE running `loginctl enable-linger`. See Common Pitfalls below for the verified default behavior.

**Example workflow step (D-02 extension):**

```yaml
- name: Install dbus-user-session and configure cgroup v2 cpu controller delegation
  run: |
    sudo apt-get update
    sudo apt-get install -y dbus-user-session
    sudo mkdir -p /etc/systemd/system/user@.service.d
    sudo tee /etc/systemd/system/user@.service.d/delegate.conf <<'EOF'
    [Service]
    Delegate=cpu cpuset io memory pids
    EOF
    sudo systemctl daemon-reload

- name: Enable lingering for runner user
  run: sudo loginctl enable-linger $USER

- name: Wait for user@$(id -u).service to be ready
  run: |
    timeout 10 bash -c 'until systemctl --user is-active default.target; do sleep 0.5; done' || true
    cat /sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service/cgroup.controllers

- name: Run RESL-NIX Linux tests under systemd user session
  run: |
    machinectl shell ${USER}@.host /usr/bin/env bash -c \
      'cd ${{ github.workspace }} && cargo test -p nono-cli --test resl_nix_linux --test resl_nix_async_signal_safety --release'
```

### Anti-Patterns to Avoid

- **Running RESL-NIX tests as root:** masks the user-delegation code path that real users hit; bypasses the `loginctl enable-linger` setup. Per D-02, the test MUST run as the runner user via `machinectl shell`.
- **Calling `format!` in async-signal-safe child branches:** the `resl_nix_async_signal_safety.rs` test asserts zero `format!` in the post-fork child arm; any Plan 37-01 swap that touches the child branch MUST preserve this invariant.
- **String path operations on `/sys/fs/cgroup`:** existing code uses component-level checks; any new path manipulation in Plan 37 MUST follow suit per CLAUDE.md.
- **Adding `#[allow(dead_code)]`:** CLAUDE.md "lazy use of dead code" rule. If the `UnsupportedKernelFeature` variant is added but no construction site exists yet, the test for the new variant's `Display` must wire it.
- **Threading `--no-auto-pull` as a global / thread-local:** D-12 explicitly says struct parameter only. Globals would create cross-test ordering bugs in Rust's parallel test runner.
- **Mutating `NONO_REGISTRY` without save/restore:** the auto-pull e2e test will set `NONO_REGISTRY` to point at the mock TCP server's `127.0.0.1:<port>`. Per CLAUDE.md, save and restore the original value (use the `EnvGuard` pattern Phase 35-02 established).
- **Pre-signing the fixture pack and checking it in:** Rekor TTL + Sigstore trust-root rotation make checked-in signed fixtures brittle. D-13 chose CI-time keyless for a reason.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| cgroup v2 detection | Custom `/proc/self/cgroup` parser | `CgroupSession::detect_from_str` + `detect()` already shipped Phase 25-01 | Has WR-03 path-traversal guard, multi-line cgroup-v1 rejection, async-signal-safe placement. |
| Signed-artifact verification | Wrap `openssl` or roll a Sigstore-bundle parser | `nono::trust::bundle::verify_bundle` (existing in `crates/nono/`) | Reuses production trust root + GitHub Actions OIDC verifier path. D-15. |
| Keyless OIDC signing in CI | `cosign` CLI + bash glue | `cargo run -p sigstore-sign --example sign_blob` (Rust workspace already present per Phase 32) | Same Rust binary that production users will run; verifies the same crypto path. |
| Multi-endpoint HTTP mocking | Add `mockito` / `wiremock` / `httpmock` dev-deps | Extend the 50-LOC std-only TCP server pattern from `registry_client::tests::spawn_one_shot_server` | Phase 26-02 deliberately avoided new dev-deps under portable-subset; D-14 holds the line. |
| `NONO_REGISTRY` URL override | Wire a new test-only env var | `NONO_REGISTRY` already exists and is the existing test seam (`registry_client.rs:311`) | Same env var production users override for self-hosted registries. |
| CLI flag → struct threading | `static AtomicBool` global | `ProfileResolverArgs` + `ResolveContext` param (D-12) | Globals break parallel test isolation. |
| systemd-user-session setup on Ubuntu 24.04 | Run tests as root or install a custom init | `loginctl enable-linger` + `dbus-user-session` + `Delegate=cpu` drop-in + `machinectl shell <user>@.host` | The canonical pattern documented by both systemd and runc upstream docs (verified via web search 2026-05-19). |
| Per-fixture TempDir cleanup | Manual `remove_dir_all` in test cleanup | `tempfile::TempDir` (already in Cargo.toml; existing pattern in `registry_client::tests`) | RAII Drop; panic-safe; pattern already verified by `tempdir_cleanup_runs_on_panic` test. |

**Key insight:** Phase 37 is "verify code that already exists." The standard stack is what's already in `Cargo.toml`. The discipline is to NOT add dependencies. Every "do I need a new tool for X?" question has an answer in the existing codebase per Phase 25-01 + Phase 26-02 + Phase 32 + Phase 35-02 + Phase 26-01 precedents.

## Runtime State Inventory

This is **not** a rename/refactor/migration phase — Phase 37 adds a new error variant, a new flag, a new workflow file, and a new integration test. Existing code is verified, not renamed. No runtime state outside the repo is affected.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — Phase 37 introduces no new persistent datastore. The mock TCP server uses ephemeral `127.0.0.1:0` (auto-allocated port). | None |
| Live service config | None — `phase-37-linux-resl.yml` IS the new live service config (CI workflow). No external service registration. | None |
| OS-registered state | The new workflow registers as a required check on PRs to main (D-03). This is a one-time GitHub branch-protection update post-merge (operator step, not a Phase 37 code change). | Operator: update branch protection to require the new workflow's job names after Phase 37 merges. |
| Secrets / env vars | `NONO_NO_AUTO_PULL` (new env var; CLI flag preferred) — pure code addition, no value stored externally. `NONO_REGISTRY` already exists for test usage. | None — both env vars are read by code; no pre-provisioning. |
| Build artifacts | None — Phase 37 adds no `pyproject.toml` / no built packages. The CI signed pack is regenerated every run. | None |

**Nothing found in 4 of 5 categories:** Verified by reading all code surfaces. Only the OS-registered state category requires a one-time operator action (branch protection) — and that's outside Phase 37's code change set.

## Common Pitfalls

### Pitfall 1: Default Ubuntu user delegation does NOT include `cpu` controller

**What goes wrong:** `cargo test --test resl_nix_linux` passes `linux_max_processes_blocks_eleventh_fork` (uses `pids.max`) and `linux_memory_limit_oom_kills_child` (uses `memory.max`) but a `--cpu-percent`-based test (which Phase 37 may need to add) silently fails because writing to `cpu.max` returns `ENOENT` (the cpu controller is not enabled in `cgroup.subtree_control` for the user's delegated subtree).

**Why it happens:** Per the systemd `CGROUP_DELEGATION.md` docs (verified 2026-05-19), default user@.service delegation grants `memory pids` ONLY. The `cpu` controller is gated behind opt-in delegation to non-root users to prevent unprivileged DoS via CPU starvation of system services. The runc cgroup-v2 docs confirm: "By default, typically only memory and pids controllers are delegated to non-root users."

**How to avoid:** Plan 37-04 MUST install a systemd drop-in BEFORE `loginctl enable-linger`:

```yaml
sudo mkdir -p /etc/systemd/system/user@.service.d
sudo tee /etc/systemd/system/user@.service.d/delegate.conf <<'EOF'
[Service]
Delegate=cpu cpuset io memory pids
EOF
sudo systemctl daemon-reload
```

**Warning signs:** A `cargo test` run where the cpu-percent test logs `ENOENT (No such file or directory)` writing to `cpu.max`, OR where `cgroup.controllers` for the delegated path shows only `memory pids` (not `cpu memory pids`). Add a workflow diagnostic step that `cat`s `/sys/fs/cgroup/.../cgroup.controllers` before running tests.

### Pitfall 2: TUF trust root flakes (D-15 prerequisite)

**What goes wrong:** Plan 26-02 SUMMARY documents 2 pre-existing failures: `nono::trust::bundle::tests::load_production_trusted_root_succeeds` and `verify_bundle_with_invalid_digest`, both with "Signature threshold of 3 not met for role root". Phase 37 CI close gate cannot be green on the same Linux runner while these are red.

**Why it happens:** Three plausible root causes (cross-verify with sigstore-rs upstream):
1. `sigstore-rs` TUF client out of date relative to upstream Sigstore trust root rotation events (web search 2026-05-19 confirms upstream issue: "sigstore-rs TUF client does not support the latest TUF spec; the Sigstore team noted they were actively working on fixing this").
2. Trust root data freshness — TUF metadata expired; fresh TUF refresh fixes it.
3. Network unavailability during test (TUF refresh requires reaching `tuf-repo-cdn.sigstore.dev`).

**How to avoid (Plan 37-06 triage):**
- **Path (a):** If sigstore-rs version has been bumped in the workspace since 2026-05-09 (Plan 26-02 close), re-run the 2 tests locally to see if they're already green. If green, no fix needed; mark "self-resolved by dependency drift" in Plan 37-06 SUMMARY.
- **Path (a, cont.):** If still red, attempt a `cargo update -p sigstore-trust-root` (or whichever sub-crate); if that fixes it, ship the version bump as a Plan 37-06 commit.
- **Path (b) fallback:** If still red after dependency bump, flip Plan 37-05's auto-pull test to use a TEST-ONLY trust root via the `NONO_TEST_HOME` seam (Phase 27.1 precedent). This trades D-15's "production trust root posture" for a green close gate. Document the trade-off in Plan 37-05 SUMMARY and queue a v2.6 follow-up to revisit when sigstore-rs upstream lands the TUF spec fix.

**Warning signs:** `cargo test --workspace` on a fresh Linux runner reports `Signature threshold of 3 not met for role root` in the failing test output. THIS IS THE SAME ERROR PHASE 26-02 SAW. Don't burn a CI run hoping it's environmental — triage via local run first.

### Pitfall 3: Async-signal-safe child branch regression

**What goes wrong:** Any change in `exec_strategy.rs` near the post-fork child arm that introduces a `format!` or `String::new()` would deadlock if the parent's allocator was locked at fork() time. Phase 37 plans 37-01 and 37-02 should NOT touch this region, but a CI Workflow plan author may inadvertently touch it.

**Why it happens:** `format!` allocates; allocators inherit lock state across `fork()`; if parent was mid-`malloc` when forking, child inherits a locked mutex and any heap allocation deadlocks.

**How to avoid:** The `resl_nix_async_signal_safety.rs` test will fail loudly if any `format!` appears between `CR-01-CHILD-ARM-START` and `CR-01-CHILD-ARM-END` sentinels. Don't disable the test; if Plan 37-01's variant swap touches code inside those sentinels, refactor to a constant `&[u8]` per the existing pattern.

**Warning signs:** `cargo test -p nono-cli --test resl_nix_async_signal_safety` fails with "expected 0 format! calls, found 1".

### Pitfall 4: `nono inspect` Limits-block string drift goes unnoticed

**What goes wrong:** The current `session_commands.rs::run_inspect` emits `memory: 100 MiB (job-wide)` not `memory: 100M (cgroup v2 memory.max)`. If Plan 37-03 is skipped, success criteria #1-3 fail when verified.

**Why it happens:** The current strings were authored before the LOCKED acceptance strings were finalized; they describe Windows Job Object semantics, not Linux cgroup-v2 semantics.

**How to avoid:** Plan 37-03 is REQUIRED, not optional. Plan must produce platform-aware strings:
- On Linux: emit the LOCKED strings (`memory: 100M (cgroup v2 memory.max)`, etc.)
- On Windows: emit the existing strings (job-wide / hard cap / active) since they describe Windows Job Object behavior accurately
- On macOS: emit appropriate setrlimit-flavored text OR mark feature as "n/a" until v2.6+ macOS UAT lands

Use `#[cfg(target_os = "linux")]` blocks around the format strings, NOT runtime checks (CLAUDE.md "explicit over implicit"). Pair with assertions in `resl_nix_linux.rs` that `cargo run --bin nono -- inspect <id>` output contains the LOCKED substrings verbatim.

**Warning signs:** Grep for `(job-wide)` / `(hard cap)` / `(active)` in `session_commands.rs::run_inspect` — if present without a `#[cfg]` gate, the strings will drift.

### Pitfall 5: External binding consumers may exhaustively match `NonoError` Display strings

**What goes wrong:** `nono-py` and `nono-ts` are separate repos that consume the C FFI. They may have code paths that string-match on `"Platform not supported:"` prefix to surface a Python/Node exception class. The new `UnsupportedKernelFeature` variant's Display starts with `"Kernel feature not supported:"` — a different prefix.

**Why it happens:** D-06 chose to map both variants to the same `NonoErrorCode::ErrUnsupportedPlatform`. Bindings have only the error CODE programmatically (which is the same) — so any exception-class differentiation has to come from the Display string parsing.

**How to avoid:** Plan 37-01 SUMMARY MUST flag this for the operator to check `nono-py` / `nono-ts` repos for any `nono_last_error()` string parsing that depends on the `"Platform not supported"` prefix. If found, those repos need a follow-up to handle the new prefix.

**Warning signs:** None at workspace `cargo check` time (the bindings are external repos). The operator should grep `nono-py` and `nono-ts` for `Platform not supported` after Plan 37-01 ships.

### Pitfall 6: Windows-host `cargo clippy --workspace` is insufficient for Linux-touching code

**What goes wrong:** Memory `feedback_clippy_cross_target` from Phase 25 CR-A: Windows-host `cargo clippy --workspace` doesn't exercise `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` blocks, so unused-import drift in those blocks goes undetected until CI fires.

**How to avoid:** Per CLAUDE.md, run cross-target clippy from the Windows host BEFORE close:

```bash
cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

If the cross-toolchain is not installed, document via `.planning/templates/cross-target-verify-checklist.md` and mark the verification REQ as PARTIAL.

**Warning signs:** A clean Windows `cargo clippy --workspace` followed by red Linux Clippy in CI on `#[cfg(target_os = "linux")]` code that Phase 37 touched.

## Code Examples

### Pull-pattern: cgroup setup → fork → child place-self

Already in `supervisor_linux.rs` (verified, lines 1128-1148, 1164-1204). Plan 37 does NOT modify this pattern — it inherits it intact.

### Pattern for the auto-pull e2e test fixture serving + verification

```rust
// crates/nono-cli/tests/auto_pull_e2e_linux.rs (NEW per D-16)
#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

const NONO_BIN: &str = env!("CARGO_BIN_EXE_nono");

/// REQ-PKGS-04 acceptance #4: --no-auto-pull falls back to ProfileNotFound.
#[test]
fn auto_pull_no_auto_pull_flag_falls_back_to_profile_not_found() {
    // Set NONO_REGISTRY to a non-existent server so an accidental auto-pull
    // would fail loudly (instead of silently proxying to real registry).
    std::env::set_var("NONO_REGISTRY", "http://127.0.0.1:1");
    let _guard = scopeguard::guard((), |_| std::env::remove_var("NONO_REGISTRY"));

    let output = Command::new(NONO_BIN)
        .args(["run", "--no-auto-pull", "--profile", "test-ns/missing-pack", "--", "echo", "hi"])
        .output()
        .expect("failed to spawn nono");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("profile not found") || stderr.contains("Profile not found"));
    // D-11: DiagnosticFormatter footer mentions --no-auto-pull
    assert!(stderr.contains("--no-auto-pull"));
}

// REQ-PKGS-04 acceptance #1, #2, #3 follow the same shape: spin up multi-endpoint
// TCP server, set NONO_REGISTRY, run nono with --profile, assert outcome.
```

### Pattern for the workflow keyless sign step

```yaml
permissions:
  contents: read
  id-token: write  # required for GitHub Actions OIDC

jobs:
  pkgs-auto-pull:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd
      - uses: dtolnay/rust-toolchain@stable

      - name: Build nono binary
        run: cargo build --workspace --release

      - name: Sign fixture pack with sigstore-sign (keyless via GH Actions OIDC)
        run: |
          # Create the policy pack manifest + artifact
          mkdir -p target/fixture-pack
          # ... build the artifact, manifest.json, etc. ...
          cargo run --release -p sigstore-sign --example sign_blob -- \
            target/fixture-pack/artifact.tar.gz \
            -o target/fixture-pack/artifact.tar.gz.sigstore.json
        env:
          SIGSTORE_ID_TOKEN_AUDIENCE: sigstore

      - name: Run auto-pull e2e test (verifies against production trust root)
        run: cargo test -p nono-cli --test auto_pull_e2e_linux --release
        env:
          NONO_FIXTURE_PACK_DIR: ${{ github.workspace }}/target/fixture-pack
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `NonoError::UnsupportedPlatform("cgroup_v2: ...")` for cgroup v1 hosts | `NonoError::UnsupportedKernelFeature { feature: "cgroup_v2", hint: "cgroup_no_v1=all" }` | Phase 37 (D-05) | Typed feature/hint fields; reuses FFI `ErrUnsupportedPlatform` code. |
| Silent auto-pull on registry profile reference | `--no-auto-pull` / `NONO_NO_AUTO_PULL=1` opt-out + `DiagnosticFormatter` footer | Phase 37 (D-09..D-12) | Users can disable auto-pull in CI / air-gapped contexts; existing happy path unchanged. |
| Plan 26-02 deferred auto-pull e2e suite | `auto_pull_e2e_linux.rs` integration test with keyless-signed ephemeral pack | Phase 37 (D-13, D-16) | First end-to-end exercise of the auto-pull crypto path against the production trust root. |
| `nono inspect` Limits emission tuned to Windows Job Object semantics | Platform-aware emission (cgroup v2 on Linux, Job Object on Windows) | Phase 37 Plan 37-03 | Success criteria #1-3 enforcement strings; aligns acceptance with implementation. |
| Phase 25-01 Linux cgroup v2 code coded but never verified on a real Linux host | GitHub Actions `ubuntu-24.04` runner with `loginctl enable-linger` + cpu delegation drop-in | Phase 37 (D-01, D-02) | First real-host kernel-enforcement verification of memory.max / cpu.max / pids.max. |

**Deprecated/outdated:**
- Phase 16 "is not enforced on linux" warning strings: removed in Plan 25-01; the `linux_no_warnings_on_resource_flags` test enforces their absence.
- `mockito` as a contemplated dev-dep: deferred from Phase 26-02; D-14 holds the line.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `cpu` controller is NOT in the default Ubuntu user-delegated `cgroup.controllers` set on the ubuntu-24.04 runner image | Common Pitfalls #1 | If WRONG (cpu IS default-delegated): Plan 37-04's drop-in step is harmless overhead. If RIGHT: the cpu-percent test silently fails without the drop-in. (`[CITED: systemd CGROUP_DELEGATION.md, runc cgroup-v2 docs]`; web search 2026-05-19 confirms default = `memory pids`.) **Verification path:** add a diagnostic `cat /sys/fs/cgroup/.../cgroup.controllers` step to the workflow. |
| A2 | The 2 TUF flakes from Plan 26-02 SUMMARY are still red as of 2026-05-19 | Pitfall #2, Plan 37-06 | If they self-resolved via sigstore-rs version drift: Plan 37-06 is a no-op verification commit. If still red: triage path needed (a) or (b) per Plan 37-06 spec. `[ASSUMED]` — researcher did not execute `cargo test` against the live workspace in this session. **Verification path:** Plan 37-06 starts with a single `cargo test -p nono load_production_trusted_root_succeeds verify_bundle_with_invalid_digest` to ground-truth. |
| A3 | The `sigstore-sign` example `sign_blob` is bundled in the workspace and produces a `.sigstore.json` bundle file consumable by `nono::trust::bundle::verify_bundle` | Code Examples, Don't Hand-Roll | If WRONG: D-13 falls back to invoking `cosign` from a system install on the runner. `[CITED: sigstore-rs README]`; the example is documented upstream. **Verification path:** Plan 37-05 starts with a `cargo run -p sigstore-sign --example sign_blob -- --help` smoke check; if absent, switch to cosign. |
| A4 | `machinectl shell <user>@.host` is available on ubuntu-24.04 runners by default | Pattern 5 | If WRONG: `apt install systemd-container` may be required. Per CGROUP_DELEGATION.md and runc docs, machinectl is part of the systemd package family and ships by default on Ubuntu. `[CITED: systemd man pages]`; standard systemd installation. |
| A5 | The Phase 41 `CapabilityRequest::path → HandleTarget::FilePath` API migration does NOT touch the `supervisor_linux.rs::cgroup` submodule or the auto-pull profile resolver | Integration Points | If WRONG: Plan 37-01 or 37-02 may need to absorb stale API references. `[VERIFIED: grep through supervisor_linux.rs and profile/mod.rs]` — neither file references `CapabilityRequest::path` or `HandleTarget::FilePath`. The Phase 41 migration touched `exec_strategy.rs` (sibling file), not the cgroup submodule or the profile module. **Verification path:** `git diff main -- crates/nono-cli/src/exec_strategy/supervisor_linux.rs crates/nono-cli/src/profile/mod.rs` should be empty against the post-Phase-41 baseline. |
| A6 | `nono-py` and `nono-ts` external binding repos do NOT exhaustively match on `nono_last_error()` strings | Pitfall #5, Integration Points | If WRONG: D-06's "FFI consumers distinguish via Display string" assumption breaks for those bindings, requiring a follow-up commit in the external repos. `[ASSUMED]` — researcher cannot inspect external repos. **Verification path:** Plan 37-01 SUMMARY flags this for the operator to grep the external repos. |
| A7 | The 3 LOCKED `nono inspect` strings (success criteria #1-3) should only render on Linux (cfg-gated) | Plan 37-03, Anti-Patterns | If WRONG (strings should always render regardless of platform): platform-aware decision becomes a single format choice. `[ASSUMED]` based on the literal references to cgroup v2 — these terms are inaccurate on Windows / macOS. **Verification path:** Plan 37-03 must decide; recommend cfg-gated per CLAUDE.md "explicit over implicit". |

**If this table is empty:** Not empty. 7 assumptions; 3 are HIGH-confidence (cited), 4 are LOW-confidence (need verification at plan-execution time).

## Open Questions

1. **Should Plan 37-04 path-filter the workflow trigger to Linux-touching PRs only?**
   - What we know: Memory `feedback_clippy_cross_target` argues always-on (Windows host can't catch Linux drift); CI minute budget argues path-filter; existing `ci.yml` uses a `changes` job to skip unrelated PRs.
   - What's unclear: The user's CI minute budget tolerance.
   - Recommendation: **Always-on**. The whole point of Phase 37 is verifying Linux-coded-on-Windows code. A path-filter that skips when only Linux files changed defeats the purpose. The workflow is two jobs at ~5 min each; this is well under the GH Actions free-tier budget for a phase-marker workflow. Reuse the existing `ci.yml` `changes` job pattern only if budget is constrained — and then path-filter to "any change to `crates/nono*` OR `bindings/c/`" so the workflow still fires on PRs that touch Windows-only files (since cross-platform regressions can manifest there).

2. **Plan 37-03 platform-aware emission strategy — `#[cfg]` gates vs runtime branches?**
   - What we know: CLAUDE.md says "explicit over implicit"; `#[cfg]` gates are explicit and compile-time-checked. Runtime branches are testable on all platforms but can be more flexible.
   - What's unclear: Whether macOS gets the cgroup-v2 string variant when macOS-side enforcement lands in v2.6+ (then the macOS branch would emit a different string).
   - Recommendation: `#[cfg(target_os = "linux")]` for the cgroup v2 strings; `#[cfg(target_os = "windows")]` for the existing job-wide / hard cap / active strings; `#[cfg(target_os = "macos")]` for whatever setrlimit emission lands in v2.6+ (today: emit `"<feature>: <value> (n/a — macOS deprioritized v2.5)"` or similar honest placeholder). Plan 37-03 commits the Linux + Windows arms; the macOS arm gets a TODO comment pointing at v2.6.

3. **Should Plan 37-01 ALSO swap the path-traversal-guard `UnsupportedPlatform` at `supervisor_linux.rs:930-933`?**
   - What we know: D-08 says "fire `UnsupportedKernelFeature` only when user passes a resource flag"; the path-traversal guard fires inside `detect_from_str` which is called from `detect()` which is called from `CgroupSession::new` which is called only when resource flags are set. Mechanically the "user passed a flag" condition is met.
   - What's unclear: Whether the hint text "cgroup v2 required; boot with ..." makes sense when the actual failure is an attacker-controlled `/proc/self/cgroup` content. The hint would mislead the user — boot flags don't fix `/proc` manipulation.
   - Recommendation: **KEEP as `UnsupportedPlatform`** for the path-traversal site. Distinct semantic: this is "your kernel is fine, but `/proc` content is malformed/malicious", not "your kernel needs reboot". Document the distinction in Plan 37-01 SUMMARY.

4. **Should the auto-pull e2e test cover MULTIPLE pack types (Policy + future Plugin) or just Policy?**
   - What we know: REQ-PKGS-04 acceptance is profile-specific; Plan 26-02 already validates pack type as Policy before walking profile artifacts.
   - What's unclear: Whether Phase 37 should also exercise a "wrong pack type" rejection path.
   - Recommendation: Include a 5th test case `auto_pull_rejects_non_policy_pack_type` to assert the existing Plan 26-02 rejection logic still fires when the auto-pull path is taken. Cheap (~30 LOC) and exercises code that today is only covered by unit tests.

5. **Phase 41 zombie-process / `.unwrap()` clippy + 1 Linux test failure — does any of this intersect Phase 37 lanes?**
   - What we know: Memory `project_phase41_open_gaps` says 3 open CI gap classes remain (macOS pre-existing compile errors, Linux clippy zombie_processes+.unwrap(), 1 Linux test failure).
   - What's unclear: Whether the Linux test failure is in `resl_nix_linux.rs` or elsewhere.
   - Recommendation: Plan 37-04 SHOULD include a step that explicitly excludes the known-failing tests via `--skip`, OR fixes them. The baseline-aware CI gate established in Phase 41 may already mark these as load-bearing-skips; Plan 37-04 should inherit that convention. **Verification path:** before Plan 37-04 close, `gh run list --workflow=ci.yml --branch=main` to see the current red list, and ensure Plan 37-04 doesn't double-report them.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (1.77+) | All compilation | ✓ (workspace pin) | 1.77 (per CLAUDE.md "Minimum Rust version: 1.77") | — |
| `cargo` | Build + test invocations | ✓ | with toolchain | — |
| GitHub Actions `ubuntu-24.04` runner | Phase 37 CI workflow (D-01) | ✓ (verified 2026-05-19) | systemd 255.4, kernel 6.17.0-azure | — |
| systemd + `loginctl` + `machinectl` | D-02 systemd-user-session setup | ✓ (default on ubuntu-24.04) | systemd 255.4 | — |
| `dbus-user-session` package | D-02 cgroup-v2 user delegation prerequisite | ✗ (NOT installed by default on Ubuntu Server) | — | `apt install -y dbus-user-session` in workflow step |
| `cosign` CLI | Optional fallback to `sigstore-sign` Rust example if A3 wrong | ✗ (not installed on runner by default) | — | `uses: sigstore/cosign-installer@v3` action — only invoked if A3 verification fails |
| GitHub Actions OIDC token | D-13 keyless signing | ✓ (auto-injected when `id-token: write` permission set) | — | — |
| `tuf-repo-cdn.sigstore.dev` reachability | D-15 production trust root fetch | ✓ (general internet; flake risk per Pitfall #2) | — | Plan 37-06 path (b) — flip to test-only trust root via NONO_TEST_HOME seam |
| Windows host (researcher's dev machine) | Workspace clippy + cross-target clippy | ✓ | — | — |
| Linux cross-target toolchain on Windows host | `cargo clippy --target x86_64-unknown-linux-gnu` per memory `feedback_clippy_cross_target` | (researcher's machine state unknown — `[ASSUMED]` available based on prior phases using it) | — | If absent: invoke `.planning/templates/cross-target-verify-checklist.md` and mark verification REQ as PARTIAL |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:**
- `dbus-user-session` — install via apt in Plan 37-04 workflow step.
- `cosign` — install via sigstore/cosign-installer action if `sigstore-sign` example proves unavailable.
- Cross-target toolchain on Windows host — document via cross-target-verify-checklist if missing.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Cargo built-in test runner (workspace standard) |
| Config file | None — workspace `Cargo.toml` defines test discovery; `[[test]]` sections per crate |
| Quick run command | `cargo test -p nono-cli --test resl_nix_linux --release` (Linux-only; compile-out on other platforms) |
| Full suite command | `cargo test --workspace --release` (Phase 37 gate) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| REQ-RESL-NIX-01 | Memory cap kernel-OOM-kills 200M alloc when --memory=100M | integration | `cargo test -p nono-cli --test resl_nix_linux linux_memory_limit_oom_kills_child --release` | ✓ exists in `resl_nix_linux.rs` (Phase 25-01) |
| REQ-RESL-NIX-01 acceptance #2 | `nono inspect <id>` shows `memory: 100M (cgroup v2 memory.max)` | integration | New test in `resl_nix_linux.rs` invoking `nono inspect` post-run and grepping the LOCKED string | ❌ Wave 0 (Plan 37-03) |
| REQ-RESL-NIX-01 acceptance #3 | Fail-closed on v1 with `UnsupportedKernelFeature` + `cgroup_no_v1` hint | unit | `cargo test -p nono error::unsupported_kernel_feature_display_contains_cgroup_no_v1_hint` | ❌ Wave 0 (Plan 37-01) — add unit test on the new variant's Display |
| REQ-RESL-NIX-01 acceptance #4 | GitHub Actions Linux runner executes the integration test | CI | `.github/workflows/phase-37-linux-resl.yml` resl-nix job | ❌ Wave 0 (Plan 37-04) |
| REQ-RESL-NIX-02 | CPU cap throttles `yes >/dev/null` to ~25% | integration | NEW test in `resl_nix_linux.rs` measuring CPU% via top/proc sampling | ❌ Wave 0 (Plan 37-04 must add or confirm sufficient existing coverage) |
| REQ-RESL-NIX-02 acceptance #2 | `nono inspect` shows `cpu_percent: 25 (cgroup v2 cpu.max 25000 100000)` | integration | New test + Plan 37-03 string fix | ❌ Wave 0 (Plan 37-03) |
| REQ-RESL-NIX-02 acceptance #3 | Fail-closed on v1 (shares UnsupportedKernelFeature path) | (covered by REQ-01 acceptance #3) | (same) | ❌ Wave 0 (Plan 37-01) |
| REQ-RESL-NIX-03 | Fork bomb contained at --max-processes 5 | integration | `cargo test -p nono-cli --test resl_nix_linux linux_max_processes_blocks_eleventh_fork --release` (existing test uses N=10; verify N=5 case OR adjust) | ✓ exists, may need parameter adjustment |
| REQ-RESL-NIX-03 acceptance #2 | `nono inspect` shows `max_processes: 5 (cgroup v2 pids.max)` | integration | New test + Plan 37-03 string fix | ❌ Wave 0 (Plan 37-03) |
| REQ-PKGS-04 acceptance #1 | Auto-pull happy path: registry profile resolves, auto-pulls, verifies, runs | integration | `cargo test -p nono-cli --test auto_pull_e2e_linux auto_pull_happy_path --release` | ❌ Wave 0 (Plan 37-05) |
| REQ-PKGS-04 acceptance #2 | Unknown name fails closed (no implicit network) | integration | `cargo test -p nono-cli --test auto_pull_e2e_linux auto_pull_unknown_name_fails_closed --release` | ❌ Wave 0 (Plan 37-05) |
| REQ-PKGS-04 acceptance #3 | Signature-failure aborts | integration | `cargo test -p nono-cli --test auto_pull_e2e_linux auto_pull_signature_failure_aborts --release` | ❌ Wave 0 (Plan 37-05) |
| REQ-PKGS-04 acceptance #4 | `--no-auto-pull` falls back to ProfileNotFound | integration | `cargo test -p nono-cli --test auto_pull_e2e_linux auto_pull_no_auto_pull_flag_falls_back_to_profile_not_found --release` | ❌ Wave 0 (Plan 37-05) |
| REQ-PKGS-04 acceptance #5 | GitHub Actions Linux runner verifies the e2e flow | CI | `.github/workflows/phase-37-linux-resl.yml` pkgs-auto-pull job | ❌ Wave 0 (Plan 37-04) |
| Async-signal-safety regression | No new `format!` calls in post-fork child arm | structural (compile-time + grep) | `cargo test -p nono-cli --test resl_nix_async_signal_safety` | ✓ exists (Phase 25-01) |
| Cross-target clippy | Linux + macOS clippy from Windows host | static analysis | `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | (manual, per CLAUDE.md mandate) |
| TUF trust root prerequisite | Plan 26-02 SUMMARY's 2 flakes resolved or bypassed | unit | `cargo test -p nono trust::bundle::tests::load_production_trusted_root_succeeds trust::bundle::tests::verify_bundle_with_invalid_digest` | ✓ exists; Plan 37-06 triages |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli --bin nono` (local-fast — ~30s on dev machine)
- **Per wave merge:** `cargo test --workspace` + `cargo clippy --workspace --target x86_64-unknown-linux-gnu` (full pre-CI gate)
- **Phase gate:** Full suite green + new `phase-37-linux-resl.yml` workflow green on a fresh GitHub Actions run before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/nono/src/error.rs` — new test `unsupported_kernel_feature_display_contains_cgroup_no_v1_hint` (REQ-RESL-NIX-01 acceptance #3) covers Plan 37-01's variant
- [ ] `crates/nono-cli/tests/auto_pull_e2e_linux.rs` — covers REQ-PKGS-04 acceptance #1-#4 (Plan 37-05)
- [ ] `.github/workflows/phase-37-linux-resl.yml` — covers REQ-RESL-NIX-01/02/03 acceptance #4 + REQ-PKGS-04 acceptance #5 (Plan 37-04)
- [ ] `crates/nono-cli/tests/resl_nix_linux.rs` — add new test asserting `nono inspect` Limits-block LOCKED strings render verbatim post-run for memory / cpu / pids (Plan 37-03)
- [ ] CPU-percent integration test in `resl_nix_linux.rs` (REQ-RESL-NIX-02) — currently no test exercises the cpu.max path; Plan 37-04 should add (or confirm existing 4 tests are sufficient + add a CPU sampling test if needed)
- [ ] Framework install: none — Cargo test runner is workspace default
- [ ] No shared fixtures or `conftest.py` equivalent needed; tests are self-contained

## Security Domain

> v2.5 milestone has `security_enforcement` semantics inherited from prior phases (the codebase is security-critical per CLAUDE.md). This section applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (Sigstore OIDC) | GitHub Actions OIDC issuer pin (`https://token.actions.githubusercontent.com`) per D-15 |
| V3 Session Management | no | n/a — no user sessions; CLI process model |
| V4 Access Control | yes (cgroup capabilities) | Linux cgroup v2 user delegation; `Delegate=cpu cpuset io memory pids` drop-in scopes which controllers the unprivileged user can configure |
| V5 Input Validation | yes (CLI args, env vars, `/proc/self/cgroup` contents) | `clap` value parsers; `enforce_content_length` for HTTP responses; existing `WR-03` path-traversal guard in `cgroup::detect_from_str` (lines 924-934) |
| V6 Cryptography | yes (Sigstore bundle verification) | Use `nono::trust::bundle::verify_bundle` (NEVER hand-roll signature verification); production Sigstore trust root per D-15 |
| V7 Data Protection at Rest | yes (signed-artifact integrity) | `tempfile::TempDir` with RAII Drop; SHA-256 mid-stream digest comparison; content-addressable verification |
| V14 Configuration | yes (env var precedence; CLI flag > env var) | `NONO_NO_AUTO_PULL` honored but CLI flag takes precedence (clap default); matches existing `NONO_LOG` / `NONO_NO_UPDATE_CHECK` convention |

### Known Threat Patterns for {Linux cgroup v2 + Sigstore HTTPS}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| `/proc/self/cgroup` path traversal (`0::/../../etc`) | Tampering | Existing component-level `Path::starts_with` + `Component::ParentDir` rejection (verified at supervisor_linux.rs:924-934) |
| Symlink TOCTOU on cgroup paths under `/sys/fs/cgroup` | Tampering | `/sys/fs/cgroup` is a kernel-managed pseudo-FS; symlinks are not honored within it. Reliance on this is documented in the WR-03 comment block. |
| Compromised registry serving wrong signed pack | Spoofing | Sigstore signature verification — D-15 + D-13 path; `nono::trust::bundle::verify_bundle` checks identity AND issuer (per Sigstore best practices, `--certificate-identity` AND `--certificate-oidc-issuer` both required) |
| Replayed stale signature (Rekor TTL) | Repudiation | CI-time signing (D-13) — fresh signature every run, no stale fixtures in repo |
| HTTP body inflation DoS (response > 64 MiB cap) | Denial of Service | Existing `enforce_content_length` pre-check + `with_config().limit()` reader cap from Plan 26-02 |
| Connection pool exhaustion (slow body attack) | Denial of Service | Existing `ureq::Agent` 4-timeout configuration (10s connect / 30s response / 300s body / 300s global) from Plan 26-02 |
| Allocator deadlock in post-fork child branch | Reliability (T-25-03 / CR-01) | `resl_nix_async_signal_safety.rs` structural test asserts zero `format!` between sentinels |
| Unprivileged cgroup-v2 escalation via misconfigured Delegate | Elevation of Privilege | Default user delegation is `memory pids` only; Plan 37-04's `Delegate=cpu cpuset io memory pids` drop-in adds cpu (required for REQ-RESL-NIX-02) but does NOT add `net_cls` / `devices` / etc. — keeps the scope minimal. |
| Auto-pull side-channel (silent network on unknown profile name) | Information Disclosure | `is_registry_ref` discriminator (Plan 26-02) routes only well-formed `namespace/name[@version]` shapes; arbitrary names fall through to existing user-profile / built-in lookup, with `ProfileNotFound` as the final error. REQ-PKGS-04 acceptance #2 enforces. |
| `--no-auto-pull` bypass via env-var name collision | Configuration | `NONO_NO_AUTO_PULL` follows existing naming convention; clap's `env` annotation guarantees the precedence: CLI > env > default. |
| Environment-variable leakage in test parallelism | Reliability | Tests that set `NONO_REGISTRY` / `HOME` etc. MUST use the EnvGuard RAII pattern (Phase 35-02 precedent; CLAUDE.md mandate) |

## Sources

### Primary (HIGH confidence)

- `crates/nono/src/error.rs` (in-tree, line-verified) — existing `NotSupportedOnPlatform` variant precedent
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs:817-1468` (in-tree, line-verified) — full `cgroup` submodule including WR-03 traversal guard, 5 detection-site `UnsupportedPlatform` errors, RAII `Drop` cleanup, async-signal-safe child placement
- `crates/nono-cli/src/session_commands.rs:513-568` (in-tree, line-verified) — current `run_inspect` Limits-block strings (CONFIRMED NOT MATCHING LOCKED acceptance strings — drives Plan 37-03 requirement)
- `crates/nono-cli/src/profile/mod.rs:2178-2330` (in-tree, line-verified) — `load_profile` + `is_registry_ref` + `load_registry_profile` auto-pull dispatcher
- `crates/nono-cli/src/registry_client.rs:1-50, 308-313, 335-460` (in-tree, line-verified) — `RegistryClient` constructor, `NONO_REGISTRY` env override, std-only TCP server fixture pattern
- `crates/nono-cli/src/cli.rs:1098-1124, 2079-2291` (in-tree, line-verified) — `PullArgs`, `RunArgs`, `WrapArgs` shapes for D-12 `ProfileResolverArgs` placement
- `crates/nono-cli/tests/resl_nix_linux.rs` (in-tree, line-verified) — existing 5 integration tests gated on `require_cgroup_v2!`
- `bindings/c/src/lib.rs:72-144` (in-tree, line-verified) — exhaustive `map_error` match where D-06 adds the new variant arm
- `.planning/phases/25-cross-platform-resl-aipc-unix-design/25-01-RESL-NIX-SUMMARY.md` (in-tree, full read) — Plan 25-01 execution summary with the canonical FFI-mapping precedent
- `.planning/phases/26-pkg-streaming-followup/26-02-PKGS-STREAMING-SUMMARY.md` (in-tree, full read) — Plan 26-02 std-only TCP server pattern + 2 TUF-flake deferral
- `.planning/REQUIREMENTS.md` § RESL-NIX, § PKGS (in-tree) — LOCKED acceptance criteria including the literal `cgroup_no_v1` hint and the `--no-auto-pull` flag
- `.planning/ROADMAP.md` § Phase 37 (in-tree) — 6 success criteria with LOCKED inspect-block strings
- `.planning/phases/35-upst3-closure-quick-wins/35-02-LINUX-LANDLOCK-PROFILES-SUMMARY.md` (in-tree) — `EnvGuard` RAII pattern + "Linux code coded on Windows, verified on GitHub Actions" precedent
- `.github/workflows/ci.yml` (in-tree, first 250 lines verified) — existing CI workflow shape that Phase 37 adds a sibling to (not modifies)
- `CLAUDE.md` (in-tree) — coding standards, path-handling rules, cross-target clippy mandate

### Secondary (MEDIUM confidence)

- [systemd CGROUP_DELEGATION.md](https://github.com/systemd/systemd/blob/main/docs/CGROUP_DELEGATION.md) (verified 2026-05-19) — cgroup v2 delegation model + Delegate= drop-in syntax
- [opencontainers/runc cgroup-v2 docs](https://github.com/opencontainers/runc/blob/main/docs/cgroup-v2.md) (verified 2026-05-19) — confirms default Ubuntu user delegation is `memory pids` only, NOT `cpu`
- [actions/runner-images Ubuntu 24.04 README](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md) (verified 2026-05-19) — confirms systemd 255.4 as PID 1 + cgroup v2 default + image-shipped packages
- [actions/runner-images PR #5812 — Enable systemd lingering](https://github.com/actions/runner-images/pull/5812) (verified 2026-05-19) — confirms `loginctl enable-linger` is the canonical incantation for Ubuntu runners
- [Sigstore Quickstart with Cosign](https://docs.sigstore.dev/quickstart/quickstart-cosign/) (verified 2026-05-19) — keyless signing + OIDC issuer pin reference
- [sigstore-rust workspace README](https://github.com/sigstore/sigstore-rust) (verified 2026-05-19) — `cargo run -p sigstore-sign --example sign_blob` pattern; sigstore-rs experimental status flag

### Tertiary (LOW confidence — flagged for validation)

- "sigstore-rs TUF client does not support the latest TUF spec" — [Sigstore Announcement blog](https://blog.sigstore.dev/tuf-root-update/) (verified 2026-05-19) — supports Pitfall #2's hypothesis that the 2 Plan 26-02 SUMMARY flakes are upstream-tracked, but doesn't confirm the in-tree workspace's current sigstore-rs version is still affected. **Validation needed at Plan 37-06 start.**
- Memory `project_phase41_open_gaps` — "1 Linux test failure" — exact test name not specified in the memory note. **Validation needed before Plan 37-04 close.**

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries are already in `Cargo.toml`; no new deps
- Architecture: HIGH — D-01..D-16 fully specified by CONTEXT.md; researcher confirmed by reading source
- Pitfalls: HIGH for pitfalls 1, 3, 4, 6 (in-tree verification); MEDIUM for pitfalls 2, 5 (depend on out-of-tree state — TUF flakes need re-run; external bindings can't be inspected from this workspace)

**Research date:** 2026-05-19
**Valid until:** 2026-06-19 (30 days; phase target close window). Re-run pitfall #2 + assumption A2 verification at execution-time start.
