---
phase: 115-v3-6-carry-forward-drain
document: code-review-remediation
source_review: 115-REVIEW.md
findings_fixed: [CR-01, WR-03]
findings_deferred: [WR-01, WR-02, WR-04, WR-05, WR-06, IN-01, IN-02, IN-03, IN-04, IN-05, IN-06, IN-07, IN-08]
completed: 2026-08-08
repos:
  - nono @ milestone/v2.13-carryforward-closeout
  - ../nono-py @ 44-broker-ffi-lockstep
---

# Phase 115 Code Review Remediation (CR-01, WR-03)

Fixes the one BLOCKER and one Warning from `115-REVIEW.md` using the
**operator-chosen approach**: validate at the enforcement boundary inside
`nono-proxy`'s core config layer, so no front-end — CLI, PyO3, TypeScript, or a
future embedder — can bypass it.

Scope was CR-01 + WR-03 only. WR-01, WR-02, WR-04, WR-05, WR-06 and all eight
Info findings are DEFERRED by operator decision and were **not** touched.

## Why the boundary, not the binding

The review's primary suggestion was to validate in the PyO3 constructor. The
operator rejected that in favour of the review's own "stronger, durable fix".

Both defects had the same shape: `nono-cli` validated a field at profile-load
time, but a Rust/PyO3/napi embedder builds a `RouteConfig` **directly in Rust**
and never touches a profile. `nono-cli`'s validators were therefore
structurally unable to be the gate — they were an early UX check that only one
of two front-ends ran.

Phase 115's governing rule is *make the defect unrepresentable, not tested-for*.
Two front-ends to one enforcement engine with divergent validation is exactly
the defect class this phase exists to close; fixing only the Python side would
leave it open for the next binding. Validating in the PyO3 constructor would
also have created a **second copy of the header/capture rule vocabulary** in a
binding — the same drift shape DRAIN-02/NEW-06 removed from the denial-category
codec earlier in this phase.

## What changed

### Enforcement point

`crates/nono-proxy/src/route.rs`, `RouteStore::load()` — a new
`route.validate()?` as the **first** statement of the per-route loop.

```
nono-cli   proxy_runtime.rs:604  nono_proxy::server::start  ─┐
                                                             ├─> RouteStore::load ─> RouteConfig::validate
nono-py    start_proxy           nono_proxy::start          ─┘   (server.rs:601)
```

Both entry points verified empirically, not by inspection:

- **CLI path**: `crates/nono-cli/src/proxy_runtime.rs:604` is the only
  `nono_proxy::server::start` call site in the workspace; `server.rs:563` is
  the only `pub async fn start` in `nono-proxy` (grepped, zero others).
- **Embedder path**: `../nono-py/src/proxy.rs:1131` — `start_proxy` →
  `nono_proxy::start(rust_config)` → the same `server::start`. Confirmed
  end-to-end against a **built wheel** (transcript below), not just by reading.

`server::start` skips `RouteStore::load` when `config.routes` is empty, which
is vacuously safe — there is no route to validate.

Placement inside the loop is deliberate: validation runs **before** glob
compilation, before any TLS CA file is read, and before the SPIFFE Workload API
connect. A route whose `inject_header` would smuggle CRLF onto the wire never
gets as far as acquiring a live credential source. Fail-closed via `?`: the
first violation aborts the whole proxy start, never a per-route skip — the same
contract as the pre-existing upstream-URL and SPIFFE-connect failures.

### Validation logic

`crates/nono-proxy/src/config.rs` — new `is_http_token_char` (public, mirrors
`nono-cli`'s private copy), three private helpers, and three `validate` methods:

| Method | Enforces | Mirrors |
|---|---|---|
| `SpiffeAuthConfig::validate` | `inject_header` non-empty RFC 7230 token; effective `credential_format` CRLF-free; `workload_api_socket`/`audience` non-empty | `profile::validate_header_name` / `validate_header_mode` |
| `CaptureConfig::validate` | `response_fields` non-empty; every field path non-empty / no leading-trailing `.` / no `..` / no NUL, on **both** `response_fields` and `request_nonce_fields`; `max_response_bytes != 0`; `max_response_bytes <= CAPTURE_MAX_RESPONSE_BYTES_CEILING` | `credential_provider::validate_capture_config` |
| `RouteConfig::validate` | route's own `inject_header` + effective `credential_format`, then delegates to the two above | `profile::validate_header_mode` |

Like `nono-cli`'s `validate_header_mode`, the credential-format check runs on
the **effective** format (`resolved_credential_format`), which is what actually
reaches the wire. Both defaults (`Bearer {}` / `{}`) are CRLF-free, so this is
strictly stronger than checking the raw `Option`.

The doc comment on `CAPTURE_MAX_RESPONSE_BYTES_CEILING` said the ceiling was
"Enforced at profile-validation time (Plan 114-08), **not here**". That is now
false in the useful direction — updated to name both layers.

### One addition beyond the review's literal text

The review's CR-01 names only `SpiffeAuthConfig`'s fields. `RouteConfig`'s
**own** `inject_header` / `credential_format` feed the identical raw
`format!("{}: {}\r\n", ..)` wire write for static-credential routes
(`credential.rs:281` → `reverse.rs`), are equally unvalidated on the PyO3 path
(`RouteConfig::new` takes `inject_header: String` verbatim), and are the same
CR-01 primitive reachable through a sibling field of the same struct.

Validating only `spiffe` would have made the "unrepresentable" claim false the
moment anyone looked one field to the left. This is treated as CR-01's own
class, not as scope creep into another finding. It is the only thing added
beyond the two findings' literal text.

### CLI-side checks

Retained unchanged as defense-in-depth. They keep better, credential-named
error messages and fire earlier in the `nono profile validate` flow. Zero lines
deleted from `profile/mod.rs` or `profile/credential_provider.rs`.

## Load-bearing verification

A grep is not proof. The `route.validate()?` line was removed, the suite re-run,
and the line restored.

**With the fix in place** (`cargo test -p nono-sandbox-proxy`):

```
test result: ok. 307 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**With `route.validate()?` replaced by a comment**
(`cargo test -p nono-sandbox-proxy --lib`):

```
failures:
    route::tests::cr01_load_rejects_crlf_bearing_route_inject_header
    route::tests::cr01_load_rejects_crlf_bearing_spiffe_credential_format
    route::tests::cr01_load_rejects_crlf_bearing_spiffe_inject_header
    route::tests::wr03_load_rejects_capture_max_response_bytes_above_ceiling
    route::tests::wr03_load_rejects_the_other_three_capture_gates

test result: FAILED. 302 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out
```

Exactly the five new gate tests fail and nothing else. The sixth new test
(`boundary_validation_does_not_over_reject_valid_routes`) is a negative control
and correctly still passes — an all-reject implementation would not satisfy it.

Two failure messages from that run are worth quoting, because they prove
**ordering** as well as presence:

```
---- cr01_load_rejects_crlf_bearing_spiffe_inject_header ----
the error must name the offending field so an operator can find it, got:
  Configuration error: route 'svc': SPIFFE JWT source failed to connect:
  ... initial synchronization timed out
```

With the gate removed, the CRLF-bearing route gets all the way to a live SPIRE
Workload API connect. With the gate in place, it is rejected before that
connect is attempted — which is why
`cr01_load_rejects_crlf_bearing_spiffe_inject_header` asserts the error does
**not** contain `"failed to connect"`.

### Tests are driven through `RouteStore::load`, not `validate()`

Deliberate. The defect was never "the rules are wrong", it was "the rules are
not on the path a non-CLI embedder takes". A test calling `RouteConfig::validate`
directly would still pass if someone deleted the `route.validate()?` line —
i.e. it would re-open the exact hole it was written to close. Driving `load()`
proves the gate is **wired**, not merely present.

### End-to-end proof through the real Python embedder

The strongest available evidence: freshly built + installed wheel, real
`nono_py.start_proxy`, review-supplied hostile literals.

```
CR-01 spiffe.inject_header: RuntimeError: Failed to start proxy: Configuration error:
  route 'svc': spiffe.inject_header 'X-Svc\r\nX-Admin: true' is not a valid HTTP token;
  header names must be alphanumeric plus !#$%&'*+-.^_`|~ (RFC 7230). A CR or LF here
  would smuggle attacker-chosen headers into an authenticated upstream request
CR-01 spiffe.credential_format: RuntimeError: Failed to start proxy: Configuration error:
  route 'svc': spiffe.credential_format must not contain CR or LF; this would enable
  HTTP header injection / request smuggling on an authenticated upstream request
CR-01 route.inject_header: RuntimeError: Failed to start proxy: Configuration error:
  route 'svc': inject_header 'X-Svc\r\nX-Admin: true' is not a valid HTTP token; ...
WR-03 capture ceiling 2**32: RuntimeError: Failed to start proxy: Configuration error:
  route 'svc': capture.max_response_bytes (4294967296) exceeds the hard ceiling of
  1048576 bytes (D-05)
WR-03 empty response_fields: RuntimeError: Failed to start proxy: Configuration error:
  route 'svc': capture.response_fields must not be empty; a capture config declaring
  nothing to rewrite is not valid
CONTROL valid route: STARTED on port 58035  <-- NOT BLOCKED

all 5 hostile configs blocked: True
valid control started: True
EXIT=0
```

The control line is the point: the gate blocks the five hostile shapes and
still starts a real proxy for a valid capture-bearing route.

## Test fixtures changed

Exactly **one**, in `crates/nono-proxy/src/route.rs`:

| Fixture | Verdict | Reasoning |
|---|---|---|
| `capture_route_config` (`route.rs`, `response_fields: vec![]`) | **Fixture was wrong** — fixed the fixture, not the validation | The empty vec never expressed anything the test needed. Its two consumers (`test_capture_declared_for_upstream_true_across_different_port` and friends) assert `capture_declared_for_upstream`'s host-only, port-ignoring matching, for which only `capture.is_some()` matters. `vec![]` was the cheapest literal that compiled while nothing checked it — precisely the "only ever valid because nothing checked it" case. Replaced with a minimal real declaration (`response_fields: [access_token/opaque]`). |

Six other `response_fields: vec![]` literals exist in `reverse.rs` (×5) and
`server.rs` (×1). None needed changing: they construct a `CaptureConfig` and
hand it directly to `relay_response_with_capture` / `RouteStore::
from_loaded_routes`, neither of which traverses `RouteStore::load`. Those tests
exercise the *rewrite* machinery, not route loading, so an empty field list is
a legitimate input there. Verified by running the suite, not by assuming.

**No security gate was weakened to make a test pass.**

## Gate results

| Gate | Result |
|---|---|
| `cargo build --workspace --all-targets` | exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | clean |
| `cargo test -p nono-sandbox-proxy` | 307 + 6 pass, 0 fail (was 301 + 6; +6 new) |
| `cargo test -p nono-sandbox-cli --bin nono` | 1537 pass / 11 fail — **byte-identical to the documented Windows-host baseline** (6 × `config::tests`, 3 × `protected_paths::tests`, `profile_cmd::test_init_allowed_when_pack_has_same_short_name`, `audit_session::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`). No new failures. |
| `../nono-py` `cargo test` | 77 pass, 0 fail (was 71; +6 new) |
| `../nono-py` `maturin build` | wheel produced (below) |

Cross-target clippy gates were not run, per `115-VALIDATION.md`: no in-scope
file is cfg-gated Unix code, under `exec_strategy/`, or under `bindings/c/src/`.

### `maturin build`

```
📦 Including license file `LICENSE`
🍹 Building a mixed python/rust project
🐍 Found CPython 3.12 at C:\Users\OMack\AppData\Local\Programs\Python\Python312\python.exe
🔗 Found pyo3 bindings
📡 Using build options features from pyproject.toml
   Compiling pyo3-build-config v0.28.3
   Compiling pyo3-macros-backend v0.28.3
   Compiling pyo3-ffi v0.28.3
   Compiling pyo3 v0.28.3
   Compiling pyo3-macros v0.28.3
   Compiling nono-py v0.70.0 (C:\Users\OMack\nono-py)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 24.18s
📦 Built wheel for CPython 3.12 to
   C:\Users\OMack\nono-py\target\wheels\nono_sandbox-0.70.0-cp312-cp312-win_amd64.whl
```

No binding drift: the wheel built clean on the first attempt and the resulting
module imported and ran the end-to-end script above. `maturin develop` was then
used to install it into `.venv` for the E2E run.

### `../nono-py` baseline notes (pre-existing, untouched)

- `cargo fmt --check` reports 84 diffs and `cargo clippy -- -D warnings`
  reports 7 errors **at HEAD before this change**; all clippy locations are in
  `src/override.rs` and all fmt diffs are outside `src/proxy.rs`. Verified by
  re-running both against a clean tree. This change adds **zero** new fmt diffs
  and **zero** new clippy findings.
- `pytest tests/` cannot collect 4 modules on a Windows host
  (`test_integration_exec`, `test_proxy_only`, `test_sandboxed_exec`,
  `test_smoke`) because `sandboxed_exec` is `#[cfg(unix)]` (`src/lib.rs:20-21`,
  documented at `python/nono_py/__init__.py:78`). Pre-existing platform gap.
- `pytest tests/` on the default basetemp additionally produces 42 errors, all
  `PermissionError: [WinError 5] Access is denied:
  'C:\Users\OMack\AppData\Local\Temp\pytest-of-OMack'` — a host tmpdir
  permission problem, not a code fault. Re-run with a writable `--basetemp`
  (and the 4 uncollectible modules plus the slow `test_live_arm` excluded):
  **5 failed, 150 passed, 2 skipped**.
- Those 5 (`test_broker_ffi_mapping::test_sandbox_init_error_maps_to_runtime_error`,
  `test_capability_set::test_multiple_paths`,
  `test_confined_run::test_write_inside_workspace_allowed`,
  `test_policy::test_validate_deny_overlaps_matches_platform_behavior`,
  `test_query::test_context_reflects_caps_at_creation_time`) are the Windows-host
  baseline — e.g. `test_multiple_paths` fails with
  `FileNotFoundError: Path does not exist: /var`, a Unix path asserted on a
  Windows host. **Not caused by this change, provably**: every one of them
  exercises the `nono` core crate (CapabilitySet / QueryContext / confined run /
  policy), and this change touched only
  `crates/nono-proxy/src/{config,route}.rs` — the `nono` core crate is
  byte-identical. Grepped the whole pytest suite for the proxy API: only
  `test_proxy_only.py` (uncollectible on Windows) and one
  `isinstance(config, ProxyConfig)` assertion in `test_policy.py` reference it,
  and **no** pytest test calls `start_proxy` at all. There is therefore no
  pytest coverage of the changed path — which is precisely why the reachability
  proof was written as a standalone E2E script and as six Rust tests in
  `../nono-py/src/proxy.rs`.

## Binding signature decision

PyO3 constructor signatures were **not** changed to `PyResult<Self>`.

The operator's rationale for the boundary fix applies to the binding too:
adding constructor validation would re-create the second-copy-of-the-rules
problem in the very place the fix exists to eliminate. A Python caller gets
`RuntimeError` from `start_proxy` rather than `ValueError` from `__init__` —
later, but still before anything is bound, connected, or served.

`_nono_py.pyi` documents this on `SpiffeAuthConfig`, `CaptureConfig`, and
`start_proxy`: what the rules are, where they are enforced, and which exception
surfaces. Parameter lists were left untouched — the stub's `tls_client_cert` /
`tls_client_key` drift is **WR-04**, which is deferred.

## What was deliberately NOT done

WR-01 (`ALL` drift guard), WR-02 (dead `validate_aws_auth` / tautological
guard), WR-04 (`.pyi` parameter drift), WR-05 (`profile diff` false positives),
WR-06 (uninstrumented denial branches), and IN-01 … IN-08 remain open exactly
as `115-REVIEW.md` records them. No adjacent lines were opportunistically
changed.

`.planning/STATE.md` and `.planning/ROADMAP.md` were not modified and no
`gsd-sdk` state/roadmap/phase verb was run.

## Commits

| Repo | Commit | Subject |
|---|---|---|
| `nono` | `a53fda18` | `fix(115): enforce route header/capture validation at the proxy boundary` |
| `../nono-py` | `d5ed3ab` | `test(115): prove the nono-proxy boundary gate is reachable from PyO3` |

Both carry `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
