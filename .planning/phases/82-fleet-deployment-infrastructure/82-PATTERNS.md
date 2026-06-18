# Phase 82: Fleet Deployment Infrastructure - Pattern Map

**Mapped:** 2026-06-18
**Files analyzed:** 8 (3 new, 5 modified)
**Analogs found:** 8 / 8

## Scope note (read first)

The static-CRT flag (D-01/D-02) is **already in `.cargo/config.toml`** (`[target.x86_64-pc-windows-msvc] rustflags = ["-C", "target-feature=+crt-static"]`, with the CI/RUSTFLAGS-override caveat documented in-file). Planner should treat DEPLOY-01's CRT half as a **verify-only** task (assert the stanza + assert CI's step-level `RUSTFLAGS` carries it), not a write. The remaining Phase 82 work is all in `build-windows-msi.ps1`, `validate-windows-msi-contract.ps1`, a new `health` subcommand + first-run provisioner in `nono-cli`, and a new `scripts/gates/deploy-silent-install.ps1` gate.

Library/CLI boundary (CLAUDE.md): **everything lands in `nono-cli` and the build scripts.** The `nono` library stays policy-free — no registry reads, no cert logic, no health command. The R-B3 SID/ownership primitives are the one library surface this phase consumes (already exported from `nono::`).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `scripts/build-windows-msi.ps1` (MOD) | config / build | transform (here-string → .wxs) | self (existing here-string anchors :225-404) | exact (extend in place) |
| `scripts/validate-windows-msi-contract.ps1` (MOD) | test | transform / assert | self (existing assertions :195-249) | exact (extend in place) |
| `.cargo/config.toml` (VERIFY) | config | n/a | self (stanza already present) | exact (already done) |
| `crates/nono-cli/src/health.rs` or `health_runtime.rs` (NEW) | command | request-response (read-only diagnostic) | `crates/nono-cli/src/classify_runtime.rs` | role+flow exact |
| first-run provisioner (NEW; e.g. `crates/nono-cli/src/provision_windows.rs`) | service / utility | file-I/O + event-driven (first-run trigger) | `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` (R-B3 primitives) + `setup.rs` (cert/idempotent steps) | role-match |
| `crates/nono-cli/src/cli.rs` (MOD) | config | n/a (clap defs) | `Commands::Classify` / `ClassifyArgs` (:774, :3165) + `SetupArgs` (:2569) | exact |
| `crates/nono-cli/src/app_runtime.rs` (MOD) | route | request-response (dispatch) | `Commands::Classify =>` arm (:61-66) | exact |
| `crates/nono-cli/src/command_runtime.rs` (MOD) | controller | request-response (run path) | `run_sandbox` (:76) — hook provisioner before exec_strategy | role-match |
| `scripts/gates/deploy-silent-install.ps1` (NEW) | test | event-driven (unattended verdict) | `scripts/gates/clean-host-install.ps1` | exact |

## Pattern Assignments

### `scripts/build-windows-msi.ps1` (config/build, here-string transform)

**Analog:** self — extend the existing here-strings; do NOT author raw `.wxs` (memory: `windows_msi_wxs_is_generated`; the emitted `dist/windows/*.wxs` is overwritten every build).

**Machine PATH `Environment` element** (`:387-403`) — DEPLOY-02 is already wired (`System="$($scopeInfo.SystemPath)"`, `Part="last"`). Verify it satisfies "fresh shell sees `nono`" rather than re-author:
```xml
      <Component Id="cmpPath" Guid="*">
        <RegistryValue Root="$($scopeInfo.RegistryRoot)" Key="Software\always-further\nono\$Scope"
            Name="InstallDir" Type="string" Value="[INSTALLFOLDER]" KeyPath="yes" />
        <Environment Id="EnvPath" Name="PATH" Action="set" Part="last"
            Permanent="no" System="$($scopeInfo.SystemPath)" Value="[INSTALLFOLDER]" />
      </Component>
```

**Non-fatal ServiceInstall** (`:229-250`) — DEPLOY-04 already shipped (`Vital="no"` on `ServiceInstall`, `ErrorControl="ignore"`). Copy this exact shape for any new non-fatal install element. Note the in-file D-04 comment: `Vital` (not `ServiceControl/@Wait`) is the rollback-suppressing knob.

**Event Log source via raw `RegistryKey`** (`:259-275`) — the proven `EventMessageFile`/`TypesSupported=7` pattern. If the planner registers a **`nono` CLI** Event Log source now (D-Discretion / forward-looking for Phase 84), clone this component verbatim with `Key="...\EventLog\Application\nono"` and `Value="[INSTALLFOLDER]nono.exe"`:
```xml
      <Component Id="cmpEventLogSource" Guid="*">
        <RegistryKey Root="HKLM" Key="SYSTEM\CurrentControlSet\Services\EventLog\Application\nono-wfp-service">
          <RegistryValue Name="EventMessageFile" Type="string" Value="[INSTALLFOLDER]nono-wfp-service.exe" KeyPath="yes" />
          <RegistryValue Name="TypesSupported" Type="integer" Value="7" />
        </RegistryKey>
      </Component>
```

**Deferred SYSTEM-context CustomAction** (`:322-333`) — the cert-import CA (D-04) copies this exact shape. The existing WFP-uninstall CA is the template for a `Directory="INSTALLFOLDER"` `ExeCommand` running `Execute="deferred" Impersonate="no"` (= LocalSystem). For the cert import, mirror the structure but `Return` policy must be **non-fatal** (D-04: cert failure does not roll back; `nono health` reports degraded) — use `Return="ignore"` exactly as the uninstall CA does, and condition it on install (not `REMOVE=ALL`). Stage the `.cer`/`.pem` into the harvest as a new `<File>` component (Claude's Discretion on exact staging).

**MajorUpgrade / UpgradeCode** (`:343-348`) — already present; no change. Per-scope `UpgradeCode` comes from `$scopeInfo`.

**ProgramData root (D-08 / Pitfall 4):** add a machine-global `%PROGRAMDATA%\nono\` directory component (SYSTEM/Admins-writable). The MSI must create ONLY this — **never** user scratch (`%LOCALAPPDATA%`), which under SYSTEM resolves to `C:\Windows\system32\config\systemprofile\...` and fails every user's R-B3 guard. Model the directory on the existing `$scopeInfo.DirectoryXml` fragment usage (`:364`).

---

### `scripts/validate-windows-msi-contract.ps1` (test, assert-on-generated-WiX)

**Analog:** self — extend the existing `Assert-Equal`/`Assert-True` block. The generator is invoked with `EmitOnly=$true` and the result parsed as `[xml]`; assertions select by `local-name()`.

**Existing assertion idiom** (`:195-249`): the service + Event Log contract is already enforced here. Copy these patterns for new elements:
```powershell
$machineServiceInstall = Get-FirstNodeByLocalName -Document $machineDoc -LocalName "ServiceInstall"
Assert-Equal -Actual $machineServiceInstall.Vital -Expected "no" -Message "...non-fatal per D-04..."

# Event Log source: iterate RegistryKey nodes, match by .Key, assert child RegistryValue
$machineRegistryKeys = $machineDoc.SelectNodes("//*[local-name()='RegistryKey']")
$eventMessageFileNode = $machineEventLogKey.SelectSingleNode("*[local-name()='RegistryValue' and @Name='EventMessageFile']")
Assert-True -Condition ($null -ne $eventMessageFileNode) -Message "..."
```
**New assertions to add:** ProgramData directory present in machine doc only (string-contains on `$machineDoc.OuterXml`, mirroring the `ProgramFiles64Folder`/`LocalAppDataFolder` checks at `:175-176`); cert-import CustomAction present (`SelectNodes("//*[local-name()='CustomAction']")`, assert `Execute='deferred'`, `Impersonate='no'`, non-fatal `Return`); user MSI must NOT contain the machine-only cert/ProgramData/EventLog elements (mirror the user-MSI-must-not-contain-ServiceInstall negative assertions at `:221-227`).

---

### `crates/nono-cli/src/health.rs` (NEW; command, read-only request-response)

**Analog:** `crates/nono-cli/src/classify_runtime.rs` — same shape: a read-only diagnostic with parallel human + `--json` output, a typed `Outcome` enum, fail-secure on query error.

**Outcome enum + dispatch** (`classify_runtime.rs:74-95`):
```rust
let outcome = match sid_result {
    Err(_) => Outcome::NotFound,          // fail-secure default
    Ok(None) => Outcome::NoAppContainer,
    Ok(Some(sid)) => { /* ... */ }
};
if args.json { print_json(pid, &outcome)?; } else { print_human(pid, &outcome); }
Ok(())
```
For `health`, model a per-subsystem state enum (D-07's four groups: install+version, WFP service via SCM, machine policy `HKLM\SOFTWARE\Policies\nono`, scratch+cert+PATH) and aggregate to the D-06 tri-state.

**JSON output (always-print contract, D-06)** (`classify_runtime.rs:127-145`):
```rust
let value = serde_json::json!({ "pid": pid, "verdict": verdict, "authoritative": false, /* ... */ });
let rendered = serde_json::to_string_pretty(&value)
    .map_err(|e| NonoError::ConfigParse(format!("JSON serialization failed: {e}")))?;
println!("{rendered}");
```
**Tri-state exit (D-06 — 0 healthy / 1 degraded / 2 broken):** `classify` does NOT vary its exit code, so for that piece the closest analog is `main.rs:151-170`, where `run_cli`'s `Err` maps to `std::process::exit(2)` / `exit(1)`. `health` must print the JSON to stdout on **every** branch and then return a status the dispatcher converts to the right exit code. **Recommended:** introduce a typed health verdict and have `run_cli`/`main` map it (do NOT call `process::exit` from inside the command body — keep the `Result`-returning convention; thread the tri-state out via a dedicated return or a `NonoError` variant the way `ActionRequired` is special-cased at `main.rs:155`).

**Machine policy read (group c):** Phase 82 only checks **presence/readability** of `HKLM\SOFTWARE\Policies\nono` (configured / not-configured / unreadable) — it does NOT parse the egress policy (that is Phase 83). There is no winreg dep yet in `nono-cli`; the existing `reg.exe`-subprocess approach in `platform.rs` is acceptable for this presence probe (research SUMMARY notes winreg replaces it only on the hot policy path in Phase 83). Apply `KEY_WOW64_64KEY` semantics if using raw FFI (Pitfall 7) — but a presence probe via `reg query` reads the 64-bit hive from a 64-bit process by default.

---

### first-run provisioner (NEW; e.g. `crates/nono-cli/src/provision_windows.rs`) (service/utility, file-I/O + first-run-trigger)

This is the D-09 unifying module: one idempotent first-run-in-user-context provisioner doing (1) user-owned `WRITE_OWNER` scratch, (2) `CurrentUser\Root` cert import, (3) `NODE_EXTRA_CA_CERTS`. Single idempotency check, single failure surface that `nono health` reads.

**Analog A — R-B3 user-ownership primitives:** `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs`. It consumes the library functions the scratch grant needs (all already `pub` from `nono::`):
```rust
use nono::{
    grant_sid_read_attributes_on_path, grant_sid_traverse_on_path, grant_sid_write_on_path,
    path_is_owned_by_current_user, revoke_sid_on_path, AccessMode, Result, WindowsFilesystemPolicy,
};
```
The **ownership gate** pattern (`dacl_guard.rs:113-137`) — `path_is_owned_by_current_user` returning `Ok(false)` → skip+warn, `Err(_)` → fail-closed (never swallow), `Ok(true)` → proceed — is exactly the discipline the scratch-create needs. For first-run scratch the provisioner CREATES `%LOCALAPPDATA%\nono\<workspace>` then makes the invoking user the owner so the R-B3 guard later passes. The test helper `take_ownership_for_current_user` (`dacl_guard.rs:506-527`) shows the canonical `icacls <path> /setowner <whoami> /Q` invocation — the production scratch path should set ownership to the invoking user the same way (memory `windows_mandatory_label_write_owner`: `%LOCALAPPDATA%`/`%USERPROFILE%`/`%TEMP%` support WRITE_OWNER; drive-root `C:\poc\*` does NOT).

**Analog B — idempotent step + cert/Authenticode logic:** `crates/nono-cli/src/setup.rs`. The `SetupRunner::run` step sequence (`:182-269`) is the model for an ordered, each-step-idempotent, admin-gated provisioning flow. Note the short-circuit pattern for one-time-admin actions (`grant_ancestors`, `:191-194`) and the admin guard (`:306-309`). The Authenticode primitive the broker self-trust gate uses lives in `crates/nono-cli/src/exec_identity_windows.rs` (`query_authenticode_status`, `WinVerifyTrust`, `AuthenticodeStatus` enum, `:65-119`) — the cert import (D-04 machine `Root`+`TrustedPublisher`, D-05 per-user `CurrentUser\Root`) must make that gate return `Valid`. Per code_context, factor the cert-import logic as **testable Rust reused by both the MSI CA and first-run** (the `nono setup`/`--trust-broker` verb concept). There is currently NO `certutil`/`addstore`/`CurrentUser\Root` import code in `nono-cli` — this is net-new; shell out to `certutil -addstore -f Root|TrustedPublisher` (machine, run by the MSI CA under SYSTEM) and to a per-user `CurrentUser\Root` import (first-run, user context).

**Idempotency marker (Claude's Discretion):** registry sentinel vs filesystem marker under `%LOCALAPPDATA%\nono\`. Either way, mirror the "already present → no-op" idempotent check shown in `exec_identity.rs::grant_ancestors_for_path` (`:110-122`).

**Hook point:** the provisioner is invoked on the `nono run` entry path **before** sandbox apply. The run path is `command_runtime::run_sandbox` (`command_runtime.rs:76`); the workspace/cwd is resolved around `:125` (`std::env::current_dir()`). Insert the first-run provisioner call after cwd resolution and before `exec_strategy` builds the confined child. Fail-NON-fatal (provisioning failures are reported by `nono health`, not abort — per code_context the fail-secure rule is Phase 83's policy reader, not Phase 82 provisioning).

---

### `crates/nono-cli/src/cli.rs` (config, clap defs)

**Analog:** the `Commands::Classify` variant (`:755-774`) for a simple read-only verb, and `ClassifyArgs` (`:3165`) for the args struct with a `--json` bool. Add a `Health(HealthArgs)` variant under the GETTING STARTED / EXPLORATION grouping with the standard `help_template`/`after_help` block. `HealthArgs` should carry `pub json: bool` (`#[arg(long, help_heading = "OPTIONS")]`) — the established pattern repeated dozens of times in this file. If the first-run provisioner needs an explicit `nono setup --provision`-style verb, extend `SetupArgs` (`:2569`) following the `register_wfp_service`/`grant_ancestors` bool-flag pattern (`:2576`, `:2626`), including the `requires =`/`conflicts_with =` constraints idiom.

---

### `crates/nono-cli/src/app_runtime.rs` (route, dispatch)

**Analog:** the `Commands::Classify` dispatch arm (`:61-66`):
```rust
Commands::Classify(args) => {
    run_command_with_update(update_handle, silent, || run_classify(args))
}
```
Add `Commands::Health(args) => run_command_with_update(update_handle, silent, || run_health(args))`. Because `health` needs a tri-state exit (D-06), the simplest analog-faithful approach is to let `run_health` return its verdict and have the outer `run_cli`/`main` map it to exit 0/1/2 (mirror the `ActionRequired` special-case at `main.rs:155-167`), rather than calling `process::exit` mid-command.

---

### `scripts/gates/deploy-silent-install.ps1` (NEW; test, unattended verdict)

**Analog:** `scripts/gates/clean-host-install.ps1` — a near-exact template for the new gate. Same two-function contract (`Test-Precondition` → `$null` | reason-string; `Invoke-Gate` → ordered verdict dict), same "gate RETURNS verdict, NEVER calls exit or Persist-Verdict" rule, same WR-01 (no stray pipeline output) discipline.

**Precondition pattern** (`clean-host-install.ps1:65-98`): elevation check → dirty-host check → MSI-staged check, each returning a reason string for SKIP_HOST_UNAVAILABLE:
```powershell
$identity  = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([...]::Administrator)) { return '...requires elevation...' }
if (Test-Path -LiteralPath 'C:\Program Files\nono\nono.exe') { return '...host is not clean...' }
if (-not (Test-Path -LiteralPath $script:MsiPath)) { return "MSI not found at $($script:MsiPath)..." }
return $null
```

**Install + version + service-state + uninstall flow** (`clean-host-install.ps1:100-188`): `msiexec /i ... /quiet /norestart /l*v <log>` with `$installOk = ($exit -eq 0 -or $exit -eq 3010)`; then `nono --version` from a **NEW pwsh session** (proves PATH propagation per D-06); service state recorded in `detail` non-fatally; `msiexec /x` cleanup. Each `Start-Process` result assigned to a named variable (WR-01).

**What the new gate must add beyond clean-host-install** (per CONTEXT `<specifics>` + Pitfall 4/5/13):
- Install under a **non-admin target user context** and assert the first-run scratch path is owned by **that user, not SYSTEM** (Pitfall 4 / D-08).
- **Degraded-service path** → assert `nono health` exits non-zero (Pitfall 5 / success criterion 5).
- **TLS-through-proxy trust** verified from three client types: PowerShell (CryptoAPI), Node.js, and nono-cli (rustls/native-certs) (Pitfall 13 / D-05).

Gate is auto-discovered by `scripts/verify-dark.ps1` (drop the `.ps1` into `scripts/gates/`; the runner globs the dir — no registration needed). The runner owns exit mapping (PASS=0/FAIL=2/SKIP=3/harness=4) and persist-before-emit.

## Shared Patterns

### R-B3 user-ownership / WRITE_OWNER
**Source:** library exports `nono::{grant_sid_write_on_path, path_is_owned_by_current_user, revoke_sid_on_path, grant_sid_traverse_on_path, grant_sid_read_attributes_on_path}`; consumer pattern in `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:113-137`; ownership-set idiom in `dacl_guard.rs:506-527` (`icacls /setowner`).
**Apply to:** first-run scratch provisioner. `%LOCALAPPDATA%`/`%USERPROFILE%`/`%TEMP%` support WRITE_OWNER; drive-roots do not (memory `windows_mandatory_label_write_owner`).

### Fail-closed vs non-fatal split
**Source:** `dacl_guard.rs` (ownership-check `Err` → revert + propagate, NEVER swallow) for security-critical paths; `build-windows-msi.ps1:319-329` (`Return="ignore"`) for non-fatal install-time actions.
**Apply to:** cert import + service + scratch are **non-fatal to install** and reported by `nono health` (degraded). The CLAUDE.md "config load failures must be fatal" rule applies to the Phase 83 policy reader, NOT Phase 82 provisioning (per code_context).

### Read-only JSON diagnostic with parallel human output
**Source:** `crates/nono-cli/src/classify_runtime.rs:89-145` (typed `Outcome` → `print_json` / `print_human`; `serde_json::json!` + `to_string_pretty`; map serde error to `NonoError::ConfigParse`).
**Apply to:** `nono health`.

### Verify-dark gate two-function contract
**Source:** `scripts/gates/clean-host-install.ps1` (full template) + `scripts/verify-dark.ps1` (auto-discovery, exit mapping, persist-before-emit). Gate header comment block at `clean-host-install.ps1:1-27` states the contract verbatim.
**Apply to:** `scripts/gates/deploy-silent-install.ps1`.

### MSI here-string is the source; .wxs is generated
**Source:** memory `windows_msi_wxs_is_generated`; `build-windows-msi.ps1` emits `dist/windows/*.wxs` from `$wxsContent` here-string (`:336-409`); contract validator regenerates via `EmitOnly` (`validate-windows-msi-contract.ps1:52-74`).
**Apply to:** ALL MSI edits — touch `build-windows-msi.ps1` + `validate-windows-msi-contract.ps1`, NEVER the emitted `.wxs`.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| per-user `CurrentUser\Root` cert import + `certutil -addstore Root/TrustedPublisher` (D-04/D-05) | utility | file-I/O | No existing cert-store *import* code in `nono-cli` — only Authenticode *verification* (`exec_identity_windows.rs`) and Sigstore trust-root refresh (`setup.rs --refresh-trust-root`, which is a different trust system). The import is net-new; closest discipline analogs are `setup.rs` step structure + the Authenticode gate it must satisfy. |
| `NODE_EXTRA_CA_CERTS` propagation to confined Node engines (D-05) | utility | env-injection | No existing env-var injection of a CA bundle path; closest is the hook env-injection in `hook_runtime_windows.rs` and the engine/profile env plumbing, but the specific NODE_EXTRA_CA_CERTS wiring is new. |
| machine-policy presence probe (`HKLM\SOFTWARE\Policies\nono`) for `nono health` group (c) | utility | request-response | The HKLM policy key does not exist yet (Phase 83 builds the reader). Phase 82 only probes presence/readability — use the `reg.exe`-subprocess approach in `platform.rs` (not winreg, which is a Phase 83 dep addition). |

## Metadata

**Analog search scope:** `crates/nono-cli/src/` (cli, app_runtime, command_runtime, classify_runtime, setup, exec_strategy_windows/dacl_guard, exec_identity_windows, main), `scripts/` (build-windows-msi, validate-windows-msi-contract, verify-dark, gates/), `.cargo/config.toml`.
**Files scanned:** ~14 read in full or targeted; 4 gate files enumerated.
**Pattern extraction date:** 2026-06-18
