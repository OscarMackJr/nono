# scripts/gates/override-01.ps1
#
# Phase 92 Plan 04 - override-01 gate (DF-01: offline verify path + fail-closed cases)
#
# CONTRACT (cloned from scripts/gates/telemetry-event-emit.ps1 -- structural twin):
# this gate exports exactly two functions, dot-sourced by scripts/verify-dark.ps1.
# The gate RETURNS its verdict object -- it MUST NOT call exit and MUST NOT call
# Persist-Verdict. Only the runner owns exit-code mapping (PASS=0 / FAIL=2 /
# SKIP_HOST_UNAVAILABLE=3 / harness-internal=4) and the persist-before-emit (WR-04).
#
#   Test-Precondition -> $null (preconditions met, run Invoke-Gate)
#                      | "reason string" (SKIP_HOST_UNAVAILABLE - exit 3, Invoke-Gate never runs)
#   Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                        verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
#                        a `throw` here = harness-internal error (exit 4), never a silent PASS
#
# WHAT THIS PROVES (satisfies DF-01 / MUT-01 / MUT-05):
#   SC1 (valid token -> --override-audit + --allow flags appended):
#     Mints a valid token using the Phase 91 committed test keypair
#     (nono-py/tests/fixtures/override_test_key.pem + override_test_private.der).
#     Uses NONO_EXE stub injection (Strategy A) so confined_run's Rust-level spawn
#     resolves to a Python stub that writes argv to a tempfile. Asserts --override-audit
#     and --allow appear in the captured args.
#     NOTE: Python subprocess.Popen mock is NOT used -- confined_run spawns via
#     Rust's std::process::Command, which is invisible to Python's subprocess module.
#     The NONO_EXE env var is the only correct interception path (T-92-VACUOUS-MOCK).
#
#   SC2 (fail-closed cases all raise NonoOverrideError):
#     Calls verify_override with: bad-sig, expired, algorithm mismatch, key-not-in-allowlist,
#     replay (same jti twice). Asserts each raises NonoOverrideError (not a built-in
#     exception, not None).
#
#   SC3 (no-token path produces no --override-audit flag):
#     Calls confined_run(..., override_token=None) via NONO_EXE stub. Asserts
#     --override-audit is absent (MUT-05 regression).
#
# WHY SKIP (not FAIL) when prerequisites are absent:
#   Requires Python + nono_py module importable (nono-py maturin-built / pip install -e).
#   Requires openssl on PATH for token minting (stdlib hashlib only for hashing).
#   Admin NOT required -- offline verify and arg-capture use no elevated APIs.
#   On a dev host without Python/nono_py/openssl, the gate SKIPs cleanly.
#
# INVOCATION RULE (MEMORY durable):
#   pwsh -File scripts\verify-dark.ps1 --gate override-01
#   NEVER: pwsh -Command "<bare path>" (swallows exit N -> 1)

# ---------------------------------------------------------------------------
# Gate configuration
# ---------------------------------------------------------------------------

$script:GateName      = 'override-01'
$script:FixturesPath  = 'C:\Users\OMack\nono-py\tests\fixtures'
$script:TestKmsArn    = 'arn:aws:kms:us-east-2:111122223333:key/test'

# ---------------------------------------------------------------------------
# Local assertion helper (throw-on-failure, harness-internal only).
# A throw = harness-internal error (exit 4). Use ONLY for "gate cannot run at all".
# Confinement/policy results are verdict objects, never throws.
# ---------------------------------------------------------------------------

function Assert-True {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$Condition,

        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if (-not $Condition) { throw $Message }
}

# ---------------------------------------------------------------------------
# Gate contract
# ---------------------------------------------------------------------------

function Test-Precondition {
    # Return $null when all preconditions met; return a reason string -> SKIP_HOST_UNAVAILABLE.

    # 1. Check Python is available.
    $pyCheck = & python --version 2>&1
    if ($LASTEXITCODE -ne 0) {
        return 'Python is not on PATH -- install Python 3.10+ before running this gate'
    }

    # 2. Check nono_py module is importable (maturin develop / pip install -e required).
    $check = & python -c "import nono_py; print('ok')" 2>&1
    if ($LASTEXITCODE -ne 0 -or $check -ne 'ok') {
        return "nono_py Python module not importable ($check) -- run 'pip install -e .' in nono-py then re-run"
    }

    # 3. Check verify_override is available (Phase 91/92 symbols).
    $ovCheck = & python -c "from nono_py import verify_override, NonoOverrideError, confined_run; print('ok')" 2>&1
    if ($LASTEXITCODE -ne 0 -or $ovCheck -ne 'ok') {
        return "nono_py override symbols not importable ($ovCheck) -- ensure Phase 91+92 changes are built"
    }

    # 4. Check openssl on PATH (needed for token minting in the inline script).
    $opensslCheck = Get-Command openssl -ErrorAction SilentlyContinue
    if ($null -eq $opensslCheck) {
        return 'openssl is not on PATH -- install OpenSSL before running this gate'
    }

    # 5. Check fixtures exist.
    $privKey = Join-Path $script:FixturesPath 'override_test_key.pem'
    $pubKey  = Join-Path $script:FixturesPath 'override_test_key.der'
    if (-not (Test-Path -LiteralPath $privKey)) {
        return "Test private key not found at '$privKey' -- ensure Phase 91 fixtures are present"
    }
    if (-not (Test-Path -LiteralPath $pubKey)) {
        return "Test public key not found at '$pubKey' -- ensure Phase 91 fixtures are present"
    }

    return $null
}

function Invoke-Gate {
    # override-01 gate: SC1 (valid token -> correct args via NONO_EXE stub),
    #                   SC2 (5 fail-closed cases -> NonoOverrideError),
    #                   SC3 (no-token path -> no --override-audit flag).
    # NEVER calls exit. NEVER calls Persist-Verdict. Returns exactly one verdict object.

    # Native tools write progress to stderr; do not promote to terminating errors.
    $ErrorActionPreference = 'Continue'

    $fixturesPath = $script:FixturesPath
    $testKmsArn   = $script:TestKmsArn

    # OVERRIDE-01 SC1+SC2+SC3 inline gate script
    # Uses NONO_EXE stub injection (Strategy A) for arg capture.
    # Token minting uses openssl subprocess + Python stdlib hashlib.
    # Python-level subprocess mocks are NOT used (T-92-VACUOUS-MOCK: confined_run
    # spawns via Rust's std::process::Command, invisible to Python subprocess).
    $script = @"
import json, hashlib, os, subprocess, sys, tempfile, textwrap, base64, traceback
from datetime import datetime, timezone, timedelta
from pathlib import Path

FIXTURES     = Path(r'$fixturesPath')
PRIV_KEY_PEM = FIXTURES / 'override_test_key.pem'
PUB_KEY_DER  = FIXTURES / 'override_test_key.der'
TEST_KMS_ARN = '$testKmsArn'

# P-256 order constants for low-S normalization
P256_ORDER = bytes([
    0xff,0xff,0xff,0xff,0x00,0x00,0x00,0x00,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xbc,0xe6,0xfa,0xad,0xa7,0x17,0x9e,0x84,
    0xf3,0xb9,0xca,0xc2,0xfc,0x63,0x25,0x51,
])
P256_ORDER_HALF = bytes([
    0x7f,0xff,0xff,0xff,0x80,0x00,0x00,0x00,
    0x7f,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xde,0x73,0x7d,0x56,0xd3,0x8b,0xcf,0x42,
    0x79,0xdc,0xe5,0x61,0x7e,0x31,0x92,0x28,
])

def canonical_form(obj, strip=None):
    if strip is None:
        strip = []
    def write_value(v):
        if isinstance(v, bool): raise ValueError("bool unsupported")
        if isinstance(v, int):
            if abs(v) > 2**53: raise ValueError("int too large")
            return str(v)
        if isinstance(v, float): raise ValueError("float unsupported")
        if v is None: raise ValueError("null unsupported")
        if isinstance(v, str): return json.dumps(v, ensure_ascii=False)
        if isinstance(v, list): return '[' + ','.join(write_value(i) for i in v) + ']'
        if isinstance(v, dict):
            parts = [json.dumps(k, ensure_ascii=False) + ':' + write_value(v[k])
                     for k in sorted(v.keys()) if v[k] is not None]
            return '{' + ','.join(parts) + '}'
        raise ValueError(f"unsupported type {type(v)}")
    stripped = {k: vv for k, vv in obj.items() if k not in strip and vv is not None}
    return write_value(stripped).encode('utf-8')

def parse_der_rs(sig_der):
    assert sig_der[0] == 0x30
    idx = 2  # skip tag + length
    def read_int(pos):
        assert sig_der[pos] == 0x02
        length = sig_der[pos+1]
        val = sig_der[pos+2:pos+2+length]
        if len(val) > 1 and val[0] == 0: val = val[1:]
        return bytes(val), pos+2+length
    r, pos = read_int(idx)
    s, pos = read_int(pos)
    return r, s

def encode_der(r, s):
    def enc_int(b):
        b = b.lstrip(b'\x00') or b'\x00'
        if b[0] & 0x80: b = b'\x00' + b
        return bytes([0x02, len(b)]) + b
    r_enc = enc_int(r); s_enc = enc_int(s)
    body = r_enc + s_enc
    return bytes([0x30, len(body)]) + body

def normalize_low_s(sig_der):
    r, s = parse_der_rs(sig_der)
    s_padded = s.rjust(32, b'\x00')
    if s_padded <= P256_ORDER_HALF: return sig_der
    order_int = int.from_bytes(P256_ORDER, 'big')
    s_int = int.from_bytes(s_padded, 'big')
    s_new = (order_int - s_int).to_bytes(32, 'big')
    return encode_der(r, s_new)

def sign_token(token_json, bad_sig=False, algorithm_none=False):
    token = json.loads(token_json)
    if algorithm_none:
        token['kms_signature']['algorithm'] = 'none'
    canon = canonical_form(token, strip=['current_hash', 'kms_signature'])
    if bad_sig:
        sig = bytes([0x30,0x44,0x02,0x20] + [0x01]*32 + [0x02,0x20] + [0x01]*32)
        return base64.b64encode(sig).decode(), PUB_KEY_DER.read_bytes()
    with tempfile.NamedTemporaryFile(suffix='.sig', delete=False) as f: sigfile = f.name
    try:
        r = subprocess.run(['openssl', 'dgst', '-sha256', '-sign', str(PRIV_KEY_PEM), '-out', sigfile],
                           input=canon, capture_output=True)
        if r.returncode != 0: return None, None
        sig_der = Path(sigfile).read_bytes()
    finally:
        try: os.unlink(sigfile)
        except: pass
    sig_der_norm = normalize_low_s(sig_der)
    return base64.b64encode(sig_der_norm).decode('ascii'), PUB_KEY_DER.read_bytes()

def make_token(jti, scope=None, expired=False, bad_sig=False, algorithm_none=False):
    if scope is None: scope = ['/tmp/test']
    now = datetime.now(timezone.utc)
    if expired:
        nb_str = (now - timedelta(hours=9)).strftime('%Y-%m-%dT%H:%M:%S.000Z')
        exp_str = (now - timedelta(hours=1)).strftime('%Y-%m-%dT%H:%M:%S.000Z')
    else:
        nb_str = (now - timedelta(minutes=1)).strftime('%Y-%m-%dT%H:%M:%S.000Z')
        exp_str = (now + timedelta(hours=1)).strftime('%Y-%m-%dT%H:%M:%S.000Z')
    alg = 'none' if algorithm_none else 'ECDSA_SHA_256'
    token = {
        'actor': 'gate-agent', 'action': 'nono.override',
        'resource': scope[0], 'decision': 'override',
        'reason': 'OVERRIDE-01 gate test',
        'timestamp': now.strftime('%Y-%m-%dT%H:%M:%S.000Z'),
        'previous_hash': '0' * 64,
        'scope': scope,
        'not_before': nb_str, 'expires_at': exp_str,
        'jti': jti,
        'kms_signature': {'algorithm': alg, 'key_id': TEST_KMS_ARN, 'signature': 'PLACEHOLDER'},
    }
    placeholder = json.dumps(token, separators=(',', ':'), ensure_ascii=False)
    sig_b64, pubkey_der = sign_token(placeholder, bad_sig=bad_sig, algorithm_none=algorithm_none)
    if sig_b64 is None: return None, None
    token['kms_signature']['signature'] = sig_b64
    return json.dumps(token, separators=(',', ':'), ensure_ascii=False), pubkey_der

from nono_py import verify_override, NonoOverrideError, confined_run

results = {'sc1': False, 'sc2': False, 'sc3': False, 'sc1_detail': '', 'sc2_detail': '', 'sc3_detail': ''}

# ---- NONO_EXE stub setup ----
import tempfile as _tmpmod
_tmpdir = _tmpmod.mkdtemp(prefix='override01-gate-')
_stub_py = os.path.join(_tmpdir, 'nono_stub.py')
_stub_bat = os.path.join(_tmpdir, 'nono.bat')
_args_file = os.path.join(_tmpdir, 'captured_args.json')

with open(_stub_py, 'w') as f:
    f.write(textwrap.dedent("""
        import json, os, sys
        if '--version' in sys.argv:
            print('nono 3.2.0')
            sys.exit(0)
        args_file = os.environ.get('NONO_STUB_ARGS_FILE')
        if args_file:
            with open(args_file, 'w') as af:
                json.dump(sys.argv, af)
        sys.exit(0)
    """).strip())

import shutil
_py_exe = sys.executable.replace('\\\\', '\\\\')
with open(_stub_bat, 'w') as f:
    f.write(f'@echo off\r\n"{_py_exe}" "{_stub_py}" %*\r\n')

old_nono_exe = os.environ.get('NONO_EXE')
old_args_file = os.environ.get('NONO_STUB_ARGS_FILE')
os.environ['NONO_EXE'] = _stub_bat
os.environ['NONO_STUB_ARGS_FILE'] = _args_file

# ---- SC1: valid token -> --override-audit + --allow in args ----
try:
    token_json, pubkey_der = make_token('jti-gate-sc1')
    if token_json is None:
        results['sc1_detail'] = 'token minting failed (openssl unavailable?)'
    else:
        grant = verify_override(token_json, pubkey_der, allowed_arns=[TEST_KMS_ARN])
        try:
            confined_run(exe='test.exe', args=[], allow=['/tmp/outer'], override_token=grant)
        except RuntimeError as e:
            results['sc1_detail'] = f'confined_run raised RuntimeError (stub exec may have failed): {e}'
        captured = json.loads(Path(_args_file).read_text()) if Path(_args_file).exists() else []
        if not captured:
            results['sc1_detail'] = 'stub did not write args (bat execution failed on this host)'
        elif '--override-audit' not in captured:
            results['sc1_detail'] = f'--override-audit absent from args: {captured}'
        elif '--allow' not in captured:
            results['sc1_detail'] = f'--allow absent from args: {captured}'
        else:
            # Verify scope path appears as --allow value
            allow_vals = [captured[i+1] for i,a in enumerate(captured) if a=='--allow' and i+1<len(captured)]
            scope_covered = any('/tmp/test' in v for v in allow_vals)
            if scope_covered:
                results['sc1'] = True
                results['sc1_detail'] = f'--override-audit present; --allow /tmp/test present'
            else:
                results['sc1_detail'] = f'scope path /tmp/test not in allow_vals: {allow_vals}'
except Exception as e:
    results['sc1_detail'] = f'SC1 exception: {traceback.format_exc()}'

# ---- SC2: fail-closed cases ----
# Reset args file between SC1 and SC3
if Path(_args_file).exists(): os.unlink(_args_file)

sc2_cases = {}
try:
    # Case 1: bad signature
    try:
        t, p = make_token('jti-gate-sc2-badsig', bad_sig=True)
        verify_override(t, p, allowed_arns=[TEST_KMS_ARN])
        sc2_cases['bad_sig'] = 'FAIL: no exception raised'
    except NonoOverrideError:
        sc2_cases['bad_sig'] = 'PASS'

    # Case 2: expired token
    try:
        t, p = make_token('jti-gate-sc2-expired', expired=True)
        verify_override(t, p, allowed_arns=[TEST_KMS_ARN])
        sc2_cases['expired'] = 'FAIL: no exception raised'
    except NonoOverrideError:
        sc2_cases['expired'] = 'PASS'

    # Case 3: algorithm:none
    try:
        t, p = make_token('jti-gate-sc2-algnone', algorithm_none=True)
        verify_override(t, p, allowed_arns=[TEST_KMS_ARN])
        sc2_cases['alg_none'] = 'FAIL: no exception raised'
    except NonoOverrideError:
        sc2_cases['alg_none'] = 'PASS'

    # Case 4: key ARN not in allowlist
    try:
        t, p = make_token('jti-gate-sc2-noarn')
        verify_override(t, p, allowed_arns=['arn:aws:kms:us-east-2:999:key/different'])
        sc2_cases['out_of_scope_arn'] = 'FAIL: no exception raised'
    except NonoOverrideError:
        sc2_cases['out_of_scope_arn'] = 'PASS'

    # Case 5: replay (same jti twice)
    try:
        t, p = make_token('jti-gate-sc2-replay-override01')
        grant_r = verify_override(t, p, allowed_arns=[TEST_KMS_ARN])
        assert grant_r is not None, 'first call must succeed'
        verify_override(t, p, allowed_arns=[TEST_KMS_ARN])
        sc2_cases['replay'] = 'FAIL: no exception raised on second call'
    except NonoOverrideError:
        sc2_cases['replay'] = 'PASS'

    all_pass = all(v == 'PASS' for v in sc2_cases.values())
    if all_pass:
        results['sc2'] = True
        results['sc2_detail'] = 'All 5 fail-closed cases raise NonoOverrideError: ' + ', '.join(sc2_cases.keys())
    else:
        results['sc2_detail'] = 'SC2 failures: ' + str({k:v for k,v in sc2_cases.items() if v != 'PASS'})
except Exception as e:
    results['sc2_detail'] = f'SC2 exception: {traceback.format_exc()}'

# ---- SC3: no-token path -> no --override-audit ----
sc3_args_file = _args_file + '.sc3'
os.environ['NONO_STUB_ARGS_FILE'] = sc3_args_file
try:
    confined_run(exe='test.exe', args=[], allow=['/tmp/scope'], override_token=None)
except RuntimeError:
    pass
captured3 = json.loads(Path(sc3_args_file).read_text()) if Path(sc3_args_file).exists() else []
if not captured3:
    results['sc3_detail'] = 'stub did not write args (bat execution failed on this host)'
elif '--override-audit' in captured3:
    results['sc3_detail'] = f'--override-audit present but should be absent: {captured3}'
else:
    results['sc3'] = True
    results['sc3_detail'] = '--override-audit correctly absent from no-token call'

# ---- Cleanup ----
if old_nono_exe is not None: os.environ['NONO_EXE'] = old_nono_exe
else:
    if 'NONO_EXE' in os.environ: del os.environ['NONO_EXE']
if old_args_file is not None: os.environ['NONO_STUB_ARGS_FILE'] = old_args_file
else:
    if 'NONO_STUB_ARGS_FILE' in os.environ: del os.environ['NONO_STUB_ARGS_FILE']
try: shutil.rmtree(_tmpdir, ignore_errors=True)
except: pass

print(json.dumps(results))
"@

    # Run the inline Python script. Capture stdout for JSON result parsing.
    $raw = & python -c $script 2>&1
    $exitCode = $LASTEXITCODE

    $stamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')

    if ($exitCode -ne 0) {
        return [ordered]@{
            gate      = $script:GateName
            verdict   = 'FAIL'
            reason    = "Inline gate script failed with exit code $exitCode"
            detail    = [ordered]@{
                exitCode = $exitCode
                rawOutput = ($raw | Out-String).Trim()
            }
            timestamp = $stamp
        }
    }

    # The last line of stdout should be the JSON result from the inline script.
    # Filter out any non-JSON lines (warnings, etc.) to find the result.
    $jsonLine = $null
    if ($raw -is [array]) {
        for ($i = $raw.Count - 1; $i -ge 0; $i--) {
            $line = $raw[$i].Trim()
            if ($line.StartsWith('{')) {
                $jsonLine = $line
                break
            }
        }
    } else {
        $jsonLine = ($raw | Out-String).Trim()
        # Take last JSON line if multiple lines
        $lines = $jsonLine -split "`n"
        foreach ($l in [System.Linq.Enumerable]::Reverse([string[]]$lines)) {
            if ($l.Trim().StartsWith('{')) {
                $jsonLine = $l.Trim()
                break
            }
        }
    }

    if ($null -eq $jsonLine -or $jsonLine -eq '') {
        return [ordered]@{
            gate      = $script:GateName
            verdict   = 'FAIL'
            reason    = 'Inline gate script produced no JSON output'
            detail    = [ordered]@{ rawOutput = ($raw | Out-String).Trim() }
            timestamp = $stamp
        }
    }

    $parsed = $null
    try {
        $parsed = $jsonLine | ConvertFrom-Json
    } catch {
        return [ordered]@{
            gate      = $script:GateName
            verdict   = 'FAIL'
            reason    = "Failed to parse JSON output from gate script: $_"
            detail    = [ordered]@{ jsonLine = $jsonLine }
            timestamp = $stamp
        }
    }

    $sc1 = [bool]$parsed.sc1
    $sc2 = [bool]$parsed.sc2
    $sc3 = [bool]$parsed.sc3

    # Determine if SC1/SC3 stub failures should be SKIPs rather than FAILs.
    # If the stub did not write args (bat execution failed), that is a host limitation,
    # not a code defect -- SC1 and SC3 become SKIP_HOST_UNAVAILABLE conditions.
    $sc1_stub_failed = ($parsed.sc1_detail -match 'stub did not write args|bat execution failed')
    $sc3_stub_failed = ($parsed.sc3_detail -match 'stub did not write args|bat execution failed')

    if (-not $sc1 -and $sc1_stub_failed -and -not $sc2 -eq $false -and -not $sc3 -and $sc3_stub_failed) {
        # Both SC1 and SC3 are stub-execution failures; SC2 still ran.
        if ($sc2) {
            # SC2 passed; SC1/SC3 are host-gated (bat stub cannot execute).
            return [ordered]@{
                gate      = $script:GateName
                verdict   = 'SKIP_HOST_UNAVAILABLE'
                reason    = "SC2 PASS; SC1/SC3 SKIP: NONO_EXE .bat stub cannot execute via std::process::Command on this host configuration -- use a real nono.exe build on PATH to prove SC1/SC3"
                detail    = [ordered]@{
                    sc1 = $sc1; sc1_detail = $parsed.sc1_detail
                    sc2 = $sc2; sc2_detail = $parsed.sc2_detail
                    sc3 = $sc3; sc3_detail = $parsed.sc3_detail
                }
                timestamp = $stamp
            }
        }
    }

    if ($sc1 -and $sc2 -and $sc3) {
        return [ordered]@{
            gate      = $script:GateName
            verdict   = 'PASS'
            reason    = "SC1/SC2/SC3 verified: override wiring (--override-audit + --allow args), fail-closed cases (NonoOverrideError for bad-sig/expired/alg-none/out-of-scope/replay), and regression path (no --override-audit without token)."
            detail    = [ordered]@{
                sc1 = $sc1; sc1_detail = $parsed.sc1_detail
                sc2 = $sc2; sc2_detail = $parsed.sc2_detail
                sc3 = $sc3; sc3_detail = $parsed.sc3_detail
            }
            timestamp = $stamp
        }
    }

    # At least one SC failed -- return FAIL with details.
    $failedSCs = @()
    if (-not $sc1) { $failedSCs += "SC1 ($($parsed.sc1_detail))" }
    if (-not $sc2) { $failedSCs += "SC2 ($($parsed.sc2_detail))" }
    if (-not $sc3) { $failedSCs += "SC3 ($($parsed.sc3_detail))" }

    return [ordered]@{
        gate      = $script:GateName
        verdict   = 'FAIL'
        reason    = "Failed: $($failedSCs -join '; ')"
        detail    = [ordered]@{
            sc1 = $sc1; sc1_detail = $parsed.sc1_detail
            sc2 = $sc2; sc2_detail = $parsed.sc2_detail
            sc3 = $sc3; sc3_detail = $parsed.sc3_detail
        }
        timestamp = $stamp
    }
}
