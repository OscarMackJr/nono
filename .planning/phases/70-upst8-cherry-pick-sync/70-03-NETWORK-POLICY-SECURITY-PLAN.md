---
phase: 70-upst8-cherry-pick-sync
plan: 03
type: execute
wave: 2
depends_on: [70-01]
files_modified:
  - crates/nono-cli/data/network-policy.json
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/filter.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono/src/net_filter.rs
autonomous: true
requirements: [UPST8-02]
must_haves:
  truths:
    - "Embedded network policy profiles (google-ai, developer, codex, claude-code) no longer include credentials by default"
    - "When network.block (or --block-net) is set, the proxy host filter denies any host not in the explicit allowlist (deny-by-default, not allow-all fallback)"
    - "ProxyConfig.strict_filter defaults to false (backward compatible); flips to true only when block was requested"
    - "HostFilter::new_strict() denies on an empty allowlist"
    - "The fork's RouteStore/CredentialStore decoupling (route.rs + credential.rs) is preserved — no regression to the pre-Phase-56 allow-all-credential path"
    - "Each cherry-picked commit carries a verbatim 6-line D-19 trailer and DCO Signed-off-by"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
  artifacts:
    - path: "crates/nono-cli/data/network-policy.json"
      provides: "Embedded profiles with credentials arrays removed"
      contains: "google-ai"
    - path: "crates/nono-cli/src/network_policy.rs"
      provides: "Updated tests verifying embedded profiles do not enable credentials by default"
      contains: "credentials"
    - path: "crates/nono-proxy/src/filter.rs"
      provides: "HostFilter::new_strict() denying on empty allowlist"
      contains: "strict"
    - path: "crates/nono-proxy/src/config.rs"
      provides: "ProxyConfig.strict_filter: bool field"
      contains: "strict_filter"
    - path: "crates/nono/src/net_filter.rs"
      provides: "net_filter strict mode integration"
      contains: "strict"
    - path: "crates/nono-cli/src/sandbox_prepare.rs"
      provides: "network_block_requested: bool field threaded to ProxyLaunchOptions -> ProxyConfig"
      contains: "network_block_requested"
  key_links:
    - from: "crates/nono-cli/src/sandbox_prepare.rs"
      to: "crates/nono-cli/src/proxy_runtime.rs"
      via: "network_block_requested: bool field on PreparedSandbox -> ProxyLaunchOptions -> ProxyConfig.strict_filter"
      pattern: "network_block_requested"
    - from: "crates/nono-proxy/src/config.rs"
      to: "crates/nono-proxy/src/filter.rs"
      via: "ProxyConfig.strict_filter -> HostFilter::new_strict()"
      pattern: "strict_filter"
    - from: "crates/nono-proxy/src/server.rs"
      to: "crates/nono-proxy/src/filter.rs"
      via: "Server reads strict_filter from config to choose new() vs new_strict()"
      pattern: "strict"
---

<objective>
Plan 70-03 absorbs Cluster C2 — network-policy security hardening (0fb59375 + bd4c469a). This plan depends on Plan 70-01 (C3) because bd4c469a's PreparedSandbox struct literal references suppressed_system_service_operations, which cc21229f (C3) introduces. C2 compiles only after C3 is applied.

0fb59375 removes implicit credential routes from embedded network profiles — a security improvement preventing credential injection on connections where not explicitly requested.

bd4c469a adds strict_filter: bool to ProxyConfig and HostFilter::new_strict() to implement deny-by-default when network.block is set — closing the security gap where the proxy fell back to allow-all for hosts not in the allowlist even when the user explicitly requested network blocking.

This plan's threat model is substantive: it verifies that the cherry-picked behavior preserves/improves the fork's network-deny and credential-injection posture and does NOT regress the fork's RouteStore/CredentialStore decoupling (Phase 56).

Purpose: Harden the fork's proxy enforcement — deny-by-default under network.block and remove implicit credential routes.

Output: Two C2 cherry-pick commits on main with D-19 trailers and DCO sign-off; workspace test suite green vs baseline.
</objective>

<execution_context>
@C:\Users\OMack\.claude\get-shit-done\workflows\execute-plan.md
@C:\Users\OMack\.claude\get-shit-done\templates\summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md
@.planning/phases/70-upst8-cherry-pick-sync/70-01-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
@.planning/templates/cross-target-verify-checklist.md

<interfaces>
<!-- C2 commits (from 69-DIVERGENCE-LEDGER.md § Cluster C2) -->
<!-- Cherry-pick source: upstream remote (https://github.com/always-further/nono.git) -->
<!-- DEPENDENCY: C3 (Plan 70-01) MUST be applied first. Verify before starting: -->
<!--   grep "suppressed_system_service_operations" crates/nono-cli/src/sandbox_prepare.rs -->
<!--   must find the field; if not, Plan 70-01 has not completed. -->

C2 commits (chronological, oldest-first per git log):
  0fb59375 — refactor(network-policy): do not enable credentials by default in profiles
             Files: crates/nono-cli/data/network-policy.json, crates/nono-cli/src/network_policy.rs
             Author: Luke Hinds <lukehinds@gmail.com>
             Upstream-tag: v0.61.0
             windows-touch: no
             Upstream-date: Mon Jun 1 18:39:27 2026 +0100

  bd4c469a — fix(proxy): deny-by-default when network.block is set (#1082)
             Files: launch_runtime.rs, main.rs, proxy_runtime.rs, sandbox_prepare.rs (C3 dep),
                    crates/nono-proxy/src/config.rs, filter.rs, server.rs,
                    crates/nono/src/net_filter.rs
             Author: Caio Silva <caio@cdcs.dev>  (NOTE: different author than C3/C4)
             Upstream-tag: v0.61.2
             windows-touch: no
             +156/-4 lines
             Upstream-date: Fri Jun 5 09:28:12 2026 +0100

D-19 trailer for 0fb59375:
  Upstream-commit: 0fb59375
  Upstream-tag: v0.61.0
  Upstream-author: Luke Hinds <lukehinds@gmail.com>
  Co-Authored-By: Luke Hinds <lukehinds@gmail.com>
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>

D-19 trailer for bd4c469a (DIFFERENT author — Caio Silva):
  Upstream-commit: bd4c469a
  Upstream-tag: v0.61.2
  Upstream-author: Caio Silva <caio@cdcs.dev>
  Co-Authored-By: Caio Silva <caio@cdcs.dev>
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>

Fork-divergence catalog (from upstream-sync-quick.md + Phase 56 fork surfaces):
  RouteStore/CredentialStore decoupling: crates/nono-proxy/src/route.rs + credential.rs are
  fork-divergent from upstream (Phase 56 REQ-NET-01). When cherry-picking bd4c469a's server.rs
  changes, verify that upstream's credential-injection path does NOT revert to pre-Phase-56
  behavior. The fork's endpoint-before-credential ordering must be preserved.

Plan 70 base SHA: 6667177e
Upstream remote alias: upstream
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 0: Prerequisite gate — verify Plan 70-01 (C3) is complete before starting C2</name>
  <files></files>
  <read_first>
    - crates/nono-cli/src/sandbox_prepare.rs (check for suppressed_system_service_operations field)
    - .planning/phases/70-upst8-cherry-pick-sync/70-01-SUMMARY.md (confirm Plan 70-01 completion status)
  </read_first>
  <action>
Before any cherry-pick, verify the C3 prerequisite is satisfied:

  1. Run: grep "suppressed_system_service_operations" crates/nono-cli/src/sandbox_prepare.rs
     This must find the field. If it does NOT exist, Plan 70-01 has not completed — STOP and complete Plan 70-01 first.

  2. Read .planning/phases/70-upst8-cherry-pick-sync/70-01-SUMMARY.md and confirm:
     - Plan 70-01 status is complete
     - Both C3 commits (cc21229f + 20cc5df9) are on main
     - Cross-target clippy status for C3 is recorded

  3. Run: git log --oneline -5 | head -5
     Confirm cc21229f and 20cc5df9 appear in recent history (as amended cherry-picks with D-19 trailers).

Only proceed to Task 1 when all three checks pass.
  </action>
  <verify>
    <automated>grep -c "suppressed_system_service_operations" crates/nono-cli/src/sandbox_prepare.rs</automated>
    Must be greater than 0. If 0, do not proceed — complete Plan 70-01 first.
  </verify>
  <acceptance_criteria>
    - grep "suppressed_system_service_operations" crates/nono-cli/src/sandbox_prepare.rs finds the field (count > 0)
    - 70-01-SUMMARY.md exists and records Plan 70-01 completion
    - Executor confirms git log shows C3 commits in recent history
  </acceptance_criteria>
  <done>C3 prerequisite field is confirmed present; safe to proceed with C2 cherry-picks.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 1: C2 cherry-picks — network-policy security hardening (0fb59375 + bd4c469a)</name>
  <files>
    crates/nono-cli/data/network-policy.json,
    crates/nono-cli/src/launch_runtime.rs,
    crates/nono-cli/src/main.rs,
    crates/nono-cli/src/network_policy.rs,
    crates/nono-cli/src/proxy_runtime.rs,
    crates/nono-cli/src/sandbox_prepare.rs,
    crates/nono-proxy/src/config.rs,
    crates/nono-proxy/src/filter.rs,
    crates/nono-proxy/src/server.rs,
    crates/nono/src/net_filter.rs
  </files>
  <behavior>
    - After 0fb59375: network-policy.json embedded profiles (google-ai, developer, codex, claude-code) have no credentials arrays; network_policy.rs tests verify embedded profiles do not enable credentials by default
    - After bd4c469a: ProxyConfig has strict_filter: bool (default false); HostFilter::new_strict() denies empty allowlist; PreparedSandbox has network_block_requested: bool; when network.block is set, strict_filter is true in the ProxyConfig so proxy denies non-allowlisted hosts; backward compatible (strict_filter=false preserves historical allow-all behavior)
    - The fork's RouteStore/CredentialStore decoupling is preserved in server.rs (no regression to pre-Phase-56 allow-all-credential path)
  </behavior>
  <read_first>
    - crates/nono-proxy/src/config.rs (full file — understand current ProxyConfig struct before cherry-picking)
    - crates/nono-proxy/src/filter.rs (full file — understand current HostFilter before cherry-picking)
    - crates/nono-proxy/src/server.rs (full file — CRITICAL: understand RouteStore/CredentialStore decoupling; must not regress)
    - crates/nono-proxy/src/route.rs (read to understand fork's RouteStore shape — Phase 56 fork surface)
    - crates/nono-proxy/src/credential.rs (read to understand fork's CredentialStore shape — Phase 56 fork surface)
    - crates/nono-cli/src/sandbox_prepare.rs (read current state post-C3; understand where to add network_block_requested)
    - crates/nono-cli/src/network_policy.rs (read before cherry-picking 0fb59375)
    - crates/nono-cli/data/network-policy.json (read before cherry-picking 0fb59375)
    - .planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md § Cluster C2 (commit rows, cross-cluster re-export check, C2->C3 dep, windows-touch: no)
    - .planning/templates/upstream-sync-quick.md (D-19 trailer, fork-divergence catalog including RouteStore/CredentialStore decoupling)
    - .planning/templates/cross-target-verify-checklist.md (MANDATORY — sandbox_prepare.rs is cfg-gated Unix surface; net_filter.rs may be too)
  </read_first>
  <action>
Cherry-pick C2 commits from the upstream remote in chronological order (oldest-first):

  1. 0fb59375 — refactor(network-policy): do not enable credentials by default in profiles
     Cherry-pick: git cherry-pick 0fb59375
     Touches only network-policy.json and network_policy.rs — pure data + test file changes.
     If conflicts: apply upstream's removal of credentials arrays from the 4 embedded profiles (google-ai, developer, codex, claude-code) while preserving any fork-specific additions to network-policy.json.
     Amend with D-19 trailer (Upstream-author: Luke Hinds <lukehinds@gmail.com>, Upstream-tag: v0.61.0).

  2. bd4c469a — fix(proxy): deny-by-default when network.block is set (#1082)
     Cherry-pick: git cherry-pick bd4c469a
     This commit has the C3 dep: it references suppressed_system_service_operations on PreparedSandbox (already present after Plan 70-01). If it ALSO references any other C3 symbols that are not yet present, resolve by applying those additions manually.

     CRITICAL fork-divergence check for sandbox_prepare.rs:
       bd4c469a adds network_block_requested: bool to PreparedSandbox. The upstream PreparedSandbox struct literal in the commit may reference suppressed_system_service_operations (C3 dep) and also the existing fork fields. When resolving conflicts in sandbox_prepare.rs, preserve ALL existing fork fields and add only network_block_requested.

     CRITICAL fork-divergence check for server.rs (Phase 56 RouteStore/CredentialStore):
       The fork's server.rs has RouteStore/CredentialStore decoupling from Phase 56. Upstream's bd4c469a modifies server.rs to thread strict_filter behavior. When applying these server.rs changes, verify that:
         a. The fork's RouteStore lookup path is preserved (not replaced with upstream's credential path)
         b. The HostFilter strict mode is applied AFTER the credential lookup, not before
         c. The deny-by-default behavior for non-allowlisted hosts is consistent with the fork's existing deny posture for network.block scenarios
       If upstream's server.rs changes conflict heavily with the fork's RouteStore/CredentialStore divergence: use D-20 manual replay for server.rs specifically (Upstream-replayed-from: bd4c469a). Document in the commit body which files were clean-picked and which were replayed.

     IMPORTANT: Do NOT introduce allow-all fallback in server.rs or filter.rs. The security contract of bd4c469a is deny-by-default under network.block — any conflict resolution must preserve this contract.

     Amend with D-19 trailer (Upstream-author: Caio Silva <caio@cdcs.dev>, Upstream-tag: v0.61.2).
     Note: bd4c469a has a DIFFERENT upstream author than the other C3/C4 commits — Co-Authored-By and Upstream-author are "Caio Silva <caio@cdcs.dev>".

D-70-E1 Windows-only-files invariant:
  git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"
  Must return 0 lines.

D-70-E3 Cross-target clippy (MANDATORY):
  sandbox_prepare.rs and net_filter.rs may contain cfg-gated Unix code. Verify by inspecting the files for #[cfg(target_os = "linux")] / #[cfg(target_os = "macos")] / #[cfg(any(...))] blocks.
  If cfg-gated blocks are present:
    Run: cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
    Run: cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
    If cross-toolchain unavailable (ring/aws-lc-sys C-toolchain missing on Windows host):
      Mark PARTIAL per cross-target-verify-checklist.md § PARTIAL Disposition.
      Human verification truth: "GH Actions Linux Clippy + macOS Clippy lanes on HEAD must report green."
      NEVER flip to VERIFIED on Windows-host cargo check alone.
  NEVER silence lints with #[allow(...)].

D-70-E4 Baseline-aware CI gate (plan base SHA: 6667177e):
  Run: cargo test --workspace (or cargo test -p nono-proxy && cargo test -p nono-cli && cargo test -p nono)
  Categorize transitions vs baseline. Known pre-existing failures (red->red carry-forward):
    4 nono-cli failures (profile_cmd init + 3 protected_paths) + 1 nono lib failure (try_set_mandatory_label)
  Any new green->red transition is a FAIL (real regression introduced by C2 cherry-picks).

D-70-E5 Cargo lockfile: no new deps expected; verify unchanged.

IMPORTANT: No .unwrap()/.expect() in conflict resolution. Use ? and NonoError.
Before push: git status --short | grep -E "build_notes|\.gsd" must return 0 lines (repo stays PUBLIC).
  </action>
  <verify>
    <automated>git log --format="%B" HEAD~2..HEAD | grep -v "^#" | grep -c "^Upstream-commit:"</automated>
    Must equal 2 (one per C2 commit). Also verify:
      git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      grep "strict_filter" crates/nono-proxy/src/config.rs (must find the field)
      grep "network_block_requested" crates/nono-cli/src/sandbox_prepare.rs (must find the field)
      cargo build --workspace (exit 0)
      cargo test -p nono-proxy (exit 0 or pre-existing failures documented)
      cargo test -p nono-cli (exit 0 or only pre-existing carry-forward failures)
  </verify>
  <acceptance_criteria>
    - Exactly 2 cherry-pick commits on main for C2 (0fb59375 then bd4c469a)
    - Each commit message contains the verbatim 6-line D-19 trailer with correct per-commit author (Luke Hinds for 0fb59375; Caio Silva for bd4c469a)
    - git log --format="%B" HEAD~2..HEAD | grep -v "^#" | grep -c "^Upstream-commit:" equals 2
    - git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" returns 0 lines (D-70-E1 PASS)
    - grep "strict_filter" crates/nono-proxy/src/config.rs finds the ProxyConfig.strict_filter field
    - grep "network_block_requested" crates/nono-cli/src/sandbox_prepare.rs finds the field
    - network-policy.json embedded profiles (google-ai, developer, codex, claude-code) no longer contain credentials arrays
    - RouteStore/CredentialStore decoupling in server.rs is preserved (no allow-all credential fallback reintroduced)
    - HostFilter has a new_strict() method or strict mode that denies on empty allowlist
    - cargo build --workspace exits 0
    - cargo test -p nono-proxy exits 0 OR only pre-existing failures documented as carry-forward
    - cargo test -p nono-cli exits 0 OR only pre-existing red->red carry-forward failures documented; NO new green->red transitions
    - Cross-target clippy: VERIFIED (both targets clean) OR PARTIAL (toolchain unavailable, CI deferred) — status explicitly recorded in SUMMARY
    - Cargo.toml and Cargo.lock unchanged (no new dependencies)
    - No .unwrap()/.expect() in any conflict-resolution code
    - git status --short | grep -E "build_notes|\.gsd" returns 0 lines before push
  </acceptance_criteria>
  <done>C2 cherry-picks are on main with correct D-19 trailers; deny-by-default under network.block is delivered; implicit credential routes removed from embedded profiles; fork's RouteStore/CredentialStore decoupling preserved.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <what-built>
    All three clusters (C3, C4, C2) are cherry-picked on main with D-19 trailers. The full UPST8 will-sync set is absorbed. This checkpoint validates the post-sync security posture before the phase closes.
  </what-built>
  <how-to-verify>
    1. Run: git log --oneline --format="%h %s" -8
       Confirm the 5 cherry-pick commits appear in history (cc21229f, 20cc5df9 for C3; db073750 for C4; 0fb59375, bd4c469a for C2).

    2. Verify D-19 trailer count:
       git log --format="%B" HEAD~5..HEAD | grep -v "^#" | grep -c "^Upstream-commit:"
       Must equal 5 (one per will-sync commit, except if C4 used D-20 in which case count is 4 Upstream-commit + 1 Upstream-replayed-from).

    3. Verify Windows-only-files invariant (full phase):
       git diff --name-only 6667177e..HEAD -- crates/ bindings/ | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"
       Must return 0 lines.

    4. Verify the security properties of C2:
       - Check that network-policy.json no longer has credentials arrays in the embedded profiles.
       - Check that ProxyConfig.strict_filter defaults to false: grep "strict_filter.*false\|strict_filter.*bool" crates/nono-proxy/src/config.rs
       - Check that HostFilter has a deny-on-empty mode: grep "new_strict\|strict" crates/nono-proxy/src/filter.rs

    5. Run the full workspace test suite:
       cargo test --workspace
       Review output: identify any new green->red transitions vs plan base SHA 6667177e.
       Pre-existing failures (red->red carry-forward, NOT regressions): 4 nono-cli + 1 nono lib.

    6. Review cross-target clippy status in 70-01-SUMMARY.md and 70-03-SUMMARY.md:
       Confirm each plan records either VERIFIED (both targets clean) or PARTIAL (CI deferred).

    7. Check repo-public invariant:
       git status --short | grep -E "build_notes|\.gsd"
       Must return 0 lines.
  </how-to-verify>
  <resume-signal>Type "approved" if all checks pass, or describe specific failures to investigate.</resume-signal>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| upstream commit -> fork main | 0fb59375 + bd4c469a absorbed via git cherry-pick from always-further/nono upstream remote |
| network profile -> credential injection | 0fb59375 removes implicit credential routes from embedded profiles; credential injection now requires explicit declaration |
| proxy filter -> outbound host | bd4c469a closes the allow-all fallback when network.block is set; HostFilter::new_strict() enforces deny-by-default for non-allowlisted hosts |
| PreparedSandbox -> ProxyLaunchOptions -> ProxyConfig | network_block_requested: bool threads the user's block intent through the stack to flip strict_filter |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-70-03-01 | Information Disclosure | network-policy.json implicit credential routes (pre-0fb59375) | mitigate | 0fb59375 is the mitigation: removing credentials arrays from embedded profiles closes the vector where credential injection occurred on connections not explicitly requesting it. Executor verifies no credentials arrays remain in the 4 affected profiles after cherry-pick. |
| T-70-03-02 | Elevation of Privilege | HostFilter allow-all fallback (pre-bd4c469a) | mitigate | bd4c469a is the mitigation: HostFilter::new_strict() + ProxyConfig.strict_filter flip deny-by-default when network.block is set. Executor must verify the backward-compat default (strict_filter=false, historical behavior) does not inadvertently widen the attack surface in non-block-net scenarios. |
| T-70-03-03 | Tampering | RouteStore/CredentialStore decoupling (Phase 56 fork surface) | mitigate | bd4c469a's server.rs changes must not regress the fork's RouteStore/CredentialStore decoupling. Executor reads route.rs + credential.rs before cherry-picking server.rs and verifies the decoupling is preserved in the conflict-resolved result. If a regression is detected: abort and use D-20 replay for server.rs preserving the decoupling invariant. |
| T-70-03-04 | Spoofing | D-19 trailer integrity (different authors: Luke Hinds vs Caio Silva) | mitigate | bd4c469a has author Caio Silva <caio@cdcs.dev> — the Co-Authored-By and Upstream-author trailer fields MUST reflect Caio Silva, not Luke Hinds. Acceptance criteria verifies per-commit author correctness. |
| T-70-03-05 | Denial of Service | strict_filter deny-by-default when network.block not set | accept | ProxyConfig.strict_filter defaults to false per the upstream commit description ("Backward compatible: HostFilter::new() and ProxyConfig.strict_filter default to the historical behavior"). Risk accepted: backward compat is preserved; non-block-net profiles unaffected. |
| T-70-03-SC | Tampering | cargo installs during cherry-pick | accept | C2 introduces no new Cargo.toml dependencies per the ledger (pure logic additions to existing proxy/cli/nono crates); Cargo.toml + Cargo.lock must be unchanged — verified in acceptance criteria. |
</threat_model>

<verification>
After all tasks complete, verify the plan as a whole:

Close-gate (Phase 34 D-34-D2 8-check format, categorized):

1. [_load_bearing] git log --format="%B" HEAD~2..HEAD | grep -v "^#" | grep -c "^Upstream-commit:" equals 2 (C2 D-19 trailers)
2. [_load_bearing] git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" returns 0 lines (D-70-E1 PASS)
3. [_load_bearing] grep "strict_filter" crates/nono-proxy/src/config.rs finds the ProxyConfig.strict_filter field
4. [_load_bearing] grep "network_block_requested" crates/nono-cli/src/sandbox_prepare.rs finds the field
5. [_load_bearing] cargo build --workspace exits 0
6. [_load_bearing] cargo test --workspace: no new green->red transitions vs baseline 6667177e (pre-existing 5 failures are carry-forward)
7. [_load_bearing] Cross-target clippy status: VERIFIED or PARTIAL with CI deferral recorded — NEVER skipped silently
8. [_environmental] git status --short | grep -E "build_notes|\.gsd" returns 0 lines (repo stays PUBLIC)

Phase-wide close-gate (UPST8-02 final validation):
  git diff --name-only 6667177e..HEAD -- crates/ bindings/ | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"
  Must return 0 lines. (D-70-E1 across all 5 cherry-picks)

  git log --format="%B" 6667177e..HEAD | grep -v "^#" | grep -c -E "^Upstream-(commit|replayed-from):"
  Must equal 5 (one per will-sync commit; C4 may be Upstream-replayed-from if D-20 was used).
</verification>

<success_criteria>
- Two C2 commits on main with correct per-commit D-19 trailers (0fb59375 author Luke Hinds; bd4c469a author Caio Silva)
- Embedded network profiles strip implicit credentials (0fb59375)
- Proxy denies non-allowlisted hosts under network.block via strict_filter (bd4c469a)
- RouteStore/CredentialStore decoupling preserved in server.rs (no Phase 56 regression)
- No windows-only files touched across C2 (D-70-E1 PASS)
- Cargo.toml and Cargo.lock unchanged
- cargo build --workspace exits 0
- No new green->red test transitions vs plan base SHA 6667177e
- Cross-target clippy: VERIFIED or PARTIAL with explicit CI deferral (never skipped)
- Human-verify checkpoint approved
- UPST8-02 satisfied: all 5 will-sync commits (C3 + C4 + C2) on main with D-19/D-20 trailers
</success_criteria>

<output>
Create .planning/phases/70-upst8-cherry-pick-sync/70-03-SUMMARY.md when done.
Include: C2 cherry-pick log (2 commits, SHAs, conflict inventory, trailer verification result including per-commit author correctness); RouteStore/CredentialStore decoupling preservation verdict; baseline-aware CI gate result (full workspace); cross-target clippy status (VERIFIED or PARTIAL with CI deferral); D-70-E1 windows-invariant status (PASS for C2 and PHASE-WIDE); phase-wide close-gate results (8-check format); human-verify checkpoint outcome; UPST8-02 satisfied or outstanding items.

Also update:
- .planning/ROADMAP.md Phase 70 entry: flip [ ] to [x], set Plans: 3/3, add completion date
- .planning/STATE.md: advance Current Focus, add Phase 70 close entry in Accumulated Context
- .planning/REQUIREMENTS.md: flip UPST8-02 [ ] to [x] and update Traceability table
</output>
