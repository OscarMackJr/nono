---
phase: 95-upstream-absorb-fork-invariant-verify
reviewed: 2026-06-26T13:16:40Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - crates/nono/src/sandbox/linux.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/config.rs
findings:
  critical: 1
  warning: 3
  info: 3
  total: 7
status: issues_found
---

# Phase 95: Code Review Report (gap-closure pass)

**Reviewed:** 2026-06-26T13:16:40Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

> Scope: gap-closure diffs since `1891ac1d` (commits `8bca078b` WR-02, `5d1e9077` WR-03,
> `2a8b639e` WR-01, `c81429aa` CR-01). This supersedes the earlier broad 95-REVIEW pass
> for these four files.

## Summary

Three of the four restored fork-invariants are correct and well-constructed:

- **WR-02** (`linux.rs` `read_msghdr_dest`): `offset_of!`/`size_of`-derived field layout with
  compile-time overlap assertions is sound on LP64 and ILP32. Buffer bounds are statically
  guaranteed by `MSGHDR_MIN_READ = MSG_NAME_LEN_OFFSET + 4` plus the
  `MSG_NAME_OFFSET + PTR_SIZE <= MSG_NAME_LEN_OFFSET` assertion. Correct.
- **WR-01** (`exec_strategy.rs`): the two post-fork-child `format!()` calls are correctly
  replaced with `const &[u8]` static byte strings written via async-signal-safe `libc::write` +
  `_exit(126)`. Change is correctly scoped to the child arm (lines 1411, 1453); the surrounding
  `format!()` at lines 1599-1668 are in the PARENT arm and are fine. A sentinel-bounded
  regression test (`resl_nix_async_signal_safety.rs`) guards the region.
- **WR-03** (`linux.rs` GPU enforcement): `collect_linux_gpu_paths` mirrors upstream
  `maybe_enable_gpu`, is gated behind `caps.gpu()` (`--allow-gpu` opt-in), skips absent devices,
  scopes NVIDIA procfs grants to `nvidia_present`, and adds `IoctlDev` only for device paths.
  `is_nvidia_compute_device` correctly excludes `nvidia-modeset` and non-numeric suffixes. Correct.

The CR-01 wiring (`reverse.rs`) introduces a **fail-open security defect**: `evaluate()` is a
three-way decision (`Deny`/`Allow`/`Approve`) but the handler only acts on `Deny`. An `Approve`
outcome — operator-configurable via `approve:` rules or `default: approve` — falls through and
the request is forwarded to the upstream with the real credential injected. See CR-01.

## Critical Issues

### CR-01: endpoint_policy `Approve` outcome is silently treated as allow (fail-open)

**File:** `crates/nono-proxy/src/reverse.rs:125-151`
**Issue:**
`CompiledEndpointPolicy::evaluate()` returns one of three variants — `Deny`, `Allow`, `Approve`
(`config.rs:295-309`, `419-455`). The new enforcement block only matches `Deny`:

```rust
if let EndpointPolicyOutcome::Deny { reason, rule_label } =
    route.endpoint_policy.evaluate(&method, &upstream_path)
{ ... send_error(stream, 403, ...); return Ok(()); }
```

Any non-`Deny` outcome falls through and the request proceeds to credential injection and
upstream forwarding. `Approve` is a first-class, operator-configurable decision:
`EndpointPolicyDecision::Approve` (`config.rs:209`), per-route `approve:` rule lists
(`config.rs:257`), and `default: approve` (`config.rs:448-453`), each carrying
`backend`/`timeout_secs` metadata signalling "route to an approval backend / hold for approval."
Treating it as a silent allow forwards the request with the real upstream credential, emits no
`audit::log_denied`, and returns no 403/hold. `evaluate()` is consumed in exactly one place
(confirmed by grep across the crate) — nothing else handles `Approve`.

This violates **Fail Secure** ("On any error, deny access. Never silently degrade to a less
secure state.") and **Explicit Over Implicit** from CLAUDE.md. The comment at `reverse.rs:119`
("evaluate explicit deny/approve/allow rules") reinforces the false impression that approve is
handled.

**Fix:** Match the full enum so the compiler forces handling of every (and future) variant.
Until an approval backend exists, `Approve` must fail closed (deny + audit), not fall through:

```rust
match route.endpoint_policy.evaluate(&method, &upstream_path) {
    EndpointPolicyOutcome::Allow { .. } => { /* proceed */ }
    EndpointPolicyOutcome::Deny { reason, rule_label } => {
        // existing 403 + audit::log_denied path
        send_error(stream, 403, "Forbidden").await?;
        return Ok(());
    }
    EndpointPolicyOutcome::Approve { rule_label, .. } => {
        let deny_reason = format!(
            "endpoint_policy requires approval (not implemented): {} {} on '{}' (rule: {})",
            method, upstream_path, service, rule_label,
        );
        warn!("{}", deny_reason);
        audit::log_denied(
            ctx.audit_log,
            audit::ProxyMode::Reverse,
            &audit::EventContext {
                route_id: Some(&service),
                denial_category: Some(nono::undo::NetworkAuditDenialCategory::EndpointPolicy),
                ..Default::default()
            },
            &service, 0, &deny_reason,
        );
        send_error(stream, 403, "Forbidden").await?;
        return Ok(());
    }
}
```

Add a test asserting a route with an `approve` rule (or `default: approve`) does NOT forward.

## Warnings

### WR-01: CR-01 regression test only greps `format!(` — misses other heap allocations

**File:** `crates/nono-cli/tests/resl_nix_async_signal_safety.rs:156-186`
**Issue:**
`cr_01_no_format_macro_in_post_fork_child_branch` enforces async-signal-safety of the post-fork
child arm via `stripped.matches("format!(").count()`. It will not catch `.to_string()`,
`String::new()`/`String::from`, `vec!`, `Vec::with_capacity`, `.collect::<Vec<_>>()`,
`.to_owned()`, or `format_args!`. A future `let m = err.to_string();` inside the child arm would
pass silently while reintroducing the allocation-deadlock class WR-01 closes. The production fix
is correct; the guard is weaker than its stated mandate ("any heap-allocating call ... is
forbidden", `exec_strategy.rs:1026-1029`).
**Fix:** Extend the forbidden-token set, e.g. assert each of
`["format!(", "to_string()", "String::", "vec!", "Vec::", ".collect(", "to_owned()", "format_args!("]`
has count 0 within the sentinel region; keep the loud failure message.

### WR-02: guard test comment-stripper corrupts lines with `//` inside string literals

**File:** `crates/nono-cli/tests/resl_nix_async_signal_safety.rs:163-170`
**Issue:**
The stripper truncates each line at the first `//`:
```rust
.map(|line| match line.find("//") {
    Some(idx) => &line[..idx],
    None => line,
})
```
A line with `//` inside a byte-string literal (e.g. a future `b"see https://...\n"`) is
truncated mid-literal. A `format!(` after such a `//` on the same line would be a false negative.
Robustness defect in the test parser, not the production code, but it weakens the WR-01 guard.
**Fix:** Strip only lines whose first non-whitespace chars are `//`, or use a minimal
string-literal-aware lexer; at minimum document the limitation.

### WR-03: `evaluate()` duplicates `is_allowed()` on every legacy-route request

**File:** `crates/nono-proxy/src/reverse.rs:97-151`
**Issue:**
For legacy routes (no explicit `endpoint_policy`), `route.endpoint_policy` is compiled FROM the
same `endpoint_rules` (`route.rs:91-92`, `config.rs:361-390`). Each request is L7-checked twice:
`endpoint_rules.is_allowed()` (line 97) then `endpoint_policy.evaluate()` (line 126), recompiling
glob matches and `format!`-building two `rule_label` strings on the hot path. Method/glob matching
is verified identical between the two (`config.rs:330-339` vs `480-483`), so this is not a
correctness bug — but it is dead work and obscures which check is authoritative.
**Fix:** After CR-01 is fixed, make `endpoint_policy.evaluate()` the single authoritative L7 gate
and remove the subsumed `is_allowed()` block (legacy rules already compile into the policy as
`allow` entries with a deny-default). Confirm via existing legacy tests in `config.rs`.

## Info

### IN-01: redundant `#[cfg(target_os = "linux")]` inside an already-Linux-only module

**File:** `crates/nono/src/sandbox/linux.rs:430, 479, 906`
**Issue:**
`linux.rs` is gated `#[cfg(target_os = "linux")] mod linux;` (`sandbox/mod.rs:13-14`), so the
file only compiles on Linux. The added `#[cfg(target_os = "linux")]` on
`is_nvidia_compute_device`, `collect_linux_gpu_paths`, and the `if caps.gpu()` block in
`apply_with_abi` are no-ops. Harmless, but they imply `apply_with_abi` has non-Linux branches and
could mislead a future reader.
**Fix:** Drop the redundant attributes or add a one-line "belt-and-suspenders" note.

### IN-02: `try_into()` error arm in `read_msghdr_dest` is unreachable

**File:** `crates/nono/src/sandbox/linux.rs:2686-2692`
**Issue:**
`buf[MSG_NAME_LEN_OFFSET..MSG_NAME_LEN_OFFSET + 4].try_into()` into `[u8; 4]` slices a fixed
4-byte range from a stack buffer of static size `MSG_NAME_LEN_OFFSET + 4`; the conversion can
never fail, so the `.map_err(...)` arm allocating a `NonoError::SandboxInit(String)` is dead.
Acceptable defensive code, but genuinely unreachable.
**Fix:** Optional — leave as defensive, or use an infallible slice-to-array copy. No action required.

### IN-03: `reverse.rs:119` comment overstates current behavior

**File:** `crates/nono-proxy/src/reverse.rs:119`
**Issue:**
"Structured L7 endpoint policy: evaluate explicit deny/approve/allow rules" claims all three
decisions are evaluated, but only `Deny` is enforced (see CR-01).
**Fix:** Update the comment to match actual behavior (it becomes accurate once CR-01 is fixed).

---

_Reviewed: 2026-06-26T13:16:40Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
