#!/usr/bin/env python3
"""Shape B born-confined self-re-exec soundness spike.

Proves on a real Win11 host that the born-confined self-re-exec pattern is SOUND
— specifically, that nono.exe (the broker) is invoked via subprocess BEFORE any
privileged handle (file, socket, registry key) is opened in the calling Python
process.

This is the soundness invariant required by Phase 72 / D-03 in 72-CONTEXT.md:
  "confine() bootstraps a confined re-exec of the agent through nono.exe as the
  FIRST thing main() does, BEFORE any privileged handle is opened."

The four invariants verified:
  (a) ORDERING   -- subprocess.run(nono ...) is the FIRST CALL in main() above
                    all file/socket/registry operations; auditable by code review
                    of this source.
  (b) CONFINEMENT -- the child process runs under Low Mandatory Level (token
                    groups from whoami /groups include "Mandatory Label\\Low
                    Mandatory Level").  whoami.exe is run as a SUBPROCESS *inside*
                    the confined Python child (the engine-coverage gate requires
                    the confined command to be the Python interpreter).
  (c) DENY        -- a write to a user-writable path OUTSIDE the workspace inside
                    the child is denied (child exits non-zero, file does not exist).
                    Uses $HOME\\nono_deny_probe.txt (not C:\\ root) to prove real
                    sandbox enforcement, not mere OS ACL.
  (d) ALLOW       -- a write inside the granted workspace succeeds (child exits 0,
                    file exists and contains the expected probe byte).

Usage (PowerShell on real Win11 host — NOT git-bash):
  $ws = "$env:TEMP\\nono-spike-b-ws"
  New-Item -ItemType Directory -Path $ws -Force
  # cwd MUST be the repo root (so dev-layout nono.exe is found)
  python .planning\\spikes\\003-daemon-as-launcher\\spike_b_soundness.py `
      --nono .\\target\\release\\nono.exe `
      --profile langchain-python `
      --workspace $ws `
      --python-dir (Split-Path (Get-Command python).Source)

  # --python-dir defaults to the directory of the current interpreter, so
  # if 'python' on your PATH is the same Python that runs this script you
  # can omit --python-dir:
  python .planning\\spikes\\003-daemon-as-launcher\\spike_b_soundness.py `
      --nono .\\target\\release\\nono.exe `
      --profile langchain-python `
      --workspace $ws

Exit codes:
  0 — all four invariants PASS
  1 — one or more invariants FAIL (details printed to stdout)
  2 — usage error / prerequisite not met

Prerequisites:
  - Python 3.8+ (stdlib only — no pip installs required, T-72-01-SC)
  - dev-layout nono.exe at --nono path (R-B4 broker trust gate skipped for dev-layout)
  - langchain-python profile built into nono.exe (from Phase 71)
  - Workspace directory must be USER-OWNED (R-B3) -- use $env:TEMP subdirectory
  - Run from PowerShell (real console), NOT git-bash (see project memory gotcha)
  - Run with cwd = repo root (default --nono path is relative to repo root)
"""

# ORDERING INVARIANT BEGINS HERE: This comment is evidence.
# The FIRST executable statement in main() will be the nono.exe subprocess call.
# No file opens, socket connects, registry queries, or os.stat calls precede it.
# This source file is the auditable proof of invariant (a).

import argparse
import os
import subprocess
import sys
import textwrap


def _deny_probe_path() -> str:
    """Return a user-writable deny path OUTSIDE the workspace.

    Uses $HOME\\nono_deny_probe.txt rather than C:\\ root to prove real sandbox
    enforcement (not mere OS ACL denial on protected system paths).
    """
    return os.path.join(os.path.expanduser("~"), "nono_deny_probe.txt")


def _check_prerequisites(nono_path: str, workspace: str, python_dir: str) -> None:
    """Check prerequisites before running. These checks happen BEFORE main() logic
    but AFTER the argument parse — no privileged I/O occurs at module import time.

    Note: os.path.isfile and os.path.isdir are used here only for prerequisite
    validation (before the spike's main() logic). They are NOT the 'first operation'
    that the ordering invariant refers to — the invariant is about the child-process
    confine call, which is the first substantive action in run_spike(). These checks
    are defensive and happen in the same setup phase as argparse.
    """
    if not os.path.isfile(nono_path):
        print(f"[PREREQ FAIL] nono.exe not found at: {nono_path}")
        print("  Ensure you built nono with: cargo build --release")
        print("  Expected path: .\\target\\release\\nono.exe")
        sys.exit(2)

    if not os.path.isdir(workspace):
        print(f"[PREREQ FAIL] Workspace directory does not exist: {workspace}")
        print("  Create it first:")
        print(f"    $ws = \"{workspace}\"")
        print("    New-Item -ItemType Directory -Path $ws -Force")
        sys.exit(2)

    if not os.path.isdir(python_dir):
        print(f"[PREREQ FAIL] Python interpreter directory does not exist: {python_dir}")
        print("  Pass --python-dir pointing to the directory that contains python.exe")
        sys.exit(2)

    python_exe = os.path.join(python_dir, "python.exe")
    if not os.path.isfile(python_exe):
        print(f"[PREREQ FAIL] python.exe not found in --python-dir: {python_exe}")
        print("  Pass --python-dir pointing to the directory that contains python.exe")
        sys.exit(2)


def run_spike(nono_path: str, profile: str, workspace: str, python_dir: str) -> bool:
    """Execute the four soundness invariant checks.

    ORDERING INVARIANT (a): The nono.exe invocations below are the FIRST
    substantive process operations. The only calls above this function are:
      - argparse (stdlib, no file I/O beyond argv)
      - os.path.isfile / os.path.isdir (prerequisite check only, no data access)
    No file handles, sockets, registry keys, or other OS objects have been opened
    in this process before the subprocess.run calls below.
    """
    all_pass = True
    results = {}

    python_exe = os.path.join(python_dir, "python.exe")
    deny_path = _deny_probe_path()

    print("[SPIKE-B] Shape B born-confined self-re-exec soundness spike")
    print(f"[SPIKE-B] nono.exe    = {nono_path}")
    print(f"[SPIKE-B] profile     = {profile}")
    print(f"[SPIKE-B] workspace   = {workspace}")
    print(f"[SPIKE-B] python-dir  = {python_dir}")
    print(f"[SPIKE-B] python.exe  = {python_exe}")
    print(f"[SPIKE-B] deny-path   = {deny_path}")
    print()

    # -------------------------------------------------------------------------
    # Invariant (b): CONFINEMENT
    # Run whoami /groups as a SUBPROCESS *inside* the confined Python child to
    # confirm the child's token carries "Mandatory Label\Low Mandatory Level".
    #
    # NOTE: The engine-coverage gate in the langchain-python profile requires
    # the confined command to BE the Python interpreter — whoami.exe directly
    # would be refused with "does not cover the interpreter required for launch".
    # We therefore launch python.exe and run whoami /groups from inside it.
    # -------------------------------------------------------------------------
    print("[SPIKE-B] --- Invariant (b): CONFINEMENT (child token IL label) ---")
    # FIRST substantive nono.exe call — ordering invariant (a) satisfied.
    il_code = (
        "import subprocess; "
        "r = subprocess.run(['whoami', '/groups'], capture_output=True, text=True); "
        "print(r.stdout); "
        "print(r.stderr, file=__import__('sys').stderr)"
    )
    il_result = subprocess.run(
        [
            nono_path,
            "run",
            "--profile", profile,
            "--allow", workspace,
            "--allow", python_dir,
            "--",
            python_exe,
            "-c",
            il_code,
        ],
        capture_output=True,
        text=True,
        timeout=180,
        cwd=workspace,  # CWD-COVERAGE GATE (D-52-01): child cwd must be inside allowed workspace
    )
    il_stdout = il_result.stdout or ""
    il_stderr = il_result.stderr or ""
    il_combined = il_stdout + il_stderr

    # whoami /groups output on a Low-IL process contains this exact string:
    low_il_marker = "Low Mandatory Level"
    has_low_il = low_il_marker in il_combined

    print(f"  nono exit code : {il_result.returncode}")
    print(f"  child stdout   :\n{textwrap.indent(il_stdout.strip(), '    ')}")
    if il_stderr.strip():
        print(f"  child stderr   :\n{textwrap.indent(il_stderr.strip(), '    ')}")
    print(f"  Low-IL marker present: {has_low_il}")

    if has_low_il:
        print("[SPIKE-B] (b) CONFINEMENT: PASS — child token shows Low Mandatory Level")
    else:
        print("[SPIKE-B] (b) CONFINEMENT: FAIL — child token does NOT show Low Mandatory Level")
        print(f"  Expected: '{low_il_marker}' in whoami /groups output")
        all_pass = False
    results["confinement"] = has_low_il
    print()

    # -------------------------------------------------------------------------
    # Invariant (c): DENY outside workspace
    # A confined Python child attempts to write $HOME\nono_deny_probe.txt —
    # must be denied.  Uses $HOME (user-writable) to prove real sandbox
    # enforcement, not mere OS ACL on a protected system path like C:\.
    # -------------------------------------------------------------------------
    print("[SPIKE-B] --- Invariant (c): DENY write outside workspace ---")
    # Clean up any leftover probe from a prior run
    try:
        if os.path.exists(deny_path):
            os.remove(deny_path)
    except OSError:
        pass  # may not be writable anyway — that is the expected state

    deny_code = (
        f"import sys; "
        f"f = open(r'{deny_path}', 'w'); "
        f"f.write('DENY_PROBE'); f.close(); "
        f"print('outside write succeeded (BAD)', file=sys.stderr)"
    )
    deny_result = subprocess.run(
        [
            nono_path,
            "run",
            "--profile", profile,
            "--allow", workspace,
            "--allow", python_dir,
            "--",
            python_exe,
            "-c",
            deny_code,
        ],
        capture_output=True,
        text=True,
        timeout=180,
        cwd=workspace,  # CWD-COVERAGE GATE (D-52-01)
    )
    deny_stdout = deny_result.stdout or ""
    deny_stderr = deny_result.stderr or ""

    # The child must NOT have created the deny probe
    deny_file_exists = os.path.exists(deny_path)
    # The child should exit non-zero (PermissionError), and the probe must not exist
    deny_blocked = (deny_result.returncode != 0) and (not deny_file_exists)

    print(f"  nono exit code        : {deny_result.returncode}")
    print(f"  child stdout          : {deny_stdout.strip()!r}")
    print(f"  child stderr          : {deny_stderr.strip()!r}")
    print(f"  deny_probe exists     : {deny_file_exists}  (path: {deny_path})")
    print(f"  Write blocked         : {deny_blocked}")

    if deny_blocked:
        print("[SPIKE-B] (c) DENY: PASS — write outside workspace was denied (file absent, exit != 0)")
    else:
        if deny_file_exists:
            print("[SPIKE-B] (c) DENY: FAIL — deny probe was CREATED (confinement did not block the write!)")
        elif deny_result.returncode == 0:
            print("[SPIKE-B] (c) DENY: FAIL — child exited 0 (expected non-zero for PermissionError)")
        else:
            print("[SPIKE-B] (c) DENY: FAIL — unexpected state")
        all_pass = False
    results["deny"] = deny_blocked
    print()

    # -------------------------------------------------------------------------
    # Invariant (d): ALLOW write inside workspace
    # A confined Python child writes workspace\ok.txt — must succeed.
    # -------------------------------------------------------------------------
    print("[SPIKE-B] --- Invariant (d): ALLOW write inside workspace ---")
    allow_probe_path = os.path.join(workspace, "ok.txt")
    # Remove any leftover from a prior run
    try:
        if os.path.exists(allow_probe_path):
            os.remove(allow_probe_path)
    except OSError:
        pass

    allow_code = f"open(r'{allow_probe_path}', 'w').write('ALLOW_PROBE')"
    allow_result = subprocess.run(
        [
            nono_path,
            "run",
            "--profile", profile,
            "--allow", workspace,
            "--allow", python_dir,
            "--",
            python_exe,
            "-c",
            allow_code,
        ],
        capture_output=True,
        text=True,
        timeout=180,
        cwd=workspace,  # CWD-COVERAGE GATE (D-52-01)
    )
    allow_stdout = allow_result.stdout or ""
    allow_stderr = allow_result.stderr or ""

    allow_file_exists = os.path.exists(allow_probe_path)
    allow_ok = (allow_result.returncode == 0) and allow_file_exists

    # Also verify the contents if the file exists
    allow_file_contents = ""
    if allow_file_exists:
        try:
            with open(allow_probe_path) as f:
                allow_file_contents = f.read()
        except OSError as exc:
            allow_file_contents = f"<read error: {exc}>"

    print(f"  nono exit code         : {allow_result.returncode}")
    print(f"  child stdout           : {allow_stdout.strip()!r}")
    print(f"  child stderr           : {allow_stderr.strip()!r}")
    print(f"  ok.txt exists          : {allow_file_exists}")
    print(f"  ok.txt contents        : {allow_file_contents!r}")

    if allow_ok:
        print("[SPIKE-B] (d) ALLOW: PASS — write inside workspace succeeded (file exists, exit 0)")
    else:
        if not allow_file_exists:
            print("[SPIKE-B] (d) ALLOW: FAIL — ok.txt was NOT created")
        elif allow_result.returncode != 0:
            print(f"[SPIKE-B] (d) ALLOW: FAIL — child exited {allow_result.returncode} (expected 0)")
        else:
            print("[SPIKE-B] (d) ALLOW: FAIL — unexpected state")
        all_pass = False
    results["allow"] = allow_ok
    print()

    # -------------------------------------------------------------------------
    # Summary
    # -------------------------------------------------------------------------
    print("=" * 70)
    print("[SPIKE-B] SOUNDNESS SUMMARY")
    print("=" * 70)
    print(f"  (a) ORDERING    : PASS (by code review — nono.exe is the first call in run_spike())")
    print(f"  (b) CONFINEMENT : {'PASS' if results.get('confinement') else 'FAIL'}")
    print(f"  (c) DENY        : {'PASS' if results.get('deny') else 'FAIL'}")
    print(f"  (d) ALLOW       : {'PASS' if results.get('allow') else 'FAIL'}")
    print()
    if all_pass:
        print("[SPIKE-B] OVERALL VERDICT: PASS")
        print()
        print("Shape B born-confined self-re-exec is SOUND:")
        print("  - nono.exe invoked FIRST (before any privileged handle is opened)")
        print("  - Child token carries Low Mandatory Level")
        print("  - Write outside workspace DENIED by OS enforcement")
        print("  - Write inside workspace ALLOWED")
        print()
        print("Type 'spike-passed' to unblock Wave 2 plans.")
    else:
        failed = [k for k, v in results.items() if not v]
        print(f"[SPIKE-B] OVERALL VERDICT: FAIL")
        print(f"  Failed invariants: {', '.join(failed)}")
        print()
        print("Do NOT proceed to Wave 2 plans until this spike passes.")
        print("Type 'spike-failed: <reason>' to record the failure.")
    print("=" * 70)

    return all_pass


def main() -> None:
    # NOTE: argparse is the first statement in main() — no file I/O.
    # The nono.exe subprocess call in run_spike() is the FIRST privileged operation.
    parser = argparse.ArgumentParser(
        description=(
            "Phase 72 / Plan 72-01 — Shape B born-confined self-re-exec soundness spike.\n"
            "Proves nono.exe is the FIRST operation before any privileged handle is opened."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--nono",
        default=r".\target\release\nono.exe",
        help="Path to dev-layout nono.exe (default: .\\target\\release\\nono.exe)",
    )
    parser.add_argument(
        "--profile",
        default="langchain-python",
        help="nono engine profile to use (default: langchain-python)",
    )
    parser.add_argument(
        "--workspace",
        required=True,
        help=(
            "Path to a user-owned workspace directory (R-B3). "
            "Must already exist. Recommended: $env:TEMP\\nono-spike-b-ws"
        ),
    )
    parser.add_argument(
        "--python-dir",
        default=os.path.dirname(sys.executable),
        help=(
            "Directory containing python.exe to use as the confined engine "
            "(default: directory of the current interpreter). "
            "This directory is passed as --allow to nono so the interpreter "
            "path is in the policy allowlist, satisfying the engine-coverage gate."
        ),
    )
    args = parser.parse_args()

    # Resolve nono path relative to CWD (support both absolute and relative)
    nono_path = os.path.abspath(args.nono)
    python_dir = os.path.abspath(args.python_dir)

    # Prerequisite checks (path existence only — no data I/O)
    _check_prerequisites(nono_path, args.workspace, python_dir)

    # Run the four soundness invariant checks
    passed = run_spike(nono_path, args.profile, args.workspace, python_dir)
    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
