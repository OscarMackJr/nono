# Phase 92: Runtime CapabilitySet Mutation + Audit Wiring - Context

**Gathered:** 2026-06-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire the Phase 91 **offline** override verifier into the `confined_run`/`confine` execution path so a verified override **additively** expands the `CapabilitySet` for **exactly one** invocation, and every expansion emits an `AuditEventPayload::PolicyOverrideApplied` event into the v3.0/v3.1 `SecurityEventLayer` HMAC chain **before the sandboxed child spawns** — an override that cannot emit its audit record is **blocked, not silently applied** (AUD-04 fail-closed).

**In scope:** MUT-01 (additive, scoped expansion), MUT-02 (invocation-scoped, no global mutation), MUT-03 (cannot weaken deny rules / bypass OS confinement), MUT-04 (path/DNS-component matching, never string `starts_with`), MUT-05 (byte-for-byte-identical no-override path, regression-proven); AUD-01 (lifecycle events into the HMAC chain), AUD-02 (embed ZT-Infra `audit.current_hash` for the bi-directional link), AUD-03 (EventIDs 10006–10010 + redaction), AUD-04 (no-audit ⇒ no-apply); VFY-01 **PARTIAL** (offline arm + live-arm composition seam only); DF-01 (`verify-dark.ps1 --gate OVERRIDE-01`).

**Explicitly NOT in this phase (Phase 93 owns these):**
- **Live `POST /actions` AND-gate** (VFY-01 clause b / ZTL-*) — only the *seam* is built here; the live call is `[BLOCKING-93]`.
- **AWS credential stripping, revocation enforcement, `nono override request/apply` CLI, DAAL anchoring** — Phase 93.
- **Non-Windows (`sandboxed_exec` / Landlock / Seatbelt) override wiring** — deferred; only the seam is documented here.

</domain>

<decisions>
## Implementation Decisions

### Mutation + audit locus (split architecture — Option B)
- **D-01 (nono-py verifies + applies; nono-cli audits + gates the spawn):** The verifier from Phase 91 stays in **nono-py** (no crypto duplication, honors the Phase 91 boundary and the milestone's "signed overrides via nono-py" framing). Flow:
  1. nono-py calls `verify_override()` (Phase 91) → `OverrideGrant`.
  2. nono-py sanitizes the grant's `scope_paths` and appends them **verbatim** as `--allow` flags on the `nono.exe run` invocation (SC1: "appends exactly the grant paths as `--allow` flags").
  3. nono-py passes **trusted audit metadata** (`zt_audit_hash`, `kms_key_id`, `jti`, granted paths) to nono.exe via a new flag.
  4. **nono-cli** emits `PolicyOverrideApplied` into its `SecurityEventLayer` HMAC chain and **aborts before spawning the child** if the event cannot be committed.
- Rationale: `SecurityEventLayer` (the HMAC chain) lives only in **nono-cli** (`crates/nono-cli/src/telemetry/mod.rs`), and nono-py has **no production path** into it. Emitting from nono-py would require relocating `SecurityEventLayer` into the core crate — out of scope and against the policy-free-core invariant. Trusting launcher-supplied audit metadata is the **same trust model as the existing `--allow` flags**, and the OS confinement layer still applies to the expanded set regardless.

### AUD-04 fail-closed handshake (close the silent-escalation window)
- **D-02 (Mandatory flag + bilateral capability gate):** Override-granted `--allow` flags are passed **only** alongside a new required flag (working name `--override-audit <metadata>`). Two-sided fail-closed:
  - **CLI side:** nono-cli treats override-granted paths present **without** a successfully-emitted `PolicyOverrideApplied` event as **FATAL** — abort before spawn (no child ever runs against override paths without a committed audit record).
  - **nono-py side:** nono-py **refuses to launch** unless the target `nono.exe` advertises override support (min-version / capability probe). This prevents a too-old CLI from silently accepting the extra `--allow` flags and never auditing them.
- This is the structural mechanism for AUD-04 across the process/repo boundary — neither side alone is sufficient.

### VFY-01 treatment (PARTIAL seam)
- **D-03 (Offline arm + composition seam only):** Phase 92 wires the **offline** verify into `confined_run`/`confine` and builds the explicit composition point where Phase 93's live `POST /actions` arm slots in (offline-pass is **necessary but not sufficient**). VFY-01 is recorded **PARTIAL** with a `[BLOCKING-93]` note in the plan. Mirrors the Phase 91 VFY-03 PARTIAL split already sanctioned. Do **not** mistake this boundary for a gap.

### Platform scope (Windows-only v1)
- **D-04 (Windows `confined_run`/`confine` only):** `confined_run`/`confine` are `#[cfg(windows)]` in nono-py; the non-Windows path is a separate `sandboxed_exec`. Phase 92 wires overrides into the **Windows** path only (where the code and the dev host live) and **documents a seam** for the non-Windows `sandboxed_exec` / Landlock+Seatbelt path. Landlock/Seatbelt override parity is a future item, not a Phase 92 gap.

### Scope matching + audit fields
- **D-05 (Reuse the nono-cli capability layer; sanitize grant paths in nono-py):** nono-py sanitizes grant paths before they become `--allow` flags — **reject `..`, require absolute, canonicalize** — then appends them verbatim. nono-cli's **existing component-wise `Path::starts_with`** capability matching enforces SC3 at the OS boundary (a grant for `/tmp/project` cannot cover `/tmp/project-evil`). No new matching engine — MUT-04 is satisfied by reuse + sanitization, never string `starts_with`.
- **D-06 (Extend the verified grant for audit fields; never re-parse the token):** AUD-02 needs `zt_audit_hash`. The token carries `current_hash: Option<String>` (`override.rs:366`) and `kms_signature.key_id` (`override.rs:312`). `kms_key_id` is already exposed on `OverrideGrant` as `signer`. Add a **read-only `zt_audit_hash` getter** to `OverrideGrant` sourced from `token.current_hash`. All audit fields are read **from the already-verified grant**, never by re-parsing the token — honors Phase 91 D-02 (closes the TOCTOU verify→apply gap). Adding a `#[getter]` does not violate the `frozen` pyclass invariant (instances remain immutable).

### Claude's Discretion
Researcher/planner decide these with fail-secure defaults; no further user input needed:
- Exact name/format of the new audit-metadata flag (`--override-audit` is a working name) and whether metadata is JSON / base64 / repeated flags.
- The CLI capability-advertisement / min-version probe mechanism (e.g. a `--capabilities` query, `nono --version` gate, or an env handshake) — must be fail-closed per D-02.
- Where the `PolicyOverrideApplied` variant's fields land on `AuditEventPayload` (`crates/nono/src/audit.rs`) vs the `SecurityEventLayer` emission shape, and how the EventID 10006–10010 constants are defined in `telemetry/event.rs`.
- The redaction shape for override events (no raw secrets; paths per existing redaction policy) — AUD-03.
- The OVERRIDE-01 gate's test-token minting (reuse the Phase 91 committed local ECDSA P-256 test keypair) and gate file structure under `scripts/gates/`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` § "Phase 92" — goal, 5 success criteria, requirement set (MUT-01..05, AUD-01..04, VFY-01, DF-01) and the 91→92→93 execution order.
- `.planning/REQUIREMENTS.md` — full MUT/AUD/VFY/DF requirement text + the architecture invariant (two-key AND gate; **additive-only**; Rust core policy-free) + the Out-of-Scope table (offline/cached verification defeats revocation; override that weakens a deny rule is forbidden).
- `.planning/phases/91-signed-override-format-verification-core/91-CONTEXT.md` — predecessor decisions: D-02 `OverrideGrant` value type (TOCTOU seam), D-04 `NonoOverrideError.kind` → EventID 10008 contract, D-05 crypto primitive (`verify_prehashed` + explicit low-S), D-06 nono-side token wire shape `[BLOCKING]`-reconcile-before-93.

### nono-py — verifier + execution entry points (Phase 92 wires these)
- `nono-py/src/override.rs` — Phase 91 verifier. Key surface: `verify_override()` (`:838`), `OverrideGrant` pyclass (`:663` — exposes `signer`=kms_key_id, `scope_paths`, `scope_domains`, `not_before`, `expires_at`, `jti`, `repo_context`), `OverrideToken` struct (`:332`; carries `current_hash` `:366`, `kms_signature.key_id` `:312`), `OverrideErrorKind` enum (`:56`), `partition_scope()` (`:918`).
- `nono-py/src/windows_confined_run.rs` — `confined_run()` (`:176`), `confine()` (`:255`), `build_nono_run_args()` (`:104` — where `--profile`/`--allow`/`--allow-cwd` are appended; the override hook goes here), `append_caps_allow_flags()` (`:133`).
- `nono-py/src/lib.rs:733–769` — module registration; confirms `confined_run`/`confine` are `#[cfg(windows)]` (`:761–764`) and the non-Windows `sandboxed_exec` registration (`:760`).

### nono-cli + core nono — audit chain (Phase 92 emits here)
- `crates/nono/src/audit.rs` — `AuditEventPayload` enum (`:67`; **add `PolicyOverrideApplied` variant here**), `AuditRecorder` (`:345`), `append_event()`, chain hashing (`hash_event` `:495`). Audit module lives in the **core nono crate** (v3.1 boundary move, per CLAUDE.md).
- `crates/nono-cli/src/telemetry/mod.rs` — `SecurityEventLayer` (`:189`), `chain_sequence()` test accessor (`:210`), `on_event()` HMAC-chain advance (`:259`), `advance_chain()` (`:113`). **This is the v3.0/v3.1 chain AUD-01 targets.**
- `crates/nono-cli/src/telemetry/event.rs:30–41` — existing EventID constants 10001–10005; **define 10006–10010** (PRESENTED / VERIFIED / REJECTED / EXPIRED / REVOKED) here.
- `crates/nono/src/capability.rs` (~`:1239–1244`) — existing component-wise `Path::starts_with` capability matching the override `--allow` paths flow through (SC3 enforcement point).

### Dark Factory gate
- `scripts/verify-dark.ps1` — gate harness (auto-discovers `scripts/gates/*.ps1`; `Test-Precondition`/`Invoke-Gate` contract; verdict JSON; exit codes 0=PASS/2=FAIL/3=SKIP_HOST_UNAVAILABLE/4=HARNESS_ERROR). Add `scripts/gates/override-01.ps1` for DF-01. Invoke via `-File`/direct, **never** `pwsh -Command "<bare path>"` (swallows exit codes).

### nono in-tree invariants
- `CLAUDE.md` § "Library vs CLI Boundary" (policy-free core) + § Security Considerations (path footgun: never string `starts_with`; canonicalize at the enforcement boundary; fail-secure).

### External dependency — ZT-Infra v2 (`C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2`)
- `docs/CANONICAL_FORM.md` — CAF v0.1 canonical form; the `current_hash`/AuditRecord shape that AUD-02's `zt_audit_hash` links to.
- `provisioner/src/audit.js` — ZT-Infra audit-chain `current_hash` semantics (the bi-directional link target).
- `proj/POC-zt-infra-e5-local-provisioner.md` — E5 composition runbook; where the nono↔ZT glue + the local provisioner the OVERRIDE-01 gate exercises live.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `nono-py/src/override.rs::verify_override` + `OverrideGrant` — the verified-grant source of truth; Phase 92 reads scope paths and audit fields from it (never re-parses the token).
- `windows_confined_run.rs::build_nono_run_args` / `append_caps_allow_flags` — the existing `--allow`-flag builder; the override `--allow` + audit-metadata flag hook lands here.
- `crates/nono-cli/src/telemetry/mod.rs::SecurityEventLayer` — the HMAC chain; `chain_sequence()` is the test hook for SC4 ("chain hash has advanced", "exactly one new entry").
- `crates/nono/src/capability.rs` component-wise `Path::starts_with` matching — satisfies SC3/MUT-04 without new code.
- Phase 91 committed local ECDSA P-256 test keypair — reuse for OVERRIDE-01 gate token minting (never a production trust root).

### Established Patterns
- **Fail-secure** (CLAUDE.md): `Result` + `?`; no `.unwrap()`/`.expect()` (clippy `unwrap_used`); `#[must_use]` on critical Results; libraries don't panic on expected errors.
- **Path security**: path-component comparison only; canonicalize at the boundary; never string `starts_with`.
- **EventID + redaction**: events use the existing telemetry EventID constant pattern; `NonoOverrideError.kind` (Phase 91 D-04) maps 1:1 to EventID 10008 (REJECTED) without string-parsing messages.
- **HMAC chain integrity**: `SecurityEventLayer` advances a sequence + chain hash per event — the SC4 "chain advanced" assertion targets this.

### Integration Points
- nono-py → nono.exe: new audit-metadata flag + bilateral fail-closed handshake (D-02) is the new boundary.
- `AuditEventPayload::PolicyOverrideApplied` (new, core crate) carries `zt_audit_hash` + `kms_key_id` (+ jti, granted paths, redacted) — emitted via `SecurityEventLayer` in nono-cli before child spawn.
- The offline-verify → composition point (D-03) is the seam Phase 93's live `POST /actions` AND-gate consumes.
- Non-Windows `sandboxed_exec` override wiring is a **documented seam only** in Phase 92 (D-04).

</code_context>

<specifics>
## Specific Ideas

- "Reuse, don't reinvent" (fork co-author): keep the Phase 91 verifier in nono-py, reuse nono-cli's existing capability matching, reuse the Phase 91 test keypair for the gate — no new crypto, no new matching engine.
- AUD-04 is the headline invariant: **no override path ever reaches a child without a committed audit event**. The bilateral handshake (D-02) is non-negotiable — the milestone exists to prevent silent privilege escalation.
- Honor Phase 91 D-02: audit fields read from the verified `OverrideGrant`, never by re-reading/re-parsing the token (TOCTOU).

</specifics>

<deferred>
## Deferred Ideas

- **Live `POST /actions` two-key AND gate** (VFY-01 clause b), **revocation enforcement**, **AWS credential stripping**, **`nono override request/apply` CLI**, **DAAL anchoring** — Phase 93.
- **Non-Windows override wiring** (`sandboxed_exec` / Landlock / Seatbelt parity) — future; only the seam is documented in Phase 92 (D-04).
- **Cross-process / persistent `jti` store** — already deferred in Phase 91 D-03; the live ZT check (Phase 93) is the durable single-use point.
- **`nono-ts` binding parity** — FUT-03, future milestone.
- **Reconciling the nono-side override-token wire shape with real KMS-issued tokens** — Phase 91 D-06 `[BLOCKING]` for Phase 93's live arm; not a Phase 92 task.

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 92-runtime-capabilityset-mutation-audit-wiring*
*Context gathered: 2026-06-21*
