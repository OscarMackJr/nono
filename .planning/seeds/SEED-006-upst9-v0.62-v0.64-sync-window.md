---
id: SEED-006
status: dormant
planted: 2026-06-19
planted_during: v3.0 close / quick 260619-duw
trigger_when: milestone scope includes upstream sync, UPST9, divergence audit, or cherry-picking from always-further/nono
scope: large
priority: P2
---

# SEED-006: UPST9 — Upstream `v0.62.0..v0.64.0` Sync Window (new/modified functions)

## Why This Matters

The fork's confirmed upstream-sync high-water mark is upstream **`v0.61.2`** (UPST8 / v2.11). Upstream `always-further/nono` has since cut **`v0.63.0`** and **`v0.64.0`**. This seed scopes the **`v0.62.0..v0.64.0`** delta — **90 commits, 140 files changed** — into a function-level inventory so the next upstream-sync milestone (UPST9) can audit + cherry-pick without re-discovering the surface from scratch.

**The headline risk:** this window contains a **major architectural relocation** — upstream moved the *entire audit/attestation/ledger stack (~1773 LOC)* **and** a *new structured-diagnostics model* **into the core `nono` library crate**. That cuts directly against the fork's load-bearing invariant that the **library is a pure, policy-free sandbox primitive** (`CLAUDE.md` § Library vs CLI Boundary: "audit"/"diagnostic UX" live in the CLI). UPST9 cannot blind-cherry-pick these clusters; they need a divergence-ledger disposition (likely `split` or `fork-preserve`) — exactly the re-export-surface hazard recorded in [[feedback_cluster_isolation_invalid]].

## When to Surface

**Trigger:** when a milestone targets the UPST9 upstream-sync cadence, an `always-further/nono` divergence audit, or a cherry-pick wave. Mirrors the Phase 42/47/48 (`DIVERGENCE-LEDGER.md`) shape.

This seed will surface during `/gsd:new-milestone` when the milestone scope matches.

## Scope Estimate

**Large.** Window = upstream `v0.62.0..v0.64.0` (90 commits; releases `v0.63.0` #1161, `v0.64.0` #1201). Re-fetch upstream at audit-open in case `v0.64.x`/`v0.65.0` land first. Most of the 90 commits are dependabot/docs/merge noise; the substantive engineering is the 13 themes below. Two themes (A, B) are HIGH-conflict library-boundary refactors; the rest are mostly additive features + Linux/proxy fixes.

## Delta by Theme (real new/modified functions)

> Compare base: `gh api repos/always-further/nono/compare/v0.62.0...v0.64.0`. SHAs are upstream.

### A. ⚠️ Audit/attestation/ledger MOVED INTO core `nono` crate — HIGH fork-conflict
Commits `a5b2a516` (move audit integrity), `aed35bec` (move audit ledger), `0b27cfc2` (#1148 move attestation), `e9529312` (ledger review/clippy).
- **NEW `crates/nono/src/audit.rs` (+1773/-0)** — `AuditRecorder::{new, new_with_policy, record_session_started, record_session_ended, record_capability_decision, record_open_url, record_network_event, event_count, finalize, append_event}`; merkle/hash `{hash_event, hash_chain, merkle_root, build_inclusion_proof, verify_inclusion_proof, hash_merkle_node, compute_session_digest}`; ledger `{validate_ledger_session_id, append_session_to_ledger_file, missing_ledger_verification_result, verify_session_in_ledger_reader}`; attestation `{sign_audit_attestation_bundle, verify_audit_attestation_bundle, verify_audit_log, extract_audit_attestation_statement, hash_ledger_link}`.
- CLI shrank to thin wrappers: `audit_integrity.rs` (+4/-396), `audit_attestation.rs` (+36/-273), `audit_ledger.rs` (+40/-286, keeps `verify_session_in_ledger`), `audit_session.rs` (+116/-22).
- **Fork note:** the fork's `nono` crate has NO audit module today (audit is CLI-side). This is a boundary-divergence decision for UPST9, not a mechanical cherry-pick.

### B. ⚠️ Structured diagnostics model in core lib + FFI exposure — HIGH fork-relevance
Commits `4ad8ba92` (#1155 move diagnostic UX out of core), `a6aa5995` (#1171 expose structured diagnostics for library + FFI), `f867aba2` (#1150 report actual blocked op, not readable path).
- **NEW `crates/nono/src/diagnostic/` module:** `codes.rs` (+188 — `NonoDiagnostic::{new, with_hint, with_remediation, with_path_access, with_detail}`, `suggested_flag_for_remediation`), `observation.rs` (+199 — `into_diagnostics`, `follow_up_diagnostics`, `diagnostic_{likely_sandbox_path, missing_path, application_failure, protected_file_write, network_blocked}`), `records.rs` (+168 — `DenialRecord::new`, `seatbelt_operation_to_access`), `report.rs` (+446 — `SessionDiagnosticReport::{from_session, from_merged_session, to_json, merge_with_proxy_json, dedupe_denials}`, `filesystem_denials_from_violations`), `detail.rs`, `mod.rs`.
- `crates/nono/src/error.rs` (+137) — `NonoError::{diagnostic_code, remediation}` (maps errors → structured codes/remediation).
- **CLI:** `crates/nono-cli/src/diagnostic/formatter.rs` (renamed, +745/-496) — `with_session_diagnostics`, `with_session_report`, `build_session_report`, many `format_*_from_diagnostic` helpers; `output.rs` `print_proxy_diagnostics`.
- **FFI (touches the fork's `bindings/c`):** NEW `bindings/c/src/diagnostic.rs` (+222 — `parse_denials/parse_ipc_denials/parse_violations`), `lib.rs` `last_diagnostic_code`/`last_remediation_json`, `types.rs` `NonoDiagnosticCode`.
- **Proxy:** NEW `crates/nono-proxy/src/diagnostic.rs` (+99 — `ProxyDiagnostic::{warning, with_credential_ref, with_hint}`, `ProxyDiagnosticCode::as_str`).
- **Fork note:** fork keeps `DiagnosticFormatter` in the CLI per CLAUDE.md. Reconcile with the fork's Windows diagnostic paths.

### C. 🔒 Linux AF_UNIX datagram bypass fix — SECURITY (Linux-only; cross-target clippy applies)
Commits `e2086877` (#1096 trap sendto/sendmsg), `6b3eb013` (#1064 guard `deduplicate()` against procfs-remap originals).
- `crates/nono/src/sandbox/linux.rs` (+256/-95) — `read_msghdr_dest`, `read_mmsghdr_dests`; seccomp filter now traps `sendto`/`sendmsg`/`sendmmsg`.
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (+394/-49) — AF_UNIX pathname send-grant enforcement.
- `crates/nono/src/capability.rs` (+133/-6) — `is_procfs_remap_original`; unix-socket send covered by connect grant.

### D. `set_vars` static env injection (feat #1134, `d48aeb7b`)
- `capability.rs` `set_vars`; `exec_strategy.rs` `push_set_vars`; NEW `exec_strategy/env_sanitization.rs` (+111 — `validate_set_vars`, `is_valid_env_var_name`; rejects `PATH` + `NONO_` prefix); `profile_runtime.rs` `expand_profile_set_vars`.

### E. Runtime state → XDG state dirs (feat #1152 `e8293b36`, #1179 `8e0d94f9`)
- NEW `crates/nono-cli/src/state_paths.rs` (+422) — `user_state_dir`, `audit_root`, `sessions_dir`, `rollback_root` + `legacy_*` variants, `maybe_migrate_legacy_audit_ledger`, discovery-root + legacy-path-warning helpers. Moves runtime state out of `~/.nono` with legacy fallback + migration. **Fork note:** Windows path resolution — verify against the fork's `%USERPROFILE%`/scratch-space provisioner (v3.0 DEPLOY work).

### F. Proxy hardening cluster
- `route.rs` (+229/-41) `select_route` — `allow_domain` endpoint route no longer shadows credential catch-all (#1132 `b0b2c743`).
- `server.rs` (+450/-69) `parse_non_connect_target`, `diagnostics`, `diagnostics_json`, `normalize_authority` — 403+audit for denied non-CONNECT (#1077 `a5d623fd`), reactive-auth keep-alive on CONNECT (#1151 `7c9abd3b`).
- `tls_intercept/handle.rs` (+298/-171) `select_upstream_strategy`, `resolve_upstream_or_deny`, `parse_inner_request`, `handle_inner_request` — respect `upstream_proxy` in TLS CONNECT (#1048/#1091 `b5f8db5c`), `Refactor forward_inner_request` (#1192 `76b7b695`), separate proxy intent from activation (#1199 `bd4b6b7f`). **Touches the fork-divergent TLS-interception surface (Phase 34 C11 `fork-preserve`) — diff-inspect.**
- `credential.rs` (+395/-58) `load_with_diagnostics`, `into_store`, `get_aws` — proxy activates with `customCredentials` set (#1197 `724bb207`).

### G. AWS auth config (feat [aws] #1166, `5bb098cd`)
- `crates/nono-proxy/src/config.rs` (+133) `AwsAuthConfig`; `profile/mod.rs` `validate_aws_auth` (mutually exclusive with `credential_key`/`oauth2`).

### H. Keyring timeout (feat #977, `c6b13345`)
- `keystore.rs` (+251/-50) `keyring_timeout`, `call_with_keyring_timeout` — `NONO_KEYRING_TIMEOUT_SECS` (default 120s, `0`=none).

### I. Store-pack session hooks / `$PACK_DIR` (feat #1073, `7d274cf7`)
- `profile/mod.rs` `resolve_store_pack_session_hooks` — `$PACK_DIR` expansion + `source_pack` propagation.

### J. PTY ctrl-z fix (fix #1135, `4179ce03`)
- `pty_proxy.rs` (+170/-9) `leave_screen_for_suspension`, `reenter_screen_for_resume`, `master_fd`, `take_suspension_request`, `restore_terminal`; `exec_strategy.rs` `signal_pty_foreground_group`, `handle_pty_suspension`.

### K. update-check CI discovery (feat #1113, `cc11b389`)
- `update_check.rs` (+195/-4) `detect_ci_provider`, `env_marker_present`.

### L. Profile namespace standardization (refactor `6d88638e`)
- `profile/mod.rs` (+1199/-18, mostly tests) `display_user_profiles_dir`, `display_trust_policy_path`; namespaced profile names.

### M. Misc fixes (low conflict risk)
- Truthy env for bool flags (#1136 `42e5bf73`): `cli.rs` `capability_elevation`/`trust_override`/`trust_proxy_ca`.
- Show blocked macOS grants in capability summary (#1178 `a0bba5eb`): `output.rs` `print_blocked_grants`.
- `go_runtime` go-build cache rw (#1173 `5413a0b3`); stale schema domains `nono.dev`→`nono.sh` (`ee7a3bda`).
- **Remove `env_clear` from session_hook subprocess (`e54cf9cb`)** — relates to the fork's Windows `env_clear` CLR-fail gotcha ([[windows_hook_interpreter_spawn_gotchas]]); review intent vs the fork's `SystemRoot`/`windir` baseline.
- `9800f307` skip pack verification on dry runs; `314bd74e` WSL2 landlock-network V4+ detection.

### Dependency bumps (note; pin/verify per [[project_workspace_crates]] 5-crate version sync)
x509-parser `0.16.0→0.18.1`, hyper `1.9.0→1.10.1`, cbindgen `0.29.2→0.29.4`, typify `0.6.2→0.7.0`, zeroize `1.8.2→1.9.0`, time `0.3.47→0.3.49`, chrono `0.4.44→0.4.45`, ignore `0.4.25→0.4.26`, which `8.0.2→8.0.3`.

## Breadcrumbs

- **Window:** upstream `always-further/nono` `v0.62.0..v0.64.0` — `gh api repos/always-further/nono/compare/v0.62.0...v0.64.0`. Fork crate currently `0.62.2`; sync high-water `v0.61.2` (UPST8 / v2.11).
- **Process precedent:** `DIVERGENCE-LEDGER.md` (Phase 42/47/48). Diff-inspect re-export surfaces, not just `--name-only` ([[feedback_cluster_isolation_invalid]]). Honor the Windows-only-files invariant (D-43-E1) — none of these upstream files are Windows backends, so the fork's Windows surface is untouched, but `cfg(linux)`/`cfg(macos)` edits MUST clear cross-target clippy ([[feedback_clippy_cross_target]]).
- **Library boundary:** `CLAUDE.md` § Library vs CLI — themes A & B violate the fork's policy-free-library invariant; need an explicit ADR/disposition.
- **Fork version leapfrog:** fork must keep its crate version PAST upstream's highest tag for a collision-free release trigger ([[project_v28_opened]]); upstream is now `0.64.0`, fork `0.62.2` — a release would need to leapfrog ≥ `0.65.0`.

## Notes

Captured 2026-06-19 at v3.0 close (quick `260619-duw`), in response to "upstream main is now at 0.64; scope the delta from 0.62." UPST9 was an explicit deferral in both v2.13 and v3.0 ("separate cadence"). Sibling seeds: [[SEED-001]], [[SEED-002]], [[SEED-003]], [[SEED-004]], [[SEED-005]] (all enterprise-horizon; this one is the upstream-sync cadence, orthogonal).
