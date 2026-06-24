---
quick_id: 260619-duw
title: Scope upstream v0.62..v0.64 delta and plant SEED-006
date: 2026-06-19
status: complete
---

# Quick Task 260619-duw — Summary

## Outcome

Scoped the upstream `always-further/nono` **`v0.62.0..v0.64.0`** window
(**90 commits, 140 files changed**; releases `v0.63.0` #1161 + `v0.64.0` #1201) and
planted **`SEED-006`** with a function-level inventory for the future UPST9 sync.

Deliverable: `.planning/seeds/SEED-006-upst9-v0.62-v0.64-sync-window.md`.

## What was found (real, verified via `gh api .../compare`)

13 substantive themes (rest of the 90 commits are dependabot/docs/merge noise):

- **A (HIGH conflict):** audit/attestation/ledger stack moved INTO core `nono` crate —
  NEW `crates/nono/src/audit.rs` (+1773); CLA audit_* files gutted to thin wrappers.
- **B (HIGH conflict):** structured-diagnostics model in core lib + FFI — NEW
  `crates/nono/src/diagnostic/{codes,observation,records,report,detail,mod}.rs`,
  `bindings/c/src/diagnostic.rs`, `crates/nono-proxy/src/diagnostic.rs`.
- **C (SECURITY, Linux):** AF_UNIX datagram bypass trap (`sendto`/`sendmsg`/`sendmmsg`)
  in `sandbox/linux.rs` + `supervisor_linux.rs`; `deduplicate()` procfs-remap guard.
- **D–M:** `set_vars` env injection, XDG state dirs (`state_paths.rs`), proxy hardening
  (route shadow fix, 403+audit, TLS upstream_proxy, customCredentials), AWS auth config,
  keyring timeout, `$PACK_DIR` session hooks, PTY ctrl-z fix, CI-env update discovery,
  profile namespace, truthy bool-flag env, plus dependency bumps.

Themes A & B violate the fork's **policy-free library** invariant (CLAUDE.md § Library
vs CLI) — UPST9 must give them an explicit divergence-ledger disposition (likely
`split`/`fork-preserve`), not a blind cherry-pick.

## Notes / fork context

- Fork crate `0.62.2`; sync high-water upstream `v0.61.2` (UPST8 / v2.11). A future
  fork release must leapfrog ≥ `0.65.0` (upstream now `0.64.0`).
- No code changed — documentation/seed only. No tests run (none applicable).
- `e54cf9cb` (remove `env_clear` from session_hook subprocess) intersects the fork's
  Windows `env_clear` CLR-fail gotcha — flagged in the seed for UPST9 review.
