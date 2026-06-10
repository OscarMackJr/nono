---
slug: cargo-audit-rustls-webpki
completed: 2026-06-10
status: complete
commit: 4aaa0508
---

# Summary: Cargo Audit rustls-webpki resolution

**Outcome:** Resolved by bumping `tools/sign-fixture` from `sigstore-* 0.7.0` to
`0.8.0` (commit `4aaa0508`). Chose remediation over the `.cargo/audit.toml`
documented-ignore alternative after empirically confirming the bump is clean.

**Why bump won over ignore:**
- `sigstore-* 0.8.0` was already resolved in the workspace lock (the main crates
  use it), so the bump **dedups** rather than adds.
- `sign-fixture` compiled with `0.8.0` with **zero source changes** to `main.rs`.
- The `Cargo.lock` diff is **purely subtractive** — removed `rustls-webpki
  0.102.8` (the vulnerable crate) + the entire `sigstore-* 0.7.0` subtree
  (~247 lines removed, 48 added).
- `cargo audit` exits **0** afterward; the only remaining items are
  informational unmaintained warnings (`async-std`, `rustls-pemfile`) which do
  not fail the audit, plus the 2 pre-existing AWS-LC `[advisories] ignore`
  entries in `.cargo/audit.toml` (left untouched).

This is real remediation (the vulnerable crate is gone from the dependency
graph) rather than suppression, and required no new ignore entries.

**Verification (local):**
- `cargo build -p sign-fixture` → Finished, no edits needed
- `grep rustls-webpki Cargo.lock` → only `0.103.13`
- `cargo audit` → exit 0

**Not yet pushed:** batched with the pending Phase 65 macOS Test fix to avoid
superseding the in-progress CI run whose macOS failure log is still being read.
CI Cargo Audit leg will confirm green on the next push.
