---
quick_id: 260629-toe
slug: v066-parity
title: "UPST11 — v0.66.0 Upstream Parity Phase Definition"
status: complete
date: 2026-06-29
kind: phase-definition
---

# UPST11 — Upstream Parity to v0.66.0 (Phase Definition)

> **What this is:** a ready-to-execute phase definition (divergence ledger + structure +
> gates) for bringing the fork to parity with upstream `nolabs-ai/nono` **v0.66.0**.
> Per `/gsd-quick`, the deliverable here is the *plan*, not the merge. The merge itself is
> milestone-shaped — promote this via `/gsd-new-milestone` (UPST11) or `/gsd-phase add` when
> ready to execute. See **Promotion** at the bottom.

## Source

- Upstream release PR: **[#1293 `chore: release v0.66.0`](https://github.com/nolabs-ai/nono/pull/1293)** — MERGED 2026-06-29 (Luke Hinds, `605bb6c5`).
  - #1293 is a *release-cut* PR only: CHANGELOG entry + version bumps `0.65.1 → 0.66.0`. The
    actual code lives in the ~19 PRs the CHANGELOG references (below).
- **Parity gap = upstream `v0.65.1 → v0.66.0`.** The fork's confirmed upstream high-water mark
  is **v0.65.1** (absorbed in milestone v3.3 / UPST10, Phases 94-97).

## ⚠️ Version-collision note (release-blocking, not merge-blocking)

The fork's crate version is **already `0.66.0`** — it was leapfrogged in v3.3 as the "first
SemVer > upstream 0.65.1." Upstream has now *also* shipped `0.66.0`, so the leapfrog is spent.
**The next real fork release must leapfrog to ≥ `0.67.0`** to stay strictly above upstream and
avoid a crates.io / git-tag collision. Do the bump as part of this phase's release-reconciliation
step (or carry it to the next release milestone), but do **not** publish `0.66.0` from the fork.

## Divergence Ledger (per upstream PR)

Dispositions are **preliminary** — confirmed against a fast collision scan of the fork tree.
Each row's "verify on absorb" note is the residual work for the executor. Disposition keys:
**ADOPT** (take as-is, may cfg-gate), **ADAPT** (take but reconcile with fork divergence),
**VERIFY** (likely already present / N/A — confirm), **HIGH-CONFLICT** (touches a fork
invariant; needs a deliberate adopt-vs-fork-divergence decision + cross-target verify).

### Refactoring — the critical item

| PR | Title | Disposition | Notes |
|----|-------|-------------|-------|
| [#1225](https://github.com/nolabs-ai/nono/pull/1225) | network: introduce `NetworkIntent`, remove `ProxyOnly` placeholders | **HIGH-CONFLICT** | Fork has deep `NetworkMode::ProxyOnly` usage in `crates/nono/src/capability.rs` (constructors at ~1045/1065, matches at ~843/1386/2711+), `crates/nono/src/manifest_convert.rs:47`, `crates/nono/src/sandbox/linux.rs:386/620`. Fork has **no** `NetworkIntent`. This restructures the core network-capability enum that the Windows WFP/AppContainer backends and the proxy-filter post-fork path key off of. **Decision required:** full-sync-adopt (precedent: v3.1 Phase 86 adopted upstream's high-conflict refactors) vs fork-divergence carve-out. Whichever — must NOT regress the policy-free library boundary (ADR-86) or the Windows network model, and MUST pass local cross-target clippy on both Unix gates. This item alone justifies a dedicated absorb wave. |

### Features

| PR | Title | Disposition | Notes |
|----|-------|-------------|-------|
| [#1268](https://github.com/nolabs-ai/nono/pull/1268) | tool-sandbox: simplify self-invocation policy | **ADAPT** | tool-sandbox is Windows-relevant (sandbox-the-tools hook model). Reconcile with fork's PreToolUse→`nono run` wrapping. |
| [#1271](https://github.com/nolabs-ai/nono/pull/1271) | tool-sandbox: add `@git:common-dir` dynamic token | **ADOPT** | Not present in fork → clean add. cfg-gate token resolution if it shells to `git`. |
| [#983](https://github.com/nolabs-ai/nono/pull/983) | proxy: HTTP/2 support for reverse proxy + credential injection | **ADAPT** | Proxy is heavily fork-diverged; `h2` already in `Cargo.lock`. Verify against fork's "proxy filtering not implemented for Windows supervised runs" path — don't claim a capability Windows doesn't enforce. |
| [#1213](https://github.com/nolabs-ai/nono/pull/1213) | tests: e2e integration tests for sandbox execution strategies | **ADOPT** | New tests. cfg-gate the Unix-only legs; ensure they don't hang on Windows (cf. the 6h ubuntu seccomp-hang lesson). |

### Bug Fixes

| PR | Title | Disposition | Notes |
|----|-------|-------------|-------|
| [#1263](https://github.com/nolabs-ai/nono/pull/1263) | network: error early on contradictory network flag combos | **ADOPT** | CLI validation; pairs with #1225's `NetworkIntent`. Apply after #1225 lands. |
| [#1127](https://github.com/nolabs-ai/nono/pull/1127) | network: wire `--allow-endpoint` through to credential routes | **VERIFY/ADAPT** | Flag already exists in fork `crates/nono-cli/src/cli.rs:2073-2077` and conflict list at :2187. The *wiring to credential routes* may be the missing piece — diff fork's endpoint→route path. |
| [#1207](https://github.com/nolabs-ai/nono/pull/1207) | sandbox: warn when capability path on a 9P filesystem | **ADOPT** | Linux-specific; cfg-gate. Pure diagnostic. |
| [#1253](https://github.com/nolabs-ai/nono/pull/1253) | tool-sandbox: skip missing fs_read/fs_write dirs instead of erroring | **ADOPT** | Behavior change; confirm it doesn't weaken fail-secure (skipping a *missing* dir is fine; skipping a *failed-to-resolve* dir is not). |
| [#1249](https://github.com/nolabs-ai/nono/pull/1249) | tool-sandbox: pass TLS trust-bundle env vars to children | **ADAPT** | Windows env handling caveat: fork's interpreter spawn does `env_clear()` then re-adds a baseline (SystemRoot/windir/...). Ensure the trust-bundle vars survive that clear on Windows (cf. windows_hook_interpreter_spawn_gotchas). |
| [#1243](https://github.com/nolabs-ai/nono/pull/1243) | proxy: match wildcard credential upstream routes | **ADOPT** | Route-matching fix; add proxy unit test. |

### CI/CD

| PR | Title | Disposition | Notes |
|----|-------|-------------|-------|
| [#1251](https://github.com/nolabs-ai/nono/pull/1251) | fix mapping err in compile step | **ADOPT** | Reconcile against fork's workflow edits. |
| [#1245](https://github.com/nolabs-ai/nono/pull/1245) | idempotent publish-crates + cross-compile check on release PRs | **ADAPT** | Reconcile with the fork's **prepare-only** release pipeline + release-readiness gate built in v3.3 Phase 97. Idempotent-publish is friendly to the fork's operator-gated push model. |

### Dependencies

| PR | Title | Disposition | Notes |
|----|-------|-------------|-------|
| [#1229](https://github.com/nolabs-ai/nono/pull/1229) | bump `sigstore-trust-root` 0.8.0 → 0.9.0 | **ADOPT** | Fork is at **0.8.0** (confirmed in `Cargo.lock`). Check the sigstore-rs cascade (sigstore-verify/sigstore-sign) for compatible pins. |
| [#1232](https://github.com/nolabs-ai/nono/pull/1232) | bump `criterion` 0.5.1 → 0.8.2 | **ADOPT** | Dev-dep (benches). Low risk; confirm bench compiles. |

### Documentation

| PR | Title | Disposition | Notes |
|----|-------|-------------|-------|
| [#1247](https://github.com/nolabs-ai/nono/pull/1247) | proxy: explain activation via custom credentials | **ADOPT** | Docs. Reconcile with fork's proxy-divergence docs. |
| [#1246](https://github.com/nolabs-ai/nono/pull/1246) | proxy: fix stale `X-Nono-Token` authentication claim | **ADOPT** | Docs accuracy fix. |

### Miscellaneous

| PR | Title | Disposition | Notes |
|----|-------|-------------|-------|
| [#1235](https://github.com/nolabs-ai/nono/pull/1235) | migrate org refs `always-further` → `nolabs-ai` | **VERIFY** | Main tree is **clean** of `always-further` (refs found only in a stale `.claude/worktrees/` agent worktree). The fork already moved (was OscarMackJr/nono per history; upstream relocated to nolabs-ai). Confirm no live source/CI refs, then mark N/A. |

## Fork invariants the absorb must NOT regress (gate list)

1. **Windows security model** — AppContainer + WFP + IL backends; supervisor-led. Anything in
   #1225/#983/#1268/#1249 that touches network or tool-sandbox must preserve kernel-enforced
   confinement and the "fail secure on unsupported shape" rule.
2. **Policy-free library boundary (ADR-86)** — the `NetworkIntent` refactor must not leak policy
   into core `nono`. Keep policy in `nono-cli`.
3. **Local cross-target clippy GREEN** — Docker `cross` (linux-gnu) + zig `cargo-zigbuild`
   (apple-darwin), both `-D warnings -D clippy::unwrap_used`. **No PARTIAL→CI** (retired in v3.3
   Phase 96). #1225, #1207, #1213, #1249 all touch cfg-gated Unix code = the classic
   Windows-host blind spot.
4. **`make ci` not clippy-only** — clippy + fmt + tests. Rustfmt has bitten twice past a
   clippy-only gate.
5. **Commit hygiene** — cherry-picks with `-x`, DCO `Signed-off-by`, Conventional Commit titles
   (lowercase initial, `feat`/`fix`/`chore` types only).
6. **No `0.66.0` publish** — leapfrog to ≥ `0.67.0` before any real release.

## Recommended phase structure (when promoted)

Mirrors the proven UPST10/v3.3 shape, scaled for a single-release (19-PR) gap:

- **Wave A — Divergence Audit.** Produce the full per-commit ledger for the `v0.65.1..v0.66.0`
  window (the CHANGELOG groups PRs, but cherry-pick works at the commit level). Confirm every
  disposition above; lock the #1225 adopt-vs-divergence decision (ADR if divergence).
- **Wave B — Absorb + Fork-Invariant Verify.** Cherry-pick/adapt in dependency order
  (#1225 → #1263; deps before code that uses them). Preserve all six gates. Run local
  cross-target clippy after each cfg-gated absorb. Code-review + verifier pass for the
  Windows-invariant regressions (the v3.3 absorb caught four).
- **Wave C — Release reconciliation.** Apply #1245/#1251 CI changes against the prepare-only
  pipeline; bump crate version to ≥ `0.67.0`; refresh release-readiness gate. (May be deferred
  to the next release milestone if this phase is sync-only.)

## Promotion

This file is a **phase definition**, not an executed merge. To act on it:

- `/gsd-new-milestone` → name it **UPST11 — Upstream Sync to v0.66.0**, seed the three waves
  above as Phases (Audit / Absorb+Verify / Release-reconcile), then `/gsd-plan-phase` each.
- Or `/gsd-phase add` a single "v0.66 Parity" phase if you want it as one phase rather than a
  milestone.

Do **not** cherry-pick blind — the fork's documented process (and the four regressions UPST10
caught) require the audited audit→absorb→verify path.
