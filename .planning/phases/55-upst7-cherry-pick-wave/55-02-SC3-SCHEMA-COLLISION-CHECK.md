# SC3 Schema-Collision Check — Plan 55-02 (C7 Cluster)

**Produced:** 2026-06-04  
**Plan:** 55-02-PROFILE-JSONC-TARGET-BINARY  
**Authority:** Phase 36 canonical sections (36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md + 36-01c-OVERRIDE-DENY-RENAME-SUMMARY.md)  
**Upstream commits inspected:** 53a0c521 (JSONC), 9398a139 (target_binary), 2bd9b4d5 (opencode removal)  
**Purpose:** Verify no schema or deserialization collision before any C7 cherry-pick lands.

---

## Check 1: target_binary field (commit 9398a139) — nono-profile.schema.json collision

**Question:** Does the `binary` field (upstream name for `target_binary`) conflict with any existing field in the fork's `nono-profile.schema.json`?

**Inspection:**

- Upstream commit 9398a139 adds `pub binary: Option<String>` to `Profile` + `ProfileDeserialize` in `profile/mod.rs` and `binary: None` to `policy.rs::ProfileDef::to_raw_profile`. It does NOT touch `nono-profile.schema.json` — the upstream does not add a `binary` field to the schema file.
- Fork's current `nono-profile.schema.json` (inspected): properties enumerated are `$schema`, `extends`, `meta`, `security`, `filesystem`, `commands`, `policy`, `network`, `linux`, `env_credentials`, `secrets`, `workdir`, `hooks`, `rollback`, `undo`, `open_urls`, `allow_launch_services`, `interactive`, `capabilities`, `unsafe_macos_seatbelt_rules`, `windows_low_il_broker`, `packs`, `command_args`. No `binary` property exists.
- The schema has `additionalProperties: false` at the top level. After cherry-pick, the fork must NOT add `binary` to the schema unless the cherry-pick itself adds it. Since upstream 9398a139 does not modify the schema file, the new `binary` field in the Rust struct is not schema-documented upstream either.
- `override_deny → bypass_protection` rename (36-01c): the `binary` field is completely orthogonal — it does not alias, rename, or collide with `bypass_protection` or any `override_deny` surface.
- Windows-specific fork additions in the schema (`windows_low_il_broker`) are unrelated to `binary`.

**Sub-checks:**
- (a) Field does not already exist under a different name: CONFIRMED — no `binary` or `executable` or similar field in the current schema.
- (b) Does not alias a fork-renamed field: CONFIRMED — no alias relationship with `bypass_protection` / `override_deny`.
- (c) Does not collide with Windows-specific fork additions: CONFIRMED — `windows_low_il_broker` is unrelated.

**VERDICT: CLEAR**

No collision. The `binary` field is a new struct field only (no schema change); the cherry-pick applies cleanly without touching `nono-profile.schema.json`.

---

## Check 2: JSONC parsing change (53a0c521) — From\<ProfileDeserialize\> for Profile canonical match

**Question:** Does the JSONC parsing change conflict with the fork's `From<ProfileDeserialize> for Profile` canonical match (36-01b) or the fork's WR-01 fail-closed security logic?

**Inspection:**

Upstream 53a0c521 makes the following key changes in `profile/mod.rs`:
1. Replaces `serde_json::from_slice` with `jsonc_parser::parse_to_serde_value` inside `parse_profile_bytes`.
2. Adds `resolve_user_profile_path` (prefers `.jsonc` over `.json`), renaming callsites from `get_user_profile_path`.
3. Extends `list_profiles` to enumerate `.jsonc` files.
4. Adds JSONC test + jsonc extension test.

**Fork-specific deserialization context (36-01b authority):**

The fork's `parse_profile_bytes` has a CRITICAL 5-step security sequence (WR-01 / Plan 36-04b):
1. `from_utf8` — bytes to str.
2. `detect_legacy_override_deny_key` — one-time deprecation warning.
3. `raw_profile_has_both_bypass_and_override_keys` — FAIL-CLOSED dual-key check (MUST precede deserialize).
4. `serde_json::from_str` — typed deserialize.
5. `validate_profile_custom_credentials` + `validate_env_credential_keys` + `validate_profile_aipc_tokens`.

**Collision analysis:**

The upstream JSONC commit replaces step 4 (`serde_json::from_str`) with `jsonc_parser::parse_to_serde_value`. This is a SAFE substitution — the JSONC parser still produces a `serde_json::Value` which is then deserialized via serde. The fork's WR-01 steps 2+3 operate on the raw string content (before deserialization) and are UNAFFECTED by the parser swap.

During cherry-pick conflict resolution, the fork must:
- RETAIN steps 1, 2, 3 verbatim (fork-specific WR-01 security, not in upstream).
- REPLACE the `serde_json::from_str` call in step 4 with the upstream's `jsonc_parser::parse_to_serde_value` call.
- RETAIN step 5 (validation calls, same in both upstream and fork).
- Add `resolve_user_profile_path` as a new function alongside (not replacing) the existing `get_user_profile_path` (which is still needed for write paths).

**From\<ProfileDeserialize\> for Profile check (36-01b):**

The canonical match was extended in 36-01b to include `commands` (CommandsConfig). The upstream 53a0c521 commit does NOT modify `From<ProfileDeserialize> for Profile` — it only modifies parsing (the format layer). No collision with the exhaustive-enumeration gate.

**Note on `parse_profile_file`:** The upstream commit does NOT modify `parse_profile_file`. The fork's `parse_profile_file` calls `serde_json::from_str` via `parse_profile_bytes` (via intermediate code). The JSONC parser change applies to `parse_profile_bytes` specifically.

**jsonc-parser crate legitimacy check (T-55-02-SC):**
- Crate: `jsonc-parser 0.32` (features: `serde`)
- Author: David Sherret — creator of dprint, the Rust JSONC/TypeScript formatter project; same author as `dprint-plugin-json`, `jsonc-parser` on npm/crates.io.
- Widely used in the Rust ecosystem (dprint, biome, etc.). No typosquatting concern.
- `serde` feature is required for `parse_to_serde_value` — correct feature selection.
- VERDICT: LEGITIMATE — proceed with Cargo.toml addition.

**VERDICT: CLEAR (with fork-specific conflict-resolution instruction)**

No collision with `From<ProfileDeserialize>` or canonical sections. The cherry-pick requires retaining the fork's WR-01 security steps (2+3) around the upstream's JSONC parser substitution. This is an expected merge conflict in `parse_profile_bytes` — resolve by keeping the fork's step-2+3 wrapper while substituting step-4.

---

## Check 3: opencode removal (2bd9b4d5) — policy.json canonical sections

**Question:** Does removing the `opencode` profile from `data/policy.json` leave any dangling reference? Does `nono-profile.schema.json` still require the `opencode` profile?

**Inspection:**

Upstream 2bd9b4d5:
- Removes the `opencode` profile definition block (~32 lines) from `data/policy.json`.
- Adds an `OfficialPack` entry for `always-further/opencode` in `migration.rs`.
- Updates `profile/builtin.rs` — removes opencode from built-in list, updates tests.
- Updates `profile/mod.rs` — adjusts tests that used opencode as a representative built-in profile to use `openclaw` or `swival`.
- Updates `profile_cmd.rs` and `profile_save_runtime.rs` — adjusts test fixtures.

**Schema dependency check:**

`nono-profile.schema.json` does NOT define any profile-specific entries — it defines the JSON structure of a profile file, not which profiles exist. It has no reference to "opencode" anywhere. Removing the opencode definition from `policy.json` does not affect the schema at all.

**Canonical sections check (36-01b/36-01c authority):**

The fork's canonical sections per 36-01b are:
- `CommandsConfig { allow, deny }` — unaffected by opencode removal
- `FilesystemConfig.deny + bypass_protection` — unaffected
- `Profile.commands` + `From<ProfileDeserialize>` exhaustive map — unaffected

None of these sections are defined inside the `opencode` profile JSON object. The opencode removal only deletes a profile instance, not a schema definition or canonical struct.

**Other canonical groups in policy.json:**

Upstream 2bd9b4d5 removes only the `"opencode"` key from `data/policy.json`'s profiles section. The `"opencode_linux"` group (at line 380) is a POLICY GROUP (not a profile) — the upstream commit does NOT remove this group. During cherry-pick, verify the diff touches only the profile definition block, not the group definitions.

**`override_deny → bypass_protection` surface:**

The opencode profile in `policy.json` uses `bypass_protection` (canonical key since 36-01d). No `override_deny` keys are in the opencode profile definition. No rename-surface collision.

**Tests in builtin.rs:**

The upstream commit changes tests to use `swival` or `openclaw` instead of `opencode` as the representative built-in profile. The fork's builtin.rs currently has tests that reference opencode (confirmed by grep). These test changes are straight-port; no fork-specific conflict expected since the fork does not add Windows-specific logic to opencode tests.

**VERDICT: CLEAR**

The opencode removal is a clean profile-instance removal. No canonical schema sections, struct definitions, or `bypass_protection`/`override_deny` rename surfaces are affected. The `opencode_linux` GROUP in `policy.json` is NOT removed (only the profile). `nono-profile.schema.json` is completely unaffected.

---

## Check 4: Phase 36 canonical-section enumeration preservation after C7

**Question:** Do the C7 changes preserve the exhaustive `From<ProfileDeserialize> for Profile` enumeration (the T-36-01-CANONICAL compile-time gate)?

**Inspection:**

Upstream 9398a139 adds `binary: raw.binary` to `From<ProfileDeserialize> for Profile`. This is an ADDITIVE extension — it adds a new field to both `Profile` and `ProfileDeserialize` and maps it in `From`. This is exactly the pattern established in 36-01b.

The canonical section invariant requires ALL Profile fields to appear in the `From` impl. After cherry-pick:
- `commands: raw.commands` (36-01b) — present
- `binary: raw.binary` (9398a139) — added by cherry-pick
- All other existing fields — preserved

The compile-time gate (if a field is added to `Profile` without adding it to `From<ProfileDeserialize>`, Rust's struct-literal completeness check will error) remains effective. The upstream adds `binary` to both sides, so the gate is satisfied.

**VERDICT: CLEAR**

---

## Summary Table

| Item | Upstream Commits | Verdict | Action Required |
|------|-----------------|---------|----------------|
| 1. target_binary schema collision | 9398a139 | **CLEAR** | None — no nono-profile.schema.json changes in upstream or fork |
| 2. JSONC deserialization collision | 53a0c521 | **CLEAR** (with merge note) | During cherry-pick: retain fork's WR-01 steps 2+3 in parse_profile_bytes around the JSONC parser substitution |
| 3. opencode removal — canonical sections | 2bd9b4d5 | **CLEAR** | None — opencode_linux GROUP is preserved; only profile instance removed |
| 4. From\<ProfileDeserialize\> enumeration preserved | 9398a139 | **CLEAR** | binary field added to both Profile + From impl — compile gate intact |
| jsonc-parser crate legitimacy | 53a0c521 | **CLEAR** | Legitimate crate by David Sherret; version 0.32 with serde feature |

**Overall verdict: ALL CLEAR — proceed with C7 cherry-picks.**

No COLLISION found. The fork-specific merge instruction for Check 2 (retain WR-01 steps 2+3 in `parse_profile_bytes`) is an expected conflict resolution, not a blocking collision — it ensures the fork's security invariant is preserved during the JSONC parser substitution.

---

## Conflict-Resolution Pre-flight for Cherry-Pick Execution

The following merge conflicts are EXPECTED (not collisions — expected divergence due to fork additions):

### profile/mod.rs — parse_profile_bytes (53a0c521)
The upstream replaces only `serde_json::from_slice` with `jsonc_parser::parse_to_serde_value`.  
The fork has a 5-step security wrapper. Resolution: keep steps 1+2+3+5; replace step 4 with JSONC parser call. The JSONC `parse_to_serde_value` returns `Result<Option<serde_json::Value>, _>` — chain with `.unwrap_or(serde_json::Value::Null)` and then `serde_json::from_value::<Profile>` OR use the `parse_profile_bytes` signature from the upstream which calls `jsonc_parser::parse_to_serde_value` + `serde_json::from_value`. Check exact upstream signature to confirm.

### profile/mod.rs — binary field addition (9398a139)
The fork has additional fields (e.g. `windows_low_il_broker` from Phase 51, `commands` from 36-01b). The `binary` field should be added AFTER `packs` and BEFORE `command_args` (matching upstream position). The `From<ProfileDeserialize>` impl should add `binary: raw.binary` AFTER `packs: raw.packs`.

### policy.rs — binary: None addition (9398a139)
The fork's `ProfileDef::to_raw_profile` has additional fields from 36-01b (`commands: CommandsConfig::default()`). Add `binary: None` after `packs: self.packs.clone()`.

---

*SC3 check complete — 2026-06-04. All items CLEAR. C7 cherry-picks may proceed.*
