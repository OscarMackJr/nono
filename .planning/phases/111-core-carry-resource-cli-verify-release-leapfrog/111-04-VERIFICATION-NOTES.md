# 111-04 Verification Notes — Combined 108-111 Fork-Invariant Pass (VERIFY-01)

> Live, pasted command output/exit codes for both mandatory cross-target clippy gates,
> the `make ci` constituents (`make` absent from PATH on this host), the
> `cargo test --workspace --no-fail-fast` baseline diff, both sibling-binding rebuilds,
> and the D-09 grep/diff structural assertions (D-01, ADR-86 carve-out).
>
> **This record supersedes `.planning/phases/110-profile-policy-absorb-platform-overrides/110-08-VERIFICATION-NOTES.md`
> per D-09.** That record predates 4 defect-fix commits landed 2026-08-04
> (`7c7a189c`/`ea26b5b2`/`6d7ef719`/`4aec1944`, closing PROF-03e's live-kernel checkpoint)
> and Phase 111's own CORE-01/CORE-02 changes — `has_port_rules()` shipped broken straight
> through the 110-08 certification. This pass covers the combined 108-111 surface honestly,
> not merely Phase 111's own small diff.

Host: Windows 11 (26200), Docker Desktop 29.6.2, `cross` 0.2.5, `cargo-zigbuild` 0.23.0,
`zig` 0.16.0, `make` absent (confirmed absent again this session).

---

## Task 1 — Both cross-target clippy gates + D-01/ADR-86 grep-diff assertions

### D-01 assertion: no `pub mod resource;` in core library

```
$ grep -n "^pub mod resource" crates/nono/src/lib.rs
(zero matches, exit 1)

$ ls crates/nono/src | grep -i resource
(zero matches, exit 1)
```

Full `pub mod` list in `crates/nono/src/lib.rs` (18 modules, none named `resource*`):
`agent, audit, capability, diagnostic, error, keystore, machine_policy, manifest,
manifest_convert, net_filter, path, query, sandbox, scrub, state, supervisor, trust, undo`.

**D-01 CONFIRMED: still holds after CORE-01/CORE-02.** Resource limits remain CLI-side per
ADR-111's ADAPT-not-adopt disposition of upstream `e6d26871`/#1269.

### ADR-86 Windows carve-out assertion: `exec_strategy_windows/` unregressed

```
$ git diff c5c8c5fc6e6b146ba5f27e6cd1d427b2da750ba2..HEAD -- crates/nono-cli/src/exec_strategy_windows/
(empty output, confirmed via `wc -l` = 0)
```

Base SHA `c5c8c5fc` = "docs(111): add validation strategy" (pre-Wave-1). Confirms neither
`111-01` (CORE-01: `policy.json` + `exec_strategy.rs` MAX_CRYPTO_THREADS) nor `111-02`
(CORE-02: `cli.rs` + `docs/cli/usage/flags.mdx` help text) touched this directory.

**ADR-86 Windows carve-out CONFIRMED: structurally unregressed.**

### Gate 1: linux-gnu (`cross clippy`)

```
docker info | grep "Server Version"  ->  Server Version: 29.6.2  (engine up)

cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
```

Exit code: **0**

Output (34 lines total — build layer cached from Plan 111-01/111-03's own gate runs on this
same unchanged tree since Wave 2 modifies no source; `Finished` in 14.82s with zero
`Compiling`/error/warning lines is expected incremental-fresh behavior, not a skipped run —
the pinned image layer itself was rebuilt/verified (`#4`/`#5`/`#6` BuildKit steps present)
before cargo ran):
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.82s
EXIT=0
```
Pinned image: `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5@sha256:9e5b39c09874bc1816c675ed11afca2c2ed6cee0c4ed2b3c1d5763c346c9ae3f`
(matches the checklist's recorded pin). Zero clippy findings.

### Gate 2: apple-darwin (`cargo-zigbuild clippy`, direct-binary form)

```
$ echo "SDKROOT=[$SDKROOT]"
SDKROOT=[]   (confirmed unset before the run)

cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

Exit code: **0**
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.17s
EXIT=0
```
Clean exit, no findings, no SDK extraction.

**Both mandatory gates GREEN over the combined 108-111 surface — no PARTIAL→CI fallback
invoked, no documented runner failure encountered.**

### Task 1 Result

D-01 confirmed holding. ADR-86 Windows carve-out confirmed structurally unregressed. Both
cross-target clippy gates GREEN, zero findings, re-run fresh over the combined 108-111 tree
per D-09/D-10.

---
