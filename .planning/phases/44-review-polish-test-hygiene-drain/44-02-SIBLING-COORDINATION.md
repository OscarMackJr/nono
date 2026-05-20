# Phase 44 Plan 44-02 — Sibling Repo Coordination Log

Per D-44-D1 + D-44-D2: sibling-repo URLs derived from this repo's
`git remote -v` upstream entry **at execute-time**.

## Derivation (D-44-D2)

Raw values captured at execute-time (verifier-greppable form):

```
UPSTREAM_URL=https://github.com/always-further/nono.git
DERIVED_ORG=always-further
```

- Upstream URL (from `git remote get-url upstream || origin`): `https://github.com/always-further/nono.git`
- Derived org (`$DERIVED_ORG`): `always-further`
- Derivation matches historically observed `always-further`: yes
- Derived-org deviation checkpoint fired: no (DERIVED_ORG matches historically observed value; auto-proceed to Option A path)
- Derived sibling URLs:
  - nono-py: `https://github.com/always-further/nono-py.git`
  - nono-ts: `https://github.com/always-further/nono-ts.git`

## Existence check (gh repo view)

| Repo                       | Status                                              | Local clone               |
| -------------------------- | --------------------------------------------------- | ------------------------- |
| `always-further/nono-py`   | exists (`https://github.com/always-further/nono-py`) | `C:\Users\OMack\nono-py` (newly cloned) |
| `always-further/nono-ts`   | exists (`https://github.com/always-further/nono-ts`) | `C:\Users\OMack\nono-ts` (newly cloned) |

## Clone provenance

- nono-py clone: `git clone https://github.com/always-further/nono-py.git` at `C:\Users\OMack\nono-py`; HEAD `e4a56f894f3b9cf108b71dc243687218bccc2b47`
- nono-ts clone: `git clone https://github.com/always-further/nono-ts.git` at `C:\Users\OMack\nono-ts`; HEAD `3f0390e1b18cf2b21389722a925441c0d777524e`

Note: this repo is being executed from a Claude Code worktree at
`C:\Users\OMack\Nono\.claude\worktrees\agent-a1997e4c572ec30bd`. The plan's
`../nono-py/` and `../nono-ts/` references are relative to the **main** repo
at `C:\Users\OMack\Nono`, so absolute paths are used here to avoid
ambiguity. The siblings are positioned adjacent to the main repo.

## Decision

Option A auto-selected (per plan-44-02 Task 1 action step "If `$DERIVED_ORG ==
always-further` and both siblings exist and clone succeeds, auto-proceed to
Task 2"). No user input required.

## Sibling commit SHAs (populated after Tasks 4 + 5)

| Sibling | Branch | Commit SHA | Subject |
|---------|--------|------------|---------|
| nono-py | _pending_ | _pending_ | _pending_ |
| nono-ts | _pending_ | _pending_ | _pending_ |

## PR coordination (plan-discretion per D-44-D1)

To be determined at clone-time by inspecting each sibling repo's
CONTRIBUTING / README conventions in Tasks 4 + 5.

## REQ-TEST-HYG-02 Determinism Check

50-consecutive-runs result: **PARTIAL — deferred to live CI lane**.

Per Roadmap SC#3: "both flakes pass deterministically across 50
consecutive runs on a Windows host (or CI lane equivalent)".

Disposition rationale:
- Executor is running inside a Claude Code worktree on a Windows
  host (`win32` platform per env), but `cargo-nextest` is NOT
  installed on this host (`cargo nextest --version` returns
  "no such command: nextest").
- The plan's Part C explicitly authorises a PARTIAL deferral via
  cross-target-verify-checklist when nextest is unavailable from
  the dev host.
- The `.config/nextest.toml` file is correctly written, doc-comments
  cross-link source to config, and the filter syntax matches the
  Option A shape from D-44-D3 / 44-PATTERNS.md § ".config/nextest.toml"
  Option A. The first CI run that wires `cargo nextest run -p
  nono-cli --test env_vars --config-file .config/nextest.toml` into
  the Windows CI lane (per 44-PATTERNS.md "CI wire-up" snippet) will
  execute the determinism check.

Follow-up: the Phase 44 verifier (or a Phase 46/47 CI hardening
plan) should wire the nextest invocation into the Windows CI
workflow and capture the 50-runs result. Until then this disposition
stays PARTIAL.
