---
phase: 78-cross-process-classification
reviewed: 2026-06-17T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - crates/nono-cli/src/agent_daemon/control_loop.rs
  - crates/nono-cli/src/agent_cli.rs
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/tests/daemon_handle_baseline.rs
findings:
  critical: 0
  warning: 4
  info: 3
  total: 7
status: issues_found
---

# Phase 78: Code Review Report

**Reviewed:** 2026-06-17
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Phase 78 adds a cross-process `Classify` verb to the daemon control pipe (`handle_classify`,
`classify_response_string`, `ControlRequest::Classify`), a daemon-first `nono classify` CLI
path (`classify_daemon_request`, `run_classify` routing), and a live integration test
(`classify_pid_returns_verdict_from_daemon`).

The core SC4 invariant (verdict-only response, no SID disclosure) is well-implemented and
deterministically tested: `classify_response_string` is a pure function that destructures the
`AiAgent { .. }` variant with a wildcard so no SID data can reach the wire, and there is a
dedicated unit test asserting the response contains neither the SID value nor the substring
"sid". The unwrap/expect prohibition is respected in non-test code (poison-tolerant
`unwrap_or_else(|p| p.into_inner())` is used on lock acquisition; the integration test and
`#[cfg(test)]` modules carry `#[allow(clippy::unwrap_used)]`). All FFI blocks in the new
integration helper (`daemon_control_pipe_request`) carry `// SAFETY:` docs. The lock-order
discipline (acquire `agent_registry` only, drop before formatting) is correct and cannot
deadlock against the documented registry→tenants order. The registry is populated at launch
(`launch.rs:745`), so the SC1 self-recognition path is genuinely wired.

The defects below concentrate in the CLI-side verdict routing and string-based error
classification, where the daemon-first path can silently degrade to the non-authoritative
structural fallback (or report an authoritative-looking `NotAnAgent`) on responses it should
treat as errors. None rise to BLOCKER because they fail toward a *less* capable verdict
("NotAnAgent" / "not authoritative") rather than a *false positive* "AiAgent", but they
undermine the authoritativeness contract the verb advertises.

## Warnings

### WR-01: `classify_daemon_request` maps any non-`"AiAgent"` daemon response to authoritative `NotAnAgent`, including error frames

**File:** `crates/nono-cli/src/agent_cli.rs:763-785`
**Issue:** The daemon may return a framed *error* string for the `classify` action — e.g. a
future malformed-request path, a registry-poison message, or any string the control loop emits
on an unexpected condition. `classify_daemon_request` matches only the exact literal `"AiAgent"`
and routes everything else through `_ => "not_an_agent"`. An error frame such as
`"error: malformed request: ..."` is therefore reported to the operator (and to tooling, via
`--json` with `"authoritative": true`) as a definitive `NotAnAgent` verdict. For a
classification verb whose entire value proposition is an *authoritative* answer, silently
laundering an error into a confident "not an agent" is a correctness defect: an operator
triaging a suspicious PID could be told it is safe when the daemon actually failed to answer.
**Fix:** Treat only the two known verdict literals as valid; surface anything else as an error
so the caller does not present a fabricated verdict.
```rust
let verdict_key = match verdict_raw {
    "AiAgent" => "ai_agent",
    "NotAnAgent" => "not_an_agent",
    other => {
        return Err(NonoError::SandboxInit(format!(
            "nono classify: daemon returned an unrecognized verdict frame: {other:?}"
        )));
    }
};
```

### WR-02: Daemon-absent fallback routing keys on a fragile substring (`"daemon-absent"`) and an over-broad `is_pipe_not_found`

**File:** `crates/nono-cli/src/app_runtime.rs:211` and `crates/nono-cli/src/agent_cli.rs:1058-1062`
**Issue:** `run_classify` decides whether to fall back to the non-authoritative structural path
with `is_pipe_not_found(&e) || e.to_string().contains("daemon-absent")`. Two problems compound:
(1) `is_pipe_not_found` returns `true` for any error message containing the bare substring
`"not available"` (line 1061) — `windows_control_pipe_request` emits
`"...pipe not available"` on *every* `CreateFileW` failure, including `GLE=5`
(ERROR_ACCESS_DENIED, which is exactly the SC3 "Low-IL caller denied" case) and `GLE=231`
(ERROR_PIPE_BUSY). A genuine access-denied or busy-pipe condition is thus misclassified as
"daemon not running" and silently downgraded to the empty-registry structural path, which
always answers `NotAnAgent`. (2) The `"daemon-absent"` marker is a free-text substring of an
error message, not a typed error; any future error string containing those characters would
also trigger the fallback. The net effect is the same fail-open as WR-01: a real failure to
reach the authoritative daemon is presented as a structural "NotAnAgent".
**Fix:** Distinguish "pipe does not exist" (GLE=2 / ERROR_FILE_NOT_FOUND) from other
`CreateFileW` failures, and stop matching the generic `"not available"` substring. Prefer a
typed signal (e.g. a dedicated `NonoError` variant or returning the GLE in structured form) over
substring sniffing.
```rust
pub(crate) fn is_pipe_not_found(err: &nono::NonoError) -> bool {
    let msg = err.to_string();
    // Only ERROR_FILE_NOT_FOUND (GLE=2) means "daemon not running".
    // Do NOT match the generic "pipe not available" tail — it also fires for
    // GLE=5 (ACCESS_DENIED / SC3 Low-IL deny) and GLE=231 (PIPE_BUSY).
    msg.contains("GLE=2):") || msg.contains("GLE=2,")
}
```

### WR-03: Response length is read but the actual read count is trusted without verifying it matches across the two CLI read sites

**File:** `crates/nono-cli/src/agent_cli.rs:1029-1045`
**Issue:** `windows_control_pipe_request` allocates `resp_buf = vec![0u8; resp_len]` from the
attacker/daemon-supplied length prefix, then reads with a *single* `ReadFile` and requires
`bytes_read2 == resp_len`. On a byte-mode pipe a single `ReadFile` can legitimately return fewer
bytes than requested when the writer flushes in segments; the function then fails the whole
request even though more data is pending. This is a robustness defect: a large (but legitimate)
response near `MAX_RESPONSE` that arrives in two pipe segments is rejected as
"ReadFile response payload failed". The daemon side (`write_framed_response`) uses `write_all`,
so today's payloads are small and this rarely bites, but the contract is fragile and any future
larger verdict payload (or a slow pipe) will intermittently fail. Note the integration-test
helper `daemon_control_pipe_request` has the identical single-read assumption (lines 1599-1610).
**Fix:** Loop `ReadFile` until `resp_len` bytes have been accumulated (or EOF), as is standard
for framed byte-pipe reads, rather than asserting a single read returns the full frame.

### WR-04: `handle_launch` panics-equivalent path avoided, but `tenants.lock().unwrap_or_else(|p| p.into_inner())` then `getrandom`/`USERPROFILE` fallbacks remain; classify path has no such issue — flag the launch token fallback reused by the classify integration test

**File:** `crates/nono-cli/src/agent_daemon/control_loop.rs:544-545`
**Issue:** The Phase-78 integration test (`classify_pid_returns_verdict_from_daemon`) exercises
`handle_launch`, which derives the per-tenant workspace from
`std::env::var("USERPROFILE").unwrap_or_else(|_| std::env::temp_dir()...)`. If `USERPROFILE` is
unset, agents silently land under the system temp dir, where a workspace path can collide with
or be readable by other local principals — a tenant-isolation weakening that the verdict-only
classify response cannot detect. While this line predates Phase 78, the new Classify SC1 path
depends on `handle_launch` producing a correctly-isolated tenant, so it is in-scope for this
verb's security story. Fail closed instead of degrading to a shared temp root.
**Fix:** Treat a missing/empty `USERPROFILE` as a fatal launch error
(`return format!("error: USERPROFILE not set — refusing to place tenant workspace in shared temp")`)
rather than silently relocating per-tenant workspaces into a world-adjacent temp directory.

## Info

### IN-01: Integration test resolves agent PID by line-prefix scan that is brittle to response-format drift

**File:** `crates/nono-cli/tests/daemon_handle_baseline.rs:1652-1672`
**Issue:** The SC1 test parses the agent PID by scanning `launch_response` for a line whose
trimmed start is `"pid="`. The launch response format (`handle_launch`, control_loop.rs:622-625)
is a free-text multi-line string; a future formatting tweak (indentation, field reorder, or
adding a `pid=` substring elsewhere) silently breaks the parse and the test panics with a
confusing message rather than a clear contract failure. Consider having the daemon emit a
structured (JSON) launch response that both the CLI and the test parse, removing the
human-format dependency.
**Fix:** Parse a structured launch response, or assert the exact expected line shape before
extracting, so format drift fails loudly and unambiguously.

### IN-02: PID reuse window between launch and classify is unguarded (inherent TOCTOU, document it)

**File:** `crates/nono-cli/tests/daemon_handle_baseline.rs:1685-1717` and `control_loop.rs:859-869`
**Issue:** Classification is by raw PID. Between the operator reading a PID and the daemon
calling `registry.classify(pid)`, the original process can exit and the PID be recycled by the
OS to an unrelated process. `AgentRegistry::classify` reads the *current* token of whatever
process now holds that PID, so a recycled PID that happens to be a different daemon-minted
AppContainer could classify as `AiAgent`, or a recycled non-agent PID as `NotAnAgent`. This is
inherent to PID-based identity and not introduced by Phase 78, but the verb advertises an
"authoritative" answer; the authoritative claim should be qualified with this TOCTOU caveat in
the `nono classify` help text and the `handle_classify` doc comment.
**Fix:** Add a doc/help note that PID-based classification is subject to PID-reuse races, and
(future) consider a process start-time or handle-based identity to close the window.

### IN-03: `MAX_RESPONSE` / `MAX_CONTROL_FRAME` magic constants duplicated across CLI, daemon, and test

**File:** `crates/nono-cli/src/agent_cli.rs:903`, `control_loop.rs:74`, `daemon_handle_baseline.rs:1500`
**Issue:** The 64 KiB frame cap is hard-coded independently in three places
(`MAX_RESPONSE = 64 * 1024`, `MAX_CONTROL_FRAME = 64 * 1024`, and the test's
`MAX_RESPONSE = 64 * 1024`). Drift between the daemon's write cap and a client's read cap would
produce confusing truncation/rejection. Define one shared constant (e.g. in the daemon module
exported as `pub(crate)`) and reference it from all sites.
**Fix:** Extract a single `CONTROL_FRAME_MAX_BYTES` constant and import it everywhere the 64 KiB
bound is needed.

---

_Reviewed: 2026-06-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
