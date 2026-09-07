---
quick_id: 260907-arx
slug: linux-native-keyring
date: 2026-09-07
status: complete
milestone: v3.7
supersedes: D-50-13 / Codex R-50-04
unblocks_jobs:
  - Cross-compile check x86_64-unknown-linux-gnu
  - Cross-compile check aarch64-unknown-linux-gnu
---

# Quick Task 260907-arx — Summary

**Status:** complete. Item 3 of 7 from the CI run `34068950597` triage.

## What was wrong

Two committed decisions contradicted each other:

| Artifact | Says |
|---|---|
| `Cross.toml` (D-50-13, Codex R-50-04) | installs `libdbus-1-dev` **so `keyring`'s `sync-secret-service` can link** |
| `ci.yml` cross-compile job | **fails the build if the binary links libdbus** — "not portable" |

The two Linux jobs failed at opposite ends of it: x86_64 (bare runner, no libdbus)
never reached the guard — `libdbus-sys`'s build script failed with
`No package 'dbus-1' found`; aarch64 (via `cross`, which *does* install it) built
fine and then tripped the guard.

**It reached production.** `system-keyring` is a default feature, nothing in the
repo passes `--no-default-features`, and `release.yml:82` builds Linux release
artifacts the same way. Distributed Linux binaries required `libdbus-1` at
runtime — for a sandboxing tool routinely run in containers, the wrong default.

## Decision

Operator chose **`linux-native`** (kernel keyutils), declining
`async-secret-service` (pure-Rust zbus, which would have preserved Secret Service
semantics but carried an unverified sync-API-inside-tokio risk).

**Accepted trade-off, stated plainly:** the Linux kernel keyring is
session/user-scoped and **volatile — stored credentials do not survive a
reboot**, unlike Secret Service. This is a real behavioral change for Linux users
of the system-keystore path. `env://` and file-based credentials are unaffected.
It also brings Linux in line with the `*-native` pattern already used for
`apple-native` and `windows-native`.

## Changes

- Linux `keyring` feature `sync-secret-service` → `linux-native` in
  `crates/nono/Cargo.toml` and `crates/nono-cli/Cargo.toml`, each noting the
  lockstep requirement (two backends in one process = a silent split-brain store).
- Three `[features]` comments in `nono`, `nono-cli`, `nono-proxy` that read
  "macOS Keychain / **Linux Secret Service**" — now false — corrected, and
  `nono-cli`'s stale "disable this for headless environments without libdbus"
  advice removed.
- `Cross.toml` rewritten as a documented tombstone. Every line in it existed to
  provision libdbus; keeping the rationale beats a silent deletion, which would
  leave the next reader unable to tell "never needed one" from "removed by
  mistake".
- `Cargo.lock` regenerated.

## Verification

1. `cargo tree --target x86_64-unknown-linux-gnu -p nono-sandbox-cli -i
   libdbus-sys` → **no such package**.
2. No `dbus`/`secret-service` crate anywhere in that graph.
3. `linux-keyutils v0.2.5` reached via `keyring` → `nono-sandbox` →
   `nono-sandbox-cli`/`-proxy`.
4. Lockfile delta is exactly the intended swap: **removes** `dbus`,
   `dbus-secret-service`, `libdbus-sys`, `pkg-config`; **adds** `linux-keyutils`.
5. **CI's own assertion, reproduced against the real artifact.** Built the Linux
   release binary (`cross build --release`, 18m02s, exit 0) and ran
   `objdump -p | awk '/NEEDED/'` inside the cross image:
   ```
   libgcc_s.so.1  libutil.so.1  librt.so.1  libpthread.so.0
   libm.so.6      libdl.so.2    libc.so.6   ld-linux-x86-64.so.2
   ```
   No libdbus. This is direct evidence, not an inference from the dep tree.
6. Cross-target clippy: **linux-gnu exit 0** (13m09s), **apple-darwin exit 0** —
   the darwin gate re-run because `Cargo.lock` is shared even though the macOS
   dep table is untouched.
7. Code is backend-agnostic — every call site uses `keyring::Entry` and
   `keyring::Error::{NoEntry, Ambiguous}`, present regardless of backend.
   Checked before changing anything.

## Follow-up, recorded not done

`ci.yml`'s cross-compile job still runs `apt-get install -y pkg-config`. Now
unnecessary (the `pkg-config` crate left the graph) but harmless; removing it
widens the blast radius of a dependency change for no gate value.

## Finding worth carrying

**Two ratified decisions can contradict each other and both keep passing review,
because each is only ever read in its own file.** `Cross.toml` provisioned the
very thing `ci.yml` asserted must be absent. Nothing cross-checks a build-time
provisioning step against a build-time prohibition, so the conflict surfaced only
when a job finally ran both halves. When adding a portability *assertion*, check
what the build *provisions* — and vice versa.
