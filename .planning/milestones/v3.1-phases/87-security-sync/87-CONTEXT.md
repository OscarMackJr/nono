# Phase 87: Security Sync - Context

**Gathered:** 2026-06-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb the two upstream security fixes in the `v0.62.0..v0.64.0` window onto the fork's
divergent Linux enforcement code, cross-target clippy clean:

- **SEC-01 (#1096, `e2086877`):** Close the Linux AF_UNIX datagram bypass — trap
  `sendto`/`sendmsg`/`sendmmsg` in the seccomp filter and gate on a connect grant.
- **SEC-02 (#1064, `6b3eb013`):** Guard `deduplicate()` against inheriting procfs-remap
  originals — preserve `/dev/null` when deduped with `/dev/stdin`.

Cluster C disposition (Phase 85 ledger) = **will-sync / adopt-upstream**, security=H dominant.
Both commits are `#[cfg(target_os = "linux")]`-only (no Windows touch), but land exactly where the
fork has diverged from upstream — so this is a **port**, not a blind cherry-pick.

**In scope:** SEC-01, SEC-02, plus hardening **CR-02** (audit-integrity bypass, see D-12).
**Out of scope:** new capabilities; CR-01 (deferred to Phase 88); anything beyond the two upstream
security commits + CR-02.

</domain>

<decisions>
## Implementation Decisions

### AF_UNIX enforcement mechanism (SEC-01)
- **D-01:** **Hybrid gate.** When the seccomp filter traps an AF_UNIX datagram
  `sendto`/`sendmsg`/`sendmmsg`: if the `CapabilitySet` has **no** unix-socket grant → bake a
  static `SECCOMP_RET_ERRNO(EPERM)` deny into the filter (deterministic, fail-secure, no
  prompt-flood). If a grant **exists** → route to the existing USER_NOTIF supervisor
  (`supervisor_linux.rs`) to validate the per-call `sockaddr_un` destination against the granted
  path(s). This preserves upstream's connect-grant semantics while matching the fork's
  path-validation model.
- **D-02:** No-grant deny is **fail-secure silent `EPERM`** by default. Where cheap, opportunistically
  attach a remediation via the Phase 86 structured-diagnostic surface (`NonoError::remediation`) —
  nice-to-have, **not** a blocker.
- **D-03:** The fork already anticipated this — `crates/nono/src/sandbox/linux.rs:844-847` carries a
  standing comment that pathname AF_UNIX grants "are not distinguishable on this Linux path until the
  seccomp AF_UNIX [trap]." This trap is the resolution of that TODO.

### Absorption strategy
- **D-04:** **`git cherry-pick -x` per commit** (one atomic commit per upstream SHA: `e2086877`,
  then `6b3eb013`), each with a DCO `Signed-off-by` trailer (Phase 86 pattern). The `-x` line
  preserves upstream provenance.
- **D-05:** Expect conflicts in the security hunks of `linux.rs` / `capability.rs` (the divergent
  files). Resolve by **porting upstream semantics onto the fork's structures** — do NOT accept
  upstream's version wholesale. Cleanly-applying hunks (helpers, tests) ride along.

### Tests & verification
- **D-06:** Port upstream's test matrix **adapted to the fork's test module**, AND add at least one
  **fork-specific test** for the net-new hybrid path (grant-present `sockaddr_un` destination
  validation via USER_NOTIF) — upstream has no equivalent for this behavior.
- **D-07:** The **Linux-execution leg is PARTIAL→CI** — this is a Windows dev-host; seccomp tests
  can't run locally. Live GH Actions Linux lane is the decisive gate. Same documented deferral
  category as cross-target clippy (not a gap).
- **D-08:** **Cross-target clippy is mandatory** on the cfg-gated Unix edits (`linux.rs`,
  `supervisor_linux.rs`, `capability.rs`) per the CLAUDE.md MUST/NEVER rule. Host has rustup std but
  no cross C-compiler → expected **PARTIAL→CI** per `.planning/templates/cross-target-verify-checklist.md`.
  Windows-host `cargo check` is NOT an accepted substitute.

### #1064 procfs-remap dedup guard (SEC-02)
- **D-09:** **Confirm-then-port.** First write a regression test reproducing the
  `/dev/null`-dropped-by-`/dev/stdin` case against the fork's `deduplicate()` (`capability.rs:1491`).
- **D-10:** If the test **fails** → port a guard adapted to the fork's platform-specific keying +
  deferred `original_updates`/`access_upgrades` logic (do not force-fit upstream's guard).
- **D-11:** If the test **passes** (the fork's divergent keying already avoids the bug) → keep the
  test as a regression guard and **document why no code change was needed**. Evidence-first; no blind
  force-fit.

### Scope of Phase-86-inherited criticals
- **D-12:** **Harden CR-02 here.** `verify_audit_log` reports `records_verified` even when `stored`
  is `None` (audit-integrity bypass — security-relevant, fits "Security Sync"). Fix it and record it
  as a **deliberate fork-divergence** in the divergence ledger / an ADR note (so future syncs expect
  the conflict on these lines). Upstream-identical code is at `e9529312` (lines ~875/915).
- **D-13:** **Defer CR-01** (FFI error paths leave `LAST_DIAGNOSTIC_CODE` stale — correctness, not a
  bypass) to **Phase 88**, where the FFI/dependency surface is already open. Upstream-identical code
  is at `a6aa5995`.

### Claude's Discretion
- Exact seccomp rule construction (BPF arg-matching for `AF_UNIX` family detection, abstract-namespace
  handling, `sendmmsg` iovec walking) — researcher/planner to resolve.
- Whether the connect-grant tracking needs supervisor-side state vs. a static filter decision —
  follows from how the fork models unix-socket path grants.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 87 scope & dispositions
- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` §"Cluster C: AF_UNIX Datagram
  Bypass and Procfs-Remap Dedup" (lines ~225-254) — will-sync disposition, per-commit table
  (`e2086877`, `6b3eb013`), and the explicit Phase 87 cross-target clippy note.
- `.planning/REQUIREMENTS.md` — SEC-01, SEC-02 (lines 38-39).
- `.planning/ROADMAP.md` §"Phase 87: Security Sync" — goal + 3 success criteria.

### Phase 86 inherited criticals (CR-01 / CR-02)
- `.planning/phases/86-library-boundary-convergence/86-REVIEW.md` §"Critical Issues" (CR-01 ~line 230;
  CR-02) — full finding text for the two upstream-inherited criticals.
- `.planning/phases/86-library-boundary-convergence/86-VERIFICATION.md` (lines ~109, 125) — records
  CR-01/CR-02 as upstream-inherited deferrals (upstream SHAs `a6aa5995` / `e9529312`).

### Code touchpoints (fork-divergent)
- `crates/nono/src/sandbox/linux.rs` — seccomp filter build; AF_UNIX comment at lines 844-847;
  fallback modes (`BlockAll`/`ProxyOnly`).
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — USER_NOTIF supervisor (TOCTOU,
  fd-injection, path resolution); destination-validation lands here for the grant-present path.
- `crates/nono/src/capability.rs` — `deduplicate()` (line 1491), `remap_procfs_self_references`
  (1353), `rewrite_procfs_self_reference` (1804, handles `/dev/stdin|stdout|stderr`),
  `deduplicate_unix_sockets` (1650).

### Process rules
- `CLAUDE.md` §"Coding Standards" — cross-target clippy MUST/NEVER rule for cfg-gated Unix edits.
- `.planning/templates/cross-target-verify-checklist.md` — PARTIAL→CI deferral procedure.

### Upstream commits to absorb
- `e2086877` — fix(linux): trap sendto/sendmsg to prevent AF_UNIX datagram bypass (#1096), v0.64.0
  (4 files, 684+/138-).
- `6b3eb013` — fix: guard deduplicate() against inheriting procfs-remap originals (#1064), v0.63.0
  (1 file, 90+/2-).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **USER_NOTIF supervisor** (`supervisor_linux.rs:128 handle_seccomp_notification`) — already does
  `recv_notif` → path read → TOCTOU validate → allow/deny/inject. The grant-present AF_UNIX
  destination-validation (D-01) extends this proven path; reuse `resolve_notif_path` /
  `notif_id_valid` rather than inventing a new notify loop.
- **Seccomp fallback modes** (`linux.rs` `SeccompNetFallback::{BlockAll, ProxyOnly, None}`) — the
  static AF_UNIX deny rule (no-grant case) slots into the existing filter-install path.
- **procfs-remap machinery** (`capability.rs` `rewrite_procfs_self_reference` for
  `/dev/stdin|stdout|stderr`) — already present; the #1064 interaction is with the fork's existing
  remap, so the guard must reconcile with it (D-10).

### Established Patterns
- **Cherry-pick `-x` + DCO, one atomic commit per upstream SHA** (Phase 86, 8 cherry-picks).
- **PARTIAL→CI deferral** for anything that can't be verified on the Windows dev-host (cross-target
  clippy + Linux seccomp test execution).
- **Fail-secure by default** — deny on any unsupported/ambiguous shape (CLAUDE.md security principles).

### Integration Points
- New AF_UNIX trap rules install alongside the existing network-fallback seccomp ruleset in
  `linux.rs::apply_with_abi`.
- Grant-present destination validation hooks into `supervisor_linux.rs::handle_seccomp_notification`'s
  syscall dispatch (currently `open`/`openat`-shaped — adds socket-send nr handling).

</code_context>

<specifics>
## Specific Ideas

- The fork's `linux.rs:844-847` TODO comment is the anchor for SEC-01 — the implementer should resolve
  (and remove/update) that comment as part of landing the trap.
- CR-02 must be recorded as a deliberate fork-divergence (ledger/ADR note) — it is the first
  intentional departure from upstream's audit code since Phase 86's convergence.

</specifics>

<deferred>
## Deferred Ideas

- **CR-01 (FFI stale `LAST_DIAGNOSTIC_CODE`)** → Phase 88 (Feature + Dependency Cherry-Pick Wave),
  where the FFI surface is already being touched. Correctness bug, not a security bypass — does not
  belong in the Security Sync phase. Upstream-identical at `a6aa5995`.

*No scope creep raised — discussion stayed within the two-security-fix boundary.*

</deferred>

---

*Phase: 87-security-sync*
*Context gathered: 2026-06-20*
