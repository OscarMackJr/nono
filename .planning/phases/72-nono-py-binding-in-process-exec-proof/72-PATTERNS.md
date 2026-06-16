# Phase 72: nono-py Binding + In-Process-Exec Proof - Pattern Map

**Mapped:** 2026-06-14
**Files analyzed:** 8 new/modified files
**Analogs found:** 8 / 8

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `../nono-py/src/lib.rs` (modify) | provider | request-response | `../nono-py/src/lib.rs` itself | self (extend) |
| `../nono-py/src/windows_confined_run.rs` (create) | service | request-response | `../nono-py/src/sandboxed_exec.rs` | role-match (Windows fork) |
| `../nono-py/src/policy.rs` (modify) | service | transform | `../nono-py/src/policy.rs` itself | self (extend profile resolution) |
| `../nono-py/Cargo.toml` (modify) | config | — | `../nono-py/Cargo.toml` itself | self (pin bump) |
| `../nono-py/python/nono_py/__init__.py` (modify) | config | — | `../nono-py/python/nono_py/__init__.py` itself | self (export additions) |
| `../nono-py/examples/15_langchain_confined.py` (create) | utility | event-driven | `../nono-py/examples/08_sandboxed_exec.py` | role-match |
| `../nono-py/tests/test_confined_run.py` (create) | test | request-response | `../nono-py/tests/test_integration_exec.py` | exact |
| `nono/proj/DESIGN-engine-abstraction.md` (create) | config | — | `nono/proj/DESIGN-library.md` | role-match (doc shape) |

---

## Pattern Assignments

### `../nono-py/src/windows_confined_run.rs` (service, request-response) — NEW FILE

**Analog:** `C:\Users\OMack\nono-py\src\sandboxed_exec.rs`

This is the Windows-only sibling of `sandboxed_exec.rs`. It wraps `nono.exe run --profile … -- exe args` (Shape A) and the born-confined self-re-exec (Shape B) via `std::process::Command`. The file MUST be `#[cfg(windows)]`-gated; `sandboxed_exec.rs` is `#[cfg(unix)]`-gated (implicitly — it uses `libc::fork`). The entire file is a platform fork: one role (spawn-confined-child), different mechanism.

**cfg-gate pattern** — `sandboxed_exec.rs` line 14-15:
```rust
use std::os::fd::FromRawFd;
use std::os::unix::ffi::OsStrExt;
// ... libc::fork, libc::execve, libc::waitpid — all unix-only
```
The new Windows file MUST NOT import any of these. Instead:
```rust
// windows_confined_run.rs
#[cfg(windows)]
use std::process::{Command, Stdio};
#[cfg(windows)]
use std::path::{Path, PathBuf};
```
And `sandboxed_exec.rs` should gain an explicit `#[cfg(unix)]` file-level gate so cross-target clippy catches drift. Copy pattern from CLAUDE.md § Cross-target discipline.

**ExecResult reuse** — `sandboxed_exec.rs` lines 19-45:
```rust
#[pyclass(frozen)]
pub struct ExecResult {
    #[pyo3(get)]
    pub stdout: Vec<u8>,
    #[pyo3(get)]
    pub stderr: Vec<u8>,
    #[pyo3(get)]
    pub exit_code: i32,
}
```
`ExecResult` is defined in `sandboxed_exec.rs`. On Windows, re-export or define a parallel struct in `windows_confined_run.rs`. Given that `ExecResult` has no Unix-specific fields, the simplest approach is to move its definition to `lib.rs` (platform-neutral) and import from both platform files. If staying in `sandboxed_exec.rs`, the Windows file defines a compatible struct with identical fields and the planner documents whether to unify.

**Function signature to copy** — `sandboxed_exec.rs` lines 88-97 (`#[pyfunction]` / `#[pyo3(signature = ...)]`):
```rust
#[pyfunction]
#[pyo3(signature = (caps, command, cwd=None, timeout_secs=None, env=None))]
pub fn sandboxed_exec(
    py: Python<'_>,
    caps: &CapabilitySet,
    command: Vec<String>,
    cwd: Option<String>,
    timeout_secs: Option<f64>,
    env: Option<Vec<(String, String)>>,
) -> PyResult<ExecResult> {
```
For `confined_run` (Shape A) adopt the same pyo3 signature idiom. Signature per D-07:
```rust
#[pyfunction]
#[pyo3(signature = (exe, args, allow=None, profile=None, cwd=None, timeout_secs=None))]
pub fn confined_run(
    py: Python<'_>,
    exe: String,
    args: Vec<String>,
    allow: Option<Vec<String>>,   // explicit paths, or None
    profile: Option<String>,      // engine profile name, or None
    cwd: Option<String>,
    timeout_secs: Option<f64>,
) -> PyResult<ExecResult> {
```
For `confine` (Shape B — born-confined self-re-exec), signature:
```rust
#[pyfunction]
#[pyo3(signature = (profile=None, allow=None, caps=None))]
pub fn confine(
    profile: Option<String>,
    allow: Option<Vec<String>>,
    caps: Option<&CapabilitySet>,
) -> PyResult<()> {
```

**Command-building pattern** — spike `003-daemon-as-launcher/src/main.rs` lines 104-109:
```rust
let mut cmd = Command::new(&nono);
cmd.arg("run").arg("--profile").arg(profile.as_str()).arg("--allow-cwd");
if let Some(d) = &e.allow_dir {
    cmd.arg("--allow").arg(d);
}
cmd.arg("--").arg(&e.exe).args(&inner).current_dir(&workdir);
let status = cmd.status();
```
The `windows_confined_run.rs` `confined_run` function wraps exactly this `Command::new(nono_exe).arg("run")...` chain. Key flags: `--profile <name>`, `--allow <path>` (per-path loop), `--allow-cwd` (if cwd is the workspace), `--`, then `<exe>` and `<args>`.

**Locate nono.exe** — copy `which()` pattern from spike `main.rs` lines 27-38:
```rust
fn which(exe: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let cand = dir.join(exe);
            if cand.is_file() { Some(cand) } else { None }
        })
    })
}
```
In `windows_confined_run.rs` expose a `find_nono_exe()` helper that: (1) checks `NONO_EXE` env var, (2) falls back to `which("nono.exe")`, (3) returns `Err(PyRuntimeError)` if not found. Never silently degrade (CLAUDE.md § Fail Secure).

**Output capture** — `sandboxed_exec.rs` `parent_process` lines 348-393 spawns reader threads on pipe fds. For Windows `Command::new`, use `.stdout(Stdio::piped()).stderr(Stdio::piped())` and `child.wait_with_output()` or spawn background threads similarly. Maintain `py.detach(|| ...)` to release the GIL during blocking wait (same as `sandboxed_exec.rs` line 137).

**Error mapping** — `lib.rs` lines 26-42 `to_py_err`:
```rust
fn to_py_err(e: NonoError) -> PyErr {
    match &e {
        NonoError::PathNotFound(_) => PyFileNotFoundError::new_err(e.to_string()),
        NonoError::SandboxInit(_) | NonoError::UnsupportedPlatform(_) => {
            PyRuntimeError::new_err(e.to_string())
        }
        _ => PyRuntimeError::new_err(e.to_string()),
    }
}
```
`windows_confined_run.rs` does not call `to_py_err` (no `NonoError` at this layer — it wraps `nono.exe`). Use `PyRuntimeError::new_err(...)` directly for subprocess failures (nono.exe not found, spawn error, non-zero exit when it means config failure).

---

### `../nono-py/src/lib.rs` (modify — add `confine` + `confined_run` pyfunction registrations)

**Analog:** `C:\Users\OMack\nono-py\src\lib.rs` (self-extension)

**Module definition pattern** — lines 717-755: every new `#[pyfunction]` must be registered in `_nono_py()`. Copy the `wrap_pyfunction!` idiom:
```rust
#[pymodule]
fn _nono_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // ... existing ...
    m.add_function(wrap_pyfunction!(sandboxed_exec::sandboxed_exec, m)?)?;
    // ADD:
    #[cfg(windows)]
    m.add_function(wrap_pyfunction!(windows_confined_run::confined_run, m)?)?;
    #[cfg(windows)]
    m.add_function(wrap_pyfunction!(windows_confined_run::confine, m)?)?;
    Ok(())
}
```
The `#[cfg(windows)]` gate on the registration means the functions only appear in the Python module on Windows. Document this in docstrings: "Windows only; raises RuntimeError on other platforms."

**Sub-module declaration pattern** — lines 17-20:
```rust
mod policy;
mod proxy;
mod sandboxed_exec;
mod undo;
// ADD:
#[cfg(windows)]
mod windows_confined_run;
```

**`ExecResult` registration** — line 728:
```rust
m.add_class::<sandboxed_exec::ExecResult>()?;
```
If `ExecResult` is unified into `lib.rs`, update this. If keeping it in `sandboxed_exec.rs`, the Windows version needs its result type registered separately or share the Unix one by moving it to `lib.rs`.

---

### `../nono-py/src/policy.rs` (modify — Windows profile resolution for `confined_run`)

**Analog:** `C:\Users\OMack\nono-py\src\policy.rs` (self-extension)

The existing `resolve_proxy_config_impl` / `resolve_groups_impl` already resolve named groups from `policy.json`. The new need: a function that resolves an engine profile name to a CLI `--profile` argument string. This is trivial (the profile name is passed verbatim to `nono run --profile <name>`), but the policy module may need a `validate_profile_name(name: &str) -> NonoResult<()>` helper that checks the name exists in the embedded policy. Pattern:

**Embedded policy JSON access** — `policy.rs` line 13:
```rust
const EMBEDDED_POLICY_JSON: &str = include_str!("../data/policy.json");
```
The profiles section of `policy.json` is parsed by `nono-cli`; `nono-py`'s `policy.rs` only parses `groups`. If profile validation is needed, check `data/policy.json` §profiles for the `langchain-python` key. Validation can parse the embedded JSON and look for a `"profiles"` top-level key.

**`group_matches_platform` pattern for Windows** — lines 652-667:
```rust
fn current_platform() -> &'static str {
    if cfg!(target_os = "macos") { "macos" }
    else if cfg!(target_os = "linux") { "linux" }
    else { "unknown" }
}
fn group_matches_platform(group: &Group) -> bool {
    match &group.platform {
        Some(platform) => platform == current_platform(),
        None => true,
    }
}
```
If a `windows`-platform key is added to `policy.json` groups for the Windows wrapper, this helper will need `"windows"` added. Check `current_platform()` — currently returns `"unknown"` on Windows. Extend to return `"windows"` on Windows via `else if cfg!(target_os = "windows")`.

---

### `../nono-py/Cargo.toml` (modify — pin bump D-11)

**Analog:** `C:\Users\OMack\nono-py\Cargo.toml` (self)

**Current state** — lines 16-22:
```toml
[dependencies]
nono = "0.57.0"
nono-proxy = "0.57.0"
pyo3 = { version = "0.28", features = ["extension-module"] }
tokio = { version = "1", features = ["rt-multi-thread"] }
libc = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**Change required** (D-11):
```toml
nono = "0.62.2"       # bump from 0.57.0; match published fork crate version
nono-proxy = "0.62.2" # same bump
# pyo3 = "0.28" — KEEP (no major migration)
# libc = "0.2" — check if windows_confined_run.rs needs windows-sys or std::process only
```
Note: `libc = "0.2"` is used only by `sandboxed_exec.rs` (Unix fork/exec). If `windows_confined_run.rs` uses only `std::process::Command`, no new crate dependencies are needed. If `windows-sys` is needed for any Windows-specific handle operation, pin at `"0.59"` per CLAUDE.md §Technology Stack.

---

### `../nono-py/python/nono_py/__init__.py` (modify — export new functions)

**Analog:** `C:\Users\OMack\nono-py\python\nono_py\__init__.py` (self)

**Export pattern** — lines 30-63 + 65-99:
```python
from nono_py._nono_py import (
    AccessMode,
    # ... existing ...
    sandboxed_exec,
    # ADD (Windows-only, will be absent on non-Windows but must not crash import):
    # confined_run,
    # confine,
)
```
Because `confined_run` and `confine` are registered only on Windows, the import must be guarded. Pattern for platform-conditional exports:
```python
import sys as _sys
if _sys.platform == "win32":
    from nono_py._nono_py import confined_run, confine
    __all__ += ["confined_run", "confine"]
```
Alternatively, expose stub functions that raise `NotImplementedError` on non-Windows. Either way, `__all__` must list them only on Windows or with a clear docstring.

---

### `../nono-py/examples/15_langchain_confined.py` (create)

**Analog:** `C:\Users\OMack\nono-py\examples\08_sandboxed_exec.py`

**File-level pattern** — `08_sandboxed_exec.py` lines 1-19:
```python
#!/usr/bin/env python3
"""Sandboxed command execution.

Demonstrates sandboxed_exec(), which forks...
"""

import contextlib
import os
import sys
import tempfile

from nono_py import AccessMode, CapabilitySet, is_supported, sandboxed_exec
```
New file follows the same header docstring pattern documenting what Shape A and Shape B demonstrate. Imports:
```python
#!/usr/bin/env python3
"""LangChain agent confined via nono-py (Windows, real OS enforcement).

Demonstrates Shape B born-confined self-re-exec: confine() makes the agent
born-confined before any privileged handle is opened. Shape A (confined_run)
wraps a separate python process through the nono.exe broker.

Requires: langchain langchain_experimental (optional extra)
Platform: Windows (confined_run / confine are Windows-only)
"""

import os
import sys
import tempfile

# Optional import — graceful degradation if langchain not installed
try:
    from langchain_experimental.tools import PythonREPLTool
    HAS_LANGCHAIN = True
except ImportError:
    HAS_LANGCHAIN = False

from nono_py import AccessMode, CapabilitySet
# confined_run / confine are Windows-only; import guarded
if sys.platform == "win32":
    from nono_py import confined_run, confine
```

**Guard pattern** — `08_sandboxed_exec.py` line 44-46:
```python
def main() -> None:
    if not is_supported():
        print("Sandboxing not supported on this platform")
        sys.exit(1)
```
For `15_langchain_confined.py`, use:
```python
def main() -> None:
    if sys.platform != "win32":
        print("This example requires Windows (real OS enforcement via nono.exe broker)")
        sys.exit(1)
    if not HAS_LANGCHAIN:
        print("langchain / langchain_experimental not installed; pip install langchain langchain_experimental")
        sys.exit(1)
```

**Proof shape** (from D-08, `<specifics>` block):
```python
with tempfile.TemporaryDirectory() as ws:
    # Shape B: confine at startup (born-confined broker re-exec)
    confine(profile='langchain-python', allow=[ws])

    tool = PythonREPLTool()

    # Write OUTSIDE the workspace -> must be DENIED
    result = tool.run(f"open(r'C:\\outside_test.txt', 'w').write('bad')")
    assert "PermissionError" in result or "Access" in result, f"Expected denial, got: {result}"
    print(f"[PASS] Write outside workspace denied: {result[:80]}")

    # Write INSIDE the workspace -> must SUCCEED
    inside_path = os.path.join(ws, "ok.txt")
    result = tool.run(f"open(r'{inside_path}', 'w').write('ok')")
    assert os.path.exists(inside_path), f"Expected file to exist after write, got: {result}"
    print(f"[PASS] Write inside workspace allowed: {inside_path}")
```

**Working directory + tempfile** — `08_sandboxed_exec.py` lines 49-52:
```python
with tempfile.TemporaryDirectory() as workdir:
    caps = build_caps(workdir)
    print(f"Working directory: {workdir}")
```
The example uses `tempfile.TemporaryDirectory()` as context manager; the new file does the same, passing the dir path to `confine(allow=[ws])`.

---

### `../nono-py/tests/test_confined_run.py` (create)

**Analog:** `C:\Users\OMack\nono-py\tests\test_integration_exec.py` + `C:\Users\OMack\nono-py\tests\conftest.py`

**Import pattern** — `test_integration_exec.py` lines 1-16:
```python
"""Integration tests for end-to-end sandboxed execution flows."""

import json
from pathlib import Path

import pytest
from utils import add_system_paths

from nono_py import (
    AccessMode,
    CapabilitySet,
    QueryContext,
    SandboxState,
    load_policy,
    sandboxed_exec,
)
```
New file:
```python
"""Integration tests for confined_run / confine (Windows broker wrapper)."""

import os
import sys
from pathlib import Path

import pytest

# confined_run / confine only exist on Windows
if sys.platform == "win32":
    from nono_py import confined_run, confine

pytestmark = pytest.mark.skipif(
    sys.platform != "win32",
    reason="confined_run / confine are Windows-only (nono.exe broker wrapper)"
)
```

**Fixture pattern** — `conftest.py` lines 52-77:
```python
@pytest.fixture(scope="session")
def _sandboxed_exec_available(tmp_path_factory: pytest.TempPathFactory) -> bool:
    if not is_supported():
        return False
    try:
        tmp = tmp_path_factory.mktemp("exec_probe")
        caps = CapabilitySet()
        add_system_paths(caps)
        caps.allow_path(str(tmp), AccessMode.READ_WRITE)
        result = sandboxed_exec(caps, ["true"], cwd=str(tmp))
        return result.exit_code == 0
    except RuntimeError:
        return False

@pytest.fixture
def require_sandboxed_exec(_sandboxed_exec_available: bool) -> None:
    if not _sandboxed_exec_available:
        pytest.skip("sandboxed_exec unavailable in this environment")
```
Mirror this with a `nono_exe_available` fixture that probes `nono.exe` presence in PATH or `NONO_EXE`:
```python
@pytest.fixture(scope="session")
def nono_exe_path() -> str:
    """Return path to nono.exe or skip."""
    import shutil
    nono = os.environ.get("NONO_EXE") or shutil.which("nono.exe")
    if not nono:
        pytest.skip("nono.exe not found; set NONO_EXE or add to PATH")
    return nono
```

**Test class + marker pattern** — `test_integration_exec.py` lines 20-39:
```python
@pytest.mark.integration
@pytest.mark.usefixtures("require_sandboxed_exec")
class TestPolicyToExec:
    def test_policy_resolve_then_exec(self, tmp_path: Path) -> None:
        """Policy load -> resolve_groups -> sandboxed_exec succeeds end-to-end."""
        ...
        result = sandboxed_exec(caps, ["echo", "policy-ok"], cwd=str(tmp_path))
        assert result.exit_code == 0
        assert b"policy-ok" in result.stdout
```
New test class follows the same structure:
```python
@pytest.mark.integration
class TestConfinedRun:
    def test_write_inside_workspace_allowed(self, tmp_path: Path, nono_exe_path: str) -> None:
        """confined_run: write inside granted workspace succeeds."""
        inside = tmp_path / "ok.txt"
        result = confined_run(
            exe=sys.executable,
            args=["-c", f"open(r'{inside}', 'w').write('ok')"],
            allow=[str(tmp_path)],
            profile="langchain-python",
        )
        assert result.exit_code == 0
        assert inside.exists()

    def test_write_outside_workspace_denied(self, tmp_path: Path, nono_exe_path: str) -> None:
        """confined_run: write outside granted workspace is denied by OS enforcement."""
        outside = tmp_path.parent / "outside_test.txt"
        result = confined_run(
            exe=sys.executable,
            args=["-c", f"open(r'{outside}', 'w').write('bad')"],
            allow=[str(tmp_path)],
            profile="langchain-python",
        )
        # Confined child exits non-zero (PermissionError / Access denied)
        assert result.exit_code != 0
        assert not outside.exists(), "Outside file must NOT be created"
```

**`pytest.raises` for error conditions** — `test_sandboxed_exec.py` lines 121-124:
```python
def test_empty_command_raises(self, base_caps):
    with pytest.raises(ValueError, match="command must not be empty"):
        sandboxed_exec(base_caps, [])
```
Mirror for `confined_run`:
```python
def test_no_profile_or_allow_raises(self, nono_exe_path: str):
    with pytest.raises((ValueError, RuntimeError)):
        confined_run(exe=sys.executable, args=["-c", "pass"], allow=None, profile=None)
```

---

### `nono/proj/DESIGN-engine-abstraction.md` (create)

**Analog:** Structure mirrors existing design docs in `nono/proj/` (e.g., `DESIGN-library.md`, `DESIGN-supervisor.md`). No code excerpts apply; this is a Markdown doc.

**Pattern:** Prose document with:
- H1 heading `# Engine-Abstraction Contract`
- H2 sections for each E-level (E1 Isolation, E2 Confinement Scope, E3 Credential Isolation, E4 Rollback/Audit, E5 Pre-exec Interception)
- A table of invariants per E-level
- A `## Forward-Compat: zt-infra.org Integration` section (D-10) documenting E5 → `POST /actions` mapping
- A `## Implementation Notes per Platform` section noting Windows (Shape A/B), Linux/macOS (Sandbox::apply / fork+exec)

Source: `.planning/research/ARCHITECTURE.md` §"The Abstraction Boundary Contract" is the canonical content to promote into this file.

---

## Shared Patterns

### PyO3 `#[pyfunction]` Registration
**Source:** `C:\Users\OMack\nono-py\src\lib.rs` lines 640-699 (existing module-level functions) + lines 744-753 (`_nono_py` module body)
**Apply to:** `windows_confined_run.rs` (all exported functions), `lib.rs` (module registration)
```rust
/// Doc comment (mandatory for public API per CLAUDE.md § Comments)
#[pyfunction]
#[pyo3(signature = (param=default, ...))]
pub fn function_name(py: Python<'_>, ...) -> PyResult<ReturnType> {
    // body
}
// In _nono_py():
#[cfg(windows)]
m.add_function(wrap_pyfunction!(windows_confined_run::function_name, m)?)?;
```

### Error Mapping (`to_py_err`)
**Source:** `C:\Users\OMack\nono-py\src\lib.rs` lines 26-42
**Apply to:** All new Rust functions that return `PyResult<_>`
```rust
fn to_py_err(e: NonoError) -> PyErr {
    match &e {
        NonoError::PathNotFound(_) => PyFileNotFoundError::new_err(e.to_string()),
        NonoError::SandboxInit(_) | NonoError::UnsupportedPlatform(_) => {
            PyRuntimeError::new_err(e.to_string())
        }
        _ => PyRuntimeError::new_err(e.to_string()),
    }
}
```
For `windows_confined_run.rs`, subprocess errors (not `NonoError`) map directly:
```rust
fn spawn_err(msg: impl Into<String>) -> PyErr {
    PyRuntimeError::new_err(msg.into())
}
```

### GIL Release During Blocking Subprocess Wait
**Source:** `C:\Users\OMack\nono-py\src\sandboxed_exec.rs` line 137
**Apply to:** `windows_confined_run.rs` `confined_run` blocking wait
```rust
py.detach(|| do_spawn_and_wait(&ctx))
```
Always release the GIL before blocking on child process completion to avoid deadlocking other Python threads.

### Platform-Conditional `#[cfg]` Gates
**Source:** `C:\Users\OMack\nono-py\src\sandboxed_exec.rs` lines 115-127 (inline `#[cfg(target_os = "linux")]`); `C:\Users\OMack\nono-py\src\lib.rs` lines 17-20 (mod declarations)
**Apply to:** All cross-platform forking points in `lib.rs` and the new Windows file
```rust
// lib.rs mod declarations:
mod sandboxed_exec;      // Unix-only; sandboxed_exec.rs uses libc
#[cfg(windows)]
mod windows_confined_run; // Windows-only; uses std::process::Command

// Registration in _nono_py():
#[cfg(not(windows))]
m.add_function(wrap_pyfunction!(sandboxed_exec::sandboxed_exec, m)?)?;
#[cfg(windows)]
m.add_function(wrap_pyfunction!(windows_confined_run::confined_run, m)?)?;
#[cfg(windows)]
m.add_function(wrap_pyfunction!(windows_confined_run::confine, m)?)?;
```
**CRITICAL:** Per CLAUDE.md § Cross-target clippy, any commit touching cfg-gated code MUST verify via `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`. If cross-toolchain absent, mark verifications PARTIAL and defer to live CI.

### Embedded Policy JSON Access
**Source:** `C:\Users\OMack\nono-py\src\policy.rs` line 13; `C:\Users\OMack\nono-py\src\lib.rs` lines 679-683
**Apply to:** Any profile-name validation in `windows_confined_run.rs` or `policy.rs`
```rust
const EMBEDDED_POLICY_JSON: &str = include_str!("../data/policy.json");
// or via:
pub fn load_embedded_policy() -> NonoResult<Policy> { load_policy(EMBEDDED_POLICY_JSON) }
```

### Test Skip Guards (platform + availability)
**Source:** `C:\Users\OMack\nono-py\tests\conftest.py` lines 52-77; `C:\Users\OMack\nono-py\tests\test_broker_ffi_mapping.py` lines 54-58
**Apply to:** `test_confined_run.py` all test classes
```python
# File-level skip:
pytestmark = pytest.mark.skipif(sys.platform != "win32", reason="Windows-only")

# Per-test skip decorator:
@pytest.mark.skipif(sys.platform != "win32", reason="confined_run is Windows-only")

# Fixture-based skip (preferred for nono.exe availability):
@pytest.fixture(scope="session")
def nono_exe_path(): ...  # calls pytest.skip() if not found
```

### Example File Structure
**Source:** `C:\Users\OMack\nono-py\examples\08_sandboxed_exec.py` full file
**Apply to:** `examples/15_langchain_confined.py`
- Shebang line `#!/usr/bin/env python3`
- Module docstring explaining what is demonstrated
- Helper `build_caps()` or equivalent setup function
- `main()` function with platform guard at top
- `if __name__ == "__main__": main()` at bottom
- Uses `tempfile.TemporaryDirectory()` as context manager for workspace
- Prints `[PASS]` / `[FAIL]` indicators for each proof step
- All proof assertions are explicit (`assert` or printed verdict) — not silent

### `__init__.py` Export Pattern
**Source:** `C:\Users\OMack\nono-py\python\nono_py\__init__.py` lines 29-101
**Apply to:** new `confined_run` / `confine` exports
```python
# Existing pattern — unconditional flat import from C extension:
from nono_py._nono_py import (sandboxed_exec, ...)

# New Windows-conditional pattern:
import sys as _sys
if _sys.platform == "win32":
    from nono_py._nono_py import confined_run, confine
# Update __all__ accordingly
```

---

## No Analog Found

All files in this phase have analogs. However, the following areas have partial analog coverage:

| File / Area | Role | Data Flow | Gap |
|---|---|---|---|
| `confine()` Shape B self-re-exec logic | service | event-driven | No prior self-re-exec function in nono-py; closest is spike-003 `main.rs` which does `Command::new(nono).arg("run")...` for external processes, not self. The born-confined re-exec is new; planner should use the spike's `Command` chain + CONTEXT.md D-03 guidance. |
| Windows profile-name validation | utility | transform | No existing profile resolver in nono-py (policy.rs resolves groups, not profile names). Profile names are passed verbatim to `nono run --profile`; validation is optional (fail-secure: nono.exe rejects unknown profiles at launch). |

---

## Metadata

**Analog search scope:** `C:\Users\OMack\nono-py\src\`, `C:\Users\OMack\nono-py\tests\`, `C:\Users\OMack\nono-py\examples\`, `C:\Users\OMack\Nono\crates\nono-cli\src\exec_strategy_windows\`, `C:\Users\OMack\Nono\.planning\spikes\003-daemon-as-launcher\src\`, `C:\Users\OMack\Nono\crates\nono\src\sandbox\windows.rs`
**Files scanned:** 22 source files read; 4 additional files via Grep/Glob
**Pattern extraction date:** 2026-06-14
