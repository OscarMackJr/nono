---
phase: 102
slug: fork-owned-package-rename
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-07-03
---

# Phase 102 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None applicable — this phase is a manifest/config identity rename with no new runtime logic to unit-test. Verification is build-green + live registry-availability checks, mirroring the phase's own success criteria. |
| **Config file** | none |
| **Quick run command** | `cargo check --workspace --all-targets` (fast compile-only check after each manifest edit — catches E0433 import breakage from a mis-renamed dependency key immediately) |
| **Full suite command** | `cargo build --workspace --all-targets` + `make build` + `cargo fmt --all -- --check` (this repo — build-green gate; NOT `make ci`/`make test`, which trip the documented pre-existing Windows baseline failures) + `maturin build` (../nono-py) + `npx napi build --platform --release` (../nono-ts) |
| **Estimated runtime** | ~cargo check <60s; full builds several min each |

---

## Sampling Rate

- **After every task commit:** `cargo check --workspace --all-targets` (after each manifest edit)
- **After every plan wave:** `make build` (this repo) + `maturin build` (../nono-py) + `napi build --platform --release` (../nono-ts)
- **Before phase gate:** all three full builds green, PLUS a fresh live re-check of the three registry availability endpoints (in case a name was claimed between research and execution)
- **Max feedback latency:** ~60s for the per-task `cargo check`

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 102-SC1 | TBD | 1 | PUB-01 (SC1) | T-102-02 | crates.io `[package] name` renamed on 3-crate publish set; internal path-deps reconciled via `package =`; `[[bin]]`/`[lib] name = "nono"` unchanged; `use nono::` still compiles | build/manifest | `cargo check --workspace --all-targets && grep -c 'name = "nono"' crates/nono-cli/Cargo.toml` | ✅ | ⬜ pending |
| 102-SC2 | TBD | 1 | PUB-01 (SC2) | — | nono-py `pyproject.toml [project] name` → `nono-sandbox`; nono-ts `package.json name` → `@oscarmackjr/nono-ts` | manifest | `grep '^name' ../nono-py/pyproject.toml`; `node -e "console.log(require('../nono-ts/package.json').name)"` | ✅ | ⬜ pending |
| 102-SC3 | TBD | 1 | PUB-01 (SC3) | T-102-01 | each new registry identity confirmed AVAILABLE (404) live before committing the rename | integration (network) | `curl -s -A "nono-plan/1.0" -o /dev/null -w "%{http_code}" https://crates.io/api/v1/crates/<name>` (×3 → 404), `curl .../pypi/nono-sandbox/json` (404), `curl .../@oscarmackjr%2Fnono-ts` (404) | ✅ | ⬜ pending |
| 102-SC4 | TBD | 2 | PUB-01 (SC4) | T-102-02 | workspace + both binding builds green under new names | build | `make build`; `cd ../nono-py && maturin build`; `cd ../nono-ts && npx napi build --platform --release` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*Task IDs are placeholders until the planner assigns plan/wave numbers.*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* Existing build tooling (`cargo`, `maturin`, `napi`) fully covers this phase — no new test files or fixtures are needed since there is no new runtime logic, only manifest identity changes.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| npm `@oscarmackjr` scope ownership | PUB-01 (out-of-scope note) | Registry account state, not a build artifact; this is a Phase 105 publish precondition, NOT a Phase 102 blocker | Confirm `www.npmjs.com/~oscarmackjr` exists before Phase 105 publish (currently 404) |

*All in-phase behaviors have automated verification; the manual item is an explicitly-deferred Phase 105 precondition.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify (build/manifest/network commands) — no Wave 0 dependencies needed
- [x] Sampling continuity: `cargo check` after every task; no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none — existing tooling suffices)
- [x] No watch-mode flags
- [x] Feedback latency < 60s (per-task cargo check)
- [x] `nyquist_compliant: true` set in frontmatter (plans 102-01..05 now assign task IDs; SC→command map encoded into task acceptance criteria)

**Approval:** approved 2026-07-03
