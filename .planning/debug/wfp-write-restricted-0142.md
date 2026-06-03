---
slug: wfp-write-restricted-0142
status: design_complete
trigger: "F-62-UAT-05 SC1 regression (live Win11, 2026-06-02): the 62-10 broker session_sid fix breaks confined child STARTUP. The fix put session_sid as the single RESTRICTING SID under WRITE_RESTRICTED on the broker's Low-IL primary token to make the WFP ALE_USER_ID filter matchable. On the installed v0.57.9 build (SHA256-confirmed = the 62-10 build) the broker spawns the Low-IL child (pid 36048) which then exits child_exit_code=3221225794 = 0xC0000142 = STATUS_DLL_INIT_FAILED. curl never runs (native binary — NOT a CLR issue). Phase 51 had specifically eliminated this exact 0xC0000142 class by routing the broker through a PLAIN Low-IL primary token (create_low_integrity_primary_token, no WRITE_RESTRICTED). So WRITE_RESTRICTED broadly breaks process startup on the broker path. The prior debug wfp-broker-token-no-sid D1 concluded restricting-SID-under-WRITE_RESTRICTED was 'the only Win32-supported way' to put session_sid on the token for WFP matching — if that universally breaks startup, the F-62-UAT-05 fix as designed is NOT viable."
created: 2026-06-02
updated: 2026-06-02
phase: 62
requirements:
  - REQ-WFP-01
supersedes_design: wfp-broker-token-no-sid (D1 conclusion falsified by this UAT)
goal: find_root_cause_only
---

# Debug: WRITE_RESTRICTED broker token → 0xC0000142 STATUS_DLL_INIT_FAILED (62-10 SC1 regression)

## Symptoms

**Expected (REQ-WFP-01 SC1):** `nono run --profile claude-code --block-net --allow-cwd -- curl.exe -sS -m 5 https://api.ipify.org` (from a profile-covered cwd, %USERPROFILE%\.claude) → the confined Low-IL child STARTS, and its outbound TCP is BLOCKED by the kernel WFP ALE_USER_ID filter (no external IP; "BLOCKED"/timeout).

**Actual (live Win11, 2026-06-02, installed nono.exe SHA256 2F910D9D…E40A + nono-wfp-service.exe SHA256 E3932131…63A6 == the 62-10 v0.57.9 build, both Authenticode Valid):**
```
broker: Low-IL primary token constructed
broker: spawned Low-IL child child_pid=36048
broker: child exited child_exit_code=3221225794   # = 0xC0000142 = STATUS_DLL_INIT_FAILED
```
curl never runs. This is a STARTUP failure (DLL initialization), NOT a network block. SC1 cannot be evaluated because the child crashes at load.

**Error code:** 3221225794 decimal = 0xC0000142 = STATUS_DLL_INIT_FAILED (the loader failed to initialize one or more DLLs for the new process).

**Timeline:** Introduced by 62-10 (executed + installed this session). Before 62-10, the broker arm (BrokerLaunchNoPty) used the parameterless `create_low_integrity_primary_token` (plain Low-IL primary token, NO WRITE_RESTRICTED) and the child started fine (but was NOT blocked — the F-62-UAT-05 match gap). Phase 51 (v2.7) explicitly chose the plain-Low-IL-broker-token path to ELIMINATE this same 0xC0000142 class for claude.exe (see memory project_v27_opened: claude.exe 0xC0000142/STATUS_DLL_INIT_FAILED regression fixed by routing through a no-PTY Low-IL broker token instead of WriteRestricted). 62-10 re-added WRITE_RESTRICTED on top of that token → 0xC0000142 returns.

**Reproduction:** Install the v0.57.9 machine MSI; from %USERPROFILE%\.claude run the SC1 command above. Deterministic (curl is native, so it is not workload-specific — WRITE_RESTRICTED breaks the loader broadly).

## Confirmed facts (from this session)

1. Installed binaries SHA256-match the 62-10 v0.57.9 build exactly (nono.exe 2F910D9D…E40A; nono-wfp-service.exe E3932131…63A6). So the 0xC0000142 IS the new WRITE_RESTRICTED token path, not a stale install.
2. 62-10's `create_low_integrity_primary_token_with_sid` (crates/nono/src/sandbox/windows.rs L702): OpenProcessToken → ConvertStringSidToSidW(session_sid) → CreateRestrictedToken(cur, WRITE_RESTRICTED, 0,null,0,null, 1, &sid_restrict, &out) → apply_low_il_label. This token is what the broker now assigns the child.
3. The parameterless `create_low_integrity_primary_token` (L536, the Phase 51 WORKING path): OpenProcessToken → DuplicateTokenEx(TokenPrimary) → apply_low_il_label. NO CreateRestrictedToken. Children start fine under it.
4. The ONLY delta between "starts" and "0xC0000142" is the CreateRestrictedToken(WRITE_RESTRICTED, restricting SID) step.
5. WFP filter scopes via FWPM_CONDITION_ALE_USER_ID + SD `D:(A;;CC;;;<session_sid>)` (nono-wfp-service.rs sid_to_security_descriptor L1347 / add_policy_filter L1426). For the filter to MATCH, the connection token must grant CC to session_sid in a WFP access check.
6. `generate_session_sid` (restricted_token.rs L21) emits `S-1-5-117-{u32}-{u32}-{u32}-{u32}` from a UUIDv4. **The `S-1-5-117` range is NOT Microsoft-reserved** (the prior design's D5(a) claim is imprecise); it is simply an authority/sub-authority shape that names no real account and appears on NO system object DACL. This is the load-bearing fact behind BOTH the loader failure AND the write-deny behaviour.
7. The WFP service ALREADY imports `FWPM_CONDITION_ALE_APP_ID` (used as the non-SID fallback at L1437). `FWPM_CONDITION_ALE_PACKAGE_ID` is NOT yet referenced.

## ROOT CAUSE (CONFIRMED)

**The 0xC0000142 is the documented, expected behaviour of `CreateRestrictedToken(WRITE_RESTRICTED, restricting_sid = <a SID present on no object DACL>)`, NOT a broker-path quirk.** The prior debug doc treated "does the loader survive WRITE_RESTRICTED on the broker path?" as the single empirical unknown. It does NOT survive. Here is the precise mechanism:

### Why WRITE_RESTRICTED denies the loader (the corrected model)

`WRITE_RESTRICTED` does NOT mean "the restricted-SID list is consulted only for `FILE_WRITE_DATA`-style requests and skipped for everything else." The documented semantics (CreateRestrictedToken / access-check algorithm) are:

- For an access request whose desired mask contains ONLY non-write rights, the SECOND (restricting-SID) access check is **skipped** → the request is granted on the normal-SID check alone.
- For an access request whose desired mask contains ANY **write-class** right, BOTH checks run, and the restricting-SID check must ALSO grant → because `S-1-5-117-*` is on no DACL, the request is **denied**.

The trap in the 62-10 design (and restricted_token.rs L82-89) is the assumption that "DLL loads, section mappings, registry traversal" are pure-read operations. They are **not**, for the process loader:

1. **Image section mapping / copy-on-write data pages.** When `ntdll!LdrpMapDllNtFileName` → `NtCreateSection`/`NtMapViewOfSection` maps a DLL's writable (`.data`/`.bss`) segments as copy-on-write, the mapping request carries `SECTION_MAP_WRITE` (a WRITE-class right). Under WRITE_RESTRICTED this triggers the second access check against the section object's DACL, which does not list `S-1-5-117-*` → ACCESS_DENIED inside `DllMain` → the loader reports **STATUS_DLL_INIT_FAILED (0xC0000142)** rather than a clean STATUS_ACCESS_DENIED at spawn.
2. **`\BaseNamedObjects` and per-session named-object directories.** CRT / KernelBase init (`DllMain` of kernelbase, ucrtbase, etc.) opens or creates synchronization objects (events, mutants, sections such as the heap's `\Sessions\<n>\BaseNamedObjects\...`) with create/map-write rights. Those create/map-write requests are WRITE-class → second check → denied. (This is the same family as the CLR `BaseNamedObjects` failure from Phase 60 F-60-UAT-05, but here it bites *native* binaries too because the loader itself needs writable shared objects.)
3. **Process/thread default-DACL writes during init.** Establishing the process heap and TEB/PEB-adjacent structures, and CSRSS-side bookkeeping, perform write-class operations checked against the (now restricting) token.

Because these are loader/`DllMain`-time operations that happen to EVERY process regardless of language runtime, the failure is **broad and deterministic** (curl, powershell, claude.exe all die), **integrity-label-independent** (the Low-IL label is applied AFTER and is orthogonal), and **cannot be narrowed away** by tweaking which user paths are DACL-granted — the denied objects are system loader/section/named-object kernel objects that nono cannot and must not grant `S-1-5-117-*` write access to.

**Conclusion:** A single-restricting-SID token where the SID is absent from all object DACLs cannot host normal process startup. `WRITE_RESTRICTED` narrows the blast radius from "everything" (the Flags=0 STATUS_ACCESS_DENIED 0xC0000022 of the original Phase 13 bug) to "write-class operations only" — but the loader's section/named-object init IS write-class, so it still dies, just with a different status code (0xC0000142 vs 0xC0000022). The 62-10/D1 design is **not viable as specified.** ROOT CAUSE CONFIRMED.

## The falsified D1 claim (corrected)

Prior debug `wfp-broker-token-no-sid` D1 (lines 51-53, 112) asserted:

> "There is no Win32 API to add an arbitrary new enabled GROUP to an existing token; CreateRestrictedToken (restricting SID) is the only injection path."

**This is FALSE, and it is the root error that drove the failed design.** It conflates two distinct facts:

- TRUE: `AdjustTokenGroups` can only **enable/disable groups that already exist** in the token; it cannot ADD a brand-new SID. `CreateRestrictedToken` is the only API that injects a new SID **into an EXISTING token in place**, and it injects it as a RESTRICTING SID (or as a disabling SID), never as a normal enabled group.
- FALSE (the unstated leap): "...therefore the only way to get session_sid into ANY token's access check is as a restricting SID." This ignores **from-scratch token construction** and **AppContainer/package tokens**, which CAN carry an arbitrary SID as a normal access-check participant WITHOUT WRITE_RESTRICTED.

Win32 primitives that DO put an arbitrary SID into a token's NORMAL access check (no restricting-SID, no WRITE_RESTRICTED double-check):

- **`NtCreateLowBoxToken`** (the documented kernel primitive behind AppContainer; ntdll, callable). It takes a base token, an AppContainer **package SID**, and an array of **capability SIDs**, and produces a "lowbox" token in which the package SID and capability SIDs are present as NORMAL, enabled access-check participants. These SIDs gate the NORMAL access check (so a DACL `(A;;...;;;<capSid>)` grants), and they do NOT impose a restricting-SID second check on unrelated objects — lowbox tokens start processes cleanly (this is exactly how every Store/UWP app and every Chromium renderer launches).
- **`CreateAppContainerProfile` / `DeriveAppContainerSidFromAppContainerName`** + **`DeriveCapabilitySidsFromName`**: the higher-level path to obtain a stable per-app package SID and arbitrary named capability SIDs, which are then fed to `NtCreateLowBoxToken` (or to `STARTUPINFOEX` `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` with `SECURITY_CAPABILITIES`).
- **`CreateProcess*` with `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`** (`SECURITY_CAPABILITIES{ AppContainerSid, Capabilities[] }`): the OS builds the lowbox token for you at spawn time. The child runs in an AppContainer; its token carries the AppContainer SID + capability SIDs as normal members.

So the design space the prior debug closed off is in fact OPEN. The corrected statement is: *to carry a WFP-matchable, non-restricting SID, do not mutate an existing token — build a NEW (lowbox/AppContainer) token, OR scope WFP by a condition other than ALE_USER_ID.*

## WFP matchability of the alternatives (does the filter still match?)

`FWPM_CONDITION_ALE_USER_ID` is a SECURITY_DESCRIPTOR-typed condition: BFE runs an access check of the **connection's token** against the filter's SD `D:(A;;CC;;;<sid>)`, requesting the `CC` right (ADS_RIGHT_DS_CREATE_CHILD, 0x1). The check GRANTS iff the connection token contains `<sid>` as an access-check participant that the DACL ACE grants `CC` to. The check is a NORMAL access check on a read-like right — so:

- A **restricting SID** satisfies it (this is why 62-10 matched in theory) — but it kills startup.
- A **normal enabled group / AppContainer package SID / capability SID** ALSO satisfies it, because it participates in the normal access check exactly as a real group would. **There is no requirement that the matched SID be a RESTRICTING Sid** — ALE_USER_ID does not consult the restricting-SID list specifically; it consults the token's effective groups. This is the crux the prior design missed.

Therefore an AppContainer/capability SID injected via a lowbox token is **both** WFP-ALE_USER_ID-matchable **and** startup-safe. There is also a dedicated, even cleaner condition:

- **`FWPM_CONDITION_ALE_PACKAGE_ID`** — a SID-typed (FWP_SID, FWP_BYTE_BLOB) condition that matches the connection's **AppContainer package SID** directly with `FWP_MATCH_EQUAL`. If the child runs in a per-run AppContainer, this matches the package SID exactly, with no SD/access-check indirection. (Requires the child to be an AppContainer process; the unconfined system curl/powershell are NOT in an AppContainer, so there is no cross-instance collision — this cleanly solves the ALE_APP_ID over-broad rejection from the prior D6.)

## DESIGN (deliverable — find_root_cause_only, NO code applied)

### D1 — Recommended fix: per-run AppContainer (lowbox) child, WFP-scoped by package SID

Replace the "session_sid as a restricting SID on a Low-IL primary token" model with a **per-run AppContainer** model:

1. **Per-run package SID.** Derive a per-run AppContainer SID. Two options:
   - (a) `DeriveAppContainerSidFromAppContainerName(<per-run-name>)` where `<per-run-name>` is `nono.session.<uuid>` (the same UUID that today seeds `generate_session_sid`). No profile registration needed for a transient container; the SID is deterministic from the name.
   - (b) `CreateAppContainerProfile("nono.session.<uuid>", ...)` for a registered profile (cleaned up with `DeleteAppContainerProfile` on Drop). (a) is lighter and sufficient for WFP scoping; prefer (a) unless profile registration is needed for storage isolation.
   The package SID has shape `S-1-15-2-...` (APPLICATION_PACKAGE_AUTHORITY) — a REAL, OS-understood SID class (unlike the synthetic `S-1-5-117-*`), so the loader's lowbox path treats it correctly.
2. **Child token.** The broker builds the child via `CreateProcessAsUserW`/`CreateProcessW` with a `STARTUPINFOEX` carrying `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` = `SECURITY_CAPABILITIES{ AppContainerSid: <package SID>, CapabilitySidCount: 0 }` (no extra capabilities — empty capability set is MORE restrictive, which is the security-correct default; add capability SIDs ONLY for resources the child must reach, e.g. none for the curl block test). This produces a lowbox token at spawn; **no CreateRestrictedToken, no WRITE_RESTRICTED, no loader-init denial.** AppContainer processes start cleanly by design.
   - NOTE: AppContainer already implies a Low-equivalent isolation (AppContainer SID < Low IL on the trust ladder for many objects) AND a `\Sessions\<n>\AppContainerNamedObjects\<pkgSid>` private named-object namespace — which is the structural reason the BaseNamedObjects/section writes SUCCEED (the OS gives the container its own writable private kernel-object directory) where the synthetic-restricting-SID token FAILED.
3. **Mandatory label.** Preserve NO_WRITE_UP. AppContainer tokens get an integrity label too; explicitly set it to Low via the existing `apply_low_il_label` helper (or rely on AppContainer's own write-up isolation — but set Low explicitly for defence-in-depth and parity with the documented model). Verify the label survives the lowbox build; if `NtCreateLowBoxToken` resets it, re-apply after.
4. **WFP request.** Switch the per-run WFP filter from ALE_USER_ID-over-synthetic-SID to **ALE_PACKAGE_ID == <package SID>** (FWP_SID-typed condition, FWP_MATCH_EQUAL). Single source: the per-run package SID replaces `session_sid` as the value carried from `execution_runtime.rs` → BOTH the token build (the broker's SECURITY_CAPABILITIES) AND the WFP service request (`network.rs` → `request.package_sid`). Keep the single-source invariant.
   - Fallback option if ALE_PACKAGE_ID is awkward to plumb: keep ALE_USER_ID with SD `D:(A;;CC;;;<packageSid>)` — the package SID is a normal access-check participant, so the existing `sid_to_security_descriptor` + ALE_USER_ID path works UNCHANGED, just fed the package SID instead of the synthetic SID. This minimises WFP-service churn (no new condition type) while still being startup-safe. RECOMMENDED as the first increment because it reuses the proven, already-shipped ALE_USER_ID marshaling (62-08 SD→FWP_BYTE_BLOB) verbatim.

### D2 — Concrete Win32 mechanics

Token build (broker, replacing `create_low_integrity_primary_token_with_sid`):
```
DeriveAppContainerSidFromAppContainerName(L"nono.session.<uuid>", &pPackageSid)   // S-1-15-2-...
// (no extra capabilities)
SECURITY_CAPABILITIES caps = { .AppContainerSid = pPackageSid, .Capabilities = NULL, .CapabilityCount = 0, .Reserved = 0 };
STARTUPINFOEX si = {0};
InitializeProcThreadAttributeList(.., 1, ..);
UpdateProcThreadAttribute(.., PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, &caps, sizeof(caps), ..);
CreateProcessW(/* or AsUser with current token */, .., EXTENDED_STARTUPINFO_PRESENT, .., &si.StartupInfo, &pi);
// optional: open child token, apply_low_il_label() for explicit Low-IL parity
FreeSid(pPackageSid);  // via RtlFreeSid / appropriate free
```
WFP request: carry `package_sid` (the SDDL string of `pPackageSid`, obtained via `ConvertSidToStringSidW`) end-to-end; the WFP service uses either ALE_PACKAGE_ID (FWP_SID over the raw SID bytes) or the existing ALE_USER_ID SD path with the package SID string.

windows-sys availability: the workspace pins windows-sys 0.45 + 0.52 (Cargo.lock). `DeriveAppContainerSidFromAppContainerName`, `SECURITY_CAPABILITIES`, `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`, and `FWPM_CONDITION_ALE_PACKAGE_ID` are all in `Win32::Security::*` / `Win32::System::Threading::*` / `Win32::NetworkManagement::WindowsFilteringPlatform::*`. Confirm the exact feature flags during impl; if a symbol is missing from the pinned windows-sys, declare a minimal `extern "system"` shim (the project already does targeted FFI declarations).

### D3 — Alternatives weighed (now that restricting-SID is FALSIFIED)

| Option | Starts cleanly? | WFP-matchable per-run? | Privilege needed | Verdict |
|---|---|---|---|---|
| (62-10) Synthetic SID as restricting SID + WRITE_RESTRICTED | **NO** (0xC0000142) | yes (if it started) | none | **REJECTED — falsified by UAT** |
| Synthetic SID as restricting SID, Flags=0 | NO (0xC0000022, Phase 13 bug) | yes | none | REJECTED (worse) |
| Add well-known SIDs (Everyone/RESTRICTED) to the restricting set to relax writes | partial/fragile — would re-open broad write access, defeating the sandbox; and still no clean loader guarantee | yes | none | REJECTED (security regression: relaxing the restricting set widens write access) |
| **Per-run AppContainer, WFP via ALE_USER_ID(packageSid) or ALE_PACKAGE_ID** | **YES** (designed-for) | **YES** (package SID is per-run unique, normal access-check participant) | none (no SeTcb/credentials) | **RECOMMENDED** |
| Distinct logon session (LogonUserW/S4U) | yes | yes (real logon SID) | SeTcbPrivilege / credentials | REJECTED (privilege + trust-model change) |
| WFP ALE_APP_ID (exe path) | yes (no token change) | NO — collides with unconfined system curl/powershell sharing the exe | none | REJECTED (over-broad, unchanged from prior D6) |

The AppContainer path is the unique option that satisfies ALL of: clean startup, per-run-unique WFP match, no new privilege, no broker-trust-model change, and it reuses the already-proven SD-over-ALE_USER_ID marshaling if the FWP_SID/ALE_PACKAGE_ID route is deferred.

### D4 — THREAT REVIEW + FAIL-CLOSED CONTRACT

- **(a) No privilege/integrity escalation.** AppContainer is strictly MORE confined than a plain Low-IL token (private object namespace, capability-gated resource access, deny-by-default for capabilities not granted). The package SID is `S-1-15-2-*` (a real OS SID class) and the capability set is EMPTY in the block test → the child can reach nothing it could not already reach, minus AppContainer's additional restrictions. The Low-IL mandatory label is preserved (re-applied if needed). VERDICT: no escalation; net tightening.
- **(b) Writable-path grants.** Under the synthetic-SID model, confined writes worked only because `AppliedDaclGrantsGuard` granted `S-1-5-117-*` write on user-owned grant paths. Under AppContainer the child's write capability is governed by the AppContainer SID + its object namespace; for the confined writable paths (cwd grants), the broker must grant the **package SID** (or the relevant capability SID) write on those user-owned paths — i.e. retarget `AppliedDaclGrantsGuard` from `session_sid` to the package SID. Same mechanism (`grant_sid_write_on_path` / `AppliedDaclGrantsGuard`), different SID. The grant stays scoped to already-user-owned grant-set paths, `FILE_GENERIC_WRITE | DELETE`, `(OI)(CI)`, REVOKED on Drop. VERDICT: intended parity, no new write surface. (This is the Phase 60 F-60-UAT-04 mechanism, reused.)
- **(c) FAIL-CLOSED CONTRACT (UNCHANGED INVARIANT).** When `network.block` is set, ANY failure in deriving the package SID, building the SECURITY_CAPABILITIES/lowbox token, threading the package SID to the WFP request, or applying the Low-IL label MUST fail closed: return `NonoError::SandboxInit`, NEVER fall back to a plain (non-AppContainer or SID-less) token, NEVER spawn the child. A child spawned without the per-run package SID that the WFP filter keys on = silent non-enforcement = worst outcome. Enforce at: (1) execution_runtime/launch (package SID must be present on this arm), (2) broker argv (require `--package-sid` / the AppContainer name under `--no-pty`), (3) broker token build (any FFI failure → propagate), (4) WFP request (must carry the same package SID — single source).
- **(d) Single-source invariant preserved.** One per-run identifier (the package SID, derived from the existing per-run UUID) flows to BOTH the broker token build AND the WFP service request, exactly as `session_sid` does today (execution_runtime.rs → token arm + network.rs request). Do not generate two.
- **(e) Cleanup.** If `CreateAppContainerProfile` (option 1b) is used, register a Drop/guard that calls `DeleteAppContainerProfile` so per-run profiles do not accumulate. Option 1a (Derive-only, no profile) needs no cleanup — PREFER it.

### D5 — Residual unknowns to validate in the follow-up live UAT

1. Does the AppContainer child start cleanly under the BROKER spawn path (Medium-IL broker self-degrade → CreateProcessW with SECURITY_CAPABILITIES, anonymous-pipe stdio, no ConPTY)? Expected YES (this is the canonical AppContainer launch shape), but it is the one thing only live Win11 can confirm — exactly the role the prior UAT played for WRITE_RESTRICTED.
2. Does `FWPM_CONDITION_ALE_USER_ID` with SD `D:(A;;CC;;;<packageSid>)` match an AppContainer child's outbound connection? (First increment — reuses existing marshaling.) If it does NOT match (some BFE builds key package identity only via ALE_PACKAGE_ID), fall back to the ALE_PACKAGE_ID FWP_SID condition (D1 step 4).
3. Do confined WRITES to the cwd grant paths still succeed once the DACL guard is retargeted from session_sid to the package SID? (Phase 60 F-60-UAT-04 says yes for a normal SID; confirm for the package SID class.)
4. Does the Low-IL mandatory label survive / need re-applying after the lowbox build?

### Implementation size: MEDIUM

Files: (1) `crates/nono/src/sandbox/windows.rs` — replace/augment `create_low_integrity_primary_token_with_sid` with an AppContainer-capable spawn helper (or expose package-SID derivation + SECURITY_CAPABILITIES build); (2) `crates/nono-shell-broker/src/main.rs` — accept the AppContainer name / package SID, build SECURITY_CAPABILITIES, spawn via STARTUPINFOEX; (3) `crates/nono-cli/src/exec_strategy_windows/launch.rs` — derive the package SID, thread it on the broker-no-PTY arm, fail-closed; (4) `crates/nono-cli/src/bin/nono-wfp-service.rs` — accept the package SID (reuse ALE_USER_ID SD path first; add ALE_PACKAGE_ID if needed); (5) retarget `AppliedDaclGrantsGuard` from session_sid to the package SID. Net larger than the 62-10 SMALL fix, but it is the only viable shape. The synthetic `S-1-5-117-*` session_sid path (restricted_token.rs WriteRestricted arm) can remain for the non-broker WriteRestricted arm OR be retired — decide during planning; it is independent of this fix.

## Specialist Review

(none requested executed this session — design-only debug; no live elevated UAT available. The deliverable is the code-ready design above, to be implemented + UAT'd in a follow-up phase plan, mirroring how wfp-broker-token-no-sid fed 62-10.)

## Current Focus

- hypothesis: CONFIRMED. The 0xC0000142 is the documented behaviour of CreateRestrictedToken(WRITE_RESTRICTED, restricting_sid absent-from-all-DACLs): the loader's image-section copy-on-write mapping and BaseNamedObjects/section creation are WRITE-class operations that trigger the restricting-SID second access check, which denies (the synthetic S-1-5-117-* is on no object DACL) → STATUS_DLL_INIT_FAILED inside DllMain. Broad, deterministic, integrity-independent, un-narrowable. The 62-10/D1 design is NOT viable. The D1 premise ("restricting SID is the ONLY way to put a WFP-matchable SID on a token") is FALSE — from-scratch AppContainer/lowbox tokens (NtCreateLowBoxToken / SECURITY_CAPABILITIES) carry an arbitrary package/capability SID as a NORMAL access-check participant that BOTH starts cleanly AND satisfies the ALE_USER_ID / ALE_PACKAGE_ID match.
- next_action: implement D1–D4 (per-run AppContainer child, WFP scoped by package SID via the existing ALE_USER_ID SD path first), then live Win11 UAT to validate D5's four unknowns. NO code applied this session (goal=find_root_cause_only).

## Evidence

- timestamp: 2026-06-02 — Live Win11: installed v0.57.9 (SHA-confirmed 62-10 build) broker spawned Low-IL child pid 36048 → exit 0xC0000142 STATUS_DLL_INIT_FAILED; curl never ran. Startup failure, not a block.
- timestamp: 2026-06-02 — Static: the only token-build delta vs the Phase 51 working path is CreateRestrictedToken(WRITE_RESTRICTED, single restricting SID = session_sid) in create_low_integrity_primary_token_with_sid (windows.rs L702) vs plain DuplicateTokenEx in create_low_integrity_primary_token (L536).
- timestamp: 2026-06-02 — Memory project_v27_opened: Phase 51 fixed the SAME claude.exe 0xC0000142 by routing the broker through the no-PTY Low-IL primary token INSTEAD OF WriteRestricted. 62-10 re-introduced WRITE_RESTRICTED → regression. Independent corroboration that WRITE_RESTRICTED, not the broker plumbing, is the startup-breaking factor.
- timestamp: 2026-06-02 — Win32 semantics: WRITE_RESTRICTED runs the restricting-SID second access check for any WRITE-class desired access. The loader's writable image-section (copy-on-write) mapping carries SECTION_MAP_WRITE, and CRT/KernelBase DllMain creates/maps writable named objects under \BaseNamedObjects — both WRITE-class → denied against S-1-5-117-* (on no DACL) → STATUS_DLL_INIT_FAILED. Confirms the mechanism; this is NOT a read-only operation as restricted_token.rs L82-89 assumed.
- timestamp: 2026-06-02 — Win32 token-construction: the prior D1 "only via restricting SID" claim is falsified by NtCreateLowBoxToken / CreateAppContainerProfile / DeriveAppContainerSidFromAppContainerName / SECURITY_CAPABILITIES — these create a NEW token carrying an arbitrary package/capability SID as a NORMAL enabled access-check participant (the launch mechanism of all UWP/Store apps and Chromium renderers), which starts cleanly AND is WFP-matchable (ALE_USER_ID SD access-check OR the dedicated FWP_SID ALE_PACKAGE_ID condition).
- timestamp: 2026-06-02 — DESIGN produced: per-run AppContainer child (Derive package SID from per-run UUID → SECURITY_CAPABILITIES → STARTUPINFOEX spawn), WFP scoped by the package SID (ALE_USER_ID SD path reused first, ALE_PACKAGE_ID as the clean fallback), DACL guard retargeted to the package SID, fail-closed + single-source invariants preserved, threat review (net tightening, no escalation, no new privilege). Impl size MEDIUM. Four live-UAT unknowns enumerated (D5).

## Eliminated

- Stale/old binary as the cause — ELIMINATED: installed SHA256 matches the 62-10 v0.57.9 build exactly.
- CLR/.NET-specific init as the cause — ELIMINATED: curl.exe is a native binary and still fails 0xC0000142 under WRITE_RESTRICTED.
- WFP activation / filter-install marshaling (F-62-UAT-01..04) — ELIMINATED: those are fixed (62-05..62-09); this is a child-STARTUP failure upstream of any block evaluation.
- "WRITE_RESTRICTED leaves all reads / the loader open, so the broker path will start cleanly" (the 62-10/D1 single empirical unknown) — ELIMINATED: the loader's section-map-write and BaseNamedObjects creation are WRITE-class, denied against the synthetic SID → 0xC0000142. WRITE_RESTRICTED does NOT keep the loader open for a SID that is on no object DACL.
- "Restricting SID is the only Win32 way to make session_sid WFP-matchable" (the D1 design premise) — ELIMINATED: AppContainer/lowbox tokens carry an arbitrary SID as a normal access-check participant, both startup-safe and WFP-matchable.
- WFP ALE_APP_ID (exe-path) scoping — ELIMINATED (re-confirmed): the confined child shares its exe with unconfined system curl/powershell, so an app-id filter blocks all instances system-wide. AppContainer package-SID scoping is the per-run-unique replacement.

## UAT FOLLOW-UP (2026-06-03): 0xC0000142 FIXED; new finding = AppContainer principal lacks read/traverse on cwd

Installed v0.57.10 (62-12 AppContainer build). Live SC1 re-run RESULT:
- `broker: token/AppContainer setup complete app_container=true` — the lowbox token + SECURITY_CAPABILITIES build OK.
- **0xC0000142 STATUS_DLL_INIT_FAILED is GONE** (debug D5 #1 core premise CONFIRMED: AppContainer startup path is viable).
- NEW failure: `CreateProcessW (AppContainer) failed (GetLastError=2)` = ERROR_FILE_NOT_FOUND.

### Confirmed root cause (verified, not hypothesized)
The confined child now runs as the PACKAGE SID (member of ALL APPLICATION PACKAGES), NOT the user. Verified evidence:
1. cwd = `C:\Users\OMack\.claude` (launch.rs normalize_windows_launch_path strips `\?\`, so it is a plain path — the verbatim-prefix theory is FALSE).
2. `Get-Acl C:\Users\OMack\.claude` grants ONLY SYSTEM, Administrators, OMack — NO ALL APPLICATION PACKAGES / S-1-15-2 ACE. An empty-capability AppContainer has NO access to it.
3. nono's package-SID DACL grant is WRITE-ONLY: SESSION_SID_WRITE_MASK = FILE_GENERIC_WRITE | DELETE = 0x00130116 (windows.rs L1220-1223) — NO FILE_TRAVERSE (0x20) / FILE_LIST_DIRECTORY (0x1).
4. curl.exe IS in System32 and grants ALL APPLICATION PACKAGES ReadAndExecute (so the EXE is reachable) — ruling out the exe-access theory; the failure is the CWD.
A process cannot have a current directory its token cannot traverse → CreateProcessW returns ERROR_FILE_NOT_FOUND.

Why it worked under the old Low-IL token: that token is the USER's identity (integrity-lowered); the user owns `.claude`, so traverse/read succeeded. Reads "just worked" under Low-IL (read-down allowed; NO_WRITE_UP only blocks writes), which is why the DACL guard only ever granted WRITE. AppContainer changes the PRINCIPAL, so reads AND traversal now also need explicit grants.

### Design implication (the real cost of the AppContainer model)
Every path the confined child must READ or TRAVERSE — the cwd, the tool's exe dir (if outside System32), its DLLs, config, node_modules (for claude.exe), etc. — must be granted to the package SID (or ALL APPLICATION PACKAGES). The current write-only grant is insufficient. The capability set already enumerates the r+w paths; the fix is to grant the package SID the FULL access matching each capability's AccessMode, with directory TRAVERSE, instead of write-only.

### Proposed fix (62-12 follow-up)
Map each capability to a package-SID grant mask:
- Read    → FILE_GENERIC_READ | FILE_TRAVERSE  (read + list + traverse; execute for exe dirs)
- Write   → FILE_GENERIC_WRITE | DELETE | FILE_TRAVERSE  (traverse needed to reach the file)
- ReadWrite → READ|WRITE|DELETE|TRAVERSE
Add a package-SID READ/TRAVERSE grant primitive alongside grant_sid_write_on_path (currently write-only), thread the AccessMode through AppliedDaclGrantsGuard, and ensure all grants revert on Drop. MINIMAL unblock for SC1 = grant the cwd read+traverse; FULL model = grant every capability path per its mode (needed for claude.exe). Same user-owned-path gate, inheritable, Drop-revoke. Fail-closed unchanged.
This is debug D5 #3 expanded from "do writes work" to "the AppContainer needs explicit grants for ALL access (read+traverse+write), because it is a different principal than the user."

### MINIMAL FIX APPLIED (2026-06-03, commit c3d7644f) — cwd read+traverse
Broadened SESSION_SID_WRITE_MASK from FILE_GENERIC_WRITE|DELETE to
FILE_GENERIC_READ|FILE_GENERIC_WRITE|FILE_EXECUTE|DELETE (0x1301BF) so the package SID can READ + TRAVERSE its
WRITABLE grant paths (incl. the cwd). Inert on the WriteRestricted arm. Build + DACL tests + clippy green. Rebuilt as
v0.57.11. NEXT UAT GOAL = the real unknown D5 #2: does the WFP ALE_USER_ID filter (SD D:(A;;CC;;;<packageSid>)) MATCH
the AppContainer child's outbound connection? Re-run SC1: (a) curl should now START (no ERROR_FILE_NOT_FOUND); (b)
observe BLOCK (no IP / timeout) = SC1 PASS, or PRINTS IP = ALE_USER_ID does not match AppContainer connections →
implement the ALE_PACKAGE_ID FWP_SID condition fallback (D1 step 4). Read-only grant paths (for claude.exe) remain the
deferred full read-grant model.

### ANCESTOR-TRAVERSE WALL (2026-06-03) — the deeper AppContainer problem
After the leaf cwd read+traverse grant (v0.57.11), CreateProcessW (AppContainer) STILL failed ERROR_FILE_NOT_FOUND
(operator's run; binary identity unconfirmed — operator then UNINSTALLED nono, so retroactive hashing is impossible).
Verified ACL evidence: `C:\Users\OMack`, `C:\Users`, and `C:\Users\OMack\.claude` ALL lack any ALL APPLICATION
PACKAGES / S-1-15-2 ACE. To set its current directory to the profile-deep cwd, the AppContainer must TRAVERSE every
ancestor (C:\, C:\Users, C:\Users\OMack); granting only the leaf `.claude` is insufficient if the lowbox token lacks
bypass-traverse (SeChangeNotifyPrivilege) OR the ancestors are not granted.

ROOT PROBLEM (architectural, not a one-line bug): the AppContainer child is a DIFFERENT principal (package SID) with
ZERO inherent access to the user profile, where BOTH the cwd AND (for the real target, claude.exe) all the tool's
files live. Making it work requires, at minimum: package-SID TRAVERSE on the cwd ancestor chain, package-SID READ on
every read path the tool touches (the deferred read-grant model), and likely bypass-traverse handling — a large,
broadening grant surface for an arbitrary user tool. AND the make-or-break question — does WFP ALE_USER_ID/ALE_PACKAGE_ID
actually MATCH an AppContainer connection (D5 #2) — is STILL UNVALIDATED because the child has never successfully
started. Decision point: push the full grant model to finally test WFP-match, vs. validate WFP-match in isolation
first (cheap experiment) before investing, vs. reconsider the approach. Recorded for the next session.
