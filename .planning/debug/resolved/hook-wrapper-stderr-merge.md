---
slug: hook-wrapper-stderr-merge
status: resolved
trigger: "Security review R-A1: nono-tool-hook.ps1 runs `$output = $inputJson | & $nono claude-code-hook 2>&1`, merging the nono process's stderr into stdout. On exit 0 it echoes $output verbatim as the hook's JSON contract. When NONO_LOG/RUST_LOG is set, nono emits tracing to stderr; the 2>&1 merge prepends those (ANSI-colored) lines to the JSON, so Claude Code cannot parse the permissionDecision. Verified live: with NONO_LOG=debug a DEBUG 'theme: mocha' line is prepended and ConvertFrom-Json FAILS."
created: 2026-06-13
updated: 2026-06-13
---

# Debug: hook wrapper merges stderr into the JSON contract (R-A1)

## Symptoms

- **Expected:** the PreToolUse hook's stdout is ONLY the JSON decision object; tracing/log output (stderr) must never corrupt it. A malformed decision must fail CLOSED (deny), never silently fail open.
- **Actual:** `crates/nono-cli/data/hooks/nono-tool-hook.ps1` line 7 uses `2>&1`, merging stderr into the captured `$output`. On `exit 0`, `$output` (stderr log lines + JSON) is echoed to stdout. Any `tracing` output (enabled by `NONO_LOG`/`RUST_LOG`, an env-influenceable state) corrupts the JSON.
- **Error/impact:** Claude Code receives non-JSON, cannot read `permissionDecision`. Depending on Claude Code's contract this is either fail-open (no decision honored) or a fail-closed loop — the security boundary is decided by a parsing accident. Violates the fail-closed design constraint.
- **Timeline:** present since the hook wrapper was introduced (Phase 60). Surfaced by the Phase-1 Windows confinement security review (R-A1, HIGH).
- **Reproduction (verified live this session):** `$env:NONO_LOG="debug"; $out = $json | & nono claude-code-hook 2>&1` → captured output begins with `2026-...Z DEBUG theme: mocha` (ANSI) then the JSON; `($out -join "`n") | ConvertFrom-Json` → PARSE FAILED.

## Current Focus

- **hypothesis:** Confirmed — the `2>&1` merge conflates the stderr log stream with the stdout JSON contract. Separating the streams (stdout = JSON only; stderr captured separately and surfaced only on failure) fixes it.
- **test:** With `NONO_LOG=debug` set and a valid PreToolUse event, the wrapper's stdout must be pure, parseable JSON (ConvertFrom-Json succeeds) and carry the correct `permissionDecision`. On a non-zero nono exit, the wrapper must emit a fail-closed `deny` JSON whose reason includes the captured stderr.
- **expecting:** Pure-JSON stdout on success regardless of NONO_LOG; fail-closed deny on any error.
- **next_action:** Confirm root cause; rewrite the wrapper to separate stdout/stderr; verify with NONO_LOG set; ensure fail-closed paths still work.

### reasoning_checkpoint
- hypothesis: "`2>&1` on line 7 merges nono's stderr (tracing logs, env-gated by NONO_LOG/RUST_LOG) into the captured stdout that is echoed verbatim as the JSON contract on exit 0, corrupting the JSON. Separating the streams fixes it."
- confirming_evidence:
  - "Live repro: NONO_LOG=debug prepended a `DEBUG theme: mocha` line to the JSON; ConvertFrom-Json FAILED. No-NONO_LOG path was clean."
  - "Direct read of line 7 confirms `... | & $nono claude-code-hook 2>&1` and line 9 echoes `$output` on exit 0."
- falsification_test: "After redirecting stderr away from stdout, run with NONO_LOG=debug and a valid event — if stdout STILL contains a DEBUG line / fails to parse, the hypothesis (stream merge is the cause) is wrong."
- fix_rationale: "Redirect child stderr to a temp file; capture child stdout cleanly into a variable. On exit 0 emit ONLY captured stdout (the JSON), never stderr. This removes the only channel by which log output reaches stdout — addresses root cause, not a symptom. Fail-closed deny on non-zero/exception surfaces the captured stderr in the reason field for diagnosis only (stdout stays pure)."
- blind_spots: "PowerShell may emit its own error records to the success-pipeline if the child writes to PS error stream rather than native stderr; mitigated by redirecting native stderr to a file and capturing native stdout only. Encoding (UTF-8 vs UTF-16) of the temp file read is a minor risk; using Get-Content -Raw."

## ROOT CAUSE (already established — live-proven, not a hunt)

Confirmed live this session. The fix is a contained rewrite of `nono-tool-hook.ps1`:
- Do NOT use `2>&1` on the success path. Capture stdout (the JSON) separately from stderr (tracing/logs).
- On `exit 0`: emit ONLY the captured stdout (the JSON contract). Discard/ignore stderr (it is diagnostic only).
- On non-zero exit OR exception: emit a fail-closed `deny` JSON object; include the captured stderr text in `permissionDecisionReason` for diagnosis.
- Keep `$ErrorActionPreference = "Stop"`, the `NONO_EXE` override, and `ConvertTo-Json -Depth 4 -Compress`.
- Clean up any temp file used to capture stderr (`finally`).

Constraints:
- Must remain fail-CLOSED: any failure to obtain a clean exit-0 JSON → deny.
- Must not regress the existing behavior where a clean run returns the handler's JSON unchanged.
- If a hook-script validation/contract test exists (search `tests/` and `scripts/` for the hook script), update/extend it to assert: (a) clean JSON stdout with NONO_LOG=debug, (b) fail-closed deny on non-zero exit.

## Evidence

- timestamp: 2026-06-13 — `nono-tool-hook.ps1:7` `$output = $inputJson | & $nono claude-code-hook 2>&1`; line 9 echoes `$output` on exit 0. Confirmed by direct read.
- timestamp: 2026-06-13 — LIVE: `NONO_LOG=debug` run produced `... DEBUG theme: mocha` (ANSI) prepended to the JSON; `ConvertFrom-Json` failed. The no-NONO_LOG path was clean (so the bug is logging-state-dependent).

## Eliminated

(none — root cause confirmed on first observation)

## Resolution

root_cause: |
  `nono-tool-hook.ps1` line 7 used `... | & $nono claude-code-hook 2>&1`, merging the
  nono process's stderr (tracing/log output, env-gated by NONO_LOG/RUST_LOG) into the
  stdout captured in `$output`, which is echoed verbatim as the hook's JSON contract on
  exit 0. With logging enabled, log lines prepend the JSON so Claude Code's
  ConvertFrom-Json fails — the security decision becomes a parsing accident (R-A1, HIGH).

fix: |
  Contained rewrite of nono-tool-hook.ps1 (both the embedded source copy and the package
  copy):
  - Removed `2>&1`. Native stderr is now redirected to a per-invocation temp file
    (`2>$stderrFile`); $stdout captures ONLY the child's stdout.
  - exit 0 -> emit ONLY captured stdout JSON; stderr (diagnostic) is discarded.
  - non-zero exit OR exception -> fail-CLOSED `deny` JSON; captured stderr is included in
    permissionDecisionReason for diagnosis (stdout contract stays clean).
  - Temp file cleaned up in a `finally` block.
  - Preserved `$ErrorActionPreference = "Stop"`, NONO_EXE override, ConvertTo-Json -Depth 4 -Compress.
  - Robustness footgun fixed: under `$ErrorActionPreference = "Stop"`, a native command
    writing ANYTHING to stderr raises a terminating NativeCommandError even on exit 0.
    EAP is relaxed to "Continue" ONLY around the native call, then restored; fail-closed
    is preserved via explicit `$LASTEXITCODE` inspection.
  - build.rs: added explicit per-file `cargo:rerun-if-changed` directives for every
    embedded data file (the bare `rerun-if-changed=data/` only watched the directory
    entry's mtime, not nested files — it would have kept embedding the STALE vulnerable
    hook). Without this, the embedded artifact does not refresh on hook edits.
  - hooks.rs: added regression test `test_embedded_tool_hook_separates_stderr_from_json_contract`
    asserting the embedded script never merges stderr into stdout, redirects to a temp
    file, and cleans up in finally.

verification: |
  Self-verified live on this Windows host (Win11):
  - T1 (success path): stub emits DEBUG to stderr + valid JSON to stdout, exit 0 ->
    hook stdout is pure parseable JSON, permissionDecision=allow, NO DEBUG leakage. PASS.
  - T2 (fail-closed): stub writes to stderr, exit 1 -> clean parseable deny JSON with
    captured stderr surfaced in permissionDecisionReason. PASS.
  - T3 (real binary): target\debug\nono.exe, NONO_LOG=debug, valid PreToolUse Read event
    -> pure parseable JSON, permissionDecision=allow, no DEBUG leakage. PASS.
  - Incidental: missing/invalid NONO_EXE -> clean parseable fail-closed deny. PASS.
  - Rust contract tests: test_embedded_tool_hook_fails_closed + new
    test_embedded_tool_hook_separates_stderr_from_json_contract both green
    (`cargo test -p nono-cli --bin nono hooks::tests::test_embedded_tool_hook`).
  All tests run faithfully as a real child process with redirected stdin (mirrors how
  Claude Code invokes the hook). Fix is staged-but-uncommitted per instructions.

files_changed:
  - crates/nono-cli/data/hooks/nono-tool-hook.ps1
  - packages/claude-code/hooks/nono-tool-hook.ps1
  - crates/nono-cli/build.rs
  - crates/nono-cli/src/hooks.rs
