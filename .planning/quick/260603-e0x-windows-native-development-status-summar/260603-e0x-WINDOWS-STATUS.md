# nono on Windows — Native Development Status

**For:** upstream (macOS/Linux) nono maintainers
**From:** the Windows-native fork
**As of:** 2026-06-03 · current released line **v0.57.5** (signed) · Windows-parity milestones **v1.0 → v2.9**

> **TL;DR.** The fork brings nono to functional parity on Windows using OS-native
> enforcement: **WFP** for network, **Low-Integrity / AppContainer** for filesystem +
> process confinement, and **Job Objects** for resource limits — the Windows analog of
> Landlock (Linux) / Seatbelt (macOS). Core sandboxing, the supervisor lifecycle,
> signed-MSI release automation, and (as of this week) **out-of-box kernel WFP network
> enforcement** are shipped and validated on live Windows 11. The current frontier is a
> **POC** that runs the Claude Code agent at Medium IL and confines each *tool call* to a
> Low-IL `nono` jail. Next up is packaging/releasing that POC as **v2.9**, then resuming
> the **UPST7** upstream-sync track (v0.58/v0.59).

---

## 1. Enforcement model — how Windows maps to the Unix backends

| Concern | Linux | macOS | **Windows (this fork)** |
|---|---|---|---|
| Filesystem confinement | Landlock | Seatbelt | **Mandatory Integrity Control** (`NO_WRITE_UP` Low-IL label) + **AppContainer** (lowbox) + per-path DACL grants |
| Network filtering | Landlock ABI v4+ (TCP) | Seatbelt `(deny network*)` | **WFP** (Windows Filtering Platform) kernel filters via a `LocalSystem` service (`FwpmFilterAdd0`) |
| Resource limits | cgroup v2 | `setrlimit` | **Job Objects** (CPU rate cap, memory, wall-clock timeout, process count) |
| Privilege-drop / spawn | `fork`+`exec` | `fork`+`exec` | **Broker process** (Medium IL) that lowers a token to Low-IL / launches a per-run AppContainer child (Windows can't drop privilege as cleanly as `fork`) |

**Two Windows-specific structural facts worth flagging:**
- **No in-process escape hatch**, same as the Unix backends — once the Low-IL label /
  AppContainer / WFP filter is applied, the child cannot widen it.
- **Per-run AppContainer identity.** Network enforcement keys a WFP `ALE_USER_ID` filter
  on a per-run **package SID** (`S-1-15-2-*`) derived from a unique profile registered for
  that run. This replaced an earlier `WRITE_RESTRICTED` restricting-SID approach that
  crashed heavy native children (`STATUS_DLL_INIT_FAILED 0xC0000142`) — see §4.

---

## 2. Available on Windows today (shipped + validated)

Full list lives in `PROJECT.md` → *Requirements › Validated*. Highlights:

**Core sandbox & CLI**
- WFP network filtering + Low-IL filesystem enforcement; capability builder
  (`--allow` / `--read` / `--block-net` / profile-backed policy); built-in profiles
  (claude-code, codex, opencode, openclaw, swival). Library stays policy-free; the CLI owns policy.
- C FFI bindings (`nono-ffi`); Windows CI lanes (build, smoke, integration, security,
  parity-regression, packaging).

**Supervisor & process model**
- Supervisor parity: `attach` / `detach` / `ps` / `stop`; live stdout streaming + stdin on
  detached sessions, clean detach/re-attach, second-attach rejection (**ATCH-01**).
- `nono wrap` (**WRAP-01**); session records `nono logs` / `inspect` / `prune` (**SESS-01/02/03**)
  + retention sweep (**CLEAN-01..04**).
- `nono shell` via a **broker-process architecture** (Medium-IL broker holds the console,
  lowers a duplicate token to Low-IL, launches the shell) — kernel `NO_WRITE_UP` write-deny
  enforced (**SHELL-01**, production-validated).
- **No-PTY Low-IL broker mode (v2.7)** so heavy self-contained runtimes (e.g. the ~234 MB
  `claude.exe`) survive `DllMain`/bootstrap under `nono run` while write-deny is preserved.

**Network, credentials, discovery**
- WFP promoted to the primary network backend; port-level allowlists (`--allow-port`,
  bind/connect, **PORT-01**); proxy credential injection (**PROXY-01**); `nono learn` via ETW
  (**LEARN-01**); runtime capability expansion over a named pipe (**TRUST-01**, stretch).
- Extended handle brokering on the capability pipe (sockets/pipes/jobs/events/mutexes with
  `DuplicateHandle` map-down + access-mask validation) (**AIPC-01**).

**Resource limits (Job Objects)** — CPU % cap, memory cap, wall-clock timeout, process-count
cap (**RESL-01..04**), surfaced in `nono inspect`.

**Release & packaging** — signed `.exe`, **machine + user MSIs**, zip; Authenticode +
sigstore/TUF signing; `release.yml` CI pipeline (produces signed MSIs on a `v*` tag push).

**🆕 Out-of-box WFP kernel network enforcement (Phase 62, completed 2026-06-03).**
A supervised `nono run --block-net` now enforces WFP kernel filtering **out of the box** on
a machine-MSI host — no manual `nono setup --start-wfp-service` step — and **fails closed**
(never silently passes through unenforced) if the service is stopped and can't be elevated.
Validated **5/5 success criteria on live Windows 11 (build 26200)**: out-of-box block,
boot-start survival, fail-closed remediation, clean uninstall (leaves nothing), and control-pipe
isolation. Security review: **33/33 threats closed**. This closes the last carry-forward gap
from the confined-coding-loop POC (below).

---

## 3. In POC / under test — "sandbox-the-tools" confined coding loop (v2.9)

**The idea.** Rather than run the whole agent inside the sandbox, run the **Claude Code TUI at
Medium IL** and confine each **tool call** to a Low-IL `nono` jail via a `PreToolUse` hook that
shells out to `nono run`. Merged as **PR #4** (experimental `claude-code` profile).

**What works in the POC (Phase 60, live-UAT PASS 5/5 on real Win11, 2026-06-01):**
- **Confined file edits** — `Write` / `Edit` / `MultiEdit` / `NotebookEdit` execute as Low-IL
  `nono`-confined file ops scoped to the target path (per-call capability mapping). A write
  outside granted scope is denied at the OS boundary.
- **Confined `Bash`** — jailed Low-IL, with the shell story steered toward PowerShell.
- **Deny-by-default** for everything else.

**Honest limits of the POC (documented, by design):**
- **Verdict = defense-in-depth, not full isolation.** The *agent process itself* stays Medium-IL
  with unconfined reads; only side-effecting tool calls are jailed.
- **Network / `WebFetch` / `WebSearch` / MCP-under-nono / `Task` subagents** are **denied** in
  the POC, not confined.
- A **fully-confined interactive TUI at Low-IL is OS-blocked** (`0xC0000142`) — this is *why*
  the design pivoted to tool-wrapping rather than jailing the whole TUI.
- Heavy-runtime `claude.exe` runs needed the no-PTY Low-IL broker (v2.7) + the AppContainer
  network arm (Phase 62); a full read-grant model for `claude.exe` under AppContainer is
  **deferred** (the lowbox is a different security principal than the user).

**How we test.** All of the above is validated by **operator-run HUMAN-UAT on physical
Windows 11 hosts** (build 26200), not just CI — kernel WFP, mandatory labels, AppContainer
spawn, and reboot/uninstall behavior can only be proven on real hardware.

---

## 4. Known limitations & explicit deferrals (so nothing is overclaimed)

- **No kernel minifilter / runtime-trust interception.** `nono-wfp-driver.sys` is an
  out-of-scope **placeholder**; all real WFP enforcement is done by the user-mode
  `LocalSystem` service. A signed kernel driver (Gap 6b) is **deferred to v3.0**.
- **WR-02 EDR HUMAN-UAT** deferred to v3.0 (needs an EDR-instrumented runner).
- **Cross-target clippy** for cfg-gated Unix code can't run on the Windows dev host; it is
  deferred to live CI (codified as a MUST/NEVER rule in `CLAUDE.md`).
- **One LOW accepted risk in Phase 62 (AR-62-10):** the WFP service has no
  `util:ServiceConfig` crash-loop recovery policy yet (deferred, needs `WixToolset.Util.wixext`).
  Residual is self-DoS only — with no restart policy a crashed service stays *stopped*, so runs
  fail **closed**; no enforcement bypass.

---

## 5. Next development milestones

| When | Milestone | Scope |
|---|---|---|
| **Now** | **Phase 61 — Ship/Release v2.9** | Package + release the confined-coding-loop POC + Phase 62 WFP enforcement: CI-signed machine+user MSIs via `release.yml`, tag + push the v2.9 milestone, write release notes. Also drains untagged post-v2.7 fixes (broker `GLE=87`, no-PTY stdout-echo, WFP service-stop/uninstall) and live-verifies `release.yml`. |
| Next | **v2.8 — UPST7 upstream sync + v2.7 drain** (Phases 53–59) | Absorb upstream **v0.58.0 + v0.59.0** while preserving the Windows-native model: JSONC profile parsing, `target_binary` profile field, configurable timeout constants, **fine-grained `allow_domain`** (URL path + HTTP-method filtering in the proxy), **Bitwarden `bw://`** credential source, **session lifecycle hooks** (needs a Windows-equivalent broker-spawned execution design + ADR — highest design risk), and **supervisor IPC robustness** (keep-alive / bounded read-timeouts translated to the Named-Pipe AIPC path). |
| v3.0 | Deferred hardening | Signed kernel minifilter (Gap 6b), EDR-instrumented HUMAN-UAT (WR-02). |

> **Note on the relationship to upstream.** Most cross-platform-core work (proxy filtering,
> credential sources, IPC hardening) ports straight from upstream; the Windows-divergent
> surfaces (TLS interception, session-hook execution, named-pipe IPC) are diff-inspected and
> given Windows-specific designs/ADRs rather than blind cherry-picks. The fork's upstream
> high-water mark is **v0.57.0**; UPST7 closes the **v0.58 + v0.59** gap.

---

### Pointers for maintainers who want to go deeper
- `PROJECT.md` — full Validated/Active/Out-of-scope requirement lists + Key Decisions.
- `ROADMAP.md` — phase-by-phase breakdown and milestone history (v1.0 → v2.9).
- `CLAUDE.md` — architecture (library-vs-CLI boundary), platform notes, security principles.
- Phase 62 artifacts (`.planning/phases/62-.../`) — the WFP-enforcement HUMAN-UAT + SECURITY review,
  a representative example of how Windows-only, hardware-gated features are validated.
