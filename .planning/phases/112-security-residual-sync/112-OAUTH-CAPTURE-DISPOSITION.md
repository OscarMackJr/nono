# Phase 112: SEC-02a/b/c Reality-Check Evidence Capture + RES-01 Skip Documentation

**Written:** 2026-08-05 (Plan 112-01, Task 3)
**Status:** SEC-02a/b/c evidence captured, disposition recorded as deferred -> Phase 114 (not an
adopt/adapt/decline verdict). RES-01's 4 SHAs recorded skip-with-reasoning per D-03.
**Phase:** 112-security-residual-sync

**Scope note:** per the ROADMAP Amendment (2026-08-05) — adjudicated by the operator after the
plan-checker found the original SEC-02 go/no-go framing exceeded the discretion `112-CONTEXT.md`
D-04 granted the planner — this document captures reality-check evidence for SEC-02a/b/c and
records the disposition as **deferred to Phase 114**. It does **not** make an adopt/adapt/decline
call. That call belongs to Phase 114, mirroring the NET-02/SPIFFE precedent (Phase 109's finding
splitting SPIFFE work into its own Phase 113 for the identical entanglement reason).

## Part 1: SEC-02a/b/c Reality-Check Evidence

### 1a. `9b692e07` (SEC-02a) — declarative sandboxed OAuth capture, diffstat and file inventory

```
$ git show 9b692e07 --stat
commit 9b692e07ee4dd156d85588b12b63691778d9e9be
Author: Luke Hinds <lukehinds@gmail.com>
Date:   Wed Jul 1 15:07:12 2026 +0100

    feat(oauth): add declarative sandboxed OAuth capture

    Add profile-driven OAuth credential providers that let agents complete OAuth
    login flows inside the sandbox while token endpoint responses are rewritten to
    sandbox-visible phantom tokens.

    The implementation adds declarative provider config for token endpoints,
    response/request token fields, API hosts, credential-store detection, helper
    commands, and generated proxy routes. Real OAuth tokens are stored outside the
    sandbox and resolved only for admitted proxy consumers.

    Also adds fail-closed handling for unsafe token responses, bounded persisted
    phantom retention, Claude/Codex-compatible profile support, and user docs for
    sandboxed OAuth login setup and security tradeoffs.

 crates/nono-cli/data/nono-profile.schema.json      | 265 +++++++-
 crates/nono-cli/data/profile-authoring-guide.md    | 153 ++++-
 crates/nono-cli/src/launch_runtime.rs              |   6 +
 crates/nono-cli/src/main.rs                        |   4 +
 crates/nono-cli/src/policy.rs                      |   2 +
 crates/nono-cli/src/profile/credential_provider.rs | 391 +++++++++++
 crates/nono-cli/src/profile/mod.rs                 | 334 ++++++++-
 crates/nono-cli/src/profile_runtime.rs             |  87 +++
 crates/nono-cli/src/proxy_command.rs               |   6 +-
 crates/nono-cli/src/proxy_runtime.rs               | 752 ++++++++++++++++++---
 crates/nono-cli/src/sandbox_prepare.rs             |  10 +
 crates/nono-proxy/src/config.rs                    |  96 +++
 crates/nono-proxy/src/filter.rs                    |  34 +-
 crates/nono-proxy/src/forward.rs                   | 194 +++++-
 crates/nono-proxy/src/lib.rs                       |   1 +
 crates/nono-proxy/src/oauth_capture/endpoint.rs    |  84 +++
 crates/nono-proxy/src/oauth_capture/jwt.rs         |  19 +
 crates/nono-proxy/src/oauth_capture/mod.rs         | 604 +++++++++++++++++
 crates/nono-proxy/src/oauth_capture/persist.rs     | 162 +++++
 crates/nono-proxy/src/oauth_capture/rewrite.rs     | 287 ++++++++
 crates/nono-proxy/src/server.rs                    | 137 +++-
 crates/nono-proxy/src/tls_intercept/h2_forward.rs  | 124 ++++
 crates/nono-proxy/src/tls_intercept/handle.rs      |  92 ++-
 docs/cli/development/oauth-capture-manual-qa.mdx   | 103 +++
 docs/cli/features/sandboxed-oauth-logins.mdx       | 635 +++++++++++++++++
 docs/docs.json                                     |   6 +-
 26 files changed, 4425 insertions(+), 163 deletions(-)
```

**File presence against the live fork tree** (re-run 2026-08-05):

| File | Fork status |
|---|---|
| `crates/nono-proxy/src/oauth_capture/{endpoint,jwt,mod,persist,rewrite}.rs` (5 files, 1,156 lines) | **ABSENT** — `ls crates/nono-proxy/src/oauth_capture` → `No such file or directory` |
| `crates/nono-proxy/src/forward.rs` (194 modified lines) | **ABSENT** — `ls crates/nono-proxy/src/forward.rs` → `No such file or directory` |
| `crates/nono-proxy/src/tls_intercept/{h2_forward,handle}.rs` | **ABSENT** — confirmed absent per the SEC-01 finding (`112-AWS-SIGV4-PROXY-AUTH-FINDING.md`) |
| `crates/nono-cli/src/proxy_command.rs` | **ABSENT until SEC-07 lands** — this commit's 6-line edit targets the file SEC-07 (`2663e990`) creates |
| `crates/nono-cli/src/profile/credential_provider.rs` (391 lines, new) | Freestanding, no fork conflict — buildable independent of the absent subsystems |
| Remaining 17 files (schema.json, docs, `profile/mod.rs`, `profile_runtime.rs`, `proxy_runtime.rs`, `launch_runtime.rs`, `main.rs`, `policy.rs`, `sandbox_prepare.rs`, `nono-proxy/{config,filter,server,lib}.rs`) | Present in the fork |

### 1b. `forward.rs`'s ancestor commit — confirmed outside this 18-commit window

```
$ git log --oneline --all --diff-filter=A -- crates/nono-proxy/src/forward.rs
149abde0 feat(proxy): add tls interception for l7-bearing connect routes
```

`149abde0` is the sole commit that creates `forward.rs`; it is not one of the 18 SHAs in the
`security-residual-and-misc` cluster (per `108-DIVERGENCE-LEDGER.md`'s itemization) and was never
absorbed by the fork. `9b692e07`'s diff to `forward.rs` is a **modify** (`--- a/...` / `+++ b/...`,
not `/dev/null`), confirming it edits a file that would need `149abde0` absorbed first — an
out-of-window prerequisite this plan does not evaluate.

### 1c. Is the captured token stripped/rewritten specifically inside `forward.rs`'s response-rewrite hook?

**Yes — confirmed directly by reading the hook's own doc comment**, `git show
9b692e07:crates/nono-proxy/src/forward.rs`:

```rust
/// Optional HTTP/1.1 response body rewrite hook.
///
/// Used for OAuth token capture, where the proxy must buffer a token endpoint
/// response, replace real token fields with phantoms, and only then release the
/// response to the sandboxed client.
pub type ResponseRewrite<'a> =
    &'a (dyn Fn(u16, &[(String, String)], &[u8]) -> Result<Vec<u8>> + Send + Sync);
```

The doc comment is explicit and unambiguous: this is the mechanism by which a real OAuth token in
an upstream response is replaced with a sandbox-visible phantom **before** the response reaches
the sandboxed client. `forward_request_with_response_rewrite()` (also in `forward.rs`, confirmed in
the same `git show`) is the call site that threads an `Option<ResponseRewrite<'_>>` through the
forwarding path — when `None` (the fork's current, absent-subsystem baseline, since the whole file
is absent), no rewrite occurs at all; the hook is what makes rewriting happen.

**No equivalent enforcement point exists elsewhere in the fork.** The fork's `oauth2.rs` module
(573 lines, present) implements a **different** feature — `client_credentials` OAuth2 token
exchange and caching for the reverse-proxy credential-injection flow (confirmed via its own module
doc comment, `crates/nono-proxy/src/oauth2.rs:1-17`: "Provides `TokenCache`... for the reverse
proxy credential injection flow where the agent never sees the real client_id/client_secret").
That is a proxy-injects-credentials-into-outbound-requests model, not a
capture-and-rewrite-inbound-token-responses model — no naming collision, but also no reusable
mechanism. The fork's `reverse.rs`/`server.rs` response paths (present) have no response-body
rewrite hook of any kind; a response is either forwarded as-is or the connection is denied/errored.

**Security conclusion:** if SEC-02a were adapted to reach ~17 present files while dropping the
`forward.rs`-dependent rewrite hook (because `forward.rs` itself is absent), the resulting feature
would let agents complete OAuth login flows and receive **real, unrewritten tokens** rather than
sandbox-visible phantoms — the exact property this evidence confirms the hook exists to prevent. A
reduced-scope absorb without an equivalent enforcement point would ship an unsafe half-feature.

### 1d. `3c59c62e` (SEC-02b) and `d033c631` (SEC-02c) — evidence

```
$ git show 3c59c62e --stat
    fix(oauth): harden capture security boundaries

 crates/nono-cli/src/profile_runtime.rs         | 188 ++++++++++++++++++++++++-
 crates/nono-proxy/src/forward.rs               |  32 ++++-
 crates/nono-proxy/src/oauth_capture/mod.rs     |  32 ++++-
 crates/nono-proxy/src/oauth_capture/persist.rs |  22 ++-
 4 files changed, 262 insertions(+), 12 deletions(-)

$ git show d033c631 --stat
    test(oauth): consume provider stdin in header fixture

 crates/nono-cli/src/proxy_runtime.rs | 5 ++++-
 1 file changed, 4 insertions(+), 1 deletion(-)
```

`3c59c62e` is a **security-hardening follow-up commit** to `9b692e07` — 3 of its 4 touched files
(`forward.rs`, `oauth_capture/mod.rs`, `oauth_capture/persist.rs`) are inside the absent SEC-02a
subsystem; its title ("harden capture security boundaries") makes it a strengthening of the exact
capture-and-rewrite flow evidenced above, not a separable feature.

`d033c631` is a single-hunk fix inside a test fixture (`proxy_runtime.rs`) that only exists because
`9b692e07`'s provider stdin-handling code created it.

Neither commit can be meaningfully evaluated independent of `9b692e07`'s disposition.

## Disposition: SEC-02a/b/c — deferred -> Phase 114

**`9b692e07` (SEC-02a), `3c59c62e` (SEC-02b), and `d033c631` (SEC-02c) are recorded as deferred ->
Phase 114.** This is not an adopt/adapt/decline verdict — Phase 114 makes that call against the
evidence captured above. The authority and reasoning for this carve-out:

**(a) Reality-check evidence gathered above:** ~9 of 26 `9b692e07` files depend on subsystems
absent from the fork (`oauth_capture/` core — 1,156 lines; `tls_intercept/{h2_forward,handle}.rs`;
`forward.rs` — created by unabsorbed ancestor `149abde0`); the remaining ~17 files are present and
could in principle host an adapted version, but only if an equivalent enforcement point for the
security property in (b) is designed first.

**(b) Security reasoning:** `forward.rs`'s `ResponseRewrite` hook is what keeps real OAuth tokens
out of the sandboxed client — its own doc comment says so explicitly, and no equivalent mechanism
exists elsewhere in the fork's proxy response path. A reduced-scope absorb that drops this hook
(because its host file is absent) would ship a feature that lets agents complete OAuth flows while
receiving real, unrewritten tokens — an unsafe half-feature, not a smaller-but-still-safe version
of the same capability. This is exactly the kind of judgment call `112-CONTEXT.md` D-05 names as
the contingent ADR-112 trigger, but resolving it (design an equivalent enforcement point, or decide
the feature isn't safely adaptable without `forward.rs`) is a design decision this plan is
explicitly scoped not to make.

**(c) ROADMAP Amendment (2026-08-05) authority:** the operator adjudicated a carve-out of SEC-02
into its own Phase 114 after the plan-checker found the original SEC-02 go/no-go framing exceeded
the discretion `112-CONTEXT.md` D-04 granted the planner (D-04 explicitly rejected a follow-on
split for the phase in general, but the operator's adjudication is a phase-boundary decision made
at execution-adjacent planning time, not the planner exceeding its own discretion). This mirrors
the NET-02/SPIFFE precedent: Phase 109 found SPIFFE (#1272) entangled with the fork-divergent
`tls_intercept.rs` in the identical shape — absent-subsystem entanglement discovered during a
reality-check pass — and split it into its own Phase 113 rather than forcing an in-phase
adapt-or-decline call. SEC-02a/b/c's carve-out to Phase 114 is the same move for the same reason.

**No partial code absorb was attempted for SEC-02a/b/c in this plan.** No source file under
`crates/` was created or modified by this task.

## Part 2: RES-01 Skip Documentation (D-03)

D-03 (LOCKED) sets RES-01 and RES-02 skip-biased by default: skip-with-recorded-reasoning unless a
task diff-proves the change applies to the fork's divergent implementation. RES-01's rationale is
that the registry/update-check surface collides with the fork's own HKLM machine-policy spine and
divergent update behavior. All 4 RES-01 SHAs are recorded below with written reasoning — none is
silently dropped.

### `f0506434` — skip, with recorded reasoning

24 files, +105/-102. Top-level org-URL references (`README.md`, `setup.rs`) already point at
`nolabs-ai/nono` from a prior phase's own update, unrelated to this commit. The commit's actual
content is a legacy-pack-migration UI (`migration.rs`, `legacy_cleanup.rs`, `pull_ui.rs` — all 3
absent from the fork). **Skip reasoning:** the fork's registry/update-check behavior already
diverges structurally from upstream's via the machine-policy spine:

```
$ grep -rln HKLM crates/nono-cli/src/*.rs
crates/nono-cli/src/cli_bootstrap.rs
crates/nono-cli/src/health.rs
crates/nono-cli/src/main.rs
crates/nono-cli/src/platform.rs
crates/nono-cli/src/policy.rs
```

5 files carry `HKLM\SOFTWARE\Policies\nono` machine-policy integration upstream has no equivalent
of (v3.0's enterprise policy spine). Absorbing upstream's legacy-pack-migration UI risks colliding
with that divergent surface for a UI upstream itself is retiring, not the fork's own migration path.

**Flagged-but-not-acted-on finding (UX-consistency, not security-relevant, out of scope per D-03's
skip bias):** this commit also touches the *pack registry namespace* prefix (a different string
than the org-URL migration already applied), and that prefix is stale:

```
$ grep -rn "always-further" crates/nono-cli/src/cli.rs crates/nono-cli/src/policy.rs \
    crates/nono-cli/src/profile/builtin.rs crates/nono-cli/data/policy.json
```

16 hits across `cli.rs` help-text examples, `policy.json`'s alias map
(`"always-further/claude": "claude-code"`), `policy.rs`, and `profile/builtin.rs` — all still
reference the `always-further/*` pack-registry-namespace prefix. This means help text may tell
users to run `nono pull always-further/claude`, which may not resolve against upstream's actual
current `nolabs-ai/*` pack registry. This is a real, provable staleness — but it is a UX-consistency
issue (stale help text / alias map), not a security-relevant gap, so D-03's skip bias applies as
the default disposition here too. Flagged for a future low-priority adapt task; not acted on in
this phase.

### `7fe0c828` — skip, with recorded reasoning

5 files, +81/-32. Adds an `X-Nono-Pull-Reason` header to registry pull requests — pure
telemetry/diagnostics for upstream's own registry service. **Skip reasoning:** same HKLM-spine
collision rationale as `f0506434` (grep evidence above applies identically — the fork's
registry/update-check behavior is already divergent). No security or correctness gap identified;
this is upstream-service-facing telemetry the fork's divergent update path has no analogous need
for.

### `0158d52f` — skip, with recorded reasoning

2 files, +175/-2. Attaches installation-context headers (`registry_client.rs`, `update_check.rs`)
to registry requests. **Skip reasoning:** same HKLM-spine collision rationale. Additive telemetry
to an upstream-service-facing client; no fork-side bug or security gap this closes.

### `762eb05b` — skip, with recorded reasoning

3 files, +87/-6. Emits `install_source` on update-check requests (`update_check.rs`,
`build.rs`), and touches `.github/workflows/release.yml`. **Skip reasoning:** same class of
telemetry as `0158d52f`, plus an additional reason specific to this commit — `release.yml` has
already been heavily customized by the fork for Trusted Signing (v3.5); absorbing an upstream
`release.yml` hunk verbatim for a telemetry field carries out-of-proportion regression risk against
that customization, on top of the general HKLM-spine collision rationale.

## Summary

- **SEC-02a/b/c:** evidence captured, disposition recorded as **deferred -> Phase 114**. Not
  silently dropped; not an adopt/adapt/decline call made in this plan.
- **RES-01 (4 SHAs):** all recorded **skip, with reasoning** per D-03, with one flagged-but-not-
  acted-on UX-staleness finding for `f0506434` noted but out of scope.
