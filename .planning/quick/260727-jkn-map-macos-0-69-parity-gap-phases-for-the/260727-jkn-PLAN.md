---
quick_id: 260727-jkn
title: Map macOS 0.69 parity-gap phases/milestones for the Windows fork
type: analysis-and-roadmap-mapping
date: 2026-07-27
status: complete
inputs:
  - 260727-jkn-upstream-macos-inventory.md   # upstream v0.64.0→v0.69.0 feature inventory
  - 260727-jkn-fork-windows-coverage.md      # fork HEAD Windows coverage inventory
---

# macOS 0.69 → Windows-Fork Parity Gap: Phase / Milestone Map

**Deliverable of a `/gsd:quick` analysis task** — no code changed. This maps the parity gap
between upstream `nolabs-ai/nono` **v0.69.0** (post-0.64 macOS/sandbox functionality) and the
Windows fork (`OscarMackJr/nono`, HEAD based on upstream **v0.66.0**, crate `0.66.1`), then
proposes the milestones and phases to close it. Evidence: the two inventory files listed in the
frontmatter (built from `git show <tag>:<path>`, CHANGELOG, and HEAD code reads).

> **Sequencing note:** the active milestone **v3.5** (Trusted Signing Go-Live) is mid-flight —
> Phase 104 is blocked at an operator checkpoint (Azure root propagation). Everything proposed
> here is **post-v3.5**. Adopt formally via `/gsd:new-milestone` once v3.5 ships.

---

## 1. The headline finding

Upstream's single largest post-0.64 feature — the first-class **`tool-sandbox/` subsystem**
(PR #1105, introduced **v0.65.0**: 12 files, per-tool-call confinement with Linux + macOS
platform drivers) — **was never absorbed into the fork.** The fork's declared base is v0.66.0,
which contains the subsystem, yet the fork tree has **0** `tool-sandbox/` files. The UPST10
(v0.64→v0.65.1) and UPST11 (v0.65.1→v0.66.0) syncs effectively deferred/dropped it.

Instead the fork built its own Windows-specific "sandbox-the-tools" path: a Claude Code
`PreToolUse` hook (`data/hooks/nono-tool-hook.ps1` → `nono claude-code-hook`) that shells each
tool call to `nono run`, enforced by a Low-IL primary-token broker + per-run AppContainer. It is
a working UAT-passing prototype, but it is **defense-in-depth mediation coupled to Claude Code**,
not a general per-command isolation engine, and `command_policies` exists only as documentation
with **zero code wiring**.

**So the "parity gap" is two very different things:**
- a **normal upstream-sync delta** (v0.67.0→v0.69.0 XP features the fork simply hasn't pulled), and
- a **standing structural divergence** (the entire tool-sandbox subsystem), which is too large and
  too Windows-mechanism-specific to fold into a routine sync absorb.

They map to two distinct milestones.

---

## 2. Parity matrix (what's a gap, what isn't)

Legend: ✅ present/at-parity · ⚠️ partial · ❌ absent · ➕ fork ahead of macOS

| Area | Upstream (macOS/XP) | Windows fork today | Verdict |
|------|--------------------|--------------------|---------|
| **tool-sandbox subsystem** (per-command confinement, `command_policies`) | ✅ native, 12 files (#1105) | ❌ hook prototype, no engine | **Biggest gap → v3.7** |
| tool-sandbox refinements (#1280/#1322/#1325/#1384/#1394/#1413/#1417) | ✅ | ❌ (no base subsystem) | Deferred to v3.7 |
| `platform_overrides` per-OS profile patch (#1371/#1380) | ✅ | ❌ (uses top-level `windows_*` flags) | Gap → v3.6 |
| profile `extends` (#1320) | ✅ | ✅ (`cli.rs:1640`, honored) | At parity |
| `$VAR` / `@git:*` token expansion (#1296/#1298/#1271) | ✅ | ⚠️ partial | Gap → v3.6 |
| port **ranges** in profiles (#1398) | ✅ Seatbelt unroll (17.7k cap) | ⚠️ discrete `Vec<u16>` only | Gap → v3.6 (WFP does ranges natively) |
| `deny_domain` deny-list model (#1374) | ✅ | ❌ (allowlist-only) | Gap → v3.6 |
| SPIFFE/SPIRE workload identity (#1272) | ✅ | ❌ | Gap → v3.6 |
| SigV4 encoded-URI fix (#1430), sibling-route cross-deny fix (#1437) | ✅ | ❌ (not pulled) | Gap → v3.6 |
| forward-proxy `HTTP_PROXY` (#1335), `no_proxy` bypass (#1415) | ✅ | ✅ already present | At parity (verify no-regress) |
| HTTP/2 proxy + credential injection (#983) | ✅ | ✅ (`config.rs:73`) | At parity |
| bun (#1305) / mise (#1387) runtime presets | ✅ | ❌ | Minor gap → v3.6 |
| `--memory` / `--max-processes` resource caps (#1269/#1403) | ✅ Linux cgroup only (no macOS) | ➕ **kernel-enforced via Job Objects** + `--cpu-percent`/`--timeout` | **Fork ahead** — CLI-flag alignment only |
| macOS-only fixes: `~/.cache` (#1378), libdispatch thread cap (#1424), port-range emitter (#1398) | ✅ | n/a (fork ships macOS binaries) | Carry upstream as-is for cross-target parity |
| WFP per-SID kernel egress block | — (macOS uses Seatbelt) | ⚠️ real but **daemon-path only**; filters leak across restart | Fork-internal hardening (surfaced by audit) |

**Platform-mechanism translation table** (from the upstream inventory §3 — the items that need a
distinct Windows primitive, relevant to v3.7):

| Unix mechanism | Windows substitute |
|----------------|--------------------|
| `SCM_RIGHTS` fd passing over `AF_UNIX` (macOS EMSGSIZE ack-order quirk #1325) | `DuplicateHandle` + named pipe, or `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` on brokered child |
| shim caller auth: `SO_PEERCRED` / `proc_pidinfo` ancestry | `GetNamedPipeClientProcessId` + `OpenProcess`/token check; ancestry via `NtQueryInformationProcess` |
| daemonized-caller attribution (#1417): cgroup lineage / reparent-to-pid-1 | Job Object membership + process-creation-time identity pin |
| per-command re-exec: `execveat`/`fexecve` binary-pin | `CreateProcess` + pre-launch hash re-verify (fork already has broker trust gating) |
| `--max-processes` cgroup v2 `pids.max` | Job Object `ActiveProcessLimit` **(already done in fork)** |

---

## 3. Proposed milestones & phases

### ▸ Milestone **v3.6 — UPST12: Upstream Sync v0.66.0 → v0.69.0** (drain-then-sync)

Follows the established fork pattern (v3.1/v3.3/v3.4): divergence-audit → absorb → fork-invariant
verify → crate leapfrog + prepare-only release. Scope is the ~40 substantive commits across 5
tags **excluding** the tool-sandbox subsystem base (deferred to v3.7 — the refinements ride on a
subsystem the fork doesn't have). Mostly cross-platform, low Windows-specific risk.

- **Phase A — UPST12 divergence audit** (`.../NN-upst12-divergence-audit/`)
  Build a DIVERGENCE-LEDGER for `v0.66.0..v0.69.0`. Diff-inspect re-export surfaces (durable:
  cluster isolation can be empirically false). Explicitly log the 7 tool-sandbox refinement PRs
  as **DEFERRED → v3.7**. Settle the `platform_overrides` (#1371) adoption as full-adopt.
- **Phase B — Absorb proxy/network deltas**
  `deny_domain` (#1374), SPIFFE/SPIRE routes (#1272), SigV4 encoded-URI fix (#1430),
  sibling-route cross-deny fix (#1437). Verify forward-proxy/`no_proxy`/HTTP-2 do not regress.
  **Durable:** re-run `maturin build` (nono-py) + `napi build` (nono-ts) after any nono-proxy
  struct touch — binding struct-drift is caught only there.
- **Phase C — Absorb profile/policy + adopt `platform_overrides`**
  Land `platform_overrides` (#1371/#1380) and **migrate the fork's `windows_low_il_broker` /
  `windows_interpreters` top-level flags into `platform_overrides.windows`** (retires the flag
  sprawl; gives the fork the intended vehicle for Windows divergence). Add `$VAR`/`@git:*` token
  expansion (#1296/#1298), bun/mise presets, and the **port-range profile schema (#1398)** with a
  **WFP-native range emitter** on Windows (WFP expresses ranges directly — simpler than Seatbelt's
  per-port unroll) plus discrete-list back-compat.
- **Phase D — Absorb macOS/core carry + resource-CLI alignment + verify + release**
  Carry upstream macOS emitters as-is for cross-target parity: macos.rs port-range unroll (#1398),
  `~/.cache` (#1378), libdispatch `MAX_CRYPTO_THREADS`=12 (#1424). Align resource-limit CLI flag
  names/semantics (#1269/#1403) onto the fork's existing Job Object impl (no new enforcement — the
  fork is already ahead). Both cross-target clippy gates GREEN locally (no PARTIAL→CI). `make ci`
  (clippy + fmt + tests). **Leapfrog all 6 crates + both binding repos to `0.70.0`** (one minor
  above upstream's 0.69.0 — upstream moves ~1 minor/week, so patch-leapfrog risks collision).
  Prepare-only release.

### ▸ Milestone **v3.7 — Windows Tool-Sandbox Parity** (the tool-sandbox reckoning)

The strategic build: give the fork a real per-command confinement engine and reconcile it with
the PreToolUse-hook prototype. Large, multi-phase, decision-gated. **Depends on v3.6** (needs
`platform_overrides` for the Windows policy surface, the port-range schema, and the version base).

- **Phase E — ADR + Windows-mechanism spike** *(decision gate)*
  Decide **(a) adopt-and-port** upstream `tool-sandbox/` and build `platform/windows.rs`, vs.
  **(b) formalize-the-hook** into a generic `command_policies` engine that happens to be
  hook-driven. Spike the two hard mechanism substitutions: SCM_RIGHTS→handle-passing
  (`DuplicateHandle`/named-pipe/`HANDLE_LIST`) and shim caller-auth
  (`GetNamedPipeClientProcessId` + token/ancestry). Deliver an ADR (mirrors ADR-86 boundary style).
- **Phase F — `command_policies` engine (platform-agnostic core)**
  Wire the already-documented `command_policies` profile schema to real code: land the
  cfg-gated module skeleton (`protocol`, `policy`, `token_broker`, `credentials`,
  `dynamic_providers`, `url_shim`, `env`, `audit_context`), reusing upstream XP logic verbatim
  where possible. No Windows enforcement yet — engine + profile plumbing + tests.
- **Phase G — Windows platform driver (`tool-sandbox/platform/windows.rs`)**
  Shim listener over a named pipe; child stdio hand-off via handle duplication; caller auth via
  `GetNamedPipeClientProcessId` + `NtQueryInformationProcess` ancestry; per-command brokered
  launch routed through the **existing** Low-IL broker + AppContainer + Job Object path; pre-launch
  binary hash re-verify (reuse the fork's broker trust gating).
- **Phase H — Credential nonce brokering + URL shim on Windows**
  `nono_<64hex>` nonce broker shared with the proxy (`SharedBroker`); env-promotion + proxy
  header-injection redemption; `scan_and_reissue` stdout redaction; `open`-shim for OAuth browser
  opens. Real secrets never enter the agent address space — matches the macOS threat model.
- **Phase I — Migrate the Claude Code hook onto the engine + clean-host UAT**
  Retarget `nono claude-code-hook` to drive `command_policies` instead of the hardcoded
  `claude-code-tools-windows-runner` PowerShell-encoded rewrite; daemonized-caller attribution via
  Job Object identity (the #1417 concept). Clean-host UAT on Win11 (ephemeral Azure VM per the
  v3.5 IaC).

### ▸ Optional small item — **WFP reachability hardening** (fold into v3.6 Phase B or a v3.7 phase)

Not upstream parity — surfaced by the fork audit: direct `nono run --block` denies at the
zero-capability AppContainer layer (no matchable WFP filter), and per-SID filters can leak/
accumulate across `nono-wfp-service` restart. Make direct-run egress-block observable/matchable and
make filter cleanup idempotent. Small; can be a carry-forward todo if not sized into a phase.

---

## 4. Recommended sequencing

```
v3.5 (in-flight, operator-blocked)  ──ship──▶  v3.6 UPST12 sync  ──▶  v3.7 Tool-Sandbox Parity
                                                (A→B→C→D)              (E gate → F → G → H → I)
```

- **v3.6 before v3.7** is a hard order: `platform_overrides`, the port-range schema, and the
  0.70.0 version base all de-risk v3.7.
- **Phase E is a decision gate** — do not build the Windows driver before the adopt-vs-formalize
  ADR lands.
- v3.6 is a familiar, low-risk sync (est. ~4 phases, XP-heavy). v3.7 is the real engineering
  investment (est. ~5 phases, novel Windows IPC/handle-passing).

---

## 5. Open decisions for the operator (before adopting)

1. **Adopt-and-port vs. formalize-the-hook** for tool-sandbox (Phase E ADR) — the fork's whole
   direction hinges on this.
2. **Crate leapfrog target** — `0.70.0` (recommended, collision-safe) vs. `0.69.1` (minimal).
3. **Scope of macOS carry** — the fork ships macOS binaries, so macOS-only fixes (#1378/#1424/#1398
   emitter) are carried for real-user parity, not just cross-target clippy. Confirm that intent.
4. **Whether `deny_domain` fits the fork's security posture** — the fork is deliberately
   allowlist-only for network; adding a deny-list model is a policy stance, not just a feature.
