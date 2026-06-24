---
phase: 92
phase_name: "runtime-capabilityset-mutation-audit-wiring"
project: "nono - Windows Parity & Quality"
generated: "2026-06-22"
counts:
  decisions: 8
  lessons: 6
  patterns: 8
  surprises: 5
missing_artifacts:
  - "UAT.md"
---

# Phase 92 Learnings: runtime-capabilityset-mutation-audit-wiring

## Decisions

### Split-architecture mutation+audit locus (nono-py applies, nono-cli audits)
nono-py runs the Phase 91 verifier and appends override `--allow` + `--override-audit` flags; nono-cli emits the `PolicyOverrideApplied` event and gates the spawn.

**Rationale:** The `SecurityEventLayer` HMAC chain lives only in nono-cli; relocating it into the core crate to emit from nono-py would violate the policy-free-core invariant. Trusting launcher-supplied audit metadata is the same trust model as the existing `--allow` flags, and the OS confinement layer still applies to the expanded set.
**Source:** 92-CONTEXT.md (D-01), 92-03-SUMMARY.md

### SecurityEventLayer.inner → Arc<Mutex<...>> + derive(Clone)
Changed the layer's inner field to `Arc<Mutex<SecurityEventLayerInner>>` and derived `Clone`, rather than wrapping the layer in `Arc<SecurityEventLayer>` inside the OnceLock.

**Rationale:** `tracing-subscriber 0.3.23` does NOT implement `Layer<S>` for `Arc<T>` (added in a later release). `Arc<Mutex<...>>` makes `SecurityEventLayer` itself O(1)-`Clone`, so the `SECURITY_LAYER` copy and the tracing-registry copy share the same chain state without changing `init_tracing_with_security`'s signature.
**Source:** 92-03-SUMMARY.md

### DECODE-ONCE at the launch_runtime boundary
The base64url-JSON decode of `--override-audit` happens in `prepare_run_launch_plan`; `execute_sandboxed` receives the typed `Option<OverrideAuditMeta>`.

**Rationale:** Keeps all string manipulation away from the security-critical spawn boundary; the AUD-04 gate operates on a type-checked struct, not raw bytes.
**Source:** 92-03-SUMMARY.md, 92-PATTERNS.md

### Strategy A stub injection over a Popen mock for the Dark Factory gate
The OVERRIDE-01 gate and pytest capture argv via a `NONO_EXE`-injected `.bat` stub, not by mocking `subprocess.Popen`.

**Rationale:** The args are built and spawned by Rust's `std::process::Command` inside the compiled extension — it is invisible to Python-level subprocess mocking (T-92-VACUOUS-MOCK). A stub that the real Rust spawn executes is the only way to exercise the real `verify_override` → `confined_run` path.
**Source:** 92-04-SUMMARY.md

### PolicyOverrideApplied is a pure data carrier; lifecycle events are Warning-level
The core-crate variant carries only data (jti, kms_key_id, zt_audit_hash, granted_path_hashes, expires_at); all 5 `SecurityEventType::PolicyOverride*` variants map to `TelemetrySeverity::Warning`.

**Rationale:** Policy-free-core invariant (no policy logic in `crates/nono`). Override lifecycle events are authorization events, not denial-only, so Warning is the correct band.
**Source:** 92-01-SUMMARY.md

### Audit fields read from the verified OverrideGrant, never by re-parsing the token
`zt_audit_hash` is sourced from `token.current_hash` at grant construction and exposed via a `#[getter]`; downstream emission reads the grant.

**Rationale:** Closes the TOCTOU verify→apply gap (Phase 91 D-02). Adding a `#[getter]` does not break the `frozen` pyclass immutability.
**Source:** 92-02-SUMMARY.md, 92-VERIFICATION.md (Truth 3)

### #[must_use = "msg"] instead of bare #[must_use]
`emit_override_event` uses a message-bearing `#[must_use]`.

**Rationale:** `Result` is already `#[must_use]`; a bare attribute triggers `clippy::double_must_use` → error under `-D warnings`. The message also documents the AUD-04 fail-closed contract for callers.
**Source:** 92-03-SUMMARY.md

### VFY-01 left PARTIAL [BLOCKING-93] by design, not as a gap
Phase 92 wires the offline arm and a documented composition seam in both `confined_run` and `confine`; the live `POST /actions` AND-gate is deferred to Phase 93.

**Rationale:** Locked decision D-03 — offline-pass is necessary but not sufficient. Mirrors the Phase 91 VFY-03 PARTIAL split already sanctioned.
**Source:** 92-CONTEXT.md (D-03), 92-02/92-04-SUMMARY.md, 92-VERIFICATION.md (Truth 11)

---

## Lessons

### `Mutex::new(HashMap::new())` cannot initialize a `static` (E0015)
A `static PROBE_CACHE: Mutex<HashMap<...>> = Mutex::new(HashMap::new())` fails because `HashMap::new()` is not `const`.

**Context:** Fixed by `LazyLock<Mutex<HashMap<...>>>` (stable since Rust 1.80; workspace MSRV is 1.82). Reach for `LazyLock` for any non-const process-lifetime static.
**Source:** 92-02-SUMMARY.md

### `Path::is_absolute()` returns false for Unix-convention paths on Windows
`/tmp/project` is not "absolute" on Windows (no drive letter), so a plan that specified `is_absolute()` for path validation rejected valid CAF v0.1 token paths.

**Context:** Override tokens use Unix-style paths. Accept a leading `/` alongside `is_absolute()` — the same reconciliation already made in `partition_scope()` (override.rs). When a plan's literal API contradicts the required cross-platform behavior, check for an established in-repo precedent before improvising.
**Source:** 92-02-SUMMARY.md

### `base64` was a transitive-only dep of nono-cli
`use base64::Engine as _` failed with E0432 because base64 reached nono-cli only transitively.

**Context:** Promote it to a direct `Cargo.toml` dep (0.22.1 was already in the lockfile, so no new package install / no supply-chain concern). A crate being in `Cargo.lock` does not make its API importable from your crate.
**Source:** 92-03-SUMMARY.md

### Multi-binary crates produce false `dead_code` warnings
`emit_override_event` is used by `execution_runtime.rs` (compiled into the `nono` binary) and tests, but `nono-agentd` does not compile that path, so the daemon binary build flags the method as dead code.

**Context:** Not real dead code — an artifact of a multi-binary crate. Annotate with `#[allow(dead_code)]` plus an explanatory comment, rather than deleting a method that production uses.
**Source:** 92-03-SUMMARY.md

### The D-02 version-probe runs before arg-building — stubs must answer `--version` first
`probe_override_support` calls `nono.exe --version` before `append_override_args`. A stub that doesn't handle `--version` returns empty output → the probe caches `false` and raises `NonoOverrideError` before any args are built.

**Context:** Every test fixture, inline stub, and gate stub that stands in for nono.exe must emit a `nono 3.2.0`-shaped `--version` response and exit 0 first. This single ordering fact caused three separate fixes across pytest fixtures and the gate.
**Source:** 92-04-SUMMARY.md

### Phase 91 symbols were compiled but not re-exported (latent ImportError)
`NonoOverrideError`, `OverrideGrant`, and `verify_override` existed in the compiled `_nono_py.pyd` but were absent from `python/nono_py/__init__.py`, so `from nono_py import verify_override` raised ImportError.

**Context:** Surfaced only when Phase 92 tests imported them. PyO3 `#[pyfunction]`/`#[pyclass]` registration in lib.rs makes a symbol reachable via the extension module, but the Python package's `__init__.py` must still re-export it (import block + `__all__`) to be a public surface.
**Source:** 92-04-SUMMARY.md

---

## Patterns

### LazyLock<Mutex<HashMap>> process-lifetime cache keyed by PathBuf
A probe/result cache that must live for the whole process and start empty.

**When to use:** Any non-const static cache (here: per-nono.exe-path override-support probe results). Keying by a unique path also isolates parallel tests that inject distinct stub paths.
**Source:** 92-02-SUMMARY.md

### Component-wise path sanitization (`Path::components()` / `Component::ParentDir`)
Reject `..` and non-absolute paths by iterating `Path::components()`, never by string `starts_with`/`contains`.

**When to use:** Any path crossing a security boundary (MUT-04, CLAUDE.md §Path Handling). String operations on paths are a documented footgun.
**Source:** 92-02-SUMMARY.md, 92-VERIFICATION.md (Truth 4)

### DECODE-ONCE at the runtime boundary
Decode/parse untrusted wire input once at the launch-plan boundary into a typed struct; the security-critical spawn gate sees only the typed value.

**When to use:** Whenever an encoded flag (base64/JSON) must be consumed at a trust boundary — eliminates re-parsing and string handling at the spawn site.
**Source:** 92-03-SUMMARY.md

### AUD-04 fail-closed pre-spawn audit gate
A `#[must_use]` emit method that returns `Err` on a poisoned mutex, plus a gate placed after proxy startup and before sandbox apply that returns `Err` (aborting the spawn) if emission fails.

**When to use:** Any "must-audit-before-acting" invariant where a missing audit record would be silent privilege escalation. The mutex-poison-as-Err mapping turns a panic source into a deny.
**Source:** 92-03-SUMMARY.md

### Dark Factory stub injection to capture real cross-language argv (Strategy A)
Inject a `.bat` (or script) stub via an env var (`NONO_EXE`) so the real Rust `std::process::Command` spawn executes it and records the argv it was given.

**When to use:** Verifying argument construction that happens inside compiled native code invisible to the host language's mocking. Exercises the real verify+build path instead of a vacuous mock.
**Source:** 92-04-SUMMARY.md

### Dark Factory gate contract (verdict-object, never exit/Persist-Verdict)
The gate returns a verdict object and provides `Test-Precondition` + `Invoke-Gate`; it never calls `exit` or `Persist-Verdict` (verify-dark.ps1 owns those), and short-circuits to `SKIP_HOST_UNAVAILABLE` on infra gaps.

**When to use:** Every `scripts/gates/*.ps1`. Mirror the closest structural-twin gate (here: `telemetry-event-emit.ps1`). Invoke via `-File`/direct, never `pwsh -Command "<bare path>"`.
**Source:** 92-04-SUMMARY.md

### Dependency-free token minting (openssl + pure-Python low-S normalization)
Mint test ECDSA P-256 tokens with `openssl dgst -sha256 -sign` and normalize to low-S in pure Python — no external crypto package.

**When to use:** Test/gate token minting where adding a crypto dependency to the test environment is undesirable; reuses the committed Phase 91 test keypair (never a production trust root).
**Source:** 92-04-SUMMARY.md

### Bilateral fail-closed capability gate across a process/repo boundary
Both sides enforce: nono-cli treats override `--allow` paths without a committed audit event as fatal; nono-py refuses to launch unless the target nono.exe advertises override support (version probe).

**When to use:** When a security invariant spans two independently-versioned components — neither side alone is sufficient to prevent silent degradation (a too-old CLI silently accepting extra `--allow` flags).
**Source:** 92-CONTEXT.md (D-02), 92-02/92-03-SUMMARY.md

---

## Surprises

### tracing-subscriber 0.3.23 doesn't implement `Layer<S>` for `Arc<T>`
The obvious `OnceLock<Arc<SecurityEventLayer>>` approach was unusable; the layer trait isn't implemented for `Arc` at the pinned version (it was added later).

**Impact:** Forced an inner-field refactor to `Arc<Mutex<...>>` + `derive(Clone)` to share chain state — a larger change than anticipated, though it left the layer cheaply cloneable.
**Source:** 92-03-SUMMARY.md

### One probe-ordering fact rippled into three separate test-harness fixes
Because `probe_override_support` calls `--version` before arg-building, every stub that lacked a `--version` handler failed identically (cached `false` → `NonoOverrideError`).

**Impact:** Three deviations in Plan 04 (the `nono_stub` fixture, a test with its own inline stub, and the gate's inline stub) all traced to the same root cause — a reminder that probe-before-act ordering is a cross-cutting fixture contract.
**Source:** 92-04-SUMMARY.md

### A latent Phase 91 packaging gap surfaced only under Phase 92's imports
The Phase 91 override symbols were in the compiled extension but never importable from `nono_py` — a real defect that shipped silently because nothing imported them until now.

**Impact:** Required a Plan 04 `__init__.py` fix; a reminder that "the `#[pyfunction]` compiles" is not "the symbol is usable from Python."
**Source:** 92-04-SUMMARY.md

### A plan's literal API spec contradicted the required cross-platform behavior
The plan said to validate paths with `is_absolute()`, but the behavioral spec required `/tmp/project` to be accepted — impossible on Windows with `is_absolute()` alone.

**Impact:** The executor had to recognize the contradiction and reconcile against an existing in-repo precedent (`partition_scope`) rather than follow the literal instruction — a case where blind plan-adherence would have produced a wrong result.
**Source:** 92-02-SUMMARY.md

### The 4 pre-existing nono-cli Windows baseline test failures persisted throughout
`profile_cmd init` + 3 `protected_paths` tests fail on this host independent of Phase 92 changes, appearing in every plan's test run.

**Impact:** None to the phase (documented non-regressions in `nono_cli_windows_baseline_test_failures`), but they are persistent noise that each plan had to explicitly distinguish from real failures.
**Source:** 92-01/92-03-SUMMARY.md
