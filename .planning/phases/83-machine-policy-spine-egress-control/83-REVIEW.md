---
phase: 83-machine-policy-spine-egress-control
reviewed: 2026-06-18T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - crates/nono/src/machine_policy.rs
  - crates/nono/src/error.rs
  - crates/nono/src/lib.rs
  - crates/nono/src/net_filter.rs
  - crates/nono/Cargo.toml
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/agent_daemon/mod.rs
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/bin/nono-agentd.rs
  - crates/nono-cli/data/network-policy.json
  - scripts/build-windows-msi.ps1
  - scripts/gates/egress-policy-deny.ps1
  - scripts/validate-windows-msi-contract.ps1
findings:
  critical: 2
  warning: 5
  info: 3
  total: 10
status: issues_found
---

# Phase 83: Code Review Report

**Reviewed:** 2026-06-18
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Phase 83 wires a machine-level egress-policy spine (`HKLM\SOFTWARE\Policies\nono`)
into the daemon startup path, with fail-secure reader semantics, a single-source
proxy/WFP derivation, and an ADMX template generated from PowerShell here-strings.

The **fail-secure reader path itself is sound**: `read_machine_egress_policy_impl`
correctly returns `Ok(None)` only on `ERROR_FILE_NOT_FOUND`, aborts with
`PolicyLoadFailed` on any other open error and on malformed REG types, and the
daemon callers (`resolve_machine_egress_policy`, both `nono-agentd.rs` startup
paths) propagate the error with `?`/`match`-to-`Err` — no `.ok()`/`unwrap_or`
on the read path. The DNS-component matching in `HostFilter` (for `*.`-prefixed
entries) is component-safe and well-tested.

However, two BLOCKER-class defects break the end-to-end egress story:

1. The single most consequential bug: GPO-configured **suffix** entries
   (`AllowedSuffixes`, documented as leading-dot `.anthropic.com`) are silently
   treated as **exact hosts** by `HostFilter::new`, so a fleet allowlist of
   suffixes matches *nothing* and denies all otherwise-allowed traffic.
2. The machine MSI **always** creates the `HKLM\SOFTWARE\Policies\nono` key with
   an `InstalledByMsi` sentinel value, but the reader treats *key-present* as
   "enforcement active." Every machine-MSI install therefore flips the daemon to
   strict deny-all egress even when the admin configured no egress policy — the
   documented "absent → fall through to per-user" branch is dead on any machine
   install.

Plus an ADMX correctness defect (named-toggle policies missing the required
`valueName` attribute), allowlist-drift via duplicated token-expansion logic, and
a gate that claims dual-layer proof while only checking one layer.

## Critical Issues

### CR-01: GPO suffix allowlist (`AllowedSuffixes`) never matches — leading-dot suffixes become exact hosts

**File:** `crates/nono/src/net_filter.rs:131-151` (with `crates/nono-cli/src/agent_daemon/mod.rs:365` and `scripts/build-windows-msi.ps1:799-809,878-882`)

**Issue:**
`HostFilter::new` only routes an entry into the wildcard-suffix bucket when it
starts with `*`:

```rust
if let Some(suffix) = lower.strip_prefix('*') {
    suffixes.push(suffix.to_string());   // "*.anthropic.com" -> ".anthropic.com"
} else {
    exact.push(lower);                   // ".anthropic.com" -> exact host (BUG)
}
```

The ADMX template (and its explain/presentation strings) instruct admins to enter
suffixes **with a leading dot and no `*`**: `.anthropic.com`
(`build-windows-msi.ps1:799-809, 880`). Those values are written to
`AllowedSuffixes`, read verbatim into `MachineEgressPolicy.allowed_suffixes`, and
flattened by `raw_allowlist()` into the allowlist that
`resolve_machine_egress_policy` (mod.rs:365) hands to `ProxyFilter::new_strict`
(`nono-agentd.rs:369`). No step ever prepends `*`.

Result: a `.anthropic.com` suffix lands in `allowed_hosts` (exact). In
`check_host`, `allowed_hosts.contains(&"api.anthropic.com")` is false and the
suffix loop never runs (the entry is not in `allowed_suffixes`). Every subdomain
the admin intended to allow is denied. A fleet that configures egress purely via
the `AllowedSuffixes` GPO list gets **total egress denial** for all intended
hosts. (Note: the preset *tokens* path is unaffected — the embedded
`network-policy.json` presets use `*.anthropic.com` with the `*`, so token
expansion produces correctly-matching entries. The bug is specific to the
admin-facing suffix list.)

**Fix:** Normalize suffix entries at the machine-policy boundary so a leading-dot
suffix becomes a wildcard the `HostFilter` understands, OR teach `HostFilter::new`
to also treat leading-dot entries as suffixes. Prefer normalizing at the source
of truth so both proxy and any future WFP/L7 consumer agree:

```rust
// In MachineEgressPolicy::raw_allowlist (or resolve_machine_egress_policy),
// convert ".anthropic.com" -> "*.anthropic.com" so HostFilter buckets it as a suffix.
out.extend(self.allowed_suffixes.iter().map(|s| {
    if s.starts_with("*.") { s.clone() }
    else if let Some(rest) = s.strip_prefix('.') { format!("*.{rest}") }
    else { format!("*.{s}") }   // bare "anthropic.com" treated as a suffix per ADMX intent
}));
```
Add a test mirroring `sc4_dns_component_matrix` but seeded from a `.anthropic.com`
(leading-dot) suffix to lock the contract.

### CR-02: Machine MSI always creates the policy key, so every machine install forces strict deny-all egress

**File:** `scripts/build-windows-msi.ps1:302-315` (with `crates/nono/src/machine_policy.rs:184-206` and `crates/nono-cli/src/bin/nono-agentd.rs:349-393`)

**Issue:**
The machine-scope MSI **unconditionally** creates
`HKLM\SOFTWARE\Policies\nono` with a sentinel `InstalledByMsi` value
(`cmpPolicySentinel`, lines 305-315), and `validate-windows-msi-contract.ps1:331`
asserts this key is always present.

The reader's contract (machine_policy.rs:9-17, 184-206) is *key-present →
`Ok(Some(policy))` → enforcement active*; only an absent key yields `Ok(None)`
(fall-through to per-user). Because the MSI always creates the key, the absent
branch is **unreachable on any machine-MSI install**. With no
`AllowedSuffixes`/`AllowedHosts`/`PresetTokens` subkeys configured,
`parse_policy` returns an *empty* `MachineEgressPolicy`, `resolve_machine_egress_policy`
reports `machine_enforcement_active = true` with an **empty** allowlist, and
`build_daemon_state` (nono-agentd.rs:366-371) starts the proxy with
`strict_filter = true` + `allowed_hosts = []` → `ProxyFilter::new_strict([])` →
**deny-all egress for every agent**, plus per-agent WFP `proxy-only` filters.

So merely installing the machine MSI (with no GPO configured at all) silently
converts every confined agent to zero network egress. This is "fail-secure" in
direction but is a behavioral trap that contradicts the documented design and
will strand fleets that installed the MSI before pushing any GPO. It is a
data-/function-loss-class defect for the intended workflow.

**Fix:** Make "enforcement active" depend on *configured egress values*, not on
mere key presence. Options:
- Reader: return `Ok(None)` when the key exists but contains no
  `AllowedSuffixes`/`AllowedHosts`/`PresetTokens` subkeys AND no egress values
  (treat the bare sentinel key as "present but unconfigured = not enforcing").
  Keep malformed-subkey → `Err` unchanged.
- OR gate `machine_enforcement_active` in `resolve_machine_egress_policy` on a
  non-empty effective allowlist / an explicit `EgressEnforcement` value the MSI
  does *not* set, so the sentinel alone never activates strict mode.

Whichever path is chosen, add a test: sentinel key present, no egress subkeys →
`machine_enforcement_active == false` and per-user fall-through preserved.

## Warnings

### WR-01: ADMX named-toggle policies lack the required `valueName` attribute — preset toggles likely won't write the token

**File:** `scripts/build-windows-msi.ps1:698-761`

**Issue:** Each preset toggle (`AllowAnthropicPreset`, `AllowOpenAIPreset`,
`AllowGitHubAPIPreset`) declares `<enabledValue><string value="anthropic"/></enabledValue>`
and `<disabledValue><delete/></disabledValue>` but the `<policy>` element has **no
`valueName` attribute**. In ADMX, `enabledValue`/`disabledValue` write to the
policy's `valueName`; without it the Group Policy editor rejects the policy or has
no registry value to set. The `<text>` element underneath (`valueName="anthropic"`)
writes a *separate, admin-typed* string value, not the enable/disable token. The
intended behavior ("enabling writes token `anthropic`") is not expressed by valid
ADMX. The MSI-contract validator only greps for `"anthropic"` substrings
(validate-windows-msi-contract.ps1:459-464), so it cannot catch this.

**Fix:** Add `valueName="<token>"` to each toggle `<policy>` element (or restructure
so the enabledValue targets a deterministic value name under `PresetTokens`), and
extend the dev-host contract check or the Windows-host generated-XML validation to
parse the ADMX and assert each toggle policy carries a `valueName` plus a matching
`enabledValue`.

### WR-02: Duplicated preset-token expansion logic invites allowlist drift between CLI and daemon

**File:** `crates/nono-cli/src/policy.rs:1450-1480` and `crates/nono-cli/src/agent_daemon/mod.rs:290-326`

**Issue:** Two independent implementations expand preset tokens to FQDNs:
`policy::expand_egress_preset_tokens` (uses the typed `network_policy::load_network_policy`)
and `agent_daemon::expand_preset_tokens_from_embedded` (hand-rolls `serde_json::Value`
navigation). They read the same embedded JSON but apply different inclusion rules —
notably `expand_egress_preset_tokens` documents excluding `suffixes` (policy.rs:1463-1466)
while the daemon version only ever reads `hosts` (mod.rs:307). If a future preset
group adds a `suffixes` field, the two layers will diverge, violating the
single-source-of-truth requirement. Compounding this, `expand_egress_preset_tokens`
is `#[allow(dead_code)]` and its doc claims it is "called by Plan 83-02 daemon-startup
wiring" (policy.rs:1445-1448) — but the daemon actually calls
`expand_preset_tokens_from_embedded`, so the CLI function is genuinely uncalled.

**Fix:** Collapse to one expansion function used by both call sites (have the daemon
call `crate::policy::expand_egress_preset_tokens`, or move the canonical logic into
the `nono`/shared module). Remove the dead `#[allow(dead_code)]` function or wire it
in. Correct the misleading doc-comment.

### WR-03: `egress-policy-deny` gate claims dual-layer proof but never probes the proxy (L7) layer

**File:** `scripts/gates/egress-policy-deny.ps1:381-399`

**Issue:** SC-3 is documented as "BOTH (a) the proxy denies a request to an
out-of-list host AND (b) WFP block filter present." The implementation sets
`$proxyLayerActive = $wfpBlockPresent` (line 399) and only ever inspects the WFP
filter dump — it makes **no actual request through the proxy** to an out-of-list
host. The PASS verdict (line 417) asserts "proxy denies an out-of-list host" that
was never tested; the L7 deny is *inferred* from the WFP block's presence. This is
a weaker proof than the gate's own contract and the Dark Factory "wired, not just
coded" mandate claim. (Compliance note: the gate correctly returns a verdict object
and never calls `exit` or `Persist-Verdict` — that part is fine.)

**Fix:** Add a real loopback probe: issue an HTTP/CONNECT through the proxy port to
`evil.example.com` and assert a deny, and a control request to an in-list host
(`api.anthropic.com`) to assert allow. Only set `proxyLayerActive` from the observed
proxy decision, then require both layers independently for PASS.

### WR-04: `build_daemon_capability_set` shells out to `where` and unconditionally trusts its first line

**File:** `crates/nono-cli/src/agent_daemon/mod.rs:179-231`

**Issue:** For each profile interpreter, the daemon runs `Command::new("where").arg(interp_name)`
and grants `Read` on the parent of the first line of stdout. `where` is resolved via
the daemon's own PATH; if PATH is attacker-influenced (or a `where.exe` shim exists in
a writable PATH dir), the resolved interpreter directory — and thus the read grant
handed to the confined AppContainer — is attacker-controlled. The interpreter names
themselves come from embedded policy (trusted), but the *resolution mechanism* is not
pinned. This widens the read surface of the sandbox via an external, PATH-dependent
process.

**Fix:** Resolve interpreters via an absolute, fixed mechanism (e.g.
`SearchPathW` with a controlled search order, as `resolve_exe_path` already does for
the engine exe) rather than spawning `where`, and canonicalize + validate the result
is under an expected root before granting. At minimum, invoke `where.exe` by absolute
`%SystemRoot%\System32\where.exe` path.

### WR-05: `build_daemon_capability_set` swallows `SystemRoot` / canonicalize failures and silently broadens or mis-scopes grants

**File:** `crates/nono-cli/src/agent_daemon/mod.rs:140-147, 159-164`

**Issue:** Two security-relevant fallbacks degrade silently rather than failing:
- exe-parent canonicalize failure falls back to the *unresolved* path (lines 140-147)
  — if the path contains an unresolved symlink/junction, the granted read scope may
  differ from the real load path (TOCTOU-adjacent; CLAUDE.md "trusting resolved paths").
- `%SystemRoot%` unset falls back to hardcoded `C:\Windows` (lines 159-164). On a host
  with a non-default system root this grants the wrong directory and mis-scopes the CLR
  baseline. Per CLAUDE.md ("Validate environment variables before use; never assume
  HOME/TMPDIR/etc. are trustworthy" and "Fail Secure"), an unset/relocated SystemRoot
  should be validated, not silently defaulted.

**Fix:** Treat canonicalize failure of a path that will become a sandbox grant as fatal
(return `Err`), and resolve `%SystemRoot%` via the validated environment (or
`GetWindowsDirectoryW`) returning `Err` if absent, rather than guessing `C:\Windows`.

## Info

### IN-01: `MachineEgressPolicy` derives `Serialize`/`Deserialize` but no field validation on deserialize

**File:** `crates/nono/src/machine_policy.rs:52-73`

**Issue:** The struct accepts arbitrary `Vec<String>` for suffixes/hosts/tokens with no
validation (empty strings, embedded whitespace, non-DNS values pass through). Given
the registry reader path validates only REG type, malformed-but-typed values (e.g. a
blank string) flow into the allowlist. Low risk today (HostFilter lowercases and
compares), but worth a normalization/validation pass to reject obviously invalid host
strings at the boundary.

**Fix:** Add a `validate()` / normalization step (trim, reject empty, basic DNS-label
sanity) called after read, surfaced as `PolicyLoadFailed` on violation.

### IN-02: `expand_egress_preset_tokens` doc-comment is stale/misleading

**File:** `crates/nono-cli/src/policy.rs:1445-1448`

**Issue:** The comment states the function "is called by Plan 83-02 daemon-startup wiring
(agent_daemon startup path)," but the daemon path uses
`expand_preset_tokens_from_embedded` instead, leaving this function uncalled (hence the
`#[allow(dead_code)]`). Misleading provenance comments make future audits harder.

**Fix:** Either wire the daemon to this function (see WR-02) or update the comment to
reflect that it is the CLI-layer counterpart and document its actual caller.

### IN-03: SC-3 launch-SID regex differs subtly between gate helpers

**File:** `scripts/gates/egress-policy-deny.ps1:92,101`

**Issue:** `Get-NonoBlockSids` matches `S-1-15-2-[\d-]+` while `Get-LaunchSid` matches
`S-1-15-2[^\s]+` (note: no hyphen after `15-2` in the second, and a greedy
non-whitespace tail). The two patterns can capture slightly different SID strings,
which could make the `$afterLaunch -contains $sid` membership test (line 379) fail to
match even when the filter exists. Cosmetic today but a latent false-FAIL source.

**Fix:** Use one shared, anchored SID regex (`S-1-15-2(?:-\d+)+`) in both helpers.

---

_Reviewed: 2026-06-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
