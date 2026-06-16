# POC Runbook — E5 Composition: zt-infra decides, nono confines (local, no AWS)

**Status:** POC runbook for a DEFERRED integration. The production E5→`POST /actions` adapter is
NOT built in nono core — it is future work tracked by [`SEED-005`](../.planning/seeds/SEED-005-zt-infra-policy-override-attestation.md)
and the forward-compat contract in [`DESIGN-engine-abstraction.md` §"Forward-Compat: zt-infra.org
Integration"](DESIGN-engine-abstraction.md). This runbook is the cheapest way to *demonstrate* the
composition end-to-end on a single dev box — no AWS, no Terraform, no KMS — using the zt-infra
**local provisioner** plus the shipped `nono-py confined_run`.

## What it proves

The E5 control-plane ↔ execution-sandbox split, live and local:

```
agent intent ──▶ POST /actions (zt-infra local provisioner)
                   │  evaluates policy + writes hash-chained signed audit
                   ▼
              decision: allow / deny
                   │
        deny  ─────┴─▶ skip exec, FAIL-CLOSED (nothing runs)
        allow ──────▶ nono_py.confined_run(...) executes the tool UNDER OS confinement
                       (Low-IL + AppContainer + Job — enforced regardless of the allow)
```

zt-infra is the **policy decision** (allow/deny + audit). nono is the **OS enforcement** *underneath*
that decision — an `allow` never bypasses nono's confinement; a `deny` short-circuits before the OS
boundary is reached. (Per `DESIGN-engine-abstraction.md`: "nono is the execution sandbox **under**
the control plane, not a replacement for it.")

## Prerequisites

- **zt-infra repo:** `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2` (the ZT-Infra v2 codebase; Apache-2.0).
  Node.js 20+, `npm`. (The full AWS deploy is NOT needed for this POC — only the local Node service.)
- **nono-py:** the `confined_run` binding built and importable as `nono_py` (sibling repo
  `C:\Users\OMack\nono-py`, branch `44-broker-ffi-lockstep`; `maturin develop --release`). Requires
  the dev-layout `nono.exe` on PATH / discoverable (R-B4 broker trust gate) and a real Win11 host.
- A covered engine profile (e.g. `langchain-python`) and a user-owned workspace (R-B3) — see the
  5-gate stack in the Phase 72 notes if `confined_run` refuses to launch.

## Step 1 — Start the zt-infra local provisioner (no AWS)

Per the zt-infra `README.md` → "Local Provisioner":

```bash
cd /c/Users/OMack/ZeroTrust2/ZERO_TRUST_V2/provisioner
npm install
npm start                    # serves POST /actions on http://127.0.0.1:3000
curl http://127.0.0.1:3000/health
```

Sanity-check the decision contract (fail-closed default — an unauthorized action is denied):

```bash
curl -sS -X POST http://127.0.0.1:3000/actions \
  -H 'content-type: application/json' \
  -d '{"actor":"demo-agent","action":"aws.ec2.terminate_instances"}' | jq .
# Expect: decision=deny, a reason, and audit.previous_hash / audit.current_hash (SHA-256),
#         audit.kms_signature.algorithm=ECDSA_SHA_256 when KMS signing is configured (optional locally).
```

To see the **allow** path, configure the local policy (`provisioner/policies/`) to permit a specific
action for `demo-agent` (e.g. a `fs.write` / `tool.exec` your POC will run). The provisioner denies
by default, so allow is opt-in.

## Step 2 — The POC glue (illustrative; NOT nono core code)

A ~30-line Python harness that authorizes via `POST /actions`, then runs the tool confined only on
`allow`. This is the throwaway demo glue the future E5 adapter would replace — it does NOT live in
the nono or nono-py source tree.

```python
# poc_e5_guarded_run.py  — POC only; not shipped in nono/nono-py
import json, urllib.request
from nono_py import confined_run

ZT_ACTIONS = "http://127.0.0.1:3000/actions"

def authorize(actor: str, action: str, resource: str = "") -> dict:
    body = json.dumps({"actor": actor, "action": action, "resource": resource}).encode()
    req = urllib.request.Request(ZT_ACTIONS, data=body,
                                 headers={"content-type": "application/json"})
    with urllib.request.urlopen(req, timeout=5) as r:
        return json.load(r)   # {decision, reason, audit:{previous_hash,current_hash,kms_signature}}

def guarded_run(actor, action, *, exe, args, allow, profile, resource=""):
    d = authorize(actor, action, resource)
    print(f"[zt] {action} -> {d['decision']}: {d.get('reason','')}")
    print(f"[zt] audit current_hash={d['audit']['current_hash'][:16]}...")
    if d["decision"] != "allow":
        print("[zt] DENY -> fail-closed: skipping exec, nothing runs.")
        return None
    # ALLOW -> execute the tool UNDER nono OS confinement (Low-IL + AppContainer + Job)
    return confined_run(exe=exe, args=args, allow=allow, profile=profile)

if __name__ == "__main__":
    # Deny path (default policy): no process is spawned.
    guarded_run("demo-agent", "aws.ec2.terminate_instances",
                exe="notepad.exe", args=[], allow=[r"C:\Users\OMack\nono-work"],
                profile="langchain-python")

    # Allow path (after permitting the action in provisioner/policies/):
    # res = guarded_run("demo-agent", "tool.exec",
    #                   exe=r"<sys.base_prefix>\python.exe",
    #                   args=["-c", "open(r'C:\\Users\\OMack\\nono-work\\ok.txt','w').write('hi')"],
    #                   allow=[r"C:\Users\OMack\nono-work", "<sys.base_prefix>"],
    #                   profile="langchain-python")
    # print("exit:", getattr(res, "exit_code", None))
```

## Expected behavior

- **deny** → the harness prints the decision + the signed audit hash and **returns without spawning
  anything** (fail-closed). The zt-infra audit ledger still records the denied intent.
- **allow** → `confined_run` spawns the tool through `nono.exe`, OS-confined: a write inside `allow`
  lands, a write outside is denied (Low-IL / AppContainer / Job). The decision is in the audit
  ledger; the enforcement is nono's, *underneath* the allow.

Both paths produce a hash-chained, (optionally KMS-)signed audit record — that linkage of *policy
decision* (zt-infra) to *OS enforcement* (nono) is the whole point of the demo.

## What this is / is NOT

- ✅ A local, AWS-free demonstration of the E5 composition for a stakeholder/POC.
- ❌ NOT a production integration. nono core builds **no HTTP client / no `POST /actions` adapter**
  in the current milestone (Phases 71–75). The generalizable E5 hook (HTTP round-trip to the control
  plane) is the future-phase work in `SEED-005` (P3, dormant, external-ledger dependency — likely its
  own milestone).
- ❌ Not on the critical path for Phase 73 (AI_AGENT marker) or Phase 75 (supplementary controls +
  secondary engines) — both are pure nono work and do not consume zt-infra.

## Pointers

- E5 contract + mapping: [`DESIGN-engine-abstraction.md`](DESIGN-engine-abstraction.md) §"E5 — Pre-Exec
  Interception Point" and §"Forward-Compat: zt-infra.org Integration".
- Deferred integration seed: [`SEED-005`](../.planning/seeds/SEED-005-zt-infra-policy-override-attestation.md).
- zt-infra local provisioner + full deploy: `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\README.md`
  ("Local Provisioner" and "Quickstart"); architecture in `docs/ARCHITECTURE.md`, onboarding in
  `docs/ONBOARDING.md`.
- nono-py binding: `C:\Users\OMack\nono-py\src\windows_confined_run.rs` (`confined_run` / `confine`).
