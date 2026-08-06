---
phase: 112-security-residual-sync
reviewed: 2026-08-05T00:00:00Z
depth: standard
pass: 2 (re-review after Critical-only fix pass)
files_reviewed: 33
files_reviewed_list:
  - crates/nono/src/lib.rs
  - crates/nono/src/sandbox/linux.rs
  - crates/nono/src/sandbox/mod.rs
  - crates/nono/src/supervisor/socket.rs
  - crates/nono/src/trust/mod.rs
  - crates/nono/src/trust/policy.rs
  - crates/nono/src/trust/types.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/proxy_command.rs
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/src/pty_proxy.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/src/timeouts.rs
  - crates/nono-cli/src/trust_cmd.rs
  - crates/nono-cli/src/trust_scan.rs
  - crates/nono-cli/tests/proxy_command_run.rs
  - crates/nono-cli/tests/socket_access_run.rs
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/connect.rs
  - crates/nono-proxy/src/external.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/server.rs
  - docs/cli/features/trust.mdx
  - tests/integration/test_trust_cli.sh
findings:
  critical: 2
  warning: 14
  info: 7
  total: 23
new_this_pass:
  critical: 2
  warning: 3
  info: 2
carried_forward:
  warning: 11
  info: 5
closed_this_pass:
  critical: 4
  warning: 1
status: issues_found
---

# Phase 112: Code Review Report (Pass 2 — adversarial re-review)

**Reviewed:** 2026-08-05
**Depth:** standard
**Files Reviewed:** 33 (diff base `39ebe2fb`; fix commits `916d3bd7`, `892d2e01`, `585f2743`, `20f9a1cd`, `6706b18d`)
**Status:** issues_found

## Summary

This is a second adversarial pass over the same 33 files. Priorities were (1) re-test
the four "fixed" Criticals against current source rather than against the resolution
notes, (2) re-assess the 12 Warnings / 5 Info that were deliberately left open, and
(3) hunt for defects introduced by the fix commits themselves.

**Verdict on the four Criticals** (full detail in `## Previously Resolved`):

| ID | Verdict |
|----|---------|
| CR-01 | **confirmed fixed** — deny layer traced end-to-end into the running filter |
| CR-02 | **partially fixed** — the headline hole is genuinely closed, but the fix left one fail-open (CR-06) and introduced one fail-closed DoS (WR-14) |
| CR-03 | **confirmed fixed** on the reaper path; three sibling exit paths still skip the drain (WR-15) |
| CR-04 | **confirmed fixed** — guard is unbypassable and runs before any kernel state is touched |

The CR-01 fix is the strongest of the four. I traced `net.deny_domain` →
`ProxyLaunchOptions.deny_domain` → `network_policy::expand_proxy_deny` →
`ProxyConfig.denied_hosts` → `ProxyFilter::with_denied_hosts` (`server.rs:554`), and
`net.block` → `strict_filter` → `ProxyFilter::new_strict` (`server.rs:547`). The deny
layer reaches the kernel-facing filter, not merely the options struct. `no_proxy` reaches
`ProxyConfig.no_proxy` and is re-validated by `validate_no_proxy_config` at
`server::start`. The D-04 deny-without-allow guard is genuine parity with
`sandbox_prepare::validate_deny_domain_requires_allow_domain` and fires from both the
CLI-flag and profile sources.

The CR-04 fix is also sound: `validate_external_tcp_delegation` is the *first* statement
in `apply_with_abi_inner` (`linux.rs:723-725`), before `requested_scopes`, before
`Ruleset::default()`, before any `restrict_self`. Both public entry points
(`apply_seccomp`, `apply_seccomp_with_abi`) funnel through it, and `SeccompOpts.handle_tcp`
is a private field so no external caller can construct a bypassing value. It checks all
four network inputs (`network_mode`, `tcp_connect_ports`, `tcp_bind_ports`,
`localhost_ports`), and the `handle_tcp &&` gate on the `open_port 0` check was correctly
removed.

**Where this pass lands the sharpest hits:**

1. **CR-05** — the CR-02 fix closed "an untrusted repo can *empty* the operator's trust
   policy" but left the mirror-image hole wide open: `merge_policies` unions
   `publishers`, so an untrusted repo's project-level `trust-policy.json` can *add* a
   trusted signer (including an inline `public_key` that `verify_keyed_crypto` will
   actually use). `trust/policy.rs`'s own module doc asserts "Project-level policy cannot
   weaken user-level or embedded policy" — the implementation contradicts it.
2. **CR-06** — the new content-based legacy detection resolves the absent-predicate case
   by `match trust::load_policy_from_str(&content) { Ok(p) => .., Err(_) => Ok(None) }`.
   The `Err(_)` discards the real deserialization error, so a *legacy* (predicate-less)
   policy with any schema defect — including the operator's own user-level policy — is
   silently reclassified as "foreign file, skipping" and trust enforcement quietly stops
   covering it. CLAUDE.md: *"Configuration load failures must be fatal."*
3. **WR-14** — the same fix made a *present-but-wrong* predicate a hard error. In-toto /
   SLSA attestation JSON has a `predicate` field (an object), so dropping such a file
   named `trust-policy.json` into a repo now aborts every `nono run` in that directory.
   Repo-content-triggered denial of service, and it defeats the exact foreign-file
   ergonomics the discriminator was introduced for.

Eleven of the twelve prior Warnings and all five Info findings still stand verbatim
against current code (line numbers corrected below). WR-11 is **closed** — its
parser-differential half was fixed by CR-02, and its second half ("misleading
`failed to parse {path}` for well-formed foreign JSON") was mis-stated in the prior
pass: well-formed JSON always deserializes into `serde_json::Value`, so it never reaches
that arm.

## Critical Issues

### CR-05: A project-level `trust-policy.json` can add trusted publishers to the effective policy

**Files:**
`crates/nono/src/trust/policy.rs:16` (invariant claim), `:105-117` (publisher union);
`crates/nono-cli/src/trust_scan.rs:172-198` (`nono run` merge), `:820-846`
(`verify_keyed_crypto` consumes the publisher's inline `public_key`);
`crates/nono-cli/src/trust_cmd.rs:1609-1638` (CLI merge)

**Issue:** `merge_policies` unions `publishers` across policy levels, deduplicating by
`name` with first-occurrence-wins. `load_scan_policy` passes `&[user, project]`, so a
project policy cannot *override* a user publisher of the same name — but it can freely
**add new ones**:

```rust
// trust/policy.rs:108-117
for publisher in &policy.publishers {
    if !seen_publisher_names.insert(publisher.name.clone()) { /* debug! */ }
    else { merged_publishers.push(publisher.clone()); }   // <-- project publisher accepted
}
```

`trust_scan::verify_keyed_crypto` then resolves the verification key **from the merged
policy's publisher entry**, preferring an inline base64 `public_key` over the system
keystore:

```rust
// trust_scan.rs:826-833
let matching = policy.matching_publishers(identity);
let pub_key_b64 = matching.iter().find_map(|p| p.public_key.as_ref());
let key_bytes = if let Some(b64) = pub_key_b64 { base64_decode(b64)? } else { ... };
```

So a repository that ships:

```json
{ "predicate": "https://nono.sh/attestation/trust-policy/v1",
  "includes": [], "enforcement": "audit",
  "publishers": [{ "name": "attacker", "key_id": "evil", "public_key": "<attacker SPKI>" }],
  "blocklist": { "digests": [] } }
```

plus `CLAUDE.md` + `CLAUDE.md.bundle` signed with the matching private key gets
`VerificationOutcome::Verified { publisher: "attacker" }` for its own instruction file,
under the operator's `enforcement: deny` user policy. The repo has nominated itself as a
trusted signer.

The only obstacle is `verify_scan_policy_signatures` (`trust_scan.rs:200-205`), which
runs *only* when `scan_has_signed_artifacts` is true and requires the project policy to
carry a valid `.bundle`. But `verify_policy_signature` explicitly does **not** pin signer
identity for keyless bundles (`trust_scan.rs:356-363`: *"Policy signature proves
provenance and tamper-resistance, not signer allowlisting"*), so an attacker can satisfy
it with a Sigstore bundle minted from their own CI. The keyed branch is safe (keystore
lookup), the keyless branch is not. And when the victim's own user policy is *unsigned*
— the state `nono trust init` leaves it in, since `run_init` (`trust_cmd.rs:140-155`)
writes the JSON but does not sign it — the whole verification block errors out rather
than protecting anything.

`crates/nono/src/trust/policy.rs:16` states the intended invariant outright:

```
//! Project-level policy cannot weaken user-level or embedded policy.
```

Adding a trusted signer weakens it. This is the exact threat model CR-02 was fixed
against (*"attacker-controllable input deciding whether the operator's policy applies"*),
and the CR-02 fix rewrote both merge sites without addressing it.

**Pre-existing:** yes — publisher union predates this phase. It is reported here because
(a) the phase rewrote both call sites, (b) the CR-02 fix's own rationale makes it
in-scope, and (c) the fix *broadened* the reachable input set by newly accepting
predicate-less project policies that were previously skipped.

**Fix:** treat the publisher set as user-authoritative. Either drop project-level
publishers entirely, or require an explicit opt-in:

```rust
/// Merge with level provenance. Project-level policies may add `includes`,
/// `files` and blocklist entries (strictly narrowing), but never publishers —
/// the trust anchor set is defined by the user/operator level only.
pub fn merge_policies_scoped(user: &TrustPolicy, project: &TrustPolicy) -> Result<TrustPolicy> {
    let mut merged = merge_policies(&[user.clone(), project.clone()])?;
    merged.publishers = user.publishers.clone();          // project cannot add signers
    Ok(merged)
}
```
and warn loudly when a project policy declares publishers that were dropped. Separately,
ignore `Publisher.public_key` when the publisher came from a project-level policy.

**Resolution (Group A fix pass):** FIXED — level-aware merge, not removal of the union.

Core crate gains the *mechanism* only (ADR-86 — `crates/nono/` stays policy-free and
takes no view on which level is which):

- `PolicyLayer<'a>` with `trust_anchor(&p)` / `narrowing_only(&p)` constructors and a
  `dropped_publishers()` accessor so callers can warn instead of discarding silently.
- `merge_policy_layers(&[PolicyLayer])` — publishers are unioned across
  `trust_anchor` layers only; everything else merges exactly as before.
- `merge_policies(&[TrustPolicy])` is kept as a thin wrapper that maps every input to
  `trust_anchor`, so the existing public API is **unchanged** for `bindings/c`,
  `nono-py` and `nono-ts`. Its doc now states the precondition explicitly.

`nono-cli` owns the *decision*. `trust_scan::project_policy_layer()` is the single
place that says "a project-level `trust-policy.json` is narrowing-only", and it emits
the operator warning naming the dropped publishers (sanitized). Both merge sites —
`trust_scan::load_scan_policy` and `trust_cmd::load_trust_policy` — go through it; a
guard on only one of the two would be a bypass.

**The rule is uniform, including the `(None, Some(project))` arm.** A project policy
contributes no trust anchors even when no user-level policy exists. Making it
conditional ("project publishers count only when unanchored") would itself be
exploitable — an attacker targeting a fresh machine that has no user policy yet would
get to self-nominate, which is exactly the state that arm already prints a warning
about. **Behaviour change:** a repository that previously self-verified its own
instruction files via a project-only policy under `enforcement: deny` will now see
`UntrustedPublisher` and be blocked, with a warning pointing at the user-level policy.
That is deliberate and fail-secure, per CLAUDE.md's "when in doubt, choose the more
restrictive option."

`Publisher.public_key` smuggling is closed as a consequence: a project publisher never
reaches the merged policy, so `verify_keyed_crypto` can never resolve an inline
attacker-supplied key from one.

The module doc at `crates/nono/src/trust/policy.rs:16` was rewritten to be TRUE of the
code: it now states which fields are monotonically narrowing, that `publishers` is the
sole widening field because it is the trust anchor set, and that `merge_policies`
treats every input as an anchor and is therefore only safe for already-trusted levels.

Regression tests — core: `narrowing_only_layer_cannot_add_a_publisher`,
`narrowing_only_layer_cannot_smuggle_an_inline_public_key`,
`narrowing_only_layer_still_narrows`, `dropped_publishers_reports_the_discarded_set`,
`merge_policies_still_treats_every_input_as_a_trust_anchor`. CLI end-to-end:
`load_scan_policy_drops_project_level_publishers`,
`load_scan_policy_drops_project_publishers_without_user_policy`.

---

### CR-06: Legacy trust-policy acceptance swallows the parse error — a malformed policy silently disables enforcement

**File:** `crates/nono-cli/src/trust_scan.rs:96-128` (the `Err(_)` arm at `:115`)

**Issue:** The CR-02 fix resolves the absent-`predicate` case by content:

```rust
None => {
    return match trust::load_policy_from_str(&content) {
        Ok(policy)  => { eprintln!("...legacy...");  Ok(Some(policy)) }
        Err(_)      => { eprintln!("...not a nono trust policy — skipping."); Ok(None) }
    };
}
```

`Err(_)` conflates two very different states:

1. a genuinely foreign JSON file (AWS IAM et al.) — correctly skipped;
2. a **real nono trust policy that fails to deserialize** — silently skipped.

Case 2 is a fail-open. `TrustPolicy` requires `includes`, `publishers`, `blocklist` and
`enforcement` (no `#[serde(default)]` on those four — `types.rs:52,62,64,66`), and
`load_policy_from_str` also runs `validate_version()` which errors on oversized
collections. So any of these in an operator's **user-level** policy silently disables it:

- `"enforcement": "denied"` (typo — the enum is `audit`/`warn`/`deny`)
- a missing `blocklist` block
- a trailing-comma-free but structurally renamed field
- `includes` with more than 100 entries (`MAX_INCLUDES`)

The result is `Ok(None)` → `load_scan_policy`'s `(None, None)` arm →
`TrustPolicy::default()` → `includes: []` → **zero files are ever selected for
verification**, with one yellow line on stderr that says the file "is not a nono trust
policy". The operator sees a message that actively misdirects them away from the real
cause (a schema error the code already computed and threw away).

This is the same class of defect CR-02 was raised for, and it directly violates
CLAUDE.md: *"Configuration load failures must be fatal. If security lists fail to load,
abort"* and *"Silent fallbacks: `unwrap_or_default()` on security config returns empty
permissions = no protection."*

Note the asymmetry the fix created: a *wrong* predicate is now a hard error (correct),
but a *missing* predicate plus a broken schema is a silent skip.

**Fix:** disambiguate structurally before deciding, and never discard the error:

```rust
None => {
    // A file carrying any nono-policy-shaped key is a nono policy. Failing to
    // parse one is a configuration load failure, and those are fatal.
    const NONO_KEYS: &[&str] = &["includes", "instruction_patterns", "publishers",
                                 "blocklist", "enforcement", "files"];
    let looks_like_nono = raw.as_object()
        .is_some_and(|o| NONO_KEYS.iter().any(|k| o.contains_key(*k)));

    return match trust::load_policy_from_str(&content) {
        Ok(policy) => { warn_legacy(path); Ok(Some(policy)) }
        Err(e) if looks_like_nono => Err(nono::NonoError::TrustPolicy(format!(
            "{}: looks like a nono trust policy but failed to load: {e}. \
             Refusing to run with trust enforcement silently disabled.",
            path.display()))),
        Err(_) => { warn_foreign(path); Ok(None) }
    };
}
```

At minimum, include `{e}` in the "not a nono trust policy" warning so the operator can
see the schema error.

**Resolution (Group A fix pass):** FIXED.

`load_nono_policy` now takes a `PolicyLevel` (`Trusted` | `Project`) — a policy
judgement, so it lives in `nono-cli`, not the policy-free core crate (ADR-86). The
`Err(_)` arm is gone; the predicate-less path is factored into `load_predicate_less`,
which never discards the deserialization error:

- `PolicyLevel::Trusted` (user config-dir policy, or a path named explicitly with
  `--policy` / `sign-policy`) — **any** load failure is fatal, unconditionally.
  Nothing but nono writes to those locations, so "not mine, skip it" is never a valid
  conclusion. CLAUDE.md: *"Configuration load failures must be fatal."*
- `PolicyLevel::Project` (auto-discovered in the working directory) — fatal when the
  document is nono-policy-shaped (`NONO_POLICY_KEYS` = `includes`,
  `instruction_patterns`, `publishers`, `blocklist`, `enforcement`), skipped otherwise.
  `files` was deliberately **excluded** from that key set: it is too generic to
  discriminate a nono policy from an arbitrary config, and including it would re-open
  WR-14's repo-content-triggered DoS.
- Either way the underlying error is surfaced, run through `sanitize_untrusted` first
  (serde echoes unknown enum variants verbatim from a file nono does not control).

Regression tests added in `trust_scan.rs`:
`load_nono_policy_rejects_nono_shaped_policy_that_fails_to_load`,
`load_nono_policy_user_level_failure_is_always_fatal`,
`load_scan_policy_aborts_on_broken_user_policy`.

## Warnings

### WR-13 (new): `handle_forward_http` ignores `require_auth` — `--no-auth` does not apply to plain-HTTP forward proxying

**File:** `crates/nono-proxy/src/server.rs:914-945` (gate at `:924`), vs `:1231`, `:1248`,
`:1258`, `:1269`, `:1300`

**Issue:** Every other request path in `handle_connection` threads
`state.config.require_auth` through — CONNECT (`:1258`/`:1269`), the external-proxy bypass
(`:1248`), the external-proxy chain (`:1231`), and the reverse proxy (`:1300`).
`handle_forward_http` — the absolute-form `http://…` forward-proxy branch reached at
`:1279` — validates unconditionally:

```rust
// server.rs:924 — no `if state.config.require_auth` guard
if let Err(e) = token::validate_proxy_auth(header_bytes, &state.session_token) {
    ... 407 ...
}
```

`nono proxy --no-auth` is documented as *"accept every request on the bind address"*
(`cli.rs:2348-2350`) and `ProxyConfig.require_auth`'s doc says *"every request on the bind
address is accepted without validating the session token"* (`config.rs:98-101`). Neither
is true for `http://` forward-proxied requests: those still 407. The failure direction is
fail-closed, so this is not an exploit — it is an unadvertised functional hole that will
be read as "`--no-auth` is broken" and invites a future "fix" that removes the check
without noticing it is the only one there. Five call sites agree; one silently does not.

**Fix:** make the intent explicit rather than accidental:

```rust
if state.config.require_auth {
    if let Err(e) = token::validate_proxy_auth(header_bytes, &state.session_token) { ... }
}
```
or, if forward-HTTP is deliberately always authenticated, say so in a comment *and* in
`--no-auth`'s help text.

---

### WR-14 (new): a foreign or malformed `trust-policy.json` in the working directory now aborts every `nono run` in that directory

**File:** `crates/nono-cli/src/trust_scan.rs:63-93`

**Issue:** The CR-02 fix converted two previously-skipped shapes into hard errors:

```rust
Some(serde_json::Value::String(other)) => return Err(...unrecognised predicate...),
Some(_)                                => return Err(...'predicate' must be a string...),
```

plus the pre-existing `serde_json::from_str::<Value>` failure at `:63-65`. All three
propagate out of `load_scan_policy`, which is on the `nono run` startup path, so the run
aborts.

The `Some(_)` (non-string) arm is the problem. **in-toto / SLSA attestation statements
carry a `predicate` field whose value is an object** — that is the canonical shape the
whole `predicateType`/`predicate` convention comes from, and it is not exotic in a repo
that already uses Sigstore. Any repository can now stop `nono run` from starting in that
directory by committing a `trust-policy.json` containing `{"predicate": {...}}`, or any
JSON with a comment (JSONC/JSON5 config), or a leading BOM. That is repo-content-triggered
denial of service on the sandbox itself.

It also defeats the stated purpose of the discriminator. `load_nono_policy`'s own doc
(`:27-31`) says the peek exists *"to avoid false errors on identically-named foreign
files"* — and the non-string arm produces exactly a false error on an identically-named
foreign file.

Fail-closed is right for *"is trust enforcement active?"*. It is not right for *"is this
file mine?"* — that question has a correct third answer (skip), and the code now refuses
to give it for two of the three foreign shapes.

**Fix:** keep the hard error only where the file self-identifies as a nono policy and
lies about the version, and skip otherwise:

```rust
// A non-string predicate is a foreign attestation shape (in-toto/SLSA), not a
// nono policy claiming a bad version. Fall through to the content check.
Some(v) if !v.is_string() => { /* fall through to the `None` content-based arm */ }
```
and downgrade the malformed-JSON case at `:63-65` to a warn-and-skip unless the content
is nono-shaped (see CR-06's `looks_like_nono` helper — the two findings share one fix).

---

### WR-15 (new): three supervisor exit paths still return without draining queued network notifications

**File:** `crates/nono-cli/src/exec_strategy.rs:3424`, `:3432`, `:3489`, `:3533`
(vs. the three drained returns at `:3466`, `:3503`, `:3515`)

**Issue:** The CR-03 fix added the drain to the reaper path. Three other exits from
`run_supervisor_loop` still skip it:

```rust
// :3424 — PTY poll teardown
if !handle_pty_poll_events(...) { break; }
// :3432 — non-EINTR poll() error
warn!("poll() error in supervisor loop: {}", err); break;
// ...
// :3489 — startup-timeout SIGKILL path
return Ok((wait_for_child(child)?, denials, ipc_denials));   // no drain
// ...
// :3533 — both `break`s land here
let status = wait_for_child(child)?;
Ok((status, denials, ipc_denials))                            // no drain
```

The startup-timeout path (`:3489`) is the one that matters: nono has just SIGKILLed the
child *because it misbehaved*, which is precisely the run whose queued network denials are
most worth recording — and they are dropped. The prior review's resolution note called
these "pre-existing behaviour on the abnormal-teardown path"; the startup-timeout return
at `:3489` is not pre-existing teardown, it is a first-class deliberate exit.

The correct shape is the one the fix declined to build: a single drained exit.

**Fix:**

```rust
macro_rules! drained_return {
    ($status:expr) => {{
        drain_pending_network_notifications(
            proxy_notify_raw_fd, config, &mut rate_limiter, &mut denials, &mut ipc_denials);
        return Ok(($status, denials, ipc_denials));
    }};
}
```
Use it at all four `return` sites, and drain once more before the trailing
`wait_for_child(child)?` at `:3533` so the `break` paths are covered too.

---

### WR-01 (carried forward): `waitpid(-1)` can steal the exit status of unrelated `std::process::Child` handles

**File:** `crates/nono-cli/src/exec_strategy.rs:3226-3245` (call site `:3465`)

**Issue:** Unchanged. `reap_reparented_orphans` uses `waitpid(Some(Pid::from_raw(-1)),
WNOHANG)`, which is process-global. Any `std::process::Child` alive in the same process
whose status has not been collected can be reaped out from under libstd, so the eventual
`Child::wait()` / `wait_with_output()` fails with `ECHILD` — and the freed PID can be
recycled. `hook_runtime::run_hook` still spawns a worker thread holding
`child.wait_with_output()` and returns on hook timeout while that thread is blocked, so
the orphaned waiter overlaps the supervised run.

The CR-03 fix made this *more* consequential, not less: `reap_reparented_orphans` is now
established as the primary exit-detection path, so any status it steals is a status the
tracked-child logic never sees.

**Fix:** unchanged — track reapable pids, or centralize reaping behind one owner:

```rust
fn reap_reparented_orphans(child: Pid, tracked: &HashSet<Pid>) -> Option<WaitStatus> {
    // skip (and do not consume) statuses for pids in `tracked`
}
```

---

### WR-02 (carried forward): `discard_late_terminal_input` consumes and discards a byte of real user type-ahead

**File:** `crates/nono-cli/src/pty_proxy.rs:1565-1613` (loop `:1576-1612`, read `:1597`)

**Issue:** Unchanged. The loop `read()`s one byte, *then* feeds it to `CprReplyParse::step`.
When `step` returns `false` the byte is already consumed and is dropped. With no CPR reply
pending, the first byte the user typed ahead is always lost — and the phase's own test
(`discard_late_terminal_input_preserves_type_ahead_with_no_reply`, `:2289`) encodes that
loss as expected behaviour (`b"xRy"` in, `b"Ry"` out).

**Fix:** unchanged — `MSG_PEEK` before consuming, or bail out unless the first readable
byte is `ESC`.

---

### WR-03 (carried forward): `discard_late_terminal_input` aborts on `EINTR` in `poll()` — the common case at teardown

**File:** `crates/nono-cli/src/pty_proxy.rs:1588-1593`

**Issue:** Unchanged. The `read()` arm retries `ErrorKind::Interrupted` (`:1607`); the
`poll()` arm does not — `if ready <= 0 ... break` cannot distinguish EINTR from timeout.
This runs on the final teardown path immediately after the child exits, when SIGCHLD (and
SIGWINCH on resize) is most likely to interrupt the 30 ms poll, so the drain aborts early
and the CPR reply lands in the shell prompt — intermittently reproducing the exact bug
this code exists to fix.

**Fix:** unchanged — `continue` on EINTR (the deadline still bounds the loop), `break`
only on a real error or timeout.

---

### WR-04 (carried forward): the `--no-auth ⇒ loopback` fail-secure guard lives only in the CLI, not in the proxy crate

**File:** `crates/nono-cli/src/proxy_command.rs:36-42`; `crates/nono-proxy/src/server.rs:496-520`;
`crates/nono-proxy/src/config.rs:98-125`

**Issue:** Unchanged, and re-verified: `server::start` (`:496`) validates `no_proxy`
(`:501`) and route conflicts (`:533`) but never checks the `require_auth` / `bind_addr`
pairing. `ProxyConfig { require_auth: false, bind_addr: <non-loopback> }` is rejected only
by `run_proxy`'s guard. Both fields are `Deserialize` (`require_auth` via
`default_require_auth`, `strict_connect_auth` via `#[serde(default)]`), so any
`ProxyConfig` deserialization — or any external embedder of the published `nono-proxy`
crate — can stand up an unauthenticated proxy reachable from the network with credential
injection routes attached. Defense in depth requires the invariant at the enforcement
boundary.

**Fix:** unchanged — re-assert it at the top of `server::start`, next to
`validate_no_proxy_config`:

```rust
if !config.require_auth && !config.bind_addr.is_loopback() {
    return Err(ProxyError::Config(
        "require_auth=false requires a loopback bind_addr; refusing to start an \
         open proxy reachable from other hosts".into()));
}
```

---

### WR-05 (carried forward): non-loopback `--listen` exposes the session token and injected credentials over plaintext HTTP

**File:** `crates/nono-cli/src/cli.rs:2338-2345`; `crates/nono-cli/src/proxy_command.rs:245-265`

**Issue:** Unchanged. With auth enabled, `--listen 0.0.0.0` is permitted. The proxy speaks
plain HTTP: the session token traverses the network in `Proxy-Authorization`, and
credential-injection routes mint real upstream secrets for whoever presents it. No TLS, no
per-source-IP restriction, no failed-auth rate limiting or lockout. The command also
prints `NONO_PROXY_TOKEN=<token>` to stderr (`proxy_command.rs:255`), which lands in CI
logs by default.

**Fix:** unchanged — add `--allow-remote` as an explicit opt-in for non-loopback binds,
reword the help (see IN-03), and gate credential-injection routes off (or warn loudly)
when `bind_addr` is not loopback.

---

### WR-06 (carried forward): `--allow-gpu` makes seccomp user-notification mandatory and fatal on every host

**File:** `crates/nono-cli/src/sandbox_prepare.rs:837`;
`crates/nono-cli/src/command_runtime.rs:374-381`;
`crates/nono-cli/src/execution_runtime.rs:479-487`;
`crates/nono/src/sandbox/linux.rs:606-625`

**Issue:** Unchanged. `proc_comm_notify: args.allow_gpu` (`sandbox_prepare.rs:837`) is set
unconditionally, regardless of whether NVIDIA hardware is present. Combined with the
(otherwise correct) change making seccomp-notify setup fatal, `--allow-gpu` now hard-fails
on WSL2 (`execution_runtime.rs:480`), and `nono wrap --allow-gpu` is a hard error outright
(`command_runtime.rs:375`). Meanwhile the Landlock grant for `/proc/self/task` was
downgraded to `Read` (`linux.rs:624`), so the previously working non-supervisor path is
gone.

The asymmetry is stark and the fix is already available in-tree: `collect_linux_gpu_paths`
computes `nvidia_present` and gates the procfs grants on it (`linux.rs:612`), but
`proc_comm_notify` is gated on the *flag*.

**Fix:** unchanged — gate on hardware presence, and add a release note that
`nono wrap --allow-gpu` is no longer supported on Linux:

```rust
#[cfg(target_os = "linux")]
proc_comm_notify: args.allow_gpu && nono::sandbox::nvidia_devices_present(),
```

---

### WR-07 (carried forward): new `unsafe` blocks lack the mandated `// SAFETY:` comments

**File:** `crates/nono-cli/src/exec_strategy.rs:1066`; `crates/nono/src/sandbox/linux.rs:5504`

**Issue:** Unchanged. CLAUDE.md: *"Unsafe Code: Restrict to FFI; must be wrapped in safe
APIs with `// SAFETY:` docs."*

```rust
// exec_strategy.rs:1066 — no SAFETY comment
let ret = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };
```
```rust
// linux.rs:5504 — no SAFETY comment; also fork() from the multi-threaded test harness
let pid = unsafe { libc::fork() };
```

Both files carry `// SAFETY:` comments on neighbouring blocks (`exec_strategy.rs:1075`,
`linux.rs:1842`, `:3577`, `:3581`), so this is inconsistent as well as non-conforming.

**Fix:** unchanged — add the justifications.

---

### WR-08 (carried forward): `restrict_execute()`'s `Refer`-on-`/` grant is safe only because of an unenforced ordering precondition

**File:** `crates/nono/src/sandbox/linux.rs:1283-1360` (signature at `:1297`,
doc claim at `:1291-1293`)

**Issue:** Unchanged. The layer does `handle_access(AccessFs::Refer)` +
`add_rule(PathBeneath::new("/", Refer))`, and the doc asserts *"Bare `Refer` alone can't
widen access."* That holds only because Landlock layers intersect **and** the first layer
(`apply`/`apply_with_abi`) also handles `Refer` and grants it narrowly. `restrict_execute`
is `pub` and its doc only says "Call this after `apply()`" — nothing enforces it. Called
standalone (or on an ABI where the earlier layer's `Refer` was dropped under
`CompatLevel::BestEffort`), this layer imposes **no** rename/link restriction anywhere
while presenting as a restriction layer.

**Fix:** unchanged — take the prior `DetectedAbi`/`RulesetStatus` as a parameter, or
document it as a hard precondition and assert a prior nono layer is active.

---

### WR-09 (carried forward): `ctrl_c()` future is recreated every `select!` iteration and can miss a SIGINT

**File:** `crates/nono-cli/src/proxy_command.rs:217-229`

**Issue:** Unchanged.

```rust
loop {
    tokio::select! {
        signal = tokio::signal::ctrl_c() => { ... break; }
        _ = audit_drain.tick() => { let _ = handle.drain_audit_events(); }
    }
}
```

`tokio::signal::ctrl_c()` constructs a fresh listener each iteration. A SIGINT delivered
after the previous future is dropped and before the new one registers is not observed —
and because tokio has replaced the default SIGINT disposition, the process does not die
either. The user must press Ctrl-C again. The window recurs on every 30 s tick.

**Fix:** unchanged — bind the stream once outside the loop
(`tokio::signal::unix::signal(SignalKind::interrupt())`, or `tokio::pin!` the future).

---

### WR-10 (carried forward): breaking public-API changes to the core `nono` crate with an inert deprecation shim

**File:** `crates/nono/src/trust/types.rs:16-20`, `:42-49`; `crates/nono/src/trust/mod.rs:64-67`

**Issue:** Unchanged, and re-verified against the current re-export list. `types.rs:20`
declares:

```rust
#[deprecated(since = "0.66.0", note = "... will be removed in v1.0.0")]
pub const TRUST_POLICY_VERSION: u32 = 1;
```

but `trust/mod.rs:64-67` re-exports only
`{BlockedPublisher, Blocklist, BlocklistEntry, Enforcement, IncludePatterns, Publisher,
SignerIdentity, TrustPolicy, VerificationOutcome, VerificationResult,
TRUST_POLICY_PREDICATE}` — `TRUST_POLICY_VERSION` is absent. `types` is a private module,
so the constant is unreachable from outside the crate: the deprecation can never be
observed by any downstream consumer. It was removed, not deprecated.

Alongside it, `TrustPolicy.version` changed `u32` → `Option<u32>` and a new
`predicate: Option<String>` field was added, breaking every external struct-literal
construction. Per MEMORY, `nono-py` and `nono-ts` are separate repos that have
historically broken on exactly this kind of core-crate struct drift.

**Fix:** unchanged — `#[allow(deprecated)] pub use types::TRUST_POLICY_VERSION;` for one
release, note the `TrustPolicy` field changes in the changelog, and rebuild
`nono-py`/`nono-ts` against the new shape.

---

### WR-12 (carried forward): the standalone proxy discards its entire network audit buffer

**File:** `crates/nono-cli/src/proxy_command.rs:211-229` (drop at `:226`)

**Issue:** Unchanged.

```rust
_ = audit_drain.tick() => {
    let _ = handle.drain_audit_events();
}
```

`nono proxy` is by construction the *only* enforcement boundary in its mode — there is no
OS sandbox behind it. Every network audit event it produces (allow/deny decisions,
authentication failures, credential-route usage) is drained and dropped to keep the ring
buffer from filling. The one mode with no kernel enforcement has zero auditability. The
module doc (`:5-9`) frames this as intentional, but "nothing else consumes it" is a gap to
close, not a justification — the CR-03 fix in this same phase was raised precisely because
losing denial telemetry is unacceptable in a security tool.

**Fix:** unchanged — at minimum emit drained events through `tracing` at `info!` on the
`nono_security` target, and offer `--audit-log <PATH>`.

## Info

### IN-01 (carried forward): `--no-auth` on loopback is still an open proxy for every local user

**File:** `crates/nono-cli/src/proxy_command.rs:36-42`, `:256-265`

**Issue:** Unchanged. The guard limits `--no-auth` to loopback, but loopback is reachable
by every local user and every container sharing the network namespace. In `--no-auth`
mode any of them can drive the credential-injection routes and obtain real upstream
secrets (`reverse.rs:210-215` skips auth wholesale). The banner at `:257-260` warns about
the missing token but not about the local multi-user exposure.

**Fix:** extend the warning — *"every local process, including other users on this host,
can use this proxy and any credentials it injects"* — and consider `SO_PEERCRED` uid
matching when `require_auth` is false.

---

### IN-02 (carried forward): `NONO_PROXY_MAX_CONNECTIONS=0` silently disables the connection ceiling

**File:** `crates/nono-cli/src/cli.rs:2356-2366`

**Issue:** Unchanged. `max_connections` is env-settable and `0` means unlimited per
`ProxyConfig.max_connections`'s doc (`config.rs:85-87`). An ambient env var removes the
DoS ceiling without appearing on the command line.

**Fix:** reject `0` from the env path, or require an explicit `--max-connections 0`.

---

### IN-03 (carried forward): `--listen` help text is self-contradictory

**File:** `crates/nono-cli/src/cli.rs:2338`

**Issue:** Unchanged, verbatim: `"Address the proxy listens on (loopback only unless
--no-auth is omitted)"`. The double negative inverts on a quick read; the actual rule is
"non-loopback is allowed only when `--no-auth` is *not* used".

**Fix:** `"Address the proxy listens on. --no-auth requires a loopback address."`

---

### IN-04 (carried forward): `test_restrict_execute_does_not_break_rename_into_new_subdir` uses a predictable temp path and raw `fork()`

**File:** `crates/nono/src/sandbox/linux.rs:5496-5546`

**Issue:** Unchanged. The test builds
`std::env::temp_dir().join(format!("nono-restrict-execute-rename-test-{}", std::process::id()))`
(`:5496-5499`) — a predictable name in a world-writable directory — then `create_dir_all`
+ `write` into it (pre-existing-symlink hazard on a shared host), and calls `libc::fork()`
(`:5504`) from the multi-threaded cargo test harness. It also asserts hard on
`apply()`/`restrict_execute()` success (exit codes 2/3, `:5540-5545`), so it fails rather
than skips in CI sandboxes where Landlock reports an ABI but `restrict_self` is blocked.

**Fix:** use `tempfile::tempdir()` for the root; treat exit codes 2/3 as skip.

---

### IN-05 (carried forward): `socket_access_run.rs` allowlist assertion was materially weakened

**File:** `crates/nono-cli/tests/socket_access_run.rs:173-192`

**Issue:** Unchanged. The assertion is `!stderr.contains(&socket_arg)`. `socket_arg` is a
full path that can appear in stderr for reasons unrelated to a denial (echoed in a
diagnostic hint, in an error message, in the profile-load trace), so the test can pass on
a genuine regression *and* fail spuriously. Note also the host caveat recorded in
`112-HUMAN-UAT.md`: every test in this file short-circuits on `python3_available()`, so
this assertion has never actually executed locally.

**Fix:** assert against the formatter's denial block — that no line matching
`^\s*{socket_arg}$` appears within the `[nono] IPC denial:` block.

---

### IN-06 (new): proxy connection URLs are string-concatenated and break for IPv6 `--listen`

**File:** `crates/nono-cli/src/proxy_command.rs:245-265`

**Issue:** `print_connection_info` builds the connection strings by interpolating
`args.listen` directly:

```rust
eprintln!("  [nono] HTTPS_PROXY=http://nono:{}@{}:{}", &*handle.token, args.listen, handle.port);
```

`args.listen` is an `IpAddr`, so `--listen ::1` (which `is_loopback()` accepts, and which
the `--no-auth` guard therefore permits) prints
`http://nono:<tok>@::1:8080` — an unparseable URL. RFC 3986 requires bracketed IPv6
literals in authority position. The user copies a string that no HTTP client accepts.

**Fix:** format the authority once, bracketing IPv6:

```rust
let authority = match args.listen {
    std::net::IpAddr::V6(a) => format!("[{a}]:{}", handle.port),
    std::net::IpAddr::V4(a) => format!("{a}:{}", handle.port),
};
```

---

### IN-07 (new): the proc-comm fast path produces no denial record or user-visible diagnostic on failure

**File:** `crates/nono-cli/src/exec_strategy/supervisor_linux.rs:357-387` (error arm `:376-384`)

**Issue:** The new NVIDIA `/proc/<tgid>/task/<tid>/comm` mediation is correctly scoped —
it runs *after* the protected-roots check (`:319-348`), validates the tgid component
against the notifying process's own tgid (`:357-359`), restricts to write-capable opens
(`:358`), and re-checks `notif_id_valid` before `inject_fd` (`:365`), which is the right
TOCTOU ordering. No security defect found.

The gap is observability. When the supervisor's own open fails it responds with an errno
and logs at `debug!` only:

```rust
Err(e) => {
    debug!("Failed to open proc comm path {} for writing: {}", ...);
    let _ = respond_notif_errno(notify_fd, notif.id, e.raw_os_error().unwrap_or(libc::EIO));
}
```

No `record_denial(...)` call, so the failure never reaches the denial diagnostic footer or
the session denial record — every other rejection path in this function records one. A
user whose CUDA init fails because of this gets no `[nono]` output at default verbosity.
Also, `is_proc_task_comm_for_tgid` runs against `try_canonicalize(&resolved_path)`, which
falls back to the uncanonicalized path when canonicalization fails; the numeric-component
grammar makes that safe today, but the fallback is undocumented at this call site.

**Fix:** record a `DenialRecord { path, access, reason: DenialReason::PolicyBlocked }` (or
a dedicated reason) in the `Err` arm, and add a one-line comment noting that the
component grammar is what makes the `try_canonicalize` fallback safe.

---

## Previously Resolved

Recorded for continuity. Independent verdicts below were reached by reading current
source, not the resolution notes.

### CR-01 — `nono proxy` silently discards the profile's deny layer, `no_proxy`, and `network.block`
**Fix commit:** `916d3bd7` (`proxy_command.rs`, `cli.rs`)
**Verdict: confirmed fixed.**

Traced end-to-end rather than to `ProxyLaunchOptions`:

- `build_launch_options` merges `net.deny_domain` / `net.no_proxy` / `net.block`
  (`proxy_command.rs:109-111`) into `ProxyLaunchOptions` (`:171-196`).
- `run_proxy` calls `proxy_runtime::build_proxy_config_from_flags(&launch_options)`
  (`proxy_command.rs:45`), which expands deny entries via
  `network_policy::expand_proxy_deny` (`proxy_runtime.rs:407`) into
  `ProxyConfig.denied_hosts` (`:410`), ORs `strict_filter` without narrowing (`:415`), and
  assigns `proxy_config.no_proxy` (`:424`).
- `nono_proxy::server::start` consumes all three: `ProxyFilter::new_strict` /
  `allow_all` / `new` selected by `strict_filter`, then `.with_denied_hosts(&config.denied_hosts)`
  (`server.rs:547-554`); `validate_no_proxy_config(&config)` (`:501`) and
  `validate_no_proxy_route_conflicts` (`:533`) re-validate `no_proxy` fail-closed.
- The post-`build_proxy_config_from_flags` overrides in `run_proxy` (`:50-54`) touch only
  `bind_addr`, `bind_port`, `max_connections`, `require_auth`, `strict_connect_auth` —
  none of the deny fields are clobbered.

The `--deny-domain` flag exists (`cli.rs:2401-2414`) with `env = "NONO_DENY_DOMAIN"`,
matching the `nono run` flag at `cli.rs:2009-2015`. The deny-without-allow guard
(`proxy_command.rs:139-149`) fires from both the CLI flag and the profile, matching
`sandbox_prepare::validate_deny_domain_requires_allow_domain` (`:306-328`).

**Bare `nono proxy` with no profile and no flags:** `allowed_hosts` empty,
`strict_filter` false, `denied_hosts` empty → `ProxyFilter::allow_all()`. It is still an
open forwarding proxy for any host, now with an explicit stderr warning
(`proxy_command.rs:271-277`) but no behavioural change. Auth is required by default
(`require_auth = !args.no_auth`, `strict_connect_auth` likewise), so it is
token-gated — the residual exposure is the one covered by WR-05 (network-reachable binds)
and IN-01 (local multi-user). Accepted as a documented residual, not re-opened.

### CR-02 — Trust-policy `predicate` migration is fail-open
**Fix commit:** `892d2e01` (`trust_scan.rs`, `trust_cmd.rs`)
**Verdict: partially fixed.**

The headline hole is genuinely closed. Both merge sites were restructured so a project
file can no longer short-circuit user-policy loading, regardless of its content:

- `trust_cmd::load_trust_policy:1594-1638` — project and user loaded independently, then
  combined in a single `(user_policy, project_policy)` match. No early `return` on the
  project arm.
- `trust_scan::load_scan_policy:158-198` — same shape, already correct before the fix and
  unchanged.

Peek-vs-full-parse differential: **no differential found** for the accepting arm.
`serde_json::Map` and `serde` both keep the last occurrence on duplicate keys, so the peek
and the full parse agree on which `predicate` wins. The non-string case
(`{"predicate": 1}`) that `as_str()` used to collapse into "absent" is now handled
explicitly (`trust_scan.rs:87-93`). `sanitize_untrusted` (`:21-23`) strips control
characters from the echoed value.

**But** two defects remain in the fixed code, both reported above as new findings:
- **CR-06** — the `Err(_)` arm of the legacy content check discards the deserialization
  error, so a malformed legacy nono policy (including the operator's user-level one) is
  silently reclassified as foreign and skipped. A fail-open of the same class the fix was
  written to remove.
- **WR-14** — the non-string-predicate hard error turns any in-toto-shaped
  `trust-policy.json` in the working directory into a hard abort of `nono run`.

And **CR-05** (project policies can add trusted publishers) is the untouched mirror image
of CR-02's own threat model.

`TrustPolicy::validate_version` (`types.rs:110-117`) was correctly *not* tightened to
require `predicate.is_some()` — doing so would reject the legacy policies the fix exists
to keep loading, since `load_policy_from_str` calls it.

### CR-03 — `reap_reparented_orphans` early-return skips the final network-notification drain
**Fix commit:** `585f2743` (`exec_strategy.rs`)
**Verdict: confirmed fixed for the path reported; incomplete overall.**

`drain_pending_network_notifications(...)` is now called before the reaper path's `return`
(`exec_strategy.rs:3466-3472`), matching the `Ok(status)` arm (`:3503`) and the `ECHILD`
arm (`:3515`). The reaper is still the first thing in the loop tail (`:3465`), before
`waitpid(child, WNOHANG)` (`:3476`), so the normal exit path is the drained one. The
reaper's own semantics are correct — `waitpid(-1, WNOHANG)` only returns terminated
children, `Ok(_)` (StillAlive) returns `None`, EINTR retries, ECHILD returns `None`.

Three sibling exits still return undrained: the two `break` paths (`:3424`, `:3432`) which
fall through to `wait_for_child` at `:3533`, and the startup-timeout SIGKILL return at
`:3489`. The prior resolution note deferred these as "pre-existing abnormal teardown"; the
startup-timeout return is not — see **WR-15**.

### CR-04 — `SeccompOpts::external_tcp()` is a fail-open network escape hatch
**Fix commit:** `20f9a1cd` (`sandbox/linux.rs`, `sandbox/mod.rs`)
**Verdict: confirmed fixed.**

The guard cannot be bypassed and runs before any kernel state is touched:

- `validate_external_tcp_delegation(caps)?` is the **first** statement in
  `apply_with_abi_inner` (`linux.rs:723-725`) — before `requested_scopes`, before
  `Ruleset::default()`, before `create()`, before `restrict_self()`.
- The only two public entry points that can set `handle_tcp = false` are `apply_seccomp`
  (`:1249`) and `apply_seccomp_with_abi` (`:1256`), both routing through
  `apply_with_abi_inner`. `SeccompOpts.handle_tcp` is a private field (`:1157`) with no
  public setter, so no external caller can construct a bypassing value.
- The guard covers all four network-policy inputs — `network_mode`, `tcp_connect_ports`,
  `tcp_bind_ports`, `localhost_ports` (`:1221-1224`) — so `SeccompNetFallback::None` is
  truthful on that path and the "cannot distinguish enforced from unenforced" ambiguity is
  resolved without adding a public enum variant (correctly avoiding a breaking change for
  `nono-py`/`nono-ts`).
- The `handle_tcp &&` gate on the `open_port 0` fail-closed check was removed (`:736`), so
  that validation applies on every Linux path again.
- `apply_external`'s doc no longer claims to record a marker (`linux.rs:1274-1277`,
  `sandbox/mod.rs:719-730`).

Residual (not re-opened as a finding, noted for the record): `apply_external()` and the
whole `apply_seccomp*` surface still have **zero production callers** — only the
declaration sites, the `lib.rs`/`mod.rs` re-exports, and the five new unit tests. It is
retained because `112-02-PLAN.md`'s must_haves require the entry points to exist. If it
still has no consumer by the next milestone, removal should be re-proposed as a scoped
decision (CLAUDE.md: *"If code is unused, either remove it or write tests that use it"* —
the fix satisfies the second clause).

---

_Reviewed: 2026-08-05 (pass 2)_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

_Pass 1 (2026-08-05): 4 Critical, 12 Warning, 5 Info. Criticals fixed in `916d3bd7`,_
_`892d2e01`, `585f2743`, `20f9a1cd`._
_Pass 2 (this document): CR-01/CR-03/CR-04 confirmed fixed; CR-02 partially fixed._
_WR-11 closed. 11 Warnings + 5 Info carried forward. New: CR-05, CR-06, WR-13, WR-14,_
_WR-15, IN-06, IN-07._
