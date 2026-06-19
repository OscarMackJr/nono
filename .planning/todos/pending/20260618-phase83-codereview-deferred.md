# TODO: Phase 83 code-review — deferred warnings/info (WR-02/03/04/05, IN-01/03)

**Captured:** 2026-06-18 (Phase 83 code-review gate; CR-01/CR-02/WR-01/IN-02 fixed inline, these deferred)
**Severity:** medium (WR-03, WR-04, WR-05) / low (WR-02, IN-01, IN-03)
**Source:** `.planning/phases/83-machine-policy-spine-egress-control/83-REVIEW.md`
**Resolves phase:** unassigned — carry-forward (candidate: a v3.0 hardening/polish phase or Phase 84 adjacent)

## Deferred findings (not addressed in Phase 83 gap-closure)

- **WR-02 — duplicated preset-token expansion (drift risk).** `policy::expand_egress_preset_tokens`
  (typed loader) and `agent_daemon::expand_preset_tokens_from_embedded` (hand-rolled serde_json)
  apply different inclusion rules (CLI excludes `suffixes`; daemon only reads `hosts`). If a preset
  group ever adds a `suffixes` field the two layers diverge — violates single-source-of-truth.
  Collapse to one canonical expander (move into shared `nono`/embedded module so the standalone
  `nono-agentd` bin can reach it), remove the dead `#[allow(dead_code)]` CLI fn or wire it.
  Files: `crates/nono-cli/src/policy.rs:1450-1480`, `crates/nono-cli/src/agent_daemon/mod.rs:290-326`.

- **WR-03 — `egress-policy-deny` gate infers L7 deny from WFP presence (host-gated).** SC-3 claims
  dual-layer proof but only inspects the WFP filter dump; it makes no real request through the proxy.
  Add a live loopback probe (CONNECT through the proxy port to an out-of-list host → assert deny;
  control to `api.anthropic.com` → assert allow) and set `proxyLayerActive` from the observed proxy
  decision, requiring both layers independently for PASS. Needs a provisioned host with the daemon +
  proxy running. File: `scripts/gates/egress-policy-deny.ps1:381-399`.

- **WR-04 — `build_daemon_capability_set` trusts PATH-resolved `where` first line.** Interpreter dir
  granted to the confined AppContainer is resolved via `Command::new("where")` over the daemon's PATH;
  a `where.exe` shim in a writable PATH dir → attacker-controlled read grant. Resolve via absolute
  `%SystemRoot%\System32\where.exe` or `SearchPathW` (as `resolve_exe_path` does), canonicalize +
  validate under an expected root. File: `crates/nono-cli/src/agent_daemon/mod.rs:179-231`.

- **WR-05 — `build_daemon_capability_set` swallows canonicalize / `%SystemRoot%` failures.** exe-parent
  canonicalize failure falls back to the unresolved path (TOCTOU-adjacent); unset `%SystemRoot%` falls
  back to hardcoded `C:\Windows` (mis-scopes CLR baseline on relocated roots). Per CLAUDE.md fail-secure
  + "validate env vars," treat both as fatal `Err` / resolve via `GetWindowsDirectoryW`.
  File: `crates/nono-cli/src/agent_daemon/mod.rs:140-147, 159-164`.

- **IN-01 — `MachineEgressPolicy` has no deserialize-time field validation.** Empty/whitespace/non-DNS
  strings pass through. Add a `validate()`/normalization step (trim, reject empty, basic DNS-label
  sanity) surfaced as `PolicyLoadFailed` on violation. File: `crates/nono/src/machine_policy.rs`.

- **IN-03 — gate SID regexes diverge.** `Get-NonoBlockSids` uses `S-1-15-2-[\d-]+` vs `Get-LaunchSid`
  `S-1-15-2[^\s]+` — latent false-FAIL on the membership test. Use one anchored `S-1-15-2(?:-\d+)+`.
  File: `scripts/gates/egress-policy-deny.ps1:92,101`.

## Note
CR-01 (leading-dot suffix normalization), CR-02 (sentinel-key-not-enforcing), WR-01 (ADMX valueName),
and IN-02 (stale doc) were FIXED inline during Phase 83 gap-closure (commits 8be03a95, b47ea26f,
56982d72, e8748421). These six remain.
