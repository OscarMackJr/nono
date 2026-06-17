# Phase 76: Self-Verifying Harness Foundation - Context

**Gathered:** 2026-06-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the **shared scripted-gate framework** (`scripts/verify-dark.ps1`) — single-invocation
unattended scripts that emit machine-readable PASS/FAIL/SKIP verdicts — so every subsequent
host-gated phase (77-80) can drop interactive human UAT in favor of a scripted run, and the
Phase 81 aggregator can evaluate milestone completion from harness output alone.

This phase delivers the **runner + per-gate contract + the `harness-self-check` gate only**.
It does NOT implement the feature gates themselves (copilot-e2e, wfp-egress-isolation,
clean-host-install, classify) — those ship in their owning phases as gate files that plug
into this framework.

**The Phase 76 gate IS:** `scripts/verify-dark.ps1 --gate harness-self-check` exits 0 with a
JSON `PASS` verdict on any Win11 host — proving the framework itself is wired before any
feature gate exists.

</domain>

<decisions>
## Implementation Decisions

### Verdict contract (D-01..D-03)
- **D-01:** Each gate emits a **typed JSON object**: `{gate, verdict, reason, detail, timestamp}`.
  `verdict` is one of `PASS` / `FAIL` / `SKIP_HOST_UNAVAILABLE`. `reason` is a short human string;
  `detail` is a free-form object for debugging context. A single `--gate` run prints one object.
- **D-02:** **Three-way exit-code mapping** for a single-gate run: `0 = PASS`, `2 = FAIL`,
  `3 = SKIP_HOST_UNAVAILABLE`. Reserve `1` / `4+` for harness-internal errors (never silently
  conflate a harness crash with a gate FAIL). Distinct FAIL vs SKIP codes let CI branch without
  parsing JSON — this is what satisfies criteria #4's "no ambiguity about why the skip occurred."
- **D-03:** The `--all` / aggregator run (built in Phase 81, but its consumption contract is fixed
  here) prints `{gates: [...], overall}`. **Aggregator exit mapping deviates from the per-gate
  mapping:** `PASS` and `PASS_WITH_SKIPS` both → exit 0; any `FAIL` → non-zero. (Per Phase 81
  criteria #2/#3 — a whole-suite host-skip must not fail CI.) Phase 76 only needs to make this
  shape *producible*; it does not implement the aggregator.

### Gate registration (D-04..D-05)
- **D-04:** **Auto-discovered gate files.** `verify-dark.ps1` is a thin runner; each gate is a
  self-contained file at `scripts/gates/<name>.ps1`. The runner globs `scripts/gates/*.ps1` to
  build the gate list for both `--gate <name>` and `--all`. Phases 77-80 **ADD a file, never edit
  the core runner** → no merge contention, no regression risk to other gates.
- **D-05:** Each gate file exports a fixed contract: a `Test-Precondition` function and an
  `Invoke-Gate` function (dot-sourced by the runner). The gate name = the file's base name, so
  `--gate copilot-e2e` resolves to `scripts/gates/copilot-e2e.ps1`. Unknown `--gate <name>` is a
  harness-internal error (exit 1+), not a FAIL.

### SKIP precondition detection (D-06..D-07)
- **D-06:** `Test-Precondition` returns **`$null` (preconditions met → run `Invoke-Gate`)** or a
  **string reason (host unavailable → emit `SKIP_HOST_UNAVAILABLE` with that reason)**. The runner
  calls `Test-Precondition` BEFORE `Invoke-Gate`; if it returns a reason, the runner emits the SKIP
  verdict (exit 3) and never enters the gate body.
- **D-07:** **SKIP vs FAIL semantics:** `SKIP_HOST_UNAVAILABLE` = expected host-absence
  (daemon not running, Copilot CLI not installed, not a clean host). `FAIL` = precondition was met
  but the actual assertion failed. An unexpected crash inside `Invoke-Gate` maps to a
  harness-internal error exit (never a silent PASS, never a swallowed exception).

### Verdict persistence + reuse (D-08..D-10)
- **D-08:** **Write file + re-run.** Every gate run writes its verdict JSON to
  `.nono-runtime/verdicts/<gate>.json` (gitignored — confirm/add the ignore rule). A partial
  `--gate` run leaves an inspectable artifact; `--all` re-runs every gate in sequence, each
  rewriting its file, then reads the dir to build the `{gates:[...], overall}` summary. One file
  per gate = single source of truth, no dedup/ordering fragility.
- **D-09:** **Net-new runner, gates may shell out.** `verify-dark.ps1` is a fresh, focused script
  whose job is the verdict contract + dispatch + persistence. A gate's `Invoke-Gate` is free to
  shell out to `cargo test` / `msiexec` / `nono.exe` and **map the child exit code to a verdict**.
  → Phase 78's `cargo test --bin nono-agentd -- classify` becomes a thin gate wrapper
  (`scripts/gates/classify-*.ps1`) that runs the cargo test and maps `$LASTEXITCODE` to PASS/FAIL.
- **D-10:** **Do NOT fold into `windows-test-harness.ps1`.** That script serves the CI
  build/smoke/regression suite (a different audience). Keep host-gated UAT-replacement separate;
  reusing its `Invoke-LoggedCargo` helper is optional and lower priority than clean separation.

### `harness-self-check` gate (D-11)
- **D-11:** The Phase 76 self-check gate (`scripts/gates/harness-self-check.ps1`) has a
  `Test-Precondition` that **always returns `$null`** (runs on any Win11 host — criteria #5) and an
  `Invoke-Gate` that trivially verifies the framework wiring (e.g. confirms it can emit a verdict,
  write its persistence file, and round-trip the JSON) and returns `PASS`. This gate is the proof
  that the framework functions before any feature gate exists.

### Claude's Discretion
- Exact field naming/casing within the verdict JSON beyond the agreed keys (`gate`, `verdict`,
  `reason`, `detail`, `timestamp`) — keep consistent and documented.
- The precise self-check assertions inside `harness-self-check`'s `Invoke-Gate` (so long as it
  exercises emit + persist + round-trip).
- Whether the runner accepts `--all` explicitly or treats "no `--gate`" as the all-run (Phase 81
  formalizes this; Phase 76 should not block it).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone scope & requirements
- `.planning/ROADMAP.md` § Phase 76 — Goal, 5 Success Criteria, host gate, unattended gate
  (`scripts/verify-dark.ps1 --gate harness-self-check`). Also read § Phases 77-81 — they are the
  consumers of this framework; their `**Unattended gate**:` lines define what gate files must exist
  and what each must produce.
- `.planning/REQUIREMENTS.md` — **DARK-01** (this phase: each host-gated verification ships as a
  single-invocation unattended script emitting machine-readable verdict) and **DARK-02** (Phase 81:
  milestone-close aggregator collects per-item verdicts).

### Existing patterns to study (PowerShell harness conventions)
- `scripts/windows-test-harness.ps1` — existing 200-line harness: `param([ValidateSet(...)])`
  pattern, `$ErrorActionPreference="Stop"`, `$PSNativeCommandUseErrorActionPreference=$false`
  (cargo writes progress to stderr), `Start-Process -RedirectStandardOutput/Error` capture +
  `$process.ExitCode` checking, `Invoke-LoggedCargo` helper. **Do NOT extend it (D-10)** — study it
  for conventions/idioms only.
- `scripts/validate-windows-msi-contract.ps1`, `scripts/test-windows-shell-write-deny.ps1` —
  other host-gated PowerShell verification scripts for style reference.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `scripts/windows-test-harness.ps1::Invoke-LoggedCargo` — child-process spawn + stdout/stderr
  capture + exit-code check pattern that a `cargo`-wrapping gate's `Invoke-Gate` (D-09) can mirror.
  Reuse the *idiom*, not the file.
- `.nono-runtime/` directory already exists in the working tree (untracked) — the natural home for
  `.nono-runtime/verdicts/<gate>.json` (D-08). Planner must confirm `.nono-runtime/` is in
  `.gitignore` (it appears untracked in git status, so likely already ignored — verify).

### Established Patterns
- PowerShell scripts use `param(...)` with `[ValidateSet(...)]` for mode selection — `--gate`
  selection should follow this idiom (a `[string]$Gate` param; validation against the discovered
  gate list rather than a hardcoded `ValidateSet`, since gates are auto-discovered per D-04).
- `$ErrorActionPreference = "Stop"` + `$PSNativeCommandUseErrorActionPreference = $false` is the
  established header for scripts that shell out to native tools — apply it so a gate shelling to
  cargo/msiexec doesn't promote benign stderr into a terminating error.

### Integration Points
- **Phase 81** consumes `.nono-runtime/verdicts/*.json` + the `{gates:[...], overall}` summary
  shape → these are the cross-phase contracts this phase MUST fix.
- **Phases 77, 79, 80** each add one `scripts/gates/<name>.ps1` file conforming to the
  `Test-Precondition` + `Invoke-Gate` contract.
- **Phase 78** adds a gate file wrapping `cargo test --bin nono-agentd -- classify` (D-09).

</code_context>

<specifics>
## Specific Ideas

- Verdict JSON example agreed during discussion:
  ```json
  { "gate": "harness-self-check", "verdict": "PASS", "reason": "framework functional",
    "detail": {}, "timestamp": "2026-06-16T..." }
  ```
- Directory layout agreed:
  ```
  scripts/
    verify-dark.ps1            # runner: glob gates, dispatch, emit, persist
    gates/
      harness-self-check.ps1   # Phase 76 (this phase — only gate built now)
      copilot-e2e.ps1          # Phase 77
      classify-cross-process.ps1 # Phase 78 (wraps cargo test)
      wfp-egress-isolation.ps1 # Phase 79
      clean-host-install.ps1   # Phase 80
  .nono-runtime/verdicts/<gate>.json
  ```
- Gate file contract sketch:
  ```powershell
  function Test-Precondition { # -> $null (run) | "reason string" (SKIP)
    if (-not (Get-Service nono-agentd -EA SilentlyContinue)) { return 'nono-agentd not running' }
    return $null
  }
  function Invoke-Gate { # -> verdict object (PASS/FAIL); may shell out
    cargo test --bin nono-agentd -- classify
    if ($LASTEXITCODE -eq 0) { <PASS> } else { <FAIL> }
  }
  ```

</specifics>

<deferred>
## Deferred Ideas

- **Feature gate bodies** (copilot-e2e, wfp-egress-isolation, clean-host-install, classify) — each
  ships in its owning phase (77-80), not here. Phase 76 builds only the framework + self-check gate.
- **The `--all` aggregator + `overall`/`PASS_WITH_SKIPS` rollup logic** — formalized in Phase 81
  (DARK-02). Phase 76 fixes the *consumption contract* (the `{gates:[...], overall}` shape and the
  aggregator exit mapping in D-03) but does not implement the rollup.
- **Reusing `Invoke-LoggedCargo` log-tee'ing inside gates** — optional ergonomics, can be added by a
  gate author later; not a Phase 76 deliverable.

None of the above are scope creep into Phase 76 — they are correctly owned by later phases.

</deferred>

---

*Phase: 76-self-verifying-harness-foundation*
*Context gathered: 2026-06-16*
