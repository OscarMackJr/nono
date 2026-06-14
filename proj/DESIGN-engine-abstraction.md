# Engine-Abstraction Contract

This document defines the stable boundary that every engine and language binding must satisfy for
nono to mediate its execution. The contract is authored here — in the main nono repo, alongside
`DESIGN-library.md` and `DESIGN-supervisor.md` — because it is binding-neutral: `nono-py` (Phase 72),
`nono-ts` (Phase 75), and the future multi-tenant daemon (Phase 74) all implement the same five
exposure points. The E5 slot is the documented integration point for the `zt-infra.org` fail-closed
agent security control plane (see "Forward-Compat: zt-infra.org Integration" below); the full
integration is a future phase, but this contract ensures nono does not paint itself into a corner.

**Version:** 1.0 (2026-06-14)
**Source:** `.planning/research/ARCHITECTURE.md` §"The Abstraction Boundary Contract" (banked,
non-negotiable invariants). Spike 003 (VALIDATED) provides empirical grounding for E1, E2, and E3.


## What Every Engine Must Expose

Every engine that nono mediates must expose all five of the following. E1–E4 are the
launch-and-confine contract. E5 is the fallback for engines nono cannot parent.

| #   | Exposed thing                                              | How nono consumes it                                                                                               | Required for                           | Owner              |
| --- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | -------------------------------------- | ------------------ |
| E1  | Engine **executable and interpreter path(s)**              | Launch profile supplies `--allow <exe-dir>` for each binary (`python.exe`, `node.exe`, engine binary); fail-secure refuse if any binary is uncovered | Confinement soundness                  | Launch profile     |
| E2  | An **ownable launch command** (argv + env)                 | Daemon or CLI is the parent; if a third party owns the spawn (IDE-embedded engine), fall back to E5 or adopt with weaker guarantees | Process boundary                        | Launch path        |
| E3  | Intended **writable workspace as an ABSOLUTE path**        | Granted and relabeled Low-writable; engines must not assume CWD inheritance                                        | Path-confinement soundness             | Launch profile + caller |
| E4  | A **network identity** (AppContainer package SID, broker no-PTY arm) | Per-agent WFP scoping; daemon assigns one SID per tenant                                              | Network confinement                    | Marker / launch path |
| E5  | *(hook-camp only)* A **pre-execution interception point**  | Only when nono cannot be the parent. Claude PreToolUse (shipped); Copilot JSON-RPC hooks; Cursor permission gate; zt-infra.org `POST /actions` (FUTURE) | Defense-in-depth for non-parent engines | Engine vendor / control plane |


## E1 — Executable and Interpreter Paths

The launch profile must enumerate every binary the engine invokes — `python.exe`, `node.exe`, the
engine's own binary, any shim or wrapper script — via `--allow <exe-dir>` entries. nono's
interpreter-coverage gate is evaluated at spawn time against this list.

**Invariants:**

- The exe/interpreter coverage gate is **fail-secure**: an uncovered binary is refused at spawn
  time. There is no degraded-confinement mode; the child process is never created.
- All paths must be **absolute**. The PowerShell-to-`C:\` relative-write trap (spike 003 finding)
  proves that engines do not uniformly inherit the launcher CWD; a relative interpreter path may
  resolve to an unexpected location.
- The gate fires for the **directly-confined command**, not just the outer wrapper. If `python.exe`
  is the confined command, the grant must cover `python.exe`'s directory, not just the engine
  launcher's directory.
- Adding `--allow <interpreter-dir>` **in addition to** `--allow <workspace>` is always required
  when the interpreter lives outside the workspace (the common case).


## E2 — Ownable Launch Command

nono (or the daemon in Phase 74) must be the **parent process** — it owns the spawn, not the IDE,
a third-party orchestrator, or an already-running process. Owning the spawn is what gives nono the
process handle needed to assign the job object, apply the token, and enforce WFP scoping at
creation time.

**Invariants:**

- If a third party owns the spawn (IDE-embedded engine, agent already running), fall back to E5
  (pre-exec interception) or adopt the agent with the explicitly documented weaker guarantees
  (Phase 73). Never claim launch-and-confine guarantees for an adoption path.
- The parent must be the one to call `CreateProcess` / `fork+exec` — not a transitive grandparent
  whose handle nono cannot observe.
- argv and env must be under nono's control at spawn time so that the exe-coverage gate can
  evaluate the full command before any process is created.


## E3 — Absolute Writable Workspace

The caller must supply the intended writable workspace as an **absolute path**. nono grants that
path for read/write and — on Windows — relabels it Low-writable so the confined agent can write to
it but cannot escalate to higher-IL paths.

**Invariants:**

- All workspace grants are **absolute paths**. Relative paths are rejected; engines do not reliably
  inherit the launcher CWD (PowerShell resolved a relative write to `C:\` in spike 003).
- The workspace must be **user-owned**, not elevated or admin-owned. An admin-owned directory
  defeats the DACL/label grant (`WRITE_DAC` is not granted to non-owners; R-B3). nono emits a
  clear diagnostic and refuses if the ownership check fails.
- Confinement guarantees: `NO_WRITE_UP` to paths outside the grant; deny-network-unless-granted.
  These are the per-invocation `nono run` guarantees and they apply to every engine the abstraction
  covers.
- The child cwd must be set to a covered workspace path. An uncovered cwd allows implicit
  relative-path escapes (D-52-01 cwd-coverage rule, observed in spike 003 harness debugging).


## E4 — Network Identity

Each confined agent is assigned a **per-agent network identity** established at spawn time. On
Windows, this is an AppContainer package SID (broker no-PTY arm); it is the principal WFP uses to
scope per-agent egress rules.

**Invariants:**

- The identity is established **at spawn time**, not mid-run. Post-hoc identity assignment is
  unsound for the same reason post-hoc IL-drop is leaky: handles and sockets opened before
  assignment operate at the pre-assignment privilege level.
- The AppContainer profile must be created via `CreateAppContainerProfile` before use. A
  derived-only SID (no registered profile) causes `CreateProcessW ERROR_FILE_NOT_FOUND` — a
  banked gotcha from Phase 62.
- The package SID needs explicit read/traverse grants on the workspace path; it is a different
  principal from the user SID.
- Phase 74 (daemon) allocates one SID per tenant; the WFP egress rules are scoped to that SID.
  Cross-tenant traffic blocking is a correctness requirement, not optional.


## E5 — Pre-Exec Interception Point (Optional)

E5 applies **only** when nono cannot be the parent process — IDE-embedded engines, already-running
processes, or runtimes where the process boundary is not under nono's control. It is the
defense-in-depth fallback, not the primary confinement boundary.

**When E5 applies:**

- IDE-embedded engines (VS Code extension host, Copilot Workspace) where the IDE owns the spawn.
- Engines invoked programmatically from an already-running process that nono did not launch.
- Any runtime where E2 (ownable launch command) cannot be satisfied.

**Built consumers (shipped):**

- **Claude PreToolUse hook** — intercepts each tool call before execution; shipped in Phase 60.
  The hook calls `nono run` per tool, providing per-call confinement where process-level
  confinement is not available.
- **Copilot JSON-RPC hooks** — Phase 75 (planned).
- **Cursor permission gate** — conditional on Cursor exposing a hook point.

**Invariants:**

- E5 is a complement to, not a substitute for, E1–E4 confinement. Even when an E5 hook fires,
  nono's OS-level enforcement (Low-IL label, AppContainer, WFP) applies underneath the hook
  decision if the engine process was launched via nono.
- Deny at E5 means **skip exec, fail-closed** — the tool call does not proceed. There is no
  partial-execution mode.
- E5 hooks must not be the sole confinement mechanism for high-risk engines. Use them for
  defense-in-depth; prefer E2 (parent-owned spawn) where achievable.

**Forward-compat note:** The E5 slot is the integration point for the `zt-infra.org` control
plane. See the next section.


## Forward-Compat: zt-infra.org Integration

This section documents how the E5 interception slot maps to the `zt-infra.org` fail-closed agent
security control plane. **The integration itself is a FUTURE phase.** This design ensures nono
does not paint itself into a corner when that integration is built.

**zt-infra.org role:** fail-closed agent security control plane; provides `POST /actions`
allow/deny decisions with a cryptographic audit trail; composes with nono as the execution sandbox;
supports LangGraph, LangChain, and MCP over HTTP; runs on Linux-container / cloud runtime.
See: [https://www.zt-infra.org/](https://www.zt-infra.org/)

**Mapping: E5 pre-exec interception → zt-infra `POST /actions`**

```
agent submits tool-call intent to zt-infra control plane via POST /actions
    ↓
control plane evaluates policy + records cryptographic audit trail
    ↓
decision: allow / deny
    ↓
deny  → skip exec, fail-closed
         nono's OS confinement still enforces the process-level boundary
allow → exec proceeds; nono enforces OS confinement UNDERNEATH the policy decision
```

**nono's role:** enforce OS confinement regardless of the E5 allow/deny decision. nono is the
execution sandbox **under** the control plane, not a replacement for it. A zt-infra `allow`
decision does not bypass nono's Low-IL label, AppContainer, or WFP enforcement; it only permits
the specific tool call to proceed to the OS boundary. A zt-infra `deny` decision short-circuits
before the OS boundary is reached.

**Implementation note for the future phase:** The E5 hook slot (currently Claude PreToolUse) must
be generalizable to an HTTP round-trip to `POST /actions`. nono's role does not change: it
supplies OS confinement beneath whatever the control-plane decides. No HTTP client or adapter is
built in this phase — the contract is documented here for design continuity.


## Implementation Notes per Platform

### Windows (Phase 72+)

Windows has no equivalent to Linux Landlock or macOS Seatbelt for in-process self-confinement.
`Sandbox::apply()` in `crates/nono/src/sandbox/windows.rs` validates the capability set and
returns; it does **not** apply OS restrictions to the running process. This is preview/advisory
behavior by design (D-01, D-02). All Windows confinement routes through the broker.

**Shape A — confined_run (spawn-confined, preferred default):**

`nono-py confined_run(exe, args, allow, profile)` spawns `nono.exe run --profile <profile>
--allow <path>... -- <exe> <args>` via `std::process::Command`. The entire engine process runs
confined from the moment it starts. This is the preferred shape for LangChain `PythonREPLTool`
because the in-process `exec()` is confined transitively — the process is confined, so every
in-process operation inherits the confinement.

```
nono-py confined_run(exe='python.exe', args=[...], allow=[ws], profile='langchain-python')
    ↓
subprocess: nono.exe run --profile langchain-python --allow <ws> --allow <python-dir> -- python.exe ...
    ↓
[exe-coverage gate] → fail-secure if uncovered
    ↓
broker arm: Low-IL primary token + AppContainer package SID
    ↓
confined python process (all in-process exec() + file I/O OS-enforced)
```

**Shape B — confine (born-confined self-re-exec):**

`nono-py confine(profile, allow, caps)` re-execs the **current** python process through `nono.exe`
as the **first operation in `main()`**, before any privileged handle is opened. The agent process
is born confined at its own entrypoint.

**Ordering is the invariant.** `confine()` MUST be called before any file handle, socket, registry
key, job object, or other privileged object is acquired. A mid-run call is unsound for the same
reason the post-hoc IL-drop is leaky (spike 002, banked): handles opened before confinement
operate at pre-confinement privilege and are not revoked.

The `NONO_ALREADY_CONFINED=1` environment variable prevents infinite re-exec. The `confine()`
implementation checks this variable **as its first operation**: if set, it returns immediately
(the process is already confined). If not set, it spawns `nono.exe run` with
`NONO_ALREADY_CONFINED=1` in the child environment, waits for the child to exit, and calls
`std::process::exit(child_code)`.

```
main() {
    confine(profile='langchain-python', allow=[ws])  # MUST be first — born-confined re-exec
    # all code below runs inside the confined process
    agent = LangChainAgent(tools=[PythonREPLTool()])
    agent.run(...)  # in-process exec() is OS-enforced
}
```

**AppContainer gotcha (banked, Phase 62):** The package SID must be registered via
`CreateAppContainerProfile` before any launch. Deriving a SID alone (without registering the
profile) causes `CreateProcessW ERROR_FILE_NOT_FOUND`. This is a mandatory pre-launch step in the
broker arm.

**CLR/PowerShell baseline env (banked, Phase 58):** The broker must preserve `SystemRoot`,
`windir`, and `SystemDrive` in the child environment. Clearing these env vars before PowerShell
causes the CLR to fail with exit code `-65536` (`0xFFFF0000`). Always re-add these baseline
variables when building the child env from scratch.

### Linux (Landlock)

`Sandbox::apply()` on the current process applies Landlock restrictions in-process — no re-exec is
required. `sandboxed_exec()` in `nono-py` forks, applies the sandbox to the child, then execs.

Shape B on Linux (`confine()` / `Sandbox::apply(self)`) is simpler and more direct than on
Windows: the in-process call genuinely enforces OS restrictions. The ordering invariant still
applies (call before opening privileged handles), but there is no re-exec and no broker needed.

Note: Landlock is strictly allow-list. `deny.access`, `deny.unlink`, and `symlink_pairs` are
macOS-only constructs; avoid depending on deny-within-allow semantics in cross-platform code.
ABI v4+ (kernel 6.1+) adds TCP network filtering; earlier ABIs are file-only.

### macOS (Seatbelt)

`Sandbox::apply()` calls `sandbox_init()` with a Seatbelt profile string, applying the profile to
the current process. Same in-process enforcement model as Linux Landlock — Shape B is direct on
macOS. No re-exec required; the broker is not involved.

The Seatbelt profile is a Scheme-like DSL. Network is denied by default with
`(deny network*)`. `/etc` on macOS is a symlink to `/private/etc`; both paths must be handled when
building allow rules for system directories.


## Contract Versioning

This contract is at **v1.0** (Phase 72, 2026-06-14). Breaking changes (removing or narrowing an
E-point, changing an invariant from MUST to SHOULD, removing a platform path) require a major
version bump and a migration guide. Non-breaking additions (new platform notes, new E5 consumers,
forward-compat mappings) increment the minor version.

Future implementations (nono-ts Phase 75, daemon Phase 74, zt-infra integration) must document
their compliance with each E-point's invariants against the version of this contract they were
built to.
