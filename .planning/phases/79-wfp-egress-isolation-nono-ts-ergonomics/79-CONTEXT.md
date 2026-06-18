# Phase 79: WFP Egress Isolation + nono-ts Ergonomics - Context

**Gathered:** 2026-06-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Two independent deliverables in one phase:

1. **WFP-01 — Empirical per-agent WFP egress isolation gate.** An automated, unattended gate (`scripts/verify-dark.ps1 --gate wfp-egress-isolation`) that launches **two** confined agents with **distinct AppContainer package SIDs**, runs them concurrently, and proves per-SID egress isolation: one agent's egress to a test server succeeds while the other (network-scoped) agent's egress is denied — both verdicts machine-verifiable in one run. This is a **test of already-shipped WFP machinery** (Phase 75 `wfp_filter_add/remove`, keyed by package SID), **not** a new WFP integration.

2. **TSRG-01 — `confinedRun` ergonomics in nono-ts.** Make `confinedRun` default to the Low-IL broker arm and auto-cover the target executable's directory, so a caller gets a working confined Low-IL child with **no manual profile or coverage flags** (`confinedRun({ target: "node" })` works out of the box). Includes a napi integration test on the Win11 dev host.

**Out of scope:** new WFP feature work (allow_domain→WFP allow-rule wiring); the Node-ESM ancestor `FILE_READ_ATTRIBUTES` problem (that is Phase 77's domain — confinedRun auto-cover here is exe-dir only); the clean-host MSI install (Phase 80).
</domain>

<decisions>
## Implementation Decisions

### WFP-01 — empirical isolation test
- **D-01 (allowed-vs-denied mechanism): block-vs-no-block contrast.** Agent B runs a **network-scoped (`network.block: true`) profile** → the shipped `wfp_filter_add` installs a per-SID WFP deny for B's package SID → B's egress is **denied**. Agent A runs a **non-blocked profile** → A's egress **succeeds**. Both run **concurrently with distinct package SIDs**, so a passing run proves the filter is correctly scoped per-SID (B's deny does not leak to A). Uses only shipped code — honors STATE's framing of WFP-01 as "a network-scoped test profile + empirical isolation test, not a new WFP integration."
  - **SC-1 wording note for the verifier:** ROADMAP SC1 phrases this as "agent A's *allowed* egress … while agent B (*no allow_domain*) is denied." Under D-01 this is realized as **A = non-blocked profile (allowed), B = block:true profile (denied)** rather than both-block:true-differing-by-allow_domain. The empirical claim WFP-01 actually requires — *per-agent egress isolation keyed by distinct package SID* — is fully proven by this shape. The proxy-mediated allow_domain variant and the allow_domain→WFP allow-rule feature were both explicitly **rejected** (the latter is new feature work, out of scope).
- **D-02 (egress target): non-loopback mock bind.** Bind the test server to a **non-loopback interface** (host LAN IPv4 or a routable test address), reusing the existing Rust mock TCP server shape (`spawn_multi_endpoint_server`), so the AppContainer's per-SID egress filter genuinely applies. Loopback (`127.0.0.1`) was rejected: AppContainer + WFP filtering may exempt loopback, which would make B's "deny" a false PASS. **Researcher MUST validate** how the per-SID WFP filter interacts with the chosen interface (and whether the AppContainer needs a network capability to reach a non-loopback host at all) before the gate is written — the gate's value is entirely in the deny being real.

### TSRG-01 — confinedRun ergonomics
- **D-03 (default broker-arm profile): new dedicated nono-ts default profile.** Add a **minimal, least-privilege profile to `policy.json`** (suggested name e.g. `nono-ts-default`; planner finalizes) that sets `windows_low_il_broker: true` with just-enough coverage. `confinedRun` uses this profile when the caller passes no profile, so the no-profile path reaches the **Low-IL broker arm** instead of today's WriteRestricted arm (which kills node `0xC0000142`). Explicit, auditable, engine-neutral, and decoupled from any AI-agent profile (reusing `claude-code` was rejected as engine-coupling; in-binding capability synthesis was rejected as diverging from the delegate-to-`nono.exe --profile` model).
- **D-04 (ergonomics & override surface): overridable options, new defaults ON, auto-cover exe-dir only.** Add **optional** flags to `confinedRun` (e.g. `lowIl?: boolean`, `autoCoverTarget?: boolean`) that **default to the new behavior** — existing callers automatically get Low-IL + auto-cover, but can opt out. **Auto-cover adds only the resolved target executable's own directory** to allowed-read paths (matches SC3 literally). Covering cwd as well was rejected to keep the default least-privilege. (Naming/typing of the options is the planner's call; keep backward-compatible with the current `confinedRun(exe, args, allow?, profile?, cwd?, timeout?)` signature.)

### Claude's Discretion
- Exact name/coverage of the new nono-ts default profile (D-03).
- Exact option names/types and how `lowIl`/`autoCoverTarget` map onto the existing positional/options signature (D-04).
- The `wfp-egress-isolation.ps1` gate's internal structure, as long as it follows the shipped `Test-Precondition`/`Invoke-Gate` contract.
- Whether the two test agents are launched via the daemon control pipe or a direct `nono run` path — pick whatever makes the concurrent-distinct-SID proof cleanest.

### Reviewed Todos (not folded)
Three pending todos keyword-matched Phase 79 at score 0.6 but belong elsewhere — **not folded**:
- `20260611-msi-vcredist-prereq.md` — clean-host MSI VC++ prereq → **Phase 80** (tagged `resolves_phase: 80`).
- `20260611-poc-cert-broker-clean-host.md` — POC-cert broker on clean host → **Phase 80**.
- `20260612-macos-rlimit-as-setrlimit-fails.md` — macOS RLIMIT_AS enforcement defect → a **macOS/v2.11 carry-forward**, unrelated to WFP/nono-ts.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### WFP-01 — shipped WFP machinery to reuse (do NOT rebuild)
- `crates/nono-cli/src/agent_daemon/launch.rs` §423-552, §665-691 — `wfp_filter_add` / `wfp_filter_remove` / `profile_needs_network_scoping` / fail-secure gate. Filter keyed to the agent's AppContainer package SID via `session_sid`; rule name `nono-agent-{tenant_id}`.
- `crates/nono-cli/src/windows_wfp_contract.rs` — `WfpRuntimeActivationRequest`/`Response` over `\\.\pipe\nono-wfp-control` (protocol v1). The WFP enforcement is a separate elevated `nono-wfp-service` (out-of-repo, IPC-driven) — see memory `windows_wfp_enforcement_is_service_only`.
- `crates/nono-cli/data/nono-profile.schema.json` §471-490 — `network.block`, `network.network_profile`, `network.allow_domain` (+ `proxy_allow`/`allow_proxy` aliases).
- `crates/nono-cli/data/policy.json` — profiles all currently `network.block:false`; the new `block:true` test profile(s) go here.
- `crates/nono-cli/tests/auto_pull_e2e_linux.rs` §57-127 — `spawn_multi_endpoint_server` reusable Rust mock server (ephemeral port, path routing). Adapt for a non-loopback bind.

### WFP-01 — the harness/gate contract
- `scripts/verify-dark.ps1` — runner; verdict classes PASS=0/FAIL=2/SKIP_HOST_UNAVAILABLE=3/HARNESS_ERROR=4; persist-before-emit (WR-04).
- `scripts/gates/copilot-e2e.ps1`, `scripts/gates/harness-self-check.ps1` — reference gates; copy the `Test-Precondition` (return `$null` to run / string reason → SKIP) + `Invoke-Gate` (return one `[ordered]@{ gate; verdict; reason; detail; timestamp }`) contract. New gate: `scripts/gates/wfp-egress-isolation.ps1`. Precondition must check the WFP service is reachable → SKIP_HOST_UNAVAILABLE if not.
- Memory `windows_appcontainer_wfp_validated` — per-run AppContainer (lowbox) scoped by package SID is WFP-kernel-blockable (both ALE_USER_ID + ALE_PACKAGE_ID); package SID needs explicit read/traverse grants.

### TSRG-01 — nono-ts (sibling repo `C:\Users\OMack\nono-ts`)
- `src/lib.rs` §350-402 — napi `confinedRun(exe, args, allow?, profile?, cwd?, timeout?)` export + non-Windows stub; `JsExecResult`.
- `src/windows_confined_run.rs` §76-94 — `build_nono_run_args` (delegates to `nono.exe run --profile <p> --allow <path> ...`). This is where the default-profile + auto-cover wiring lands.
- `tests/` + `package.json` §53 — `npm test` → `node test.js`; **no confinedRun integration test exists yet** — add one (no-profile-flags path on Windows).
- Memory `project_v212_phase71` (body) — `confinedRun(no profile)` uses the WriteRestricted arm → node dies `0xC0000142`; needs a `windows_low_il_broker` profile. napi win32 loader branch + `cfg(windows)` exports **must be regenerated on Windows** (was fixed `2bac4e2`). Drive-root `C:\poc\*` workspaces fail R-B3 — use `%USERPROFILE%`.

### Phase scope docs
- `.planning/ROADMAP.md` §139-150 — Phase 79 goal + 4 success criteria.
- `.planning/REQUIREMENTS.md` — WFP-01, TSRG-01.
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **WFP per-agent filter (shipped):** `wfp_filter_add/remove` in `launch.rs`, keyed by package SID — the test just exercises it via a `block:true` profile; no new WFP code.
- **Mock TCP server:** `spawn_multi_endpoint_server` (Rust, ephemeral port, HTTP routing) — adapt to a non-loopback bind for the egress target.
- **Gate framework:** `verify-dark.ps1` + the two reference gates — copy the `Test-Precondition`/`Invoke-Gate` contract verbatim.
- **confinedRun delegation:** `build_nono_run_args` already emits `--profile`/`--allow` to `nono.exe` — D-03/D-04 wiring slots in here (inject default profile when none given; append target-exe-dir to `--allow`).

### Established Patterns
- **Fail-secure WFP gate:** if `network.block:true` but the WFP service is unreachable, the agent is terminated before resume (`launch.rs` step 6.5). The gate's `Test-Precondition` should mirror this — WFP service down → SKIP_HOST_UNAVAILABLE, not FAIL.
- **Per-SID grants:** AppContainer package SID is a distinct principal — needs explicit read/traverse grants (memory `windows_appcontainer_wfp_validated`); relevant if the test agents need to reach a non-loopback mock.
- **Dark-factory verdict discipline:** persist-before-emit, machine-readable PASS/FAIL/SKIP — the new gate must not `exit` or persist itself (runner owns that).

### Integration Points
- New `block:true` test profile(s) → `policy.json`.
- New gate → `scripts/gates/wfp-egress-isolation.ps1` (auto-discovered by the runner).
- New default profile (`nono-ts-default`) → `policy.json`; consumed by `confinedRun` default path in `windows_confined_run.rs`.
- nono-ts napi rebuild on Windows (win32 loader + cfg(windows) exports) before the integration test can pass.
</code_context>

<specifics>
## Specific Ideas

- WFP-01's value is entirely in the **deny being real** — the researcher must confirm the chosen non-loopback target is genuinely blocked by the per-SID filter (and not silently allowed/exempt) before the gate is trusted. A green gate against an exempt target is worse than no gate.
- Keep `confinedRun` backward-compatible: new behavior is the default but every new knob is optional and opt-out-able.
</specifics>

<deferred>
## Deferred Ideas

- **allow_domain→WFP allow-rule wiring on Windows** — translating a profile's `allow_domain` into per-SID WFP *allow* filters (so both agents could be `block:true` and differ only by allow_domain). Real feature work; explicitly rejected for Phase 79's test. Candidate for a future network-hardening milestone.
- **confinedRun auto-cover of cwd / target ancestors** — broader coverage convenience (and the Node-ESM ancestor-RA problem) belongs to the Phase 77 RA-grant lineage, not here.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` — belongs to **Phase 80** (clean-host MSI).
- `20260611-poc-cert-broker-clean-host.md` — belongs to **Phase 80**.
- `20260612-macos-rlimit-as-setrlimit-fails.md` — macOS/v2.11 carry-forward, unrelated.
</deferred>

---

*Phase: 79-wfp-egress-isolation-nono-ts-ergonomics*
*Context gathered: 2026-06-18*
