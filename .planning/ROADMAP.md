# Roadmap: nono Windows Parity & Quality

This roadmap tracks the path to full Windows/Unix parity and ongoing quality-of-life work for `nono`.

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 1–4 (shipped 2026-03-31; tag `v1.0`)
- ✅ **v2.0 Windows Gap Closure** — Phases 5–15 (shipped 2026-04-18; tag `v2.0`)
- ✅ **v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup** — Phases 16–21 + 18.1 (shipped 2026-04-21; tag `v2.1`)
- ✅ **v2.2 Windows/macOS Parity Sweep** — Phases 22–24 (shipped 2026-04-29; tag `v2.2`)
- 🏗️ **v2.3 Linux POC Unblock + Deferreds Closure** — Phases 25–32 + 27.1 (started 2026-04-29)

## Phases

<details>
<summary>✅ v1.0 Windows Alpha (Phases 1–4) — SHIPPED 2026-03-31</summary>

- [x] Phase 1: Windows Control Foundation (3/3 plans) — completed 2026-04-04
- [x] Phase 2: Persistent Sessions (4/4 plans) — completed 2026-04-04
- [x] Phase 3: Network Sandboxing (4/4 plans) — completed 2026-04-04
- [x] Phase 4: State Integrity & Deployment (3/3 plans) — completed 2026-04-05

See `.planning/milestones/v1.0-*` if archived separately; the `v1.0` git tag points at the formal shipped state.

</details>

<details>
<summary>✅ v2.0 Windows Gap Closure (Phases 5–15) — SHIPPED 2026-04-18</summary>

- [x] Phase 5: Windows Detach Readiness Fix (1/1 plan) — completed 2026-04-05
- [x] Phase 6: WFP Enforcement Activation (2/2 plans) — completed 2026-04-06
- [x] Phase 7: Quick Wins (2/2 plans) — completed 2026-04-08
- [x] Phase 8: ConPTY Shell (1/1 plan, UAT-driven) — completed 2026-04-10
- [x] Phase 9: WFP Port-Level + Proxy Filtering (4/4 plans) — completed 2026-04-10
- [x] Phase 10: ETW-Based Learn Command (3/3 plans) — completed 2026-04-10
- [x] Phase 11: Runtime Capability Expansion — stretch (2/2 plans) — completed 2026-04-11
- [x] Phase 12: Milestone Bookkeeping Cleanup (3/3 plans) — completed 2026-04-11
- [x] Phase 13: v2.0 Human Verification UAT (1/1 plan) — resolved 2026-04-18
- [x] Phase 14: v2.0 Fix Pass (2/3 plans, 1 escalated to Phase 15) — complete-with-carry-forward 2026-04-18
- [x] Phase 15: Detached Console + ConPTY Architecture Investigation (3/3 plans) — completed 2026-04-18

Full details: `.planning/milestones/v2.0-ROADMAP.md`.

</details>

<details>
<summary>✅ v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup (Phases 16–21 + 18.1) — SHIPPED 2026-04-21</summary>

- [x] Phase 16: Resource Limits — RESL-01..04 (2/2 plans) — completed 2026-04-18
- [x] Phase 17: Attach-Streaming — ATCH-01 (2/2 plans) — completed 2026-04-19
- [x] Phase 18: Extended IPC — AIPC-01 (4/4 plans) — completed 2026-04-19
- [x] Phase 18.1: Extended IPC Gap Closure (4/4 plans) — completed 2026-04-21
- [x] Phase 19: Cleanup — CLEAN-01..04 (4/4 plans) — completed 2026-04-19
- [x] Phase 20: Upstream Parity Sync — UPST-01..04 (4/4 plans) — completed 2026-04-19
- [x] Phase 21: Windows Single-File Filesystem Grants — WSFG-01..03 (5/5 plans) — completed-with-issues 2026-04-20 (supervisor-pipe regression surfaced + resolved 2026-04-20; Phase 18.1 closed the 5 AIPC UAT gaps)

Full details: `.planning/milestones/v2.1-ROADMAP.md`.

</details>

<details>
<summary>✅ v2.2 Windows/macOS Parity Sweep (Phases 22–24) — SHIPPED 2026-04-29</summary>

- [x] Phase 22: UPST2 — Upstream v0.38–v0.40 Parity Sync (6/6 plans, PROF + POLY + PKG + OAUTH + AUD-01..04) — completed 2026-04-28
- [x] Phase 23: Windows Audit-Event Retrofit (1/1 plan, AUD-05) — completed 2026-04-29
- [x] Phase 24: Parity-Drift Prevention (2/2 plans, DRIFT-01 + DRIFT-02) — completed 2026-04-27

Full details: `.planning/milestones/v2.2-ROADMAP.md`.

</details>
<details>
<summary>✅ v2.3 Linux POC Unblock + Deferreds Closure (Phases 25–34, incl. 27.1, 27.2) — SHIPPED 2026-05-12</summary>

- [x] Phase 25: Cross-Platform RESL + AIPC Unix Design (6/6 plans, REQ-AIPC-NIX-01 shipped; REQ-RESL-NIX-01..03 host-blocked carry-forward to v2.4) — completed 2026-05-10
- [⚠️] Phase 26: PKG Streaming Follow-Up (1/2 plans, REQ-PKGS-02 + REQ-PKGS-03 shipped via Plan 26-01; REQ-PKGS-01 + REQ-PKGS-04 host-blocked carry-forward to v2.4) — partial 2026-05-01
- [⚠️] Phase 27: Audit-Attestation Hardening (1 plan, REQ-AAH-01 closed transitively via Phase 27.1 + 27.2) — partial 2026-04-29
- [x] Phase 27.1: NONO_TEST_HOME Seam (3/3 plans, REQ-NTH-01..03) — INSERTED + completed 2026-05-04
- [x] Phase 27.2: Audit-Attestation Test Re-Enablement (4/4 plans, REQ-AAHX-01..03) — INSERTED + completed 2026-05-05
- [x] Phase 28: Authenticode Chain-Walker Subject Extraction (1/1 plan, REQ-AUDC-01..03) — completed 2026-04-30
- [x] Phase 29: WR-01 Reject-Stage Unification (1/1 plan, REQ-WRU-01..02, locked-as-design Option c) — completed 2026-04-30
- [x] Phase 30: Windows nono shell Architecture Investigation (5/5 plans; failure-mode finding + broker-pattern PoC) — completed 2026-05-08
- [x] Phase 31: Broker-Process Architecture / SHELL-01 (6/6 plans; SHELL-01 → ✔ validated; operator field-test SUCCESS recorded 2026-05-09) — completed 2026-05-09
- [x] Phase 32: Sigstore Integration (5/5 plans, TUF cached-root + keyless hardening + broker self-trust-anchor; 16 D-32-* decisions) — completed 2026-05-10
- [x] Phase 33: Upstream v0.40.1..v0.52.0 audit + parity-strategy ADR (4/4 plans; Option A `continue` accepted; DIVERGENCE-LEDGER.md; G-25-DRIFT-01 empirically disproved) — completed 2026-05-11
- [x] Phase 34: UPST3 — Upstream v0.41–v0.52 Sync Execution (13/13 plans; 12 cluster dispositions resolved; ~75 commits; 2 mid-flight splits; 4 D-20 manual-replays; 13 deferrals tracked) — completed 2026-05-12

**Carry-forwards to v2.4** (captured in `.planning/MILESTONE-CONTEXT.md`):
- Theme 1 — Complete the partial upstream ports (10 P34-DEFER-* items: 04b-1/2, 06-1, 08a-1, 08b-1/2, 09-1/2, 01-1/10-1)
- Theme 2 — v2.3 host-blocked carry-forwards (Plans 25-01 RESL Unix backends + 26-02 PKGS streaming/auto-pull)
- Theme 3 — UPST4 (upstream v0.52.1 / v0.52.2 / v0.53.0 ingestion per "lazily-evaluated cadence" ADR rule)

**Audit verdict at close:** `gaps_found` from `.planning/milestones/v2.3-MILESTONE-AUDIT.md` (2026-05-09 + Phase 34 post-audit close 2026-05-12). Gate triggered by institutional artifact gaps (4 phases missing VERIFICATION.md final: 26, 27, 28, 29) + 5 host-blocked requirements + Phase 31 verification = human_needed. Substantively healthy: 14/14 integration points WIRED, 5/5 E2E flows PASS, 12/12 cluster dispositions resolved, 0 D-34-E1 violations across 75 Phase 34 commits.

Full details: `.planning/milestones/v2.3-ROADMAP.md`.

</details>

