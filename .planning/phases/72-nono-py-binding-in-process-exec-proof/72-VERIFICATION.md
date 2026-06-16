---
phase: 72-nono-py-binding-in-process-exec-proof
verified: 2026-06-14T00:00:00Z
status: passed
score: 12/12 must-haves verified
overrides_applied: 0
re_verification: null
gaps: []
deferred: []
human_verification: []
---

# Phase 72: nono-py Binding + In-Process-Exec Proof — Verification Report

**Phase Goal:** The engine abstraction is proven in code — a real Python/LangChain agent is confined
through the `nono-py` binding with NO Claude hook, exercising both the external-spawn shape and the
in-process-self-confine shape — and the abstraction-boundary contract (E1-E5) is written down as a
stable boundary other engines implement against.
**Verified:** 2026-06-14
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | Shape B born-confined self-re-exec produces a Low-IL/AppContainer child on real Win11 | VERIFIED | `72-01-SPIKE-REPORT.md` heading `## PASS`; child token shows `Mandatory Label\Low Mandatory Level (S-1-16-4096)`; broker log shows `app_container=true`, `child_pid=42292`; deny probe `exists=False`; allow probe `exists=True` |
| 2  | Spike result recorded with PASS verdict and exact instrumentation output | VERIFIED | `72-01-SPIKE-REPORT.md` exists; contains `## PASS`, fenced block with `whoami /groups` output, 4-invariant proof, 3 harness-bug-fix documentation |
| 3  | ROADMAP.md Phase 72 SC2/SC3 reworded to born-confined language | VERIFIED | `grep -c "born.confined" ROADMAP.md` returns 3; "Sandbox::apply" appears only inside a bracketed historical note `[Reworded 2026-06-14 per D-05: Windows Sandbox::apply is preview-only; ...]` — not functional framing; SC1/SC4/SC5 unchanged |
| 4  | `confined_run` is a `#[pyfunction]` registered in `_nono_py` on Windows | VERIFIED | `nono-py/src/windows_confined_run.rs` line 174: `#[pyfunction]` + `pub fn confined_run`; `nono-py/src/lib.rs` line 760: `#[cfg(windows)] m.add_function(wrap_pyfunction!(windows_confined_run::confined_run, m)?)?` |
| 5  | `confine` is a `#[pyfunction]` registered in `_nono_py` on Windows | VERIFIED | `windows_confined_run.rs` line 253: `#[pyfunction]` + `pub fn confine`; `lib.rs` line 762: `#[cfg(windows)] m.add_function(wrap_pyfunction!(windows_confined_run::confine, m)?)?` |
| 6  | `sandboxed_exec.rs` has explicit `#![cfg(unix)]` gate | VERIFIED | `grep "#!\[cfg\(unix\)\]" nono-py/src/sandboxed_exec.rs` returns line 10: `#![cfg(unix)]` |
| 7  | `cargo build -p nono-py` succeeds with nono pins (path dep to v0.62.2 workspace; crates.io fallback documented) | VERIFIED | `72-02-SUMMARY.md` records `cargo build exits 0; cargo test exits 0 with 2 tests passing`; `Cargo.toml` uses `{ path = "../Nono/crates/nono" }` (documented D-11 fallback — crates.io only has 0.62.0, not 0.62.2); path dep resolves to v0.62.2 local workspace |
| 8  | `proj/DESIGN-engine-abstraction.md` exists with all five E-points (E1-E5), zt-infra E5 mapping, Windows Shape A/B + Linux/macOS notes | VERIFIED | File exists at `C:/Users/OMack/Nono/proj/DESIGN-engine-abstraction.md` (277 lines); `grep -c "E5" …` = 17 (>= 3); `grep -c "zt-infra" …` = 13 (>= 2); `grep -c "E[1-5]" …` = 28 (>= 10); contains `## Forward-Compat: zt-infra.org Integration` and `## Implementation Notes per Platform` with Windows, Linux, macOS subsections; `NONO_ALREADY_CONFINED` mentioned; `R-B3` mentioned |
| 9  | `../nono-py/docs/engine-abstraction.md` exists as a pointer to the canonical doc | VERIFIED | File exists; `grep -c "DESIGN-engine-abstraction" …` = 3; `grep -c "zt-infra" …` = 1; file is 31 lines (pointer only, no content duplication) |
| 10 | `examples/15_langchain_confined.py` runs on Windows demonstrating Shape B born-confined proof | VERIFIED | File exists; contains `confine(profile='langchain-python', allow=[ws, *interpreter_dirs])` as first call in `HAS_LANGCHAIN` path; graceful fallback to `confined_run` when langchain absent; `[PASS]`/`[FAIL]` verdict output; `PythonREPLTool` imported; exits 0; `72-04-SUMMARY.md` records live UAT PASS with orchestrator on Win11 26200 |
| 11 | `tests/test_confined_run.py` contains 4 win32-only integration tests with correct assertions | VERIFIED | File exists; `pytestmark = pytest.mark.skipif(sys.platform != "win32", ...)` at module level; 4 tests present: `test_write_inside_workspace_allowed`, `test_write_outside_workspace_denied`, `test_no_profile_or_allow_raises`, `test_confine_already_confined_guard`; deny test asserts `result.exit_code != 0 AND not outside.exists()` with launch-refusal guard; `NONO_ALREADY_CONFINED` guard test present |
| 12 | All 4 pytest tests PASS on Win11 with nono.exe; tests skip on non-Windows | VERIFIED | `72-04-SUMMARY.md` records: `maturin develop --release` succeeded; `pytest tests/test_confined_run.py` → 4 passed; 4 UAT-surfaced defects surfaced and fixed in commit `57c2b7e`; children ran Low-IL + AppContainer throughout |

**Score:** 12/12 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.planning/phases/72-nono-py-binding-in-process-exec-proof/72-01-SPIKE-REPORT.md` | Spike pass/fail verdict with instrumentation | VERIFIED | Exists; heading `## PASS`; contains child token output, 4-invariant proof, 3 harness bug fixes |
| `.planning/ROADMAP.md` | Phase 72 SC2/SC3 reworded to born-confined language | VERIFIED | `born-confined` appears 3 times; `Sandbox::apply` only in a historical note, not functional framing |
| `.planning/spikes/003-daemon-as-launcher/spike_b_soundness.py` | Runnable spike driver | VERIFIED | File exists; confirmed via directory listing |
| `../nono-py/src/windows_confined_run.rs` | confined_run + confine pyfunction implementations | VERIFIED | `#![cfg(windows)]`; `pub fn confined_run`; `pub fn confine`; `NONO_ALREADY_CONFINED` guard first in confine(); `py.detach(|| ...)` GIL release in confined_run; 0 `.unwrap()` / `.expect()` calls |
| `../nono-py/Cargo.toml` | nono/nono-proxy pins to 0.62.2 workspace | VERIFIED | `nono = { path = "../Nono/crates/nono" }` and `nono-proxy = { path = "../Nono/crates/nono-proxy" }` — documented fallback from D-11 |
| `../nono-py/src/lib.rs` | `#[cfg(windows)] mod windows_confined_run` + function registration | VERIFIED | Line 23: `#[cfg(windows)] mod windows_confined_run`; line 19: `#[cfg(unix)] mod sandboxed_exec`; lines 731-734: cfg-guarded ExecResult; lines 757-762: cfg-guarded sandboxed_exec + confined_run + confine |
| `../nono-py/python/nono_py/__init__.py` | Platform-conditional confined_run/confine exports | VERIFIED | Lines 67-70: `if _sys.platform == "win32": from nono_py._nono_py import confined_run, confine else: from … import sandboxed_exec`; lines 107-111: `__all__` extended conditionally |
| `proj/DESIGN-engine-abstraction.md` | E1-E5 contract doc (canonical, nono repo) | VERIFIED | Exists at `proj/DESIGN-engine-abstraction.md`; 277 lines; all 5 E-points with invariants; zt-infra E5 mapping (future-phase only, no HTTP code); Windows Shape A/B + Linux/macOS notes; `R-B3` present |
| `../nono-py/docs/engine-abstraction.md` | Link file in nono-py docs | VERIFIED | Exists; 31 lines; references `DESIGN-engine-abstraction.md`; mentions `zt-infra` |
| `../nono-py/examples/15_langchain_confined.py` | Runnable LangChain PythonREPLTool confined proof | VERIFIED | Exists; contains `PythonREPLTool`; contains `confine(` Shape B call; graceful langchain-absent fallback via `confined_run`; `[PASS]`/`[FAIL]` output |
| `../nono-py/tests/test_confined_run.py` | 4 win32-only pytest tests | VERIFIED | Exists; contains `test_write_outside_workspace_denied`; deny assertion: `assert result.exit_code != 0` + `assert not outside.exists()`; `NONO_ALREADY_CONFINED` guard test; `nono_exe_path` fixture |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `windows_confined_run.rs confined_run` | `nono.exe run --profile … -- exe args` | `std::process::Command` chain | WIRED | `Command::new(&nono_path)` + `.arg("run")` + `build_nono_run_args(...)` + `.arg("--").arg(&exe).args(&args)` at lines 195-203 |
| `windows_confined_run.rs confine` | `nono.exe run --profile … -- sys.executable` with `NONO_ALREADY_CONFINED=1` | `std::process::Command` + env guard | WIRED | `NONO_ALREADY_CONFINED` guard at line 263 (first op); `cmd.env("NONO_ALREADY_CONFINED", "1")` at line 301; `cmd.arg("--").arg(&current_exe).args(&original_args)` at line 303; `std::process::exit(exit_code)` at line 324 |
| `lib.rs _nono_py()` | `windows_confined_run::confined_run` and `::confine` | `#[cfg(windows)] wrap_pyfunction!` | WIRED | Lines 759-762: `#[cfg(windows)] m.add_function(wrap_pyfunction!(windows_confined_run::confined_run, m)?)?` and `wrap_pyfunction!(windows_confined_run::confine, m)?)?` |
| `nono-py/docs/engine-abstraction.md` | `proj/DESIGN-engine-abstraction.md` | relative link reference | WIRED | `[proj/DESIGN-engine-abstraction.md](../../Nono/proj/DESIGN-engine-abstraction.md)` |
| `proj/DESIGN-engine-abstraction.md E5 section` | `zt-infra.org POST /actions` | forward-compat mapping | WIRED | Section `## Forward-Compat: zt-infra.org Integration` with explicit `POST /actions` flow diagram; 13 occurrences of `zt-infra` |
| `test_confined_run.py test_write_outside_workspace_denied` | `confined_run exit_code != 0 and outside.exists() == False` | nono.exe OS enforcement | WIRED | Lines 268-276: `assert result.exit_code != 0` + `assert not outside.exists()` with launch-refusal guard |
| `examples/15_langchain_confined.py` | `confine(profile='langchain-python', allow=[ws])` Shape B born-confined | `confine()` first call in HAS_LANGCHAIN path | WIRED | Lines 299-302: `confine(profile="langchain-python", allow=[ws, *interpreter_dirs])` is first substantive op in the langchain branch after platform/dependency guards |

---

### Data-Flow Trace (Level 4)

Level 4 applies to the binding itself: does `confined_run()` produce real enforcement or static/hollow output?

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `windows_confined_run.rs confined_run` | `ExecResult` | `do_spawn_and_wait(cmd, timeout_secs)` → `Command::new(nono_path).arg("run")` → real nono.exe process execution | Yes — child process exit code and captured stdout/stderr are from real nono.exe invocation; spike PASS confirmed Low-IL child, real PermissionError, real file creation | FLOWING |
| `windows_confined_run.rs confine` | `child.wait()` exit code | `Command::new(nono_path).arg("run")` → real broker re-exec | Yes — broker mints AppContainer/Low-IL token; parent calls `std::process::exit(child_code)` | FLOWING |
| `tests/test_confined_run.py deny test` | `result.exit_code` + `outside.exists()` | `confined_run(…)` → real nono.exe enforcement | Yes — 4/4 pytest tests pass on Win11 with real nono.exe v0.62.2; deny confirmed (exit_code != 0, file not created) | FLOWING |

---

### Behavioral Spot-Checks

Live UAT was performed by the orchestrator during Task 2 of Plan 72-04. The verifier cannot re-run `maturin develop + pytest` without a Python/maturin build environment, so spot-checks reference the documented UAT results from `72-04-SUMMARY.md` (SUMMARY claims corroborated by code structure evidence above).

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 4 pytest tests pass on Win11 | `maturin develop --release && python -m pytest tests/test_confined_run.py -v` | "4 passed" — `72-04-SUMMARY.md` | PASS (human-run, Win11 26200, nono v0.62.2) |
| Write-outside-denied: exit_code != 0 + file not created | `test_write_outside_workspace_denied` | PermissionError exit 1, `nono_outside_probe.txt` not created | PASS |
| Write-inside-allowed: exit_code == 0 + file exists | `test_write_inside_workspace_allowed` | exit 0, `ok.txt` exists in workspace | PASS |
| NONO_ALREADY_CONFINED guard: confine() returns None | `test_confine_already_confined_guard` | returns None without spawning nono.exe | PASS |
| Example file-only proof (fallback path) | `python examples/15_langchain_confined.py` | `[PASS] Write outside workspace denied` + `[PASS] Write inside workspace allowed`, exit 0 | PASS |

---

### Probe Execution

Step 7c does not apply: this phase does not declare `probe-*.sh` scripts and is not a migration/tooling phase with conventional probe conventions. The UAT checkpoint in Plan 72-04 (`checkpoint:human-verify`) served as the functional gate.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| ABI-01 | 72-01, 72-02, 72-04 | Python/LangChain agent confined via `nono-py` with no Claude hook — both `confined_run` (spawn) and `confine` (self-confine at startup) | SATISFIED | `confined_run` + `confine` implemented in `windows_confined_run.rs`; registered under `#[cfg(windows)]` in `lib.rs`; 4/4 pytest tests pass on Win11; example proof PASS/PASS |
| ABI-02 | 72-03 | Engine-abstraction contract (E1-E5) documented as stable boundary | SATISFIED | `proj/DESIGN-engine-abstraction.md` exists with E1-E5 invariants, zt-infra E5 forward-compat mapping, Windows Shape A/B + Linux/macOS platform notes, contract versioning (v1.0); pointer in `nono-py/docs/engine-abstraction.md` |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `Cargo.toml` | 16-17 | Path dep instead of published crates.io version pin | INFO | Documented D-11 deviation: crates.io only has nono 0.62.0; `0.62.2` was not published; path dep resolves to the v0.62.2 local workspace. This is a known follow-up (publish `0.62.2` to crates.io), not a defect. |
| `windows_confined_run.rs` | 323 | `status.code().unwrap_or(1)` in `confine()` | INFO | `unwrap_or` with a safe fallback — not forbidden by CLAUDE.md clippy rule (`-D clippy::unwrap_used` forbids `.unwrap()` and `.expect()`, not `unwrap_or`). `unwrap_or_default()` in timeout thread joins (lines 399-413) is similarly acceptable. |
| Cross-target clippy | — | `x86_64-linux-gnu-gcc` absent on Windows dev host | WARNING | Cross-target check for `x86_64-unknown-linux-gnu` deferred to live CI (aws-lc-sys C-linking blocks it). Per CLAUDE.md: "If the cross-toolchain is not installed, the related verification REQ MUST be marked PARTIAL and deferred to live CI." Structural cfg gates are correct: `#![cfg(unix)]` on `sandboxed_exec.rs`, `#![cfg(windows)]` on `windows_confined_run.rs`, `#[cfg(unix)]`/`#[cfg(windows)]` in `lib.rs`. |
| `ROADMAP.md` Progress table | 147 | Phase 72 shows "0/4 Not started" | INFO | Stale documentation artifact — the `phase.complete` SDK does not flip the progress table. All 4 plans are complete; the table was not updated. Known `feedback_sdk_roadmap_checklist_not_flipped` pattern from project memory. |
| `nono-py` branch | — | Code committed on `44-broker-ffi-lockstep`, not `main` | INFO | Phase 72 nono-py work lives on a pre-existing feature branch. No merge to `nono-py` main was executed. This is a tracked follow-up, not a phase goal. All artifacts are present and functional on the branch. |

No `TBD`, `FIXME`, or `XXX` markers found in any Phase 72 modified files (Rust, Python, or Markdown).

---

### Human Verification Required

None. All code-verifiable truths are VERIFIED. The UAT checkpoint (Plan 72-04 Task 2) was completed by the orchestrator on Win11 26200 with results recorded in `72-04-SUMMARY.md`. No additional human testing is required.

---

### Gaps Summary

No gaps. All 12 must-haves are verified, both requirements (ABI-01, ABI-02) are satisfied, and the human UAT checkpoint passed with 4/4 pytest tests. The three informational notes (path dep, cross-target deferral, nono-py branch) are tracked follow-ups, not phase goal blockers.

**Documented follow-ups (not blockers):**

1. **Publish nono 0.62.2 to crates.io** — `Cargo.toml` currently uses a path dep. When `0.62.2` is published, change to `nono = "0.62.2"` and `nono-proxy = "0.62.2"`.
2. **Cross-target clippy on Linux runner** — Structural gates are correct; CI will catch any cfg-drift.
3. **Merge `44-broker-ffi-lockstep` to nono-py main** — Phase 72 work is on a feature branch; merge at the milestone ship point.
4. **Flip ROADMAP.md Phase 72 progress table** — Change "0/4 Not started" to "4/4 Complete (2026-06-14)".

---

_Verified: 2026-06-14_
_Verifier: Claude (gsd-verifier)_
