# Phase 93: Live ZT-Infra Integration + Revocation + Request Flow - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-22
**Phase:** 93-live-zt-infra-integration-revocation-request-flow
**Areas discussed:** Live-check composition, POST /actions mapping, Config sourcing (VFY-03a), CLI request/apply UX

---

## Live-check composition

| Option | Description | Selected |
|--------|-------------|----------|
| Python orchestration layer | New Python entry orchestrates Rust offline verify → urllib POST /actions → confined_run; Rust stays offline/policy-free; reused by CLI apply + gates | ✓ |
| Inline in confined_run/confine | Live call internal to confined_run/confine; fewer surfaces, not independently reusable | |
| HTTP inside Rust verify | Push POST /actions into Rust (ureq/reqwest); rejected by research | |

**User's choice:** Python orchestration layer
**Notes:** Keeps the policy-free-core + mockable-live-arm story; consistent with milestone research (urllib, zero deps).

### Live-failure surfacing

| Option | Description | Selected |
|--------|-------------|----------|
| Distinct kinds, both fail-closed | deny → LiveRevoked → EventID 10010; timeout/unreachable/non-200 → LiveUnavailable → EventID 10008; both block | ✓ |
| Single generic kind | Any live failure → one LiveDenied → 10008 | |

**User's choice:** Distinct kinds, both fail-closed
**Notes:** Operators can distinguish a deliberately-revoked token from a down endpoint in the HMAC chain; still fail-closed.

---

## POST /actions mapping

| Option | Description | Selected |
|--------|-------------|----------|
| jti in action (per-token revoke) | actor=signer ARN, action="override.apply:<jti>", resource=repo_context, correlation_id=jti; per-jti deny-list revocation | ✓ |
| Coarse action + jti elsewhere | action="override.apply"; jti in resource/correlation_id; deny would revoke ALL overrides | |
| Scope-derived action | action encodes scope; per-scope not per-token revocation | |

**User's choice:** jti in action (per-token revoke)
**Notes:** Grounded in provisioner `evaluateAction` keying only on actor+action; enables exact ZTL-03 per-token revocation.

### Endpoint auth

| Option | Description | Selected |
|--------|-------------|----------|
| Network-trust, no app auth | JSON only; NONO_ZT_ACTIONS_URL trust anchor; optional NONO_ZT_ACTIONS_HEADER passthrough, unused by default | ✓ |
| Mandatory auth header | Require bearer/token header, fail-closed if absent | |

**User's choice:** Network-trust, no app auth
**Notes:** Matches provisioner reality (no app auth; publicIngress:false + Tailscale/SSM network trust). Optional header is the forward hook.

---

## Config sourcing (VFY-03a)

| Option | Description | Selected |
|--------|-------------|----------|
| Policy-authoritative + env ops | HKLM policy authoritative for trust roots (pubkey + ARN); env only for ops (URL, timeout); missing = fail-closed; env can't widen trust | ✓ |
| Env-sufficient (dev-first) | Env supplies pubkey+ARN+URL; HKLM optional; env can set trust roots | |
| Policy-only strict | Everything from HKLM; no env at all | |

**User's choice:** Policy-authoritative + env ops
**Notes:** Defense-in-depth; leverages the v3.0 enterprise policy spine; env cannot install a rogue trust root.

### Read mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| nono-py reads HKLM directly | windows-sys registry read in nono-py (confined_run is Windows-only); env fallback for ops; no nono.exe round-trip | ✓ |
| Env-only (enterprise populates) | nono-py reads env only; policy→env bridge out-of-band | |
| Delegate to nono.exe | nono-py shells `nono override config` using the v3.0 reader | |

**User's choice:** nono-py reads HKLM directly
**Notes:** Self-contained; fits the "enforcement surface = nono-py" framing; caches VerificationKey per key_id (closes the [BLOCKING-93] pubkey seam).

---

## CLI request/apply UX

| Option | Description | Selected |
|--------|-------------|----------|
| Split by capability | request = nono.exe-native (DiagnosticFormatter only); apply = nono-py console entry (full offline+live verify) | ✓ |
| All nono.exe, delegate apply | Both nono.exe subcommands; apply shells into nono-py verifier | |
| All nono-py console script | Both delivered by nono-py; request re-implements denial surfacing outside DiagnosticFormatter | |

**User's choice:** Split by capability
**Notes:** Each command lives where its capability already is; no duplicated verifier, no nono.exe→nono-py shell-back.

### Request/apply shape

| Option | Description | Selected |
|--------|-------------|----------|
| JSON bundle + one-shot apply | request emits JSON request bundle (scope/repo/denial/nonce) + human summary; apply <token> -- <cmd> verifies (offline+live) then runs confined | ✓ |
| Human text + verify-only apply | request = human text; apply = verify-only, no run | |

**User's choice:** JSON bundle + one-shot apply
**Notes:** Machine-readable bundle feeds the approver/KMS-signing pipeline; apply mirrors confined_run at the CLI.

---

## Claude's Discretion

- ZTL-04 `AWS_*` strip locus (`exec_strategy/env_sanitization.rs`); ZTL-05 DAAL async (never `flush_daal:true` on hot path); DF-02 OVERRIDE-02 gate target (local provisioner + SKIP_HOST_UNAVAILABLE); bi-directional hash refinement (live response `current_hash` vs token's); Python orchestration entry name/signature + urllib impl; request-bundle JSON schema; console-script registration; Phase 91 D-06 token wire-shape reconciliation (research item).

## Deferred Ideas

- Non-Windows override wiring; nono-ts parity (FUT-03); M-of-N approval (FUT-01); push/webhook revocation (FUT-02); mandatory app-level endpoint auth; crate publish / `v*.*.*` release.
- Reviewed-not-folded todos: msi-vcredist, poc-cert-broker, macos-rlimit (keyword-coincidence; out of override scope).
