---
slug: wfp-broker-token-no-sid
status: resolved
resolution: "SUPERSEDED 2026-06-03. The restricting-SID design this session produced was FALSIFIED — CreateRestrictedToken(WRITE_RESTRICTED) crashed confined children (0xC0000142). The real fix is the per-run AppContainer (lowbox) whose package SID is WFP-matchable via ALE_USER_ID — see resolved/wfp-write-restricted-0142.md, shipped v0.57.12 (plans 62-12/62-13). REQ-WFP-01 verified 5/5 SC PASS on live Win11 (62-HUMAN-UAT.md)."
trigger: "WFP --block-net does not block the confined child: the broker-arm (BrokerLaunchNoPty) Low-IL primary token does NOT carry the synthetic session_sid that the WFP ALE_USER_ID filter matches on, so the filter installs but matches nothing. Surfaced during 62-04 HUMAN-UAT (live Win11) after 62-05..62-09 made WFP fully activate."
created: 2026-06-02
updated: 2026-06-02
phase: 62
requirements:
  - REQ-WFP-01
---

# Debug + DESIGN: broker Low-IL token lacks session_sid → WFP filter matches nothing

> **RESOLVED (superseded) 2026-06-03.** This session's restricting-SID design was falsified (WRITE_RESTRICTED crashed native children with 0xC0000142). Superseded by the per-run AppContainer fix in `resolved/wfp-write-restricted-0142.md` (shipped v0.57.12, plans 62-12/62-13). REQ-WFP-01 now verified 5/5 SC PASS on live Win11. Retained for the historical record.

## Symptoms

**Expected:** `nono run --profile claude-code --block-net --allow-cwd -- curl.exe -sS -m 5 https://api.ipify.org` → confined child's outbound TCP blocked by the kernel WFP filter (REQ-WFP-01 SC1).

**Actual (live Win11, 2026-06-02, hash-confirmed fixed service 5A3355A1...):** All mechanics work — broker spawns ("Low-IL primary token constructed", "spawned Low-IL child child_pid=34064"), WFP activates (no FwpmFilterAdd0 error). BUT `curl` PRINTED THE EXTERNAL IP `68.237.8.196` and exited 0 — **outbound was NOT blocked**. The WFP filter installed but did not match the child's traffic.

**Prior layers (all FIXED + committed this session):** 62-05 generator start=auto, 62-06 driver-gate, 62-07 null filter displayName, 62-08 SD→FWP_BYTE_BLOB, 62-09 persistent WFP session. WFP now activates cleanly; this is the enforcement-MATCH gap.

## Confirmed root cause

The WFP block filter scopes to the connection's user token via `FWPM_CONDITION_ALE_USER_ID` with a security descriptor `D:(A;;CC;;;<session_sid>)` (nono-wfp-service.rs sid_to_security_descriptor + add_policy_filter). `<session_sid>` is the SYNTHETIC per-session SID (`generate_session_sid`). For the filter to match a connection, the connecting process's token must carry `session_sid`.

- The `WriteRestricted` token arm adds `session_sid` as a RESTRICTING SID on the child token (so WFP would match) — but `WriteRestricted` breaks child process startup (curl AND powershell both "failed to start" in live UAT).
- The `BrokerLaunchNoPty` arm (selected by `profile.windows_low_il_broker=true`, e.g. claude-code) starts children fine, but its token is built by `nono::create_low_integrity_primary_token()` which takes NO sid; nono passes the broker only `--shell`/`--shell-arg` (launch.rs ~L1816). So the broker child token does NOT carry `session_sid`.
- CONFIRMED by the codebase's own comment — `dacl_guard.rs` L32-34: "The grant is OPERATIVE only on the `WriteRestricted` token arm ... On every other arm (`BrokerLaunch*`, `LowIlPrimary`, ...) the SID is on no child token."

### Verification (static trace, this session)

1. `nono/src/sandbox/windows.rs:534` `create_low_integrity_primary_token()` — parameterless. Mechanism: `OpenProcessToken` → `DuplicateTokenEx`(TokenPrimary) → `CreateWellKnownSid(WinLowLabelSid)` → `SetTokenInformation(TokenIntegrityLevel)`. It NEVER touches the SID/group list. CONFIRMED: broker child token has no session_sid.
2. `nono-shell-broker/src/main.rs:235` calls the parameterless fn; argv parser (L73-161) accepts only `--shell/--shell-arg/--inherit-handle/--cwd/--no-pty`. No SID channel exists. CONFIRMED.
3. `launch.rs` `select_windows_token_arm` (L1166): cascade is detached→Null, pty→BrokerLaunch, **prefers_low_il_broker && has_session_sid → BrokerLaunchNoPty**, has_session_sid→WriteRestricted, caps→LowIlPrimary. claude-code sets `windows_low_il_broker=true` → BrokerLaunchNoPty. Its match arm (L1300) sets `h_token = null` (broker self-degrades). CONFIRMED the broker arm carries no SID.
4. `launch.rs` no-PTY broker_args build (L1816-1829): only `--shell/--shell-arg/--no-pty/--inherit-handle/--cwd`. No session_sid passed. CONFIRMED.
5. WriteRestricted reference: `restricted_token.rs:34` `create_restricted_token_with_sid` = `CreateRestrictedToken(token, WRITE_RESTRICTED, 0,null,0,null, 1, &sid_restrict, ...)` — session_sid as the single RESTRICTING SID. The in-file comment (L82-89) documents the WORKING reference: under WRITE_RESTRICTED, **WFP's ALE_USER_ID match is a read-like access check** that passes (the restricting SID remains on the token and is visible to the WFP access check); only WRITE-class second-checks are gated. CONFIRMED that a restricting SID satisfies the ALE_USER_ID match today.
6. WFP service: `nono-wfp-service.rs:1247` SDDL = `D:(A;;CC;;;{sid})`; `add_policy_filter` L1326 `FWPM_CONDITION_ALE_USER_ID` + `FWP_MATCH_EQUAL` over that SD. The `CC` right (`ADS_RIGHT_DS_CREATE_CHILD`, mask 0x1) is the conventional WFP user-ID probe: BFE access-checks the connection token against the SD requesting CC; if the token grants CC to session_sid → condition matches → BLOCK applies. CONFIRMED.
7. Single-source: `execution_runtime.rs:379` generates ONE `session_sid` into `config.session_sid`, consumed by BOTH the token arm AND the WFP service request (`network.rs:542` `request.session_sid = ...`). CONFIRMED the broker injection must use this same string.

Net: WFP SID-scoping (built/validated against WriteRestricted) and the broker arm (needed for child startup) are mutually exclusive AS BUILT. REQ-WFP-01 needs both. Root cause CONFIRMED.

## DESIGN (deliverable — find_root_cause_only, NO code applied)

### D1 — SID form for the ALE_USER_ID match

The broker child token is an **integrity-lowered PRIMARY token** (label-only), NOT a restricted token. Therefore the WriteRestricted double-access-check hazard does NOT apply to the broker arm — there is no restricted-SID second check to satisfy, and no need for `WRITE_RESTRICTED` to keep reads open.

Chosen SID form: **add `session_sid` as an ENABLED GROUP** (`SE_GROUP_ENABLED`) on the broker's Low-IL primary token via `AddSidToBoundaryDescriptor`-style group injection — concretely via `CreateRestrictedToken` used as a *group-adding* call is NOT appropriate (that produces a restricted token). The correct primitive is to inject the SID into the token's **groups** list. Two viable mechanics, in preference order:

- **(Preferred) `CreateRestrictedToken` with `WRITE_RESTRICTED` + the session_sid as the single restricting SID** — i.e. mirror `create_restricted_token_with_sid` EXACTLY, then ALSO apply the Low-IL label. Rationale: this is the *proven* WFP-matchable shape (reference #5/#6), and the `WRITE_RESTRICTED` flag is what made the WriteRestricted arm's reads (DLL loads etc.) work. BUT live UAT showed WriteRestricted breaks `curl`/`powershell` startup *on the non-broker path*. The broker path differs (Medium-IL broker self-degrades, anonymous-pipe stdio, no ConPTY) so this MAY start cleanly — but it reintroduces the exact startup-risk class the broker arm exists to avoid. **Treat as fallback only.**
- **(Recommended) Add `session_sid` as an ENABLED GROUP (not restricting SID)** on the duplicated primary token, BEFORE lowering integrity. An enabled group participates in the NORMAL access check, so the WFP CC-probe against `D:(A;;CC;;;<sid>)` GRANTS and the filter matches — with NO restricted-token double-check and NO startup-risk. Group injection on a primary token is done by building a `TOKEN_GROUPS` with `{Sid: session_sid, Attributes: SE_GROUP_ENABLED}` and calling `CreateRestrictedToken(token, 0, 0, null, 0, null, 0, null, &out)` is NOT a group-add. The actual group-add primitive is **`AdjustTokenGroups` cannot ADD groups** (only enable/disable existing). To ADD a brand-new SID to a primary token's group list, use **`CreateRestrictedToken` is the only documented in-place path that injects SIDs** — and it injects them as RESTRICTING SIDs, not normal groups. **Therefore the only Win32-supported way to make a NEW synthetic SID appear in a token's access check is as a restricting SID (WRITE_RESTRICTED) — option (Preferred) above.**

DESIGN DECISION (D1 final): There is **no API to add an arbitrary new enabled GROUP to an existing token**; `CreateRestrictedToken` (restricting SID) is the only mechanism, and `WRITE_RESTRICTED` is required to keep startup reads open. So the broker token = **`CreateRestrictedToken(WRITE_RESTRICTED, restricting-SID = session_sid)` applied to the duplicated token, THEN lower integrity to Low via the existing label step.** This is byte-for-byte the WriteRestricted shape PLUS the Low-IL label. The startup-risk question (does curl start under this on the broker path?) becomes the ONE empirical unknown to validate in the follow-up UAT — but it is the only correct shape. Mandatory-label NO_WRITE_UP write-deny is preserved by the Low-IL label; the DACL guard (already applied) supplies the writable-path grants for session_sid, which become OPERATIVE (intended parity, see D5).

### D2 — How WriteRestricted adds session_sid (mirror target)

`restricted_token.rs:34` `create_restricted_token_with_sid(sid)`:
`OpenProcessToken(DUP|QUERY|ASSIGN_PRIMARY)` → `ConvertStringSidToSidW(sid)` → `SID_AND_ATTRIBUTES{Sid, Attributes:0}` → `CreateRestrictedToken(cur, WRITE_RESTRICTED, 0,null,0,null, 1, &sid_restrict, &out)` → `LocalFree(sid)`. RAII `RestrictedToken` Drop closes the handle.

### D3 — Library change (`nono` crate, `sandbox/windows.rs`)

Add a sibling fn (do NOT break the parameterless one — LowIlPrimary fallback + tests + nono-cli legacy still use it):

```
pub fn create_low_integrity_primary_token_with_sid(session_sid: &str) -> Result<OwnedHandle>
```

Body = current `create_low_integrity_primary_token` body, but instead of `DuplicateTokenEx` → label, do: `OpenProcessToken` → `ConvertStringSidToSidW(session_sid)` (reject malformed → `NonoError::SandboxInit`) → `CreateRestrictedToken(cur, WRITE_RESTRICTED, 0,null,0,null, 1, &sid_restrict, &out)` → `LocalFree(sid)` → apply the EXISTING Low-IL mandatory-label block (`CreateWellKnownSid(WinLowLabelSid)` + `SetTokenInformation(TokenIntegrityLevel)`) to the restricted token. Return `OwnedHandle`. Keep RAII/handle safety (OwnedHandle Drop). Refactor the shared label-apply into a private helper so both fns stay byte-equivalent. The parameterless fn stays as-is (or delegates with `None`).

### D4 — Broker plumbing (3 edit sites)

1. **`launch.rs` no-PTY broker_args (L1816-1829):** push `--session-sid` + `config.session_sid` (the SAME string fed to the WFP service request). FAIL-CLOSED: this arm is only reached when `has_session_sid` is true, so the value is always present; assert/`ok_or` so a missing SID errors rather than silently spawning an unmatched child.
2. **`nono-shell-broker/src/main.rs` argv parser (L73-161):** add `--session-sid <value>` → `Option<String>` field on `BrokerArgs`; validate via `ConvertStringSidToSidW` (reject malformed → error). When `--no-pty` is set, `--session-sid` is REQUIRED (fail-closed: missing/invalid SID under no-PTY = hard error, never spawn).
3. **`nono-shell-broker/src/main.rs:235`:** when `session_sid` is present, call the new `nono::create_low_integrity_primary_token_with_sid(sid)`; else keep the parameterless fn (PTY/legacy path unchanged).

(The PTY broker_args at L1490 is intentionally NOT changed — PTY path waives per-session WFP, same as today.)

### D5 — THREAT REVIEW + FAIL-CLOSED CONTRACT

- **(a) No privilege/integrity escalation.** `session_sid` is a synthetic per-session SID in the Microsoft-reserved `S-1-5-117-*` range (`generate_session_sid`, random UUID-derived). It names no real account and appears on no system object's ACL except the per-run capability pipe and the per-run DACL grants nono itself adds. As a RESTRICTING SID under `WRITE_RESTRICTED` it can only NARROW write access (double-check), never widen it. The token's integrity stays **Low** (label applied AFTER the restricted-token build). VERDICT: no escalation.
- **(b) DACL grants become operative — INTENDED PARITY, not a new surface.** `mod.rs:334` already applies `AppliedDaclGrantsGuard::snapshot_and_apply(fs_policy, session_sid)` for any session_sid; today it is INERT on the broker arm (dacl_guard.rs L32-34). Injecting the SID makes those write-grants operative — but they grant write ONLY on paths that are ALREADY in the user's grant set AND already user-owned (`path_is_owned_by_current_user` gate), with `FILE_GENERIC_WRITE | DELETE` (not FullControl), `(OI)(CI)` inheritable, and REVOKED on Drop. This is the SAME grant the WriteRestricted arm already relies on for confined writes. VERDICT: intended parity; no new write surface beyond the existing capability grant set. (Note: this also makes confined WRITES work on the broker arm under WRITE_RESTRICTED — a functional REQUIREMENT, since WRITE_RESTRICTED would otherwise deny all confined writes.)
- **(c) FAIL-CLOSED CONTRACT.** If SID parse/injection fails (malformed SID, `ConvertStringSidToSidW`==0, `CreateRestrictedToken`==0) when `network.block` is set, the run MUST fail closed: return `NonoError::SandboxInit`, NEVER fall back to the parameterless (SID-less) token, NEVER spawn the child. Spawning an unmatched child = silent non-enforcement (the WFP filter installs but matches nothing) = the worst outcome (operator believes they are blocked; they are not). Enforce at: (1) launch.rs broker_args (SID must be present on this arm), (2) broker argv parser (`--no-pty` ⇒ `--session-sid` required), (3) broker token build (any FFI failure → propagate, do not degrade).

### D6 — Alternatives (evaluated)

- **WFP app-id scoping (`FWPM_CONDITION_ALE_APP_ID`):** matches by EXE path. Brittle: the confined child shares its exe with unconfined invocations (e.g. system `curl.exe`/`powershell.exe`), so the filter would block ALL instances system-wide, not just the confined run. Also the broker→child exe varies. REJECTED.
- **Integrity-level scoping:** WFP has no first-class IL condition; and other Low-IL processes on the box would collide. Not per-run. REJECTED.
- **Distinct logon session (`LogonUserW`/`S4U`):** heavyweight, needs credentials or `SeTcbPrivilege`, changes the broker trust model, and the per-session SID already gives per-run granularity. REJECTED as overkill.
- **CHOSEN: restricting-SID injection** — reuses the proven WriteRestricted+WFP path, is per-run unique, requires no new privileges, and unifies with the already-applied DACL guard. ACCEPTED.

### Implementation size: SMALL–MEDIUM

Files: (1) `crates/nono/src/sandbox/windows.rs` — new `_with_sid` fn + shared label helper + unit test; (2) `crates/nono-shell-broker/src/main.rs` — `BrokerArgs.session_sid` field, parser arm + validation, conditional token call, parse test; (3) `crates/nono-cli/src/exec_strategy_windows/launch.rs` — no-PTY broker_args push + fail-closed assert. ~3 source files + 2 tests (token-carries-SID unit test in nono; argv parse/reject test in broker). The ONE empirical unknown (does `curl`/`powershell` start under WRITE_RESTRICTED+Low-IL on the broker path?) requires a follow-up live Win11 UAT after implementation.

## Current Focus

- hypothesis: CONFIRMED + DESIGNED. Broker (BrokerLaunchNoPty) child token lacks session_sid; WFP ALE_USER_ID filter never matches. Fix = inject session_sid as a WRITE_RESTRICTED restricting SID into the broker's Low-IL primary token (new `nono::create_low_integrity_primary_token_with_sid`), thread `--session-sid` through launch.rs→broker argv→token build, fail-closed on any SID failure.
- next_action: implement per D3/D4 (separate session), then live Win11 UAT to validate startup-under-WRITE_RESTRICTED on the broker path. NO code applied this session (goal=find_root_cause_only).

## Evidence

- timestamp: 2026-06-02 — Live Win11: broker spawned Low-IL child (pid 34064), WFP activated (no error), curl returned external IP 68.237.8.196 exit 0 → NOT blocked.
- timestamp: 2026-06-02 — Static: nono-shell-broker/src/main.rs L235 `nono::create_low_integrity_primary_token()` (no sid arg); launch.rs L1816+ no-PTY broker_args = only --shell/--shell-arg/--no-pty/--inherit-handle/--cwd (no sid passed).
- timestamp: 2026-06-02 — Static: dacl_guard.rs L32-34 explicitly states the synthetic SID is on NO child token for BrokerLaunch*/LowIlPrimary arms (only WriteRestricted carries it); mod.rs:334 applies the guard for any session_sid (inert on broker arm today).
- timestamp: 2026-06-02 — Static: WFP filter scopes via FWPM_CONDITION_ALE_USER_ID + SD `D:(A;;CC;;;<sid>)` over session_sid (nono-wfp-service.rs L1247/L1326). CC = ADS_RIGHT_DS_CREATE_CHILD probe.
- timestamp: 2026-06-02 — Static: restricted_token.rs:34 + comment L82-89 = the WORKING reference; restricting-SID under WRITE_RESTRICTED keeps the WFP read-like ALE_USER_ID check matching. Single session_sid source = execution_runtime.rs:379, also fed to WFP request (network.rs:542).
- timestamp: 2026-06-02 — DESIGN produced: new `create_low_integrity_primary_token_with_sid`, `--session-sid` plumbing, fail-closed contract, threat review (no escalation; DACL parity intended), alternatives rejected (app-id/IL/logon-session). Impl size SMALL-MEDIUM.

## Eliminated

- WFP activation / marshaling / session-model as the cause — all fixed (62-05..62-09); FwpmFilterAdd0 now returns 0. The filter installs; it simply does not match the broker child's token.
- Adding session_sid as a plain ENABLED GROUP — REJECTED: no Win32 API adds an arbitrary NEW SID as a normal group to an existing token; `CreateRestrictedToken` (restricting SID) is the only injection path, hence WRITE_RESTRICTED is required.
