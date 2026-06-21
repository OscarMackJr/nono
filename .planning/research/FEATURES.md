# Feature Research

**Domain:** Signed policy-exception management for a capability-based OS sandbox (break-glass / signed override system)
**Researched:** 2026-06-21
**Confidence:** HIGH — grounded in the existing nono codebase, the ZT-Infra v2 source, and established patterns from PAM/AWS SCP/RBAC exception workflows; no WebSearch inference required

---

## Scope Reminder

This file covers ONLY the NEW signed-override feature set added in v3.2.
The following are already built and must NOT be re-researched or re-planned:

- nono OS confinement (`confined_run`/`confine` in nono-py, CapabilitySet, Landlock/Seatbelt/AppContainer)
- Group/deny policy resolver (`crates/nono-cli/src/policy.rs`)
- `sigstore-rs` attestation primitives (`crates/nono/src/trust/bundle.rs`)
- SecurityEventLayer HMAC-chained telemetry (EventIDs 10001-10005, tracing::Layer, agent daemon wiring)
- ZT-Infra v2 `POST /actions` allow/deny decision contract + hash-chained KMS-signed audit + DAAL ledger

---

## Feature Landscape

### Category A: Exception Token Format

The signed token is the load-bearing artifact. Everything else (verification, runtime mutation, revocation, audit) hangs off what the token contains. Get the format wrong and every downstream feature is broken.

#### Table Stakes

| Feature | Why Expected | Complexity | Existing Dependency | Notes |
|---------|--------------|------------|---------------------|-------|
| Signer identity field | Auditors must know who authorized the override — "manager approval" is the non-self-service invariant | LOW | sigstore-rs identity certificates; ZT-Infra `actor` field | Use the existing sigstore certificate-identity model (`CertificateInfo`); do not roll a custom signer-identity scheme |
| Scope field: allowed paths and/or network domains | The override must expand exactly the blocked resource, nothing more | MEDIUM | `CapabilitySet` / `FsCapability` / `AccessMode` | Scope entries must be path-canonicalized at token-creation time, not at verification time — prevents TOCTOU scope-creep |
| Expiry timestamp (absolute UTC, not relative duration) | Without expiry, a granted exception becomes a permanent backdoor | LOW | — | Absolute UTC avoids clock-skew ambiguity across hosts; must be checked at verification time against the host clock with a max-skew tolerance |
| Repo-context binding | Same token must not apply to a different repository — prevents lateral movement via token sharing | MEDIUM | — | Bind to canonical repo root path (or a stable repo identifier such as git remote URL + HEAD commit hash). Verification checks the current working context against the binding |
| Override ID (UUID or random 128-bit) | Required for revocation lookup and audit correlation; also replay-prevention anchor | LOW | — | Generate at issuance time; include in every audit event |
| Token format: signed JSON envelope | Operator-readable, loggable, compatible with existing NDJSON audit infrastructure | MEDIUM | `AuditRecorder` NDJSON; sigstore DSSE envelope shape | Use sigstore DSSE (Dead Simple Signing Envelope) or a stripped version of it; reuse the existing `trust::bundle` signing surface |

#### Differentiators

| Feature | Value Proposition | Complexity | Existing Dependency | Notes |
|---------|-------------------|------------|---------------------|-------|
| Single-use vs reusable flag in token | Allow approvers to issue one-shot emergency tokens vs. limited-window reusable tokens for CI pipelines | MEDIUM | Override ID for replay tracking | A single-use token is consumed on first verified use; a reusable token is valid until expiry. Single-use requires a local consumed-token store (append-only) |
| Reason/justification field | Forces approver to write a human-readable rationale; becomes audit evidence | LOW | — | Free-text string, stored in the audit event. Not machine-enforced but logged permanently |
| Agent-identity binding | Bind the token to a specific AppContainer/agent SID so only the approved agent can use it, not any process on the host | HIGH | `nono-agentd` daemon; AppContainer SID per-agent | Requires the daemon to expose the agent's SID at token-verification time; significant implementation lift |

#### Anti-Features

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Self-service token issuance | Developers want fast unblocking without waiting for approval | Defeats the non-self-service invariant entirely; the point of the system is that the developer CANNOT unblock themselves | Non-self-service approval workflow; make the approval request frictionless (email/Slack link) rather than removing the human approver |
| Open-ended scope ("allow everything") | Operators want a quick override for an unknown blocklist | A wildcard scope is indistinguishable from disabling the sandbox; it destroys least-privilege | Force scope to enumerate specific paths/domains; reject token if scope is empty or contains `*` without explicit admin escalation level |
| Long-lived tokens (30+ day expiry) | Reduces re-approval burden | Long-lived tokens accumulate; forgotten tokens persist as standing backdoors | Maximum expiry cap enforced at verification (suggested: 8 hours for developer override, 24 hours for CI); issuers must request renewal |
| Token sharing across repos | One approval covers multiple projects | Allows a token issued for repo A to apply confinement expansion in repo B | Repo-context binding is mandatory and verified before any expansion is applied |

---

### Category B: Override Verification

The verification pipeline is the trust enforcement point. It runs inside the `nono-py` binding at the moment `confined_run` or `confine` is about to apply the CapabilitySet. A failed verification must be indistinguishable from "no override present" from the sandbox's perspective — no partial expansion.

#### Table Stakes

| Feature | Why Expected | Complexity | Existing Dependency | Notes |
|---------|--------------|------------|---------------------|-------|
| Cryptographic signature check | The signature is the fundamental trust anchor — an unverified token is just a JSON file | MEDIUM | `sigstore_verify::verify_with_key` / `sigstore_verify::verify`; `trust::bundle` wrappers | Reuse existing `verify_with_key` from `crates/nono/src/trust/bundle.rs`; do not write a custom ECDSA verifier |
| ZT-Infra `POST /actions` lookup | The token's override ID is verified against the ZT-Infra control plane to confirm it was legitimately issued and not revoked | HIGH | ZT-Infra v2 `POST /actions`; `ActionAuditor` hash-chained KMS-signed audit | This is the non-self-service enforcement point: even if the cryptographic signature is valid, the token is rejected unless the ZT-Infra control plane confirms the override ID as `allow` and not revoked |
| Expiry check | Expired tokens must be rejected even if the signature is valid | LOW | Host clock; override token `expires_at` field | Check `now() > expires_at`; add a clock-skew tolerance of at most 30 seconds; never accept a token with no expiry field |
| Scope-vs-request match | The override scope must be a superset of the expansion being requested for the current invocation; out-of-scope = fail-closed | MEDIUM | `CapabilitySet` path comparison; `Path::starts_with()` (never string `starts_with`) | Scope matching uses `Path::starts_with()` component comparison — this is a SECURITY-CRITICAL path; string operations are a vulnerability (see CLAUDE.md §Common Footguns) |
| Repo-context match | The token's repo binding must match the current invocation's repo root | MEDIUM | Git working directory detection (already used in nono-cli profiles) | Mismatch = fail-closed; log the mismatch to the SecurityEventLayer HMAC chain |
| Fail-closed on any verification error | Any error — network timeout, malformed token, AWS KMS unavailable, clock unavailable — must result in no expansion | LOW | nono's `Fail Secure` principle (CLAUDE.md) | The fallback when ZT-Infra is unreachable is DENY. Live verification is required; offline use is not supported for overrides |

#### Differentiators

| Feature | Value Proposition | Complexity | Existing Dependency | Notes |
|---------|-------------------|------------|---------------------|-------|
| DAAL ledger cross-check | After signature + ZT-Infra check, also verify the override-event hash exists on the DAAL on-chain record | HIGH | ZT-Infra DAAL/DAS; `DAALog.sol`; Alchemy receipt verification | Provides additional tamper-evidence for high-sensitivity overrides; adds latency. Recommended: make this async or optional — local audit + ZT-Infra check is the synchronous gate; DAAL cross-check is post-hoc evidence |
| Replay prevention (single-use token consumed-record check) | A stolen single-use token cannot be replayed even within the expiry window | MEDIUM | Append-only local consumed-token store (NDJSON file in the session state dir) | Check the consumed-token store before applying any single-use token; append the override ID on first successful use; second use = fail-closed + audit event |
| Revocation list polling | Operators can revoke a token before expiry by adding its ID to a revocation list that nono polls | HIGH | ZT-Infra control plane or local file; cache with TTL | Polling interval must be short enough to matter (suggested: 60 seconds). The simpler path: treat ZT-Infra `POST /actions` lookup as the revocation check — if the override ID comes back `deny` or `not_found`, treat as revoked |

#### Anti-Features

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Offline / cached verification | Avoids latency and network dependency | Cached `allow` decisions can persist after revocation; defeats the purpose of revocation | Require live ZT-Infra check; if ZT-Infra is unavailable, fail-closed |
| Best-effort verification ("try to verify, continue anyway") | Prevents developer blocking when the control plane is down | Silently degrades to no security; the "security on best-effort" footgun | Hard fail-closed on ZT-Infra unavailability; document this in the error message so developers know to escalate to the control plane operator |
| Self-signed token accepted without ZT-Infra lookup | Allows offline use cases | Removes the non-self-service invariant; a developer can sign their own token with their own key | Require both: valid signature AND ZT-Infra `allow` decision for the override ID |

---

### Category C: Runtime CapabilitySet Mutation

When verification passes, the CapabilitySet for the current invocation is temporarily expanded to include the override's scope. This is the execution surface — it must be scoped, temporary, and leave no residual expansion after the session ends.

#### Table Stakes

| Feature | Why Expected | Complexity | Existing Dependency | Notes |
|---------|--------------|------------|---------------------|-------|
| CapabilitySet expansion scoped to the current invocation | The override must only apply to the `confined_run`/`confine` call it was presented to — not globally, not persistently | MEDIUM | `CapabilitySet` builder in `crates/nono/src/capability.rs`; `nono-py` `confined_run`/`confine` API | Add a `with_override(token)` method that merges the override scope into the CapabilitySet for the duration of the call. The expanded CapabilitySet is a local variable, not a mutation of shared state |
| Override does not bypass OS confinement | A verified override expands what the OS sandbox is told to allow — it does not remove the sandbox | LOW (conceptual; already in the architecture) | nono OS sandbox (Landlock/Seatbelt/AppContainer) | From `POC-zt-infra-e5-local-provisioner.md`: "an `allow` never bypasses nono's confinement" — the override widens what is in the CapabilitySet that is enforced at the OS layer |
| Override scope is additive, not replacing | The expanded CapabilitySet includes the baseline profile/group-resolved capabilities PLUS the override scope — override cannot remove existing denies | LOW | `policy.rs` `never_grant` semantics | Override additions go through the same canonicalization and source-tracking as user-provided capabilities; they cannot silence `deny` rules or `never_grant` entries in the policy |
| `CapabilitySource::Override` variant for override entries | Capabilities added by an override must be identifiable in audit and diagnostics | LOW | `CapabilitySource` enum in `capability.rs` (currently User/Profile/Group/System) | Add a new variant: `CapabilitySource::Override { id: String, signer: String, expires_at: DateTime<Utc> }`. Used by DiagnosticFormatter and the SecurityEventLayer event |
| No residual state after invocation | The expanded CapabilitySet lives only for the duration of `confined_run`; subsequent calls start from the baseline profile | LOW | `confined_run` call is already stateless with respect to capabilities | Enforce by design: the expansion is local to the `confined_run`/`confine` call stack; no global static or shared mutable state is written |

#### Differentiators

| Feature | Value Proposition | Complexity | Existing Dependency | Notes |
|---------|-------------------|------------|---------------------|-------|
| Expiry-aware session watchdog | If a `confined_run` session outlives the override token's expiry, the session is terminated or the override is rescinded mid-session | HIGH | `exec_strategy.rs` Monitor/Supervised strategy; session lifecycle | Requires the supervisor to track override expiry and send SIGTERM (or Windows job termination) when the token expires. Recommended as v1.x, not v1 |
| Dry-run mode that shows which capabilities the override would add | Developers can preview what an override would actually expand before presenting it | LOW | Existing `--dry-run` output in `crates/nono-cli/src/output.rs` | `--dry-run` with an override token prints the delta: baseline CapabilitySet vs. override-expanded CapabilitySet, labeled with the override ID |

#### Anti-Features

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Persistent override mode (override applies to all future sessions until expiry) | Reduces re-presenting the token on every invocation | Widens the attack window; a stolen token or compromised developer machine can be exploited for the entire override window without per-invocation verification | Require the token to be presented (and verified live) on each `confined_run` call; the verification is fast once the control plane HTTP connection is established |
| Override can remove deny rules | Operators want to temporarily silence a deny rule for a specific path | A deny-rule removal is semantically equivalent to a policy relaxation for the entire profile, not an exception for one invocation | Only additive expansions are supported; if a deny rule is wrong, update the profile or create a new profile group — do not use the override mechanism to suppress deny rules |

---

### Category D: Tamper-Evident Audit Linkage

Every override event must be a first-class entry in the existing SecurityEventLayer HMAC chain. Overrides that are not audited are invisible to SIEM/compliance tooling and undermine the non-self-service invariant.

#### Table Stakes

| Feature | Why Expected | Complexity | Existing Dependency | Notes |
|---------|--------------|------------|---------------------|-------|
| Override-presented event in SecurityEventLayer | The moment an override token is presented (before verification), emit an event to the HMAC chain | LOW | `SecurityEventLayer` (`crates/nono-cli/src/telemetry`); EventID namespace 10001-10005 | Assign new EventIDs (10006+ or extend existing) for: OVERRIDE_PRESENTED, OVERRIDE_VERIFIED, OVERRIDE_REJECTED, OVERRIDE_EXPIRED, OVERRIDE_REVOKED. Include override ID, signer identity, scope summary, expiry in the event data |
| Override-verified event in HMAC chain | A verified+applied override advances the HMAC chain with the override details — creates a non-repudiable record that the expansion occurred | LOW | Same as above | The event data must include: override ID, signer, scope (redacted paths/domains), session ID, timestamp, CapabilitySet delta description |
| Override-rejected event in HMAC chain | A rejected override (invalid sig, expired, out-of-scope, ZT-Infra deny) must be audited even though no expansion occurred | LOW | Same as above | A failed override attempt is a potential security probe; it must appear in the audit chain with the rejection reason |
| ZT-Infra audit-record cross-reference | The local HMAC event includes the ZT-Infra `current_hash` from the `POST /actions` response, creating a bi-directional link between nono's audit chain and ZT-Infra's audit chain | MEDIUM | ZT-Infra `audit.current_hash` in the `POST /actions` response; `ActionAuditor` KMS-signed hash | This is the "tamper-evident link to SEED-003" from the SEED-005 breadcrumbs. The nono audit event embeds the ZT-Infra hash; the ZT-Infra audit record references the agent/action. Either can be used to reconstruct the other |
| Windows Event Log emission for override events | Override events are forwarded to the Windows Application Event Log alongside path-deny and network-deny events | LOW | Existing SecurityEventLayer dual-emit to Application Event Log + ETW (v3.0) | No new infrastructure needed; the SecurityEventLayer already emits to Event Log. New EventIDs for override events must be documented in the ADMX template |

#### Differentiators

| Feature | Value Proposition | Complexity | Existing Dependency | Notes |
|---------|-------------------|------------|---------------------|-------|
| DAAL async anchoring of override events | The override event hash (not the token contents) is anchored on-chain via ZT-Infra DAAL, making it tamper-detectable by a third party | MEDIUM | ZT-Infra DAAL/`attestAction` sidecar; DAALog.sol | Only the hash is anchored — not paths, signer identity, or payloads (privacy constraint from `docs/ENTERPRISE_READINESS.md` and `docs/DAAL.md`). Anchoring is async and does not block the override verification path |
| Audit report: overrides active/used in session | Operators can query which overrides were applied in a session for compliance reporting | MEDIUM | `AuditRecorder` NDJSON; session summary | Add an `overrides_applied` array to the `AuditIntegritySummary` or equivalent session-close record |

#### Anti-Features

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Silent override (no audit event for "routine" approved overrides) | Reduce audit noise for frequently-used overrides | A routine override is still an exception; silence makes SIEM correlation impossible and removes the non-repudiability guarantee | Every override, no matter how routine, emits an audit event. Use EventID filtering at the SIEM layer if needed, not at emission |

---

### Category E: Revocation

An override that can never be revoked before expiry is a standing risk once issued. Revocation must be first-class — not an afterthought.

#### Table Stakes

| Feature | Why Expected | Complexity | Existing Dependency | Notes |
|---------|--------------|------------|---------------------|-------|
| Revocation via ZT-Infra `POST /actions` returning deny for an override ID | The simplest revocation model: operator marks the override ID as revoked in ZT-Infra policy; the next `POST /actions` check returns `deny`; nono fails closed | LOW (integration) / MEDIUM (ZT-Infra policy change) | ZT-Infra control plane; `policies/actions.json` structure | ZT-Infra already supports explicit `deny` entries in its policy JSON. Adding an override ID to the deny list is sufficient revocation. No new nono-side mechanism needed |
| Revocation audited in HMAC chain | When an override is rejected due to ZT-Infra returning `deny` for a previously-valid override ID, emit an OVERRIDE_REVOKED audit event | LOW | SecurityEventLayer | Differentiate OVERRIDE_REVOKED (was valid, now revoked) from OVERRIDE_REJECTED (never valid) in the EventID schema |

#### Differentiators

| Feature | Value Proposition | Complexity | Existing Dependency | Notes |
|---------|-------------------|------------|---------------------|-------|
| Immediate revocation broadcast (operator pushes a signal that nono polls) | Short time-to-revocation for in-flight overrides | HIGH | Requires a push/notification channel or short-interval polling from nono to ZT-Infra | Not required for v1; the ZT-Infra check on each `confined_run` call provides per-invocation revocation checking. An in-session revocation watchdog (see Category C) covers the mid-session case |

---

### Category F: Exception Request Flow (Non-Self-Service)

The request flow is the human-process layer that feeds into the cryptographic layer. It is out of scope for nono itself to enforce approval, but nono must provide the CLI affordances that make the non-self-service workflow possible.

#### Table Stakes

| Feature | Why Expected | Complexity | Existing Dependency | Notes |
|---------|--------------|------------|---------------------|-------|
| `nono override request` CLI command | Developer-facing entry point: outputs a structured exception request for submission to an approver | MEDIUM | `crates/nono-cli/src/cli.rs`; `clap`; DiagnosticFormatter denial output | The command reads the most recent denial from the audit log (or takes CLI args), generates a structured JSON request, and prints it + optionally writes it to a file the developer can send to their manager |
| Denial context in the request | The exception request includes the exact path or domain that was denied, the profile that blocked it, and the suggested minimum scope | MEDIUM | `DiagnosticFormatter` + `AuditEventPayload::CapabilityDecision` | The DiagnosticFormatter already surfaces flag suggestions for denials; the request generator re-uses this to propose a minimal override scope |
| `nono override apply <token-file>` CLI command | Developer-facing command to verify a received token before use | LOW | `crates/nono-cli/src/cli.rs`; override verification logic | Outputs: VALID (with expiry, scope, signer), EXPIRED, INVALID (with reason). Does not execute anything — pure verification. The nono-py `confined_run` path accepts the token as a parameter |

#### Differentiators

| Feature | Value Proposition | Complexity | Existing Dependency | Notes |
|---------|-------------------|------------|---------------------|-------|
| Machine-readable denial to override request conversion | Automates the request generation from a captured audit event | MEDIUM | `AuditRecorder` NDJSON; `AuditEventPayload` | Reads the most recent `CapabilityDecision` denial event and emits a pre-populated override request JSON. Reduces human error in describing the scope |

---

## Feature Dependencies

```
[Exception Token Format (A)]
    required-by --> [Override Verification (B)]
    required-by --> [Runtime CapabilitySet Mutation (C)]
    required-by --> [Tamper-Evident Audit Linkage (D)]
    required-by --> [Revocation (E)]

[Override Verification (B)]
    required-by --> [Runtime CapabilitySet Mutation (C)]
    gate-for --> [Audit Linkage (D)]  (rejection events require attempted verification)

[Runtime CapabilitySet Mutation (C)]
    feeds --> [Audit Linkage (D)]  (override-applied event includes CapabilitySet delta)

[Exception Request Flow (F)]
    requires --> [Token Format (A)]  (request output must match what approver-side tooling signs)
    enables --> [Override Verification (B)]  (the token produced by approval is fed back in)

[Revocation (E)]
    requires --> [Override Verification (B)]  (revocation is checked as part of verification)
    feeds --> [Audit Linkage (D)]  (OVERRIDE_REVOKED events)
```

### Dependency Notes

- **Token format (A) must be finalized before any other category.** Every other feature depends on what fields the token carries. A post-hoc format change breaks verification, audit, and revocation simultaneously.
- **Verification (B) must be atomic.** Partial verification success — e.g., valid signature but no ZT-Infra check — must be treated as full failure. The security contract is AND(signature valid, ZT-Infra allow, not expired, scope matches, repo matches), not OR.
- **Runtime mutation (C) must be invocation-scoped.** The expanded CapabilitySet is a local value passed into `confined_run`/`confine`, not a mutation of any shared state. This is structurally enforced by the nono-py API shape.
- **Audit linkage (D) must be non-optional.** The SecurityEventLayer emit must happen regardless of whether the override is verified or rejected. An exception that leaves no trace is not an exception system — it is a bypass.

---

## MVP Definition

### Launch With (v1 — all required for the milestone to be meaningful)

- [ ] **Token format (A: table stakes)** — Signer identity, scope (paths/domains), expiry, repo-context binding, override ID, signed JSON envelope. This is the contract everything else is built on.
- [ ] **Cryptographic signature verification (B: table stakes)** — `verify_with_key` / `verify` reuse from `trust::bundle`; no custom ECDSA.
- [ ] **ZT-Infra `POST /actions` live lookup (B: table stakes)** — Non-self-service enforcement; fail-closed on unavailability.
- [ ] **Expiry + scope + repo-context match checks (B: table stakes)** — All three checked on every call; any mismatch = fail-closed.
- [ ] **Additive CapabilitySet expansion for the invocation (C: table stakes)** — `CapabilitySource::Override` variant; no global mutation; no deny-rule removal.
- [ ] **Override events in SecurityEventLayer HMAC chain (D: table stakes)** — OVERRIDE_PRESENTED / OVERRIDE_VERIFIED / OVERRIDE_REJECTED; ZT-Infra `current_hash` cross-reference embedded.
- [ ] **Revocation via ZT-Infra `POST /actions` deny (E: table stakes)** — No new infrastructure; OVERRIDE_REVOKED event emitted.
- [ ] **`nono override request` and `nono override apply` CLI commands (F: table stakes)** — Developer-facing entry/exit points.

### Add After Validation (v1.x)

- [ ] **Single-use token replay prevention** — Consumed-token NDJSON store; second use = fail-closed.
- [ ] **Expiry-aware session watchdog** — Terminates a supervised run if the override token expires mid-session.
- [ ] **DAAL async anchoring of override event hashes** — Post-hoc tamper evidence; does not block authorization path.
- [ ] **`--dry-run` override preview** — Shows CapabilitySet delta before executing.

### Future Consideration (v2+)

- [ ] **Agent-identity (AppContainer SID) binding** — Tokens bound to a specific daemon-launched agent; requires daemon-side SID lookup at verification time.
- [ ] **Immediate revocation broadcast / push polling** — Short time-to-revocation for in-flight overrides.
- [ ] **Machine-readable denial to override request auto-conversion** — Reads audit log to pre-populate the request.
- [ ] **Audit report: overrides-applied-in-session summary** — Session-close record includes override IDs used.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Token format (A: table stakes) | HIGH | MEDIUM | P1 |
| Signature verification via sigstore-rs (B) | HIGH | LOW (reuse existing) | P1 |
| ZT-Infra live lookup (B) | HIGH | HIGH (AWS/host-gated integration) | P1 |
| Expiry/scope/repo-context checks (B) | HIGH | MEDIUM | P1 |
| CapabilitySet additive expansion (C) | HIGH | MEDIUM | P1 |
| `CapabilitySource::Override` variant (C) | MEDIUM | LOW | P1 |
| SecurityEventLayer HMAC events for overrides (D) | HIGH | LOW (reuse existing layer) | P1 |
| ZT-Infra hash cross-reference in audit event (D) | MEDIUM | LOW | P1 |
| Revocation via ZT-Infra deny (E) | HIGH | LOW (reuse verification path) | P1 |
| `nono override request` CLI command (F) | MEDIUM | MEDIUM | P1 |
| `nono override apply` CLI command (F) | MEDIUM | LOW | P1 |
| Single-use replay prevention (B differentiator) | HIGH | MEDIUM | P2 |
| Expiry-aware session watchdog (C differentiator) | MEDIUM | HIGH | P2 |
| DAAL async anchoring (D differentiator) | MEDIUM | HIGH (AWS/DAAL gated) | P2 |
| Dry-run override preview (C differentiator) | LOW | LOW | P2 |
| Agent-SID binding (A differentiator) | MEDIUM | HIGH | P3 |
| Revocation broadcast / push polling (E differentiator) | MEDIUM | HIGH | P3 |

---

## Exception Lifecycle: Full Behavioral Specification

### 1. Request Phase

Developer hits a nono block. `nono override request` (or manual inspection of the audit log) produces a structured JSON request containing: the denied path/domain, the profile that blocked it, the suggested minimum scope, the repo root, and a proposed expiry window. Developer sends this to an approver (engineering manager or security team).

### 2. Approval Phase (outside nono — ZT-Infra operator side)

Approver reviews the request. If approved, the approver uses ZT-Infra policy operator tooling to:
- Add a policy entry to ZT-Infra's `POST /actions` that allows a specific override ID action for the agent/actor
- Issue a signed override token: a JSON document containing signer identity, scope, expiry, repo-context binding, and override ID, signed with the approver's KMS key (AWS KMS P-256 — same `ECC_NIST_P256` key type used by the CAF federation model in `docs/ONBOARDING.md`)
- The issuance event is recorded in ZT-Infra's hash-chained KMS-signed audit log

### 3. Delivery Phase

The signed token file is delivered to the developer out-of-band (email, Slack, secrets manager). The developer verifies it locally with `nono override apply <token-file>` before use. This step does not consume a single-use token; it only validates.

### 4. Verification Phase (on every `confined_run` / `confine` call)

The nono-py binding (or nono-cli) runs the full AND-gate:

1. Parse token JSON and check: override ID present, expiry field present
2. `verify_with_key` — cryptographic signature check using the approver's public key
3. Expiry check: `now() > expires_at` → REJECTED (OVERRIDE_EXPIRED event)
4. Repo-context check: token binding matches current invocation repo root → REJECTED on mismatch
5. Scope check: requested CapabilitySet additions are a subset of the token scope → REJECTED on out-of-scope (use `Path::starts_with()`, never string `starts_with`)
6. ZT-Infra `POST /actions` live lookup with the override ID as the action → REJECTED if `deny`, `not_found`, or network error (fail-closed)
7. (v1.x) Single-use check: override ID not in the consumed-token NDJSON store → REJECTED if replay

All steps must pass (AND gate). Any step failure → fail-closed, audit event emitted, no CapabilitySet expansion.

### 5. Expansion Phase

On full verification pass:
- Add the token's scope to the CapabilitySet as `CapabilitySource::Override { id, signer, expires_at }` entries
- The expanded CapabilitySet is passed to `confined_run`/`confine` for the current invocation only
- OVERRIDE_VERIFIED audit event emitted with ZT-Infra `current_hash` cross-reference

### 6. Execution Phase

The sandboxed process runs under the expanded CapabilitySet. The OS sandbox (Landlock/Seatbelt/AppContainer) enforces the expanded allowlist — nothing outside the expanded CapabilitySet is possible structurally. An `allow` from ZT-Infra does not bypass the OS sandbox; it widens what the OS sandbox is told to allow.

### 7. Audit Phase

The SecurityEventLayer HMAC chain records the override event (both verified and rejected). The chain advances. The event includes the ZT-Infra `current_hash`, creating a bi-directional audit link. Windows Event Log and ETW emit the event via the existing dual-emit path. DAAL async anchoring (v1.x) submits only the event hash — not paths, signer identity, or payloads.

### 8. Revocation Phase

At any point while a token has not yet expired, the approver can revoke by:
- Adding the override ID to ZT-Infra's `deny` list in the policy
- The next `POST /actions` lookup returns `deny` → the override is rejected even if the cryptographic signature is still valid
- OVERRIDE_REVOKED audit event is emitted

### Out-of-scope override behaviors (all fail-closed)

- Token with no expiry field → REJECTED
- Token with expiry > maximum cap (8 hours for developer, 24 hours for CI — configurable) → REJECTED
- Token with wildcard scope (no explicit path/domain) → REJECTED
- Token repo-context does not match current repo → REJECTED
- Token scope covers a path, but the specific sub-path requested is not covered → REJECTED (scope-creep prevention: verification matches the exact requested path against the token scope list using `Path::starts_with()`)
- ZT-Infra returns HTTP error (500, timeout, DNS failure) → REJECTED (fail-closed)
- Cryptographic signature valid, ZT-Infra `allow`, but token has already been consumed (single-use replay) → REJECTED

---

## Sources

- `crates/nono/src/capability.rs` — CapabilitySet builder, CapabilitySource enum, AccessMode, FsCapability (HIGH confidence — source code)
- `crates/nono-cli/src/policy.rs` — Group resolver, `never_grant` semantics, the policy layer that override additions must not circumvent (HIGH confidence — source code)
- `crates/nono/src/audit.rs` — AuditRecorder, NDJSON event format, HMAC chain, sigstore attestation (HIGH confidence — source code)
- `crates/nono/src/trust/bundle.rs` — sigstore-rs `verify_with_key`, DSSE envelope, CertificateInfo (HIGH confidence — source code)
- `crates/nono-cli/src/agent_daemon/telemetry_init.rs` — SecurityEventLayer wiring, HMAC chain advancement, SpyLayer (HIGH confidence — source code)
- `proj/POC-zt-infra-e5-local-provisioner.md` — E5 composition contract: ZT-Infra decides, nono enforces underneath; fail-closed semantics; `guarded_run` pattern (HIGH confidence — source doc)
- `.planning/seeds/SEED-005-zt-infra-policy-override-attestation.md` — Scope definition, breadcrumbs to sigstore-rs + policy.rs + CapabilitySet (HIGH confidence — source doc)
- `.planning/PROJECT.md` — v3.2 milestone goal: non-self-service, ZT-Infra integration, SecurityEventLayer linkage (HIGH confidence — source doc)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\policies\actions.json` — defaultDecision: deny, allow/deny list structure showing per-action override model (HIGH confidence — source file)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\ARCHITECTURE.md` — Layer model: ZT-Infra as policy decision, nono as execution containment (HIGH confidence — source doc)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\DAAL.md` — DAAL async anchoring, non-blocking sidecar model, data boundary (no prompts/payloads on-chain) (HIGH confidence — source doc)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\ENTERPRISE_READINESS.md` — Failure behavior table, fail-closed on policy engine unavailability (HIGH confidence — source doc)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\PROJECT_SCOPE.md` — ZT-Infra narrowest claim; nono as flagship local execution containment pairing (HIGH confidence — source doc)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\ONBOARDING.md` — AWS KMS P-256 signing key model; `min_confirmations` finality policy (HIGH confidence — source doc)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\README.md` — Local provisioner, KMS signing, hash-chained audit, DAAL (HIGH confidence — source doc)
- CLAUDE.md §Security Considerations — Path handling with `Path::starts_with()`, fail-secure principle, path security (HIGH confidence — project rules)

---

*Feature research for: signed policy-exception (break-glass / signed override) system for nono v3.2*
*Researched: 2026-06-21*
