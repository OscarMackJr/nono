---
quick_id: 260727-jkn
status: complete
date: 2026-07-27
---

# Summary — macOS 0.69 → Windows-fork parity gap map

**Task:** Review upstream macOS functionality post-0.64 (baseline now v0.69.0,
`nolabs-ai/nono`), compare to the Windows fork, and map new phases/milestones.

**Type:** Analysis + roadmap-mapping. No code changed (deliverable is planning artifacts).

## What was produced

1. `260727-jkn-upstream-macos-inventory.md` — upstream feature inventory v0.64.0→v0.69.0
   (tool-sandbox subsystem deep-dive, per-PR feature/fix table, platform-mechanism items,
   macos.rs Seatbelt diff).
2. `260727-jkn-fork-windows-coverage.md` — fork HEAD Windows coverage, PRESENT/PARTIAL/ABSENT
   per feature area, verified against code (not just memory).
3. `260727-jkn-PLAN.md` — **the deliverable**: parity matrix + proposed milestones/phases.

## Headline finding

Upstream's biggest post-0.64 feature — the first-class **`tool-sandbox/` per-command
confinement subsystem** (PR #1105, v0.65.0) — was **never absorbed** into the fork (0 files),
despite the fork's declared v0.66.0 base containing it. The fork built a Claude-Code-coupled
`PreToolUse` hook prototype instead. This is a standing structural divergence, not a version lag.

## Proposed map

- **v3.6 — UPST12 sync v0.66.0→v0.69.0** (drain-then-sync, ~4 phases): absorb the XP deltas —
  `deny_domain`, SPIFFE/SPIRE, SigV4/sibling-route fixes, `platform_overrides` (+ migrate the
  fork's `windows_*` flags into it), `$VAR`/`@git` tokens, port-range schema (WFP-native emitter),
  bun/mise presets, macOS carry, resource-CLI alignment. Leapfrog crates to `0.70.0`.
- **v3.7 — Windows Tool-Sandbox Parity** (~5 phases, decision-gated): ADR + Windows-IPC spike →
  `command_policies` engine → `platform/windows.rs` driver (SCM_RIGHTS→handle-passing, peer-auth
  via `GetNamedPipeClientProcessId`) → credential nonce brokering + URL shim → migrate the hook
  onto the engine + clean-host UAT. **Depends on v3.6.**
- Optional: WFP reachability hardening (daemon-path-only + filter leak) — fork-internal.

**Fork is AHEAD** on resource controls (Job Object `--max-processes`/`--memory`/`--cpu-percent`,
kernel-enforced) — only CLI-flag alignment needed there.

## Sequencing

All post-v3.5 (v3.5 Trusted-Signing is operator-blocked in-flight). v3.6 before v3.7 (hard order).
Adopt formally via `/gsd:new-milestone` once v3.5 ships. Four operator decisions flagged in
PLAN.md §5 (adopt-vs-formalize, version target, macOS-carry scope, deny_domain policy stance).
