---
phase: 26
slug: pkg-streaming-followup
status: verified
threats_open: 0
asvs_level: 1
created: 2026-05-09
---

# Phase 26 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail. Built retroactively from Plan 26-01 + Plan 26-02 STRIDE registers, verified against UAT evidence + targeted code spot-checks.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Manifest JSON → `install_manifest_artifact` (Plan 26-01) | Untrusted `package.json` artifact entries cross into filesystem-write operations. `artifact.path` and `artifact.artifact_type` are attacker-controlled when the pack is hostile. | Strings (path, type discriminator) |
| Registry HTTP endpoint → `RegistryClient::download_artifact_to_path` (Plan 26-02) | Untrusted network response streams into a `tempfile::TempDir` PathBuf. Tampered bytes, oversized payloads, hung connections all originate here. | Bytes (artifact body), JSON (manifest, bundle) |
| Content-Length pre-check + `with_config().limit()` reader cap (Plan 26-02) | Mid-stream size gate. Defense-in-depth against memory-bomb DoS (T-26-02-02). Fixed-const REGISTRY_*_LIMIT_BYTES (2 MiB JSON / 8 MiB bundle / 64 MiB artifact). | u64 (Content-Length header), bytes (stream prefix) |
| `extends: ["registry://..."]` → `load_registry_profile` → `run_pull` (Plan 26-02) | Profile-controlled URI triggers network pull through the same trust path as direct `nono pull`. | String (registry-ref) |

---

## Threat Register

### Plan 26-01 — Fork-architectural decisions (PKGS-02, PKGS-03)

| Threat ID | Category | Component | Disposition | Mitigation | Status | Evidence |
|-----------|----------|-----------|-------------|------------|--------|----------|
| T-26-01-01 | Tampering | manifest `artifact.path` with `..` traversal | mitigate | `validate_relative_path` rejects `..` Path components at input-string layer BEFORE any filesystem syscall. Defense-in-depth: `validate_path_within` rejects post-canonicalization. | closed | UAT Test 6 — `validate_relative_path_rejects_traversal` passed |
| T-26-01-02 | Tampering | manifest `artifact.path` with absolute path | mitigate | `validate_relative_path` rejects Unix `/foo` and Windows `C:\foo` + `\\server\share` shapes at input-string layer. | closed | UAT Test 6 — `validate_relative_path_rejects_absolute_path` passed |
| T-26-01-03 | Tampering | manifest `artifact.path` with symlink-traversal | mitigate | `validate_path_within` (canonicalize-and-component-compare) at `package_cmd.rs:1043`; preserved verbatim by Plan 26-01 (D-20 manual replay rationale). | closed | Plan 26-01 SUMMARY confirms `validate_path_within` preserved at line 1043; covered by existing v2.2 regression tests in `crates/nono-cli/src/package_cmd.rs` |
| T-26-01-04 | Tampering | manifest `artifact_type` with unknown variant string | mitigate | serde deserializer fails-closed on unknown variants (`#[serde(rename_all = "snake_case")]` enum does not silently coerce). | closed | UAT Test 5 — `artifact_type_unknown_fails_closed` passed |
| T-26-01-05 | Tampering | new Plugin artifact_type with `..` traversal in path | mitigate | Plugin arm at `package_cmd.rs:752` uses `staging_root.join("plugins").join(file_name(&artifact.path)?)` — same `file_name` helper as Hook (L712), Instruction (L720), Script (L747) arms. `file_name` extracts only the basename, defeating in-string `..`. Post-match `validate_path_within` fires on the produced `store_path`. | closed | Code spot-check: `grep -n 'file_name(&artifact.path)' package_cmd.rs` shows L761 inside Plugin arm (alongside L712, L720, L747 for sibling arms); Plan 26-01 SUMMARY confirms |
| T-26-01-06 | Information Disclosure | regression risk: live Plugin arm copying the deferred-divergence comment's unsafe `staging_root.join(&artifact.path)` example | mitigate | Live arm at L752-761 mirrors the Script arm shape (`file_name` helper), NOT the comment's example shape. Comment block at lines 671-688 was deleted atomically with arm addition (Plan 26-01 commit `797f3295`). | closed | Plan 26-01 SUMMARY confirms `grep -c 'upstream ec49a7af also adds an ArtifactType::Plugin' package_cmd.rs = 0` (deferred-divergence comment removed) |
| T-26-01-07 | Repudiation | absence of D-19 cherry-pick provenance trailers | mitigate | All cherry-pick commits include `Upstream-commit:` trailers (or `Upstream-commit: ... Upstream-replay: manual` pair for D-20 manual replays). All commits include `Signed-off-by:` DCO line per CLAUDE.md § Coding Standards. | closed | `git log --format='%b' 9cb7770f..c00254d9` shows `Upstream-commit: 115b5cfa`, `Upstream-commit: 9ebad89a (replayed manually)`, `Upstream-replay: manual`, plus `Signed-off-by:` on every commit |
| T-26-01-08 | Elevation of Privilege | `crates/nono/` modification slipping in via cherry-pick (widening surface beyond `nono-cli`) | mitigate | D-19 byte-identical preservation enforced via Gate 5: `git diff --stat 57be91a9..HEAD -- crates/nono/` MUST return empty. | closed | UAT Test 3 — `git diff --stat` returned empty diff |

### Plan 26-02 — Streaming + Auto-pull (PKGS-01, PKGS-04)

| Threat ID | Category | Component | Disposition | Mitigation | Status | Evidence |
|-----------|----------|-----------|-------------|------------|--------|----------|
| T-26-02-01 | Tampering | registry returns tampered artifact bytes | mitigate (BLOCKING) | Streaming SHA-256 incremental in `RegistryClient::download_artifact_to_path`; mismatch rejects BEFORE `install_artifacts` copies bytes from TempDir to install_dir. | closed | UAT Test 9 — `download_artifact_to_path_computes_digest_of_streamed_bytes` passed |
| T-26-02-02 | Denial of Service | memory bomb (registry returns 10 GB artifact) | mitigate (BLOCKING) | Content-Length pre-check + `with_config().limit()` reader cap at fixed-const REGISTRY_*_LIMIT_BYTES (2 MiB JSON / 8 MiB bundle / 64 MiB artifact). Per Plan 26-02 Deviation #2, upstream-aligned fixed-const approach replaces the plan's original `--max-size` flag. Surfaces violations as `NonoError::RegistryError(format!(...))`. | closed | UAT Test 9 — `download_artifact_to_path_rejects_oversize_via_content_length` + 3× `enforce_content_length_*` boundary tests all passed |
| T-26-02-03 | Denial of Service | hung connection (registry never responds) | mitigate | ureq Agent timeouts: 10 s connect / 30 s response / 300 s body / 300 s global. | closed | UAT Test 9 — `registry_client_connect_timeout_fires_within_bounded_window` passed (10.02 s wall time = 10 s connect-timeout budget) |
| T-26-02-04 | Denial of Service | misconfigured `--max-size` flag (e.g. 0 bytes) | mitigate → N/A | Original threat assumed a `--max-size` flag would exist. Per Plan 26-02 Deviation #2, the flag was NOT added (upstream alignment with fixed-const REGISTRY_*_LIMIT_BYTES). The threat scenario is therefore N/A — there is no runtime config surface to misconfigure. Tightening/loosening the cap requires source-code modification + recompile, which is auditable through Sigstore-signed binary distribution. | closed | UAT Test 4 — `nono pull --help` shows no `--max-size` flag (upstream alignment confirmed) |
| T-26-02-05 | Tampering | hostile profile injects `extends: ["registry://attacker/exfil@1.0.0"]` | accept | Auto-pull path uses the SAME trust path as direct `nono pull`: signed-artifact verification + bundle subjects check + namespace assertion (per existing PKG-04). Profile content trust is upstream's stance; v2.3 does not re-litigate. | closed (accepted) | See Accepted Risks Log entry AR-26-01 |
| T-26-02-06 | Information Disclosure | TempDir-staged artifact bytes readable by other users on multi-user host | mitigate | `tempfile::TempDir` defaults to `0o700` perms on Unix; on Windows uses default ACL inheriting from user profile. Per-pull TempDir lives on the `VerifiedDownloads` wrapper; Drop fires unconditionally on success/error/panic. | closed | tempfile crate semantics (documented `0o700` default); Plan 26-02 SUMMARY confirms `_tempdir: TempDir` on `VerifiedDownloads`; T-26-02-07 panic test exercises the Drop path |
| T-26-02-07 | Tampering | panic mid-stream leaves partial bytes in TempDir; race-to-read | mitigate | `TempDir` Drop runs on panic (unconditional). No partial bytes survive after a mid-stream panic. | closed | UAT Test 9 — `tempdir_cleanup_runs_on_panic` passed |
| T-26-02-08 | Repudiation | cherry-pick provenance lost | mitigate | D-19 trailers enforced on Tasks 3 + 4. T3 commit body contains `Upstream-commit: 9ebad89a (replayed manually)` AND `Upstream-replay: manual`. T4 contains `Upstream-commit: 115b5cfa`. | closed | `git log --format='%b' 9cb7770f..c00254d9` confirms both trailer pairs present |
| T-26-02-09 | Tampering | bundle JSON tampered then committed to `bundle_json` field (wider blast radius) | mitigate | `bundle_json` is parsed via `nono::trust::load_bundle_from_str` BEFORE the string is committed to the `VerifiedDownloads` wrapper. If parse fails, `?` returns early and the field is never populated. | closed | Code spot-check: `package_cmd.rs:463-464` shows `let bundle_json = client.download_bundle(...)?; let bundle = nono::trust::load_bundle_from_str(&bundle_json, ...)?;` — parse precedes wrapper-struct construction at L530 |
| T-26-02-10 | Elevation of Privilege | auto-pull triggered by profile resolve in unprivileged context, but pull writes to install_dir under elevated user | accept | Auto-pull inherits the calling user's permissions (matches direct-pull behavior). No privilege escalation introduced. | closed (accepted) | See Accepted Risks Log entry AR-26-02 |

*Status: open · closed (mitigated/N/A) · closed (accepted)*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-26-01 | T-26-02-05 | A hostile profile that references `registry://attacker/exfil@1.0.0` triggers auto-pull through the same trust path as direct `nono pull` — Sigstore-verified signature + bundle subjects check + namespace assertion enforced by upstream-derived PKG-04. The marginal risk introduced by auto-pull (vs direct invocation) is the absence of an explicit user prompt before the network round-trip. The v2.3 milestone does not re-litigate upstream's profile-content trust posture. Future hardening would either (a) prompt before each auto-pull or (b) require an allow-list of `registry://` namespaces in user config. | Plan 26-02 (accepted disposition); confirmed during Phase 26 secure audit | 2026-05-09 |
| AR-26-02 | T-26-02-10 | Auto-pull inherits the calling user's permissions. If the user invokes `nono` under an elevated context (e.g. via `sudo nono ...`), the auto-pull writes to that elevated user's install_dir — same behavior as direct-pull. There is no privilege escalation introduced by the auto-pull dispatcher; the threat is structurally equivalent to existing direct-pull risk and is bounded by OS process privilege model. | Plan 26-02 (accepted disposition); confirmed during Phase 26 secure audit | 2026-05-09 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-05-09 | 18 | 18 (16 mitigated + 2 accepted) | 0 | /gsd-secure-phase 26 (orchestrator inline; no auditor spawn — threats_open shortcut) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-05-09
