# Phase 93: Live ZT-Infra Integration + Revocation + Request Flow - Research

**Researched:** 2026-06-22
**Domain:** Live HTTP AND-gate orchestration (Python `urllib`), Windows HKLM trust-root reads (nono-py), AWS-cred env stripping, CLI command split, Dark Factory scripted gates — over the Phase 91/92 signed-override surface
**Confidence:** HIGH (all load-bearing claims grounded in read source at file:line; live AWS/KMS control plane unreachable from this host — those claims are scoped to the local provisioner + what is statically verifiable)

---

## Summary

Phase 93 closes the two-key AND gate by bolting a **live `POST /actions`** check onto the Phase 91 offline verifier. The architecture is settled by locked decisions D-01..D-08: the Rust `verify_override()` stays offline-only and policy-free; a **new Python orchestration layer** (in the `nono_py` package, not Rust) performs the live `urllib.request` call, fails closed on deny/timeout/unreachable/malformed, and only on a live `allow` proceeds to `confined_run`/`confine`. The KMS pubkey + ARN allowlist move from test-injection to a **fail-secure `HKLM\SOFTWARE\Policies\nono` read** (D-05/D-06), mirroring the in-tree `crates/nono/src/machine_policy.rs` reader pattern. `AWS_*` is stripped at the single `is_dangerous_env_var` chokepoint (ZTL-04). Two CLI affordances split by capability: `nono override request` (Rust, nono.exe) and `nono override apply` (Python console-script, nono-py).

**Three findings contradict or sharpen the locked assumptions and must surface to the planner:**

1. **D-03 mapping is half-right.** The provisioner's `evaluateAction` keys **only on `action`** — `actor` is required-non-empty but is **never matched** (`policy.js:31-59`); `resource` and `correlation_id` are **audit-only** (`server.js:37-45`). Per-token revocation via `action = "override.apply:<jti>"` works exactly as D-03 intends, but the planner must NOT build any actor-based matching expectation.

2. **D-06 token reconciliation: a provisioner AuditRecord is NOT an override token.** The nono `OverrideToken` (`override.rs:329-366`) extends the CAF AuditRecord base with **signed override-specific fields** (`scope`, `not_before`, `expires_at`, `jti`, `repo_context`) under `deny_unknown_fields`. The provisioner's `record()` (`audit.js:236-257`) emits ONLY base AuditRecord fields — no scope/jti. So the override token is a **nono-side construct** (as D-06 anticipated), the provisioner does **not** issue it, and the OVERRIDE-02 gate must **mint its own token with the Phase 91 test keypair** (exactly as override-01 already does, `override-01.ps1:229-254`). Real-KMS reconciliation reduces to: confirm a real KMS `Sign(MessageType=DIGEST)` over the canonical 32-byte digest produces a low-S DER signature that `verify_ecdsa_digest` accepts — which `CANONICAL_FORM.md:154-162` and the provisioner's own `signHash` (`audit.js:184-197`) confirm is the exact same primitive.

3. **Live-deny/unavailable audit events have no nono-cli emission path on the reject branch.** The nono-cli `SecurityEventLayer` HMAC chain is only reached when nono-py spawns `nono.exe run ... --override-audit` (`execution_runtime.rs:259-288`). On a live **deny**, nono-py fails closed **before** spawning nono.exe — so EventID 10010 (REVOKED) / 10008 (REJECTED) cannot land in the nono-cli chain by the existing path. This is an architectural tension the planner must resolve (options in Open Questions OQ-1).

**Primary recommendation:** Build the live arm as a pure-Python module (`nono_py/_live.py` or similar) invoked from the Python wrappers around `confined_run`/`confine` (D-07's `apply` path reuses it). Source trust roots via a new `#[cfg(windows)]` Rust reader in nono-py that mirrors `machine_policy.rs` using the **`winreg` crate** (the in-tree pattern — not raw `windows-sys`), exposed as a PyO3 function. Strip `AWS_*` by adding one `key.starts_with("AWS_")` clause to `is_dangerous_env_var`. Mirror `override-01.ps1` for `override-02.ps1`, adding the live provisioner precondition + `SKIP_HOST_UNAVAILABLE`.

---

<user_constraints>
## User Constraints (from 93-CONTEXT.md)

### Locked Decisions (D-01..D-08 — research HOW to implement, do not re-litigate)

- **D-01 (Python orchestration layer):** Rust `verify_override()` stays offline-only and policy-free. A new **Python-side orchestration layer** does: Rust offline verify → `OverrideGrant`, then a **Python `urllib.request` `POST /actions`** live call, and only on live `allow` do `confined_run`/`confine` proceed. The live arm is independently **mockable** and reused by CLI `apply` (D-07) + the OVERRIDE-02 gate. Do **not** push HTTP into Rust (no `ureq`/`reqwest`). Offline-pass is necessary but not sufficient.
- **D-02 (Distinct fail-closed kinds):** live **`deny`** → kind **`LiveRevoked`** → **EventID 10010 (REVOKED)**; **timeout (>2s) / unreachable / non-200 / malformed** → kind **`LiveUnavailable`** → **EventID 10008 (REJECTED)**. **Both block the run** (fail-closed). Extends the Phase 91 D-04 kind→EventID map.
- **D-03 (OverrideGrant → {actor, action, resource}):** `actor` = signer ARN (`OverrideGrant.signer`/`kms_key_id`), `action` = **`"override.apply:<jti>"`**, `resource` = `repo_context`, `correlation_id` = `jti`. Provisioner `evaluateAction` keys on `actor`+`action` (exact / prefix-`*`; `resource` audit-only). Per-token revocation = operator adds deny rule on `action "override.apply:<jti>"`; normal overrides permitted by allow rule `"override.apply*"`; provisioner `defaultDecision` is `deny`. The live check is the **sole revocation enforcement point** — no new infra in nono.
- **D-04 (Network-level trust; no app-level auth):** nono sends the JSON body only; trust is network-level (`NONO_ZT_ACTIONS_URL` is the trust anchor). Provide an **optional** env header passthrough (working name `NONO_ZT_ACTIONS_HEADER`) — unused/empty by default. Do not invent a mandatory app-level auth.
- **D-05 (Policy-authoritative trust model):** `HKLM\SOFTWARE\Policies\nono` is **authoritative for trust roots** (KMS pubkey DER+base64, ARN allowlist). Env supplies **only non-trust ops config** (`NONO_ZT_ACTIONS_URL`, timeout, optional header). **Missing trust material in policy = fail-closed deny.** Env **cannot override or widen** trust roots.
- **D-06 (nono-py reads HKLM directly; caches VerificationKey per `key_id`):** nono-py does its **own registry read** of the policy spine for override trust config (env fallback only for ops config). `VerificationKey` built from policy pubkey DER, **cached per `key_id`** (closes the `[BLOCKING-93]` pubkey seam in `verify_ecdsa_digest`/`verify_override_impl`, which today take `pubkey_der` test-injected).
- **D-07 (Split by capability):** `nono override request` is **nono.exe-native (Rust)** — needs only `DiagnosticFormatter` context, no crypto/live. `nono override apply` is **nono-py-delivered** (console entry reusing the Python orchestration layer). No duplicated verifier, no nono.exe→nono-py shell-back.
- **D-08 (`request` = JSON bundle; `apply` = one-shot verify-then-run):** `request` emits a **structured JSON request bundle** (scope paths/domains, `repo_context`, denial reason, fresh nonce) + human-readable summary. `apply <token-path> -- <command>` runs **full fail-closed verification (offline + live)**, then executes confined in one shot (CLI mirror of `confined_run`).

### Claude's Discretion (researcher/planner decide with fail-secure defaults)

- **ZTL-04 (`AWS_*` strip locus):** extend `crates/nono-cli/src/exec_strategy/env_sanitization.rs` to drop **all** `AWS_*`; SC3 env-inspection test. Respect the Windows `SystemRoot`/`windir` baseline re-add for CLR children.
- **ZTL-05 (DAAL async):** nono **never sets `flush_daal:true`** on the hot path; DAAL anchoring (response `audit.daal`) is the provisioner's async concern. Anchoring must not block/fail the spawn path.
- **DF-02 (OVERRIDE-02 gate):** against the **local provisioner**; `SKIP_HOST_UNAVAILABLE` (exit 3) when provisioner/AWS absent. `Test-Precondition`/`Invoke-Gate` contract; never `exit`/`Persist-Verdict`; invoked via `-File`/direct.
- **Bi-directional hash refinement (AUD-02):** lean — prefer the **live `/actions` response's fresh `audit.current_hash`** on the live-verified path; fall back to the token hash offline.
- Exact Python orchestration entry name/signature; `urllib` request/timeout impl; request-bundle JSON schema; console-script registration; `nono override` subcommand grouping in the `Commands` enum.
- **Phase 91 D-06 token wire-shape reconciliation** (a research item; OVERRIDE-02 mints via local provisioner + Phase 91 test keypair).

### Deferred Ideas (OUT OF SCOPE)

- Non-Windows override wiring (`sandboxed_exec` / Landlock / Seatbelt parity) — `confined_run`/`confine` stay Windows-only.
- `nono-ts` binding parity (FUT-03), M-of-N / threshold approval (FUT-01), push/webhook revocation (FUT-02).
- Mandatory app-level endpoint auth (bearer/mTLS in nono) — `NONO_ZT_ACTIONS_HEADER` passthrough is the forward hook only.
- Crate publish / `v*.*.*` release — milestone-marker only; future release leapfrogs to ≥ `0.65.0`.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ZTL-01 | Live decision endpoint configurable (`NONO_ZT_ACTIONS_URL`); integrates ZT-Infra v2 control plane | `POST /actions` contract grounded (`server.js:25-70`); endpoint is the D-04 trust anchor; env-only ops config (D-05). Local provisioner `127.0.0.1:3000` (`server.js:7-8`). |
| ZTL-02 | Live `POST /actions` bounded 2s timeout; fail-closed on timeout/error/`deny` | `urllib.request.urlopen(timeout=2.0)` (Python; D-01). Fail-closed mapping table below. `LiveRevoked`/`LiveUnavailable` kinds (D-02). |
| ZTL-03 | Revocation honored — id on deny-list rejected next live check; no new revocation infra | Provisioner `deny[]` is the enforcement point (`policy.js:42-45`); D-03 `action="override.apply:<jti>"` maps a per-token deny rule. `defaultDecision:"deny"` (`policy.js:3-12`). |
| ZTL-04 | `AWS_*` stripped from sandboxed child env (extend exec-strategy filter) | Single chokepoint `is_dangerous_env_var` (`env_sanitization.rs:16-77`); feeds all exec paths (Unix `exec_strategy.rs:554/769`, Windows `launch.rs:684`, hooks). Add `key.starts_with("AWS_")`. |
| ZTL-05 | Override authorizations anchored to DAAL **async/non-blocking** | nono never sets `flush_daal:true` (default `[]`, `server.js:46-49`); `auditor.daal.drain()` only runs if requested. `attestAction` returns immediately when DAAL disabled (`daal.js:50-60`); enqueue is fire-and-forget (`daal.js:304-317`). |
| CLI-01 | `nono override request` surfaces denial context (paths/domains, repo) from `DiagnosticFormatter` | Rust-native command (D-07); reuses `diagnostic/formatter.rs` `PolicyExplanation`/`DiagnosticFormatter` (`formatter.rs:33-46`). Lands in `Commands` enum (`cli.rs:644`). |
| CLI-02 | `nono override apply` runs full fail-closed verify before expansion | Python console-script (D-07/D-08) reusing the D-01 orchestration layer; mirrors `confined_run` (`windows_confined_run.rs:367-420`). |
| DF-02 | Live AWS/KMS + DAAL paths exercised by scripted gates emitting `SKIP_HOST_UNAVAILABLE` when absent | Mirror `override-01.ps1`; add local-provisioner precondition probe + token-mint-and-live-verify. `verify-dark.ps1` exit 3 contract. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Offline ECDSA verify | nono-py Rust (`override.rs`) | — | Already built (Phase 91); stays policy-free, offline-only (D-01) |
| Live `POST /actions` HTTP call | **nono-py Python** (`_live.py`) | — | D-01 explicit: `urllib`, mockable, no Rust net dep; reused by CLI `apply` + gate |
| Trust-root sourcing (pubkey DER + ARN allowlist) | nono-py Rust (`#[cfg(windows)]` HKLM reader) | env (ops config only) | D-05/D-06: policy authoritative; nono-py reads HKLM itself (no nono.exe round-trip) |
| `VerificationKey` construction + per-`key_id` cache | nono-py Rust | — | Closes the test-injected `pubkey_der` seam in `verify_ecdsa_digest` (D-06) |
| `AWS_*` env strip | **nono-cli** (`env_sanitization.rs`) | — | Child env is built by nono.exe; the single `is_dangerous_env_var` chokepoint covers all exec paths |
| Audit emission (HMAC chain) | nono-cli (`SecurityEventLayer`) | — | Chain lives only in nono-cli (Phase 92 D-01); reached via `--override-audit` at spawn time |
| `nono override request` (denial bundle) | nono-cli Rust | `DiagnosticFormatter` | D-07: needs only diagnostic context, no crypto |
| `nono override apply` (verify-then-run) | nono-py Python console-script | D-01 orchestration layer | D-07/D-08: capability already lives in nono-py |
| DAAL anchoring | **ZT-Infra provisioner** (async) | — | ZTL-05: nono never waits; `audit.daal` is informational |
| Revocation decision | **ZT-Infra provisioner** (`deny[]`) | — | ZTL-03: live check is the sole enforcement point; no nono-side infra |

## Standard Stack

### Core (no new crates required)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Python `urllib.request` | stdlib (py3.10+) | Live `POST /actions` call + 2s timeout | D-01 mandates stdlib; no new dep; `urlopen(timeout=)` is the bounded-timeout primitive `[CITED: docs.python.org/3/library/urllib.request]` |
| `sigstore-verify` | 0.8 (already a direct dep) | `VerificationKey::verify_prehashed` | Already in use (`override.rs:632`); no change `[VERIFIED: nono-py/Cargo.toml:26]` |
| `winreg` | 0.x (transitive via `nono` core) | HKLM trust-root read in nono-py | In-tree pattern (`machine_policy.rs:269`); see Don't Hand-Roll. **Verify it is a direct dep of nono-py or promote it.** `[ASSUMED]` — needs `cargo tree` confirm at plan time |
| `base64`, `serde_json`, `chrono` | already direct deps | token meta, pubkey decode, timestamps | `[VERIFIED: nono-py/Cargo.toml:22-25]` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `clap` v4 | already in nono-cli | `nono override request` subcommand | CLI-01 only; nested `OverrideArgs` + `OverrideCommands` enum (mirror `ProfileCmdArgs`, `cli.rs:1058`) |
| `pytest` >= 8 | nono-py dev dep | live-arm + AWS-strip + apply tests | `[VERIFIED: nono-py/pyproject.toml]` (`requires-python = ">=3.10"`, pytest>=8) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `winreg` crate (in nono-py) | raw `windows-sys` `RegGetValueW`/`RegOpenKeyExW` | D-06 says "own `windows-sys` call", but the **in-tree pattern uses `winreg`** (`machine_policy.rs:269-270`). `winreg` is higher-level, matches the existing fail-secure reader, and avoids hand-rolling UTF-16/WOW64 handling. **Recommend `winreg`**; flag the D-06 wording as non-binding on the crate choice (it specifies "nono-py reads HKLM itself", not the exact API). |
| Python `urllib` | Python `http.client` | `urllib.request.urlopen(timeout=)` is the documented bounded-timeout one-liner; `http.client` is lower-level with no upside here |
| New Python `_live.py` module | PyO3 Rust `#[cfg(windows)]` HTTP | D-01 explicitly forbids Rust HTTP. The live arm MUST be Python. |

**Installation:** No new external packages. (Possible `winreg` promotion from transitive→direct in `nono-py/Cargo.toml` — confirm via `cargo tree -p nono-py | grep winreg` at plan time.)

**Version verification (run at plan time):**
```bash
cd C:/Users/OMack/nono-py && cargo tree -p nono-py 2>/dev/null | grep -i winreg   # confirm winreg availability
python --version   # confirm >=3.10 for urllib timeout semantics
```

## Package Legitimacy Audit

> No new external packages are introduced. All dependencies are already direct deps of `nono-py` (`Cargo.toml:15-27`) or Python stdlib. `winreg` is an established Rust crate already used by the `nono` core crate (`machine_policy.rs:269`).

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `sigstore-verify` | crates.io | established | — | sigstore/sigstore-rs | N/A (already in tree) | Approved (existing dep) |
| `winreg` | crates.io | 8+ yrs | high | github.com/gentoo90/winreg-rs | N/A (already used by core nono) | Approved (existing transitive; possible promotion) |
| `urllib.request` | Python stdlib | — | — | python/cpython | N/A | Approved (stdlib) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was not run — no new packages to vet. All deps are pre-existing in-tree or stdlib.*

## Architecture Patterns

### System Architecture Diagram

```
                          nono override apply <token-path> -- <cmd>
                          (Python console-script, nono-py — D-07/D-08)
                                         │
   confined_run(override_token=grant)    │
   (Python wrapper around the Rust       │
    #[pyfunction], see OQ-2)             ▼
            │                  ┌──────────────────────────┐
            │                  │  Python orchestration     │   D-01
            │   ①  read HKLM   │  layer  (_live.py)        │
            ├──────────────────┤                           │
            │   trust roots    │  1. read trust config     │
            ▼   (D-06 reader)  │     (pubkey DER + ARNs)   │──② verify_override()
   ┌────────────────────┐      │     from HKLM (D-05/06)   │     [Rust, offline,
   │ nono-py Rust HKLM  │◀─────┤  2. offline verify ───────┼──▶  policy-free]
   │ reader (winreg,    │      │     → OverrideGrant        │     → OverrideGrant
   │ #[cfg(windows)])   │      │  3. POST /actions ────────┼──③ urllib, 2s timeout
   └────────────────────┘      │     {actor,action,        │        │
        fail-secure            │      resource,            │        ▼
        (D-07 taxonomy)        │      correlation_id}      │   ZT-Infra POST /actions
                               │  4. decision==allow? ─────┤   (NONO_ZT_ACTIONS_URL;
                               │     allow → proceed       │    127.0.0.1:3000 local
                               │     deny  → LiveRevoked   │    or Tailscale/SSM AWS)
                               │     timeout/err →         │        │
                               │            LiveUnavailable│        ▼ evaluateAction
                               └───────────┬───────────────┘   keys on action ONLY
                                           │ allow             (policy.js — D-03)
                                           ▼                   deny[] = revocation
                          spawn  nono.exe run --allow <scope>      (ZTL-03)
                                 --override-audit <b64-meta>       │ resp.audit.
                                           │                       │ current_hash
                                           ▼                       │ (AUD-02 refine)
                          ┌────────────────────────────────┐       │
                          │ nono-cli execution_runtime      │◀──────┘
                          │  • AUD-04 pre-spawn gate        │
                          │    emit PolicyOverrideVerified  │  EventID 10007
                          │    → SecurityEventLayer HMAC    │
                          │  • build_child_env:             │
                          │    is_dangerous_env_var strips  │  ZTL-04
                          │    AWS_* (+ existing dangerous)  │
                          │  • apply_pre_fork_sandbox        │  AppContainer+WFP
                          └────────────────────────────────┘   (sandbox NEVER bypassed)
                                           │
                                           ▼
                                   confined child process
                                   (no AWS_* in env; scope = base + grant)

   nono override request  (nono.exe Rust, D-07/CLI-01)
        │  reads DiagnosticFormatter denial context (paths/domains/repo)
        ▼  emits JSON request-bundle {scope, repo_context, reason, nonce} + summary
   → operator/approver/KMS-signing pipeline  (out of nono; produces a signed token)
```

### Recommended Project Structure

```
nono-py/
├── src/
│   ├── override.rs            # Phase 91/92: ADD per-key_id VerificationKey cache;
│   │                          #   change verify_override_impl to accept pubkey via
│   │                          #   trust-config lookup, not test param (D-06 seam close)
│   ├── override_trust.rs      # NEW #[cfg(windows)] HKLM reader (winreg) →
│   │                          #   {pubkey_der_by_key_id, allowed_arns}; mirrors
│   │                          #   machine_policy.rs fail-secure taxonomy (D-05/06)
│   ├── windows_confined_run.rs# Phase 92 seam (:404-406, :513-515) consumed by Python
│   └── lib.rs                 # register override_trust reader fn + new exceptions
├── python/nono_py/
│   ├── __init__.py            # export the live-arm wrappers + apply entry
│   ├── _live.py              # NEW: POST /actions, 2s timeout, fail-closed mapping (D-01)
│   └── _cli_apply.py         # NEW: `nono override apply` console-script (D-07/D-08)
├── tests/
│   ├── test_live_arm.py       # NEW: mock urllib; deny→LiveRevoked, timeout→LiveUnavailable
│   ├── test_override_apply.py # NEW: apply path end-to-end with mocked live
│   └── fixtures/override_test_key.{pem,der}  # reuse (token minting)
└── pyproject.toml            # ADD [project.scripts] nono-override-apply = "..."

Nono/
├── crates/nono-cli/src/
│   ├── cli.rs                 # ADD Override(OverrideArgs) to Commands; OverrideCommands enum
│   ├── exec_strategy/env_sanitization.rs  # ADD AWS_* to is_dangerous_env_var (ZTL-04)
│   └── override_request.rs    # NEW: `nono override request` runtime (DiagnosticFormatter)
└── scripts/gates/override-02.ps1  # NEW: mirror override-01.ps1 + live precondition
```

### Pattern 1: Fail-closed live-arm decision mapping (D-02)

**What:** Map every live-call outcome to allow-or-deny; never fail open.
**When to use:** The core of `_live.py`.

```python
# Source: D-01/D-02 + provisioner contract (server.js:51, status 200 allow / 403 deny)
import json, urllib.request, urllib.error

def live_check(actions_url, grant, *, timeout=2.0, extra_header=None):
    """Returns None on allow; raises NonoOverrideError(kind) on any deny/failure (fail-closed)."""
    body = json.dumps({
        "actor": grant.signer,                          # D-03: signer ARN
        "action": f"override.apply:{grant.jti}",        # D-03: jti in action for per-token revoke
        "resource": grant.repo_context or "",           # D-03: audit-only on provisioner side
        "correlation_id": grant.jti,                    # D-03
        # NEVER set flush_daal — ZTL-05 async anchoring (omit entirely; provisioner default [])
    }).encode("utf-8")
    req = urllib.request.Request(actions_url, data=body,
                                 headers={"Content-Type": "application/json"}, method="POST")
    if extra_header:                                    # D-04 optional passthrough (empty by default)
        k, _, v = extra_header.partition(":")
        if k.strip():
            req.add_header(k.strip(), v.strip())
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            payload = json.loads(resp.read())           # 200 → allow branch
    except urllib.error.HTTPError as e:
        if e.code == 403:                               # provisioner deny → REVOKED
            raise _override_error("LiveRevoked", "live ZT-Infra check returned deny")
        raise _override_error("LiveUnavailable", f"live check HTTP {e.code}")  # non-200/403
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, OSError) as e:
        # timeout / unreachable / malformed → UNAVAILABLE (fail-closed)
        raise _override_error("LiveUnavailable", f"live check failed: {type(e).__name__}")
    if payload.get("decision") != "allow":              # belt-and-suspenders (200 but not allow)
        raise _override_error("LiveRevoked", "live decision not allow")
    return payload.get("audit", {}).get("current_hash")  # AUD-02 fresh hash (prefer over token)
```

**Live-call outcome → kind → EventID table (D-02):**

| Outcome | HTTP | kind | EventID | Run |
|---------|------|------|---------|-----|
| `decision: allow` | 200 | — | 10007 VERIFIED (at apply) | proceeds |
| `decision: deny` | 403 | `LiveRevoked` | **10010 REVOKED** | **blocked** |
| timeout > 2s | — | `LiveUnavailable` | **10008 REJECTED** | **blocked** |
| connection refused / DNS fail | — | `LiveUnavailable` | 10008 | blocked |
| non-200/403 status | 4xx/5xx | `LiveUnavailable` | 10008 | blocked |
| malformed/empty body | 200 | `LiveUnavailable` | 10008 | blocked |
| 200 but `decision != allow` | 200 | `LiveRevoked` | 10010 | blocked |

### Pattern 2: Per-`key_id` VerificationKey cache (D-06)

**What:** Build `VerificationKey` from the policy-sourced DER once per `key_id`, cache it.
**When to use:** Closes the `verify_ecdsa_digest` test-injected `pubkey_der` seam (`override.rs:609-634`).

```rust
// Source: D-06 + override.rs:621-634 (existing verify_ecdsa_digest)
use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;
static VK_CACHE: LazyLock<Mutex<HashMap<String, Vec<u8>>>> =  // key_id -> DER (cache the bytes;
    LazyLock::new(|| Mutex::new(HashMap::new()));             //  VerificationKey is not Clone-cheap)
// Lookup order (D-05): HKLM policy ONLY for trust roots. Env supplies NONE of these.
// Missing key_id in policy → Err(KeyNotAllowlisted) — fail-closed deny.
```

### Pattern 3: nested `nono override` clap subcommand (CLI-01)

**What:** Add `Override(OverrideArgs)` to the `Commands` enum, with an inner `OverrideCommands` enum.
**When to use:** Mirrors `Profile(ProfileCmdArgs)` (`cli.rs:1058`, inner enum `cli.rs:1432-1450`).

```rust
// Source: cli.rs:644 (Commands enum), cli.rs:1432-1450 (ProfileCommands nesting pattern)
// In Commands: Override(OverrideArgs),
// pub enum OverrideCommands { Request(OverrideRequestArgs), /* Apply lives in nono-py per D-07 */ }
// NOTE: `apply` is NOT added to nono.exe — D-07 keeps it in nono-py. The nono.exe
// `override` group carries ONLY `request`. (Document this asymmetry to avoid a plan-checker flag.)
```

### Anti-Patterns to Avoid

- **Pushing HTTP into Rust** (`ureq`/`reqwest`) — violates D-01. The live arm is Python.
- **Re-parsing the token between offline and live** — violates the TOCTOU closure (Phase 91 D-02). The live arm consumes the already-verified `OverrideGrant` (`override.rs:640-689`), reading `signer`/`jti`/`repo_context` from it.
- **Env-sourced trust roots** — violates D-05. Env supplies ops config (URL/timeout/header) only; pubkey + ARNs come from HKLM.
- **`flush_daal:true` on the hot path** — violates ZTL-05; would block on ledger finality (`daal.js:336-344` `drain()` awaits in-flight).
- **String `starts_with` on scope paths** — CLAUDE.md footgun; `sanitize_override_path` already uses `Path::components()`/`Component::ParentDir` (`windows_confined_run.rs:287-288`). Note: `is_dangerous_env_var("AWS_*")` via `key.starts_with("AWS_")` is a **string-prefix on an env-var NAME, not a path** — this is correct and matches the existing `LD_`/`DYLD_` prefix checks (`env_sanitization.rs:18-19`).
- **Failing open when HKLM is unreadable** — must mirror `machine_policy.rs:477-482` (present-but-unreadable → `Err`, never fall through).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HKLM fail-secure read | raw `RegGetValueW` + UTF-16 + WOW64 juggling | `winreg` crate, mirroring `machine_policy.rs:265-499` | Existing pattern handles `ERROR_FILE_NOT_FOUND`→absent, `ACCESS_DENIED`→abort, wrong-REG-type→malformed, `KEY_WOW64_64KEY` view — all the D-07 taxonomy edge cases |
| ECDSA verify over digest | new crypto path | existing `verify_ecdsa_digest` (`override.rs:621`) | Already low-S-enforced (`override.rs:788`), `verify_prehashed`, aws-lc-rs-backed |
| CAF canonicalization | re-implement | existing `canonical_sha256` (`override.rs:775`) proven against 9 test vectors | Cross-language byte-exactness is the whole point of CAF |
| HTTP timeout | manual socket + select | `urllib.request.urlopen(timeout=)` | stdlib, one line, exactly the bounded-timeout primitive D-01 wants |
| AWS_* matching | new env filter | extend `is_dangerous_env_var` (`env_sanitization.rs:16`) | Single chokepoint already feeds all exec paths; one `starts_with("AWS_")` clause |
| Revocation store | a deny-list in nono | the provisioner `deny[]` (`policy.js:42-45`) | ZTL-03 explicit: live check IS the revocation point; no nono-side infra |
| DAAL anchoring | nono-side ledger write | the provisioner's async `attestAction` | ZTL-05: anchoring is the provisioner's concern; nono reads `audit.daal` informationally |

**Key insight:** Every Phase 93 capability has an existing in-tree pattern to mirror or an existing function to call. The phase is composition + one HKLM reader + one Python module + one CLI command + one gate — not new crypto, new HTTP stack, or new revocation infra.

## Runtime State Inventory

> This phase adds a live network dependency and a registry-read dependency. Not a rename/refactor, but the new external/stored-state surfaces are material to planning.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | In-process `jti` consumed-set (Phase 91 D-03, `override.rs` `check_and_consume_jti`); **NOT persisted** across processes — the live ZT check is the durable single-use point | None — by design; document that durable replay protection rides on the live `deny[]` |
| Live service config | **NEW:** `NONO_ZT_ACTIONS_URL` (env, ops config); the ZT-Infra provisioner deny-list lives in the provisioner's `ACTION_POLICY_FILE` / DB, **NOT in nono's git** (`policy.js:24` `/etc/zt-provisioner/actions-policy.json`). Revocation is an operator action on the provisioner, not a nono code change | Document the operator runbook: add `{action:"override.apply:<jti>"}` to provisioner `deny[]` to revoke |
| OS-registered state | **NEW:** `HKLM\SOFTWARE\Policies\nono` trust-root values (KMS pubkey DER+base64, ARN allowlist) — set via GPO/MSI, **not in git** (D-05). Mirrors the v3.0 egress-policy spine (`machine_policy.rs`) | Plan must define the value-name shape (see OQ-4) and the ADMX/GPO authoring note; gate must seed an HKCU test key (override-01 uses fixtures, not HKLM) |
| Secrets/env vars | `AWS_*` credentials in the **parent** env are stripped before the child (ZTL-04). `NONO_ZT_ACTIONS_URL`/`NONO_ZT_ACTIONS_HEADER` are ops config, never trust roots (D-05). No new secret keys minted by nono | Code: add `AWS_*` strip; test: SC3 child-env inspection |
| Build artifacts | nono-py adds `[project.scripts]` console entry → a new `nono-override-apply` script installed into the venv `Scripts/` on `pip install -e .` | Document that `pip install -e .` (or `maturin develop`) must be re-run after pyproject `[project.scripts]` change for the console-script to appear |

**Nothing found in category:** No databases store the override token; no Task Scheduler / pm2 / launchd state; no SOPS keys renamed.

## Common Pitfalls

### Pitfall 1: D-03 actor-matching assumption
**What goes wrong:** Planner assumes the provisioner matches on `actor`+`action` (as D-03's prose implies) and builds an actor allowlist expectation on the provisioner side.
**Why it happens:** D-03 says "keys only on `actor`+`action`" but the code (`policy.js:31-59`) only ever calls `matches(rule.action, action)` — `actor` is required-non-empty (`evaluateAction` line 35-37) but **never used in a match**.
**How to avoid:** The nono side must still SEND `actor` (it's required or the request is denied with `"actor is required"`), but per-token revocation rides **entirely on `action`**. The allow rule is `"override.apply*"`; the deny rule is `"override.apply:<jti>"`. Plan the provisioner policy fixtures accordingly.
**Warning signs:** A gate that revokes by changing `actor` instead of adding an `action` deny rule will silently still allow.

### Pitfall 2: Provisioner AuditRecord mistaken for an override token
**What goes wrong:** Planner expects `POST /actions` to RETURN a signed override token, or expects the provisioner to issue tokens.
**Why it happens:** Both are CAF objects; superficially similar.
**How to avoid:** The provisioner `record()` (`audit.js:236-257`) emits `{actor,action,resource,decision,reason,timestamp,previous_hash,current_hash,kms_signature,daal}` — **no `scope`/`jti`/`expires_at`**. The override token (`override.rs:329-366`) requires those under `deny_unknown_fields`. They are **distinct constructs**. The token is minted out-of-band (approver + KMS `Sign`), the provisioner only ALLOWS/DENIES it at apply time. The gate mints its own token (override-01 pattern).
**Warning signs:** A `serde` parse error "missing field `scope`" when feeding a provisioner response to `verify_override`.

### Pitfall 3: Live-deny audit event never reaches the HMAC chain
**What goes wrong:** D-02 maps live-deny → EventID 10010, but on a live deny nono-py fails closed **before** spawning `nono.exe` — and the chain is only written by nono-cli at spawn (`execution_runtime.rs:259-288`). The REVOKED/REJECTED event has no emission path.
**Why it happens:** Phase 92 wired emission to the spawn path only; the live arm rejects before spawn.
**How to avoid:** See OQ-1. Options: (a) nono-py invokes `nono.exe override audit-emit <meta>` (a new thin subcommand) on the reject branch; (b) accept that reject events are surfaced only via the `NonoOverrideError` raised to the Python caller + the provisioner's own audit chain, and record VFY-01 closure without a nono-cli chain entry for the reject case; (c) emit at apply-time only (allow path) and document the reject path as provisioner-audited. **This is a real decision the planner must make explicit.**
**Warning signs:** A success criterion claiming "10010 appears in the nono-cli HMAC chain on revocation" with no code path that emits it.

### Pitfall 4: Stripping `AWS_REGION` breaks nothing but stripping it from the provisioner-side breaks the gate
**What goes wrong:** ZTL-04 strips `AWS_*` from the **sandboxed child**. If the OVERRIDE-02 gate runs the provisioner in the same shell, stripping `AWS_*` from the gate's own env (not the child) could break the provisioner's KMS calls.
**Why it happens:** Conflating "child env" with "gate env".
**How to avoid:** ZTL-04 strips only in `build_child_env` (the confined child), never the gate/provisioner process. The provisioner runs as a separate process with its own (full) env. The SC3 test inspects the **child's** env, not the parent's.
**Warning signs:** Provisioner KMS `Sign` failing with missing-credentials during the gate.

### Pitfall 5: `urllib` default proxy/redirect handling
**What goes wrong:** `urllib.request.urlopen` honors `HTTP_PROXY`/`HTTPS_PROXY` env and follows redirects, which could route the trust-critical live check through an attacker-controlled proxy or to an unexpected host.
**Why it happens:** stdlib defaults.
**How to avoid:** Build the opener with an empty `ProxyHandler({})` to disable env-proxy, and do not follow cross-host redirects (the provisioner returns 200/403 directly, never 3xx — `server.js:51`). The endpoint is the D-04 trust anchor; it must not be silently re-pointed.
**Warning signs:** A live check that "passes" against a host other than `NONO_ZT_ACTIONS_URL`.

### Pitfall 6: Cross-target clippy on the new HKLM reader
**What goes wrong:** The new `override_trust.rs` is `#[cfg(windows)]`; Windows-host `cargo check` does not run clippy on the Linux/macOS cfg branches, and the non-Windows stub may drift.
**Why it happens:** CLAUDE.md cross-target MUST/NEVER rule; recurrent in this fork.
**How to avoid:** Provide a non-Windows stub that returns the same fail-closed `Result` shape (mirror `machine_policy.rs:533-536` `Ok(None)` stub). Mark the cross-target clippy verification PARTIAL→CI per the checklist if the cross-toolchain is absent (the standing pattern, e.g. 92-VERIFICATION `Anti-Patterns` row).
**Warning signs:** `unreachable_patterns` or `dead_code` on Linux for Windows-only enum arms.

## Code Examples

### Reading the trust config from HKLM (D-06, mirroring machine_policy.rs)

```rust
// Source: machine_policy.rs:466-498 (the fail-secure read taxonomy to mirror)
// nono-py/src/override_trust.rs (NEW, #[cfg(windows)])
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
use winreg::RegKey;
const POLICY_PATH: &str = r"SOFTWARE\Policies\nono";
const ERROR_FILE_NOT_FOUND: i32 = 2;
// Read sub-key `Override\AllowedKeyArns` (N×REG_SZ, ADMX <list>) and
// `Override\KmsPublicKeys\<key_id>` (REG_SZ, DER+base64). Absent → fail-closed deny.
// Present-but-unreadable → Err (NEVER fall through — machine_policy.rs:477-482).
```

### AWS_* strip (ZTL-04, one clause)

```rust
// Source: env_sanitization.rs:16-77 (is_dangerous_env_var)
// ADD near the LD_/DYLD_ prefix checks (line 18-19):
//     || key.starts_with("AWS_")   // ZTL-04: AWS credentials never reach the child
// This is cfg-UNCONDITIONAL (correct: AWS creds are dangerous on every platform),
// unlike the Windows-gated PATH/SystemRoot block (lines 66-77). The existing
// SystemRoot/windir baseline re-add for CLR children (separate code path) is untouched.
```

### OVERRIDE-02 gate precondition delta (mirror override-01.ps1:77-115)

```powershell
# Source: override-01.ps1 Test-Precondition + server.js:7-8 (127.0.0.1:3000)
# ADD to Test-Precondition (after the override-01 checks):
#   6. Probe the local provisioner /health endpoint.
$ztUrl = $env:NONO_ZT_ACTIONS_URL
if (-not $ztUrl) { return 'NONO_ZT_ACTIONS_URL not set — start the local provisioner (cd provisioner; npm start) and set NONO_ZT_ACTIONS_URL=http://127.0.0.1:3000/actions' }
try {
    $health = Invoke-WebRequest -Uri ($ztUrl -replace '/actions$','/health') -TimeoutSec 2 -UseBasicParsing
    if ($health.StatusCode -ne 200) { return "ZT-Infra provisioner unhealthy ($($health.StatusCode)) — SKIP_HOST_UNAVAILABLE" }
} catch { return "ZT-Infra provisioner unreachable at $ztUrl — start it (npm start) then re-run; SKIP_HOST_UNAVAILABLE" }
# Invoke-Gate then: mint token (override-01 make_token), seed an allow rule
# override.apply*, verify allow → grant; seed a deny rule override.apply:<jti>,
# verify deny → NonoOverrideError(LiveRevoked).
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Offline-only verify (Phase 91) | Offline + live AND-gate (Phase 93) | This phase | An offline-valid but revoked token is now rejected on the next live check |
| `pubkey_der` test-injected (`override.rs:609`) | HKLM-sourced, per-`key_id` cached (D-06) | This phase | Closes the `[BLOCKING-93]` VFY-03a seam; production trust roots |
| EventID map ends at apply (10007) | adds reject(10008)/revoke(10010) for live arm (D-02) | This phase | Operators get a revoked-vs-infra-down signal (subject to OQ-1 emission path) |

**Deprecated/outdated:**
- The Phase 91 `verify_keyed_signature` (Sigstore DSSE) reference — already superseded by `verify_prehashed` (Phase 91 D-05); do not reintroduce.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `winreg` is reachable as a (transitive) dep of nono-py and can be promoted to direct | Standard Stack | If absent, must add it or use raw `windows-sys`; low risk (core nono already uses it) |
| A2 | The real KMS-issued token's byte layout matches CAF v0.1 once it carries the override-specific signed fields | Summary §2 / OQ-3 | If a real provisioner-issued token uses a different envelope, `verify_override` parse fails; mitigated because the token is a nono-side construct minted by the approver pipeline, not the provisioner — the pipeline can be shaped to match |
| A3 | The `Override\` sub-key value-name shape (`AllowedKeyArns`, `KmsPublicKeys\<key_id>`) is the planner's to define | OQ-4 | Wrong names just need a doc/ADMX fix; no security impact (fail-closed if absent) |
| A4 | nono-py adding `[project.scripts]` for `nono override apply` is acceptable as a `nono-override-apply` console entry (the literal `nono override apply` 3-token form needs nono.exe delegation OR the user invokes the console-script directly) | OQ-5 | If the user expects `nono override apply` to dispatch from nono.exe, that needs a nono.exe→console-script shell-out, which D-07 forbids ("no nono.exe→nono-py shell-back"). Flag for confirmation. |

## Open Questions

1. **Where do live-reject (10008/10010) audit events get emitted into the nono-cli HMAC chain?**
   - What we know: the chain is written only by nono-cli at spawn via `--override-audit` (`execution_runtime.rs:259-288`); the live arm fails closed in nono-py before spawn.
   - What's unclear: D-02 maps reject→EventID, but no code path emits on the reject branch.
   - Recommendation: Planner picks one — (a) a new `nono.exe override audit-emit <meta> --kind rejected|revoked` thin subcommand nono-py invokes on the reject branch (keeps the chain authoritative, costs one CLI round-trip); (b) reject events are surfaced via the raised `NonoOverrideError` + the provisioner's own KMS-signed audit chain only, and VFY-01 closes with a documented note that the nono-cli chain records only the **allow** (applied) path. **Lean: (a)** for AUD-01 completeness, but (b) is defensible and cheaper. Make it an explicit SC.

2. **Is the live arm invoked from inside the Rust `confined_run` `#[pyfunction]`, or from a Python wrapper around it?**
   - What we know: `confined_run`/`confine` are Rust `#[pyfunction]`s (`windows_confined_run.rs:367,460`); D-01 says the live arm is Python `urllib`. The seam comment sits at `:404-406`/`:513-515` (Rust, before `append_override_args`).
   - What's unclear: a Rust `#[pyfunction]` cannot call a Python `urllib` function cleanly mid-body without re-entering Python.
   - Recommendation: Make the live check a **Python-level pre-step**: a Python wrapper (`confined_run_checked` or the existing `confined_run` re-exported through a thin Python shim in `__init__.py`) that calls `_live.live_check(grant)` BEFORE calling the Rust `confined_run(override_token=grant)`. The Rust seam comment then documents "live check performed by the Python caller; this fn assumes the grant is live-verified." Cleanest given D-01.

3. **Real-KMS token reconciliation (Phase 91 D-06 `[BLOCKING]`):** confirm a real `KMS Sign(DIGEST)` over the canonical bytes yields a signature `verify_ecdsa_digest` accepts.
   - What we know: `CANONICAL_FORM.md:154-162` + the provisioner `signHash` (`audit.js:184-197`) both sign the raw 32-byte SHA-256 digest with `ECDSA_SHA_256`, DER, base64, low-S-normalized — **identical** to what `verify_ecdsa_digest` expects (`override.rs:621-634` + low-S at `:788`). The 9 test vectors prove canonical-byte parity.
   - What's unclear: whether the **approver pipeline** that mints real override tokens preserves the `deny_unknown_fields` shape (no extra fields).
   - Recommendation: The OVERRIDE-02 gate's local-keypair mint IS the reconciliation proof at the byte/crypto level. Document that the production minting pipeline must produce a token matching `OverrideToken` (`override.rs:329-366`) exactly; this is a contract on the approver tooling, not nono code. Close `[BLOCKING-93]` by recording this contract.

4. **HKLM `Override\` value-name schema (D-06):** what value names hold the pubkey and ARNs?
   - What we know: the egress spine uses `AllowedSuffixes\`/`AllowedHosts\`/`PresetTokens\` as ADMX `<list>` sub-keys of N×REG_SZ (`machine_policy.rs:430-432`).
   - Recommendation: Mirror that — `Override\AllowedKeyArns\` (N×REG_SZ list) and `Override\KmsPublicKeys\` (named values: value-name=`key_id`, data=DER+base64 REG_SZ). Define in the plan; author an ADMX note. Fail-closed if absent (D-05).

5. **Does `nono override apply` dispatch from nono.exe or as a standalone console-script?**
   - What we know: D-07 says `apply` is nono-py-delivered and forbids nono.exe→nono-py shell-back; CLI-02 text says "via `nono override apply`".
   - Recommendation: Register a `[project.scripts]` console entry. The literal token form is ambiguous — confirm whether the operator types `nono override apply ...` (needs nono.exe to dispatch, contradicting D-07) or `nono-override-apply ...` (clean console-script). **Surface to user via AskUserQuestion during discuss/plan.** Default lean: console-script `nono-override-apply`, documented as the `apply` affordance.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| ZT-Infra local provisioner | OVERRIDE-02 live gate; live-arm integration test | ✗ (not running this session) | node `127.0.0.1:3000` (`server.js:7-8`) | `SKIP_HOST_UNAVAILABLE` (exit 3) — DF-02 contract |
| AWS KMS + DAAL control plane | end-to-end real-token live path | ✗ (unreachable from this host) | ZT-Infra v2 AWS | local provisioner with `signer` injected (`audit.js:177`) or `algorithm:"none"` local-fallback (NOT accepted by verifier — correct) |
| Python ≥ 3.10 | live arm, apply, tests | ✓ (assumed; pyproject requires) | `>=3.10` | none — hard requirement |
| `nono.exe` ≥ 3.2 on PATH | confined_run probe (`probe_override_support`) | host-gated | 3.2+ (`windows_confined_run.rs:238-259`) | gate uses `nono 3.2.0` stub (`override-01.ps1:270`) |
| openssl on PATH | token minting in gates | host-gated | any | gate SKIPs (`override-01.ps1:98-102`) |
| `winreg` crate | HKLM trust read | ✓ (in-tree via nono core) | matches `machine_policy.rs` | raw `windows-sys` |
| Elevated host + GPO-seeded HKLM | production trust-root read | ✗ | — | gate seeds HKCU test key (no admin) like `machine_policy.rs:825` tests |

**Missing dependencies with no fallback:** none block planning (all live/AWS paths are `SKIP_HOST_UNAVAILABLE` by the Dark Factory mandate).
**Missing dependencies with fallback:** ZT-Infra provisioner (start locally via `cd provisioner && npm install && npm start`); AWS KMS (local provisioner with injected signer or test keypair).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework (nono-py) | `pytest` >= 8 (`pyproject.toml [dependency-groups].dev`) |
| Framework (nono / nono-cli) | Rust built-in test runner (`cargo test`) |
| Config file | `nono-py/pyproject.toml [tool.pytest.ini_options]` (`testpaths=["tests"]`); `Nono/Makefile` (`make test`) |
| Quick run (nono-py) | `cd nono-py && python -m pytest tests/test_live_arm.py -x` |
| Quick run (nono-cli) | `cargo test -p nono-cli --lib exec_strategy::env_sanitization` |
| Full suite | `make test` (Nono workspace) + `cd nono-py && python -m pytest` |
| Dark Factory gate | `pwsh -File scripts\verify-dark.ps1 --gate override-02` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ZTL-01 | `NONO_ZT_ACTIONS_URL` read; POST body shape (actor/action/resource/correlation_id per D-03) | unit (mocked urllib) | `pytest tests/test_live_arm.py::test_post_body_shape -x` | ❌ Wave 0 |
| ZTL-02 | 2s timeout → `LiveUnavailable`; deny → `LiveRevoked`; both fail-closed | unit (mocked urllib raising TimeoutError / HTTPError 403) | `pytest tests/test_live_arm.py::test_timeout_fails_closed -x` | ❌ Wave 0 |
| ZTL-03 | deny-list id rejected on next live check (provisioner deny rule on `override.apply:<jti>`) | integration (live provisioner) / gate | `verify-dark.ps1 --gate override-02` (SKIP if no provisioner) | ❌ Wave 0 |
| ZTL-04 | child env has no `AWS_*` | unit (Rust) + integration | `cargo test -p nono-cli --lib env_sanitization::aws` + `pytest tests/test_aws_strip.py` | ❌ Wave 0 |
| ZTL-05 | nono never sends `flush_daal:true`; spawn not blocked on anchoring | unit (assert body omits flush_daal) | `pytest tests/test_live_arm.py::test_no_flush_daal -x` | ❌ Wave 0 |
| CLI-01 | `nono override request` emits JSON bundle from DiagnosticFormatter context | unit (Rust) | `cargo test -p nono-cli --lib override_request` | ❌ Wave 0 |
| CLI-02 | `nono override apply` runs offline+live verify before run | integration (mocked live) | `pytest tests/test_override_apply.py -x` | ❌ Wave 0 |
| DF-02 | live gate emits PASS on allow+deny path, SKIP when provisioner absent | gate | `pwsh -File scripts\verify-dark.ps1 --gate override-02` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** the quick run for the touched repo (`pytest tests/test_live_arm.py -x` or the relevant `cargo test -p` filter).
- **Per wave merge:** `make test` (Nono) + `python -m pytest` (nono-py) green.
- **Phase gate:** full suite green + `override-02` gate PASS or SKIP_HOST_UNAVAILABLE (never FAIL) before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `nono-py/tests/test_live_arm.py` — ZTL-01/02/05 (mock `urllib.request.urlopen`)
- [ ] `nono-py/tests/test_aws_strip.py` — ZTL-04 child-env inspection (Windows-marked)
- [ ] `nono-py/tests/test_override_apply.py` — CLI-02 apply path (mocked live)
- [ ] `crates/nono-cli/.../env_sanitization.rs` test module — add `aws_*` cases (extend existing `#[cfg(test)] mod tests`)
- [ ] `crates/nono-cli/src/override_request.rs` test module — CLI-01 bundle shape
- [ ] `scripts/gates/override-02.ps1` — DF-02 live gate (mirror override-01)
- [ ] Provisioner policy fixtures: an `allow` rule `override.apply*` and a `deny` rule `override.apply:<jti>` (gate seeds these or points `ACTION_POLICY_FILE` at a temp file, `policy.js:24`)

## Security Domain

> `security_enforcement` is not set to `false` in config — included. This is a security-critical fork (CLAUDE.md: "SECURITY IS NON-NEGOTIABLE").

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | partial | Endpoint trust is network-level (D-04); no app-level auth. The signer ARN is authenticated cryptographically (KMS sig) + allowlisted (D-05). |
| V4 Access Control | yes | Two-key AND gate; ARN allowlist (VFY-04); per-token revocation via provisioner `deny[]` (ZTL-03). Fail-closed on missing trust material (D-05). |
| V5 Input Validation | yes | `deny_unknown_fields` on `OverrideToken` (`override.rs:328`); `urllib` response parsed with `json.loads` + decision-field check; reject malformed → `LiveUnavailable`. |
| V6 Cryptography | yes | ECDSA P-256, low-S enforced (`override.rs:788`), `verify_prehashed` (no hand-rolled crypto); pubkey DER from policy, never env. |
| V9 Communication | yes | Live check over `NONO_ZT_ACTIONS_URL`; disable env-proxy + cross-host redirects (Pitfall 5); HTTPS expected for the AWS-fronted URL (Tailscale/SSM). |
| V12 Files/Resources | yes | Scope paths sanitized (`Path::components()`, no string `starts_with`); `AWS_*` stripped from child env. |

### Known Threat Patterns for {ZT-Infra live arm + HKLM trust + env strip}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Fail-open on live-check failure | Elevation of Privilege | Fail-closed mapping (D-02); every `urllib` exception → deny |
| Rogue pubkey/ARN via tampered env | Tampering / EoP | Trust roots ONLY from HKLM; env cannot widen (D-05); missing → deny |
| Endpoint re-pointing (proxy/redirect) | Tampering / Spoofing | Empty `ProxyHandler`, no cross-host redirects (Pitfall 5); URL is the trust anchor |
| AWS credential leak to child | Information Disclosure | `AWS_*` strip at `is_dangerous_env_var` (ZTL-04); SC3 child-env test |
| Replayed/revoked token | Spoofing / EoP | In-process jti set (offline) + live `deny[]` (durable, cross-process) — ZTL-03 |
| TOCTOU verify→apply | EoP | Live arm consumes the immutable `OverrideGrant` (`override.rs:660` `frozen`); never re-parses |
| DAAL anchoring used as a gate (DoS) | Denial of Service | ZTL-05: anchoring is async, never blocks spawn; nono never sets `flush_daal` |
| Unreadable HKLM → silent fall-through | EoP | Mirror `machine_policy.rs:477-482` — present-but-unreadable aborts, never falls through |

## Sources

### Primary (HIGH confidence — read at file:line this session)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\src\server.js:7-70` — `POST /actions` request/response contract; 200 allow / 403 deny; `audit.current_hash`; `daal_flush` opt-in.
- `...\provisioner\src\policy.js:14-59` — `evaluateAction` keys on `action` only; `deny[]` enforcement; `defaultDecision:"deny"`; prefix-`*` `matches`.
- `...\provisioner\src\audit.js:107-257` — `record()` AuditRecord shape; `signHash` (KMS `Sign` DIGEST mode, low-S normalize); `normalizeEcdsaDerLowS`.
- `...\provisioner\src\daal.js:50-60,304-344` — `attestAction` / `enqueueAction` / `drain` async semantics (ZTL-05).
- `...\docs\CANONICAL_FORM.md:118-162` — R10 strip, hash + signature computation (raw 32-byte digest, ECDSA P-256, low-S).
- `...\test-vectors\canonical-form\vectors.json` (TV-01..09) — AuditRecords/registry entries, NO override fields (token-distinct proof).
- `nono-py/src/override.rs:300-366,604-868` — `OverrideToken`/`KmsSignature` wire shape; `verify_ecdsa_digest`; `OverrideGrant`; `verify_override_impl`; `[BLOCKING-93]` seam.
- `nono-py/src/windows_confined_run.rs:160-330,367-520` — probe, sanitize, `append_override_args`, `confined_run`/`confine`, VFY-01 seam comments.
- `nono-py/src/lib.rs:1-74`; `nono-py/python/nono_py/__init__.py:32-119`; `nono-py/pyproject.toml`; `nono-py/Cargo.toml` — module/exports/deps/console-script gap.
- `crates/nono/src/machine_policy.rs:265-536` — the fail-secure HKLM `winreg` reader pattern D-05/D-06 mirror.
- `crates/nono-cli/src/exec_strategy/env_sanitization.rs:16-100` — `is_dangerous_env_var` chokepoint (ZTL-04 locus) + AWS_* allow-pattern precedent (tests at `:348-357`).
- `crates/nono-cli/src/execution_runtime.rs:200-288` — AUD-04 pre-spawn gate; `--override-audit` emission path (Pitfall 3 evidence).
- `crates/nono-cli/src/cli.rs:644,1058,1432-1450,1628-1648,2021-2032` — `Commands` enum, `ProfileCommands` nesting pattern, `OverrideAuditMeta`/`--override-audit`.
- `crates/nono-cli/src/diagnostic/formatter.rs:1-60` — `DiagnosticFormatter`/`PolicyExplanation` (CLI-01).
- `scripts/gates/override-01.ps1` — the gate contract + `make_token` to mirror for override-02.
- `.planning/phases/92-...-VERIFICATION.md` — wired surfaces (file:line) Phase 93 extends; VFY-01 PARTIAL evidence.

### Secondary (MEDIUM confidence)
- `proj/POC-zt-infra-e5-local-provisioner.md` (cited in CONTEXT but not found at that path this session); startup grounded instead from `provisioner/package.json` (`npm start`) + `server.js:7-8` (`127.0.0.1:3000`).
- `Python urllib.request.urlopen(timeout=)` semantics `[CITED: docs.python.org/3/library/urllib.request]`.

### Tertiary (LOW confidence — flagged for validation)
- A1 (`winreg` reachable as nono-py dep) — confirm `cargo tree` at plan time.
- A4/OQ-5 (`nono override apply` literal-token dispatch vs console-script) — confirm with user.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new deps; all in-tree/stdlib, file:line verified.
- Architecture: HIGH — every tier grounded in read source; D-01..D-08 cross-checked against code; 3 contradictions surfaced.
- Pitfalls: HIGH — each tied to a specific file:line behavior (provisioner keying, token shape, emission path, urllib defaults).
- Live/AWS claims: MEDIUM — control plane unreachable this host; scoped to local provisioner + static verification per the mandate.

**Research date:** 2026-06-22
**Valid until:** 2026-07-22 (stable in-tree; provisioner contract is local and pinned. Re-check if ZT-Infra v2 `server.js`/`policy.js` change or nono-py adds a HTTP dep.)
