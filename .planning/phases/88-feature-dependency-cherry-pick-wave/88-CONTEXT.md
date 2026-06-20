# Phase 88: Feature + Dependency Cherry-Pick Wave - Context

**Gathered:** 2026-06-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb the additive, low-conflict cherry-picks from the upstream `v0.62.0..v0.64.0`
window across the 5-crate workspace, per the Phase 85 DIVERGENCE-LEDGER dispositions:

- **Cluster D** (FEAT-01) — `set_vars` static env injection (`d48aeb7b`)
- **Cluster E** (FEAT-02) — XDG state dirs + config paths (`e8293b36`, `8e0d94f9`)
- **Cluster G** (FEAT-03) — AWS auth config (`5bb098cd`, tls_intercept hunk won't-apply)
- **Cluster H** (FEAT-04) — `NONO_KEYRING_TIMEOUT_SECS` (`c6b13345`)
- **Cluster I** (FEAT-05) — `$PACK_DIR` session-hook support (`7d274cf7`)
- **Cluster K + M** (FEAT-06) — update-check CI-provider discovery (`cc11b389`), truthy
  env bool flags (`42e5bf73`), profile namespace standardization (`6d88638e`, Cluster L)
- **Cluster J** (DEPS-01) — PTY ctrl-z hang fix (`4179ce03`)
- **Cluster M (split)** — misc additive fixes (`a0bba5eb`, `ee7a3bda`, `7e076d2d`,
  `9800f307`, `e54cf9cb` env_clear Unix-only)
- **DEPS-02** — 9 dependency bumps absorbed across all 5 crates
- **CR-01** — FFI stale `LAST_DIAGNOSTIC_CODE` fix deferred from Phase 87

**In scope:** FEAT-01..06, DEPS-01, DEPS-02, and CR-01 (deferred-from-87 fork hardening).
**Out of scope:** new capabilities; the proxy hardening cluster F (Phase 89); the v3.0
host-gated UAT drain (Phase 90); the audit/diagnostics boundary work (already done, Phase 86).

**Disposition source:** Every cluster's `will-sync`/`split` disposition and per-commit SHA
table is locked in the Phase 85 ledger — this phase executes those decisions, it does not
re-litigate them.

</domain>

<decisions>
## Implementation Decisions

### XDG state-dir reconciliation (FEAT-02, Cluster E)
- **D-01:** **Adopt upstream's `state_paths.rs` as the single source of truth.** Cherry-pick
  the module; rewrite the fork's existing `crates/nono-cli/src/config/mod.rs` helpers
  (`user_state_dir()`, `legacy_windows_state_dir()`) to **delegate** to it. Converges toward
  upstream's layout (the milestone goal) rather than keeping a parallel fork implementation.
- **D-02:** **Windows arm → `%LOCALAPPDATA%\nono`.** `dirs::state_dir()` returns `None` on
  Windows (XDG state is Linux-only), so the Windows path must be mapped explicitly to
  `%LOCALAPPDATA%\nono`, reconciled against the **v3.0 scratch-space provisioner** (verify the
  provisioner's runtime-path location matches so state/scratch don't split). This is the only
  Windows-touch decision in the wave.
- **D-03:** **One-time auto-migrate (move), fail-secure.** On first run, if legacy `~/.nono`
  exists and the new location does not, move state once (matches upstream #1152/#1179 intent).
  On **any** migration error, abort rather than silently splitting state across two trees.

### Dependency bumps (DEPS-02)
- **D-04:** **Resolve latest-compatible via `cargo update`.** Most workspace specs are already
  loose (`hyper="1"`, `zeroize="1"`, `ignore="0.4"`, `cbindgen="0.29"`, `chrono="0.4"`,
  `which="8"`), so those bumps are pure `Cargo.lock` updates. **`x509-parser` (0.18.1) and
  `time` (0.3.49) are transitive** (not direct deps in any Cargo.toml) → lockfile-only via
  `cargo update -p`. **`typify` 0.6→0.7 is the ONLY direct Cargo.toml spec edit** (`crates/nono/Cargo.toml:71`).
- **D-05:** **One atomic DEPS commit for all 9 bumps** — the typify spec edit + the Cargo.lock
  updates land together (cleanest history for a pure dep wave). Watch for typify-0.7 codegen
  fallout (it drives JSON-schema type generation); if it forces non-trivial source changes,
  the planner may split that out.
- **D-06:** **Explicit 5-crate path-dep pin checklist gate** in the plan before `make ci`
  (per `project_workspace_crates` — this drift class has slipped before). Verify the internal
  `nono`/`nono-proxy` path-dep `version` pins are synced across all 5 `Cargo.toml` files.

### Profile namespace standardization (Cluster L, FEAT-06)
- **D-07:** **Adopt upstream's namespace convention + keep bare-name aliases.** Rename built-in
  profiles to the namespace form but register the old bare names (`claude-code`, `default`,
  `codex`, …) as aliases so existing `--profile claude-code` invocations, docs, and the fork's
  Windows interpreter mappings keep working. Converges naming without breaking users.
- **D-08:** **Namespace the fork-only profiles consistently too** (`nono-ts-wfp-test-open`,
  `nono-ts-wfp-test-blocked`, `nono-ts-default`, `swival`, etc.) so the whole set is uniform —
  no mixed convention.
- **D-09:** FEAT-06 also absorbs **update-check CI-provider discovery** (`cc11b389`) and
  **truthy env values for bool CLI flags** (`42e5bf73`) — independent of the rename; absorb
  even if the rename's alias work is larger.

### CR-01 FFI stale-diagnostic fix (deferred from Phase 87)
- **D-10:** **Clear-on-entry across ALL FFI entry points.** Reset `LAST_DIAGNOSTIC_CODE`
  (and `last_remediation_json`) at the start of every public `extern "C"` fn that can set it,
  so each call's diagnostic reflects only that call. Systematic fix for the whole FFI surface,
  not just the one observed path. Upstream-identical code is at `a6aa5995`.
- **D-11:** **Dedicated FFI regression test + standalone fork-divergence commit.** Add an FFI
  test asserting the code is cleared between calls; land CR-01 as **its own commit** (not a
  cherry-pick — it is a fork fix on inherited code) and record it as a **deliberate
  fork-divergence** in the divergence ledger (mirroring the Phase 87 CR-02 addendum pattern,
  so future syncs expect the conflict on these lines).

### Sequencing & absorption mechanics (carried forward / ledger-locked)
- **D-12:** **`git cherry-pick -x` + DCO `Signed-off-by` per upstream SHA** (Phase 86/87
  pattern). One atomic commit per cherry-pick where clean.
- **D-13:** **Cherry-pick Cluster I (`7d274cf7`) BEFORE Cluster M (`e54cf9cb`)** — both touch
  `crates/nono-cli/src/hook_runtime.rs`; I-before-M avoids a rebase conflict (ledger Check 6).
- **D-14:** **`e54cf9cb` env_clear removal is Unix-path ONLY.** Apply to `hook_runtime.rs`;
  `hook_runtime_windows.rs` **RETAINS** `env_clear()` + the `SystemRoot`/`windir`/`SystemDrive`
  baseline restore (per `windows_hook_interpreter_spawn_gotchas` — removing it causes CLR init
  failure `0xFFFF0000`/`-65536`). Do NOT apply `e54cf9cb` to the Windows hook path.
- **D-15:** **`5bb098cd` (AWS auth) and `b0b2c743`-style tls_intercept hunks won't-apply** —
  the fork has no `crates/nono-proxy/src/tls_intercept/` directory (Phase 34 C11 fork-preserve).
  Extract only the shared-surface hunks (`route.rs`, `credential.rs`, `server.rs`,
  `config.rs`, `network_policy.rs`, `profile/mod.rs`); skip the tls_intercept hunks. Verify the
  AWS-auth behavior on the fork's non-TLS-intercept proxy path.

### Claude's Discretion
- **set_vars (FEAT-01) env-name validation internals** — the requirement locks "reject `PATH`
  and the `NONO_` prefix"; exact validation/error-surface wiring is planner/researcher territory.
- **AWS auth config (FEAT-03) mutual-exclusion enforcement** — `AwsAuthConfig` must be mutually
  exclusive with `credential_key`/`oauth2`; where/how that validation lands (profile load vs
  proxy route config) is for the planner.
- **`$PACK_DIR` / `source_pack` propagation details** (FEAT-05) and pack-verification dry-run
  skip (`9800f307`) internals.
- typify-0.7 codegen split decision (D-05) if fallout is non-trivial.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 88 scope & dispositions (PRIMARY — read first)
- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` — per-cluster
  dispositions + per-commit SHA tables. Specifically: Cluster D (lines ~258-276), E (~280-302),
  G (~378-400), H (~404-422), I (~426-445), J (~449-466), K (~470-487), L (~491-508),
  M split + e54cf9cb annotation (~512-546), Release/dep-bumps (~550-572), and the "Excluded as
  Noise" dep-bump note (~794-799). Empirical Cross-Check (~617-669) confirms I/M share
  `hook_runtime.rs`.
- `.planning/REQUIREMENTS.md` — FEAT-01..06 (lines 43-48), DEPS-01/DEPS-02 (lines 57-58).
- `.planning/ROADMAP.md` §"Phase 88: Feature + Dependency Cherry-Pick Wave" — goal + 5 success
  criteria.

### CR-01 (deferred from Phase 87)
- `.planning/phases/87-security-sync/87-CONTEXT.md` D-13 + `<deferred>` — CR-01 deferral
  rationale; upstream-identical code at `a6aa5995`.
- `.planning/phases/86-library-boundary-convergence/86-REVIEW.md` §"Critical Issues" CR-01
  (~line 230) — full finding text.
- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` §"Phase 87 CR-02
  Addendum" (~lines 807-828) — the ledger-addendum + regression-test pattern CR-01 should mirror.

### Code touchpoints (fork-specific)
- `crates/nono-cli/src/config/mod.rs` — existing `user_state_dir()` (line 174),
  `legacy_windows_state_dir()` (204), `user_config_dir()` (157), `nono_home_dir()` (130).
  These delegate to the adopted `state_paths.rs` (D-01).
- `crates/nono-cli/src/audit_session.rs` — `ensure_audit_session_dir()` (45),
  `ensure_rollback_session_dir()` (72) — state roots that move under XDG/`%LOCALAPPDATA%`.
- `crates/nono-cli/src/hook_runtime.rs` (Unix) + `hook_runtime_windows.rs` (Windows carve-out, D-14).
- `crates/nono-cli/data/policy.json` §`profiles` (line 605+) + `crates/nono-cli/src/profile/builtin.rs`
  — bare profile names + fork-only profiles to namespace/alias (D-07/D-08).
- `bindings/c/src/diagnostic.rs` + `bindings/c/src/lib.rs` — FFI thread-local diagnostic store
  for CR-01 clear-on-entry (D-10).
- All 5 `Cargo.toml` (workspace root + `crates/nono`, `crates/nono-cli`, `crates/nono-proxy`,
  `bindings/c`) — dep specs + internal path-dep version pins (D-06). `crates/nono/Cargo.toml:71`
  = the typify spec edit.

### Process rules
- `CLAUDE.md` §"Coding Standards" — cross-target clippy MUST/NEVER (FEAT-02/Cluster G touch
  no `#[cfg]` Unix blocks beyond what cherry-picks bring; verify per-commit).
- `.planning/templates/cross-target-verify-checklist.md` — PARTIAL→CI deferral for anything
  unverifiable on the Windows dev-host (Unix-path features, Linux/macOS clippy).
- Memory `project_workspace_crates` — 5-Cargo.toml + path-dep-pin sync rule (D-06 gate).
- Memory `windows_hook_interpreter_spawn_gotchas` — env_clear/CLR-baseline rationale (D-14).

### Upstream commits to absorb (per-cluster)
- D: `d48aeb7b` · E: `e8293b36`, `8e0d94f9` · G: `5bb098cd` · H: `c6b13345` · I: `7d274cf7`
  · J: `4179ce03` · K: `cc11b389` · L: `6d88638e` · M (split): `42e5bf73`, `a0bba5eb`,
  `ee7a3bda`, `7e076d2d`, `9800f307`, `e54cf9cb` (Unix-only).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`config/mod.rs` state-path helpers already exist** (`user_state_dir`, `legacy_windows_state_dir`,
  `nono_home_dir`, `user_config_dir`) — D-01 delegates these to the adopted upstream module
  rather than building from scratch. `NONO_TEST_HOME` override + fail-through-to-`dirs` pattern
  is already established and must be preserved through the migration.
- **FFI thread-local diagnostic store** (`bindings/c/src/lib.rs` error store + `diagnostic.rs`
  from Phase 86) — CR-01's clear-on-entry hooks into the existing thread-local set/get pattern.
- **policy.json + builtin.rs profile registry** — the namespace + alias work (D-07/D-08) extends
  the existing two-surface profile definition (data + code references).

### Established Patterns
- **Cherry-pick `-x` + DCO, one atomic commit per upstream SHA** (Phase 86 = 8 picks, Phase 87 = 2).
- **Deliberate fork-divergence recorded as a ledger addendum + regression test** (Phase 87 CR-02
  is the template for CR-01's D-11).
- **PARTIAL→CI deferral** for Unix-path features unverifiable on the Windows dev-host.
- **Windows hook carve-out** — Unix loses `env_clear()`, Windows keeps it + baseline restore (D-14).

### Integration Points
- XDG state module → audit/session/rollback root callsites (`audit_session.rs`, `undo/snapshot.rs`)
  + the v3.0 Windows scratch-space provisioner (must verify same location, D-02).
- AWS auth config → fork's non-TLS-intercept proxy credential path (`credential.rs`, `route.rs`,
  `server.rs`); tls_intercept hunks skipped (D-15).
- Profile aliases → CLI `--profile` resolution + Windows interpreter mappings.

</code_context>

<specifics>
## Specific Ideas

- The fork ALREADY partially implements XDG state (`config/mod.rs` `user_state_dir()` via
  `dirs::state_dir()`); the FEAT-02 work is convergence/reconciliation, not greenfield — the
  implementer must avoid leaving two parallel state-path implementations (D-01).
- CR-01 is the SECOND deliberate fork-divergence on the Phase-86-converged surface (after CR-02);
  the ledger addendum must make the future-sync conflict expectation explicit (D-11).
- typify 0.7 is the one dep bump with real code-fallout risk (JSON-schema codegen) — everything
  else is mechanical (D-05).

</specifics>

<deferred>
## Deferred Ideas

- **Cluster F proxy hardening** (route/403/TLS-CONNECT/reactive-auth/customCredentials) → Phase 89
  (PROXY-01/02). Note: FEAT-03 AWS auth (this phase) and `customCredentials` (Phase 89) share the
  credential path — Phase 89 depends on this phase's AWS-auth landing.
- **`TlsInterceptIntent` assessment** (`bd4b6b7f`) → Phase 89 (can it be defined without the
  backing tls_intercept module?).
- **policy.json `go_runtime` go-build cache group** (`5413a0b3`) — noise per the drift-tool path
  filter; deferred to a future policy.json sync pass, not UPST9.

None of the above are scope creep raised in discussion — they are ledger-dispositioned
out-of-phase items recorded so they are not lost.

</deferred>

---

*Phase: 88-feature-dependency-cherry-pick-wave*
*Context gathered: 2026-06-20*
