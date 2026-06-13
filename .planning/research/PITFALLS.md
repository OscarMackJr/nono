# Pitfalls Research

**Domain:** Persistent, multi-tenant, user-mode Windows agent-confinement daemon — launch-and-confine of arbitrary AI engines + a long-running multi-client capability pipe + `AI_AGENT` process marking (nono v2.12 "AI Agent Abstraction")
**Researched:** 2026-06-13
**Confidence:** HIGH (grounded in the in-tree `socket_windows.rs` / `exec_strategy_windows/launch.rs` code, the banked spike 001-003 findings, project memory, and verified Win32 job-object semantics)

> Scope note: this file covers ONLY the pitfalls that are *new with a persistent multi-tenant daemon*. The per-invocation traps already banked in the milestone context (post-hoc IL-drop is demote-only; user-owned workspace R-B3; exe/interpreter coverage gate; absolute grants; CLM on the hook path; broker dev-layout/signing R-B4; LangChain in-process `exec()`) are NOT re-derived here — they carry forward unchanged and are referenced where a daemon amplifies them.

---

## Critical Pitfalls

### Pitfall 1: Multi-tenant capability pipe with no server-side client authentication (cross-tenant capability theft)

**What goes wrong:**
The daemon serves N agents over one persistent multi-instance pipe (`PIPE_UNLIMITED_INSTANCES`). Agent A connects to a pipe instance and is served Agent B's capability grants — or escalates its own — because the daemon never proves *which tenant* is on the connection. The current single-tenant code (`socket_windows.rs`) verifies the **server** PID from the client side (`verify_connected_server_pid` / `GetNamedPipeServerProcessId`, line ~1840) but does the inverse — the **server authenticating the client** — only implicitly via the SDDL DACL. A single shared DACL that admits "any Low-IL same-session process" admits *every* tenant to *every* instance.

**Why it happens:**
The single-tenant supervisor pipe never needed to distinguish callers — there was one child. Generalizing to N tenants, the obvious move is "reuse the proven pipe, bump instance count to unlimited," which silently drops the one-child assumption that made the DACL sufficient. `CreateNamedPipeW` instances under one name are indistinguishable at the DACL level unless each instance is built with a *distinct* per-tenant security descriptor.

**How to avoid:**
- Bind the connection to a tenant identity the kernel vouches for, not a self-reported id in the JSON frame. On the server side call `ImpersonateNamedPipeClient` + `GetTokenInformation` to read the connected client's token (session SID / AppContainer package SID / job membership), then `RevertToSelf`. Match that against the tenant the grant belongs to *before* answering any capability request.
- Prefer **per-tenant pipe instances with per-tenant SDDL**: derive a distinct security descriptor per agent embedding that agent's session/package SID (the machinery already exists — `bind_low_integrity_with_session_and_package_sid`, per-SID DACL ACEs). One name, but each accepted instance carries only its own tenant's SID in the DACL.
- Treat any `agent_id` field in the wire frame as *untrusted routing hint only* — authorize against the impersonated token, never the field.
- Use `GetNamedPipeClientProcessId` and confirm the PID is inside that tenant's `AI_AGENT` job (`IsProcessInJob`).

**Warning signs:**
- The accept loop reuses one `CAPABILITY_PIPE_SDDL` constant for all tenants.
- Capability lookups key off a field parsed from the request body.
- No `ImpersonateNamedPipeClient` call anywhere in the daemon accept path.
- A test that connects as "tenant B" and successfully reads tenant A's grants is missing.

**Phase to address:**
Persistent multi-tenant daemon phase (the IPC generalization). This is the load-bearing security property of the whole daemon — gate the phase on a cross-tenant-denial test.

---

### Pitfall 2: `AI_AGENT` marker spoofing — a non-agent claims the marker, or an agent sheds it

**What goes wrong:**
The marker (named job object + SID) is used to decide "this process is a confined agent." Two failure directions:
- **Forge:** a non-agent process opens the well-known named job via `OpenJobObjectW("nono-ai-agent-<id>")` and `AssignProcessToJobObject`s itself in (or names its own job with the expected pattern) so it is *treated as* a marked agent and granted capabilities over the pipe.
- **Shed:** a confined agent escapes the marker so the daemon stops applying policy — e.g. it spawns a descendant that breaks out of the job (a child created with `CREATE_BREAKAWAY_FROM_JOB` when the job permits breakaway), or it relies on the marker being a mutable, self-reported attribute (env var, argv).

**Why it happens:**
A named job object is, by design, **openable by name** by any same-session process with the right access — the name is a rendezvous, not a secret. If marking == "is in a job whose name matches a pattern," the pattern is guessable and the job is openable. Env-var / argv markers are trivially forgeable. The "shed" direction happens because job membership is escapable unless breakaway is explicitly denied and the marker is set *at spawn under the daemon's control*, not adopted post-hoc.

**How to avoid:**
- The marker must be **established by the daemon at spawn-time** (launch-and-confine), bound to the confining token, not a label a process can self-apply. The authoritative identity is the **restricting/session SID and AppContainer package SID baked into the token** the daemon created — those cannot be forged by a same-user process (it cannot mint another principal's token).
- Do not name jobs with a guessable scheme as the *trust* signal. Use the named job for *kill-group / enumeration / resource caps*; use the token SID for *authorization*. (STACK.md already says: "Use a SID *in addition* (for WFP/pipe DACL scoping), not instead.")
- Deny breakaway: do NOT set `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` / `..._BREAKAWAY_OK` on the `AI_AGENT` job (the existing launch sets `KILL_ON_JOB_CLOSE | DIE_ON_UNHANDLED_EXCEPTION` — confirm breakaway is not also enabled).
- ACL the job object so only the daemon (SYSTEM/owner) gets `JOB_OBJECT_ALL_ACCESS`; agents get no `OpenJobObjectW` rights to their own or others' jobs.
- For *adopted* (not launched) agents, treat the marker as **best-effort/untrusted** — adoption-after-spawn cannot achieve launch-time soundness (post-hoc IL-drop is demote-only). Never grant capabilities purely on "found a process in an AI_AGENT-named job."

**Warning signs:**
- Authorization decisions read the job name or an env var.
- The job object's DACL allows the agent's own SID `OpenJobObjectW`.
- No test that a process *outside* the daemon's spawn path can/can't acquire the marker.
- Breakaway flags are set "to make subprocess management easier."

**Phase to address:**
Per-agent `AI_AGENT` marker phase (inside the daemon work). Pair the marker with the launch-and-confine phase so the marker is always a spawn-time property.

---

### Pitfall 3: Trying to launch-and-confine an in-process-`exec()` engine that is already running (structural impossibility)

**What goes wrong:**
For LangChain's `PythonREPLTool` (and any in-process `exec()` tool), the risky operation runs **inside the engine process** — there is no child process to wrap. The sound model (FEATURES.md) is that nono **parents the python interpreter itself**, so the whole interpreter is sandboxed and the in-process `exec()` is confined transitively. The pitfall is the daemon trying to "confine the tool call" of an engine *it did not launch* — there is structurally nothing to confine: the daemon can only post-hoc IL-drop the running interpreter, which is leaky/unsound (handles opened before the drop survive; no restricting SID retrofit; network uncovered; per `windows-confinement-model.md`).

**Why it happens:**
The Claude PreToolUse mental model ("intercept each tool call → `nono run`") is sticky. It works because Claude's *shell* tool spawns a child. It does **not** generalize to in-process `exec()` — there is no spawn to intercept. Teams reach for the daemon's "adopt a running agent" path, hit post-hoc IL-drop, and ship an unsound boundary believing it equivalent to launch-time.

**How to avoid:**
- The daemon must be the **parent** of the interpreter for in-process engines. Confinement is applied to `python.exe` at spawn, before any `exec()` runs. Confined-because-the-whole-process-is-sandboxed is the only sound story.
- Where the daemon cannot be the parent (engine already running, IDE-embedded), be explicit that this is **demote-only** incident response, not confinement — and the `nono-py` binding's answer is to apply confinement **from inside the process at startup** (the binding's `confined_run` / in-process sandbox-self), before the agent does any work.
- Document the hard limit: an engine that does in-process risky ops and is launched by a third party cannot be soundly confined by an external daemon. This is a contract on E2 ("a launch command nono can own").

**Warning signs:**
- A code path that "confines LangChain" by finding a running `python.exe` and dropping its IL.
- The `nono-py` binding offering only an external-launch API and no in-process sandbox-self entry point.
- Tests that prove confinement only via a subprocess `ShellTool`, never via `PythonREPLTool` `exec()`.

**Phase to address:**
Engine abstraction boundary + `nono-py` binding phase. The binding must demonstrate in-process confinement (sandbox-self at startup), not just external launch.

---

### Pitfall 4: Persistent-daemon attack surface — a privileged, always-on target that an escaped agent can pivot through

**What goes wrong:**
The per-invocation `nono run` model is ephemeral: the supervisor exists only for one child and dies with it. A persistent daemon is a long-lived process with elevated reach (it launches confined processes, manipulates tokens, owns job objects, talks to the elevated `nono-wfp-service`, holds open handles to every tenant). If the daemon runs as SYSTEM/admin, an agent that escapes confinement (or a malicious capability request) can pivot through the daemon to **all tenants** and to **the host** — a far worse blast radius than escaping one ephemeral supervisor. The daemon is now persistent state that survives between agents, so a compromise persists too.

**Why it happens:**
The existing in-tree service pattern (`nono-wfp-service`) runs as **LocalSystem** because WFP filter installation requires it. Cloning that pattern for the agent daemon ("it's just a second instance of a service we already ship") inherits SYSTEM privilege the *launcher* role does not need. The daemon conflates two roles: the unprivileged launcher/IPC role and the privileged network-enforcement role.

**How to avoid:**
- **Least-privilege split.** Run the agent-launcher daemon at the *user's* privilege (it launches same-user confined children; it does not need SYSTEM). Keep WFP filter manipulation in the existing separate elevated `nono-wfp-service` behind its own narrow control-pipe protocol. Do not merge the launcher into the elevated service.
- Harden the daemon's own control surface like the capability pipe: bounded (64 KiB) length-prefixed frames (already the pattern), strict deserialization, reject oversized/malformed, per-client authorization (Pitfall 1).
- The daemon must never expand a running agent's capabilities (no escape hatch — core nono invariant). Capability *grants* are decided at launch; the pipe answers queries, it does not widen the sandbox.
- Isolate tenants from each other in the daemon's own memory/state: a compromised request handler for tenant A must not be able to read tenant B's grants, handles, or tokens. Avoid one global handle table indexed by client-supplied id.
- Treat every capability request as hostile input — it may originate from a prompt-injected agent.

**Warning signs:**
- The daemon binary registered as a `start=auto` SYSTEM SCM service like `nono-wfp-service`.
- The launcher and WFP-control responsibilities in one process.
- Any code path that, on request, calls a "grant more access to PID N" function.
- Shared mutable global state keyed by an untrusted tenant id.

**Phase to address:**
Persistent multi-tenant daemon phase — specifically the daemon-hosting/privilege-model sub-task. Write the privilege model down (ADR) before coding the service host.

---

### Pitfall 5: Token & job-object handle lifetime in a long-running process (leaks, reuse, orphans)

**What goes wrong:**
The per-invocation model leaks nothing of consequence — the process dies and the kernel reclaims everything. A daemon that lives for days accumulates:
- **Handle leaks:** every launched agent allocates a job handle, a token, pipe instances, process/thread handles. The existing `Drop` impls (`launch.rs` `CloseHandle(self.job)` etc.) assume a short-lived owner. If the daemon stores per-agent state in a map and an agent exits without the entry being cleaned, handles leak until the daemon exhausts the desktop heap / handle table.
- **Token reuse across agents:** reusing one confining token (or job) for multiple agents collapses tenant isolation — agent B inherits agent A's restricting SID / workspace relabel, so B can write A's workspace, and WFP scoping (per-package-SID) blurs.
- **Orphaned jobs:** `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` is set (`launch.rs:222`) and the daemon holds the only job handle, so the job's processes die when the daemon closes the handle — good for teardown but a footgun if the daemon restarts/crashes (all agents die) or if a handle is closed early. Conversely, if the daemon drops a job-handle reference but the agent is still running and no one holds `KILL_ON_JOB_CLOSE`, the job becomes an unreferenced orphan with no kill-group.

**Why it happens:**
Lifetime correctness that is *automatic* for an ephemeral process becomes *manual bookkeeping* in a daemon. The proven code was written for one child; reuse "to avoid re-allocating tokens" is a tempting micro-optimization that destroys isolation.

**How to avoid:**
- **One fresh confining token + one fresh job per agent.** Never reuse a token or job across tenants. Mint per-agent (the SID nonce machinery — `getrandom` `create_nonce_hex` / `generate_session_sid` — already exists for this).
- Tie every per-agent resource (token, job, pipe instances, process handles) to a single owning struct with a `Drop` that closes all of them, and remove it from the registry on agent exit. Reap exited agents promptly: wait on the job's completion port / process handle and clean up deterministically, not lazily.
- Decide the crash-teardown policy deliberately: `KILL_ON_JOB_CLOSE` means a daemon crash kills all agents (fail-secure, arguably correct). If you want agents to survive a daemon restart, you need a different ownership model (re-`OpenJobObjectW` on restart) — but then you must re-establish trust on adoption (Pitfall 2).
- Audit for handle growth: the daemon should expose/log a live count of open jobs/tokens/pipe instances; a monotonically rising count under steady tenant churn is a leak.

**Warning signs:**
- A "token pool" or "reuse the job if the workspace matches" optimization.
- Per-agent state stored in a map with no removal-on-exit path.
- Handle count rising over hours of use with stable concurrent-agent count.
- No integration test that launches→exits 100 agents and asserts handle count returns to baseline.

**Phase to address:**
Persistent multi-tenant daemon phase (token/job *reuse* across agents was the explicitly unspiked part of spike 003/004). Make per-agent fresh-token + deterministic-reap a success criterion.

---

### Pitfall 6: Nested-job collisions and silent confinement loss (Windows 8+ job hierarchy semantics)

**What goes wrong:**
On Windows 8+ a process can belong to a *hierarchy* of nested jobs, but with hard constraints: `AssignProcessToJobObject` on an already-jobbed process succeeds only if the target job is empty or in the existing hierarchy, **and the target job has no UI limits and no conflicting limits**. Two daemon-specific failures:
- The agent engine (e.g. a `node`/`python` launched from a parent that *already* placed it in a job — common under some shells, terminals, or CI runners) is already in a job; the daemon's `AssignProcessToJobObject` into the `AI_AGENT` job fails or only nests, and resource/kill semantics don't behave as the daemon assumes.
- The daemon nests its `AI_AGENT` job under an outer job whose limits silently override or conflict, so the confinement/resource caps the daemon thinks it set don't actually apply.

**Why it happens:**
The single-`nono run` path controlled the full spawn and rarely hit pre-existing jobs. A daemon launching arbitrary engines from arbitrary contexts (and adopting running ones) routinely meets already-jobbed processes. The Win32 nested-job rules are subtle and the failure is often silent (limits don't apply) rather than a hard error.

**How to avoid:**
- Spawn the engine **suspended and assign it to the `AI_AGENT` job before it runs any code** (the launch path already creates suspended; `launch.rs:2005` terminates on assign failure — keep that fail-secure stance). This guarantees the job is established first.
- On `AssignProcessToJobObject` failure, **fail secure** — terminate the suspended process, never let an unconfined agent run (the existing `terminate_suspended_process` on assign failure is the correct pattern; ensure the daemon path keeps it).
- Detect pre-existing job membership (`IsProcessInJob` against the default/NULL job) for adopt-mode; if the process is already in a foreign job with conflicting limits, refuse to claim it as confined.
- Do not set UI limits on the `AI_AGENT` job (UI limits block nesting).

**Warning signs:**
- `AssignProcessToJobObject` return value ignored or only logged.
- Resource caps (`JOB_OBJECT_LIMIT_JOB_MEMORY`, active-process limit — already wired in `launch.rs`) not actually observed in testing.
- Launching engines from terminals/CI without testing the already-jobbed case.

**Phase to address:**
Generic launch-and-confine (productionize) phase, hardened in the daemon phase for the adopt path.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Reuse one capability-pipe SDDL for all tenants | Less code; reuse proven constant | Cross-tenant capability theft (Pitfall 1) | Never for multi-tenant |
| Reuse one confining token/job across agents | Avoid per-agent token mint cost | Tenant isolation collapse; WFP scope blur (Pitfall 5) | Never |
| Run the daemon as LocalSystem like `nono-wfp-service` | Mirror the in-tree service pattern; one binary | Huge blast radius on escape (Pitfall 4) | Only the WFP-control role; never the launcher role |
| Mark agents with an env var / argv flag | Trivial to set/read | Forgeable marker (Pitfall 2) | Never as a trust signal; OK as a *hint* alongside token SID |
| Lazy cleanup of exited-agent state | Simpler accept loop | Handle leak over days (Pitfall 5) | Never in a long-running daemon |
| Post-hoc IL-drop to "confine" an adopted agent | Confine things you didn't launch | Unsound boundary (handles survive, no restricting SID, no network) | Only as a demote/IR lever, explicitly labeled |
| Self-reported `agent_id` in the wire frame for authz | Simple routing | Spoofable tenant identity (Pitfall 1) | As routing hint only, never for authorization |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Multi-instance named pipe (`PIPE_UNLIMITED_INSTANCES`) | Assume DACL alone isolates tenants | `ImpersonateNamedPipeClient` + per-tenant SID match before serving; per-tenant SDDL |
| `nono-wfp-service` (elevated) | Merge launcher into the elevated service | Keep launcher at user privilege; call the WFP service over its existing narrow control pipe |
| AppContainer per-agent SID | `DeriveAppContainerSidFromAppContainerName` only | Must also `CreateAppContainerProfile` (memory `windows_appcontainer_wfp_validated`; derive-only → `CreateProcessW ERROR_FILE_NOT_FOUND`) |
| Spawning under the broker arm | Run from `Program Files` install | Needs dev-layout or signed `nono.exe` (R-B4 broker trust gate) |
| Engine launched from a parent that pre-jobs it | Assume `AssignProcessToJobObject` always works | Spawn suspended, assign first, fail-secure on assign failure; handle nested-job rules (Pitfall 6) |
| `env_clear()` before spawning CLR/PowerShell engines | Strip all env for hygiene | Preserve `SystemRoot`/`windir`/`SystemDrive` baseline (memory `windows_hook_interpreter_spawn_gotchas`; else CLR `0xFFFF0000`) |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| One thread per tenant (sync `PeekNamedPipe` poll) generalized to N | Thread-count blowup; scheduler thrash | Use `tokio::net::windows::named_pipe` task-per-connection for the daemon (STACK.md) | Tens+ of concurrent agents |
| Handle table growth from un-reaped agents | Daemon `OpenProcess`/`CreateFile` start failing after hours | Deterministic per-agent `Drop` + reap on exit (Pitfall 5) | Long uptime under churn |
| Unbounded capability-request queue per client | Memory growth; one chatty agent starves others | Bounded per-tenant frames (64 KiB cap exists) + per-tenant fairness/timeout | A misbehaving/looping agent |
| Re-deriving AppContainer profile per request | Latency spike per launch | Derive+register once per agent, cache the SID | Many short-lived launches |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Authorize capability requests by client-supplied id | Cross-tenant capability theft | Authorize by impersonated client token / job membership (Pitfall 1) |
| Named job is the trust signal | Marker forgery by any same-session process opening the job by name | Trust the token SID, not the job name; ACL the job to daemon-only (Pitfall 2) |
| Daemon runs as SYSTEM | Escaped agent pivots to all tenants + host | Least-privilege launcher; isolate WFP-control in its own elevated service (Pitfall 4) |
| Job allows breakaway | Agent child escapes confinement/marker | Do not set breakaway flags; verify children stay in-job |
| Post-hoc IL-drop treated as confinement | Leaky boundary: pre-drop handles survive, no restricting SID, network uncovered | Launch-time confinement only; IL-drop = demote-only |
| Capability pipe answers "expand access" requests | Defeats the no-escape-hatch invariant | Grants fixed at launch; pipe is query-only |
| Trusting an adopted (not launched) process as confined | Adoption can't achieve launch-time soundness | Adopt = best-effort demote; never grant on adoption alone |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Silent confined-write failure (admin-owned workspace) | "Why didn't my agent write the file?" — fails secure but opaque | Clear diagnostic naming R-B3 + `takeown`/non-elevated-create guidance |
| Engine refused for uncovered exe/interpreter with a cryptic error | User can't tell which binary to `--allow` | Diagnostic naming the exact uncovered exe/interpreter path |
| Relative-path grant silently resolves to `C:\` and is denied | Confusing deny on a path the user "did grant" | Reject/normalize relative grants; require absolute (banked contract) |
| Daemon crash kills all agents (KILL_ON_JOB_CLOSE) with no warning | All running agents vanish | Document the teardown policy; surface daemon health; consider supervised restart |
| Cursor-on-Windows appears supported then fails | User wastes time; thinks nono is broken | Per-engine fit doc: Cursor CLI is WSL-only (FEATURES.md) |

## "Looks Done But Isn't" Checklist

- [ ] **Multi-tenant pipe:** Often missing *server-side* client authentication — verify a "connect as tenant B, read tenant A's grants" test fails closed.
- [ ] **AI_AGENT marker:** Often missing forge/shed resistance — verify a non-daemon-spawned process cannot acquire the marker and a child cannot break away from the job.
- [ ] **Per-agent isolation:** Often missing fresh-token-per-agent — verify two concurrent agents cannot write each other's relabeled workspace and have distinct WFP package SIDs.
- [ ] **Handle lifetime:** Often missing reap-on-exit — verify launch+exit of 100 agents returns handle/job count to baseline.
- [ ] **Daemon privilege:** Often missing least-privilege split — verify the launcher daemon is NOT SYSTEM and the WFP role is separate.
- [ ] **In-process engine:** Often missing the sandbox-self path — verify LangChain `PythonREPLTool` `exec()` (not just `ShellTool`) is confined via the binding.
- [ ] **Assign-to-job failure:** Often missing fail-secure — verify a process that can't be assigned to its `AI_AGENT` job is terminated, never run unconfined.
- [ ] **Adopt mode:** Often mislabeled as confinement — verify adoption is documented/coded as demote-only, not a capability grant.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Cross-tenant capability theft shipped | HIGH | Add server-side impersonation/token-match; rev the wire protocol; revoke trust in client-supplied id; audit any grants served |
| Token/job reuse across agents | MEDIUM | Refactor to per-agent fresh token+job; add isolation test; no data migration needed |
| Daemon shipped as SYSTEM | MEDIUM | Re-register launcher at user privilege; move WFP calls behind the existing elevated service; re-test |
| Marker forgeable | MEDIUM | Switch authz to token SID; ACL the job daemon-only; add forge test |
| Handle leak in production | LOW-MEDIUM | Add per-agent owning struct + reap-on-exit; restart daemon to reclaim; add growth assertion test |
| "Confined" an in-process engine post-hoc | HIGH | Re-architect to parent the interpreter (or in-process sandbox-self via binding); relabel adoption as demote-only |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1. Cross-tenant capability theft | Persistent multi-tenant daemon (IPC) | Negative test: tenant B denied tenant A's grants; impersonation in accept path |
| 2. AI_AGENT marker forge/shed | Daemon — marker sub-task | Negative tests: non-spawned process can't get marker; child can't break away; job ACL'd daemon-only |
| 3. In-process-exec engine confinement | Engine abstraction + `nono-py` binding | LangChain `PythonREPLTool` `exec()` confined via sandbox-self at startup |
| 4. Daemon attack surface / privilege | Daemon — service-host/privilege model (ADR) | Launcher runs non-SYSTEM; WFP role separate; bounded/authorized control surface |
| 5. Token/job lifetime, reuse, orphans | Daemon — token/job reuse (was unspiked) | Fresh token+job per agent; 100-agent reap returns to baseline handle count |
| 6. Nested-job collisions / silent loss | Generic launch-and-confine (productionize) + daemon adopt path | Already-jobbed engine handled; assign-failure fails secure; resource caps observed |

## Sources

- In-tree code (HIGH — authoritative, current):
  - `crates/nono/src/supervisor/socket_windows.rs` — SDDL pipe scoping, per-session/package SID ACEs, `PIPE_UNLIMITED_INSTANCES`, `verify_connected_server_pid` (server-PID verify, line ~1840), 64 KiB framing, no server-side `ImpersonateNamedPipeClient` (the multi-tenant gap).
  - `crates/nono-cli/src/exec_strategy_windows/launch.rs` — job-object lifecycle: `CreateJobObjectW`, `AssignProcessToJobObject`, `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | DIE_ON_UNHANDLED_EXCEPTION` (line ~222), suspended-spawn + `terminate_suspended_process` on assign failure (line ~2005), resource-cap flags.
- Banked spike findings (HIGH):
  - `.claude/skills/spike-findings-nono/references/windows-confinement-model.md` (spikes 001 INVALIDATED / 002 PARTIAL — post-hoc IL-drop leaky).
  - `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` (spike 003 VALIDATED; exe-coverage, absolute grants, R-B3, R-B4; the unspiked multi-tenant marker/token-reuse parts).
- Sibling research (HIGH): `.planning/research/STACK.md`, `FEATURES.md`, `ARCHITECTURE.md` (v2.12).
- Project memory (HIGH): `windows_appcontainer_wfp_validated`, `windows_hook_interpreter_spawn_gotchas`, `feedback_windows_supervised_needs_real_console`, `windows_appcontainer_cap_pipe_reachability`.
- Verified Win32 semantics (MEDIUM-HIGH): [AssignProcessToJobObject (jobapi2.h)](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject) — Windows 8+ nested-job rules (already-jobbed process: target must be empty/in-hierarchy, no UI limits); [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects); [Why is my process in a Job if I didn't put it there?](https://learn.microsoft.com/en-us/archive/blogs/alejacma/why-is-my-process-in-a-job-if-i-didnt-put-it-there).

---
*Pitfalls research for: persistent multi-tenant Windows agent-confinement daemon (nono v2.12)*
*Researched: 2026-06-13*
