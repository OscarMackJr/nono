---
phase: 55-upst7-cherry-pick-wave
reviewed: 2026-06-04T00:00:00Z
depth: standard
files_reviewed: 26
files_reviewed_list:
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/data/policy.json
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/learn.rs
  - crates/nono-cli/src/learn_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/pack_update_hint.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/profile/builtin.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile_cmd.rs
  - crates/nono-cli/src/profile_save_runtime.rs
  - crates/nono-cli/src/pty_proxy.rs
  - crates/nono-cli/src/session_commands.rs
  - crates/nono-cli/src/startup_runtime.rs
  - crates/nono-cli/src/timeouts.rs
  - crates/nono-proxy/src/connect.rs
  - crates/nono/Cargo.toml
  - crates/nono/src/diagnostic.rs
  - crates/nono/src/scrub.rs
findings:
  critical: 1
  warning: 4
  info: 3
  total: 8
status: issues_found
---

# Phase 55: Code Review Report

**Reviewed:** 2026-06-04
**Depth:** standard
**Files Reviewed:** 26
**Status:** issues_found

## Summary

This phase is a UPST7 cherry-pick wave touching profile loading (JSONC + `binary`
field), diagnostic footer polish, pack-update-hint robustness, centralized
timeout constants with overflow clamping, the proxy 502 failure path, and a
sigstore 0.7→0.8 dependency bump.

The proxy 502 hardening (`connect.rs`), the timeout-overflow clamp
(`timeouts.rs`), and the scrub.rs Cow-deref port are clean and well-tested. The
sigstore bump is confined to `verify`/`sign`/`trust-root` constructor usage as
documented and does not change the verification trust surface.

The one BLOCKER is a **security regression introduced by the JSONC change**: the
fail-closed dual-key guard for `bypass_protection` / `override_deny` still uses a
strict `serde_json` parser, while the main profile parser is now JSONC-tolerant.
A JSONC profile containing a comment or trailing comma can therefore slip both
keys past the guard, re-opening the exact non-deterministic deny-list fail-open
the WR-01 check was built to prevent. This must be fixed before ship.

## Critical Issues

### CR-01: JSONC profiles bypass the fail-closed `bypass_protection`/`override_deny` dual-key guard

**File:** `crates/nono-cli/src/profile/mod.rs:87-96` (used at `2593` and `2639`)
**Issue:**
The phase switched the final profile deserialize from `serde_json::from_str` to
`jsonc_parser::parse_to_serde_value` (comments + trailing commas allowed) in both
`parse_profile_bytes` (line 2608) and `parse_profile_file` (line 2654). The
security pre-check `raw_profile_has_both_bypass_and_override_keys` was NOT
updated — it still parses the raw text with strict `serde_json::from_str` and, on
any parse error, returns `false`:

```rust
fn raw_profile_has_both_bypass_and_override_keys(raw: &str) -> bool {
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return false, // main parser will surface the real error
    };
    ...
}
```

The "main parser will surface the real error" assumption is now false: the main
parser accepts JSONC. So a user profile such as:

```jsonc
{
  "meta": { "name": "evil" },
  "policy": {
    // this comment makes serde_json::from_str fail
    "bypass_protection": ["/etc/shadow"],
    "override_deny": ["/some/other/path"],
  }
}
```

fails the strict pre-check (returns `false`, no error), then parses cleanly via
JSONC. Serde's `#[serde(alias = "override_deny")]` on `bypass_protection` then
silently keeps one list and drops the other in JSON-key-order-dependent fashion.
This is precisely the security-relevant fail-open (a deny-rule relaxation list
applied non-deterministically) that the WR-01 guard was designed to refuse. The
same gap applies to `detect_legacy_override_deny_key` /
`raw_profile_has_legacy_override_deny_key` (line 48-54), though that one only
suppresses a deprecation warning (lower impact).

**Fix:** Make the pre-check use the same JSONC-tolerant parse as the main path,
so the strict/lenient parsers can never disagree on which keys are present:

```rust
fn raw_profile_has_both_bypass_and_override_keys(raw: &str) -> bool {
    let opts = jsonc_parser::ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        ..Default::default()
    };
    let value: serde_json::Value =
        match jsonc_parser::parse_to_serde_value(raw, &opts) {
            Ok(v) => v,
            Err(_) => return false, // genuinely malformed; main parser errors
        };
    if let Some(policy) = value.get("policy").and_then(|v| v.as_object()) {
        return policy.contains_key("bypass_protection")
            && policy.contains_key("override_deny");
    }
    false
}
```

Apply the same JSONC parse in `raw_profile_has_legacy_override_deny_key`. Add a
regression test: a JSONC profile (with a comment) carrying both keys must return
`Err(ProfileParse)` from `parse_profile_bytes`/`parse_profile_file`.

## Warnings

### WR-01: `--detach-timeout` env binding can break ordinary `nono run` invocations

**File:** `crates/nono-cli/src/cli.rs:2236-2245`
**Issue:**
The new flag is declared with both `env = "NONO_DETACH_STARTUP_TIMEOUT"` and
`requires = "detached"`. In clap, an arg populated from its `env` source is
treated as "present," which fires `requires`. A user who exports
`NONO_DETACH_STARTUP_TIMEOUT` globally (the documented way to tune the timeout —
see `timeouts.rs::detach_startup_timeout`) will then have every non-`--detached`
`nono run` fail argument validation with "the following required argument was not
provided: --detached". The env var is also read independently by
`timeouts::detach_startup_timeout()`, so the binding is redundant on top of being
hazardous.
**Fix:** Drop the `env = ...` binding from this clap arg (let
`timeouts::detach_startup_timeout()` own env reading), or drop `requires =
"detached"` and ignore the value when `--detached` is absent. Add a test that
`Cli::parse_from(["nono","run","--","echo"])` succeeds with
`NONO_DETACH_STARTUP_TIMEOUT` set in the environment.

### WR-02: `binary` precedence is correct but the `command` arg is no longer `required`, weakening the "missing command" error surface

**File:** `crates/nono-cli/src/cli.rs:2414-2415`, `crates/nono-cli/src/command_runtime.rs:50-71`
**Issue:**
`command` lost `required = true`. The empty-command case is now only caught deep
inside `resolve_program_from_profile_or_cli`, which returns `NonoError::NoCommand`
only after `load_profile(name)?` has run. Two consequences: (1) `nono run`
(no profile, no command) now surfaces a generic library error instead of clap's
usage message; (2) for a profile with no `binary` and no trailing command, the
profile is fully loaded (including a possible registry auto-pull) before the
"no command" error fires, changing failure ordering and side effects vs. the
previous fast clap rejection.
**Fix:** Keep the deferred resolution, but short-circuit with a clear
user-facing error (and clap-style usage hint) when both `command.is_empty()` and
the resolved profile has no honoured `binary`, before any network-capable
`load_profile` side effects. At minimum add a test asserting the error message
for `nono run` with neither command nor profile-binary.

### WR-03: `output.rs` path/label split mis-renders denial lines whose label group contains spaces or whose path has no ` (`

**File:** `crates/nono-cli/src/output.rs:617-626`
**Issue:**
`render_diagnostic_line` now splits the diagnostic path line on the *last* ` (`
via `rfind(" (")` to bold only the path. The diagnostic footer in
`diagnostic.rs:1469-1486` now appends multi-token labels like
`(read)  [permanently restricted, save skipped]`. That trailing label is `[...]`
(square brackets), so `rfind(" (")` still matches the `(read)` access group —
but only if a ` (` exists. A denial line emitted without an access-type paren
group (or whose only ` (` is inside the path, e.g.
`/home/u/a (b)/file` with no trailing ` (read)`) will bold the wrong span: the
test `render_diagnostic_footer_splits_path_on_last_paren_group` covers the
embedded-paren happy case but not the "path contains ` (` and there is no
trailing access group" case, where everything from the path's own ` (` onward is
rendered unbolded as if it were the label.
**Fix:** Anchor the split to the known footer grammar instead of a generic
`rfind(" (")`: split on the last ` (` only when the tail matches
`(<access>)` optionally followed by ` [<labels>]`. Add a regression test for a
path containing ` (` with no trailing access-type group.

### WR-04: Orphaned `opencode_linux` policy group left behind after profile removal

**File:** `crates/nono-cli/data/policy.json:380`
**Issue:**
The `opencode` built-in profile was removed (39 lines deleted), but the
`opencode_linux` group it referenced remains defined in `policy.json` and is now
referenced by no profile. Policy loading does not validate that defined groups
are reachable, so this is silent dead config that will drift and confuse future
audits of the group surface.
**Fix:** Remove the `opencode_linux` group definition, or, if it is intentionally
retained for the registry pack to reference, add a comment documenting that and a
test asserting the pack consumes it.

## Info

### IN-01: `canonical_for_denial` positional lookup is ambiguous for duplicate denial paths

**File:** `crates/nono/src/diagnostic.rs:1551-1561`
**Issue:**
`canonical_for_denial` locates the precomputed canonical entry via
`self.denials.iter().position(|d| d.path == denial.path)`, which returns the
*first* index whose path matches. If two `DenialRecord`s share the same `path`
(e.g. a Read and a Write denial for the same file), both map to the first index.
The canonical values would be identical in practice, so this is currently
harmless, but the parallel-vec-by-position contract is fragile if
`canonical_denial_paths` is ever built from a differently-ordered source.
**Fix:** Index `canonical_denial_paths` by the same enumeration index used to
build it, or key suppression off a `HashMap<PathBuf, PathBuf>` rather than
positional alignment.

### IN-02: `refresh_in_background_process` silently swallows all spawn errors

**File:** `crates/nono-cli/src/pack_update_hint.rs:194-206`
**Issue:**
`let _ = child.spawn();` discards the spawn result entirely. This is intentional
(zero-startup-latency, best-effort hint refresh), but with `NONO_LOG=trace` there
is no diagnostic at all if the detached helper fails to launch, making the
"hints never appear" symptom hard to debug.
**Fix:** Log at `debug!`/`trace!` on spawn error before discarding, mirroring the
`tracing::debug!` style used elsewhere in this module.

### IN-03: Pack-update-hint state file is read/written without locking across concurrent helpers

**File:** `crates/nono-cli/src/pack_update_hint.rs:260-293`
**Issue:**
`save_state` uses a pid-scoped temp + atomic rename (good, prevents partial
writes), but two concurrently-running helper processes can each `load_state`,
mutate disjoint entries, and `rename` — last-writer-wins drops the other's
update. This only costs a one-cycle-delayed hint, so it is acceptable, but worth
noting since the pid-scoped temp name implies concurrency was anticipated.
**Fix:** Acceptable as-is for a best-effort cache; if stronger consistency is
ever wanted, read-modify-write under an advisory lock on the state file.

---

_Reviewed: 2026-06-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
