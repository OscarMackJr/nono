---
slug: cargo-audit-rustls-webpki
created: 2026-06-10
type: quick
---

# Quick Task: Resolve Cargo Audit failure (rustls-webpki via sign-fixture)

## Problem

The CI **Cargo Audit** leg fails on 4 `rustls-webpki` advisories
(RUSTSEC-2026-0099 / 0098 / 0049 / 0104). The vulnerable instance is
`rustls-webpki 0.102.8`, pulled **only** by the dev/CI `sign-fixture` tool via
`sigstore-* 0.7.0` (`tools/sign-fixture/Cargo.toml`). Production nono binaries
already use the patched `rustls-webpki 0.103.13` (the main workspace uses
`sigstore-* 0.8.0`). Surfaced during the v2.10 Phase 65 CI rehab.

## Options considered

- **(a) Bump `sign-fixture` to `sigstore-* 0.8.0`** — real remediation; removes
  the vulnerable crate. Risk: `0.7→0.8` API breakage in `sign-fixture`'s source.
- **(b) Documented-ignore in `.cargo/audit.toml`** — zero code risk, matches the
  existing AWS-LC ignore pattern, but suppresses rather than fixes.

## Decision: (a) bump

Empirically validated: `0.8.0` is already in the workspace lock, so the bump
**dedups** rather than adds. `sign-fixture` compiled with `0.8.0` **with no
source changes**, the `Cargo.lock` diff is **purely subtractive** (drops
`rustls-webpki 0.102.8` + the entire `sigstore-* 0.7.0` subtree), and
`cargo audit` then exits 0. Strictly better than suppression — no `.cargo/audit.toml`
entry needed.

## Steps

1. `tools/sign-fixture/Cargo.toml`: `sigstore-sign` / `sigstore-oidc` /
   `sigstore-rekor` `0.7.0 → 0.8.0`.
2. Rebuild `sign-fixture` (verify no source changes needed).
3. Confirm `Cargo.lock` drops `rustls-webpki 0.102.8` + `sigstore 0.7.0`.
4. Confirm `cargo audit` exits 0 (only informational unmaintained warnings remain).

## Verification

- `cargo build -p sign-fixture` — PASS (Finished, no edits to `src/main.rs`)
- `grep rustls-webpki Cargo.lock` — only `0.103.13` remains
- `cargo audit` — exit 0
