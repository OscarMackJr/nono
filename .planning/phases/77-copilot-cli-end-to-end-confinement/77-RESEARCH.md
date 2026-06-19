# Phase 77: Copilot CLI End-to-End Confinement - Research

**Researched:** 2026-06-17
**Domain:** Windows AppContainer confinement (Node-ESM module resolution), DACL/SID grant machinery, Rust CLI surface, dark-factory PowerShell gate
**Confidence:** HIGH for code-anchor facts (all read on-disk), MEDIUM for Copilot shim runtime behavior (verified via official docs + npm docs, not on a live host), HIGH for the D-05 SID-stability falsification (read from source).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Confinement target is the **standalone `@github/copilot` (Node) CLI** — invoked as `copilot`, NOT `gh copilot`, NOT a native-PE `copilot.exe`. This is the Node-ESM `realpathSync`/`lstat` ancestor-walk case.
- **D-02:** `copilot-cli` profile must add **`node.exe` interpreter coverage** (`windows_interpreters: ["node.exe"]`). The current profile's D-06 finding ("native PE, no Node interpreter needed") is **stale/superseded** for this target. Researcher confirms on-host how the installed `copilot` resolves to Node, then wires coverage.
- **D-03 (derived):** SC1's `gh copilot suggest "list files"` example string is **superseded** — gate + SC1 use standalone `copilot`. Planner reconciles ROADMAP/REQUIREMENTS wording.
- **D-04 (derived):** Runtime code (CPLT-01) grants RA on every ancestor nono **can ACL as the current user** (user-ownable). System ancestors `C:\` and `C:\Users` are covered exclusively by the one-time-admin step (CPLT-02). Split is determined by OS ownership, not configuration.
- **D-05:** Rely on the carried-forward fact that the **package SID is derived from the profile name and stable across runs** — making a one-time admin grant durable. **Researcher must re-confirm SID stability before locking the setup step.** ⚠️ **SEE OPEN QUESTION OQ-1 — THIS PREMISE IS FALSIFIED BY THE LIVE CODE.**
- **D-06:** Expose the one-time-admin grant as a **generic, reusable command** — `nono setup --grant-ancestors --profile <p>` (or equivalent generic shape). NOT Copilot-specific. Exact flag spelling is planner discretion as long as it is generic, idempotent, one-time-admin, non-destructive.
- **D-07:** `copilot-e2e` gate treats Copilot install + GitHub auth + network as a **precondition**. If absent → `SKIP_HOST_UNAVAILABLE`, NOT `FAIL`.
- **D-08:** `PASS` requires a real, authenticated `copilot` suggestion that prints output with **zero `STATUS_ACCESS_DENIED` and zero Node module-resolution crash** under AppContainer. Gate must distinguish a *confinement* failure (FAIL) from an unrelated Copilot/auth/network failure (SKIP).
- **D-09:** Admin grant is **permanent and documented, no undo command**. `FILE_READ_ATTRIBUTES` is attribute-read only, scoped to one package SID, on `C:\`/`C:\Users` only. Non-destructive (adds one allow-ACE per ancestor; alters/removes nothing). No `--revoke` this phase.

### Claude's Discretion
- Exact CLI flag/subcommand spelling for the generic grant command (D-06), provided it stays generic + idempotent + non-destructive.
- The minimal scriptable "real task" command/args for the gate (D-08), provided it exercises the Node-ESM ancestor walk and is unattended.
- The precise precondition probe the gate uses to detect Copilot install + auth (D-07).
- Whether `node.exe` interpreter coverage is enough or the standalone `copilot` shim needs additional path coverage (D-02) — settle empirically during research.

### Deferred Ideas (OUT OF SCOPE)
- `--revoke` / uninstall counterpart for the ancestor grant (D-09: permanent, no undo this phase).
- Copilot-specific setup flag (rejected in favor of generic `--grant-ancestors --profile <p>`, D-06).
- Reviewed-but-not-folded todos: MSI VC++ prereq (Phase 80), POC-cert broker (enterprise), macOS rlimit defect (other platform).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CPLT-01 | nono grants ancestor-chain `FILE_READ_ATTRIBUTES` (RA) up to the drive root for a confined target's package SID, so Node-ESM module resolution succeeds under AppContainer. | An almost-complete analog already exists: `AppliedAncestorTraverseGuard` (`dacl_guard.rs:222`) grants the package SID `FILE_TRAVERSE \| FILE_LIST_DIRECTORY` (0x21) on user-owned cwd ancestors. CPLT-01 needs a parallel guard that walks the **target binary's resolution path** (and/or all package-SID grant paths) granting RA. The generic mask-parameterized core `edit_dacl_for_sid` (`windows.rs:1629`) makes adding a `grant_sid_read_attributes_on_path` trivial. See Architecture Patterns. |
| CPLT-02 | Idempotent one-time-admin setup step grants package-SID RA on system ancestors (`C:\`, `C:\Users`) nono cannot ACL at runtime — documented, verified non-destructive. | Extend `SetupArgs` (`cli.rs:2570`). ⚠️ **BLOCKED on OQ-1: the package SID is per-run (UUID), not profile-stable — a one-time admin grant on a per-run SID cannot be durable.** The setup command's grantee SID must be re-derived from a *stable* identifier. See Open Questions + Architecture Patterns for the two candidate resolutions. |
| CPLT-03 | Copilot CLI completes a real task end-to-end under confinement, proven by an unattended scripted gate. | New `scripts/gates/copilot-e2e.ps1` implementing the two-function contract (`Test-Precondition` / `Invoke-Gate`) consumed by `scripts/verify-dark.ps1`. Full contract documented in Architecture Patterns + Validation Architecture. |
</phase_requirements>

## Summary

Phase 77 turns the v2.12 Phase 75 "confine-only" Copilot re-scope into a real end-to-end confinement. The mechanical building blocks are almost all present and battle-tested: the DACL-grant core (`edit_dacl_for_sid`) is already mask-parameterized, an ancestor-walk-with-ownership-gate RAII guard (`AppliedAncestorTraverseGuard`) already exists for the cwd-traverse case, the `windows_interpreters` profile field is an established pattern (`aider`/`langchain-python` use `["python.exe"]`), and the Phase 76 dark-factory gate contract is fully specified and reference-implemented in `harness-self-check.ps1`. CPLT-01 is therefore a near-clone of an existing guard with a different access mask and a different walk target; CPLT-03 is a copy-the-shape gate.

**The one load-bearing surprise is OQ-1, which falsifies decision D-05.** D-05 (and CPLT-02's whole premise) assumes the AppContainer package SID is "derived from the profile name and stable across runs." The live code proves otherwise: `generate_app_container_name()` (`restricted_token.rs:51`) returns `format!("nono.session.{}", Uuid::new_v4().simple())` — a **fresh random UUID per run**, and `derive_app_container_sid` deterministically derives the package SID from that per-run name. The SID is therefore *unique per run*, not stable across runs. A one-time-admin RA grant keyed to one run's package SID would be useless on the next run. This must be resolved before CPLT-02 can be planned — the phase needs either (a) a stable, profile-derived AppContainer moniker for confined engines, or (b) a different stable principal for the system-ancestor grant (e.g. the `ALL APPLICATION PACKAGES` / `ALL RESTRICTED APPLICATION PACKAGES` well-known SIDs that every AppContainer carries). This is a planning decision that should go back to the user via discuss-phase.

**Primary recommendation:** Plan CPLT-01 as a new `AppliedAncestorReadAttributesGuard` mirroring `AppliedAncestorTraverseGuard` (granting `FILE_READ_ATTRIBUTES` on user-owned ancestors of the target/grant paths, stopping at the first non-owned ancestor), backed by a new `grant_sid_read_attributes_on_path` in `windows.rs`. Plan CPLT-03 as `scripts/gates/copilot-e2e.ps1` copying the `harness-self-check.ps1` shape. **Escalate OQ-1 (per-run SID) before locking CPLT-02's grantee** — recommend the well-known `ALL APPLICATION PACKAGES` SID (`S-1-15-2-1`) as the stable admin-grant principal, which is genuinely durable and engine-agnostic.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Launch-time ancestor RA grant (CPLT-01) | `nono-cli` exec strategy (`exec_strategy_windows/`) | `nono` lib (DACL primitive) | Policy + lifetime (RAII revert) is CLI; the raw `edit_dacl_for_sid` Win32 call is the library primitive. Mirrors the existing labels/dacl/traverse guard split. |
| RA-only DACL primitive (`grant_sid_read_attributes_on_path`) | `nono` lib (`sandbox/windows.rs`) | — | Pure Win32 SID/DACL editing belongs in the library; CLI guards call it. |
| One-time-admin system-ancestor grant (CPLT-02) | `nono-cli` setup runtime | `nono` lib (DACL primitive + SID derivation) | Setup UX/idempotency is CLI; the grant + SID derivation are library calls. |
| `copilot-cli` profile shape (`node.exe` coverage) | `nono-cli` policy data (`policy.json`) | `nono-cli` profile loader | Profiles are CLI-owned policy; library is policy-free. |
| Unattended e2e gate (CPLT-03) | `scripts/gates/` (PowerShell) | `scripts/verify-dark.ps1` runner | Dark-factory harness is operator-tier scripting, outside the Rust workspace. |
| Package-SID derivation / AppContainer moniker | `nono` lib (`derive_app_container_sid`) + `nono-cli` (`generate_app_container_name`) | — | Library does the FFI derivation; CLI owns the moniker string (the per-run-vs-stable decision lives here — OQ-1). |

## Standard Stack

This is an internal-only phase — no new external crates. All work uses crates already in the workspace.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `windows-sys` | 0.59 (workspace pin) | `SetEntriesInAclW`, `GetNamedSecurityInfoW`, `SetNamedSecurityInfoW`, `DeriveAppContainerSidFromAppContainerName`, `ConvertStringSidToSidW` | Already the sole low-level Win32 surface for the DACL/SID machinery being extended. `[CITED: crates/nono/src/sandbox/windows.rs]` |
| `uuid` | (workspace pin) | `Uuid::new_v4().simple()` for the per-run AppContainer moniker | Already used in `generate_app_container_name`. `[CITED: restricted_token.rs:52]` |
| `clap` | v4 | `SetupArgs` flag derive | Existing setup-command surface. `[CITED: cli.rs:2570]` |
| PowerShell | (host) | `copilot-e2e.ps1` gate | Dark-factory harness language (Phase 76). `[CITED: scripts/verify-dark.ps1]` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tracing` | workspace | warn/debug logging in the new guard | Mirror existing guards' `tracing::warn!`/`debug!` on skip/revert. |
| `tempfile` | workspace (dev) | unit-test scratch dirs | The new guard's tests mirror `dacl_guard.rs` tests using `tempdir()`. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New `AppliedAncestorReadAttributesGuard` | Widen the existing `AppliedAncestorTraverseGuard` mask to also include `FILE_READ_ATTRIBUTES` | Tempting (one guard, one walk) but the traverse guard walks **cwd** ancestors and stops at the first non-owned; the RA need is on the **target-binary resolution chain to the drive root** and (per the 75-08 finding) needs RA on `C:\` itself. Different walk target + different stop condition + the system-root ancestors need the admin grant, not the runtime guard. Keep them separate; document the relationship. |
| New RA SID for the admin grant | Reuse the per-run package SID | Falsified by OQ-1 — per-run SID is not durable. |

**No `npm install` / package additions** — therefore the **Package Legitimacy Audit section is omitted** (no external packages installed by this phase).

## Architecture Patterns

### System Architecture Diagram

```
                          nono run --profile copilot-cli -- copilot suggest "..."
                                          │
                                          ▼
                        ┌─────────────────────────────────────┐
                        │ exec_strategy_windows::               │
                        │   prepare_live_windows_launch()       │   (mod.rs:336)
                        └─────────────────────────────────────┘
                                          │
        ┌──────────────────┬──────────────┼───────────────────┬──────────────────────┐
        ▼                  ▼              ▼                    ▼                       ▼
 validate_windows_   AppliedLabels   AppliedDacls      AppliedAncestor-      ★ NEW: AppliedAncestor-
 launch_paths        Guard (NO_      Guard (pkg-SID    TraverseGuard          ReadAttributesGuard
 (interpreter        WRITE_UP        write on          (pkg-SID TRAVERSE      (pkg-SID FILE_READ_
  coverage gate;     labels)         writable grants)  on user-owned cwd      ATTRIBUTES on user-owned
  node.exe must                                        ancestors)             ancestors of target +
  be covered)                                                                 grant paths → CPLT-01)
        │                                                                              │
        ▼                                                                              ▼
   launch broker (windows_low_il_broker) → AppContainer child (package SID S-1-15-2-<per-run uuid>)
        │                                                                              │
        ▼                                                                              ▼
   node.exe runs copilot ESM entry → realpathSync/lstat walks every ancestor ──► RA grant satisfies
   of the module path up to C:\  ──────────────────────────────────────────────►  the lstat at each
                                                                                    user-owned ancestor;
                                                                                    C:\ + C:\Users satisfied
                                                                                    by the one-time-admin
                                                                                    grant (CPLT-02)

  ── separately, run once with admin ──
  nono setup --grant-ancestors --profile copilot-cli   (CPLT-02; cli.rs SetupArgs)
        │
        ▼
  derive STABLE grantee SID  ──►  grant FILE_READ_ATTRIBUTES on C:\ and C:\Users  (permanent, D-09)
  (⚠ OQ-1: which SID? per-run package SID is NOT durable)

  ── verification ──
  scripts/verify-dark.ps1 --gate copilot-e2e   (CPLT-03)
        │
        ▼
  Test-Precondition (copilot installed? authed? network?) ──► null = run / "reason" = SKIP_HOST_UNAVAILABLE
        │
        ▼
  Invoke-Gate: run `nono run --profile copilot-cli -- copilot ...` ; assert no STATUS_ACCESS_DENIED,
               no Node module-resolution crash ──► PASS / FAIL
```

### Recommended Project Structure (files touched)
```
crates/nono/src/sandbox/windows.rs        # + grant_sid_read_attributes_on_path() + PACKAGE_SID_READ_ATTRS_MASK
crates/nono/src/lib.rs                     # re-export the new grant fn (line ~86 cluster)
crates/nono-cli/src/exec_strategy_windows/
    dacl_guard.rs                          # + AppliedAncestorReadAttributesGuard (mirror traverse guard)
    mod.rs                                  # wire the new guard into prepare_live_windows_launch + PreparedWindowsLaunch
crates/nono-cli/data/policy.json           # copilot-cli: add "windows_interpreters": ["node.exe"]  (line ~917)
crates/nono-cli/src/profile/builtin.rs     # update copilot_cli_profile_present / _is_native_pe tests (lines 249-299)
crates/nono-cli/src/cli.rs                 # SetupArgs: + --grant-ancestors + --profile <p> (line ~2570)
crates/nono-cli/src/setup.rs               # setup runtime: handle the grant-ancestors path
scripts/gates/copilot-e2e.ps1              # NEW gate (CPLT-03)
```

### Pattern 1: Mask-parameterized DACL grant (the reusable primitive)
**What:** Every per-SID DACL grant routes through one shared core that takes the access mask as a parameter.
**When to use:** Adding `grant_sid_read_attributes_on_path` — just call the core with a new mask constant.
```rust
// Source: crates/nono/src/sandbox/windows.rs:1629 (edit_dacl_for_sid)
fn edit_dacl_for_sid(
    path: &Path, sid: &str, access_mask: u32,
    access_mode: ACCESS_MODE, inheritance: u32,
) -> Result<()> { /* GetNamedSecurityInfoW → SetEntriesInAclW (MERGE) → SetNamedSecurityInfoW */ }

// Existing public wrappers (windows.rs:1770 / 1816 / 1857):
pub fn grant_sid_write_on_path(path, sid, inheritable) -> Result<()>     // SESSION_SID_WRITE_MASK 0x1301BF
pub fn grant_sid_traverse_on_path(path, sid) -> Result<()>               // PACKAGE_SID_TRAVERSE_MASK 0x21
pub fn grant_sid_read_on_path(path, sid) -> Result<()>                   // PACKAGE_SID_READ_MASK (FILE_GENERIC_READ) 0x120089

// ★ NEW for CPLT-01 — the MINIMAL grant per D-09:
const PACKAGE_SID_READ_ATTRS_MASK: u32 = {
    use windows_sys::Win32::Storage::FileSystem::FILE_READ_ATTRIBUTES; // 0x80
    FILE_READ_ATTRIBUTES   // plus SYNCHRONIZE/READ_CONTROL only if a live host proves lstat needs them
};
pub fn grant_sid_read_attributes_on_path(path: &Path, sid: &str) -> Result<()> {
    edit_dacl_for_sid(path, sid, PACKAGE_SID_READ_ATTRS_MASK, SET_ACCESS, NO_INHERITANCE)
}
```
**Note on minimality (D-09):** `FILE_READ_ATTRIBUTES` = `0x80`. The existing traverse mask pairs `FILE_LIST_DIRECTORY` for stat-during-path-resolution. The 75-08 finding says Node's `realpathSync`/`lstat` needs `FILE_READ_ATTRIBUTES` on each ancestor. Whether `SYNCHRONIZE`/`READ_CONTROL`/`FILE_LIST_DIRECTORY` are also required is the empirical question best answered on the live host during execution (the gate is the proof). Start minimal (`FILE_READ_ATTRIBUTES` only), widen only if the live gate shows `STATUS_ACCESS_DENIED` on the ancestor walk. `[VERIFIED: crates/nono/src/sandbox/windows.rs read in session]`

### Pattern 2: Ownership-gated ancestor-walk RAII guard (the CPLT-01 template)
**What:** Walk path ancestors from leaf upward, grant the package SID on each **user-owned** ancestor, **stop at the first non-owned ancestor**, revert all grants on Drop, fail-closed on ownership-check or grant errors on owned ancestors.
**When to use:** CPLT-01 — clone `AppliedAncestorTraverseGuard` (`dacl_guard.rs:222-307`), swap the grant call to `grant_sid_read_attributes_on_path`, and point the walk at the **target-binary resolution chain** (and/or the package-SID grant-set paths) rather than only the cwd.
```rust
// Source: crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:236
for ancestor in current_dir.ancestors().skip(1) {
    match path_is_owned_by_current_user(ancestor) {     // gate (windows.rs path-owner check)
        Ok(true)  => { grant_sid_traverse_on_path(ancestor, package_sid)?; applied.push(ancestor) }
        Ok(false) => break,   // C:\Users, C:\ — cannot edit DACL (no WRITE_DAC) → admin grant covers these
        Err(err)  => { revert_all(); return Err(err) }   // NEVER swallow ownership errors
    }
}
```
**Crucial alignment with D-04:** the runtime guard **stops at the first non-owned ancestor**. `C:\Users` and `C:\` are SYSTEM/TrustedInstaller-owned, so the runtime guard provably never touches them — which is exactly why CPLT-02 (admin) is required for those two. The split is enforced structurally by the ownership gate, matching D-04 verbatim.

### Pattern 3: Dark-factory gate two-function contract (the CPLT-03 template)
**What:** A gate file dot-sourced by `verify-dark.ps1` exports exactly two functions and **never calls `exit`** (the runner owns exit mapping).
```powershell
# Source: scripts/gates/harness-self-check.ps1 (reference contract for phases 77-80)
function Test-Precondition {
    # return $null  → preconditions met, run Invoke-Gate
    # return "reason string" → SKIP_HOST_UNAVAILABLE (exit 3), Invoke-Gate NEVER runs
}
function Invoke-Gate {
    # return an [ordered]@{ gate; verdict; reason; detail; timestamp } dict
    # verdict ∈ { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
    # a THROW here = harness-internal error (exit 4), never silently PASS (D-07)
}
```
Runner exit mapping (`verify-dark.ps1`): `PASS`=0, `FAIL`=2, `SKIP_HOST_UNAVAILABLE`=3, harness-internal error=4. Verdict is persisted to `.nono-runtime/verdicts/copilot-e2e.json` **before** the stdout line (WR-04). `[VERIFIED: scripts/verify-dark.ps1 + harness-self-check.ps1 read in session]`

### Anti-Patterns to Avoid
- **Reusing the per-run package SID for the admin grant.** It is UUID-derived per run (`restricted_token.rs:52`) → not durable. (OQ-1.)
- **Granting more than `FILE_READ_ATTRIBUTES` on `C:\` / `C:\Users`.** D-09 commits to attribute-read only; full read on the drive root would broaden exposure for every confined engine. Stay minimal.
- **Letting the gate emit `FAIL` when Copilot is just missing/unauthenticated.** That conflates the *confinement* assertion with Copilot's own availability (D-07/D-08). Detect those in `Test-Precondition` → `SKIP_HOST_UNAVAILABLE`.
- **Calling `exit` inside the gate.** Breaks the runner's verdict→exit contract (D-02). Return the dict.
- **Editing `dist/windows/nono-machine.wxs` or any generated artifact** — not in this phase's scope, but a standing repo gotcha.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Add an RA ACE to a path's DACL | A bespoke `SetEntriesInAclW` call | `edit_dacl_for_sid` core + a new mask constant | The merge-preserve-existing-DACL + fail-closed-at-every-step logic is already correct and tested. `[CITED: windows.rs:1629]` |
| Walk ancestors with ownership gating + RAII revert | A new loop | Clone `AppliedAncestorTraverseGuard` | Drop-safety, LIFO revert, fail-closed ownership handling, and the "stop at first non-owned" rule are all solved. `[CITED: dacl_guard.rs:222]` |
| Detect path ownership | String prefix / `starts_with` on paths | `nono::path_is_owned_by_current_user` | Path-component-correct; CLAUDE.md forbids string path comparison. `[CITED: dacl_guard.rs:113]` |
| Derive the package SID from a name | Manual SID arithmetic | `nono::derive_app_container_sid` + `package_sid_to_string` | Deterministic Win32 derivation already wrapped fail-closed. `[CITED: windows.rs:747/789]` |
| Gate verdict emit/persist/exit mapping | Per-gate JSON + exit logic | Return the dict from `Invoke-Gate`; let `verify-dark.ps1` own emit/persist/exit | The runner already enforces the verdict→exit contract, persists-before-emit, and normalizes stray-array returns. `[CITED: verify-dark.ps1]` |
| Interpreter coverage for a Node engine | A new launch-gate code path | Add `"windows_interpreters": ["node.exe"]` to the profile | The coverage gate (`validate_windows_launch_paths`, mod.rs:345) already consumes `config.interpreters`; `aider`/`langchain-python` prove the pattern with `["python.exe"]`. `[CITED: policy.json:900/935]` |

**Key insight:** Phase 77 is overwhelmingly *assembly of existing primitives*. The only genuinely new Rust is one ~10-line library function (RA grant) + one cloned guard. The only genuinely new "design" is resolving OQ-1 (stable grantee SID).

## Runtime State Inventory

> This phase adds a **permanent OS-registered DACL grant** (CPLT-02, D-09) — so the inventory matters even though it is not a rename/refactor.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no datastore keys, no collections. Verified: phase touches DACLs + a profile + a script only. | None. |
| Live service config | None — no n8n/Datadog/Tailscale config. The `copilot-cli` profile lives in `policy.json` (in git, embedded at build via `build.rs`). | Rebuild required after profile edit (embedded data). |
| OS-registered state | ⚠ **CPLT-02 adds a PERMANENT allow-ACE to the DACL of `C:\` and `C:\Users`** for the (stable) grantee SID. This is OS-registered state that **persists after uninstall** (D-09: no undo command). It is additive (one allow-ACE per ancestor; removes/alters no existing ACE, no deny-ACE). | Document in user-facing docs as a permanent, non-destructive grant. No migration. Re-running the setup command must be **idempotent** (detect the ACE already present → no-op, don't stack duplicate ACEs). |
| Secrets/env vars | None added. The gate reads GitHub auth state that Copilot manages itself (precondition probe only — nono never handles the token). | None. |
| Build artifacts | `policy.json` is embedded at build time via `build.rs`; `crates/nono-cli/data/` changes require a rebuild to take effect. The `copilot` shim itself is an npm global install under `%APPDATA%\npm` — external to nono, present only on hosts where the gate runs. | `make build` after the profile edit. Gate host needs `npm install -g @github/copilot`. |

**The CPLT-02 idempotency assertion is a first-class verification target** (see Validation Architecture): running the setup command twice must leave exactly one ACE, and must never modify or remove a pre-existing ACE.

## Common Pitfalls

### Pitfall 1: Per-run package SID defeats the one-time-admin grant (OQ-1)
**What goes wrong:** Plan CPLT-02 to grant the package SID RA on `C:\`/`C:\Users`, derive that SID from the per-run `nono.session.<uuid>` moniker → the grant is scoped to a SID that will never exist again → next `nono run` produces a different package SID with no RA on the system roots → Node-ESM walk denied → CPLT-03 FAILs intermittently or always.
**Why it happens:** D-05 asserts the SID is "derived from the profile name and stable." The code derives it from a per-run UUID (`restricted_token.rs:52`), not the profile name.
**How to avoid:** Resolve OQ-1 before locking CPLT-02. Two viable fixes (escalate to user): **(A)** grant the well-known `ALL APPLICATION PACKAGES` SID (`S-1-15-2-1`) — every AppContainer's token carries it, it is genuinely stable and engine-agnostic, RA-only on `C:\`/`C:\Users` is a negligible exposure widening; **(B)** introduce a *stable, profile-derived* AppContainer moniker for confined engines (e.g. `nono.engine.<profile>`) so the package SID is reproducible — but this is a larger change touching the moniker single-source invariant (broker + WFP both derive from the same name) and per-run uniqueness is currently load-bearing for WFP per-agent isolation (Phase 79 WFP-01 relies on distinct package SIDs per agent). **(A) is strongly preferred** — it is durable, generic (matches D-06's engine-agnostic intent), and does not disturb the per-run-SID WFP isolation model.
**Warning signs:** Any plan task that derives the admin-grant SID from `app_container_name` or a per-run UUID.

### Pitfall 2: `copilot` shim resolution on Windows is fragile (D-02 coverage gap)
**What goes wrong:** `windows_interpreters: ["node.exe"]` covers Node, but the `copilot` command on Windows is a `%APPDATA%\npm\copilot.cmd` (or `.ps1`) shim that itself must be resolvable and must locate the package's JS entry. A documented bug exists where `copilot.ps1` "fails to find the npm-installed binary on Windows when both are in PATH." If nono's launch-coverage gate covers only `node.exe` but the actual invocation goes through the `.cmd`/`.ps1` shim under `%APPDATA%\npm`, the shim dir and the package's install dir (`%APPDATA%\npm\node_modules\@github\copilot`) may also need coverage.
**Why it happens:** npm global installs on Windows create `.cmd`/`.ps1` shim wrappers; the real entry is `node <pkg>\index.js`. The launch target nono sees depends on how the gate invokes `copilot` (bare `copilot`, `copilot.cmd`, or `node <path>`).
**How to avoid:** Settle empirically on the live host (D-02 is explicitly "settle during research/execution"). Recommended gate invocation that sidesteps shim ambiguity: resolve the package entry and invoke **`node <copilot-entry>`** directly, OR `--allow %APPDATA%\npm` and `%APPDATA%\npm\node_modules` in the gate's nono invocation. The exe-coverage contract (R-B3 / spike-findings) means an uncovered shim/interpreter → fail-secure refusal to launch, which surfaces as a clear coverage error (not a silent failure).
**Warning signs:** Launch refused with an "executable not covered" diagnostic before Copilot even starts.

### Pitfall 3: Cross-target clippy blindness on cfg-gated Windows code
**What goes wrong:** The new `grant_sid_read_attributes_on_path` and `AppliedAncestorReadAttributesGuard` are `#[cfg(target_os = "windows")]`. A Windows-host `cargo check` does NOT run clippy and does NOT exercise the non-Windows cfg branches; CI runs `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` on ubuntu+macos and will catch drift the dev host cannot.
**Why it happens:** This dev host cannot compile non-Windows targets; the v2.12 milestone repeatedly shipped latent cross-target errors (see MEMORY: PR #9 layered E0124/E0425/dead-code errors).
**How to avoid:** Per CLAUDE.md, any commit touching cfg-gated code MUST be verified via `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`, or marked PARTIAL and deferred to live CI per the cross-target-verify checklist. Ensure the new Windows-only items are properly cfg-gated so non-Windows builds don't see dead code (mirror how `AppliedAncestorTraverseGuard` and its tests are gated: `#[cfg(test)] #[cfg(target_os = "windows")]`).
**Warning signs:** New `pub fn` with no non-Windows definition referenced unconditionally; tests that reference Windows-only symbols without cfg gating.

### Pitfall 4: Gate non-determinism / interactivity
**What goes wrong:** `copilot` is "an interactive, agentic shell you launch once" (per GitHub docs). A naive gate that launches interactive Copilot will hang the unattended harness.
**Why it happens:** The CLI's default mode is a REPL; the dark-factory mandate requires a single unattended invocation.
**How to avoid:** Use Copilot's non-interactive/one-shot mode (the gate's "real task" must be a single scriptable command that exits, e.g. a `-p`/prompt-and-exit form — confirm the exact flag on the live host). Apply a timeout. The assertion is on confinement behavior (no `STATUS_ACCESS_DENIED`, no Node module-resolution crash), not on Copilot's answer quality.
**Warning signs:** Gate exceeds timeout; no stdout captured.

## Code Examples

### Adding the RA grant (library)
```rust
// crates/nono/src/sandbox/windows.rs — mirror grant_sid_read_on_path (line 1857)
/// Adds an allow-ACE granting `sid` FILE_READ_ATTRIBUTES (0x80) on `path`,
/// PRESERVING the existing DACL. The MINIMAL right that satisfies Node-ESM
/// realpathSync/lstat's per-ancestor stat under AppContainer (CPLT-01, D-09).
#[cfg(target_os = "windows")]
pub fn grant_sid_read_attributes_on_path(path: &Path, sid: &str) -> Result<()> {
    use windows_sys::Win32::Security::Authorization::SET_ACCESS;
    use windows_sys::Win32::Security::NO_INHERITANCE;
    edit_dacl_for_sid(path, sid, PACKAGE_SID_READ_ATTRS_MASK, SET_ACCESS, NO_INHERITANCE)
}
// Then re-export in lib.rs alongside grant_sid_read_on_path (line ~86).
```

### Idempotent admin grant (setup runtime, CPLT-02)
```rust
// crates/nono-cli/src/setup.rs — sketch
// 1. Resolve the STABLE grantee SID (OQ-1 — recommend ALL APPLICATION PACKAGES "S-1-15-2-1").
// 2. For each system ancestor in ["C:\\", "C:\\Users"]:
//    - check the ACE is not already present (idempotency — query DACL, EqualSid match);
//      reuse the dacl_contains_sid technique from dacl_guard.rs tests (GetAce loop).
//    - if absent: grant_sid_read_attributes_on_path(ancestor, sid)  (needs admin/WRITE_DAC on C:\).
//    - never remove/modify an existing ACE (D-09 non-destructive).
// Fail-closed: surface a clear "requires elevation" error if SetNamedSecurityInfoW returns access-denied.
```

### The CPLT-03 gate (skeleton)
```powershell
# scripts/gates/copilot-e2e.ps1
function Test-Precondition {
    if (-not (Get-Command copilot -ErrorAction SilentlyContinue)) { return 'copilot CLI not installed' }
    # probe auth + network (e.g. a cheap authed status call); if not authed/online → return a reason string
    return $null
}
function Invoke-Gate {
    # Run a ONE-SHOT confined copilot task under nono and capture stdout/stderr.
    #   nono run --profile copilot-cli -- copilot <one-shot prompt flag> "list files"
    # Assert (FAIL on violation, but a missing-copilot/auth error is a SKIP via Test-Precondition):
    #   - no 'STATUS_ACCESS_DENIED' / 'Access is denied' in output
    #   - no Node module-resolution crash (e.g. 'Cannot find module', ENOENT on a module path, ERR_MODULE_NOT_FOUND)
    #   - copilot printed a real suggestion (non-empty expected output)
    return [ordered]@{ gate='copilot-e2e'; verdict=$verdict; reason=$reason; detail=$detail; timestamp=(Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ') }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Copilot "confine-only" (SC3 re-scope) | End-to-end confinement via ancestor-RA grant | Phase 77 (this) | The whole point of CPLT-01/02/03. |
| `copilot-cli` = native PE, no interpreter (D-06 finding) | `copilot` = standalone Node CLI, needs `node.exe` coverage (D-01/D-02) | Phase 77 | The `copilot_cli_profile_is_native_pe` test (builtin.rs:290) must be inverted; the profile description string (policy.json:907) is now wrong. |
| Interactive human SC3 UAT | Unattended `verify-dark.ps1 --gate copilot-e2e` | Phase 76/77 | Dark-factory mandate. |
| `gh copilot suggest` example (ROADMAP) | Standalone `copilot` invocation (D-03) | Phase 77 | Planner reconciles wording. |

**Deprecated/outdated:**
- The `copilot-cli` profile's `meta.description` (policy.json:907) and the two builtin.rs tests (`copilot_cli_profile_present` partly, `copilot_cli_profile_is_native_pe` entirely) encode the stale D-06 native-PE assumption and must be updated.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `FILE_READ_ATTRIBUTES` (0x80) alone is the minimal mask satisfying Node-ESM `realpathSync`/`lstat` on each ancestor. The 75-08 finding says "FILE_READ_ATTRIBUTES"; whether `SYNCHRONIZE`/`READ_CONTROL`/`FILE_LIST_DIRECTORY` are also needed is unverified on a live host. | Standard Stack / Pattern 1 | If too narrow, the live gate shows `STATUS_ACCESS_DENIED` on the walk → widen the mask. Cheap to fix; the gate is the detector. |
| A2 | The `copilot` Windows invocation resolves to `node.exe` running the package JS entry via an `%APPDATA%\npm\copilot.cmd`/`.ps1` shim. | Pitfall 2 | If the shim layout differs (e.g. a bundled standalone exe variant), interpreter coverage and the gate invocation change. Settle on the live host (D-02). `[CITED: npmjs.com/@github/copilot + npm Windows global-bin docs]` |
| A3 | The well-known `ALL APPLICATION PACKAGES` SID (`S-1-15-2-1`) is present on every nono AppContainer child token and is a sound, stable grantee for the CPLT-02 admin grant. | Pitfall 1 / OQ-1 | If a confined child's token does NOT carry it (e.g. a "restricted" AppContainer flavor uses `ALL RESTRICTED APPLICATION PACKAGES` `S-1-15-2-2` instead), the grant wouldn't apply. Verify which package-group SID the broker's `SECURITY_CAPABILITIES` produces on the live host before locking. |
| A4 | `copilot` has a non-interactive one-shot mode suitable for an unattended gate. | Pitfall 4 | If it is REPL-only, the gate must drive it via stdin scripting + timeout. Confirm the flag on the live host. |
| A5 | `policy.json` is embedded at build time and a profile edit needs `make build` to take effect. | Runtime State Inventory | Standard for this repo (`build.rs` embeds `data/`); low risk. `[CITED: CLAUDE.md "Embedded at build time via build.rs"]` |

## Open Questions

1. **OQ-1 (BLOCKING for CPLT-02): The package SID is per-run (UUID), not profile-stable — D-05 is falsified.**
   - **What we know:** `generate_app_container_name()` returns `nono.session.<Uuid::new_v4().simple()>` (`restricted_token.rs:52`); `derive_app_container_sid` deterministically derives the package SID from that name. So the SID is unique per run, not stable across runs. Confirmed by reading the source AND the test `generate_app_container_name_is_unique_and_well_formed` (restricted_token.rs:262) which asserts two calls differ. Also: per-run uniqueness is load-bearing for Phase 79 WFP-01 (distinct package SID per agent).
   - **What's unclear:** Which stable principal should the one-time-admin RA grant target?
   - **Recommendation:** Escalate to discuss-phase. Strongly prefer grantee = **`ALL APPLICATION PACKAGES` (`S-1-15-2-1`)** — durable, engine-agnostic (matches D-06 intent), RA-only on `C:\`/`C:\Users` is a negligible widening, and it leaves the per-run-SID WFP isolation model untouched. Verify (A3) which app-package-group SID the broker token actually carries before locking. The alternative (a stable profile-derived moniker) conflicts with the per-run-uniqueness invariant and is higher risk.

2. **OQ-2: Exact RA mask (A1).** Start `FILE_READ_ATTRIBUTES` only; let the live gate prove whether more bits are needed. Resolvable during execution, not blocking for planning.

3. **OQ-3: `copilot` shim coverage + one-shot flag (A2/A4).** The gate's nono invocation and the profile's coverage may need `%APPDATA%\npm` + `%APPDATA%\npm\node_modules`. Settle on the live host (D-02 is explicitly empirical). Not blocking for the CPLT-01 code; blocking for the CPLT-03 gate body.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Win11 host (real) | CPLT-01 runtime guard live proof, CPLT-03 gate | Operator host only (dev host is Win11 but tests are `#[ignore]`-gated for CreateAppContainerProfile) | Win11 26200 per MEMORY | None — confinement proof is host-gated (→ SKIP_HOST_UNAVAILABLE by design). |
| `@github/copilot` npm CLI | CPLT-03 gate | ✗ (must `npm install -g @github/copilot`) | requires Node ≥ 22 | Gate `Test-Precondition` → SKIP_HOST_UNAVAILABLE. |
| Node.js ≥ 22 | `copilot` interpreter | host-dependent | — | Same SKIP path. |
| GitHub auth + network | CPLT-03 PASS (authed suggestion) | host-dependent | — | SKIP_HOST_UNAVAILABLE (D-07). |
| Linux + macOS cross-toolchain | cross-target clippy on new cfg-gated code | host-dependent (per MEMORY, often not installed → CI is the only signal) | — | Mark verification PARTIAL, defer to live CI per cross-target-verify checklist. |
| `make build` (Rust 1.77+, Cargo) | rebuild after profile edit (embedded data) | ✓ | Rust 1.77 | — |

**Missing dependencies with no fallback:** None that block *planning*. The live-host + Copilot-install + auth gaps are *by design* handled via `SKIP_HOST_UNAVAILABLE` (D-07) — the gate is built to run unattended on any host and skip cleanly where prerequisites are absent.

**Missing dependencies with fallback:** Cross-toolchain (→ defer to CI, PARTIAL); Copilot/Node/auth (→ SKIP).

## Validation Architecture

> `nyquist_validation: true` in `.planning/config.json` — section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework (Rust) | Rust built-in test runner (`cargo test`); Windows-only DACL tests `#[cfg(test)] #[cfg(target_os = "windows")]` |
| Framework (gate) | PowerShell dark-factory harness (`scripts/verify-dark.ps1` + `scripts/gates/`) |
| Config file | `Cargo.toml` (workspace); gates auto-discovered from `scripts/gates/*.ps1` (no config) |
| Quick run command | `cargo test -p nono windows::tests::` (lib DACL fn) / `cargo test -p nono-cli dacl_guard::tests::` (guard) |
| Full suite command | `make test` (Rust) + `pwsh scripts/verify-dark.ps1 --gate copilot-e2e` (gate) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CPLT-01 | `grant_sid_read_attributes_on_path` adds an RA ACE and `revoke_sid_on_path` removes it | unit (Windows) | `cargo test -p nono windows::tests::grant_read_attributes` | ❌ Wave 0 (mirror `grant_sid_traverse_on_path` test at windows.rs:5118) |
| CPLT-01 | `AppliedAncestorReadAttributesGuard` grants RA on user-owned ancestors, reverts on Drop | unit (Windows) | `cargo test -p nono-cli dacl_guard::tests::ancestor_read_attributes` | ❌ Wave 0 (mirror `ancestor_traverse_grants_owned_ancestors_and_reverts_on_drop`, dacl_guard.rs:533) |
| CPLT-01 | Guard STOPS at first non-owned ancestor (never touches `C:\Users`/`C:\`) — proves the D-04 split | unit (Windows) | `cargo test -p nono-cli dacl_guard::tests::ancestor_read_attributes_stops_at_non_owned` | ❌ Wave 0 (mirror `ancestor_traverse_stops_at_non_owned_ancestor`, dacl_guard.rs:570) |
| CPLT-01 | `copilot-cli` profile declares `["node.exe"]` interpreter coverage | unit | `cargo test -p nono-cli profile::builtin::tests::copilot_cli` | ⚠ EXISTS but ASSERTS THE OPPOSITE — `copilot_cli_profile_is_native_pe` (builtin.rs:290) asserts `windows_interpreters.is_empty()`. Must be INVERTED to assert `["node.exe"]`. |
| CPLT-02 | Admin grant is idempotent — running twice leaves exactly one ACE | unit (Windows) + gate | `cargo test -p nono-cli setup::tests::grant_ancestors_idempotent` | ❌ Wave 0 (use the `dacl_contains_sid`/`GetAce` count technique from dacl_guard.rs:323) |
| CPLT-02 | Admin grant is non-destructive — pre-existing ACEs unchanged, no deny-ACE touched (D-09) | unit (Windows) | `cargo test -p nono-cli setup::tests::grant_ancestors_non_destructive` | ❌ Wave 0 |
| CPLT-02 | `nono setup --grant-ancestors --profile <p>` parses + dispatches | unit | `cargo test -p nono-cli` (clap parse test on SetupArgs) | ❌ Wave 0 |
| CPLT-03 | Unattended gate: confinement failure → FAIL; copilot/auth/network absence → SKIP | gate (host) | `pwsh scripts/verify-dark.ps1 --gate copilot-e2e` (exit 0=PASS / 2=FAIL / 3=SKIP / 4=harness-error) | ❌ Wave 0 (`scripts/gates/copilot-e2e.ps1`) |
| CPLT-03 | Gate emits a valid verdict object + persists to `.nono-runtime/verdicts/copilot-e2e.json` | gate | (covered by running the gate; runner enforces shape) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** the relevant `cargo test -p <crate> <module>::` quick run (< 30s).
- **Per wave merge:** `make test` (Rust full suite) + cross-target clippy per CLAUDE.md (or PARTIAL + defer to CI).
- **Phase gate:** `make ci` green where compilable on the dev host; the host-gated `copilot-e2e` gate is the CPLT-03 acceptance proof (run on the operator's real Win11 host; SKIP is an acceptable verdict where Copilot/auth absent, but a real PASS is required to close CPLT-03 — see D-08).

### Wave 0 Gaps
- [ ] `crates/nono/src/sandbox/windows.rs` — unit tests for `grant_sid_read_attributes_on_path` (apply + revoke + bad-SID fail-closed), mirroring lines 5118-5136.
- [ ] `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` — `AppliedAncestorReadAttributesGuard` + its two tests (owned-ancestor grant/revert; stop-at-non-owned).
- [ ] `crates/nono-cli/src/profile/builtin.rs` — INVERT `copilot_cli_profile_is_native_pe` → assert `windows_interpreters == ["node.exe"]`; update `copilot_cli_profile_present`'s description-coupled expectations.
- [ ] `crates/nono-cli/src/setup.rs` — idempotency + non-destructiveness unit tests for the grant-ancestors path (Windows-gated).
- [ ] `scripts/gates/copilot-e2e.ps1` — the gate (Test-Precondition + Invoke-Gate); covers CPLT-03.
- [ ] No new framework install needed — Rust test runner + the Phase 76 PowerShell harness already exist.

## Security Domain

> `security_enforcement` not set to `false` in config — section included. This is a security-critical confinement codebase (CLAUDE.md: "SECURITY IS NON-NEGOTIABLE").

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture | yes | Least-privilege RA grant (D-09); structural runtime-vs-admin split by OS ownership (D-04). |
| V4 Access Control | yes | DACL allow-ACE for a specific SID; `path_is_owned_by_current_user` gate before any DACL edit; fail-closed on ownership-check errors (never swallow). |
| V5 Input Validation | yes | SID strings parsed via `ConvertStringSidToSidW` fail-closed (`parse_sid`, windows.rs:1555); profile name validated (non-empty) before SID derivation. |
| V6 Cryptography | no | No crypto in this phase. |
| V7 Errors/Logging | yes | `tracing::warn!`/`debug!` on skip/revert; `NonoError::DaclApplyFailed` carries path + HRESULT + hint. |
| V12 File/Resources | yes | Path-component comparison only (CLAUDE.md footgun #1); canonicalization at the enforcement boundary; minimal grant scope (RA, not full read). |

### Known Threat Patterns for {Windows AppContainer + DACL grant}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Over-broad grant on `C:\`/`C:\Users` | Elevation of Privilege / Information Disclosure | `FILE_READ_ATTRIBUTES` only (0x80) — attribute-read, not content-read; scoped to one stable package-group SID; documented non-destructive (D-09). |
| Granting a principal that is broader than intended (e.g. full read, or a user-group SID) | EoP / Info Disclosure | Grantee = a package-group SID that only AppContainer tokens carry (recommend `ALL APPLICATION PACKAGES`); never the interactive user or Everyone. |
| Duplicate/stacked ACEs on re-run | Tampering (DACL bloat) | CPLT-02 idempotency check (query DACL, EqualSid match before grant) — verified by a unit test. |
| TOCTOU on ancestor ownership | Tampering | Ownership check + grant are sequential Win32 calls on the same path; the runtime grant is reverted on Drop; system roots are admin-only and rarely change ownership. Accept-minimal per the existing traverse-guard threat model (T-62-33). |
| String path comparison vulnerability | Tampering | `Path`-component comparison and `path_is_owned_by_current_user` — never `starts_with` on path strings (CLAUDE.md footgun #1). |
| Fail-open on grant error | (Defense-in-depth) | `edit_dacl_for_sid` fails closed at every step; the guard reverts already-applied entries and propagates the original error. |

## Sources

### Primary (HIGH confidence)
- `crates/nono/src/sandbox/windows.rs` (lines 720-820 SID derivation; 1480-1880 mask constants + `edit_dacl_for_sid` + grant fns) — read in session.
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` (full file) — `AppliedDaclGrantsGuard` + `AppliedAncestorTraverseGuard` shapes + tests.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` (120-160 ExecConfig; 300-459 guard wiring) — read in session.
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` (34-53) — **`generate_app_container_name` = per-run UUID (falsifies D-05)**.
- `crates/nono-cli/data/policy.json` (860-936) — `copilot-cli`, `aider`, `langchain-python` profile shapes.
- `crates/nono-cli/src/profile/builtin.rs` (220-299) — the stale native-PE tests to invert.
- `crates/nono-cli/src/cli.rs` (640-665 Commands::Setup; 2570-2638 SetupArgs) — setup surface.
- `scripts/verify-dark.ps1` + `scripts/gates/harness-self-check.ps1` (full files) — Phase 76 gate contract + exit mapping.
- `.planning/phases/77-.../77-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/milestones/v2.12-ROADMAP.md` (SC3/75-08 lines 69-84).
- `.claude/skills/spike-findings-nono/` (SKILL.md + engine-agnostic-confinement.md) — exe-coverage + absolute-grant + R-B3 contracts.

### Secondary (MEDIUM confidence)
- [npmjs.com/package/@github/copilot](https://www.npmjs.com/package/@github/copilot) — `npm install -g @github/copilot`, Node ≥ 22, interactive agentic shell.
- [GitHub Docs: Installing Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli) — install channels (npm/brew/winget/standalone).
- [npm Windows global-bin shim docs / grokipedia](https://grokipedia.com/page/npm_global_bin_directory_on_Windows) — `%APPDATA%\npm\*.cmd` shim layout.
- [microsoft/vscode#291990](https://github.com/microsoft/vscode/issues/291990) — `copilot.ps1` shim resolution fragility on Windows (informs Pitfall 2).

### Tertiary (LOW confidence)
- Copilot one-shot/non-interactive flag exact spelling (A4) — not verified; settle on live host.

## Metadata

**Confidence breakdown:**
- Code anchors (guard shapes, mask core, SID derivation, setup surface, gate contract): HIGH — every cited line read on-disk this session.
- D-05 falsification (per-run SID): HIGH — read from source + an asserting unit test.
- Copilot shim runtime behavior (A2/A4): MEDIUM — official docs + npm docs, not a live host.
- Minimal RA mask (A1): MEDIUM — the 75-08 finding names `FILE_READ_ATTRIBUTES`; exact sufficiency is a live-host question the gate answers.
- Recommended admin grantee SID (A3): MEDIUM — `ALL APPLICATION PACKAGES` is the standard AppContainer group SID, but verify the broker token carries it before locking.

**Research date:** 2026-06-17
**Valid until:** 2026-07-17 (stable internal codebase; Copilot CLI is fast-moving — re-confirm shim/flag facts at execution time).
