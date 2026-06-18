# Phase 79: WFP Egress Isolation + nono-ts Ergonomics - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-18
**Phase:** 79-wfp-egress-isolation-nono-ts-ergonomics
**Areas discussed:** WFP allowed-vs-denied mechanism, WFP egress target & loopback, confinedRun default broker-arm profile, confinedRun ergonomics & override surface

---

## WFP allowed-vs-denied mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| Block vs no-block contrast | Agent B block:true (per-SID WFP deny) denied; agent A non-blocked, reaches mock. Concurrent distinct SIDs prove per-SID scoping. Shipped code only. | ✓ |
| Proxy-mediated allow_domain | Both block:true; A reaches mock via nono-proxy allow_domain; tests proxy+WFP combo. | |
| Wire allow_domain→WFP allow rules | New Windows feature: translate allow_domain into per-SID WFP allow filters. Largest scope. | |

**User's choice:** Block vs no-block contrast (recommended).
**Notes:** Honors STATE's framing of WFP-01 as "a test, not a new WFP integration." Verifier note recorded in CONTEXT D-01: SC1's "agent A allowed via allow_domain" is realized as A=non-blocked / B=block:true; the per-SID isolation claim is still fully proven.

---

## WFP egress target & loopback

| Option | Description | Selected |
|--------|-------------|----------|
| Non-loopback mock bind | Bind the Rust mock to a non-loopback interface so the per-SID egress filter genuinely applies; deny is real. | ✓ |
| Loopback mock + confirm WFP covers it | Keep 127.0.0.1, first confirm the filter blocks loopback for the SID. Risk: loopback exemption → false PASS. | |
| External known IP | Target a real external host; not hermetic, needs outbound network. | |

**User's choice:** Non-loopback mock bind (recommended).
**Notes:** CONTEXT D-02 flags that the researcher MUST validate the per-SID WFP filter actually blocks the chosen non-loopback target (and whether the AppContainer needs a network capability to reach it) before the gate is trusted — a green gate against an exempt target is worse than no gate.

---

## confinedRun default broker-arm profile

| Option | Description | Selected |
|--------|-------------|----------|
| New dedicated nono-ts default profile | Minimal least-privilege policy.json profile (windows_low_il_broker:true) used when no profile passed. Engine-neutral. | ✓ |
| Reuse an existing broker-arm profile | Point default at e.g. claude-code; less surface but engine-coupled. | |
| Synthesize capabilities in-binding | Build caps + broker arm in napi binding; diverges from delegate-to-nono.exe model. | |

**User's choice:** New dedicated nono-ts default profile (recommended).
**Notes:** Fixes the no-profile → WriteRestricted → node 0xC0000142 failure. Name (`nono-ts-default`) and coverage left to planner.

---

## confinedRun ergonomics & override surface

| Option | Description | Selected |
|--------|-------------|----------|
| Overridable opts, defaults on; cover exe dir only | Optional lowIl/autoCoverTarget flags default to new behavior; auto-cover only the target exe's dir (matches SC3). Backward-compatible. | ✓ |
| Hard defaults, cover exe dir only | No opt-out; simplest API. | |
| Overridable opts; cover exe dir + cwd | Also auto-add cwd; broader default grant. | |

**User's choice:** Overridable opts, defaults on; cover exe dir only (recommended).
**Notes:** Keeps least-privilege default; cwd auto-cover and the Node-ESM ancestor-RA problem are deferred (Phase 77 lineage). Option names/typing left to planner.

---

## Claude's Discretion

- Exact name/coverage of the new nono-ts default profile.
- Option names/types for `lowIl`/`autoCoverTarget` and how they map onto the existing `confinedRun` signature.
- Internal structure of `wfp-egress-isolation.ps1` (within the shipped gate contract).
- Whether the two test agents launch via the daemon control pipe or a direct `nono run` path.

## Deferred Ideas

- allow_domain→WFP allow-rule wiring on Windows (future network-hardening milestone).
- confinedRun auto-cover of cwd / target ancestors (Phase 77 RA-grant lineage).
- 3 keyword-matched todos (msi-vcredist-prereq, poc-cert-broker-clean-host → Phase 80; macos-rlimit → macOS/v2.11) reviewed, not folded.
