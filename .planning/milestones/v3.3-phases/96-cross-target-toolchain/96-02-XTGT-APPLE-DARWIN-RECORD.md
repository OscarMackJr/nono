# 96-02 — apple-darwin cross clippy gate: RECORD

**Phase:** 96-cross-target-toolchain · **Plan:** 02 · **Date:** 2026-06-26
**Requirement:** XTGT-03 (apple-darwin clippy passes locally OR documented hard-blocker → PARTIAL→CI)
**Result:** ✅ GREEN — the ONE bounded `cargo-zigbuild clippy` attempt for `x86_64-apple-darwin` exited 0 on this Windows dev host, with `SDKROOT` UNSET and no macOS SDK extraction. apple-darwin flips to **LOCAL-RUNNABLE** (the D-04 clean-exit branch, not the hard-blocker branch).

Cross-reference sibling: `96-01-XTGT-LINUX-GNU-RECORD.md` (linux-gnu gate GREEN via `cross clippy`). Both targets are now provably runnable locally on this host.

---

## 1. Wave 2 tool installs (XTGT-01 / D-03 precondition) — resolved versions recorded

Both host tools are from official orgs (ziglang, rust-cross) per the Research Package Legitimacy
Audit (OK verdict). No npm/PyPI packages introduced (slopcheck N/A).

| Tool | Install command | Resolved version (verbatim) |
|------|-----------------|------------------------------|
| `zig` | `winget install --id zig.zig` | `0.16.0` (`zig version` → `0.16.0`) |
| `cargo-zigbuild` | `cargo install --locked cargo-zigbuild` | `0.23.0` (`cargo-zigbuild --version` → `cargo-zigbuild 0.23.0`) |

Verbatim version probes:

```
$ zig version
0.16.0

$ cargo-zigbuild --version
cargo-zigbuild 0.23.0
```

**PATH note (reproducibility):** winget installs the `zig` shim under
`%LOCALAPPDATA%\Microsoft\WinGet\Links` (`/c/Users/OMack/AppData/Local/Microsoft/WinGet/Links`);
`cargo-zigbuild.exe` lands in `%USERPROFILE%\.cargo\bin`. Both must be on `PATH` for the gate. A
fresh shell picks up the winget Links dir automatically; this session prepended it explicitly.

## 2. The ONE bounded attempt (D-03 one-shot cap) — exact invocation

Exactly **one** `cargo-zigbuild clippy` invocation was made. `SDKROOT` was confirmed UNSET before
the run and no macOS SDK extraction was attempted (D-04(b) licensing line respected).

```bash
$ echo "SDKROOT is: [${SDKROOT:-<unset>}]"
SDKROOT is: [<unset>]

$ cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

**Invocation-form note (consumed by Plan 03):** the *working* command is the **direct binary**
`cargo-zigbuild clippy …`, NOT `cargo zigbuild clippy …`. Under cargo's external-subcommand
mechanism, `cargo zigbuild clippy` invokes `cargo-zigbuild` with `zigbuild` as the first argument,
which collides with the binary's own `zigbuild` subcommand and mis-parses `clippy` as a stray arg.
cargo-zigbuild 0.23.0 exposes `clippy` as a first-class subcommand on the binary itself
(`cargo-zigbuild <COMMAND>` where `COMMAND ∈ {check, clippy, doc, run, rustc, test, zigbuild, zig}`),
so the canonical local invocation is the `cargo-zigbuild clippy …` form recorded above. That `-D
clippy::unwrap_used` was accepted (not rejected as an unknown lint) and the run emitted `Checking`
lines confirms genuine clippy execution, not a plain `cargo check`.

## 3. Why this did NOT wall out at the expected aws-lc-sys SDK probe (the A3 outcome)

The Research (§ Pitfall 5, assumption A3) flagged the very likely D-04(b) wall: `aws-lc-sys 0.41.0`'s
`build.rs` runs a compile-AND-link C feature probe (`memcmp_invalid_stripped_check.c`) that, under
zig's `-nostdinc` + partial darwin headers, typically fails on a missing SDK header
(`fatal error: '<x>.h' file not found`) or framework (`framework not found for '-framework ...'`).

On this host that wall did **not** materialize. With zig 0.16.0 + cargo-zigbuild 0.23.0, zig's
bundled macOS C target support (`-target x86_64-macos`) satisfied the `aws-lc-sys 0.41.0` /
`ring 0.17.14` build-dep C probes well enough that both `aws-lc-sys` and the downstream
`aws-lc-rs 1.17.0` compiled, and the macOS-specific dep graph (`core-foundation`,
`core-foundation-sys`, `security-framework`, `security-framework-sys`) checked clean — i.e. the
`#[cfg(target_os = "macos")]` branches (`crates/nono/src/sandbox/macos.rs`, the Unix
`exec_strategy/` paths, `bindings/c/src/*`) were genuinely exercised. This is assumption A3
resolving to the favorable side; D-03/D-04 covered both outcomes, so the plan holds either way.

## 4. Captured outcome — clean exit (SC#3 branch (a))

The C-linking build deps that drive the probe (authoritative, from the run log):
`aws-lc-sys v0.41.0`, `aws-lc-rs v1.17.0`, `ring v0.17.14`.

Tail of the single run (verbatim):

```
   Compiling nono-cli v0.62.2 (C:\Users\OMack\Nono\crates\nono-cli)
    Checking rustls-webpki v0.103.13
    ...
    Checking nono-proxy v0.62.2 (C:\Users\OMack\Nono\crates\nono-proxy)
    Checking nono-shell-broker v0.62.2 (C:\Users\OMack\Nono\crates\nono-shell-broker)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4m 37s
=== EXIT CODE: 0 ===
```

All workspace crates (`nono`, `nono-ffi`, `nono-cli`, `nono-proxy`, `nono-shell-broker`,
`nono-fltmgr-client`, plus `sign-fixture`) checked clean for `x86_64-apple-darwin`.

**Exit 0. Zero errors, zero warnings under `-D warnings -D clippy::unwrap_used`.**

None of the D-04 stop signatures (`'<x>.h' file not found`, `framework not found for '-framework ...'`,
`unable to find Darwin SDK`, SDKROOT-required, aws-lc-sys link/emulation mismatch) appeared — so no
hard-blocker branch was entered and `SDKROOT` was never set.

## 5. Compliance attestations

- **D-03 one-shot:** exactly ONE `cargo-zigbuild clippy --target x86_64-apple-darwin` invocation (above). No iteration.
- **D-04(b) licensing line:** `SDKROOT` confirmed `<unset>` before the run; no macOS SDK acquired/extracted. The clean exit was reached without crossing the licensing boundary.
- **D-05 no-silencing:** no source changes were needed (clean on first attempt), so no `#[allow(...)]` was added.

---

## 6. Disposition (XTGT-03 resolution) — apple-darwin is LOCAL-RUNNABLE

XTGT-03 resolves to **exactly one** explicit end-state: the **clean-exit branch** of D-04
(the bounded attempt yielded a clean `clippy` run), NOT the XTGT-03(b) hard-blocker branch.

**apple-darwin = LOCAL-RUNNABLE.** Reproducible setup:

| Element | Value |
|---------|-------|
| Working invocation | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` |
| Tooling | `zig 0.16.0` + `cargo-zigbuild 0.23.0` (host installs; versions in § 1) |
| Env requirement | `SDKROOT` MUST stay UNSET (it was unset for the green run; setting it would be the D-04(b) violation and is unnecessary) |
| Outcome | exit 0, zero warnings/errors under `-D warnings -D clippy::unwrap_used` |

The XTGT-03(b) hard-blocker record is **not** written, because no SDK/framework/Mach-O failure
signature was captured — the gate passed without crossing the proprietary-SDK licensing boundary.

### Plan 03 handoff flag (D-07 per-gate retirement)

> **apple-darwin Q3 branch FLIPS to "MUST run locally."** Rewrite the
> `.planning/templates/cross-target-verify-checklist.md` decision tree so the apple-darwin gate is
> **MUST run locally via `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`** (with `zig`+`cargo-zigbuild` setup and `SDKROOT` UNSET), and the
> auto-default-to-PARTIAL→CI is **retired** for apple-darwin (PARTIAL allowed only on a *documented*
> zig/cargo-zigbuild failure, NOT a stopped-tool or absent-toolchain state). Both cross-targets
> (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`) are now provably
> local-runnable on this host — the verifier may flip Unix-touching REQs to VERIFIED locally for
> BOTH gates. Use the `cargo-zigbuild clippy` form (direct binary), not `cargo zigbuild clippy`
> (see § 2 invocation-form note).

---

*Consumed by Plan 96-03 (checklist / CLAUDE.md rewrite) and by `/gsd:verify-work`.*
