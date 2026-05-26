# Phase 51: No-PTY Low-IL broker + token routing + write-deny preservation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-26
**Phase:** 51-No-PTY Low-IL broker + token routing + write-deny preservation
**Areas discussed:** Broker routing predicate, No-PTY stdio mechanism, Cascade arm structure, Write-deny test shape

---

## Broker routing predicate

### Q1 — What triggers the no-PTY Low-IL broker route vs WriteRestricted?

| Option | Description | Selected |
|--------|-------------|----------|
| Profile-gated opt-in | New profile field opts a profile into the broker route; claude-code sets it, others keep WriteRestricted. Deterministic, auditable, honors REQ-WSRH-02 "still reachable". | ✓ |
| Blanket — all non-PTY run | Every non-PTY session-SID `nono run` routes through broker; WriteRestricted retired on run path. Conflicts with "not a blanket removal". | |
| Binary-shape heuristic | Detect heavy-runtime PE shape; route only those. Non-deterministic security envelope, hard to audit/test. | |
| You decide | — | |

**User's choice:** Profile-gated opt-in

### Q2 — Profile-only or also a per-invocation CLI override?

| Option | Description | Selected |
|--------|-------------|----------|
| Profile-only for v2.7 | Opt-in lives solely as a profile field. Smallest surface; CLI override deferrable to REQ-WSRH-AUDIT-01. | ✓ |
| Profile + CLI override | Add a `--low-il-broker`/`--no-low-il-broker` flag too. More flexible but adds clap wiring + conflict rules to test now. | |
| You decide | — | |

**User's choice:** Profile-only for v2.7

### Q3 — Which built-in profiles set the field in v2.7?

| Option | Description | Selected |
|--------|-------------|----------|
| claude-code only | Only the single confirmed 0xC0000142 case; matches the deferred REQ-WSRH-AUDIT-01 boundary. | ✓ |
| All heavy-runtime profiles | Set on claude-code + other likely-heavy profiles. Widens exposure ahead of the deferred audit. | |
| You decide | — | |

**User's choice:** claude-code only

---

## No-PTY stdio mechanism

### Q1 — How should the no-PTY broker wire the child's stdio?

| Option | Description | Selected |
|--------|-------------|----------|
| Anonymous pipes (relayed) | nono.exe creates pipes, passes them via --inherit-handle; supervisor relays to nono's stdout. Reuses Phase 17. Supports redirection + capture. | ✓ |
| Inherited console (direct) | Child writes straight to nono's console. Simplest relay-wise but breaks `> file` redirection and supervised capture. | |
| You decide | — | |

**User's choice:** Anonymous pipes (relayed)

---

## Cascade arm structure

### Q1 — How to represent the no-PTY broker route in `select_windows_token_arm`?

| Option | Description | Selected |
|--------|-------------|----------|
| Distinct arm variant | Add `WindowsTokenArm::BrokerLaunchNoPty`. Clean unit-test target; existing PTY tests keep asserting `BrokerLaunch`, proving the Phase 31 path untouched. | ✓ |
| Reuse BrokerLaunch + pty flag | Distinguish by `pty.is_none()` downstream. Minimal enum churn but cascade decision no longer self-documents the no-PTY route. | |
| You decide | — | |

**User's choice:** Distinct arm variant

---

## Write-deny test shape

### Q1 — What fidelity should the NO_WRITE_UP regression test have?

| Option | Description | Selected |
|--------|-------------|----------|
| Real-spawn integration test | Spawn a real Low-IL child; child writes a Medium-IL-labeled path; assert kernel MIC denial. Matches REQ-WSRH-03 wording + Phase 31 precedent. | ✓ |
| Construction-level unit test | Assert token is Low-IL + path label is NO_WRITE_UP, no child spawn. Lighter but lower fidelity. | |
| You decide | — | |

**User's choice:** Real-spawn integration test

### Q2 — CI behavior when a label can't be applied?

| Option | Description | Selected |
|--------|-------------|----------|
| Hard fail, no silent skip | Test FAILS loudly if it can't set up the fixture/spawn. Consistent with BROKER-CR-04 (v2.5 Phase 41) retiring silent-SKIP. | ✓ |
| Host-gated #[ignore] | Run only on a configured Windows host. Avoids CI flakiness but risks the test silently never running — the pattern BROKER-CR-04 closed. | |
| You decide | — | |

**User's choice:** Hard fail, no silent skip

---

## Claude's Discretion

- Exact profile-field name (`windows_low_il_broker` suggested, not locked) and schema placement.
- Name/signature of the new `select_windows_token_arm` input and profile-field resolution into it.
- stdin wiring on the no-PTY path (inherit nono's stdin vs a fourth pipe).
- Whether the broker needs an explicit `--no-pty` CLI signal or can infer the mode from the handle set.

## Deferred Ideas

- CLI override flag for ad-hoc routing — deferred with REQ-WSRH-AUDIT-01.
- Profile-wide heavy-runtime audit (other Electron/Node/CLR profiles) — REQ-WSRH-AUDIT-01.
- Windows-host field validation + `windows-poc-handoff.mdx` doc update — Phase 52 (REQ-WSRH-04/06).
