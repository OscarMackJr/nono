# Phase 93: Live ZT-Infra Integration + Revocation + Request Flow - Context

**Gathered:** 2026-06-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Close the **two-key AND gate** and complete the v3.2 signed-override surface. Bolt a **live `POST /actions`** check onto Phase 91's offline verifier so an override is accepted only when **both** the KMS signature verifies offline **and** a live ZT-Infra lookup returns `allow`; revoked tokens are rejected on the next live check (no new revocation infra); `AWS_*` credentials never reach the sandboxed child; override authorizations anchor to the ZT-Infra DAAL ledger **asynchronously/non-blocking**; and a developer can request and apply overrides from the CLI. This is the **final phase** of v3.2.

**In scope:** ZTL-01 (configurable live endpoint), ZTL-02 (2s-timeout fail-closed live call), ZTL-03 (deny-list revocation honored on next check), ZTL-04 (`AWS_*` stripped from the child env), ZTL-05 (async non-blocking DAAL anchoring); CLI-01 (`nono override request` surfaces denial context), CLI-02 (`nono override apply` runs full fail-closed verify before expansion); DF-02 (`verify-dark.ps1 --gate OVERRIDE-02` live gate with `SKIP_HOST_UNAVAILABLE`).

Also closes the carried-forward seams: **VFY-01 clause (b)** — the live arm Phase 92 left as a `[BLOCKING-93]` composition seam; **VFY-03 clause (a)** — production KMS-pubkey + ARN-allowlist sourcing left `[BLOCKING-93]` in Phase 91; **Phase 91 D-06** — reconcile the nono-side override-token wire shape with provisioner-issued tokens.

**Explicitly NOT in this phase (future):**
- **Non-Windows override wiring** (`sandboxed_exec` / Landlock / Seatbelt parity) — `confined_run`/`confine` are Windows-only; only the seam is documented (Phase 92 D-04).
- **`nono-ts` binding parity** (FUT-03), **M-of-N / threshold approval** (FUT-01), **push/webhook revocation** (FUT-02).
- **Crate publish / `v*.*.*` release** — milestone-marker only; a future release leapfrogs the crate to ≥ `0.65.0`.

</domain>

<decisions>
## Implementation Decisions

### Live-check composition (where the `POST /actions` AND-gate lives)
- **D-01 (Python orchestration layer):** The Rust `verify_override()` stays **offline-only and policy-free**. A new **Python-side orchestration layer** performs: Rust offline verify → `OverrideGrant`, then a **Python `urllib.request` `POST /actions`** live call, and only on a live `allow` do `confined_run`/`confine` proceed. The live arm is independently **mockable** and is reused by the CLI `apply` path (D-07) and the OVERRIDE-02 gate. Do **not** push HTTP into Rust (no `ureq`/`reqwest` net dep). Offline-pass is necessary but not sufficient (mirrors the Phase 92 D-03 seam).
- **D-02 (Distinct fail-closed kinds for the live arm):** A live **`deny`** response → `NonoOverrideError` kind **`LiveRevoked`** → **EventID 10010 (REVOKED)**. A **timeout (>2s) / unreachable / non-200 / malformed** response → kind **`LiveUnavailable`** → **EventID 10008 (REJECTED)**. **Both block the run** (fail-closed). Extends the Phase 91 D-04 kind→EventID map; gives operators a revoked-vs-infra-down signal in the HMAC audit chain without ever failing open.

### `POST /actions` request mapping + endpoint trust
- **D-03 (OverrideGrant → {actor, action, resource}; jti in `action` for per-token revoke):** Map the verified grant as `actor` = signer ARN (`OverrideGrant.signer` / `kms_key_id`), `action` = **`"override.apply:<jti>"`**, `resource` = `repo_context`, `correlation_id` = `jti`. The provisioner's `evaluateAction` keys **only on `actor` + `action`** (exact / prefix-`*` match; `resource` is audit-only), so per-token revocation (ZTL-03) works by an operator adding a deny rule matching `action` `"override.apply:<jti>"`; normal overrides are permitted by an allow rule `"override.apply*"` (the provisioner `defaultDecision` is `deny`). This makes the **live check the sole revocation enforcement point** — no new infra in nono.
- **D-04 (Network-level trust; no app-level auth; optional header passthrough):** nono sends the JSON body only; trust is **network-level** — `NONO_ZT_ACTIONS_URL` is the trust anchor (`127.0.0.1` local provisioner, or a Tailscale/SSM-fronted URL for the real AWS control plane; `/api/status` advertises `publicIngress:false`). Provide an **optional** env-supplied header passthrough (working name `NONO_ZT_ACTIONS_HEADER`) for future bearer/mTLS-proxy use — **unused/empty by default**. The provisioner has no app-level auth; do not invent a mandatory one.

### Config sourcing — closes VFY-03 clause (a) `[BLOCKING-93]`
- **D-05 (Policy-authoritative trust model):** `HKLM\SOFTWARE\Policies\nono` (the v3.0 enterprise policy spine, fail-secure reader from Phase 83) is **authoritative for trust roots** — the KMS public key (DER+base64) and the ARN allowlist. Env vars supply **only non-trust operational config** (`NONO_ZT_ACTIONS_URL`, timeout, the optional header). **Missing trust material in policy = fail-closed deny.** Env **cannot override or widen** trust roots (prevents a tampered environment from installing a rogue pubkey/ARN).
- **D-06 (nono-py reads HKLM directly; caches VerificationKey per `key_id`):** `confined_run`/`confine` are `#[cfg(windows)]`, so nono-py does its **own `windows-sys` registry read** of the policy spine for the override trust config (env fallback only for the ops config per D-05) — no nono.exe round-trip. The `VerificationKey` is built from the policy pubkey DER and **cached per `key_id`** (closes the Phase 91/93 `[BLOCKING-93]` pubkey seam in `verify_ecdsa_digest` / `verify_override_impl`, which today take `pubkey_der` test-injected).

### CLI request/apply UX (CLI-01 / CLI-02)
- **D-07 (Split by capability):** `nono override request` is **nono.exe-native (Rust)** — it only needs `DiagnosticFormatter` denial context (paths/domains/repo), no crypto/live. `nono override apply` is **nono-py-delivered** (a console entry that reuses the Python orchestration layer, D-01, for the full offline+live verify). Each command lives where its capability already is — **no duplicated verifier**, no nono.exe→nono-py shell-back.
- **D-08 (`request` = JSON bundle; `apply` = one-shot verify-then-run):** `nono override request` emits a **structured JSON request bundle** (scope paths/domains, `repo_context`, denial reason, a fresh nonce) for the approver/KMS-signing pipeline **plus** a human-readable summary. `nono override apply <token-path> -- <command>` runs **full fail-closed verification (offline + live)**, then executes the command confined in **one shot** (the CLI mirror of `confined_run`).

### Claude's Discretion
Researcher/planner decide these with the fail-secure defaults noted; no further user input needed:
- **ZTL-04 (`AWS_*` strip locus):** extend the existing exec-strategy env filter at `crates/nono-cli/src/exec_strategy/env_sanitization.rs` to drop **all** `AWS_*` vars from the child env; verify via an env-inspection test (SC3). Respect the Windows `SystemRoot`/`windir` baseline re-add already established for CLR children.
- **ZTL-05 (DAAL async):** nono **never sets `flush_daal:true`** on the hot path — authorization returns on the `/actions` decision without waiting on ledger finality; DAAL anchoring (the response `audit.daal` field) is the provisioner's async concern. Anchoring must not block or fail the spawn path.
- **DF-02 (OVERRIDE-02 gate):** runs against the **local provisioner** (like OVERRIDE-01), emitting `SKIP_HOST_UNAVAILABLE` (exit 3) when the provisioner/AWS is absent. Follows the `Test-Precondition`/`Invoke-Gate` contract; **never** calls `exit` or `Persist-Verdict`; invoked via `-File`/direct (never `pwsh -Command "<bare path>"`).
- **Bi-directional hash refinement (AUD-02):** decide whether the emitted `PolicyOverrideApplied` uses the **live `/actions` response's fresh `audit.current_hash`** (authoritative for *this* authorization) vs the token's embedded `current_hash` (the Phase 92 default). Lean: prefer the live-response hash on the live-verified path, fall back to the token hash offline.
- Exact Python orchestration entry name/signature; `urllib` request/timeout implementation; the request-bundle JSON schema; the console-script registration for `nono override apply` (nono-py `pyproject`/`entry_points`); the `nono override` subcommand grouping in the `Commands` enum.
- **Phase 91 D-06 token wire-shape reconciliation:** confirm provisioner-issued / real-KMS token bytes match the CAF v0.1 shape nono parses (a research item; the OVERRIDE-02 gate mints via the local provisioner + the Phase 91 test keypair).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope, requirements & predecessor decisions
- `.planning/ROADMAP.md` § "Phase 93" — goal, the 5 success criteria, requirement set (ZTL-01..05, CLI-01/02, DF-02), 91→92→93 order.
- `.planning/REQUIREMENTS.md` — full ZTL/CLI/DF text + the architecture invariant (two-key AND gate; additive-only; policy-free core) + Out-of-Scope (offline/cached verification defeats revocation; override never weakens a deny / never bypasses the OS layer).
- `.planning/phases/92-runtime-capabilityset-mutation-audit-wiring/92-CONTEXT.md` — D-01..D-06 carry-forwards; **D-03 is the live-arm composition seam this phase consumes**; the `--override-audit` bilateral handshake; EventID 10006–10010 map.
- `.planning/phases/92-runtime-capabilityset-mutation-audit-wiring/92-VERIFICATION.md` — VFY-01 PARTIAL `[BLOCKING-93]` evidence; exact wired surfaces (file:line) this phase extends.
- `.planning/phases/91-signed-override-format-verification-core/91-CONTEXT.md` — D-04 `NonoOverrideError.kind` → EventID contract (extended by D-02 here), D-05 crypto primitive (`verify_prehashed` + low-S), **D-06 token wire-shape `[BLOCKING]`** to reconcile here.

### nono-py — verifier + Python live-arm orchestration (this phase extends)
- `nono-py/src/override.rs` — `verify_override()` (`:855`), `verify_override_impl()` (`:762`), `verify_ecdsa_digest()` (`:621`) with the **`[BLOCKING-93]` VFY-03a pubkey seam** (`:609`); `OverrideGrant` (`:640`, carries `signer`, `scope_paths`, `scope_domains`, `jti`, `repo_context`, `zt_audit_hash` `:813`); `OverrideErrorKind` enum (add `LiveRevoked`/`LiveUnavailable` per D-02).
- `nono-py/src/windows_confined_run.rs` — `probe_override_support()` (`:170`), `append_override_args()` (`:307`), `confined_run()` (`:367`), `confine()`; the live-arm call sits between the offline grant and `append_override_args` (D-01).
- `nono-py/src/lib.rs` — module registration; confirms `confined_run`/`confine` are `#[cfg(windows)]` (where the HKLM read in D-06 is in-scope).
- `nono-py/tests/test_override_wiring.py` + `nono-py/tests/fixtures/override_test_key.der` — Phase 92 pytest harness + the reusable ECDSA P-256 test keypair (token minting for OVERRIDE-02; never a production trust root).

### nono-cli + core nono — audit chain, env filter, CLI surface
- `crates/nono-cli/src/telemetry/event.rs` — EventID constants 10006–10010; D-02 uses **10008 (REJECTED)** + **10010 (REVOKED)**.
- `crates/nono-cli/src/telemetry/mod.rs` — `SecurityEventLayer`, `emit_override_event()` + `SECURITY_LAYER` OnceLock (Phase 92); the live-arm denial events flow through here.
- `crates/nono-cli/src/execution_runtime.rs` — Phase 92 AUD-04 pre-spawn gate; the `--override-audit` receive path.
- `crates/nono-cli/src/exec_strategy/env_sanitization.rs` — **ZTL-04 `AWS_*` strip locus** (existing env filter to extend); respect the Windows `SystemRoot`/`windir` baseline.
- `crates/nono-cli/src/cli.rs` — `Commands` enum (`:644`) where **`nono override request`** lands; `OverrideAuditMeta` (`:1628`) + `--override-audit` field (`:2021`).
- `crates/nono-cli/src/diagnostic/formatter.rs` (+ `diagnostic/mod.rs`) — `DiagnosticFormatter` denial rendering for **CLI-01** `nono override request`.
- `crates/nono-cli/src/policy.rs` + `crates/nono-cli/src/platform.rs` + `crates/nono-cli/src/cli_bootstrap.rs` — the v3.0 `HKLM\SOFTWARE\Policies\nono` machine-policy reader pattern (D-05/D-06 mirror this; nono-py reads HKLM with its own `windows-sys` call).

### Dark Factory gate
- `scripts/verify-dark.ps1` — gate harness (auto-discovers `scripts/gates/*.ps1`; `Test-Precondition`/`Invoke-Gate`; verdict JSON; exit 0=PASS/2=FAIL/3=SKIP_HOST_UNAVAILABLE/4=HARNESS_ERROR). Add `scripts/gates/override-02.ps1` for DF-02.
- `scripts/gates/override-01.ps1` — Phase 92 OVERRIDE-01 sibling; mirror its contract for OVERRIDE-02.

### External dependency — ZT-Infra v2 (`C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2`)
- `provisioner/src/server.js` — the **`POST /actions` contract**: request `{actor, action, resource, correlation_id?, cross_org?, flush_daal?}`; response `{ok, decision: allow|deny, reason, audit:{timestamp, previous_hash, current_hash, kms_signature, daal}, daal_flush}` (200 allow / 403 deny).
- `provisioner/src/policy.js` — `evaluateAction` keys **only on `actor`+`action`** (grounds D-03); `deny[]` list = the revocation enforcement point (ZTL-03); `defaultDecision: "deny"`.
- `provisioner/src/audit.js` — `current_hash` semantics (AUD-02 bi-directional link; the live-response hash refinement).
- `provisioner/src/daal.js` — DAAL drain/`flush_daal` semantics (ZTL-05 async anchoring).
- `docs/CANONICAL_FORM.md` + `test-vectors/canonical-form/vectors.json` — CAF v0.1 normative form (token wire-shape reconciliation, Phase 91 D-06).
- `proj/POC-zt-infra-e5-local-provisioner.md` — E5 composition runbook; local provisioner startup (`cd provisioner && npm install && npm start` on `127.0.0.1:3000`) the OVERRIDE-02 gate exercises.

### nono in-tree invariants
- `CLAUDE.md` § "Library vs CLI Boundary" (policy-free core), § Security Considerations (never string `starts_with`; canonicalize at the boundary; fail-secure), § Coding Standards (no `.unwrap()`/`.expect()`; `#[must_use]`; **cross-target clippy MUST/NEVER** for any cfg-gated Unix code touched).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `nono-py/src/override.rs::verify_override` + `OverrideGrant` — the offline verified-grant source; the Python live arm wraps this, never re-parsing the token (TOCTOU closure).
- `windows_confined_run.rs::{probe_override_support, append_override_args, confined_run, confine}` — the Phase 92 override path; the live call slots between the offline grant and `append_override_args`.
- `crates/nono-cli/src/telemetry/{event.rs,mod.rs}` — EventIDs 10008/10010 + `SecurityEventLayer`/`emit_override_event` already exist (Phase 92); the live-arm denial events reuse them.
- `crates/nono-cli/src/exec_strategy/env_sanitization.rs` — existing env filter; ZTL-04 extends it for `AWS_*`.
- `crates/nono-cli/src/diagnostic/formatter.rs` — `DiagnosticFormatter`; CLI-01 `nono override request` reuses its denial rendering.
- v3.0 `HKLM\SOFTWARE\Policies\nono` reader (`policy.rs`/`platform.rs`) — the pattern D-06 mirrors in nono-py.
- Phase 91 test keypair + Phase 92 pytest/gate harness — reused for OVERRIDE-02 token minting against the local provisioner.

### Established Patterns
- **Fail-secure** (CLAUDE.md): `Result` + `?`; no `.unwrap()`/`.expect()`; `#[must_use]` on critical Results; libraries never panic on expected errors. Every live-arm failure path denies.
- **Path/DNS security**: component comparison only; canonicalize at the boundary; never string `starts_with`.
- **EventID + redaction**: `NonoOverrideError.kind` maps 1:1 to EventIDs without string-parsing messages; events carry no raw secrets/key material.
- **HMAC chain integrity**: `SecurityEventLayer` advances seq + chain hash per event.
- **Dark Factory**: gates emit machine-readable verdicts; host-gated paths `SKIP_HOST_UNAVAILABLE`, consistent with prior milestones.

### Integration Points
- nono-py Python live arm → ZT-Infra `POST /actions` (new outbound boundary; `urllib`, 2s timeout, fail-closed).
- nono-py → HKLM policy spine (new read for trust roots, D-06).
- nono-py live-deny/unavailable → nono-cli `SecurityEventLayer` via the existing `--override-audit` + `emit_override_event` path (EventIDs 10008/10010).
- `nono override request` (nono.exe) → `DiagnosticFormatter`; `nono override apply` (nono-py console entry) → the Python orchestration layer.
- exec-strategy env filter → child env (`AWS_*` strip, ZTL-04).

</code_context>

<specifics>
## Specific Ideas

- "Reuse, don't reinvent": keep the verifier in nono-py, reuse the Phase 92 EventID/audit wiring, reuse the v3.0 policy-spine pattern, reuse the Phase 91 keypair + Phase 92 gate harness — no new crypto, no new revocation infra, no Rust HTTP stack.
- The **live check IS the revocation point** — `ZTL-03` adds no nono-side infrastructure; an operator revokes by adding a provisioner deny rule on `override.apply:<jti>`.
- **Fail-closed is non-negotiable across the new live arm**: deny, timeout, unreachable, malformed, and missing-trust-material all deny and run nothing (D-02, D-05).
- Honor the TOCTOU closure: the live arm consumes the already-verified `OverrideGrant`; the token is never re-parsed between offline and live checks.

</specifics>

<deferred>
## Deferred Ideas

- **Non-Windows override wiring** (`sandboxed_exec` / Landlock / Seatbelt parity) — future; `confined_run`/`confine` stay Windows-only.
- **`nono-ts` binding parity** (FUT-03), **M-of-N / threshold approval** (FUT-01), **push/webhook revocation** (FUT-02) — future milestone.
- **Mandatory app-level endpoint auth (bearer/mTLS in nono)** — out of scope per D-04; the optional `NONO_ZT_ACTIONS_HEADER` passthrough is the forward hook.
- **Crate publish / `v*.*.*` release** — milestone-marker only; future release leapfrogs to ≥ `0.65.0`.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md`, `20260611-poc-cert-broker-clean-host.md`, `20260612-macos-rlimit-as-setrlimit-fails.md` — surfaced by keyword coincidence (phase/uat/host); none relate to ZT-Infra/override scope. Reviewed and left deferred (same disposition as the v3.2 milestone open).

</deferred>

---

*Phase: 93-live-zt-infra-integration-revocation-request-flow*
*Context gathered: 2026-06-22*
