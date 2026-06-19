# 79-01 SUMMARY — WFP-01 per-SID egress isolation gate

**Status:** COMPLETE (checkpoint PASS via dark-factory gate)
**Requirement:** WFP-01
**Branch:** milestone/v2.13-carryforward-closeout

## What was delivered

1. **Three policy.json profiles** (`crates/nono-cli/data/policy.json`, commit `507ff683`):
   `nono-ts-wfp-test-open` (network.block:false), `nono-ts-wfp-test-blocked` (network.block:true — first and only `block:true` profile), `nono-ts-default` (TSRG-01, consumed by Plan 79-02). `cargo test -p nono-cli` green (only the 4 documented pre-existing baseline failures; zero new).

2. **WFP-01 dark-factory gate** `scripts/gates/wfp-egress-isolation.ps1` — rewritten as a **daemon-path structural proof** (initial egress-probe version `3357d968` + cwd-fix `042b988e`, superseded by rewrite `0624256d`). Exports `Test-Precondition`/`Invoke-Gate`, no bare `exit`, returns the locked `[ordered]@{gate;verdict;reason;detail;timestamp}` shape.

3. **Live verdict:** `pwsh scripts/verify-dark.ps1 --gate wfp-egress-isolation` → **PASS** (runner exit 0), `blockedHasFilter=true`, `openHasFilter=false`, distinct AppContainer package SIDs. SC1 satisfied.

## MAJOR DEVIATION — the plan's OQ-1 was empirically falsified

The plan assumed (OQ-1) that a direct `nono run --profile <block:true>` exercises WFP per-SID filters via `prepare_network_enforcement → install_wfp_network_backend`. **Live Win11 testing disproved this**, in layers:

- **Confined AppContainers have ZERO network capabilities.** The broker creates every lowbox child with `SECURITY_CAPABILITIES{ CapabilityCount: 0 }` (nono-shell-broker/src/main.rs ~420). A confined agent has no `internetClient` and no loopback exemption, so it cannot egress to ANY target: external IP → `curl (7)` instant reject; loopback mock → `curl (28)` silent drop. `network:{block:false}` does NOT grant egress — it only controls WFP filter installation. So an "allowed" confined agent never egresses, and **no egress probe (loopback or external) can distinguish allowed vs blocked.**
- **Direct `nono run` with these (windows_low_il_broker) profiles installs NO WFP filter** — `select_network_backend` returns None (no debug "selecting backend" line; `netsh wfp show filters` shows 0 nono filters); the "net outbound blocked" banner is the zero-cap AppContainer, not WFP.
- **Per-SID WFP egress isolation is a DAEMON-path feature** — `agent_daemon/launch.rs::wfp_filter_add` (gated by `profile_needs_network_scoping` reading `policy.profiles[name].network.block`) installs a per-package-SID `FWP_ACTION_BLOCK` filter (`FWPM_CONDITION_ALE_USER_ID`, 4 ALE layers) via nono-wfp-service. This is the Phase-74 multi-tenant path.

**Operator decisions during the checkpoint** (3 AskUserQuestion rounds, each after a deeper invalidation): (1) reshape gate → external egress → then proven infeasible (zero caps) → **daemon-path structural proof**; (2) filter leak → **log as follow-up + clean host now**.

## Final gate design (daemon-path structural proof)

Launch a blocked and an allowed agent through `nono agent launch --profile <p> -- cmd /c "for /L %i in (1,1,60000000) do @rem"` (CPU busy-loop keep-alive — a zero-cap agent can't run ping/curl and self-exits instantly, reaping the filter before slow `netsh` can see it). Parse each `sid=S-1-15-2-...` from the launch response, then `netsh wfp show filters` → assert a nono block filter exists for the blocked SID and none for the allowed SID (baseline-delta so leaked filters don't affect the verdict). Test-Precondition SKIPs on missing elevation / nono-wfp-service / nono-agentd; the gate reclassifies elevated-daemon ("workspace not owned") and wfp-service-down launch failures as SKIP, not FAIL.

## Host setup the gate requires (also durable in memory `wfp_confined_egress_and_daemon_gate`)

- **Elevation** — `netsh wfp show filters` needs admin.
- **Fresh nono-wfp-service** — the installed `C:\Program Files\nono\nono-wfp-service.exe` (LocalSystem) was stale and rejected `activate_blocked_mode` ("unsupported request kind"); redeployed the fresh build + restarted.
- **Non-elevated nono-agentd** — an elevated daemon creates `C:\Users\<u>\nono-agents\<id>` owned by BUILTIN\Administrators → `DaemonDaclGuard` rejects "workspace not owned by current user". nono-agentd is a TYPE 50 USER_OWN_PROCESS TEMPLATE; `sc start` fails 1058. Started non-elevated via `runas /trustlevel:0x20000 "<...>\nono-agentd.exe --foreground"`.
- Both nono.exe and nono-agentd.exe embed policy.json — rebuilt both after the profile add.

## Follow-ups recorded (out of scope for this closeout)

- **WFP filter leak** (tracked follow-up, per operator decision): daemon-path per-SID block filters accumulate and survive a nono-wfp-service restart (not dynamic-session-scoped). Found 32 leaked filters (8 dead SIDs × 4 layers) from prior testing; **host cleaned** this session via `FwpmFilterDeleteById0`. Future fix: ensure filters are removed on agent reap / use dynamic-session scoping.
- The original `nono-ts-wfp-test-open/blocked` profiles' `filesystem.allow: C:\Windows\System32` + `workdir.access:none` shape is retained but is now only used by the daemon-launched agents (System32 coverage for cmd.exe; the busy-loop needs no egress).

## Cross-target clippy

PARTIAL — no Rust changed in this plan (policy.json is data; gate is PowerShell). Workspace clippy unaffected. Per `.planning/templates/cross-target-verify-checklist.md`, no cfg-gated code touched.

## Commits (C:\Users\OMack\Nono)

- `507ff683` — three policy.json profiles
- `3357d968` — initial gate (egress-probe) [superseded]
- `042b988e` — gap-closure cwd-coverage (--workspace) [superseded]
- `0624256d` — gate rewrite: daemon-path structural proof (final)
