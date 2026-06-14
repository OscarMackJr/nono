# Plan 73-03 Summary — Live wiring + `nono classify` verb + SC4/SC5

**Plan:** 73-03 (wave 2) · **Requirement:** MARK-01 · **Status:** Code-complete; host-gated UAT PENDING
**Executed:** 2026-06-14 (inline on main working tree — the background worktree executor was denied Bash; orchestrator completed it inline)

## What was delivered

Three automated tasks (all committed atomically), then the plan's final
`checkpoint:human-verify` gate, which is a real-Win11-host UAT documented as
PENDING below (not blocked-on inline — no host available in this session).

### Task 1 — mint→AgentRegistry wiring on the shipping path (`a021b10c`)
`execution_runtime.rs`: immediately after `windows_package_sid` is derived (the
per-run AppContainer package SID, also the WFP E4 identity), the SID is inserted
into a per-run `Arc<Mutex<nono::AgentRegistry>>`. This is the SHIPPING-path proof
of **SC1** — a launched `BrokerLaunchNoPty` agent is recorded in the launcher's
private authorization set at spawn time. `map_err` on `PoisonError` (no
`.unwrap()` per `clippy::unwrap_used`); insert clones so the `ExecConfig.package_sid`
move is unaffected. Registry is per-run/in-memory/single-launch (Phase 74 makes it
the daemon's shared state). `#[cfg(target_os = "windows")]`-gated.

### Task 2 — `nono classify <pid>` verb (`7b3fa977`)
- `classify_runtime.rs` (NEW, 177 lines): `run_classify(args, Arc<Mutex<AgentRegistry>>)`.
  Standalone the registry is empty, so the verb **never** emits an authoritative
  `AI_AGENT` verdict — it reports a structural pre-filter only (AppContainer SID
  present? in a job via `IsProcessInJob`?). Outcomes: `structural match
  (non-authoritative)` / `not an agent` / `not an agent (process not found or
  access denied)`. A non-authoritative **NOTE** is printed on **every** output
  path (human + JSON `note` field). Fail-secure throughout; `process_in_job` is
  cfg-gated with a non-Windows `false` stub.
- `cli.rs`: `ClassifyArgs { pid: u32, --json }` + `Commands::Classify` with help
  text stating the check is structural/non-authoritative.
- `app_runtime.rs`: dispatch arm creating a fresh per-call registry.
- `main.rs`: `mod classify_runtime;` (not cfg-gated — compiles everywhere via the
  `AgentRegistry` non-Windows stub).
- `cli_bootstrap.rs`: `Classify` added to the non-verbose command group (the
  `cli_verbosity` exhaustive match — required to compile).

### Task 3 — SC4 in-process integration tests + SC5 docs (`56fda2bd`)
- `launch.rs` `broker_dispatch_tests`: `sc4_classify_real_agent` (real
  AppContainer child + SID inserted → `AiAgent`) and `sc4_classify_spoof_not_agent`
  (real AppContainer, SID NOT inserted → `NotAnAgent`, proving D-02: registry
  membership not namespace is the predicate). Shared `spawn_appcontainer_child`
  helper mirrors the proven `SECURITY_CAPABILITIES` sequence from
  `examples/spike_wfp_appcontainer.rs`. Both `#[ignore]` (require a real Win11 host
  + `CreateAppContainerProfile`); the default `cargo test` skips them.
- `proj/DESIGN-engine-abstraction.md`: appended "AI_AGENT Marker: Scope and
  Adopted-Agent Limits" — standalone classify is structural/non-authoritative
  (empty cross-process registry), adopted agents are best-effort/demote-only, the
  `AgentRegistry` is the sole authoritative predicate.

## Self-Check: PASSED (automated, host-independent)

- `cargo build -p nono-cli` ✓
- `cargo test -p nono -p nono-cli` (non-ignored) ✓ — SC4 tests compile + are
  `ignored`; 6 sibling `broker_dispatch_tests` pass; `nono::agent` 4/4 pass;
  job-hardening 3/3 pass.
- `nono classify --help` shows the `PID` positional + structural/non-authoritative
  note ✓; `nono classify <pid>` (human + `--json`) emits the NOTE on every path
  and never prints `AI_AGENT` ✓.
- Grep gates: `adopted` ×5 (≥3) ✓, `non-authoritative` ×4 (≥2) ✓,
  `sc4_classify_real_agent` in launch.rs ✓, `AgentRegistry` in execution_runtime.rs
  ×3 ✓, `BREAKAWAY_OK` in launch.rs ×4 ✓.

### Clippy
- `cargo clippy -p nono -p nono-cli --bin nono --tests`: **zero warnings
  attributable to Phase 73 code.** One pre-existing warning remains —
  `cloned_ref_to_slice_refs` at `crates/nono/src/sandbox/windows.rs:3798` (a test
  NOT touched by Phase 73; a new lint from the clippy 1.95.0 bump). Left as-is
  (out of Phase 73 scope, consistent with the "don't chase pre-existing baseline
  issues mid-phase" guidance). Flag for a follow-up sweep.
- **Cross-target clippy (Linux + macOS): PARTIAL — deferred to live CI** per
  `.planning/templates/cross-target-verify-checklist.md`. The Windows dev host
  cannot cross-compile (`aws-lc-sys` C toolchain). New non-Windows surface is
  limited: `classify_runtime::process_in_job` non-Windows stub + `mod
  classify_runtime` (un-gated, relies on the `AgentRegistry` stub from Plan 01).

## PENDING — host-gated human-verify checkpoint (blocking gate)

Run on a **real Win11 host** with a **dev-layout** `nono.exe` (`target/release/`,
not a Program Files install — the broker trust gate). The orchestrator could not
execute these in-session (no host).

1. `target\release\nono.exe classify --help` → shows `PID` positional + structural/non-authoritative note.
2. `target\release\nono.exe classify <unrelated-pid>` → `not an agent` + NOTE.
3. `target\release\nono.exe classify <current-powershell-pid>` → `not an agent` + NOTE.
4. **Structural match:** Terminal 1 `target\release\nono.exe run --profile claude-code -- cmd /c "ping -n 30 127.0.0.1"`; Terminal 2 `target\release\nono.exe classify <child-pid>` → `structural match (non-authoritative)` + Package SID + the long NOTE. (`AI_AGENT` is NOT expected — second terminal's registry is empty; structural match is correct for Phase 73.)
5. **SC4 (authoritative):** `cargo test -p nono-cli --target x86_64-pc-windows-msvc -- --ignored sc4_classify_real_agent` and `... sc4_classify_spoof_not_agent` → both pass.
6. `grep -c "adopted" proj/DESIGN-engine-abstraction.md` ≥3; `grep -c "non-authoritative" ...` ≥2. (already ✓ statically)
7. Job SD hardening (Plan 02): `... launch::*::job_never_has_breakaway_ok` + `... job_security_descriptor_denies_low_il` → pass. (already ✓ statically)

Resume signal: `approved` once 1–5 confirm on a real host; else `issues: <description>`.

## Key files
- created: `crates/nono-cli/src/classify_runtime.rs`
- modified: `crates/nono-cli/src/execution_runtime.rs`, `cli.rs`, `app_runtime.rs`, `main.rs`, `cli_bootstrap.rs`, `exec_strategy_windows/launch.rs`, `proj/DESIGN-engine-abstraction.md`

## Commits
- `a021b10c` feat(73-03): wire mint->AgentRegistry insert into live launch path
- `7b3fa977` feat(73-03): ship `nono classify <pid>` structural marker verb
- `56fda2bd` feat(73-03): SC4 in-process integration tests + SC5 adopted-agent docs
