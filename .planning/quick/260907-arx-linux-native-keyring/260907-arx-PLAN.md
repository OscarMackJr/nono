---
quick_id: 260907-arx
slug: linux-native-keyring
date: 2026-09-07
description: Switch the Linux keyring backend to linux-native so binaries stop linking libdbus
milestone: v3.7
phase_context: CI run 34068950597 triage — item 3 of 7
operator_decision: "linux-native" (chosen 2026-09-07, in the operator's own words)
supersedes: D-50-13 / Codex R-50-04 (Cross.toml libdbus provisioning)
---

# Quick Task 260907-arx

## The defect: two committed decisions in direct contradiction

| Artifact | Says |
|---|---|
| `Cross.toml` (D-50-13, Codex R-50-04) | installs `libdbus-1-dev` **so `keyring`'s `sync-secret-service` can link** — libdbus is *expected* |
| `.github/workflows/ci.yml` cross-compile job | **fails the build if the binary links libdbus** — "the release binary is not portable" |

Both were committed. The two Linux cross-compile jobs failed at opposite ends of
that contradiction:

- **`x86_64-unknown-linux-gnu`** (bare `ubuntu-22.04`, installs only
  `pkg-config`): never got as far as the guard — `libdbus-sys`'s build script
  failed, `No package 'dbus-1' found`.
- **`aarch64-unknown-linux-gnu`** (`cross`, which *does* install libdbus per
  `Cross.toml`): built fine, then tripped the guard —
  `target/aarch64-unknown-linux-gnu/release/nono links libdbus — the release
  binary is not portable`.

Same root cause, two symptoms, which is why they read as unrelated failures.

## It reached production, not just CI

`system-keyring` is a **default** feature; nothing in the repo passes
`--no-default-features`; and `release.yml:82` builds Linux release artifacts with
the same command shape that made the aarch64 job link libdbus. So distributed
Linux binaries have been non-portable by the guard's own standard — they require
`libdbus-1` at runtime and fail on minimal containers. For a sandboxing tool
frequently run in CI and containers, that is the wrong default.

## Decision

Operator chose **`linux-native`** (kernel keyutils via `linux-keyutils`),
declining `async-secret-service` (pure-Rust zbus, which would have preserved
Secret Service semantics but carried an unverified sync-API-inside-tokio risk).

**Accepted trade-off, stated plainly:** the Linux kernel keyring is
session/user-scoped and **volatile — stored credentials do not survive a
reboot**, unlike Secret Service (GNOME Keyring / KWallet). This is a real
behavioral change for Linux users of `nono`'s system-keystore path. The
`env://` and file-based credential paths in `keystore.rs` are unaffected.

It also brings Linux in line with the `*-native` pattern already used on the
other two platforms (`apple-native`, `windows-native`).

## Changes

1. `crates/nono/Cargo.toml` and `crates/nono-cli/Cargo.toml` — Linux target dep
   `features = ["sync-secret-service"]` → `["linux-native"]`, each carrying a
   comment naming the reason and the lockstep requirement (two different
   backends in one process would be a silent split-brain store).
2. Three `[features]` comments in `nono`, `nono-cli`, `nono-proxy` that all read
   "macOS Keychain / **Linux Secret Service**" — now false. Corrected, and
   `nono-cli`'s stale "disable this for headless/container environments without
   libdbus" advice removed, since nothing links libdbus any more.
3. `Cross.toml` — every line in it existed solely to provision libdbus. Rewritten
   as a documented tombstone rather than deleted, so a future reader can tell
   "we never needed one" from "someone removed it by mistake."
4. `Cargo.lock` regenerated.

## Verification

1. **`libdbus-sys` absent from the Linux graph** —
   `cargo tree --target x86_64-unknown-linux-gnu -p nono-sandbox-cli -i libdbus-sys`
   must report *no such package*.
2. **No dbus/secret-service crate anywhere** in that graph.
3. **`linux-keyutils` reached via `keyring`**, through `nono-sandbox` →
   `nono-sandbox-cli`/`-proxy`.
4. **Lockfile delta is exactly the intended swap:** removes `dbus`,
   `dbus-secret-service`, `libdbus-sys`, `pkg-config`; adds `linux-keyutils`.
5. **The CI assertion itself, reproduced locally:** `cross build --release
   --target x86_64-unknown-linux-gnu -p nono-sandbox-cli`, then
   `objdump -p <bin> | awk '/NEEDED/'` — must list **no `libdbus`**. This is the
   exact check `ci.yml` runs, so passing it locally is direct evidence rather
   than an inference.
6. Cross-target clippy gate for linux-gnu (the dependency change is Linux-only;
   the apple-darwin graph is untouched, but `Cargo.lock` is shared, so the darwin
   gate is re-run as well).
7. Code is backend-agnostic — all call sites use `keyring::Entry` and
   `keyring::Error::{NoEntry, Ambiguous}`, both present regardless of backend.
   Verified by grep before changing anything.

## Follow-up, not done here

`ci.yml`'s cross-compile job still runs `apt-get install -y pkg-config`. It is
now unnecessary (the `pkg-config` *crate* left the graph) but harmless, and
removing it widens the blast radius of a dependency change for no gate value.
Recorded rather than silently trimmed.
