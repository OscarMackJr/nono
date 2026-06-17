# Phase 77: Copilot CLI End-to-End Confinement - Context

**Gathered:** 2026-06-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Make **GitHub Copilot CLI complete a real task fully under AppContainer confinement** — replacing the v2.12 Phase 75 "confine-only" SC3 re-scope. Three deliverables:

1. **Runtime fix (CPLT-01):** nono grants `FILE_READ_ATTRIBUTES` (RA) on every ancestor in the confined target's package-SID path up to the drive root *at launch time*, so Node-ESM module resolution (`realpathSync`/`lstat` walking every path ancestor) succeeds under AppContainer instead of being denied with `STATUS_ACCESS_DENIED`.
2. **One-time-admin grant (CPLT-02):** an idempotent setup command grants the package-SID RA on the *system* ancestors (`C:\`, `C:\Users`) that nono cannot ACL at runtime as the current user — verified non-destructive.
3. **Scripted end-to-end gate (CPLT-03):** an unattended `scripts/verify-dark.ps1 --gate copilot-e2e` proves Copilot completes a real task under confinement, replacing the interactive SC3 UAT.

**In scope:** the ancestor-RA runtime code path, the admin setup command, the `copilot-cli` profile update for the chosen target binary, and the `copilot-e2e` harness gate.
**Out of scope:** any non-Copilot engine work; WFP/network changes beyond Copilot's existing `network: { block: false }`; the harness *framework* itself (delivered in Phase 76).
</domain>

<decisions>
## Implementation Decisions

### Target binary
- **D-01:** The confinement target is the **standalone `@github/copilot` (Node) CLI** — invoked as `copilot` — NOT `gh copilot` (gh-extension) and NOT a native-PE `copilot.exe`. This is the Node-ESM `realpathSync`/`lstat` ancestor-walk case the phase goal describes.
- **D-02:** The `copilot-cli` profile must be updated to add **`node.exe` interpreter coverage** (`windows_interpreters: ["node.exe"]`). The current profile's D-06 finding ("copilot.exe is a native PE, no Node interpreter needed") is **stale/superseded** for this target — researcher should confirm on-host how the installed standalone `copilot` resolves to a Node process, then wire interpreter coverage accordingly.
- **D-03 (derived, locked):** SC1's example string `gh copilot suggest "list files"` in the ROADMAP is **superseded** — the gate and SC1 proof use the standalone `copilot` invocation. Planner should reconcile the ROADMAP/REQUIREMENTS example wording with the standalone binary.

### Ancestor-RA grant split
- **D-04 (derived, locked):** The runtime code (CPLT-01) grants RA on every ancestor in the package-SID path that nono **can ACL as the current user** (the user-ownable ancestors). The **system ancestors `C:\` and `C:\Users`** — which nono cannot ACL at runtime as a non-elevated user — are covered exclusively by the one-time-admin setup step (CPLT-02). The split is determined by OS ownership, not configuration.
- **D-05 (FALSIFIED + RESOLVED 2026-06-17):** The carried-forward assumption that the AppContainer package SID is profile-derived/stable is **WRONG** — `generate_app_container_name()` (`restricted_token.rs:52`) returns `nono.session.<Uuid::new_v4().simple()>`, so the package SID is **per-run (UUID)**, and per-run uniqueness is load-bearing for Phase 79 WFP isolation. **Resolution (user decision, locked):** the CPLT-02 one-time-admin durable RA grant on the system ancestors targets the **well-known `ALL APPLICATION PACKAGES` SID (`S-1-15-2-1`)** — durable, engine-agnostic, RA-only (attribute-read, not content) widening, and leaves the per-run-SID WFP model untouched. Every per-run AppContainer token is a member of `ALL APPLICATION PACKAGES`, so the durable grant covers any run. The **CPLT-01 runtime grant on user-ownable ancestors keeps targeting the per-run package SID** (same-run, tighter scope) — only the durable admin grant uses the well-known SID.

### Setup command surface
- **D-06:** Expose the one-time-admin grant as a **generic, reusable command** — `nono setup --grant-ancestors --profile <p>` (or equivalent generic shape that takes a profile and grants package-SID RA on the system ancestors for that profile's derived SID). NOT a Copilot-specific `--copilot-ancestors` flag. Rationale: the v2.13 / SEED-004 theme is engine-agnostic confinement; future Node engines reuse the same command. Exact flag/subcommand naming is planner discretion as long as it is generic, idempotent, one-time-admin, and non-destructive.

### Gate auth model
- **D-07:** The `copilot-e2e` gate treats **Copilot install + GitHub auth + network as a precondition**. If absent, the gate emits **`SKIP_HOST_UNAVAILABLE`** (per the Phase 76 gate contract) — NOT `FAIL`.
- **D-08:** `PASS` requires a **real, authenticated `copilot` suggestion that prints output with zero `STATUS_ACCESS_DENIED` and zero Node module-resolution crash** under AppContainer. The gate must distinguish a *confinement* failure (FAIL) from an unrelated Copilot/auth/network failure (SKIP) — confinement-fix verification is the assertion, not Copilot's own availability.

### Grant posture (persistence / reversibility)
- **D-09:** The admin grant is **permanent and documented, with no undo command**. Justification: `FILE_READ_ATTRIBUTES` is attribute-read only (not content read), granted to the well-known `ALL APPLICATION PACKAGES` SID (per D-05 resolution), on `C:\`/`C:\Users` only. Document it as non-destructive (adds one allow-ACE per ancestor; alters/removes no existing ACE and no deny-ACE). No `--revoke` counterpart in this phase.

### Claude's Discretion
- Exact CLI flag/subcommand spelling for the generic grant command (D-06), provided it stays generic + idempotent + non-destructive.
- The minimal scriptable "real task" command/args for the gate (D-08), provided it exercises the Node-ESM ancestor walk and is unattended.
- The precise precondition probe the gate uses to detect Copilot install + auth (D-07).
- Whether `node.exe` interpreter coverage is enough or the standalone `copilot` shim needs additional path coverage (D-02) — settle empirically during research.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Carry-forward origin (the problem this phase fixes)
- `.planning/milestones/v2.12-ROADMAP.md` § Phase 75 SC3 + GAP-75-C (75-08) — the original Copilot confine-only re-scope and the Node-ESM `realpathSync`/`lstat` ancestor-RA finding that defines CPLT-01.
- `Skill("spike-findings-nono")` — `./.claude/skills/spike-findings-nono/SKILL.md` + `references/engine-agnostic-confinement.md`: exe-coverage + absolute-grant contracts, R-B3 (workspace must be `%USERPROFILE%`-owned), user-mode-only constraint.

### Confinement / AppContainer mechanics
- `proj/DESIGN-engine-abstraction.md` — E1–E5 engine-abstraction contract (interpreter coverage model the `node.exe` addition plugs into).
- Memory `windows_appcontainer_wfp_validated.md` — package SID derived from profile name (CreateAppContainerProfile), stable across runs; package SID needs explicit read/traverse grants (different principal than the user).
- Memory `feedback_windows_mandatory_label_write_owner.md` — drive-root vs `%USERPROFILE%` ownership rules; `path_is_owned_by_current_user` + effective-rights mask check pattern (informs the runtime-vs-admin ancestor split, D-04).

### Harness gate framework (dependency: Phase 76)
- `.planning/phases/76-self-verifying-harness-foundation/` — `scripts/verify-dark.ps1` runner + `scripts/gates/harness-self-check.ps1` reference for the per-gate `Test-Precondition` / `Invoke-Gate` contract that `copilot-e2e` must implement (PASS / FAIL / SKIP_HOST_UNAVAILABLE).

### Code anchors (scout)
- `crates/nono-cli/data/policy.json` § `copilot-cli` (≈ line 902) — profile to update with `node.exe` interpreter coverage.
- `crates/nono-cli/src/profile/builtin.rs` (≈ line 252, D-06 finding) — stale native-PE assumption to revise.
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` + `crates/nono-cli/src/exec_strategy_windows/launch.rs` — existing per-path SID grant/guard machinery (`grant_sid_write_on_path` / `AppliedDaclGrantsGuard` analog) where the launch-time ancestor-RA grant (CPLT-01) belongs.
- `crates/nono-cli/src/cli.rs` (≈ line 665, `Setup(SetupArgs)`) — existing `nono setup` surface to extend for the generic `--grant-ancestors` command (D-06).
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Per-path SID grant + RAII guard:** `grant_sid_write_on_path` / `revoke_sid_on_path` / `AppliedDaclGrantsGuard` (`exec_strategy_windows/dacl_guard.rs`, from Phase 60 F-60-UAT-04) — the launch-time ancestor-RA grant should follow this same grant-then-guard shape (granting `FILE_READ_ATTRIBUTES` instead of write).
- **`nono setup` command:** existing `Setup(SetupArgs)` in `cli.rs` — extend rather than add a new top-level verb for the generic ancestor-grant command.
- **Profile interpreter model:** `windows_interpreters` already exists on sibling profiles (`langchain-python` uses `["python.exe"]`) — adding `["node.exe"]` to `copilot-cli` is an established pattern, not new machinery.

### Established Patterns
- **Executable-coverage contract:** launch policy must cover the engine's executable + interpreter path or nono fail-secure refuses to launch (R-B3 / spike-findings). The `node.exe` coverage addition is required for the standalone Copilot to launch at all.
- **Package SID is a distinct principal** from the user — RA grants must target the derived package SID, not the user SID.
- **Dark-factory gate contract (Phase 76):** every host-gated proof is a named gate emitting a typed verdict; `copilot-e2e` plugs into `scripts/verify-dark.ps1`.

### Integration Points
- Launch-time RA grant hooks into the Windows exec strategy where the AppContainer/lowbox child is spawned (`exec_strategy_windows/launch.rs`).
- The `copilot-e2e` gate file lands under `scripts/gates/` alongside `harness-self-check.ps1` and is dispatched by `verify-dark.ps1 --gate copilot-e2e`.
</code_context>

<specifics>
## Specific Ideas

- Canonical proof command is the **standalone `copilot` suggestion** (e.g., a "list files"-style suggest), run under `nono run --profile copilot-cli`, asserting no `STATUS_ACCESS_DENIED` and no Node module-resolution crash.
- The runtime-vs-admin split is the crux: nono auto-grants the **user-ownable** ancestors at launch; only `C:\` and `C:\Users` need the one-time-admin step.
</specifics>

<deferred>
## Deferred Ideas

- **`--revoke` / uninstall counterpart for the ancestor grant** — considered and explicitly deferred (D-09: permanent, documented, no undo this phase). Revisit if a clean-uninstall story becomes a requirement.
- **Copilot-specific setup flag** — rejected in favor of the generic `--grant-ancestors --profile <p>` (D-06).

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` — MSI VC++ prereq; belongs to **Phase 80** (clean-host install), not Copilot confinement. Weak keyword match only.
- `20260611-poc-cert-broker-clean-host.md` — untrusted-POC-cert broker; **enterprise-hardening** deferral, out of v2.13 scope.
- `20260612-macos-rlimit-as-setrlimit-fails.md` — macOS resource-limit defect; unrelated platform, not this phase.
</deferred>

---

*Phase: 77-copilot-cli-end-to-end-confinement*
*Context gathered: 2026-06-17*
