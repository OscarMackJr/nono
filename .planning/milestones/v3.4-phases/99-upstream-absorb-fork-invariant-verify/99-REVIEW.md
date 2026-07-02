---
phase: 99-upstream-absorb-fork-invariant-verify
reviewed: 2026-06-30T00:00:00Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono-cli/src/capability_ext.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/setup.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/src/test_env.rs
  - crates/nono-cli/src/update_check.rs
  - crates/nono-proxy/Cargo.toml
  - crates/nono-proxy/README.md
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/credential.rs
  - crates/nono-proxy/src/lib.rs
  - crates/nono-proxy/src/pool.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/route.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono-proxy/src/token.rs
  - crates/nono/Cargo.toml
  - crates/nono/src/sandbox/linux.rs
findings:
  critical: 0
  warning: 4
  info: 4
  total: 8
status: issues_found
---

# Phase 99: Code Review Report

**Reviewed:** 2026-06-30
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

## Summary

Phase 99 hand-replayed 9 upstream commits (Clusters A/C/D/E/F/G) into the
security-critical sandboxing fork: the `NetworkIntent` refactor (72bcfd66),
contradictory-flag validation (d457ecc3), HTTP/2 pooling + endpoint routing
(cdeeb5b9/46bcfbb9/08ca19a8), the 9P-filesystem warning (5b8e94da), org-ref
migration (c808f000), and the sigstore-trust-root bump (2e64798d).

The core security-sensitive logic is generally sound and **fails closed** where
it matters:

- `host_port_matches` (route.rs) correctly guards the classic suffix-injection
  vuln via the `prefix.ends_with('.')` check — `*.openai.com:443` does NOT match
  `evilopenai.com:443`, and the apex is correctly excluded. No defect.
- `parse_allow_endpoint_arg` and `build_proxy_config_from_flags` are fail-fast
  (Result-returning, error on unknown service prefix). No silent no-ops.
- Removal of the `ProxyOnly { port: 0 }` placeholder does **not** introduce a
  fail-open: every CLI proxy trigger in `has_proxy_flags()` maps 1:1 to a
  `would_activate` trigger, so the "proxy cap set but proxy inactive" state is
  no longer reachable.
- No raw `.unwrap()`/`.expect()` in production hunks; all test usage is
  `#[cfg(test)]` + `#[allow(clippy::unwrap_used)]`.

No Critical findings. The defects below are a misleading/inert security
dependency, an inert HTTP/2 feature, an internal contradiction between the two
Cluster A commits, and a narrowed safety warning — plus four lower-severity
quality/accuracy items.

## Warnings

### WR-01: `sigstore-trust-root = "=0.9.0"` is an unused dependency; the TUF-root "bump" has no runtime effect

**File:** `crates/nono/Cargo.toml:54`
**Issue:** The replay of upstream 2e64798d adds a direct dependency on
`sigstore-trust-root = "=0.9.0"`, but no code in the `nono` crate references it.
Verification uses `sigstore_verify::trust_root::TrustedRoot`
(`crates/nono/src/trust/bundle.rs:32`), which is backed by `sigstore-verify =
"0.8.0"` and its transitive `sigstore-trust-root 0.8.0`. The in-tree comment
itself acknowledges the "dual-version resolution" and defers the
`sigstore-verify 0.8.0→0.9.0` bump to "a future sync phase."

Consequence: the actual attestation/TUF trust-root used at runtime remains
0.8.0. The 0.9.0 pin pulls a second, unused copy of a security-critical crate
into the dependency tree and the comment ("tracks the upstream-mandated coherent
version of the TUF root") implies a security upgrade that is not in effect. An
operator reading the changelog would believe the trust root was upgraded for
verification when it was not. This also conflicts with CLAUDE.md's "avoid
unused/dead code."
**Fix:** Either defer the trust-root pin until `sigstore-verify` is bumped to
0.9.0 in the same change (so the new root is actually consumed), or, if it must
land now, drop the direct dependency and reword the comment to state plainly
that the TUF root in effect is still 0.8.0. Do not leave an unused
security-crate version pinned in-tree.

### WR-02: `--allow-http2` / `ProxyConfig.enable_h2` is a runtime no-op

**File:** `crates/nono-proxy/src/pool.rs:173` (and `crates/nono-cli/src/cli.rs:2169`, `crates/nono-proxy/src/reverse.rs:61`)
**Issue:** `enable_h2` is consumed only by `UpstreamPool` (`build_pooled_client`
selects `enable_all_versions()` vs `enable_http1()`). But `UpstreamPool::send()`
is never called: the pool is stored on `ProxyState` (server.rs) and
`ReverseProxyCtx` (reverse.rs:61) yet `handle_reverse_proxy` still uses the
direct-TLS forwarding path (`tls_connector`), which ignores `enable_h2`. The
99-06 summary confirms this ("`UpstreamPool::send()` ... is not yet called from
`handle_reverse_proxy`").

Consequence: a user who sets `--allow-http2` (or profile `network.allow_http2`)
expecting HTTP/2 multiplexing silently gets HTTP/1.1. The flag, the schema
field, the config field, and the entire pool module are accepted and documented
as enabling HTTP/2 but have no effect on any real request. This is a
capability-claims-vs-reality gap in a feature surface that Phase 100 release
notes will advertise.
**Fix:** Either wire `UpstreamPool::send()` into `handle_reverse_proxy` so
`enable_h2` actually governs forwarding, or gate `--allow-http2`/`allow_http2`
behind a clear "not yet wired" warning (and mark the pool `#[cfg(...)]`/feature)
so users are not misled. At minimum, document in `--allow-http2` help text that
it is currently inert.

### WR-03: `validate_block_net_conflicts` contradicts the `strict_filter` design — profile `network.block` + `allow_domain`/`credential` is rejected instead of running in strict mode

**File:** `crates/nono-cli/src/sandbox_prepare.rs:601` vs `crates/nono-cli/src/proxy_runtime.rs:147`
**Issue:** `prepare_proxy_launch_options` is designed (per its own comments and
`strict_filter = prepared.profile_network_block`) to treat **profile
`network.block` combined with proxy flags as "strict mode: deny unlisted
hosts"** — `block_wins = args.block_net || (profile_network_block &&
!would_activate)`, so a profile block with active proxy config yields a live,
strict-filtered proxy.

However, the fork extended `validate_block_net_conflicts` to fold
`prepared.profile_network_block` into its `block_net` check (a fork deviation
beyond upstream d457ecc3, see 99-03 SUMMARY). That validator runs first (both in
`launch_runtime.rs` and the dry-run path in `command_runtime.rs`) and **errors**
when `profile_network_block` is true together with credentials, a network
profile, or `allow_domain`. The test `profile_network_block_with_credential_from_profile_errors`
locks this in.

Consequence: a profile such as `{ "network": { "block": true, "allow_domain":
["api.github.com"] } }` — the natural way to express "block everything except
GitHub via proxy filtering" — now fails with
`"--block-net and --allow-domain are contradictory"` instead of running in the
strict-filter mode the very next stage was written to produce. The
profile-driven `strict_filter` path is reachable only via `upstream_proxy` /
`custom_credentials` (the two triggers the validator does not check), making the
feature partially dead and the behavior internally inconsistent. It is
fail-closed (an error, not an opening), so this is a correctness/consistency bug
rather than a vulnerability.
**Fix:** Decide the intended semantics and make both commits agree. If profile
`block` + proxy flags should mean strict mode, the validator must scope its
contradiction checks to **CLI** `args.block_net` only (not
`profile_network_block`). If it should be an error, remove/adjust the
`strict_filter = prepared.profile_network_block` path and its comments so it is
not advertised as a supported mode.

### WR-04: 9P-filesystem safety warning is silently skipped for any 9P mount not under `/mnt`

**File:** `crates/nono/src/sandbox/linux.rs` (`unsupported_filesystem_dev`, the `if !path.starts_with("/mnt") { return None; }` guard)
**Issue:** The new warning exists to tell users that Landlock enforcement on 9P
mounts is unreliable ("grants may be silently ignored ... causing unexpected
access denials"). The detection short-circuits to `None` for any path not under
`/mnt`. 9P/`v9fs` and QEMU `virtfs` mounts are routinely mounted at arbitrary
paths (custom WSL2 `drvfs` mounts via `/etc/fstab`, VM shared folders under
`/home`, `/srv`, `/workspace`, etc.). For those, `statfs` is never called and no
warning is emitted.

Consequence: a user whose working directory is a 9P mount outside `/mnt`
receives no warning and may assume Landlock is enforcing their grants when it is
not — the exact false-assurance the warning was added to prevent. The path guard
uses `Path::starts_with` (component comparison, correct per CLAUDE.md), so this
is a scope limitation, not a string-comparison bug. It is diagnostic-only (no
enforcement decision depends on it), hence Warning not Critical.
**Fix:** Either run the `statfs` check for all capability paths (it is a single
syscall per unique mount, already deduplicated by `f_fsid`), or broaden the
fast-path guard to the set of mount roots 9P actually appears under, and update
the doc comment to state the heuristic's blind spot explicitly.

## Info

### IN-01: `endpoint_restrictions` application does not match its comment ("runs before domain-endpoint routes are merged")

**File:** `crates/nono-cli/src/proxy_runtime.rs:293-311`
**Issue:** The comment claims the `--allow-endpoint` prefix lookup "runs before
domain-endpoint routes are merged so the prefix lookup only matches credential
routes (never `_ep_*` entries)." In fact `resolved.routes.extend(endpoint_routes)`
(line 288) merges the `_ep_*` routes **before** `build_proxy_config` (289), and
the restriction loop (298) iterates `proxy_config.routes`, which already contains
them. A `service_prefix` whose trimmed value equals an `_ep_*` route prefix would
attach rules to that route via `find(...)`. The mis-attachment only **adds**
`endpoint_rules` (more restrictive), so it is fail-secure in direction and not
exploitable, but the stated invariant is unguarded and the comment is wrong.
**Fix:** Either restrict the `find` to credential routes explicitly (e.g. skip
`_ep_`-prefixed routes), or correct the comment to describe the actual ordering
and why over-matching is harmless.

### IN-02: Fail-secure guard in `execution_runtime.rs` is now unreachable (dead) with a stale comment

**File:** `crates/nono-cli/src/execution_runtime.rs:235-243`
**Issue:** The guard `if matches!(caps.network_mode(), ProxyOnly { .. }) &&
!network.is_proxy_active() { return Err(...) }` runs before
`start_proxy_runtime`. Since the `ProxyOnly { port: 0 }` placeholder was removed
from both `apply_cli_network_mode` and the profile path (capability_ext.rs), and
`ProxyOnly` is now set only inside `start_proxy_runtime` (which runs after this
point), `caps.network_mode()` can never be `ProxyOnly` here. The guard is dead
code; its comment still cites `capability_ext.rs:535` and `proxy_runtime.rs lines
52-78`, which no longer describe the code.
**Fix:** Remove the unreachable guard (the placeholder it defended is gone) or,
if kept as defense-in-depth, update the comment and add a test that can actually
reach it. Avoid leaving stale line-number references.

### IN-03: `unsupported_filesystem_dev` doc/return-type mismatch and incomplete `// SAFETY` for the `transmute`

**File:** `crates/nono/src/sandbox/linux.rs` (`unsupported_filesystem_dev`)
**Issue:** (a) The doc comment says the function "returns the device ID
(`st_dev`) of the mount," but it actually returns `f_fsid` transmuted to `u64`.
(b) The `// SAFETY:` comment justifies only the `statfs` out-pointer; the
`std::mem::transmute::<libc::fsid_t, u64>(stat.f_fsid)` relies on the size
equivalence of `fsid_t` (`[c_int; 2]`) and `u64` but the safety note does not
state that invariant. The transmute is sound (sizes are compile-time checked and
equal on supported targets), and the value is only a dedup key, so this is
documentation hygiene, not a bug.
**Fix:** Correct the doc to say `f_fsid`, and extend the SAFETY note to record
the 8-byte size equivalence (or use a `const` size assertion / `bytemuck`-style
cast).

### IN-04: `UpstreamPool` is allocated per proxy session but is dead infrastructure

**File:** `crates/nono-proxy/src/server.rs:307-309` (and `crates/nono-proxy/src/pool.rs`)
**Issue:** `UpstreamPool::new(...)` builds a real `hyper_util` `Client` (TLS
config clone, connector build) on every `server::start`, and the per-route
client `Mutex<HashMap<...>>` is allocated, yet `send()` is never invoked (see
WR-02). The 314-line module plus the new `bytes`/`http`/`hyper-util
client-legacy`/`hyper-rustls http2` dependencies exist solely as unused
infrastructure. It is reachable only through `pub` struct fields, so no
`dead_code` lint fires, but it conflicts with CLAUDE.md's "avoid dead code"
guidance and adds per-session allocation for no benefit.
**Fix:** Track the unwired pool as an explicit, time-boxed TODO and avoid
constructing it until it is consumed, or feature-gate it. Consolidate with the
WR-02 resolution.

---

_Reviewed: 2026-06-30_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
