# Phase 93: Live ZT-Infra Integration + Revocation + Request Flow - Pattern Map

**Mapped:** 2026-06-22
**Files analyzed:** 12 (8 NEW / 4 MODIFY)
**Analogs found:** 12 / 12 (every file has an in-tree analog — this phase is composition, not greenfield)

> **Two repos.** nono-py is a SEPARATE repo at `C:\Users\OMack\nono-py` (NOT a subdir of Nono).
> Core/CLI/gates live in `C:\Users\OMack\Nono`. Each file below is tagged with its repo root.
> All analog file:line citations were verified by reading the live source this session.

---

## File Classification

| New/Modified File | Repo | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|------|-----------|----------------|---------------|
| `python/nono_py/_live.py` (NEW) | nono-py | service (live-arm orchestration) | request-response (HTTP, fail-closed) | nono-py `windows_confined_run.rs` seam + the D-01/D-02 Pattern-1 in RESEARCH | role-match (no existing Python HTTP module) |
| `python/nono_py/__init__.py` (MODIFY) | nono-py | provider (export surface + Python wrapper) | event-driven (module registration) | nono-py `python/nono_py/__init__.py:29-117` (existing exports + `win32` platform gate) | exact |
| `python/nono_py/_cli_apply.py` (NEW) | nono-py | controller (console-script entry) | request-response (verify-then-run) | `windows_confined_run.rs::confined_run` (:367-420) + `_live.py` | role-match |
| `src/override.rs` (MODIFY) | nono-py | model + utility (error enum + VK cache) | transform (digest→verify) | `OverrideErrorKind` enum (`override.rs:55-108`); `verify_ecdsa_digest` (`override.rs:621-634`) | exact (same file) |
| `src/override_trust.rs` (NEW) | nono-py | service (HKLM trust-root reader) | file-I/O (registry read, fail-secure) | `crates/nono/src/machine_policy.rs:265-536` `windows_reader` mod | exact (mirror) |
| `src/lib.rs` (MODIFY) | nono-py | provider (PyO3 registration) | event-driven | `nono-py/src/lib.rs:1-74` module/exception registration | exact |
| `src/windows_confined_run.rs` (MODIFY) | nono-py | controller (spawn seam) | request-response | `:395-407` VFY-01 seam comment (Phase 92 left the slot) | exact (same file) |
| `crates/nono-cli/src/exec_strategy/env_sanitization.rs` (MODIFY) | Nono | middleware (env filter) | transform (env strip) | `is_dangerous_env_var` (`env_sanitization.rs:16-77`) — `LD_`/`DYLD_` prefix clauses | exact (same fn) |
| `crates/nono-cli/src/cli.rs` (MODIFY) | Nono | config (clap defs) | request-response | `Profile(ProfileCmdArgs)` (`cli.rs:1058`) + `ProfileCommands` enum (`cli.rs:1429-1451`) | exact (nesting pattern) |
| `crates/nono-cli/src/override_request.rs` (NEW) | Nono | controller (`nono override request`) | request-response (denial bundle) | `crates/nono-cli/src/diagnostic/formatter.rs:33-46` `PolicyExplanation`/`DiagnosticFormatter` | role-match |
| `pyproject.toml` (MODIFY) | nono-py | config (console-script reg) | n/a | `nono-py/pyproject.toml` `[project]`/`[tool.maturin]` (NO `[project.scripts]` exists yet) | role-match |
| `scripts/gates/override-02.ps1` (NEW) | Nono | test (Dark Factory gate) | event-driven (verdict) | `scripts/gates/override-01.ps1` (full file — `Test-Precondition`/`Invoke-Gate` contract) | exact (mirror) |

---

## Pattern Assignments

### `python/nono_py/_live.py` (NEW — service, request-response) — nono-py

**Analog:** No existing Python HTTP module in nono-py. Build per RESEARCH Pattern-1 (D-01/D-02). The
**outcome→kind→EventID mapping** is the load-bearing contract; the kind strings MUST match the new
Rust `OverrideErrorKind` variants (`LiveRevoked`/`LiveUnavailable`) added to `override.rs`.

**Fail-closed mapping (verbatim from RESEARCH, the executor must replicate exactly):**

| Outcome | HTTP | kind | EventID | Run |
|---------|------|------|---------|-----|
| `decision: allow` | 200 | — | 10007 VERIFIED | proceeds |
| `decision: deny` | 403 | `LiveRevoked` | **10010 REVOKED** | blocked |
| timeout > 2s | — | `LiveUnavailable` | **10008 REJECTED** | blocked |
| conn refused / DNS fail | — | `LiveUnavailable` | 10008 | blocked |
| non-200/403 status | 4xx/5xx | `LiveUnavailable` | 10008 | blocked |
| malformed/empty body | 200 | `LiveUnavailable` | 10008 | blocked |
| 200 but `decision != allow` | 200 | `LiveRevoked` | 10010 | blocked |

**Request body shape (D-03 — consumes the already-verified `OverrideGrant`, never re-parses token):**
```python
body = json.dumps({
    "actor": grant.signer,                    # OverrideGrant.signer (override.rs:665, #[pyo3(get)])
    "action": f"override.apply:{grant.jti}",  # grant.jti (override.rs:678) — jti carries per-token revoke
    "resource": grant.repo_context or "",     # grant.repo_context (override.rs:681, Option<String>)
    "correlation_id": grant.jti,
    # NEVER set flush_daal — ZTL-05 async anchoring (omit entirely)
}).encode("utf-8")
```
**Critical (Pitfall 5):** build the opener with an empty `ProxyHandler({})` to disable env-proxy and
do not follow cross-host redirects — the `NONO_ZT_ACTIONS_URL` is the D-04 trust anchor.
Use `urllib.request.urlopen(req, timeout=2.0)` (the bounded-timeout primitive).
On allow, return `payload.get("audit", {}).get("current_hash")` (AUD-02 fresh hash, preferred over token's).

**`OverrideGrant` getters available** (verified `override.rs:660-716`, `#[pyclass(frozen)]`):
`signer`, `not_before`, `expires_at`, `jti`, `repo_context` (`#[pyo3(get)]`); `scope_paths()`,
`scope_domains()`, `zt_audit_hash()` (explicit `#[getter]` returning clones).

---

### `python/nono_py/__init__.py` (MODIFY — provider) — nono-py

**Analog:** existing export block + platform gate at `__init__.py:29-117`.

**Existing platform-gate pattern to extend (lines 69-73, 113-117):**
```python
# sandboxed_exec is Unix-only (fork+exec path); confined_run/confine are Windows-only.
if _sys.platform == "win32":
    from nono_py._nono_py import confined_run, confine
else:
    from nono_py._nono_py import sandboxed_exec
```
**To add:** export the Python live-arm wrapper(s) and `_live.live_check`. If the live check is a Python
pre-step wrapping the Rust `confined_run` (OQ-2 recommendation), the wrapper (`confined_run_checked` or a
re-exported shim) lands here, and is added to `__all__` + the `win32` `__all__` extension (line 114-115).
Follow the **alphabetical** ordering already used in the import block and `__all__`.

---

### `python/nono_py/_cli_apply.py` (NEW — controller, console-script) — nono-py

**Analog:** `windows_confined_run.rs::confined_run` (:367-420) is the confined-run shape this CLI mirrors;
reuses `_live.live_check` (D-01) for the offline+live verify.

**Flow (D-08, one-shot verify-then-run):** parse `<token-path> -- <command>` → read token → Rust
`verify_override()` (offline) → `_live.live_check(grant)` (live, fail-closed) → `confined_run(...,
override_token=grant, ...)`. Any `NonoOverrideError` propagates and blocks the run (no exec on failure).
Registered as a `[project.scripts]` console-entry (`nono-override-apply`, see pyproject below; OQ-5 — the
literal `nono override apply` 3-token dispatch is flagged for user confirmation, D-07 forbids nono.exe→nono-py shell-back).

---

### `src/override.rs` (MODIFY — model + utility) — nono-py

**Analog A — `OverrideErrorKind` enum** (`override.rs:55-82`). Add two variants + their `as_str()` arms:

```rust
// Existing enum (override.rs:56-82) — add LiveRevoked / LiveUnavailable:
pub enum OverrideErrorKind {
    BadSignature, Expired, NotYetValid, OutOfScope, Replay,
    AlgorithmMismatch, KeyNotAllowlisted, Parse, MissingField,
    // ADD (D-02):
    // LiveRevoked,      // live POST /actions returned deny → EventID 10010 (REVOKED)
    // LiveUnavailable,  // timeout/unreachable/non-200/malformed → EventID 10008 (REJECTED)
}
```
And mirror in `as_str()` (`override.rs:89-101`) — the stable string codes are consumed by the
nono-cli EventID map (Phase 92). Display impl (`:104-108`) needs no change (delegates to `as_str()`).

**Analog B — `verify_ecdsa_digest` pubkey seam** (`override.rs:621-634`, carries the `[BLOCKING-93]`
doc at `:609-616`). Today `pubkey_der` is a test-injected `&[u8]` param. D-06 closes this: source DER
from the HKLM reader (`override_trust.rs`) and cache `VerificationKey` per `key_id`.
```rust
pub(crate) fn verify_ecdsa_digest(pubkey_der: &[u8], digest: &[u8; 32], sig_b64: &str)
    -> Result<(), OverrideErrorKind> {
    let pub_key = DerPublicKey::from(pubkey_der.to_vec());
    let vk = VerificationKey::from_spki(&pub_key, SigningScheme::EcdsaP256Sha256)
        .map_err(|_| OverrideErrorKind::BadSignature)?;
    // ... verify_prehashed (existing, low-S already enforced upstream at :788)
}
```
**Cache pattern (RESEARCH Pattern-2):** `static VK_CACHE: LazyLock<Mutex<HashMap<String, Vec<u8>>>>`
keyed on `key_id` → DER bytes. Lookup order is HKLM policy ONLY (D-05); missing `key_id` →
`Err(OverrideErrorKind::KeyNotAllowlisted)` (fail-closed). Update the `[BLOCKING-93]` doc-comment to
reflect closure.

---

### `src/override_trust.rs` (NEW — service, file-I/O) — nono-py

**Analog:** `crates/nono/src/machine_policy.rs:265-536` `windows_reader` mod — the canonical
fail-secure HKLM `winreg` reader in this fork. **Mirror its taxonomy exactly.**

**Imports + constants to replicate** (`machine_policy.rs:269-273`):
```rust
#[cfg(target_os = "windows")]
mod windows_reader {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
    use winreg::{RegKey, RegValue};
    const ERROR_FILE_NOT_FOUND: i32 = 2; // key/sub-key absent
```

**Fail-secure read taxonomy to mirror** (`machine_policy.rs:277-315`):
- wrong REG type → `Err("expected REG_SZ, got ...")` (malformed → abort, NEVER fall through)
- sub-key absent (`ERROR_FILE_NOT_FOUND`) → `Ok(Vec::new())` / `Ok(None)` (the *parent* key existing gates enforcement)
- present-but-unreadable → `Err` (mirror `machine_policy.rs:477-482` — **never** fall through; Pitfall 6)
- always open with `KEY_READ | KEY_WOW64_64KEY` (64-bit view)
- use `read_list_subkey` shape for ADMX `<list>` N×REG_SZ sub-keys

**Value-name schema to define (OQ-4 — mirror the egress spine's `<list>` shape):**
`Override\AllowedKeyArns\` (N×REG_SZ list) + `Override\KmsPublicKeys\` (named values: value-name=`key_id`,
data=DER+base64 REG_SZ). Missing trust material → fail-closed deny (D-05).

**Cross-target stub (Pitfall 6 / CLAUDE.md cross-target MUST):** provide a non-Windows stub returning the
same fail-closed `Result` shape (mirror `machine_policy.rs:533-536` `Ok(None)`/`Err` stub). Mark cross-target
clippy PARTIAL→CI per `.planning/templates/cross-target-verify-checklist.md` if the cross-toolchain is absent.

> **Crate note (A1):** `winreg` is in-tree via core `nono` (`machine_policy.rs:269`). Confirm it is a
> direct dep of nono-py (`cargo tree -p nono-py | grep winreg`) and promote transitive→direct in
> `nono-py/Cargo.toml` if needed. Do NOT hand-roll raw `windows-sys` `RegGetValueW` (D-06's "own
> windows-sys call" wording is non-binding on the crate choice — RESEARCH Alternatives).

---

### `src/lib.rs` (MODIFY — provider, PyO3 registration) — nono-py

**Analog:** `nono-py/src/lib.rs:1-74` — module + exception registration. The existing `NonoOverrideError`
exception and `verify_override`/`OverrideGrant` are already registered (confirmed re-exported in
`__init__.py:44-66`). Add the `override_trust` reader `#[pyfunction]` registration here following the
same `m.add_function`/`m.add_class` pattern used for the existing override symbols.

---

### `src/windows_confined_run.rs` (MODIFY — controller, spawn seam) — nono-py

**Analog:** the seam already exists — Phase 92 left the explicit slot at `:404-406`:
```rust
// :396-407 (existing) — append override args then the VFY-01 seam:
if let Some(ref grant) = override_token {
    probe_override_support(&nono_path)?;
    append_override_args(&mut cmd, grant)?;
}
// VFY-01 PARTIAL [BLOCKING-93]: Phase 92 wires the offline verify arm.
// Phase 93 adds the live POST /actions AND-gate here before confined_run
// is called (D-03 composition seam; VFY-01 clause b).
```
**Per OQ-2 (recommended):** the live check is a **Python pre-step** (`_live.live_check`) performed by the
Python caller BEFORE the Rust `confined_run` is entered — a Rust `#[pyfunction]` cannot cleanly call
`urllib` mid-body. Update this seam comment to: "live check performed by the Python caller; this fn assumes
the grant is live-verified." The same seam exists in `confine` (Shape B, ~:513-515).

---

### `crates/nono-cli/src/exec_strategy/env_sanitization.rs` (MODIFY — middleware) — Nono

**Analog:** `is_dangerous_env_var` (`env_sanitization.rs:16-77`) — the single chokepoint feeding all
exec paths. Add one prefix clause next to the existing `LD_`/`DYLD_` prefix checks:
```rust
pub(crate) fn is_dangerous_env_var(key: &str) -> bool {
    // Linker injection
    key.starts_with("LD_")
        || key.starts_with("DYLD_")
        // ADD (ZTL-04): AWS credentials never reach the sandboxed child
        || key.starts_with("AWS_")
        // ... existing clauses
}
```
**Correctness notes:**
- `AWS_` strip is **cfg-UNCONDITIONAL** (correct — AWS creds are dangerous on every platform), UNLIKE
  the Windows-gated `PATH`/`SystemRoot` block (`:66-77`). Do not gate it.
- `key.starts_with("AWS_")` is a string-prefix on an env-var NAME, not a path — this matches the
  existing `LD_`/`DYLD_` precedent and is NOT the CLAUDE.md path-`starts_with` footgun.
- The Windows `SystemRoot`/`windir` baseline re-add for CLR children is a separate code path — untouched.
- **Test:** extend the existing `#[cfg(test)] mod tests` (AWS allow-pattern precedent at `:348-357`)
  with `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_SESSION_TOKEN` → dangerous (SC3).

---

### `crates/nono-cli/src/cli.rs` (MODIFY — config, clap defs) — Nono

**Analog:** `Profile(ProfileCmdArgs)` in the `Commands` enum (`cli.rs:644` enum start; `:1058` variant) +
the inner `ProfileCommands` subcommand enum (`cli.rs:1429-1451`):
```rust
// Commands enum (cli.rs:644) — ADD:  Override(OverrideArgs),
#[derive(Subcommand, Debug)]
pub enum OverrideCommands {           // mirror ProfileCommands (cli.rs:1429-1451)
    Request(OverrideRequestArgs),
    // Apply lives in nono-py per D-07 — NOT added to nono.exe.
    // Document this asymmetry to avoid a plan-checker/help-coverage flag.
}
```
The `Profile` variant carries a rich `#[command(help_template=..., after_help=...)]` block
(`cli.rs:1058`) — mirror that help-template style for the `Override` variant. The existing
`OverrideAuditMeta` (`cli.rs:1628`) + `--override-audit` field (`:2021`) are the Phase 92 audit
plumbing and are **not** changed by CLI-01.

---

### `crates/nono-cli/src/override_request.rs` (NEW — controller, CLI-01) — Nono

**Analog:** `crates/nono-cli/src/diagnostic/formatter.rs:33-46` `PolicyExplanation` + the
`DiagnosticFormatter` denial-rendering surface (CLI-01 needs only diagnostic context, no crypto — D-07).

**Reusable types** (verified `formatter.rs:21-46`):
```rust
pub struct PolicyExplanation {
    pub path: PathBuf,
    pub access: AccessMode,       // nono::AccessMode
    pub reason: String,           // "sensitive_path" | "insufficient_access" | "path_not_granted"
}
// Core policy-free denial records come from nono::diagnostic::{DenialRecord, DenialReason, ...}
```
**Flow (D-08):** gather denial context (scope paths/domains, `repo_context`, denial reason) via
`DiagnosticFormatter` → emit a structured JSON request bundle `{scope, repo_context, reason, nonce}`
(fresh nonce) + a human-readable summary. The bundle feeds the out-of-nono approver/KMS-signing pipeline.
**Test:** add a `#[cfg(test)] mod tests` asserting the bundle JSON shape (CLI-01).

---

### `pyproject.toml` (MODIFY — config, console-script) — nono-py

**Analog:** `nono-py/pyproject.toml` `[project]` block (`:5-31`). **There is NO `[project.scripts]`
section today** — this is a genuine new section.
```toml
# ADD a [project.scripts] table (new):
[project.scripts]
nono-override-apply = "nono_py._cli_apply:main"   # OQ-5 — console-entry for the apply affordance
```
**Build note:** `pip install -e .` (or `maturin develop`) MUST be re-run after adding `[project.scripts]`
for the console-script to appear in the venv `Scripts/`. The existing `[tool.maturin]` (`:42-45`,
`python-source = "python"`, `module-name = "nono_py._nono_py"`) is unchanged.

---

### `scripts/gates/override-02.ps1` (NEW — test, Dark Factory gate) — Nono

**Analog:** `scripts/gates/override-01.ps1` (full file). Mirror the contract EXACTLY:
- exports exactly two functions, dot-sourced by `scripts/verify-dark.ps1`
- `Test-Precondition` → `$null` (run) | `"reason"` (SKIP_HOST_UNAVAILABLE, exit 3)
- `Invoke-Gate` → `[ordered]@{ gate; verdict; reason; detail; timestamp }`,
  verdict ∈ `{ PASS | FAIL | SKIP_HOST_UNAVAILABLE }`
- **NEVER** call `exit`; **NEVER** call `Persist-Verdict`; a `throw` = harness-internal (exit 4)
- gate config vars at top: `$script:GateName`, `$script:FixturesPath`, `$script:TestKmsArn`
  (`override-01.ps1:51-53`)
- reuse the Phase 91 test keypair: `C:\Users\OMack\nono-py\tests\fixtures\override_test_key.{pem,der}`

**Precondition delta to ADD after the override-01 checks** (RESEARCH Code Example):
```powershell
# 6. Probe the local provisioner (server.js: 127.0.0.1:3000).
$ztUrl = $env:NONO_ZT_ACTIONS_URL
if (-not $ztUrl) { return 'NONO_ZT_ACTIONS_URL not set — start provisioner (cd provisioner; npm start) ...' }
try {
    $health = Invoke-WebRequest -Uri ($ztUrl -replace '/actions$','/health') -TimeoutSec 2 -UseBasicParsing
    if ($health.StatusCode -ne 200) { return "provisioner unhealthy ($($health.StatusCode)) — SKIP_HOST_UNAVAILABLE" }
} catch { return "provisioner unreachable at $ztUrl — SKIP_HOST_UNAVAILABLE" }
```
**Invoke-Gate body:** mint a token (override-01 `make_token` pattern), seed an allow rule `override.apply*`,
verify allow→grant; seed a deny rule `override.apply:<jti>`, verify deny→`NonoOverrideError(LiveRevoked)`
(ZTL-03 revocation proof). **Invocation:** `pwsh -File scripts\verify-dark.ps1 --gate override-02` —
NEVER `pwsh -Command "<bare path>"` (swallows exit N→1; MEMORY durable).

---

## Shared Patterns

### Fail-secure (every new failure path denies)
**Source:** CLAUDE.md § Coding Standards + Security Considerations.
**Apply to:** all NEW files. `Result` + `?`; no `.unwrap()`/`.expect()` (clippy-enforced); `#[must_use]`
on critical Results; libraries never panic on expected errors. Every live-arm outcome maps to allow-or-deny.

### kind → EventID 1:1 (no string-parsing of messages)
**Source:** `nono-py/src/override.rs:84-101` (`OverrideErrorKind::as_str()`) →
`crates/nono-cli/src/telemetry/event.rs:48-56` (EventID constants 10006-10010) and the
`SecurityEventType → event_id` map (`event.rs:99-109`).
**Apply to:** `_live.py`, `override.rs`. The new `LiveRevoked`→10010 / `LiveUnavailable`→10008 mapping
rides this existing 1:1 contract. The reject-emission code path is OQ-1 (planner decision — see below).
```rust
// event.rs:52-56 (existing constants the new kinds map to):
pub const EVENT_ID_POLICY_OVERRIDE_REJECTED: u32 = 10008;
pub const EVENT_ID_POLICY_OVERRIDE_REVOKED: u32  = 10010;
```

### HMAC chain integrity (audit emission)
**Source:** `crates/nono-cli/src/telemetry/mod.rs:321-344` `emit_override_event` + `SECURITY_LAYER`
OnceLock (`mod.rs:48-49`). Signature:
```rust
pub fn emit_override_event(&self, event_type: &SecurityEventType, jti: &str,
    kms_key_id: &str, zt_audit_hash: Option<&str>) -> Result<String, &'static str>
```
**Apply to:** the chosen reject-emission path (OQ-1). `#[must_use]` — `Err` is FATAL (AUD-04 fail-closed,
abort before spawn). **NOTE (Pitfall 3 / OQ-1):** on a live *deny*, nono-py fails closed BEFORE spawning
`nono.exe`, so the existing `--override-audit`-at-spawn path (`execution_runtime.rs:259-288`) does NOT
emit on the reject branch. The planner MUST pick: (a) NEW thin `nono.exe override audit-emit <meta>
--kind rejected|revoked` subcommand nono-py invokes on reject; or (b) reject events surfaced only via
the raised `NonoOverrideError` + provisioner's own audit chain. This is an explicit SC.

### Path/DNS security (component comparison)
**Source:** CLAUDE.md footguns; `windows_confined_run.rs:287-288` `sanitize_override_path`
(`Path::components()`/`Component::ParentDir`).
**Apply to:** `_cli_apply.py` scope handling, `override_request.rs`. Never string-`starts_with` on paths.
(The `AWS_` env-name prefix in `env_sanitization.rs` is exempt — it is a var NAME, not a path.)

### Dark Factory verdict contract
**Source:** `scripts/gates/override-01.ps1` (contract docblock `:5-45`).
**Apply to:** `override-02.ps1`. Return a verdict object; never `exit`/`Persist-Verdict`; host-gated
paths `SKIP_HOST_UNAVAILABLE` (exit 3, never FAIL); invoke via `-File`.

---

## No Analog Found

None. Every Phase 93 file has an in-tree analog to mirror or an existing function to call. The two NEW
files with the weakest direct analog still have a strong pattern source:

| File | Role | Data Flow | Closest Pattern (not a 1:1 file analog) |
|------|------|-----------|-----------------------------------------|
| `python/nono_py/_live.py` | service | request-response | RESEARCH Pattern-1 (D-01/D-02) + provisioner `server.js` contract; no pre-existing Python HTTP module in nono-py |
| `python/nono_py/_cli_apply.py` | controller | request-response | composition of `confined_run` (Rust) + `_live.live_check`; no existing console-script in nono-py |

---

## Metadata

**Analog search scope:**
- nono-py repo: `C:\Users\OMack\nono-py\src\{override.rs, windows_confined_run.rs, lib.rs}`,
  `python/nono_py/__init__.py`, `pyproject.toml`, `tests/fixtures/`
- Nono workspace: `crates/nono/src/machine_policy.rs`,
  `crates/nono-cli/src/{exec_strategy/env_sanitization.rs, cli.rs, diagnostic/formatter.rs,
  telemetry/{mod.rs,event.rs}, execution_runtime.rs}`, `scripts/gates/override-01.ps1`

**Files read this session (verified file:line):** override.rs (49-198, 600-720), machine_policy.rs
(260-379), env_sanitization.rs (1-90), cli.rs (640-651, 1050-1078, 1428-1457), __init__.py (1-120),
pyproject.toml (full), telemetry/mod.rs (1-50, 300-344), telemetry/event.rs (grep), formatter.rs (1-50),
windows_confined_run.rs (395-424), override-01.ps1 (1-130).

**Pattern extraction date:** 2026-06-22

**Open questions carried to planner (do not resolve here):**
- OQ-1: reject (10008/10010) emission path into the nono-cli HMAC chain (Pitfall 3) — planner picks (a) or (b).
- OQ-2: live check as Python pre-step around the Rust `confined_run` (recommended) — confirm.
- OQ-4: `Override\` HKLM value-name schema — planner defines + ADMX note.
- OQ-5 / A4: `nono-override-apply` console-script vs literal `nono override apply` dispatch — confirm with user (D-07 forbids nono.exe→nono-py shell-back).
- A1: confirm `winreg` is a (promotable) nono-py dep via `cargo tree`.
