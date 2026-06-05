# Phase 57: Bitwarden Credential Source - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Add a `bw://` credential source that resolves Bitwarden secrets through the existing
keystore abstraction in `crates/nono/src/keystore.rs`, alongside the current
`op://` / `keyring://` / `env://` / `file://` / `apple-password://` schemes. Secrets are
held in `Zeroizing<String>` with in-place truncation. Cross-platform; the entire surface
is isolated to the keystore layer — no platform-specific code paths above it, and the
`--credential` flag accepts `bw://` identically to the other schemes (REQ-CRED-01).

**In scope:** `bw://` URI parsing + validation, dispatch to two Bitwarden backends, secret
extraction into `Zeroizing<String>`, fail-closed error handling, unit tests.

**Out of scope (new capabilities — not this phase):** interactive vault unlock, credential
*writing*/sync, caching/daemonization of sessions, Bitwarden org/collection management,
any change to the proxy credential-injection layer (`nono-proxy/src/credential.rs`).
</domain>

<decisions>
## Implementation Decisions

### Backends & auth model (D-01)
- **D-01:** `bw://` resolves through **BOTH** Bitwarden tools, abstracted behind the one
  scheme, **both delivered in Phase 57** (single cohesive keystore change, one review):
  - **`bw` CLI** (password-manager / classic vault) — for `bw://item/...` references.
  - **`bws` Secrets Manager CLI** — for `bw://secret/<uuid>` references.
- **D-02:** Auth is **environment-token only**, sourced by the operator before the run:
  - `bw` backend → `BW_SESSION` (operator runs `bw unlock --raw` and exports it).
  - `bws` backend → `BWS_ACCESS_TOKEN` (service-account access token).
  - No interactive unlock; no passphrase-in-env auto-unlock (the auto-unlock option was
    explicitly **rejected** — keeps the threat surface tight).

### URI grammar (D-03) — "typed first segment"
- **D-03:** The first path segment after `bw://` names the backend/object type, making the
  two backends explicit and unambiguous:
  ```
  bw://item/<id>/password          # bw CLI — login password
  bw://item/<id>/username          # bw CLI — login username
  bw://item/<id>/totp              # bw CLI — current TOTP code (time-sensitive)
  bw://item/<id>/notes             # bw CLI — secure-note body (may be multi-line)
  bw://item/<id>/field/<name>      # bw CLI — named custom field
  bw://secret/<uuid>               # bws — whole opaque secret value (NO field selector)
  ```
- **D-04:** **ID-based addressing only** (not by item name). Both `<id>` (bw item id) and
  `<uuid>` (bws secret id) are validated against a strict injection-safe charset
  (hex/UUID + hyphen). Rationale: uniqueness + the strongest validation story, mirroring
  `op://`'s forbidden-char rejection. Reject query strings/fragments and any char outside
  the allowed set.
- **D-05:** Reserved field names (`password`, `username`, `totp`, `notes`) are matched
  exactly; custom fields go through the explicit `field/<name>` prefix so a custom field
  named "password" can never collide with the reserved selector. `<name>` is validated
  (no shell metacharacters; the bw item JSON is parsed, not string-spliced).
- **D-06:** `bws` secrets are single opaque values — **field selectors do not apply** to
  the `secret/` form. `bw://secret/<uuid>/anything` is a validation error.

### Field types supported (D-07)
- **D-07:** For the `bw` item backend: **username, password, TOTP, notes, and custom
  fields** (effectively all). Extraction reads the item as JSON via the `bw` CLI and
  selects the requested field, rather than relying on field-specific subcommands, so one
  code path covers every field type. (Password-only was *not* chosen — full coverage.)

### Failure behavior (D-08)
- **D-08:** **Fail-closed with a clear, actionable diagnostic.** If the required token is
  absent, the CLI binary is missing, the vault is locked, or the item/secret/field is not
  found, the run **aborts** — the secret is never silently skipped or defaulted. The
  diagnostic names the exact remediation (e.g. "`bw://` requires `BW_SESSION`; run
  `bw unlock --raw` and export it"). Matches the fork's fail-secure conventions.

### Security posture (locked by REQ-CRED-01 — carried forward, not re-decided)
- `Zeroizing<String>` for all secret fields, cleared on drop, in-place truncation.
- Clean under `cargo clippy -D clippy::unwrap_used` with no exceptions.
- Follow the `op://` analog for subprocess hygiene: capture CLI stdout, trim, wrap in
  `Zeroizing`; reject injection chars at validation time. (Known limitation inherited from
  `op://`: the intermediate `Vec<u8>` of subprocess stdout is not itself zeroized.)

### Claude's Discretion
- Exact module decomposition within `keystore.rs` (helper fn names, where the JSON parse
  lives), the JSON-parsing approach for `bw get item`, and the precise diagnostic wording.
- Whether `bw`/`bws` are invoked with explicit `--nointeraction --raw`-style flags (follow
  what each CLI documents for non-interactive scripted use).
- Test fixture strategy (mock CLI shim vs. trait-injected command runner) — pick what fits
  the existing keystore test style.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirement & roadmap
- `.planning/REQUIREMENTS.md` §REQ-CRED-01 — the locked requirement (keystore abstraction,
  `Zeroizing<String>`, in-place truncation, isolated to `keystore.rs`, cross-platform).
- `.planning/ROADMAP.md` §"Phase 57: Bitwarden Credential Source" — goal + 3 success
  criteria (no-leak, Zeroizing + unwrap-clean, scheme parity at the keystore boundary).

### Code anchors (the implementation surface)
- `crates/nono/src/keystore.rs` — THE file to change. Study the `op://` path as the direct
  analog: `OP_URI_PREFIX`, `validate_op_uri` (segment + forbidden-char checks),
  `is_op_uri`, `load_from_op`, and the dispatch ladder in `load_secret_by_ref`
  (~line 204). Add `bw://` as a new branch following the same is/validate/load pattern.
  Also note `FORBIDDEN_URI_CHARS`, `redact_keyring_uri` (redaction precedent), and
  `LoadedSecret` / `load_secrets`.
- `crates/nono/src/error.rs` — `NonoError` variants (`SecretNotFound`, `ConfigParse`) used
  for fail-closed diagnostics.

### Project security standards
- `CLAUDE.md` §Security Considerations / §Path Handling — string-prefix-on-paths ban,
  fail-secure, `zeroize` for secrets, no `.unwrap()`/`.expect()`.

### External (web — for the researcher, not repo files)
- Bitwarden `bw` CLI docs (get item/JSON shape, `--session`, `--nointeraction`, `totp`).
- Bitwarden Secrets Manager `bws` CLI docs (`bws secret get <id>`, `BWS_ACCESS_TOKEN`).

No project ADR exists for credential sources — the `op://`/`keyring://` implementations in
`keystore.rs` are the de-facto design reference.
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `keystore.rs` per-scheme pattern (`is_X_uri` / `validate_X_uri` / `load_from_X` + a
  branch in `load_secret_by_ref`) — `bw://` slots straight into this with two load paths.
- `op://` 1Password backend — closest analog (external CLI, injection-char rejection,
  no query/fragment, stdout→`Zeroizing`, fail-closed). Mirror its structure.
- `FORBIDDEN_URI_CHARS` + `validate_op_uri` segment logic — reuse the validation idiom for
  the `<id>`/`<uuid>` and `field/<name>` segments.
- `redact_keyring_uri` — precedent for log-safe rendering of a credential ref.

### Established Patterns
- Single dispatch point (`load_secret_by_ref`) means `bw://` needs exactly one new branch;
  keeps the surface "isolated to keystore.rs" as REQ-CRED-01 demands.
- Secrets always returned as `Zeroizing<String>`; callers set env vars from `.as_str()`.

### Integration Points
- `load_secret_by_ref` dispatch ladder (~`keystore.rs:204`) — add the `bw://` branch
  (before the keyring fallthrough).
- No changes above the keystore layer: `--credential` / profile `secrets` mappings already
  flow arbitrary scheme strings into `load_secrets` unchanged.
</code_context>

<specifics>
## Specific Ideas

- Two backends, one scheme, distinguished by a **typed first path segment**
  (`item/` vs `secret/`) — see D-03 grammar block. This is the operator-facing contract.
- `bw://item/<id>/field/<name>` is the deliberate, collision-proof custom-field selector.
- Diagnostics must name the exact env token + unlock command for the relevant backend.
</specifics>

<deferred>
## Deferred Ideas

- **Non-interactive auto-unlock** (passphrase/client-secret in env → `bw unlock`):
  considered and **rejected** for Phase 57 (larger threat surface). Could be revisited as
  an opt-in future enhancement if operators ask for it.
- **Addressing items by name** (vs id): rejected for validation-safety reasons; a future
  phase could add an explicit, validated name-lookup mode if desired.
- Credential *writing*, session caching/daemonization, org/collection management — out of
  scope; separate capabilities.

None of these block Phase 57.
</deferred>

---

*Phase: 57-bitwarden-credential-source*
*Context gathered: 2026-06-05*
