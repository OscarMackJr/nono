---
phase: 70-upst8-cherry-pick-sync
reviewed: 2026-06-13T00:00:00Z
depth: standard
files_reviewed: 21
files_reviewed_list:
  - crates/nono-cli/data/network-policy.json
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/src/package_cmd.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile_runtime.rs
  - crates/nono-cli/src/profile_save_runtime.rs
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/wiring.rs
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/filter.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono/src/diagnostic.rs
  - crates/nono/src/net_filter.rs
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
status: issues_found
---

# Phase 70: Code Review Report

**Reviewed:** 2026-06-13
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Summary

Phase 70 absorbs three upstream cherry-pick clusters: C3 (diagnostic suppression
+ profile registry-ref extends), C4 (`nono pull --force` recovery + strict pack
verification), and C2 (deny-by-default network policy when `network.block` is
requested + credential removal from embedded network profiles).

The security-sensitive parts are mostly sound and, in several places, strictly
more secure than the pre-change code:

- `HostFilter::new_strict` correctly denies on empty allowlist; the deny/link-local
  checks still run first, so strict mode cannot be used to bypass cloud-metadata
  protection. Wildcard suffix matching remains component-safe (`*` stripped to
  `.example.com`, `ends_with` + length guard prevents `evilexample.com`).
- The diagnostic suppression list cannot hide real filesystem denials — both
  `diagnostic.rs` (`retain` over `non_fs_violations` only) and
  `exec_strategy.rs` (`filter_suppressed_system_service_violations` keeps any op
  with an FS-access mapping) structurally prevent suppression of FS denials.
- `verify_profile_packs` was hardened from soft-skip to hard-fail on missing
  lockfile entry / missing trust bundle (fail-secure, correct).
- Credential removal from embedded network profiles closes implicit credential
  disclosure; the new sweep test asserts ALL embedded profiles resolve to zero
  implicit credentials.

The principal concern is that the C2 deny-by-default mechanism is **inert in the
fork's architecture** (WR-01): `strict_filter` is only consumed when the proxy is
active, but the only triggers for `network_block` (`--block-net` /
`profile.network.block`) force `NetworkMode::Blocked`, which forces the proxy
inactive. The fix compiles, tests pass, but it never engages on the documented
path. Two lower-severity wiring/UX concerns (WR-02, WR-03) and four INFO items
round out the findings. No BLOCKER-level defects were found.

## Warnings

### WR-01: C2 `strict_filter` deny-by-default is unreachable on the `network.block` path

**File:** `crates/nono-cli/src/proxy_runtime.rs:90-116`, `crates/nono-cli/src/proxy_runtime.rs:220`, `crates/nono-cli/src/capability_ext.rs:1015-1016`, `crates/nono-cli/src/capability_ext.rs:1175-1176`

**Issue:** The C2 fix wires `network_block` → `ProxyConfig.strict_filter` so the
proxy filter denies unlisted hosts (instead of allow-all) when an empty allowlist
is produced under `network.block`. But `strict_filter` is only consumed inside
`server::start`, which only runs when `proxy.active == true`
(`start_proxy_runtime` returns early when `!proxy.active`).

`proxy.active` is computed in `prepare_proxy_launch_options`:
```rust
let active = if matches!(prepared.caps.network_mode(), nono::NetworkMode::Blocked) {
    // ... warn ...
    false        // <-- proxy never starts when network is Blocked
} else { /* proxy/credential/profile flags present */ };
```
Both triggers for `network_block_requested` force `NetworkMode::Blocked`:
- `args.block_net` → `caps.set_network_blocked(true)` (`capability_ext.rs:1175`)
- `profile.network.block` → `caps.set_network_blocked(true)` (`capability_ext.rs:1015`)

And `network_block_requested = args.block_net || profile_network_block`
(`sandbox_prepare.rs:565`). Therefore whenever `network_block` is true, the proxy
is forced inactive and `strict_filter` is never read. There is no code path where
`network_block == true` AND `proxy.active == true`. The `network.block` case the
SUMMARY claims to fix (T-70-03-02) is in this fork handled by full OS-level
network blocking, so the upstream's "proxy allow-all fallback under block" threat
does not exist here in the first place — and the ported guard never engages.

This is not a vulnerability (the fork is already fail-secure: blocked = no proxy =
no network), but it is a defense that does nothing on its documented trigger, with
tests (`proxy_runtime.rs` strict-filter tests) that only exercise the synthetic
`ProxyLaunchOptions { active:true, network_block:true }` combination that the real
preparation pipeline can never produce. Future refactors that decouple
`network.block` from `NetworkMode::Blocked` would silently rely on this guard
without it ever having been exercised end-to-end.

**Fix:** Either (a) document in the C2 code/SUMMARY that `strict_filter` is a
forward-compat guard that is intentionally inert in the current fork because
`network.block` maps to full OS-level `NetworkMode::Blocked` (no active proxy),
mirroring how WR-01-equivalent stubs are documented elsewhere; or (b) add an
end-to-end assertion that `network_block == true` implies `proxy.active == false`
so the contract is locked, e.g.:
```rust
// In prepare_proxy_launch_options, after computing `active`:
debug_assert!(
    !(active && prepared.network_block_requested),
    "network.block must force the proxy inactive; strict_filter would otherwise \
     be the only line of defense and is not exercised on this path"
);
```
At minimum, replace the synthetic `active:true, network_block:true` test fixtures
with a test that drives `prepare_proxy_launch_options` from a real
`PreparedSandbox` with `network_block_requested:true` and asserts the resulting
`active == false`.

### WR-02: `nono pull --force` recovery is advertised in doc comments but does nothing

**File:** `crates/nono-cli/src/package_cmd.rs:868-889`, `crates/nono-cli/src/wiring.rs:23-62`

**Issue:** `install_package` now takes a `force` parameter and constructs
`ExecuteOptions { allow_unmanaged_identical_write_files: force }`, but the struct
is only passed to `tracing::debug!` and then dropped — it is never threaded into
any write/execute path. The doc comments on `ExecuteOptions` describe concrete
recovery behavior ("adopt an existing unmanaged destination when its content
exactly matches the pack source (SHA-256 byte-exact comparison)") as if it were
active. A user running `nono pull <pack> --force` to recover after metadata loss
will see the documented behavior promised but receive only a debug log line. The
real recovery effect of `--force` (lockfile up-to-date bypass at
`package_cmd.rs:31` and signer-pinning override at `package_cmd.rs:49`) is
unrelated to this new stub.

**Fix:** This is an accepted forward-compat stub (deferred to v2.5-FU-3), so the
fix is documentation honesty, not behavior: change the `ExecuteOptions` doc
comment and `install_package` comment to use future/conditional phrasing (e.g.
"WILL allow ... once the WriteFile execute path lands (v2.5-FU-3); currently a
no-op recorded for tracing only"). Avoid present-tense descriptions of behavior
that is not yet wired, so the next reader/auditor does not assume the byte-exact
adoption is enforced.

### WR-03: Suppressed FS-mapped operations with no target are treated inconsistently between modules

**File:** `crates/nono-cli/src/exec_strategy.rs:1942-1957`, `crates/nono/src/diagnostic.rs:1192-1198`

**Issue:** `filter_suppressed_system_service_violations` (exec_strategy) keeps a
violation if `seatbelt_operation_to_access(op).is_some() || !suppressed.contains(op)`.
`violations_to_denials` (diagnostic) only routes a violation to the
non-FS-suppressible list when it has **both** an FS-access mapping **and** a
`target`. A violation like `SandboxViolation { operation: "file-read-data",
target: None }` therefore:
- is **kept** by `filter_suppressed_system_service_violations` (because
  `seatbelt_operation_to_access` returns `Some`), even if the operation name is in
  the suppress list; but
- is **eligible for suppression** inside `diagnostic.rs::format_footer`, because
  `violations_to_denials` pushes it to `non_fs_violations` (target is `None`), and
  the new `retain` then drops it if the op is in the suppress list.

The two layers disagree on whether a target-less FS-op is suppressible. The
security impact is benign (this only affects footer rendering, never enforcement,
and an FS denial without a target is non-actionable anyway), but the divergence is
a latent correctness trap: the `should_offer_profile_save` gate keys off the
exec_strategy-filtered `visible_sandbox_violations`, so a target-less suppressed
FS-op would still trigger the save prompt even though it is hidden from the footer
by `diagnostic.rs`.

**Fix:** Make the two filters use the same predicate. Either gate the
exec_strategy "keep FS op" branch on `target.is_some()` to match
`violations_to_denials`:
```rust
.filter(|violation| {
    let is_fs_with_target = violation.target.is_some()
        && nono::diagnostic::seatbelt_operation_to_access(&violation.operation).is_some();
    is_fs_with_target || !suppressed_operations.contains(&violation.operation)
})
```
or expose a single shared helper from `nono::diagnostic` and call it from both
sites so suppression semantics cannot drift.

## Info

### IN-01: Credential removal from embedded profiles is a silent behavior change for existing users

**File:** `crates/nono-cli/data/network-policy.json:118-152`

**Issue:** `claude-code`, `codex`, `developer`, and `opencode` network profiles no
longer carry implicit `credentials` arrays. Users who relied on
`--network-profile claude-code` to auto-inject the `anthropic`/`github`/`gitlab`
credential routes will now get host allow-listing but no credential injection
unless they pass `--credential` explicitly. This is the intended (more secure)
behavior and is well-covered by the new sweep test, but it is a breaking UX change
with no in-code migration hint.

**Fix:** Ensure release notes / CHANGELOG call out that network profiles no longer
auto-enable credential routes and that `--credential <name>` is now required.
Optionally emit a one-line stderr hint when a known profile is selected without
any credentials resolved.

### IN-02: `is_registry_ref` accepted as `extends` base broadens what a saved profile can inherit

**File:** `crates/nono-cli/src/profile_save_runtime.rs:618-625`, `crates/nono-cli/src/profile/mod.rs:2541-2551`

**Issue:** `prepare_profile_save_from_patch` now allows a registry ref (e.g.
`always-further/claude@1.2.0`) as the saved profile's `extends` base, in addition
to local profile names. `is_registry_ref` is component-aware and rejects `.`/`~`/`/`
prefixes and `.json` suffixes, so it is not a path-traversal vector. The self-
reference guard (`*name != profile_name`) is preserved and tested. No defect; noted
only because the `extends` resolution path now reaches the auto-pull registry
machinery from the profile-save flow, widening the trust surface of a saved patch
to whatever signer that registry pack pins.

**Fix:** None required. Confirm that registry-ref `extends` resolution at load time
still enforces signer pinning (it does, via `verify_profile_packs`), and that a
saved profile inheriting a registry ref cannot be loaded with auto-pull silently
fetching an unpinned pack.

### IN-03: `execution_runtime.rs` removed local `ignored_denial_paths` computation — verify all call sites populate the flag

**File:** `crates/nono-cli/src/execution_runtime.rs:422-461`

**Issue:** The local `ignored_denial_paths` computation in `execute_sandboxed` was
removed in favor of `flags.ignored_denial_paths` (now populated upstream in
`profile_runtime.rs`). This is correct for the `run`/`shell`/`wrap` paths which set
the field from `PreparedSandbox`. The risk is a future `LaunchPlan` producer that
forgets to populate `flags.ignored_denial_paths` would silently lose the
`[save skipped]` annotation (UX only, not security). `ExecutionFlags::defaults`
initializes it to `Vec::new()`, so the failure mode is graceful (no annotations).

**Fix:** None required; documented here so the dependency on upstream population is
explicit. Consider a comment on `ExecConfig.ignored_denial_paths` noting it must be
sourced from `PreparedSandbox`, not recomputed.

### IN-04: Trailing blank lines / extra blank line in `wiring.rs` stub block

**File:** `crates/nono-cli/src/wiring.rs:62-63`

**Issue:** The new `ExecuteOptions` block introduces a double blank line after the
struct (`...}` followed by two blank lines before the next section banner). Purely
cosmetic; `cargo fmt` may or may not normalize it depending on config.

**Fix:** Run `cargo fmt --all` to normalize spacing.

---

_Reviewed: 2026-06-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
