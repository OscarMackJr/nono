---
phase: 105
slug: live-multi-registry-publish
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-07-03
---

# Phase 105 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Scope + Irreversibility Note (READ FIRST)

Live registry publishing is IRREVERSIBLE and operator-in-loop. This phase AUTHORS + HARDENS + DRY-RUNS the publish machinery; the live publishes are operator checkpoints (D-01: sequenced only after Phase 104's `v0.66.1` tag). Per D-02, platform coverage is scoped to locally-buildable (Windows native + crates source + host PyPI wheel); reduced Linux/macOS coverage is a documented known limitation. Per D-03, publishing is `workflow_dispatch`-only. Per D-04, the npm `@oscarmackjr` scope creation is a hard operator precondition. Autonomous verification = dry-run-green + machinery correctness; the live publish itself is NOT automated.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None (publish-pipeline tooling, not app logic). Verification = dry-run-green + read-only registry-state checks. |
| **Config file** | none |
| **Quick run command** | `pwsh -File scripts/release-dry-run.ps1` (existing, safe, no live action) |
| **Full suite command** | `pwsh -File scripts/release-dry-run.ps1` + the new index-poll + npm platform-completeness checks (all runnable with zero registry credentials) |
| **Estimated runtime** | dry-run <1-2 min |

---

## Sampling Rate

- **After every task commit:** `pwsh -File scripts/release-dry-run.ps1` after every script/CI change (crates.io leg must stay green; PyPI/npm legs produce the expected not-yet-live SKIP/BLOCKED states)
- **After every plan wave:** the npm platform-completeness check + a fresh live re-check of name-availability/scope-ownership (a name can be squatted between research and execution)
- **Before phase gate (before any live-publish checkpoint is offered):** all above green + Phase 104 `v0.66.1` tag confirmed (D-01) + npm scope-ownership/auth + cargo/PyPI credentials operator checklist confirmed
- **Max feedback latency:** <2 min (dry-run)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 105-CRATES | TBD | 1 | PUB-02 (SC1) | squat / token-leak | crates.io publish in dependency order (nono-sandbox→-proxy→-cli) with index-visibility poll (cargo≥1.66 blocking + defensive sparse-index poll, NOT fixed sleep); resumable (skip already-published); token via env only, never echoed | integration (read-only pre-live) | `curl -A "<ua>" https://index.crates.io/no/no/nono-sandbox` (×3 → 404 not-yet-published); `pwsh scripts/crates-index-poll.ps1` unit-testable against an already-published crate | ❌ W0 | ⬜ pending |
| 105-PYPI | TBD | 1 | PUB-02 (SC2) | squat | maturin build (host) + `twine upload --skip-existing`; post-publish coverage verified via pypi.org JSON (not just exit code) | build + integration | `maturin build` (host, proven Phase 102); `curl https://pypi.org/pypi/nono-sandbox/json` (404 pre-publish) | ⚠️ W0 (twine install) | ⬜ pending |
| 105-NPM | TBD | 1 | PUB-02 (SC3) | broken-optionalDep DoS | publish platform packages FIRST then main package LAST; platform-completeness is a HARD pre-publish gate; build+publish the host-buildable `win32-x64-msvc` leg; make optionalDependencies CONSISTENT with the published set (avoid missing-platform-package failure); `--access public` for scoped | manifest/build | the Node platform-completeness one-liner (currently FAILS on the win32-x64-msvc gap — the gap this task closes) | ❌ W0 | ⬜ pending |
| 105-DISPATCH | TBD | 1 | PUB-02 | accidental-publish | publish-crates job converted to `workflow_dispatch`-only (D-03), never auto on tag-push; renamed selectors + dependency-order + index-poll; token via secrets env only | static/yaml | grep: publish-crates has `workflow_dispatch` trigger + no tag-push auto-run; renamed `-p nono-sandbox*` selectors; YAML parses | ❌ W0 | ⬜ pending |
| 105-RESOLVE | TBD | 2 | PUB-02 (SC4) | — | `cargo install nono-sandbox-cli` / `pip install nono-sandbox` / `npm i @oscarmackjr/nono-ts` all resolve in an isolated temp env post-publish | integration (post-live only) | `scripts/verify-post-publish-resolve.ps1` (isolated temp installs + cleanup) — runnable only AFTER live publish | ❌ W0 | ⬜ blocked-until-live |
| 105-LIVE | TBD | 2 | PUB-02 (SC1-3) | squat / irreversible | operator triggers the workflow_dispatch publish + PyPI + npm live, after a fresh availability/scope re-check + Phase 104 tag confirmation | manual (operator, outward-facing, IRREVERSIBLE) | N/A — operator checkpoint | N/A | ⬜ blocked (op + 104) |

*Status: ⬜ pending · ✅ green · ❌ red · blocked-until-live = only verifiable after the irreversible publish*
*Task IDs are placeholders until the planner assigns plan/wave numbers.*

---

## Wave 0 Requirements

- [ ] `pip install twine` — absent on this host; needed for the live PyPI upload leg
- [ ] `scripts/crates-index-poll.ps1` — new reusable sparse-index bounded-poll helper (no existing equivalent)
- [ ] `scripts/verify-post-publish-resolve.ps1` — new SC4 wrapper (isolated temp installs + cleanup)
- [ ] `../nono-ts/npm/win32-x64-msvc/` folder + host build leg (close the missing-platform-package gap for the shipped platform)
- [ ] (deferred per D-02) forking `nono-py`/`nono-ts` to `OscarMackJr/*` for full multi-platform CI — NOT in this phase

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live crates.io / PyPI / npm publish (SC1-3) | PUB-02 | Outward-facing + IRREVERSIBLE + needs credentials + gated on Phase 104 tag | Operator triggers the workflow_dispatch publish + PyPI/npm legs after the pre-publish re-check |
| npm `@oscarmackjr` scope creation | PUB-02 (D-04) | Requires an npm account/org the operator owns | Operator creates npm user/org `oscarmackjr`, authenticates, `npm publish --access public` |
| Post-publish resolve (SC4) | PUB-02 | Only meaningful once the packages actually exist live | `scripts/verify-post-publish-resolve.ps1` after publish |

*Autonomous work (the machinery, dry-run, index-poll helper, npm platform-gap fix, workflow_dispatch conversion) is all dev-host-verifiable; only the live publish + scope creation + post-publish resolve are manual.*

---

## Validation Sign-Off

- [x] All autonomous tasks have `<automated>` verify (dry-run / index-poll / platform-completeness / yaml grep)
- [x] Sampling continuity: dry-run after every machinery change
- [x] Wave 0 covers all MISSING references (twine, index-poll helper, resolve wrapper, win32 leg)
- [x] No watch-mode flags
- [x] Feedback latency < 2 min
- [x] `nyquist_compliant: true` set (plans 105-01..05 assign task IDs; Wave 0 gaps folded into Wave-1 tasks — twine install, crates-index-poll.ps1, verify-post-publish-resolve.ps1, win32 leg)
- [x] Registry tokens NEVER echoed/logged; pre-publish availability re-check before EVERY live checkpoint; publish is workflow_dispatch-only (never accidental)

**Approval:** pending
