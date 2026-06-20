---
phase: 88-feature-dependency-cherry-pick-wave
reviewed: 2026-06-20T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - bindings/c/src/diagnostic.rs
  - bindings/c/src/lib.rs
  - crates/nono-cli/src/exec_strategy/env_sanitization.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/state_paths.rs
  - crates/nono-cli/src/config/mod.rs
  - crates/nono-cli/src/hook_runtime.rs
  - crates/nono-proxy/src/credential.rs
  - crates/nono-proxy/src/reverse.rs
findings:
  critical: 0
  warning: 4
  info: 5
  total: 9
status: issues_found
---

# Phase 88: Code Review Report

**Reviewed:** 2026-06-20
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Phase 88 is a UPST9 feature + dependency cherry-pick wave on a security-critical
sandboxing system. I reviewed the 9 in-scope source files through a security-first
lens against the project CLAUDE.md invariants.

The headline security mechanisms are correctly implemented:

- **CR-01 FFI stale-state reset** is genuinely wired at the entry of every
  `pub unsafe extern "C"` function that calls `set_last_error`/`map_error`
  (state.rs, query.rs, sandbox.rs, capability_set.rs setters, fs_capability.rs
  `fs_access`/`fs_source_tag`, diagnostic.rs). A regression test
  (`diagnostic_code_is_cleared_between_calls`) locks the set_last_error-only path.
- **FEAT-01 `validate_set_vars`** correctly rejects `PATH` and any `NONO_*` key,
  and is invoked fatally at profile-load (`profile/mod.rs:2926` and `:2989`).
- **FEAT-02 XDG/LOCALAPPDATA path resolution** (`state_paths.rs`, `config/mod.rs`)
  uses `Path::starts_with` component comparison (`is_under_legacy`), canonicalizes
  via `try_canonicalize`, and fails closed on absent `%LOCALAPPDATA%`/`HOME`.
- **D-03 migration fail-secure**: `maybe_migrate_legacy_audit_ledger()` propagates
  every fs error via `?` (temp-file + atomic rename) and is called with `?` from
  `audit_ledger.rs:31` — no silent degradation.
- **D-14 platform divergence**: `env_clear()` correctly remains on
  `hook_runtime_windows.rs:301` (with OS-baseline re-add) and is absent on the
  Unix `hook_runtime.rs` path.
- **AWS auth mutual exclusion** is enforced at the CLI validation boundary
  (`profile/mod.rs:1127-1148`), and the D-15 501 stub on the non-TLS proxy path
  runs only after session-token authentication.

The findings below are correctness/consistency gaps, not exploitable holes, but
several touch the security contract surface and should be fixed.

## Warnings

### WR-01: CR-01 invariant incomplete — string-returning FFI getters can leave stale error state

**File:** `bindings/c/src/fs_capability.rs:34-46, 57-69, 167-182`; `bindings/c/src/capability_set.rs:427-435`
**Issue:** `lib.rs:84` documents that `clear_last_call_state()` is "Called at the
entry of every `pub unsafe extern "C"` function that can set any thread-local."
Four string-returning getters violate that invariant: `nono_capability_set_fs_original`,
`nono_capability_set_fs_resolved`, `nono_capability_set_fs_source_group_name`, and
`nono_capability_set_summary` all call `rust_string_to_c`, which calls
`set_last_error(...)` when the string contains an interior NUL byte
(`lib.rs:219-223`). None of these four functions call `clear_last_call_state()` at
entry. Consequence: a C caller that inspects `nono_last_error()` after one of these
calls can observe a **stale error** from a prior FFI call even when the current call
succeeded (returned a non-NULL pointer) — exactly the cross-call leak class CR-01
was created to close. The path-display strings are unlikely to contain NUL in
practice, so this is a contract/consistency defect rather than a live exploit.
**Fix:** Add the same guard at the entry of each of the four getters:
```rust
pub unsafe extern "C" fn nono_capability_set_fs_original(
    caps: *const NonoCapabilitySet,
    index: usize,
) -> *mut c_char {
    crate::clear_last_call_state(); // CR-01: reset stale state before any rust_string_to_c
    if caps.is_null() {
        return std::ptr::null_mut();
    }
    // ...
}
```
Alternatively, tighten the lib.rs doc comment to scope the invariant to functions
that set errors *directly*, and document that getters returning `rust_string_to_c`
do not clear — but the additive guard is the safer choice given the stated contract.

### WR-02: AWS/credential mutual exclusion not enforced at the proxy boundary (defense-in-depth gap)

**File:** `crates/nono-proxy/src/config.rs:163-178`; `crates/nono-proxy/src/credential.rs:158-226`
**Issue:** `RouteConfig` documents `aws_auth` as "Mutually exclusive with
`credential_key` and `oauth2`" (config.rs:174-177) and `oauth2` as mutually
exclusive with `credential_key` (config.rs:166), but `nono-proxy` performs **no
runtime validation** of these invariants. In `CredentialStore::load`
(credential.rs:158) the logic is `if route.credential_key.is_some() { ... }
else if route.aws_auth.is_some() { ... }`. If a `RouteConfig` arrives with BOTH
`credential_key` and `aws_auth` set, `credential_key` silently wins and `aws_auth`
is dropped with no error. The CLI validates this at `profile/mod.rs:1127-1148`, so
the only callers today are safe — but `nono-proxy` is a separate crate with public
`CredentialStore::load` / `RouteConfig`, and any other embedder (or a future
direct proxy-config load path) gets silent, fail-*open*-shaped behavior instead of
a hard error. CLAUDE.md: "Configuration load failures must be fatal."
**Fix:** Reject conflicting auth fields inside `CredentialStore::load` before the
`if/else if`, returning `ProxyError::Credential`:
```rust
for route in routes {
    let has_static = route.credential_key.is_some();
    let has_aws = route.aws_auth.is_some();
    let has_oauth = route.oauth2.is_some();
    if (has_static as u8 + has_aws as u8 + has_oauth as u8) > 1 {
        return Err(ProxyError::Credential(format!(
            "route '{}' sets more than one of credential_key/aws_auth/oauth2; \
             these are mutually exclusive",
            route.prefix
        )));
    }
    // ...
}
```

### WR-03: Unix hooks inherit the full parent environment (platform-inconsistent with Windows env_clear)

**File:** `crates/nono-cli/src/hook_runtime.rs:189-225`
**Issue:** `build_hook_command` (Unix) constructs `Command::new(script)` and sets
only the `NONO_*` vars; it never calls `env_clear()`. The Windows analog
(`hook_runtime_windows.rs:301`) calls `env_clear()` and re-adds only a minimal
OS baseline. The Unix hook therefore inherits the **entire** parent environment —
including `LD_PRELOAD`, `NODE_OPTIONS`, `OP_SESSION_*`, etc. — into the
host-privileged hook process. This is not a sandbox escape (hooks run pre-sandbox
with host privileges by design, and the `is_dangerous_env_var` filter is correctly
applied to what the hook *exports back* at `hook_runtime.rs:120`). The risk is the
asymmetry: a poisoned parent env (e.g. `nono` launched with an attacker-influenced
`LD_PRELOAD`) is forwarded verbatim into the hook on Unix but stripped on Windows,
and 1Password session/credential vars (`OP_SESSION_*`, `OP_SERVICE_ACCOUNT_TOKEN`)
that `is_dangerous_env_var` exists specifically to contain are visible to the hook
subprocess. If D-14 is a deliberate decision to match upstream `e54cf9cb`, this is
acceptable but under-documented relative to the security model.
**Fix:** Either (a) document explicitly in the `hook_runtime.rs` module header why
the Unix path intentionally inherits the parent env while Windows clears it (the
current doc only says hooks "run outside the sandbox with host privileges"), or
(b) for parity, `env_clear()` then re-add a vetted allowlist plus a curated PATH on
Unix as well. At minimum, confirm the divergence is the intended D-14 outcome and
cross-reference the divergence-ledger entry from the code comment.

### WR-04: Reverse-proxy header parsing silently drops all headers / Content-Length on invalid UTF-8

**File:** `crates/nono-proxy/src/reverse.rs:461-498`
**Issue:** `filter_headers` does `std::str::from_utf8(header_bytes).unwrap_or("")`
(line 462) and `extract_content_length` does `.ok()?` (line 490). If the inbound
header block contains any invalid UTF-8 byte, `filter_headers` returns an empty
vector (no headers forwarded upstream) and `extract_content_length` returns `None`
(body never read). Combined, a request with a non-UTF-8 header byte and a real body
results in the body being left unread on the socket while the upstream request is
sent with zero forwarded headers — a request/response desync on a keep-alive-style
read loop, and a silent loss of headers the operator may expect to be forwarded.
HTTP header field-values are technically opaque octets, not guaranteed UTF-8.
**Fix:** Decode headers leniently and consistently (e.g. `from_utf8_lossy`) or
reject the request with a 400 when the header block is not valid UTF-8, so the
body-length and header-forwarding decisions stay coherent:
```rust
let header_str = match std::str::from_utf8(header_bytes) {
    Ok(s) => s,
    Err(_) => { send_error(stream, 400, "Bad Request").await?; return Ok(()); }
};
```
Apply the same decode result to both `filter_headers` and `extract_content_length`
(parse once, pass the `&str` in) so they cannot disagree.

## Info

### IN-01: `extract_content_length` honors the first of possibly-duplicate Content-Length headers

**File:** `crates/nono-proxy/src/reverse.rs:489-498`
**Issue:** `extract_content_length` returns the first `content-length:` line found.
A client sending two conflicting `Content-Length` headers could create a length
ambiguity (classic request-smuggling shape). The proxy binds to localhost and the
session token gates access, so impact is low, but the parser should be strict.
**Fix:** Reject requests with more than one `Content-Length` header (or any
`Transfer-Encoding: chunked` combined with `Content-Length`).

### IN-02: `CredentialStore` documents an `aws_routes` placeholder that is silently unreachable when paired with credential_key

**File:** `crates/nono-proxy/src/credential.rs:219-226`
**Issue:** Tied to WR-02 — the `aws_routes.insert` only runs in the `else if`
branch, so an AWS route is registered only when `credential_key` is absent. With
the mutual-exclusion check from WR-02 in place this becomes unreachable-by-contract;
without it, AWS routes are silently dropped. Documented here for traceability.
**Fix:** Covered by WR-02.

### IN-03: `protected_state_roots` returns non-canonical paths when the directories do not yet exist

**File:** `crates/nono-cli/src/state_paths.rs:148-156`
**Issue:** `try_canonicalize` returns the input path unchanged when the target does
not exist. Before first run, `protected_state_roots()` therefore returns
non-canonical forms (e.g. `~/.local/state/nono` literal), which on macOS may not
match a later-canonicalized `/private/...`-resolved comparison. The downstream
"must not be grantable" check should compare canonicalized forms on both sides;
verify the consumer canonicalizes the candidate grant path too.
**Fix:** Ensure the grantability check canonicalizes the candidate path with the
same `try_canonicalize` helper before comparing against `protected_state_roots()`.

### IN-04: Windows `is_dangerous_env_var` strips `TEMP`/`TMP`/`SystemRoot`/`PATH` for inherited child env — relies on out-of-scope baseline re-add

**File:** `crates/nono-cli/src/exec_strategy/env_sanitization.rs:66-77`
**Issue:** The D-09 Windows arm marks `PATH`, `TEMP`, `TMP`, `SystemRoot`, `windir`,
`COMSPEC`, etc. as dangerous unconditionally on Windows. `should_skip_env_var`
(line 88) is shared by the sandboxed-child env build, so these are stripped from the
child unless `launch.rs::build_child_env` re-adds an OS baseline. That re-add lives
in `exec_strategy_windows/launch.rs` (out of scope for this review). If the baseline
re-add regresses, Windows sandboxed children would fail interpreter/CLR startup
(the documented -65536 class). Flagging for cross-file verification, not as a defect
in the reviewed file.
**Fix:** Confirm `build_child_env` re-adds `SystemRoot`/`windir`/`SystemDrive` (and
a curated PATH) after `should_skip_env_var` filtering; add a Windows test asserting
the child env contains `SystemRoot`.

### IN-05: `resolve_xdg_state_base` warns-and-falls-back on a relative `XDG_STATE_HOME` rather than failing closed

**File:** `crates/nono-cli/src/state_paths.rs:32-46`
**Issue:** When `XDG_STATE_HOME` is set but not absolute, the function logs a
`tracing::warn!` and falls back to `$HOME/.local/state`. This matches the
`resolve_user_config_dir` behavior and is reasonable for a state dir, but per
CLAUDE.md "Validate environment variables before use" a malicious relative
`XDG_STATE_HOME` is ignored silently-ish (only a warn). Acceptable because the
fallback is a fixed safe default, not attacker-controlled. Documented for awareness.
**Fix:** None required; optionally elevate the relative-path case to an error if
strict env validation is desired for state paths (consistent with `validated_home`
which *does* fail closed on relative `HOME`).

---

_Reviewed: 2026-06-20_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
