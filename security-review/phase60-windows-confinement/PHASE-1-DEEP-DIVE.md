# Windows Native Confinement — Phase 1 Deep-Dive & Vulnerability Assessment

**Scope:** `nono` + `nono wfp` + `zt-infra` integration (Phase 60 "sandbox-the-tools" confined coding loop)
**Build under review:** `main` @ `6922b700`; live telemetry captured against `target\debug\nono.exe` (built 2026-06-13)
**Host:** Windows 11 Enterprise 26200
**Reviewer mandate:** validate whether application-level tool hooks + WFP egress filters + MIC Low-IL jails guarantee absolute agent confinement without unacceptable enterprise friction.

---

## 0. Headline Verdict

**Confinement on Windows is defense-in-depth, not isolation — and that is the honest design ceiling, not a bug to be patched.**

The three layers are real and each enforces at the kernel:

1. **MIC Low-IL jail** (mandatory label `NO_WRITE_UP` + optional `WRITE_RESTRICTED` restricting-SID) — genuine kernel write confinement.
2. **WFP egress** (`FwpmFilterAdd0` under a dedicated sublayer, default-deny) — genuine kernel network block, **but only on the AppContainer/broker arm** where a package SID exists to scope it.
3. **Application hook** (`PreToolUse` mediation) — a *policy router*, structurally bypassable, and **must not be treated as a security boundary**.

The single most important architectural truth surfaced by this review: **the `PreToolUse` hook and its self-disable guard are not load-bearing security controls.** They depend on model cooperation and on a CWD-coarse, junction-blind path check. The actual containment is the OS integrity label and the WFP filter. Every recommendation below pushes enforcement *down* to those kernel layers and treats the hook as best-effort UX steering.

Against the three stated constraints:

| Constraint | Status |
|---|---|
| Host-native (no hypervisor) | **MET** — Landlock-equivalent via MIC/WFP, no VM dependency. |
| Fail-closed | **MET for OS layers** (WFP fail-closed verified; broker fail-closed on token failure). **PARTIAL at the hook layer** — the PowerShell wrapper can emit unparseable output that Claude Code may treat as fail-open (Risk R-A1). |
| Zero-Trust identity (no reliance on mutable disk JSON) | **NOT MET** — the policy that governs the sandbox is an unsigned, user-writable JSON file. The crypto machinery to fix this already exists in `nono::trust` but is **not wired** to the profile load path (Risk R-T1, the central zt-infra gap). |

---

## 1. Target A — Tool Hook & Model Response Logic

**Files:** `crates/nono-cli/src/claude_code_hook.rs`, `crates/nono-cli/data/hooks/nono-tool-hook.ps1`, `packages/claude-code/claude-code-tools-windows-runner.profile.json`

### 1.1 How the mediation works (verified live)

`pre_tool_use_response()` (`claude_code_hook.rs:38`) routes by tool name:

- **Read / Glob / Grep** → `allow` unconfined (`is_read_only_tool`, `:243`).
- **Write / Edit / MultiEdit** → `deny` with an `additionalContext` string carrying a **bare inner PowerShell command** that performs the operation via `[System.IO.File]::WriteAllText/ReadAllText` (`build_confined_write_cmd` `:440`, `build_confined_edit_cmd` `:458`, `build_confined_multiedit_cmd` `:478`). Content is base64-encoded (raw bytes) to neutralize quotes/newlines/metacharacters; paths use `powershell_single_quoted` doubling.
- **Bash** → `allow` with `updatedInput.command` rewritten to a **double-wrapped** trampoline.
- **NotebookEdit** → `deny`, informative only (no retry command).
- **everything else (Task, WebFetch, MCP, …)** → `deny`.

**Live telemetry (this session, `target\debug\nono.exe`):**

```
# Write decision (verbatim stdout):
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny",
"permissionDecisionReason":"Write is an in-process tool that cannot be confined directly; use the Bash tool with the PowerShell command in additionalContext to perform the confined file operation",
"additionalContext":"Use the Bash tool with this PowerShell command: $b = [Convert]::FromBase64String('aGVsbG8gd29ybGQ='); [System.IO.File]::WriteAllText('test.txt', [System.Text.Encoding]::UTF8.GetString($b))"}}
exit=0

# Bash decision: updatedInput.command =
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand <B64-UTF16LE>
  └─ decodes to: & 'C:\...\nono.exe' run --profile 'claude-code-tools-windows-runner' --allow-cwd -- powershell.exe -NoProfile -NonInteractive -EncodedCommand <B64-inner>
       └─ inner decodes to: Get-ChildItem -Force
```

The double-wrap (outer trampoline PS → `nono run` → inner confined PS) was decoded end-to-end and confirmed correct.

### 1.2 Determinism of "denied tool → Low-IL Bash retry" (the critical question)

**The conversion is model-dependent and therefore *not* deterministic.** The hook emits a deny + a natural-language instruction (`"Use the Bash tool with this PowerShell command: …"`); it relies on Claude to (a) read `additionalContext`, (b) re-issue the *exact* command as a Bash call, and (c) not paraphrase or truncate it. Live Phase 60 UAT (`60-HUMAN-UAT.md`, item 3) observed this working reliably "across all runs … no nudge" — but that is an empirical behavioral observation on one model version (Claude Code v2.1.159), **not a structural guarantee.** Three concrete failure modes:

- **R-A2 (context drift / paraphrase):** the model rewrites the PS command (e.g. "fixes" a quote, splits a long base64 blob, adds its own `-Encoding`), producing a different — possibly path-escaping or simply broken — command. Nothing validates that the Bash command Claude sends matches the one the hook proposed. The Bash arm wraps *whatever arrives* (`wrapped_bash_command`, `:382`) after a single `command_contains_nono_wrapper` idempotency check (`:267`).
- **R-A3 (payload size):** a large `Write` (e.g. a 200 KB file) becomes a multi-hundred-KB base64 string inside `additionalContext`. There is no size cap. If the model's context or the tool-call serialization truncates it, the retried `WriteAllText` writes a **silently corrupted/truncated file** with exit 0 — a data-integrity failure, not a security one, but invisible.
- **R-A4 (fail-closed loop / no fallback):** if the model simply gives up after the deny (or the runner profile is missing, or the broker gate refuses), no file operation ever happens and there is no non-model path to complete it. This is the motivation for the **Contingency Fallback (A1)** in §6.

### 1.3 Wrapper fail-open risk (NEW finding, verified live)

`nono-tool-hook.ps1:7` runs `$output = $inputJson | & $nono claude-code-hook 2>&1`. The `2>&1` **merges stderr into stdout**, and on `exit 0` the script echoes `$output` verbatim. `nono` emits `tracing` output to stderr; with `NONO_LOG`/`RUST_LOG` set (a normal operator-debugging state, and trivially attacker-influenceable via environment), the merged stream prepends log lines to the JSON.

**Verified this session:**
```
$env:NONO_LOG="debug"; $out = $json | & nono claude-code-hook 2>&1
# captured $output (what ps1 echoes on exit 0):
2026-06-13T12:25:30Z DEBUG theme: mocha          <-- ANSI-colored stderr line
{"hookSpecificOutput":{...,"permissionDecision":"allow",...}}
# ConvertFrom-Json => PARSE FAILED
```

If Claude Code cannot parse the hook's JSON, the `permissionDecision` is lost. Depending on Claude Code's contract for malformed hook output this is either a **fail-open** (no decision → default-allow) or a **fail-closed loop** (hook error → tool blocked). Either way the boundary is decided by a parsing accident, violating the fail-closed constraint. **Fix: capture stdout and stderr separately; only stdout is the JSON contract; log stderr out-of-band or into the deny reason on failure only.**

### 1.4 Other Target-A observations

- **Idempotency guard is substring-based** (`command_contains_nono_wrapper`, `:267`: `" nono run "` / `"nono run "`). A benign user command that contains the literal `nono run ` would be passed through un-wrapped (`Ok(None)`), i.e. **unconfined**. Low severity (requires the model to emit that exact substring) but it is a string-match security decision — the kind CLAUDE.md explicitly warns against.
- **Self-disable guard precedes payload construction** in every arm (Write/Edit/MultiEdit call `cwd_self_disable_risk_reason()` first, `:62/:102/:150`) — good ordering, verified by `windows_write_arm_cwd_guard_fires_before_ps_cmd`.
- **Non-Windows behavior is an outright deny** for Write/Edit/MultiEdit (`:95/:143/:183`) — correct fail-closed; the confined-write vehicle is Windows-only.

---

## 2. Target B — Windows Mandatory Integrity Control (MIC) Boundary

**Files:** `crates/nono/src/sandbox/windows.rs`, `crates/nono-cli/src/exec_strategy_windows/{launch,restricted_token,dacl_guard,labels_guard,mod}.rs`, `crates/nono-shell-broker/src/main.rs`, `crates/nono/src/supervisor/socket_windows.rs`

### 2.1 Token-drop mechanics

Two enforcement arms, selected by `select_windows_token_arm()` (`launch.rs:1166`):

- **`WriteRestricted`** (`restricted_token.rs:55`): `CreateRestrictedToken(…, WRITE_RESTRICTED, …, 1, &session_sid, …)` with a synthetic SID `S-1-5-117-<guid>`. **The child stays Medium-IL**; write confinement comes from the restricting-SID *double access check* (a write succeeds only if the object's DACL grants the synthetic SID), not from MIC. Because the synthetic SID is on no path's DACL by default, `AppliedDaclGrantsGuard` (`dacl_guard.rs:92`) must add it to each writable granted path.
- **`BrokerLaunch` / `BrokerLaunchNoPty`** (the runner profile's `windows_low_il_broker: true` path): spawns `nono-shell-broker.exe` (Medium-IL), which builds a **Low-IL primary token** (`create_low_integrity_primary_token`, `windows.rs:534`; `SetTokenInformation(TokenIntegrityLevel, WinLowLabelSid)`) and `CreateProcessAsUserW`s the child. Write confinement rides on the **mandatory label**: the out-of-grant Medium-IL world is `NO_WRITE_UP`-protected, the CWD is relabeled Low via `try_set_mandatory_label` (`windows.rs:1040`). This arm is required because the desktop .NET CLR / Windows PowerShell **cannot start under `WRITE_RESTRICTED`** (it creates `BaseNamedObjects` kernel sync objects the synthetic restricting SID can't satisfy — Phase 60 F-60-UAT-05, a kernel-object-namespace limit no filesystem `--allow` can fix).

### 2.2 WRITE_OWNER / DACL requirement

`SetNamedSecurityInfoW(LABEL_SECURITY_INFORMATION)` requires **WRITE_OWNER**, which only the owner holds implicitly (`labels_guard.rs:102-133`). Consequences:
- System paths (TrustedInstaller-owned `C:\Windows`) are **skipped** — correct, they're not the agent's writable surface.
- The DACL/label guards are RAII (`AppliedLabelsGuard`, `AppliedDaclGrantsGuard`, `AppliedAncestorTraverseGuard`) with snapshot/apply/revert-on-Drop and **fail-closed** on ownership-check error.
- **Silent no-op surface (R-B3):** when a writable path is *not owned* by the current user, or already carries a *pre-existing mandatory-label ACE*, the guard **skips** it and only `warn!`s ("label guard: path has pre-existing mandatory-label ACE; skipping apply"). On `WriteRestricted` this means confined writes there are silently denied (fail-secure); but on the label arm a *pre-existing* low/medium label set by a third party silently wins, so nono's confinement may have "no observable enforcement effect" on that path. An operator who doesn't read `warn!` logs cannot tell containment didn't apply.

### 2.3 Medium↔Low IPC (capability/supervisor pipe)

Per-session duplex named pipe (`socket_windows.rs`), advertised via a rendezvous file passed in `NONO_SUPERVISOR_PIPE`. DACL/SDDL `CAPABILITY_PIPE_SDDL` (`:63`):
```
D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;LW)
```
The SACL is a **Low label with `NW` (NoWriteUp removed)** — i.e. the Low-IL child is *intentionally* permitted to write up to the pipe (it must, to send requests). Hardening that holds up well:
- Per-request **32-byte session-token** check in **constant time** (`supervisor.rs:2095`), before any approval backend runs.
- **Replay protection** via `seen_request_ids`, deliberately **not** reset on reconnect (`socket_windows.rs:470`).
- Handle brokering **maps down** (`dwOptions = 0`, not `DUPLICATE_SAME_ACCESS`) for events/mutexes/jobs/pipes; supervisor **refuses to broker its own containment Job**.
- SID strings are injection-validated before SDDL embedding (`S-1-…` / `S-1-15-2-…`, length-bounded); malformed → fatal, no fallback.
- 64 KiB framed-JSON cap; bounded reads with disconnect/timeout classification.

**Caveats (R-B1, R-B2):**
- **R-B1 — logon-session widening:** the `WriteRestricted` arm adds the **current logon SID** to the cap-pipe DACL (`socket_windows.rs:1652`), broadening write access from the synthetic SID alone to *every process in the same logon session*. By design (a Medium-IL child has no distinguishing SID), but it means a co-resident same-session process can talk to the capability pipe. Narrower than `Everyone`, still wider than the jailed child.
- **R-B2 — rendezvous file integrity:** the pipe name embeds SHA-256(path)+16-byte nonce (anti-guessing), but the rendezvous *file* is an ordinary user file protected only by filesystem ACLs.

No clear **escalation/injection** path from Low→Medium was found: the child can only write framed bytes that are token-gated, replay-protected, and discriminator-validated; it never receives the supervisor's handles except down-mapped ones for approved requests. This is a well-constructed boundary.

### 2.4 Broker trust gate (and its weak link) — **HIGH-severity finding R-B4**

Before spawning the broker, `verify_broker_authenticode()` (`launch.rs:2059`) requires both `nono.exe` and `broker.exe` to be Authenticode-`Valid` with **identical signer subject + SHA-1 thumbprint**, fail-closed, no env/CLI escape hatch, re-checked every dispatch. Strong. **But** it is skipped entirely when `is_dev_build_layout()` (`launch.rs:2039`) returns true, and that function is a **path-substring match**:
```rust
s.contains(r"\target\debug\") || s.contains(r"\target\release\")
    || s.contains("/target/debug/") || s.contains("/target/release/")
```
Any unsigned `nono.exe` + sibling broker placed under a directory whose path merely *contains* `\target\release\` (e.g. `C:\Users\victim\target\release\nono.exe`) **bypasses Authenticode verification**. On a shared or attacker-writable machine this is a real trust-anchor bypass. The dev-ergonomics tradeoff is understandable but the signal is path-string, not build provenance. **Fix in §6.**

---

## 3. Target C — Path-Cover Guard Rails & Self-Disable (CR-01)

**File:** `crates/nono-cli/src/claude_code_hook.rs` (`cwd_self_disable_risk_reason_for` `:279`, `cwd_covers_home_claude_state` `:301`, `path_covers` `:330`, `canonicalize_with_existing_prefix` `:336`). **Patch:** `ddb711dc` (the CR-01 fail-open fix), `…c4714a0c`.

### 3.1 What CR-01 fixed (and that the fix is sound)

Pre-CR-01, `path_covers` did `canonical_existing_path(child).unwrap_or_else(|_| child.to_path_buf())`. For the **common case where `~/.claude.json` does not yet exist**, `canonicalize()` fails, the fallback returns the *raw* `C:\Users\…\.claude.json`, and `starts_with()` against a `\\?\C:\…`-canonical parent CWD returns **false** — the guard silently fails open. CR-01 replaces this with `canonicalize_with_existing_prefix()`, which walks the ancestor chain to the deepest existing prefix, canonicalizes that to `\\?\` form, and re-appends the stripped tail so both sides of `starts_with()` are in the same canonical namespace. It also (WR-01) adds a symmetric `cwd.starts_with(home/.claude)` check for CWDs *inside* `~/.claude`, and (WR-02) fails closed when `home_dir()` is `None`.

**Verified live this session** (`target\debug\nono.exe`, controlled `USERPROFILE`):
- CWD = `~` (home) → **deny** (`covers Claude Code hook settings or agent state`), even though `.claude.json` need not exist — the absent-file fail-open is closed. ✔
- CWD = repo root containing `.claude\` → **deny** (project-local branch). ✔
- CWD = `C:\temp\NONO-C~1\proj` (**8.3 short name**), project has `.claude` → **deny**: `canonicalize()` normalized the 8.3 alias to `\\?\C:\temp\nono-c-test2\proj` and the `.claude` child was detected. **8.3 short-name aliasing does NOT bypass the guard.** ✔

### 3.2 The residual bypass — junction inside the granted CWD (**verified live, MEDIUM**)

The guard detects `.claude` only by the literal `cwd.join(".claude").exists()` and by home-coverage of `~/.claude`/`~/.claude.json`. It does **not** resolve the targets of *child* reparse points. Verified:

```
# CWD = work; junction work\notclaude --> fakehome\.claude ; literal work\.claude absent
{"hookSpecificOutput":{...,"permissionDecision":"allow","permissionDecisionReason":"Bash command rewritten through nono tool sandbox",...}}
```
A differently-named junction pointing at `~/.claude` is **invisible** to the guard → the hook engages and grants `--allow-cwd`. The Low-IL child could then traverse `work\notclaude\…` to reach `~/.claude`.

**Why this is "only" Medium, not Critical:** the OS layer backstops it. `~/.claude` is Medium-IL; the broker-arm child is Low-IL, so `NO_WRITE_UP` blocks the write regardless of the junction. On the `WriteRestricted` arm the child is Medium-IL but its restricting SID is absent from `~/.claude`'s DACL, so `WRITE_RESTRICTED` denies it. Either way the *write* is blocked at the kernel — **the junction defeats the hook guard but not the OS containment.** This is precisely why the guard must be classified as defense-in-depth.

**Why it still matters:** (a) it proves the guard is not a reliable control, so any future change that makes it load-bearing — e.g. the F-60-UAT-01 proposal to narrow it to "deny only if the write target is within `.claude`" — **must canonicalize/resolve reparse points** or it will be trivially bypassable; (b) `add_deny_access` for `~/.claude` is a documented **Windows no-op** for the allow-overlap case (project memory; the deny entry in the runner profile is inert), so the hook guard + IL label are the *only* two protections, and one of them is bypassable.

### 3.3 Self-DoS / poisoned-repo friction (LOW security, HIGH ops — R-C2)

Because the guard fires whenever `CWD\.claude` exists, an agent can **permanently disable its own confined Bash** by creating `.\.claude` (a confined write *inside* the granted CWD is allowed), after which every Bash wrap is denied fail-closed. A repository that simply *ships* a `.claude/` directory makes nono confinement unusable in that repo (this is the root of F-60-UAT-01, which forced `CLAUDE_CONFIG_DIR` relocation as the only viable deployment). Fail-secure, but a denial-of-service and a serious enterprise-friction blocker for the normal project-scoped-hook workflow.

---

## 4. Deliverable 1 — UAT Validation Matrix (Items 1–5)

### 4.1 Execution reality

The Phase 60 cookbook's UAT 1–5 are **interactive, real-host E2E items** that require: a live Claude Code session driving the hook; a signed **or** dev-layout `nono.exe` (broker trust gate); the runner profile installed under `%APPDATA%\nono\profiles\`; and — for `network.block:true` — an **elevated** `nono-wfp-service`. They cannot be fully executed non-interactively from this review environment (the Bash/MSYS path fails the broker's real-console requirement; `CreateProcessAsUserW` returns GLE=87 without a real console — project memory `feedback_windows_supervised_needs_real_console`). The authoritative live pass is recorded in `60-HUMAN-UAT.md` (Win11, Claude Code v2.1.159, `nono 0.57.5` dev-layout, **5/5 PASS** with `network.block:false`).

What I **independently re-verified this session** is the deterministic **decision layer** — the exact allow/deny/rewrite telemetry the hook emits — plus the self-disable guard and payload structure (§1–§3). That is the portion that is reproducible without an interactive console.

### 4.2 Matrix

| # | UAT item | SC | Layer re-verified here | Result | Telemetry / evidence |
|---|----------|----|------------------------|--------|----------------------|
| 1 | Confined edit lands | SC1 | Decision layer ✔ (full E2E = live host) | **PASS (decision)** / PASS (live, `60-HUMAN-UAT`) | `Write{test.txt,"hello world"}` → `deny` + `additionalContext` = `$b=[Convert]::FromBase64String('aGVsbG8gd29ybGQ='); [System.IO.File]::WriteAllText('test.txt', …UTF8.GetString($b))`, exit 0. Live: 12 B file landed via broker arm, child exit 0. |
| 2 | Out-of-scope write denied at OS boundary | SC1 | — (kernel boundary, live host) | PASS (live) | Live: `Write ..\outside.txt` ran in Low-IL shell; OS denied `UnauthorizedAccessException: Access to 'C:\Users\OMack\outside.txt' is denied`; no file created. Mandatory-label boundary holds. |
| 3 | deny + additionalContext → Bash retry (A1) | — | Decision layer ✔ | **PASS (decision)** / PASS (live) | Hook deny carries a non-empty inner PS command in every Write/Edit/MultiEdit case (verified for special chars too). Model conversion observed reliable live — **but model-dependent, see R-A2/A4.** |
| 4 | PowerShell steering unprompted | SC2 | n/a (model behavior) | PASS (live) | Live: "List the files" → Claude used `Get-ChildItem -Force \| Format-Table` confined under broker arm, not `ls`. Steering text in `packages/claude-code/CLAUDE.md`. |
| 5 | E2E read→edit→run | SC4 | Decision layer ✔ (Read allow, Bash rewrite) | **PASS (decision)** / PASS (live) | Read=allow; Bash rewrite double-wrap decoded end-to-end (`nono run --profile … --allow-cwd -- powershell.exe …EncodedCommand <inner>`). Live: read `notes.md` → write `greet.ps1` → run `& greet.ps1` → "Hello from nono POC", exit 0. |

**Process-token level (design):** parent `claude.exe` = Medium-IL; confined child = **Low-IL** (broker primary token via `windows_low_il_broker:true`) or Medium-IL+`WRITE_RESTRICTED` (legacy arm). CWD relabeled Low; out-of-grant world Medium-IL `NO_WRITE_UP`.

**Caveats carried forward:** (a) production `network.block:true` needs the elevated WFP service (F-60-UAT-03, not exercised here); (b) the broker arm requires dev-layout or signed binary (broker gate, R-B4); (c) `pwsh.exe` (PS7/MSIX) is *not* the vehicle — the hook uses `powershell.exe` (Windows PowerShell 5.1), so the `WindowsApps` launch-policy gap is moot for the shipped path.

---

## 5. Deliverable 2 — Risk & Vulnerability Register

Severity = likelihood × blast radius given the **defense-in-depth (not isolation)** posture. "Escape" = cross the file/network/identity boundary; "DoS" = render confinement or the agent unusable.

| ID | Title | Class | Sev | Mechanism | Backstop / mitigation | Fix |
|----|-------|-------|-----|-----------|------------------------|-----|
| **R-B4** | Broker Authenticode gate bypassed by path substring | Escape (trust anchor) | **HIGH** | `is_dev_build_layout()` skips Authenticode if path contains `\target\release\`; unsigned broker under such a path is trusted (`launch.rs:2039`). | None once triggered — the gate *is* the control. | Gate dev-bypass on `#[cfg(debug_assertions)]` build provenance, not a runtime path string; or require the dev binary live under a verified-trusted root. |
| **R-T1** | Sandbox policy is unsigned mutable JSON | Identity / ZT | **HIGH** | Resolver prefers `%APPDATA%\nono\profiles\*.json` with **no** signature/digest/issuer check (`profile/mod.rs:2508`). Anyone who can write that dir rewrites the policy that governs the jail. | Parse-failure is fatal, but a *well-formed tampered* profile loads. | Wire `nono::trust::{sign_policy_bytes, verify_keyed_signature, SignerIdentity}` (already present) into the profile load path; map issuer to the zt-infra ledger root. §6.3. |
| **R-A1** | Hook wrapper merges stderr→stdout, can emit unparseable JSON | Fail-open/loop | **HIGH** | `nono-tool-hook.ps1:7` `2>&1`; any `tracing` line corrupts the JSON contract (verified). | Default (no `NONO_LOG`) path is clean. | Separate stdout (JSON) from stderr; never echo stderr on the success path. |
| **R-A4** | Model-dependent retry has no non-model fallback | DoS / availability | **MED** | If the model doesn't convert deny→Bash, the op never completes; runner-profile-missing / broker-gate failure also dead-ends. | Live runs reliable on v2.1.159. | Contingency Fallback A1 — hook spawns `nono run` directly. §6.1. |
| **R-C1** | Junction inside CWD bypasses self-disable guard | Escape (hook layer) | **MED** | Differently-named junction → `~/.claude` is invisible to the guard (verified `allow`). | OS IL label blocks the write (`~/.claude` Medium-IL vs Low-IL child); `WRITE_RESTRICTED` SID-DACL blocks on legacy arm. | If the guard is ever narrowed, resolve reparse points; treat OS label as the real control. |
| **R-A2** | Model paraphrases/mutates the proposed PS command | Integrity / escape | **MED** | Bash arm wraps *whatever arrives*; no equivalence check vs the hook's proposal. | Confined runner still bounds the result to CWD + Low-IL. | Have the hook emit an opaque single-use token that the Bash arm validates, instead of free-form PS text. §6.1. |
| **R-B1** | Cap-pipe DACL widened to whole logon session (WriteRestricted) | Info / lateral | **MED** | `socket_windows.rs:1652` adds current logon SID to pipe DACL. | Per-request constant-time token + replay + discriminator gating. | Prefer the AppContainer/broker arm (package-SID-scoped pipe); document WriteRestricted as weaker. |
| **R-N1** | WFP scope is program-image (AppID), not identity, on non-AppContainer arms | Net leak | **MED** | `package_sid` is `None` off the broker arm → WFP filter falls back to `ALE_APP_ID` (`nono-wfp-service.rs:1556`), matching the image not the per-run jail. | Default-deny BLOCK still applies to the image; broker arm scopes to package SID correctly. | Confirm `WriteRestricted + network.block` reachability; if reachable, document image-scope or require broker arm for network blocking. |
| **R-B3** | Silent label/DACL skip on pre-labeled or non-owned paths | Silent non-enforcement | **MED** | Guards `warn!` and skip; operator sees no hard error (`labels_guard.rs:87`). | Fail-secure for writes (denied), but enforcement may be a no-op. | Promote skip-on-writable-granted-path to an error or a structured telemetry event (§6.4). |
| **R-C2** | `.claude` in CWD permanently trips guard (poisoned repo / self-DoS) | DoS / friction | **LOW(sec)/HIGH(ops)** | Any `CWD\.claude` → all Bash wraps denied; repos shipping `.claude/` are unusable. | Fail-closed (secure). | Narrow guard to the specific write target + `CLAUDE_CONFIG_DIR` deployment guidance; MSI provisions config out-of-band. §6.2. |
| **R-A3** | Large Write/Edit base64 in additionalContext can truncate | Integrity | **LOW** | No size cap on the inner PS payload; truncation → silent corrupt file, exit 0. | — | Cap payload size; above the cap, stage content to a temp file inside the grant and reference it. |
| **R-A5** | `command_contains_nono_wrapper` substring lets `nono run ` pass un-wrapped | Escape | **LOW** | `:267` substring match → `Ok(None)` unconfined passthrough. | Requires the model to emit that exact substring. | Replace substring check with a structural marker (env var / sentinel arg the hook injects). |
| **R-B2** | Rendezvous file integrity rests on filesystem ACLs | Info | **LOW** | `NONO_SUPERVISOR_PIPE` file is an ordinary user file. | SHA-256+nonce pipe name; token-gated pipe. | Tighten rendezvous file DACL; consider passing the name via inherited handle only. |
| **R-T2** | Merkle state root computed but not signed/anchored | Auditability | **LOW** | `undo/merkle.rs` builds a root "ready to be signed" but no signature is wired. | — | Sign the root with the keyed signer; anchor to ledger for tamper-evident agent-action proof. |

---

## 6. Deliverable 3 — Formal Recommendations (PoC → Production)

### 6.1 Contingency Fallback Plan — model-independent confined spawn (closes R-A4, R-A2)

**Problem:** today `Write/Edit/MultiEdit` are denied and rely on Claude to re-issue a Bash call. If the model fails to convert, the operation dead-ends.

**Design — make the hook *do the work* instead of *instructing* the model:**

1. **Direct-execute mode (primary).** In the `Write/Edit/MultiEdit` arm, after constructing the inner PS command, the hook itself invokes `nono run --profile <runner> --allow-cwd -- powershell.exe -EncodedCommand <inner>` synchronously (the same trampoline it currently asks Claude to run), captures exit/stderr, and returns a **terminal** `PreToolUse` decision:
   - success → `permissionDecision: "deny"` *with* an `additionalContext` confirming the file was written confined (so the model doesn't retry), **or** preferably a `"allow"` of a no-op stand-in — whichever maps cleanly to Claude Code's "the side effect already happened" semantics. (Recommend a short spike to pick the exact decision shape Claude Code honors without double-execution.)
   - failure → `deny` with the captured nono stderr as the reason (fail-closed, no silent loss).
   This removes the model from the *execution* path entirely; the model only decides *whether* to edit, nono decides *how* and *whether it's allowed*.
2. **Token-validated retry (defense for R-A2) if direct-execute is not adopted.** The hook emits an **opaque single-use token** (HMAC over `{tool, file_path, content_digest, nonce}`, key held by the supervisor) instead of free-form PS text. The Bash arm only wraps a command bearing a valid, unexpired, unused token, and reconstructs the PS command server-side from the bound parameters — so a paraphrased/mutated model command is rejected, and the executed command is provably the hook's proposal.
3. **Fallback ordering, all fail-closed:** direct-execute → (on broker-gate/profile error) structured deny with remediation → never silent allow.

**Effort:** ~1 phase. Risk: must verify Claude Code's contract for "tool whose effect the hook performed" to avoid double-writes; gate behind a profile flag (`hook_direct_execute: true`) for A/B.

### 6.2 Silent Enterprise Installer — code-signed MSI/MSIX (closes R-C2 friction, supports R-B4, R-T1)

The existing MSI pipeline (`scripts/build-windows-msi.ps1`, machine + user `.wxs`, **generated** — see project memory) is the foundation. Production blueprint:

- **Machine-wide, code-signed MSI** (EV cert) installing `nono.exe` + `nono-shell-broker.exe` + `nono-wfp-service.exe` to `C:\Program Files\nono\`, **co-signed with the same subject/thumbprint** so the broker Authenticode gate (R-B4) passes *without* the dev-layout bypass — then **remove/disable the path-substring dev bypass in release builds** (`#[cfg(not(debug_assertions))]`).
- **Service provisioning:** register and auto-start `nono-wfp-service` at install (`sc create … start=auto`), so `network.block:true` works without an ad-hoc elevated `nono setup --start-wfp-service` (closes F-60-UAT-03 for enterprise).
- **Workspace permission provisioning:** an install-time/first-run step that pre-creates the standard agent workspace root with correct ownership so the WRITE_OWNER/label-apply path (R-B3) never silently skips. Ship a `nono setup --provision-workspace <dir>` that sets owner + a clean ACL and emits a structured result.
- **Hook + config deployment out-of-band:** install the `PreToolUse` hook into a **machine or relocated user config** (the `CLAUDE_CONFIG_DIR` model that Phase 60 proved is the only working deployment) and the runner profile into `%PROGRAMDATA%\nono\profiles\` (machine, read-only to non-admins) — which *also* removes the R-T1 user-writable-policy exposure for the default profile. This sidesteps R-C2 entirely: confinement no longer depends on a project-local `.claude` that trips the self-disable guard.
- **MSIX** option for Store/Intune deployment; note PS7-as-MSIX (`WindowsApps`) is not on the confined path, so no launch-policy change needed.

### 6.3 Zero-Trust policy signing — wire `nono::trust` into profile load (closes R-T1, the central zt-infra gap)

The crypto stack already exists and is unused on the profile path:

| Needed | Exists today | Where |
|---|---|---|
| Sign a policy doc (P-256 + DSSE + in-toto `NONO_POLICY_PREDICATE_TYPE`) | ✔ `sign_policy_bytes`/`sign_policy_file` | `crates/nono/src/trust/signing.rs` |
| Verify bundle + **pin signer identity** | ✔ `verify_bundle`, `verify_keyed_signature`, `SignerIdentity` | `trust/bundle.rs`, `profile_runtime.rs:299-342` |
| Strictest-wins, project-can't-weaken-user, no-TOFU merge | ✔ `TrustPolicy::evaluate_file` | `trust/policy.rs` |
| Trust root | ✔ TUF root (local cache) | `bundle.rs::load_production_trusted_root` |

**Design:**
1. At `profile/mod.rs:2510` (user-profile load), require a verified detached signature (`.nono-trust.bundle` sidecar or keyed signature) for every `profiles/*.json` *before* it governs a sandbox — mirroring the instruction-file interceptor (`trust_intercept.rs`). Fail-closed if absent (configurable enforcement `Deny|Warn|Audit` for migration).
2. Replace/augment the TUF file root with a **zt-infra ledger-anchored issuer set**: `SignerIdentity` (keyless `{repo, workflow, git_ref}` or keyed `{key_id}`) resolves against a ledger record rather than a local file, so revocation and issuance are decentralized-ledger-driven. The verification call site doesn't change; only the trust-root provider does.
3. Sign the **Merkle state root** (R-T2) with the same keyed signer and anchor it to the ledger → tamper-evident proof of exactly what the agent did, mapping cleanly into the zt-infra audit paradigm.

**Net:** policy stops being "trusted because it's on disk" and becomes "trusted because a ledger-anchored issuer signed it," with no new crypto code — only new call-site wiring and a ledger trust-root adapter.

### 6.4 EDR/SIEM integration — structured telemetry to Windows Event Log

Today, denials and skips surface only as `tracing` lines (and R-A1 shows that stream is fragile). Spec:

- **Channel:** a dedicated ETW provider / custom Event Log channel `nono/Operational` (and `nono/Security` for boundary events), registered by the MSI so SecOps can subscribe via WEF → SIEM.
- **Event taxonomy (structured, one event ID per class):**
  - `1001 ToolDenied` — `{tool, file_path?, reason, decision, profile, pid, parent_pid, integrity_level, cwd}`
  - `1002 BashRewritten` — `{original_cmd_digest, profile, allow_cwd, arm}`
  - `1100 BoundaryDenied` (OS-enforced) — `{path, op, integrity_level, ntstatus}` (the `UnauthorizedAccessException` / `Access is denied` class from UAT 2)
  - `1200 NetworkBlocked` (WFP) — `{remote_addr, port, filter_id, sublayer, scope_principal}`
  - `1300 GuardFired` — self-disable / junction / fail-closed `{cwd, branch, reason}`
  - `1400 EnforcementSkipped` — **promotes R-B3 from a `warn!` to a first-class security event** `{path, reason: pre_existing_label|not_owned}` so silent non-enforcement is visible to SecOps.
  - `1500 TrustVerification` — profile/broker signature pass/fail `{subject, thumbprint, signer_identity, outcome}`.
- **Correlation:** every event carries a per-run `session_id` (the 32-byte token's public correlation half, never the secret) so a SIEM can reconstruct a full agent run: rewrite → deny → boundary-block → network-block.
- **Schema:** emit as Event Log XML *and* a JSON `EventData` blob for schema-on-read SIEMs (Sentinel/Splunk). Reuse `audit_attestation.rs` to **sign** the security-channel batch so log tampering is detectable — consistent with §6.3.
- **Fail-closed coupling:** a boundary/trust event MUST be emitted *before* the corresponding deny returns, so a dropped log can't hide a denied escape attempt.

---

## 7. Open Items for Phase 2

1. **Decide R-C1/F-60-UAT-01 properly:** narrow the self-disable guard to the specific write target (with reparse-point resolution) vs. mandate the `CLAUDE_CONFIG_DIR`/MSI deployment and document project-scoped hooks as unsupported. Security-sensitive — needs `/gsd:debug` + a plan, not an inline edit.
2. **Confirm `WriteRestricted + network.block` reachability** (R-N1); if reachable, either force the broker arm for network blocking or document image-scoped WFP.
3. **Spike the §6.1 direct-execute decision shape** against the current Claude Code hook contract before committing the fallback.
4. **Elevated WFP live-UAT** (F-60-UAT-03) remains the one unverified kernel-network item; pair with the MSI auto-start service work.
