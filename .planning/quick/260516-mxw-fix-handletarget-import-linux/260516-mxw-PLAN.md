---
quick_id: 260516-mxw
slug: fix-handletarget-import-linux
type: execute
task_count: 1
files_modified:
  - crates/nono-cli/src/exec_strategy.rs
autonomous: true
commit_message_suggested: "fix(quick/260516-mxw): import HandleTarget from nono::supervisor (Phase 41 regression)"
must_haves:
  truths:
    - "crates/nono-cli/src/exec_strategy.rs compiles on Linux and macOS targets"
    - "request_path() helper resolves HandleTarget through nono::supervisor (the actual re-export path)"
    - "No other file is touched; library public-API surface is unchanged"
  artifacts:
    - path: "crates/nono-cli/src/exec_strategy.rs"
      provides: "request_path() helper with correct HandleTarget import path"
      contains: "use nono::supervisor::HandleTarget;"
  key_links:
    - from: "crates/nono-cli/src/exec_strategy.rs:2634"
      to: "crates/nono/src/supervisor/mod.rs:56"
      via: "use nono::supervisor::HandleTarget;"
      pattern: "use nono::supervisor::HandleTarget;"
---

<objective>
Fix the unresolved-import regression at `crates/nono-cli/src/exec_strategy.rs:2634`.
The line currently reads `use nono::HandleTarget;`, but the `nono` library does NOT
re-export `HandleTarget` at its top level — only at `nono::supervisor::HandleTarget`.

Purpose: Unblock CI for PR #922. The Phase 41 Plan 41-01 work introduced `fn request_path()`
with a broken import path, surfaced on `oscarmackjr-twg:main` CI run `25970910911`:

- `error[E0432]: unresolved import 'nono::HandleTarget'` at exec_strategy.rs:2634
- Failed lanes: Test(ubuntu-latest), Test(macos-latest), Clippy(macos-latest)
- Windows lanes were still in_progress at observation time and would have hit the same error

Why the regression was missed: Phase 41 cross-target Linux clippy was explicitly SKIPPED on
the Windows host. REQ-CI-01 was marked VERIFIED on grep evidence alone. This reinforces the
existing memory entry `feedback_clippy_cross_target` (Phase 25 CR-A regression lesson):
**Windows-host `cargo clippy --workspace` cannot catch cfg-gated Unix code drift; run
`--target x86_64-unknown-linux-gnu` clippy for Linux-touching plans.**

Why Option A (targeted import path) over Option B (expand lib.rs public API):
Option A is a one-line change confined to exec_strategy.rs. Option B grows the library's
surface area (adds `HandleTarget` to the top-level `pub use` block). exec_strategy.rs is the
sole consumer of `HandleTarget` in the binary crate; threading the namespace is cheaper than
promoting the type to top-level. Principle: no public-API expansion without an explicit reason.

Output: A single-line edit in `crates/nono-cli/src/exec_strategy.rs`, ready for commit with
DCO sign-off and push to update PR #922 CI.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md

<interfaces>
<!-- Key types and contracts the executor needs. Extracted from codebase. -->
<!-- Executor should use these directly — no codebase exploration needed.   -->

From crates/nono/src/lib.rs:102-107 (top-level supervisor re-exports — note the OMISSION):
```rust
#[cfg(target_os = "windows")]
pub use supervisor::BrokerTargetProcess;
pub use supervisor::{
    ApprovalBackend, ApprovalDecision, CapabilityRequest, GrantedResourceKind, ResourceGrant,
    ResourceTransferKind, SupervisorSocket, UrlOpenRequest,
};
// HandleTarget is INTENTIONALLY not in this list. Consumers must use nono::supervisor::HandleTarget.
```

From crates/nono/src/supervisor/mod.rs:50-64 (HandleTarget IS re-exported here):
```rust
pub use types::{
    ApprovalDecision,
    AuditEntry,
    CapabilityRequest,
    GrantedResourceKind,
    // Phase 18 additions:
    HandleKind,
    HandleTarget,
    PipeDirection,
    ResourceGrant,
    ResourceTransferKind,
    SocketProtocol,
    SocketRole,
    SupervisorMessage,
    SupervisorResponse,
    UrlOpenRequest,
};
```

From crates/nono-cli/src/exec_strategy.rs:2629-2642 (current state of the request_path helper):
```rust
/// Reads the request's path using the AIPC-01 shape (target = Some(FilePath{path}))
/// with fallback to the Phase 11 deprecated `path` field. Localizes the
/// deprecation surface to ONE place: once Phase 11 wire-shape is retired,
/// only this helper changes.
fn request_path(request: &nono::CapabilityRequest) -> &std::path::Path {
    use nono::HandleTarget;                              // <-- BROKEN: HandleTarget not at nono::
    match &request.target {
        Some(HandleTarget::FilePath { path }) => path.as_path(),
        _ => {
            #[allow(deprecated)]
            { &request.path }
        }
    }
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Fix HandleTarget import path in request_path() helper</name>
  <files>crates/nono-cli/src/exec_strategy.rs</files>

  <read_first>
    1. Read `crates/nono-cli/src/exec_strategy.rs` lines 2625-2645 to confirm the current
       state matches the action snippet below. If line numbers have drifted, locate the
       `fn request_path(` definition (there is exactly one) and edit the `use nono::HandleTarget;`
       line inside its body.
    2. Read `crates/nono/src/lib.rs` lines 100-111 to confirm `HandleTarget` is NOT in the
       top-level `pub use supervisor::{...}` block (it is intentionally omitted).
    3. Read `crates/nono/src/supervisor/mod.rs` line 56 to confirm `HandleTarget` IS exported
       from `nono::supervisor`.

    If any of these three facts no longer hold, STOP — the codebase has changed since this
    plan was written and the fix may need to be re-derived.
  </read_first>

  <action>
    Replace exactly ONE line inside the `fn request_path()` body in `crates/nono-cli/src/exec_strategy.rs`.

    Before (line 2634):
    ```rust
        use nono::HandleTarget;
    ```

    After (line 2634):
    ```rust
        use nono::supervisor::HandleTarget;
    ```

    Constraints:
    - The surrounding context (lines 2629-2633 and 2635-2642) MUST remain unchanged.
    - The 4-space indent (this is a function-local `use`) MUST be preserved.
    - The `match` arm `Some(HandleTarget::FilePath { path }) => path.as_path(),` on line 2636
      MUST remain unchanged — only the `use` import path changes.
    - The `#[allow(deprecated)]` block (lines 2638-2640) and the `&request.path` fallback
      MUST remain unchanged.
    - The 14 call sites of `request_path(&request)` elsewhere in the file MUST NOT be touched.
    - No other file in the workspace is modified.

    Use the Edit tool with old_string set to `    use nono::HandleTarget;` (4 leading spaces)
    and new_string set to `    use nono::supervisor::HandleTarget;` (4 leading spaces).
    The match is unique because this is the only top-level `use nono::HandleTarget` in the file.
  </action>

  <verify>
    <automated>
      bash -lc '
        set -e
        cd "$(git rev-parse --show-toplevel)"

        # Assertion 1: fixed import path appears exactly once
        FIXED=$(grep -c "use nono::supervisor::HandleTarget;" crates/nono-cli/src/exec_strategy.rs)
        if [ "$FIXED" != "1" ]; then
          echo "FAIL: expected exactly 1 occurrence of fixed import, got $FIXED"; exit 1
        fi

        # Assertion 2: broken import path is gone
        BROKEN=$(grep -c "use nono::HandleTarget;" crates/nono-cli/src/exec_strategy.rs || true)
        if [ "$BROKEN" != "0" ]; then
          echo "FAIL: broken import still present ($BROKEN occurrences)"; exit 1
        fi

        # Assertion 3: the match arm using HandleTarget::FilePath is still in place
        ARM=$(grep -v "^#" crates/nono-cli/src/exec_strategy.rs | grep -c "HandleTarget::FilePath")
        if [ "$ARM" != "1" ]; then
          echo "FAIL: expected exactly 1 HandleTarget::FilePath match arm, got $ARM"; exit 1
        fi

        # Assertion 4: workspace still compiles on the host. cargo check is preferred over
        # full build (faster, same name-resolution surface). If cargo is unavailable, skip
        # and defer to post-push CI verification.
        if command -v cargo >/dev/null 2>&1; then
          cargo check --workspace --all-targets
        else
          echo "WARN: cargo not on PATH; deferring compile check to CI run on PR #922"
        fi

        echo "OK: import fix verified"
      '
    </automated>
  </verify>

  <acceptance_criteria>
    - `crates/nono-cli/src/exec_strategy.rs:2634` reads `    use nono::supervisor::HandleTarget;`
      (verified by grep count == 1 on the exact line)
    - `grep -c "use nono::HandleTarget;" crates/nono-cli/src/exec_strategy.rs` returns 0
      (no stale broken imports remain)
    - The `request_path()` fn body (lines 2635-2641) is unchanged:
      - `match &request.target {` on line 2635
      - `Some(HandleTarget::FilePath { path }) => path.as_path(),` on line 2636
      - `#[allow(deprecated)]` block at 2638
      - `&request.path` fallback at 2639
    - The 14 call sites of `request_path(&request)` elsewhere in exec_strategy.rs are unchanged
      (only the helper's internal `use` import path is modified)
    - No other file is modified in this change
    - `cargo check --workspace` passes on the host platform if cargo is available
    - Full Linux/macOS verification deferred to the post-push CI run on PR #922
  </acceptance_criteria>

  <done>
    Single Rust source line replaced at `crates/nono-cli/src/exec_strategy.rs:2634`.
    Ready for commit with DCO sign-off (`Signed-off-by: ...`) using suggested message:
    `fix(quick/260516-mxw): import HandleTarget from nono::supervisor (Phase 41 regression)`
    and push to update PR #922 CI lanes.
  </done>
</task>

</tasks>

<verification>
- `grep -n "use nono::supervisor::HandleTarget;" crates/nono-cli/src/exec_strategy.rs` returns 1 hit at line 2634
- `grep -n "use nono::HandleTarget;" crates/nono-cli/src/exec_strategy.rs` returns no hits
- `cargo check --workspace --all-targets` (if cargo on PATH) exits 0
- Post-push: CI run on PR #922 shows Test(ubuntu-latest), Test(macos-latest), and Clippy(macos-latest)
  lanes pass the name-resolution stage (the original E0432 is gone)
</verification>

<success_criteria>
- Single-line edit applied at exec_strategy.rs:2634 (broken import → namespaced import)
- No other file in the workspace is modified
- Library public API (crates/nono/src/lib.rs top-level re-exports) is unchanged — `HandleTarget`
  remains accessible only via `nono::supervisor::HandleTarget`, preserving the deliberate
  surface-area boundary
- Host-platform `cargo check` passes (where available)
- Commit message uses the suggested form with DCO sign-off
- CI run on PR #922 clears the E0432 error on Linux/macOS lanes
</success_criteria>

<lesson_reinforced>
This regression reinforces `feedback_clippy_cross_target` (Phase 25 CR-A lesson):
**Windows-host `cargo clippy --workspace` cannot catch unused-import or unresolved-import
drift inside cfg-gated Unix-only code paths.** Phase 41 verifier marked REQ-CI-01 VERIFIED
on grep evidence alone, skipping cross-target Linux clippy. For any future plan that touches
files compiled on Linux/macOS but verified on a Windows host, the verification step MUST run:

```
cargo clippy --workspace --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
```

(or invoke the equivalent CI lane locally) before declaring REQ-CI-01 verified.

This particular `use nono::HandleTarget;` line is not even cfg-gated, but it lives in
`exec_strategy.rs` which contains heavy `#[cfg(unix)]` regions; the Windows-host clippy run
likely never type-checked the surrounding function in a configuration that would surface
the error. The cross-target check is the only reliable safety net.
</lesson_reinforced>

<output>
After completion, create `.planning/quick/260516-mxw-fix-handletarget-import-linux/260516-mxw-SUMMARY.md`
documenting:
- The one-line edit applied
- `cargo check` result on the host platform (or "deferred to CI" note)
- Reminder that CI on PR #922 must be re-checked to confirm the E0432 error is gone on
  Linux and macOS lanes
- Cross-link to `feedback_clippy_cross_target` memory entry
</output>
