---
status: partial
phase: 83-machine-policy-spine-egress-control
source: [83-VERIFICATION.md]
started: 2026-06-18
updated: 2026-06-18
---

## Current Test

[awaiting human testing — all 3 items are host-gated (provisioned/domain-joined Windows + fresh nono binary)]

## Tests

### 1. Live fleet GPO push (deny-by-default allowlist via GPO ADMX)
expected: On a domain-joined Win11 VM, load `nono.admx`/`nono.adml` into the Central Store, configure "Outbound Egress: Allowed Domain Suffixes" with `.anthropic.com` (the leading-dot ADMX-documented format — CR-01 path), apply to the host, restart the daemon, and launch a confined agent. The agent reaches `api.anthropic.com` and is denied an out-of-list host at BOTH the proxy (L7) and kernel WFP (L3/4) layers. A host with NO GPO configured (bare MSI install, sentinel key only) falls through to per-user (CR-02 path) — egress is NOT deny-all.
result: [pending]

### 2. Intune OMA-URI preset toggle
expected: On an Intune-enrolled device, enable the "Allow Anthropic" preset toggle via OMA-URI (`./Device/.../PresetTokens`). The stable token `anthropic` (NOT literal FQDNs) is written to `HKLM\SOFTWARE\Policies\nono\PresetTokens`; nono expands it to `*.anthropic.com` at runtime (WR-01 valueName fix lets the toggle write its token).
result: [pending]

### 3. egress-policy-deny Dark Factory gate — full SC-3 run
expected: With a FRESH `nono.exe` (with the `agent launch` subcommand) on PATH, admin elevation, and the nono-wfp-service + nono-agentd running, run `pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny`. SC-2 (fail-secure: malformed key → non-zero daemon exit) already PASSES on the dev host. SC-3 (dual-layer deny: per-agent WFP block filter present after `agent launch`) PASSES. NOTE: the dev host has a STALE `C:\Program Files\nono\nono.exe` without `agent` — prepend `target\release` to PATH or install the post-Phase-83 build first. (WR-03 deferred: SC-3's proxy-layer proof is currently structural/WFP-inferred, not a live HTTP probe.)
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
