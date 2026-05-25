## Plan 48-05: Cluster C6 — macOS Grant Restore + Localhost Outbound

This section covers the **3 commits comprising Cluster C6** (macOS exact-path/future-file grant
restore + `open_port 0` localhost outbound wildcard) cherry-picked from upstream `always-further/nono`
into the fork. Cluster C6 is Wave 2 of Phase 48 UPST6 sync (running in parallel with C5, C7, C8, C9).

**Cluster:** C6 (macOS grant restore + localhost outbound)
**Disposition:** `will-sync` (per fork-side Phase 47 UPST6 audit ledger row for C6)
**Upstream SHA range:** `abca959a..74b0be71` (3 commits, authored 2026-05-14..16 by SequeI / Leonardo Zanivan)
**Upstream tags:** `v0.55.0` (all 3 commits)
**Fork baseline:** `3f638dc6`
**Fork branch:** `worktree-agent-a2e067712af893078` (Wave 2 parallel worktree)
**Plan:** `48-05`
**Requirement contribution:** REQ-UPST6-02 (C6 cluster of 3 commits discharged)

### Cherry-pick manifest (upstream → fork; upstream-chronological order)

| # | Upstream SHA | Fork SHA | Subject |
|---|-------------|----------|---------|
| 1 | `abca959a` | `55fd1d56` | feat(macos): treat open_port 0 as localhost:* outbound |
| 2 | `2c3742ab` | `1945ecfd` | fix(cli): preserve macOS future-file grants in why --self |
| 3 | `74b0be71` | `72791f5c` | fix(cli): unify macOS exact-path grant restore |

### Key changes

- **C6-03 (abca959a):** Introduces `open_port: [0]` as a macOS-only wildcard meaning `localhost:*`
  TCP outbound. Adds `push_localhost_tcp_outbound_seatbelt_rules` to `sandbox/macos.rs` which
  generates `(allow network-outbound (remote tcp "localhost:*"))` for port 0. Linux explicitly
  rejects port 0 with a `NonoError::SandboxInit` error ("open_port 0 is macOS-only"). Three new
  macos.rs tests cover: blocked profile with port-zero wildcard, mixed zero and fixed ports, and
  proxy profile with port-zero wildcard. Updates `capability.rs` doc comment, profile schema, and
  profile-authoring-guide for the new semantics.

- **C6-01 (2c3742ab):** Makes `new_future_file_capability` `pub(crate)` and adds 3 new tests to
  `sandbox_state.rs` exercising `parse_capability_source` for `CapabilitySource::Group` and
  `CapabilitySource::ExactPath` variants. Ensures `why --self` output correctly lists macOS
  future-file grants.

- **C6-02 (74b0be71):** Consolidates `restore_existing_file_capability` +
  `restore_missing_file_capability` into a single `restore_exact_path_capability` function in
  `capability_ext.rs`. Adds `#[cfg(target_os = "macos")]` import for `new_exact_path_capability`
  to `sandbox_state.rs`. Reduces duplication and unifies the grant-restore code path.

### Conflict resolution

| Commit | Conflict | Resolution |
|--------|----------|------------|
| `abca959a` | `sandbox/linux.rs`: fork's C5 test name vs upstream's test name (same location, ABI guard differs) | Accept upstream's test body with `AccessNet::from_all(detected.abi).is_empty()` ABI guard |
| `abca959a` | `profile/mod.rs`: fork's 4-line `open_port` doc vs upstream's 1-line | Accept upstream's 1-liner (carries alias annotation) |
| `2c3742ab` | `sandbox_state.rs` imports: fork's Windows test_env import + upstream's CapabilitySource import | Keep both imports |
| `2c3742ab` | `sandbox_state.rs` tests: fork's Windows test (cut off) + 3 upstream new tests | Reconstruct fork's Windows test; include all 3 upstream tests |
| `2c3742ab` | `parse_capability_source`: upstream used Edition 2024 `if let` guard syntax | Convert to nested `if let` for Edition 2021 compatibility |
| `74b0be71` | `capability_ext.rs` × 3: `handle_exact_directory_path` → `new_exact_path_capability` refactor | Adapt upstream's refactored body; remove `allow_parent_of_protected` param (fork doesn't have it); hardcode `false` |

### Fork-invariant preservation

1. **PATTERNS.md row #1 — sandbox/linux.rs strictly allow-list:** C6-03 does NOT introduce
   a deny-style code path. The `open_port 0` check in `apply_with_abi` returns `Err` immediately
   as a pre-condition guard (input validation). No Landlock deny rule added.

2. **PATTERNS.md row #4 — capability.rs path canonicalization:** The `add_localhost_port` doc
   update is documentation-only. Grant-resolution logic and path canonicalization unchanged.

3. **PATTERNS.md row #6 — profile/mod.rs exhaustive `From<ProfileDeserialize>` match:**
   C6-03 touched `profile/mod.rs` only to update the `open_port` field doc comment. No new
   struct fields added by upstream; exhaustive match is preserved unchanged.

4. **Windows-only files invariant (D-48-E1):** Zero files touched under
   `crates/nono-cli/src/exec_strategy_windows/`, `crates/nono-shell-broker/`, or `*_windows.rs`.

5. **D-19 trailer + DCO:** All 3 cherry-picks carry 7-line D-19 trailer block + `Co-Authored-By`
   + `Signed-off-by: Oscar Mack Jr` per Phase 48 conventions.

### Security posture (STRIDE coverage)

- **T-48-05-01 (Elevation of Privilege — exact-path grant unification):** Mitigated.
  `new_exact_path_capability` canonicalizes paths at grant time. Cross-target macOS clippy gate
  (Gate 4) confirmed zero new errors from C6. Upstream-tested.

- **T-48-05-02 (Spoofing — `open_port 0` misinterpreted on Linux):** Mitigated.
  Linux unconditionally rejects `port 0` with `NonoError::SandboxInit("open_port 0 is macOS-only")`.
  cfg-gating verified in `sandbox/linux.rs` — no macOS Seatbelt code runs on Linux.

### CI status

Local macOS dev host: build clean; 1836 tests pass across workspace. 1 pre-existing failure
(`audit_verify_reports_signed_attestation_with_pinned_public_key`) is a sandbox denial for
`/Users/oscarmack/nono` (pre-dates C6; confirmed by stash-verify at HEAD~3). Cross-target Linux
clippy deferred to CI (PARTIAL _environmental — cross-toolchain not installed). macOS clippy has
8 pre-existing Class-B errors (zero new from C6). Windows-lane gates (`wfp_port_integration`,
`learn_windows_integration`) marked `_environmental` per Claude's Discretion in CONTEXT.md — C6
has zero Windows surface.

### Source artifacts

- [`48-05-CLOSE-GATE.md`](.planning/phases/48-upst6-sync-execution/48-05-CLOSE-GATE.md) — 9-gate matrix
- [`48-05-SUMMARY.md`](.planning/phases/48-upst6-sync-execution/48-05-SUMMARY.md) — plan close summary
