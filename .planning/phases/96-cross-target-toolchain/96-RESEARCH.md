# Phase 96: Cross-Target Toolchain - Research

**Researched:** 2026-06-26
**Domain:** Rust cross-target clippy tooling (Windows host → linux-gnu via `cross`/Docker; apple-darwin via cargo-zigbuild) + GSD verification-protocol rewrite
**Confidence:** HIGH (linux-gnu mechanism + Cross.toml + host probes verified in-session); MEDIUM (apple-darwin zig path — the SDK wall is well-evidenced but the exact failure line depends on which build-dep fires first)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Linux-gnu mechanism**
- **D-01:** `cross clippy` is the canonical mechanism. Document and run
  `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`.
  cross 0.2.5 + Docker 29.5.3 already installed; the cross image ships `x86_64-linux-gnu-gcc` so the
  C-linking crates (`aws-lc-sys`, `ring`) link cleanly. Zero new host install. (Chosen over native
  gnu-gcc linker install and over WSL2.)
- **D-02:** The `cross` form **replaces the bare `cargo clippy --target` string** in CLAUDE.md and the
  checklist. It discharges SC#2's `cargo clippy` contract because `cross clippy` runs `cargo clippy`
  *inside* the pinned Linux container — same lints, real Linux cfg branches. The bare
  `cargo clippy --workspace --target x86_64-unknown-linux-gnu ...` is NOT independently runnable on this
  host (no native linker), so it must not remain the documented "runnable" command.

**apple-darwin disposition**
- **D-03:** Time-boxed single-path attempt, then hard-blocker. Evaluate exactly ONE approach
  (cargo-zigbuild first). Capped at **one plan / one wave** of effort.
- **D-04:** Stop conditions (whichever hits first):
  (a) the bounded single-path effort produces no clean `clippy` exit by end of its plan/wave, OR
  (b) any path requires **acquiring/extracting the proprietary macOS SDK on the Windows host** — stop
  immediately on licensing grounds, do not attempt the extraction.
  On stop → write the **XTGT-03(b) hard-blocker record** and commit apple-darwin to **PARTIAL→CI**. If
  the bounded attempt *does* yield a clean clippy run, apple-darwin flips to local-runnable instead.

**Drift-fix scope**
- **D-05:** Fix ALL drift the linux-gnu gate surfaces — including upstream-inherited lints — because this
  is the first local run of the gate and SC#2 requires it to exit 0 under `-D warnings -D clippy::unwrap_used`.
  **No `#[allow(...)]` silencing** (checklist Anti-pattern 2 / CLAUDE.md Unwrap Policy); use cfg-gates,
  visibility changes, or structural fixes. No no-new-since-baseline bound — a green gate is the deliverable.

**Doc home + retirement (XTGT-04)**
- **D-06:** Setup + canonical invocation live in `.planning/templates/cross-target-verify-checklist.md`
  (it already owns the "Cross-Toolchain Setup" section and the PARTIAL disposition being retired).
  CLAUDE.md gets a **one-line pointer**, not a duplicate runbook.
- **D-07:** Retirement is per-gate and evidence-based. Rewrite the checklist decision tree so:
  - **linux-gnu** → "MUST run locally via `cross clippy`; PARTIAL only on a *documented* Docker/cross failure."
  - **apple-darwin** → disposition follows D-03/D-04's outcome (stays explicitly PARTIAL→CI *with the
    hard-blocker rationale* if the bounded attempt fails, or flips to local-required if it passes).
  Do NOT retire both gates' default unconditionally.

### Claude's Discretion
- Pinning/recording the exact `cross` image tag, plan/wave decomposition, the precise cargo-zigbuild
  invocation tried, and whether to wire the linux-gnu gate into a `make` target are the planner's/
  researcher's call, subject to the locked decisions above.

### Deferred Ideas (OUT OF SCOPE)
- **Crate leapfrog ≥0.65.0, signed MSI/wheel/npm pipeline, operator runbook** → Phase 97.
- **Cross-target verification of `nono-py` / `nono-ts`** (separate binding repos) — this phase covers the
  **workspace** cfg-gated Unix surface only.
- **Wiring the linux-gnu gate into a dedicated `make` target / CI parity job** — planner discretion, not a
  locked requirement.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| XTGT-01 | A local cross C toolchain (cross/Docker or equivalent) installed + documented (setup + invocation) so cfg-gated Unix code compiles locally. | cross 0.2.5 + Docker 29.5.3 confirmed installed; `Cross.toml` already configures the `x86_64-unknown-linux-gnu` target (libdbus pre-build). The ONLY missing runtime prerequisite is the **Docker Linux engine daemon being started** (probed NOT running this session — see Environment Availability). Setup = "start Docker Desktop, then `cross clippy …`". |
| XTGT-02 | linux-gnu clippy runs locally + passes; drift fixed in-milestone. | `cross clippy` (built-in subcommand, verified) runs `cargo clippy` inside the pinned `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5` image which ships `x86_64-linux-gnu-gcc` + libdbus → resolves the exact C-linker-absent block Phase 95 hit. Expected drift classes in § Common Pitfalls. |
| XTGT-03 | apple-darwin clippy passes locally OR documented hard-blocker → PARTIAL→CI. | cargo-zigbuild supports a `clippy` subcommand; clippy only emits `dep-info,metadata` (no final link). BUT `aws-lc-sys` build.rs runs a **compile-and-link C feature probe** (`memcmp_invalid_stripped_check.c`) that hits the Mach-O linker / macOS-SDK wall — the documented D-04(b) stop. Exact stop signatures in § apple-darwin. |
| XTGT-04 | CLAUDE.md + checklist updated to reflect local-runnable status, retiring PARTIAL→CI *default* for runnable gate(s). | Rewrite plan in § Architecture Patterns "Doc/Protocol Rewrite". |
</phase_requirements>

## Summary

This phase is toolchain-and-protocol work, not code-absorb work. The cfg-gated Unix drift surface was
created by Phase 95's absorb (HEAD `be42a5af`); Phase 95 could only mark both cross-targets PARTIAL→CI
because the Docker Linux engine wasn't running and no C cross-linker was on PATH. Phase 96's job is to
make the linux-gnu gate provably green locally, resolve apple-darwin to an explicit end-state, and rewrite
the verification protocol so the auto-default-to-PARTIAL is retired per-gate.

The **linux-gnu path is essentially de-risked**: `cross` 0.2.5 and Docker 29.5.3 are installed, `Cross.toml`
already has the `x86_64-unknown-linux-gnu` target stanza (it installs `libdbus-1-dev` + `pkg-config` in the
image — added in Phase 50 for exactly this), and `cross clippy` is a first-class built-in subcommand that
runs `cargo clippy` *inside* the pinned Linux container with a real `x86_64-linux-gnu-gcc`. The single
gotcha is operational, not technical: **Docker Desktop's Linux engine daemon must be running** — this
session probed it as NOT running (`npipe:////./pipe/dockerDesktopLinuxEngine` absent), which is the same
state that forced Phase 95's PARTIAL. The plan must include a "start Docker + confirm `docker info`
returns a Server section" precondition before the gate command.

The **apple-darwin path is the genuinely hard one and very likely terminates at the D-04 hard-blocker.**
cargo-zigbuild (not installed; needs `zig` + `cargo-zigbuild`) does support `cargo zigbuild clippy`, and
clippy itself only type-checks (emits `.rmeta`, never links a final binary) — so naively the macOS
framework-link requirement seems dodgeable. The wall is upstream of clippy: `aws-lc-sys 0.41.0`'s
`build.rs` performs a **compile-AND-link C feature probe** for the target, and `zig`'s `-target
x86_64-macos` mode bundles only partial libc headers under `-nostdinc`, failing on missing SDK headers
(`fatal error: '<x>.h' file not found`) or framework linking (`framework not found for '-framework
CoreFoundation'`) — both of which are only fixable by pointing `SDKROOT` at an extracted proprietary macOS
SDK. That extraction is the D-04(b) licensing wall: stop immediately, do not attempt it.

**Primary recommendation:** Plan two waves. Wave 1 = linux-gnu (start Docker → run pinned `cross clippy` →
fix all surfaced drift structurally → record image tag). Wave 2 = bounded apple-darwin attempt (install
zig+cargo-zigbuild → one `cargo zigbuild clippy` run → on the first SDK/framework/Mach-O-link failure,
write the XTGT-03(b) hard-blocker record and stop). Then rewrite the checklist (D-06/D-07 home) + the
CLAUDE.md one-line pointer (D-02), per-gate and evidence-based.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| linux-gnu clippy execution | Build tooling (`cross` + Docker container) | — | Containerized cross-compile; the lints run on the cfg(linux) branches of `crates/nono/src/sandbox/linux.rs`, `exec_strategy/supervisor_linux.rs`, `bindings/c/src/*` |
| apple-darwin clippy attempt | Build tooling (`cargo-zigbuild` + zig on host) | — | Host-driven cross-compile via zig as C compiler/linker; blocked at build-dep C link, not Rust |
| Drift fixes | Source (`crates/nono/src/sandbox/linux.rs`, `exec_strategy/`, `bindings/c/src/`) | — | Structural cfg-gate / visibility / lint fixes per D-05 |
| Verification protocol | Docs (`.planning/templates/cross-target-verify-checklist.md`) | Docs (`CLAUDE.md` pointer) | D-06 puts the runbook in the checklist; CLAUDE.md gets a one-liner |

## Standard Stack

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| `cross` | 0.2.5 (installed) `[VERIFIED: in-session probe]` | Run `cargo clippy` for `x86_64-unknown-linux-gnu` inside a pinned Docker image with a real GNU cross-linker | Canonical D-01 mechanism; ships `x86_64-linux-gnu-gcc`; reproducible via pinned image |
| Docker Desktop | 29.5.3 (installed; **daemon not running this session**) `[VERIFIED: in-session probe]` | Container runtime that backs `cross` | cross's default backend on Windows |
| `Cross.toml` | present (repo root) `[VERIFIED: read in-session]` | Pre-installs `libdbus-1-dev` + `pkg-config` into the linux-gnu image so the `keyring` crate's `sync-secret-service` (Linux-gated) links against `dbus-1.pc` | Already configured for this exact target (Phase 50 D-50-13) |
| `zig` | NOT installed — needs install | C compiler/linker backend for cargo-zigbuild apple-darwin attempt | Bundles partial cross-SDK; the D-03 first-attempt tool |
| `cargo-zigbuild` | NOT installed — needs install | Cargo subcommand wiring zig as linker; supports `cargo zigbuild clippy` `[CITED: github.com/rust-cross/cargo-zigbuild]` | D-03 chosen apple-darwin path |

### apple-darwin install steps (Wave 2 precondition)
```bash
# zig (pick a recent stable; cargo-zigbuild docs track zig >= 0.11)
# On Windows, install via: winget install zig.zig   (or scoop install zig)
# then:
cargo install --locked cargo-zigbuild
zig version            # confirm on PATH
cargo zigbuild --version
```
> `[ASSUMED]` exact zig version compatibility — verify cargo-zigbuild's current README MSRV/zig matrix at
> install time; zig's darwin-target header bundling has changed across 0.8→0.13 (multiple upstream issues
> below). Pin whatever zig version the install actually resolves and record it.

### Alternatives Considered (locked OUT by CONTEXT — do not re-explore)
| Instead of | Could Use | Why rejected per CONTEXT |
|------------|-----------|--------------------------|
| `cross` for linux-gnu | native `x86_64-linux-gnu-gcc` install, or WSL2 | D-01 chose `cross` (zero host install, reproducible) |
| cargo-zigbuild for apple-darwin | osxcross (extract macOS SDK on Linux/Windows) | D-04(b): SDK extraction is the licensing wall — forbidden |

**Installation (linux-gnu — already satisfied except daemon):**
```bash
# Already installed: cross 0.2.5, Docker 29.5.3, rustup targets x86_64-unknown-linux-gnu + x86_64-apple-darwin
# Cross.toml already present. The ONLY runtime step:
#   1. Start Docker Desktop (ensure the Linux engine, not Windows containers)
#   2. Confirm:  docker info   → must print a "Server:" section (not just Client:)
```

## Package Legitimacy Audit

> This phase installs no Rust *crate* dependencies into the workspace. The only new host tools are `zig`
> and `cargo-zigbuild` (Wave 2). Registry verification below; slopcheck targets npm/PyPI, not these host
> tools, so it does not apply — verification is via the official source repos.

| Tool | Registry | Age | Source Repo | Verdict | Disposition |
|------|----------|-----|-------------|---------|-------------|
| `cargo-zigbuild` | crates.io | mature (multi-year, v0.22.x) `[CITED: crates.io/crates/cargo-zigbuild]` | github.com/rust-cross/cargo-zigbuild | OK (official rust-cross org) | Approved for Wave 2 attempt |
| `zig` | ziglang.org / winget `zig.zig` | mature, official toolchain | github.com/ziglang/zig | OK | Approved for Wave 2 attempt |
| `cross` | already installed 0.2.5 | mature (cross-rs) | github.com/cross-rs/cross | OK | Already present |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
> No npm/PyPI packages introduced; slopcheck N/A. The two host tools are from well-known official orgs
> (rust-cross, ziglang). If the planner wants belt-and-suspenders, gate the `cargo install cargo-zigbuild`
> behind a `checkpoint:human-verify` — but this is optional given the official source.

## Architecture Patterns

### System Architecture Diagram

```
                      ┌─────────────────────────── Windows 11 dev host ───────────────────────────┐
                      │                                                                            │
  developer ──run──▶  │  cross clippy --workspace --target x86_64-unknown-linux-gnu               │
                      │        │                                                                   │
                      │        ▼                                                                   │
                      │  cross 0.2.5 ──reads──▶ Cross.toml [target.x86_64-unknown-linux-gnu]       │
                      │        │                  (pre-build: apt install libdbus-1-dev pkg-config)│
                      │        ▼                                                                   │
                      │  Docker Desktop (Linux engine)  ◀── MUST be running (npipe daemon)         │
                      │        │                                                                   │
                      │        ▼                                                                   │
                      │  ┌── ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5 ──────────────────┐  │
                      │  │  x86_64-linux-gnu-gcc present → aws-lc-sys/ring C build+link OK       │  │
                      │  │  cargo clippy runs ⟶ exercises cfg(target_os="linux") branches:       │  │
                      │  │     sandbox/linux.rs · supervisor_linux.rs · bindings/c/src/*         │  │
                      │  │  emits warnings/errors under -D warnings -D clippy::unwrap_used       │  │
                      │  └──────────────────────────────────────────────────────────────────────┘ │
                      │        │                                                                   │
                      │        ▼  exit 0 (green) ──▶ XTGT-02 satisfied; drift fixed structurally   │
                      │                                                                            │
                      │  ── apple-darwin attempt (Wave 2, bounded) ──                              │
                      │  cargo zigbuild clippy --target x86_64-apple-darwin                        │
                      │        │                                                                   │
                      │        ▼                                                                   │
                      │  zig cc (-target x86_64-macos, -nostdinc, bundled partial libc)            │
                      │        │                                                                   │
                      │        ▼                                                                   │
                      │  aws-lc-sys build.rs: compile+LINK memcmp_invalid_stripped_check.c         │
                      │        │                                                                   │
                      │        ├─ needs SDK header ⟶ "fatal error: '<x>.h' file not found"         │
                      │        └─ needs framework  ⟶ "framework not found for '-framework ...'"    │
                      │                  │                                                         │
                      │                  ▼  ⟶ SDKROOT=<proprietary macOS SDK> required = D-04(b) STOP│
                      │                  ▼  ⟶ write XTGT-03(b) hard-blocker record → PARTIAL→CI     │
                      └────────────────────────────────────────────────────────────────────────────┘
```

### Recommended Plan/Wave Structure
```
Wave 1 — linux-gnu gate (XTGT-01 + XTGT-02)
  ├── precondition: start Docker Linux engine; confirm `docker info` Server section
  ├── run pinned `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
  ├── fix ALL surfaced drift structurally (D-05) — re-run to green
  └── record exact image tag (ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5) + green exit

Wave 2 — apple-darwin bounded attempt (XTGT-03)
  ├── install zig + cargo-zigbuild (record versions)
  ├── ONE `cargo zigbuild clippy --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`
  └── on first SDK/framework/Mach-O wall → write XTGT-03(b) hard-blocker record, STOP (PARTIAL→CI)
        OR (unlikely) clean exit → apple-darwin flips to local-runnable

Wave 3 (or tail of W1/W2) — protocol rewrite (XTGT-04)
  ├── rewrite .planning/templates/cross-target-verify-checklist.md (D-06 home; D-07 per-gate tree)
  └── replace CLAUDE.md bullet with cross-form + one-line pointer (D-02)
```

### Pattern 1: Pinned cross image tag (reproducibility)
**What:** cross's default image for a target is `ghcr.io/cross-rs/<target>:<cross-version>` — i.e. for
cross 0.2.5 the linux-gnu image is `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5`. `[ASSUMED]` exact
tag — confirm at run time from `cross`'s pull log (it prints the image it pulls). The Cross.toml stanza
does NOT override `image`, so the default versioned tag is used; that default IS already pinned to the
cross binary version, which gives reproducibility without extra config.
**When to use:** Record the actual pulled tag in the checklist's setup section so another dev reproduces
the same image (SC#1 "sufficient for another developer to reproduce").
**Note:** If the planner wants hard-pinning independent of the cross binary, add `image = "ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5"` to the existing Cross.toml stanza — optional, discretionary.

### Pattern 2: Docker daemon precondition gate
**What:** Before the gate command, assert the Linux engine is up.
```bash
docker info 2>&1 | grep -q "Server Version" || { echo "Docker Linux engine NOT running — start Docker Desktop"; exit 1; }
```
**Why:** This session's probe showed `docker ps` failing with
`failed to connect to the docker API at npipe:////./pipe/dockerDesktopLinuxEngine` — the exact state that
forced Phase 95 PARTIAL. Without this gate the executor may misread a daemon-down failure as a "documented
Docker failure" and wrongly defer to PARTIAL, defeating the phase.

### Pattern 3: cargo-zigbuild clippy invocation (apple-darwin)
**What:** cargo-zigbuild exposes Cargo subcommands including `clippy`. `[CITED: github.com/rust-cross/cargo-zigbuild]`
```bash
cargo zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```
**When to use:** Exactly once (D-03 one-attempt cap). zig provides `cc`/`clang` for the build-dep C
compiles that Phase 95 found absent (`failed to find tool "cc"`). If it reaches the aws-lc-sys link probe
and fails there, that is the D-04 wall — do not iterate.

### Anti-Patterns to Avoid
- **Silencing drift with `#[allow(...)]`** — D-05 / checklist Anti-pattern 2 / CLAUDE.md Unwrap Policy.
  Fix with cfg-gates, visibility (`pub(crate)`), `_`-prefix for genuinely-unused-on-this-target, or
  structural change. Note CLAUDE.md also bans `#[allow(dead_code)]`.
- **Reading a daemon-down Docker as a "documented Docker failure"** that justifies PARTIAL. The D-07
  carve-out allows PARTIAL only on a *documented cross/Docker failure* — a stopped daemon is an operator
  precondition, not a cross/Docker capability gap. Start the daemon and re-run.
- **Attempting macOS SDK extraction** to get apple-darwin green — D-04(b) hard stop, licensing.
- **Re-running the Phase 95 absorb** — out of scope; this phase only clears the *existing* drift surface.
- **`cargo check`-as-clippy** or host-target `cargo clippy --workspace` (no `--target`) — checklist
  Anti-patterns 3 & 4; neither exercises Unix cfg branches.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| linux-gnu cross-linker on Windows | manual MinGW/gnu-gcc install + env wiring | `cross` 0.2.5 (already installed) + existing Cross.toml | cross ships the matched `x86_64-linux-gnu-gcc` + libdbus image; reproducible; D-01 |
| macOS C toolchain on Windows | osxcross / hand-assembled SDK | `cargo-zigbuild` (zig bundles much of it) | D-03; and the moment it needs the SDK = D-04 stop, not a hand-roll |
| Image reproducibility | custom Dockerfile | cross's versioned default tag (or `image=` pin in Cross.toml) | already version-pinned to the cross binary |

**Key insight:** Both cross-target mechanisms already exist as mature tools; this phase is about *running*
them correctly and *recording the protocol*, not building any tooling. The only genuinely novel artifact
is the XTGT-03(b) hard-blocker *record* (prose), if apple-darwin walls out.

## Runtime State Inventory

> This is a toolchain+protocol+lint-fix phase, not a rename/migration. No stored data, live-service config,
> OS-registered state, secrets, or build artifacts carry a renamed string. Inventory categories all resolve
> to "None":

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no datastore keys touched | none |
| Live service config | None — no external service config | none |
| OS-registered state | None — no task/service registrations | none |
| Secrets/env vars | `SDKROOT` is the ONLY env var of interest, and setting it would BE the D-04(b) violation (it would point at an extracted proprietary SDK) — so it must remain UNSET | none (deliberately do not set SDKROOT) |
| Build artifacts | `target/x86_64-unknown-linux-gnu/` + `target/x86_64-apple-darwin/` dirs will be created by the gates; these are throwaway clippy outputs, not shipped artifacts | none (gitignored `target/`) |

**Nothing found in any category requiring migration** — verified by phase scope (no source string rename;
only cfg-gated lint fixes + doc edits).

## Common Pitfalls

### Pitfall 1: Docker Linux engine not running (the Phase 95 trap, re-armed)
**What goes wrong:** `cross clippy` fails to connect to the Docker API; executor mis-files it as a Docker
capability failure and defers to PARTIAL — exactly what Phase 95 did at HEAD `be42a5af`.
**Why it happens:** Docker Desktop's Linux engine daemon is not started (this session probed it DOWN:
`npipe:////./pipe/dockerDesktopLinuxEngine` absent). `docker --version` and even partial `docker info`
(Client section) succeed even when the daemon is down — misleading.
**How to avoid:** Gate on `docker info` printing a **Server** section (Pattern 2) before the clippy run.
**Warning signs:** `failed to connect to the docker API at npipe:////./pipe/dockerDesktopLinuxEngine` /
`open //./pipe/dockerDesktopLinuxEngine: The system cannot find the file specified`.

### Pitfall 2: Expecting `cross clippy` to be unsupported
**What goes wrong:** Assuming cross only does `build`/`test` and falling back to `cross build`.
**Why it happens:** Sparse docs. **Fact:** `clippy` is a first-class built-in cross subcommand (alongside
build/check/clippy/run/test/bench). `[VERIFIED: cross docs + WebSearch cross-rs]` It runs `cargo clippy`
inside the container, so `-D warnings -D clippy::unwrap_used` after `--` behaves identically to native.
**How to avoid:** Use `cross clippy …` directly (D-01 verbatim command).

### Pitfall 3: cfg-gated lint drift classes on first local linux-gnu run (D-05 targets)
**What goes wrong:** The cfg(linux) branches have never been clippy-linted locally on this host; upstream
absorb (Phase 95) added `linux.rs`/`supervisor_linux.rs`/tool-sandbox `linux.rs` hunks. First run will
likely surface a cluster of lints invisible to Windows-host clippy. `[ASSUMED]` exact lints — these are
the *probable* classes to expect and fix structurally:
- `dead_code` / unused imports on items only referenced in a `cfg(target_os="macos")` sibling branch (fix:
  tighten cfg-gates or `pub(crate)` visibility, NOT `#[allow(dead_code)]`).
- `clippy::unwrap_used` / `clippy::expect_used` in newly-absorbed Linux code paths (fix: `?` propagation
  via `NonoError` per CLAUDE.md Error Handling).
- `clippy::needless_return`, `clippy::redundant_clone`, `clippy::useless_conversion` on absorbed hunks.
- `unused_variables` / `unused_mut` behind `cfg(target_os="linux")`.
- `clippy::missing_safety_doc` on `bindings/c/src/*` FFI consumed by the Unix runtimes (fix: add `// SAFETY:`
  per CLAUDE.md Unsafe Code rule).
- `offset_of!`-derivation / static-byte-string warnings noted in Phase 95-07 summary for `linux.rs` /
  `exec_strategy.rs`.
**How to avoid:** Fix each structurally; re-run `cross clippy` to confirm exit 0. `make ci` (clippy+fmt+
tests) must still pass on the Windows host after the fixes (don't break native-target clippy).
**Warning signs:** Any `#[allow(...)]` appearing in a fix diff = red flag.

### Pitfall 4: rustfmt-clean ≠ clippy-clean (CI regression vector)
**What goes wrong:** A structural drift fix passes `cross clippy` but breaks `cargo fmt --check`, which CI
catches separately (memory `feedback_fmt_check_in_verify_gate`: Rustfmt CI failed twice on 2026-06-25).
**How to avoid:** Run `make fmt-check` (or `make ci`) after every drift fix, not just clippy.
**Warning signs:** Conventional-commit title check also rejects uppercase-initial subjects and
non-feat/fix types — keep commit subjects lowercase, `fix(...)`/`chore(...)`.

### Pitfall 5: zig's `-nostdinc` + partial darwin headers (the apple-darwin reality)
**What goes wrong:** `cargo zigbuild clippy` for apple-darwin fails not in Rust but in a build-dep C
compile/link. zig uses `-nostdinc` and bundles only partial libc for `-target x86_64-macos`, so
aws-lc-sys / ring C probes can fail on a missing SDK header or framework. `[CITED: github.com/rust-cross/cargo-zigbuild#28, ziglang/zig#1349, #10513, #24024]`
**Why it happens:** `aws-lc-sys 0.41.0` build.rs compiles **and links** a feature-detection C program
(`memcmp_invalid_stripped_check.c`); the Mach-O link / framework search needs the real macOS SDK
(`SDKROOT`). `[CITED: github.com/aws/aws-lc-rs#933 — Mach-O vs ELF linker mismatch, SDK required]`
**How to avoid (per D-04):** Do NOT try to fix it by setting `SDKROOT` — that requires extracting the
proprietary SDK = D-04(b) hard stop. Capture the failure and write the hard-blocker record.
**Stop signatures to recognize (any one = D-04 wall):**
- `fatal error: '<header>.h' file not found` during a darwin C build-dep compile
- `warning(link): framework not found for '-framework CoreFoundation'` (or any `-framework`)
- `error: unable to find Darwin SDK` / prompts to set `SDKROOT`
- linker-emulation mismatch (`无法辨认的仿真模式: llvm` / "linker command failed") from aws-lc-sys probe

## Code Examples

### linux-gnu gate (canonical, D-01 verbatim)
```bash
# precondition
docker info 2>&1 | grep -q "Server Version" || { echo "start Docker Linux engine"; exit 1; }
# gate
cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
# expect exit 0 after drift fixes; record the pulled image tag from cross's log
```

### apple-darwin bounded attempt (D-03 one-shot)
```bash
# install (record versions)
winget install zig.zig        # or scoop install zig
cargo install --locked cargo-zigbuild
# single attempt — do NOT set SDKROOT
cargo zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
# on first SDK/framework/Mach-O wall → write XTGT-03(b) record, STOP
```

### Drift fix pattern (structural, no allow)
```rust
// BAD (forbidden by D-05 / Anti-pattern 2)
#[allow(dead_code)]
fn linux_only_helper() { /* ... */ }

// GOOD — tighten the cfg so the item only exists where it's used
#[cfg(target_os = "linux")]
fn linux_only_helper() { /* ... */ }

// GOOD — propagate instead of unwrap (CLAUDE.md Error Handling)
let v = maybe.ok_or(NonoError::PathNotFound(p.clone()))?;
```

### Doc/Protocol Rewrite (XTGT-04, D-06/D-07) — checklist Decision Tree, target shape
```
Q2 (linux-gnu): MUST run locally via:
    cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
  - clean exit → REQ may flip to VERIFIED at codebase level.
  - errors → close before flipping.
  - PARTIAL is allowed ONLY on a DOCUMENTED cross/Docker failure (image pull failure, daemon
    capability gap) — NOT a stopped daemon (start it) and NOT "toolchain absent" (it is present).
Q3 (apple-darwin): disposition = <result of Phase 96 Wave 2>:
  - if Wave 2 walled out → stays explicitly PARTIAL→CI with the recorded hard-blocker rationale;
    the bare `cargo clippy --target x86_64-apple-darwin` remains documented as not-locally-runnable.
  - if Wave 2 passed → flips to "MUST run locally via cargo zigbuild clippy …".
```
And the CLAUDE.md bullet (D-02) collapses to: the `cross` linux-gnu command + apple-darwin disposition +
a one-line pointer to `.planning/templates/cross-target-verify-checklist.md` (no duplicated runbook).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Bare `cargo clippy --target x86_64-unknown-linux-gnu` documented as the "runnable" command | `cross clippy …` (containerized, ships the linker) | This phase (D-02) | The bare command was never runnable on this host (no native linker) — replacing it removes a doc lie |
| Auto-default-to-PARTIAL→CI for BOTH cross-targets | Per-gate evidence-based retirement (linux-gnu local-required; apple-darwin per Wave-2 outcome) | This phase (D-07) | Verifier may now flip Unix-touching REQs to VERIFIED locally for linux-gnu |

**Deprecated/outdated:**
- The checklist's § "Cross-Toolchain Setup" `rustup target add …` block as the *primary* setup — those
  targets are already added; the real setup is "cross + Docker running" (linux-gnu) and "zig +
  cargo-zigbuild" (apple-darwin attempt). Rewrite this section accordingly (D-06).
- The checklist Q2 "No (toolchain missing — linker not found)" auto-PARTIAL branch for linux-gnu — retired
  by D-07 (toolchain is present via cross).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Default cross image tag is `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5` | Pattern 1 | Low — executor records the *actual* pulled tag from cross's log; doc just needs the real value |
| A2 | First linux-gnu run surfaces the specific lint classes listed (dead_code, unwrap_used, FFI safety-doc, etc.) | Pitfall 3 | Low — these are *probable*; the gate output is authoritative. D-05 says fix whatever appears |
| A3 | apple-darwin walls out at aws-lc-sys SDK/framework link (very likely) | apple-darwin / Pitfall 5 | Medium — if zig's bundled SDK happens to satisfy the C probes, apple-darwin could pass; D-03/D-04 already cover both outcomes, so either way the plan holds |
| A4 | Exact zig version for cargo-zigbuild compatibility | Standard Stack | Low — verify README matrix at install; record resolved version |
| A5 | `cargo zigbuild clippy` subcommand works as build-like cargo subcommand | Pattern 3 | Low — confirmed cargo-zigbuild supports clippy; if not, the apple-darwin attempt fails fast → same D-04 record path |

**These five `[ASSUMED]` items do NOT block planning** — the locked decisions (D-03/D-04 in particular)
make the apple-darwin outcome resilient to A3/A5, and A1/A2 resolve to whatever the live gate emits.

## Open Questions

1. **Will the first linux-gnu `cross clippy` run produce a LARGE drift backlog or a small one?**
   - What we know: Phase 95 absorbed `9ce74e92`, `11fd10e0`, and tool-sandbox `linux.rs` hunks; the
     prior PARTIAL was a *C-linker* failure, not a Rust clippy failure — Phase 95's static grep probes
     suggested the changed Rust was clean.
   - What's unclear: whether the *full workspace* linux-gnu clippy (first ever local run) surfaces
     latent upstream-inherited lints beyond the changed files.
   - Recommendation: Plan Wave 1 with an explicit "triage + fix loop" task, not a single fix task; size
     for the possibility of a multi-file structural fix pass.

2. **Does the planner want to wire linux-gnu into a `make` target?**
   - What we know: CONTEXT marks this planner discretion (deferred-ideas list).
   - Recommendation: Optional `make clippy-linux` target = nice DX, low cost; but keep it additive to
     `make ci` and not a locked requirement.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cross` | linux-gnu gate (D-01) | ✓ | 0.2.5 | — |
| Docker Desktop (binary) | cross backend | ✓ | 29.5.3 | — |
| Docker **Linux engine daemon (running)** | cross backend at runtime | ✗ (DOWN this session) | — | **Start Docker Desktop** (blocking precondition, not a true fallback) |
| `Cross.toml` linux-gnu stanza | image libdbus pre-build | ✓ | present | — |
| rustup `x86_64-unknown-linux-gnu` std | linux-gnu | ✓ | installed | — |
| rustup `x86_64-apple-darwin` std | apple-darwin | ✓ | installed | — |
| `zig` | apple-darwin attempt (D-03) | ✗ | — | install via winget/scoop |
| `cargo-zigbuild` | apple-darwin attempt (D-03) | ✗ | — | `cargo install --locked cargo-zigbuild` |
| macOS SDK (`SDKROOT`) | apple-darwin C link | ✗ (and MUST stay absent) | — | **none — by design**; needing it = D-04(b) STOP |

**Missing dependencies with no fallback:**
- Docker Linux engine daemon must be *running* — there is no fallback; it is a required precondition the
  executor starts manually before the gate. (Not a true blocker since the binary is installed.)

**Missing dependencies with fallback:**
- `zig` / `cargo-zigbuild` — install in Wave 2 (records versions); a one-time host install.
- macOS SDK — intentionally has NO fallback; its absence is the expected D-04 outcome, not a problem to fix.

## Validation Architecture

> `.planning/config.json` — `workflow.nyquist_validation` not confirmed in-session; including per default
> (absent = enabled). This phase's "tests" are the gate commands themselves plus the existing suite
> regression check.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner + clippy + rustfmt (via `make ci`) |
| Config file | `Makefile` targets; `Cross.toml` for cross |
| Quick run command | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` |
| Full suite command | `make ci` (native-target clippy + fmt + tests) — must stay green after drift fixes |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| XTGT-01 | toolchain installed + documented | smoke | `cross --version && docker info \| grep "Server Version"` | ✓ (host) |
| XTGT-02 | linux-gnu clippy exits 0 | gate | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | ✓ |
| XTGT-03 | apple-darwin passes OR hard-blocker recorded | gate + doc | `cargo zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (one shot) | ✓ |
| XTGT-04 | checklist + CLAUDE.md updated | doc-review | manual diff review of the two files | ✓ |

### Sampling Rate
- **Per task commit:** `make ci` (don't break native-target clippy/fmt/tests with a drift fix)
- **Per wave merge:** `cross clippy …` green (Wave 1); apple-darwin verdict recorded (Wave 2)
- **Phase gate:** Wave 1 green + Wave 2 resolved + both docs rewritten before `/gsd:verify-work`

### Wave 0 Gaps
- None — no new test files needed. The gates ARE the validation; the existing Rust suite + `make ci`
  guard against regressions. (One caveat: the pre-existing baseline test fails noted in memory
  `nono_cli_windows_baseline_test_failures` and Phase 95-07 summary — 11 nono-cli + 1 nono — are NOT
  regressions from this phase; don't chase them.)

## Security Domain

> `security_enforcement` not explicitly disabled → included. This phase changes NO security-relevant code
> behavior; it adds lint coverage to existing cfg-gated security code and edits docs. The security value
> is *defensive*: making the linux-gnu Landlock/seccomp/path-handling code (`sandbox/linux.rs`,
> `supervisor_linux.rs`) provably clippy-clean locally closes the exact Windows-host blind-spot that let
> WR-02/WR-03 regressions slip in prior phases (per Phase 95-07 plan note).

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | indirect | the linted code IS the capability-enforcement surface (Landlock); lint coverage hardens it but adds no new control |
| V5 Input Validation | no (no new inputs) | — |
| V6 Cryptography | no (aws-lc-sys/ring are deps; not modified) | — |
| V14 Configuration / Build | yes | reproducible pinned cross image; no SDK extraction (supply-chain/licensing discipline) |

### Known Threat Patterns for {Rust cross-target tooling}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Silencing a real Unix bug with `#[allow(...)]` to pass the gate | Tampering (hides defect) | D-05 forbids it; structural fixes only |
| Misreading daemon-down as "toolchain absent" → false PARTIAL → un-linted Unix security code ships | Repudiation / Elevation (regression slips) | Pattern 2 daemon precondition gate; D-07 narrows the PARTIAL escape hatch |
| Extracting proprietary macOS SDK on the host to force apple-darwin green | (licensing/supply-chain) | D-04(b) hard stop; SDKROOT must stay unset |

## Sources

### Primary (HIGH confidence)
- In-session host probes — `cross --version` (0.2.5), `docker --version` (29.5.3) + `docker ps`
  (daemon DOWN: `npipe:////./pipe/dockerDesktopLinuxEngine` absent), `rustup target list --installed`
  (both linux-gnu + apple-darwin present), `zig`/`cargo zigbuild` absent.
- Read in-session: `Cross.toml` (x86_64-unknown-linux-gnu stanza w/ libdbus pre-build), `Cargo.lock`
  (aws-lc-sys 0.41.0, ring 0.17.14, cc 1.2.61, dbus/libdbus-sys present), CLAUDE.md cross-target bullet,
  `.planning/templates/cross-target-verify-checklist.md`, `.planning/phases/95-*/95-07-SUMMARY.md`
  (HEAD `be42a5af`, both targets PARTIAL→CI, Docker engine pipe absent), 95-CONTEXT.md, REQUIREMENTS.md
  (XTGT-01..04), ROADMAP §Phase 96 SC 1–4, 94-DIVERGENCE-LEDGER (cfg-gated annotations).

### Secondary (MEDIUM confidence)
- cross 0.2.5 supports `clippy` as a built-in subcommand (build/check/clippy/run/test/bench) —
  WebSearch (cross-rs) verified against cross docs.rs crate page.
- cargo-zigbuild supports a `clippy` subcommand — github.com/rust-cross/cargo-zigbuild + docs.rs.
- clippy emits `dep-info,metadata` only (no final link) — rust-clippy issue #3663 + Cargo docs.

### Tertiary (LOW confidence — flagged for run-time confirmation)
- Exact default cross image tag string `:0.2.5` — confirm from cross's pull log at run time (A1).
- Exact first-run lint backlog — authoritative only from the live gate (A2).
- aws-lc-sys darwin failure exact line — aws/aws-lc-rs#933 + cargo-zigbuild#28 + ziglang/zig#1349/#10513/#24024
  give the *class* of failure (SDK headers / `-framework` not found / Mach-O linker mismatch); the precise
  line depends on which build-dep fires first (A3).

## Metadata

**Confidence breakdown:**
- linux-gnu mechanism (D-01/D-02): HIGH — tools + Cross.toml verified present; `cross clippy` confirmed;
  only the daemon-start precondition is operational.
- apple-darwin mechanism (D-03/D-04): MEDIUM — the SDK wall is well-evidenced and the D-04 stop is
  explicit; precise failure line resolves at run time but does not change the disposition logic.
- Drift classes (D-05): MEDIUM-LOW — probable classes listed; live gate is authoritative.
- Protocol rewrite (D-06/D-07): HIGH — target shape derived directly from the locked decisions + the
  current checklist text.

**Research date:** 2026-06-26
**Valid until:** ~2026-07-26 (stable tooling; re-confirm cargo-zigbuild/zig version matrix at install time)
