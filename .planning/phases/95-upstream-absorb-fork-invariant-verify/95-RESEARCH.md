# Phase 95: Upstream Absorb + Fork-Invariant Verify — Research

**Researched:** 2026-06-26
**Domain:** Rust workspace cherry-pick absorb, fork-invariant preservation, Windows security model verification
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01 — Cluster B shared-surface extraction only.** Extract additive `crates/nono/src/audit.rs`
event-type definitions (CR-02 carve-out, additive-only, MUST NOT touch `records_verified: event_count > 0`)
plus proxy-surface hunks from #1105 that apply cleanly. SKIP the tool-sandbox subsystem directory
(absent in fork) and SKIP the `tls_intercept/` hunks (fork has no `crates/nono-proxy/src/tls_intercept/`).

**D-02 — Cluster C preserve fork divergence; apply bug fix only.** Upstream `9b37dc52` reverses the
fork's Phase 89 fail-secure proxy-activation behavior. Keep the fork's expression and the
`proxy_activates_with_custom_credentials_only` guard test as a regression sentinel; apply ONLY the
`credentials_intent` bug-fix block. Do NOT adopt upstream's explicit-activation refactor wholesale.

**D-03 — Land in Phase 95, defer cross-target clippy to Phase 96.** Cherry-pick Clusters A and B in
this phase; gate on native Windows clippy + `make build` + `make test`. Cross-target clippy
(`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`) runs in Phase 96, recorded PARTIAL→96 per
`.planning/templates/cross-target-verify-checklist.md`.

**D-04 — No-NEW-failures vs documented baseline.** SC2 passes if cherry-picks introduce zero new
failures relative to the known ~5-red Windows baseline. Do NOT expand scope to fix pre-existing
baseline reds. Planner MUST capture the baseline red set at the phase-base commit BEFORE any
cherry-pick.

### Claude's Discretion

- **Cluster D `sigstore-verify` dep-bump evaluation:** Whether to absorb the `sigstore-verify` 0.8.0→0.9.0
  bump in this phase as dependency/security hygiene, or fold into Phase 97 with Cluster D — guided by D-04
  (no-new-failures) and whether the bump is build-clean on Windows.
- Cherry-pick mechanics (`git cherry-pick -x` vs manual replay per commit), commit ordering, and plan/wave
  decomposition.

### Deferred Ideas (OUT OF SCOPE)

- **Cluster D** (release metadata: v0.64.1/v0.65.0/v0.65.1 + leapfrog floor >= 0.65.0) → Phase 97.
- **Full tool-sandbox subsystem absorb** (the #1105 hunks skipped by the D-01 split) → future phase.
- MSI VC++ prereq, POC-cert broker clean-host → FUT-03.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UPST10-02 | All will-sync clusters absorbed into fork (cherry-pick with `-x` or manual replay), each commit DCO-signed, without regressing Windows security model | SHA reachability confirmed; cherry-pick strategy documented; conflict surfaces mapped per cluster |
| UPST10-03 | Fork-divergent invariants explicitly preserved and verified post-sync — Windows backend, ADR-86 audit/diagnostics boundary, exec_strategy_windows/ denial-rendering fork — with `make build` + `make test` green on dev host | Invariant files identified; verification methods (git diff + test names) specified; baseline capture protocol documented |

</phase_requirements>

---

## Summary

Phase 95 absorbs three upstream commits (`9ce74e92` Cluster A, `11fd10e0` Cluster B split,
`9b37dc52` Cluster C split) from the `nolabs-ai/nono` `v0.64.0..v0.65.1` window, then verifies the
fork's three structural invariants are unregressed. All three SHAs are already present in the local
object store (confirmed via `git cat-file -t`). The `upstream` remote points at `nolabs-ai/nono`
and has already been fetched (Phase 94). No additional `git fetch` is needed before cherry-picking.

The absorb work decomposes cleanly into three distinct operations with different conflict strategies:
(A) straight cherry-pick with expected IPC ordering change; (B) surgical manual-replay extracting only
the additive hunks, skipping tool-sandbox dir and tls_intercept/; (C) manual-replay applying only the
`credentials_intent` fix, preserving the fork's fail-secure `active` predicate.

The most consequential research finding is that Cluster B (`11fd10e0`) is structurally INCOMPATIBLE
with a straight cherry-pick — it introduces `ApprovalRequest` (an enum that replaces `CapabilityRequest`)
and removes `wait_for_child_with_startup_timeout` from exec_strategy.rs as part of tool-sandbox
plumbing. A straight cherry-pick would fail to compile. The ledger's split guidance is correct: manual
hunk extraction is mandatory for Cluster B.

The baseline-red Windows test set (D-04) is: 1 failure in `nono` lib (`try_set_mandatory_label`) +
4 failures in `nono-cli` (`profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` + 3
`protected_paths::tests::*`). Confirmed at HEAD (`0e3d1a68`) via live `cargo test`.

**Primary recommendation:** Three-wave plan: Wave 1 = baseline capture + Cluster A cherry-pick + gate;
Wave 2 = Cluster B manual extraction + Cluster C manual extraction + gate; Wave 3 = fork-invariant
checklist + PARTIAL→96 handoff record.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| AF_UNIX mediation IPC (Cluster A) | Library (`crates/nono/src/sandbox/linux.rs`) | CLI (`exec_strategy.rs` supervisor loop) | Syscall filter + IPC handshake lives in the library sandbox driver; supervisor loop in CLI coordinates timing |
| Audit event types (Cluster B additive) | Library (`crates/nono/src/audit.rs`) | — | Event type definitions are library-side per ADR-86; CLI adds recording calls only |
| Proxy activation predicate (Cluster C) | CLI (`crates/nono-cli/src/proxy_runtime.rs`) | CLI (`network_policy.rs`) | Policy decision lives in CLI per ADR-86 library/CLI boundary |
| Windows backend verification | CLI (`exec_strategy_windows/`) | Library (`sandbox/windows.rs`) | All Windows denial rendering is CLI-side (ADR-86 D-03 deliberate fork carve-out) |
| Fork-invariant guard tests | Library tests in `audit.rs` | CLI tests in `proxy_runtime.rs` | CR-02 guard in library; Cluster F proxy guard in CLI |

---

## Standard Stack

This phase installs NO new external packages. All tooling is pre-existing.

| Tool | Version | Purpose |
|------|---------|---------|
| `git cherry-pick -x` | system git | Cluster A absorb with upstream ref trailer |
| `git cherry-pick --no-commit` + manual hunk apply | system git | Clusters B and C manual extraction |
| `make ci` | Makefile | Gate after each wave (clippy + fmt + tests) |
| `cargo test --workspace` | Cargo 1.82 | Baseline capture and post-absorb gate |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | Rust 1.82 | Native Windows clippy gate (D-03) |

---

## Package Legitimacy Audit

> Not applicable — this phase installs no new external packages.

**Note on `sigstore-verify` 0.9.0 (Claude's Discretion):** If the planner decides to absorb the
sigstore-verify dep bump in this phase, the package is a minor-version bump of an existing
dependency already in `crates/nono/Cargo.toml`. No slopcheck required (known dep, not a new
introduction). The bump is a Cargo.toml one-line change + Cargo.lock regeneration.

---

## Architecture Patterns

### System Architecture Diagram

```
Phase 95 data flow:

upstream remote (nolabs-ai/nono)
  → [already fetched in Phase 94; objects in local repo]
  → SHA guard: git cat-file -t {9ce74e92, 11fd10e0, 9b37dc52}
  → all three: COMMIT (verified)

WAVE 1:
  baseline capture → cargo test --workspace → record 5 known-red tests
  git cherry-pick -x 9ce74e92 → (conflict expected in exec_strategy.rs)
  → resolve: preserve fork's IPC ordering; supervisor loop context
  → git am --continue / commit with DCO trailer
  → make ci gate

WAVE 2:
  git cherry-pick --no-commit 11fd10e0  [Cluster B]
  → extract hunks:
      APPLY: crates/nono/src/audit.rs (additive structs only, skip test rename)
      APPLY: crates/nono/src/sandbox/linux.rs (restrict_execute + has_execute)
      APPLY: crates/nono/src/sandbox/mod.rs (re-export restrict_execute)
      APPLY: crates/nono/src/keystore.rs (cmd:// URI prefix)
      APPLY: crates/nono/src/scrub.rs (SENSITIVE_ENV_VARS list)
      APPLY: crates/nono-proxy/src/route.rs (endpoint_policy field; SKIP tls_intercept refs)
      APPLY: crates/nono-proxy/src/server.rs (shutdown_tx send; SKIP approval/nonce_resolver)
      SKIP: crates/nono-cli/src/tool-sandbox/ (entire dir, absent in fork)
      SKIP: crates/nono-proxy/src/tls_intercept/ (entire dir, absent in fork)
      SKIP: exec_strategy.rs timeout refactor (entangled with tool-sandbox plumbing)
      SKIP: supervisor/types.rs ApprovalRequest enum (tool-sandbox plumbing)
      SKIP: audit.rs test rename (CapabilityRequest→ApprovalRequest, fork still uses CapabilityRequest)
  → commit with DCO trailer
  → git cherry-pick --no-commit 9b37dc52  [Cluster C]
  → extract hunks:
      APPLY: proxy_runtime.rs credentials_intent block (if has_credentials || !prepared.custom_credentials.is_empty())
      PRESERVE: proxy_runtime.rs active predicate (keep custom_credentials disjunct at lines 95, 118)
      PRESERVE: proxy_activates_with_custom_credentials_only test (keep assertion opts.active == true)
      SKIP: proxy_runtime.rs test rename (test_proxy_is_active→inactive)
      SKIP: network_policy.rs all_names change (fork already returns early on empty service_names)
      SKIP: launch_runtime.rs is_active refactor (fork uses bool field, not CredentialProxyIntent)
  → commit with DCO trailer
  → make ci gate

WAVE 3:
  fork-invariant verification checklist (3 invariants)
  PARTIAL→96 handoff record
  DIVERGENCE-LEDGER update (mark will-sync rows closed)
```

### Recommended Project Structure

No new directories. All changes land in existing files.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cherry-pick conflict resolution | Manual rewrite from scratch | `git cherry-pick --no-commit` + selective `git checkout --patch` | git tracks the upstream-ref connection; -x trailer proves absorb lineage |
| Fork baseline test tracking | Custom test runner | `cargo test --workspace 2>&1` piped to a file | Standard tool; captures all crate test results in one command |
| DCO sign-off | Manual commit message edit | `git commit -s` or `--trailer "Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>"` | Consistent format; forgetting DCO requires amend which loses the -x trailer |

---

## SHA and Commit Inventory (Verified)

[VERIFIED: git cat-file -t on each SHA at HEAD `0e3d1a68`]

| Cluster | SHA (short) | SHA (full prefix) | Subject | Disposition | Cherry-pick Strategy |
|---------|-------------|-------------------|---------|-------------|---------------------|
| A | 9ce74e92 | 9ce74e9213e85e4bd5471136d70d0fb8d7c0de24 | fix(sandbox): exempt IPC fd from sendmsg trapping to resolve af_unix_mediation deadlock (#1210) | will-sync | `git cherry-pick -x` with conflict resolution |
| B | 11fd10e0 | 11fd10e0c88e747b6d751fec9b38f207276b747c | feat(sandbox): tool sandbox (#1105) | split | `git cherry-pick --no-commit` + manual hunk extraction |
| C | 9b37dc52 | 9b37dc523c37bb943b797021cf9180729214be97 | refactor(credentials): require explicit activation for custom credentials (#1215) | split | `git cherry-pick --no-commit` + manual hunk extraction |

No `git fetch` required — all three SHAs are already in the local object store.

---

## Conflict Surface Map (Per-Cluster)

### Cluster A (`9ce74e92`) — AF_UNIX Mediation Deadlock Fix

**Files touched (6):**
1. `crates/nono/src/sandbox/linux.rs` — BPF filter install ordering; no fd-based exemptions
2. `crates/nono/src/supervisor/socket.rs` — new `read_fd_number()` helper (additive)
3. `crates/nono-cli/src/exec_strategy.rs` — supervisor loop IPC handshake timing
4. `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — notify fd acquisition via pidfd_getfd
5. `crates/nono-cli/tests/socket_access_run.rs` — new integration test (234 lines)
6. `crates/nono-cli/data/profile-authoring-guide.md` — docs

**Expected conflict:** `exec_strategy.rs` — this file is heavily used by both Cluster A and Cluster B.
Cluster A touches `execute_supervised()` around line 1029 (supervisor loop IPC sequence). If
cherry-picking Cluster A first (correct order), the conflict is resolvable: Cluster A's change is
functional (correct IPC handshake ordering); the fork's current HEAD has the pre-fix IPC logic.

**Fork-preserve rule for Cluster A:** None — this IS the fix. Accept the upstream change on all
conflict hunks. The only invariant to protect is `exec_strategy_windows/` which is NOT touched by
Cluster A (windows-touch: no).

**Cross-target note (D-03):** `sandbox/linux.rs` and `exec_strategy/supervisor_linux.rs` contain
`#[cfg(target_os = "linux")]` blocks. Per CLAUDE.md MUST/NEVER rule, cross-target clippy is REQUIRED
but deferred to Phase 96. Record as PARTIAL→96.

### Cluster B (`11fd10e0`) — Tool Sandbox Feature (split extraction)

**Strategy: `git cherry-pick --no-commit 11fd10e0` then selectively stage only the approved hunks.**

[VERIFIED: git show 11fd10e0 --stat; actual diff inspection]

**APPLY — 7 files with additive/self-contained changes:**

| File | What to apply | What to skip |
|------|--------------|-------------|
| `crates/nono/src/audit.rs` | New `SandboxRuntimeAuditEvent` and `CommandPolicyAuditEvent` structs + `record_sandbox_runtime_event()` + `record_command_policy_event()` methods | Test import rename (`CapabilityRequest` → `ApprovalRequest`); new struct-literal fields in tests (`endpoint_policy_action`, `approval_backend`, etc.) — these require `ApprovalRequest` enum which is tool-sandbox machinery |
| `crates/nono/src/audit.rs` | `record_network_event()` signature change: `event: NetworkAuditEvent` → `event: Box<NetworkAuditEvent>` | Nothing else |
| `crates/nono/src/sandbox/linux.rs` | `has_execute()` method on `DetectedAbi`; `restrict_execute()` pub fn | Nothing — no tls_intercept or tool-sandbox refs in these hunks |
| `crates/nono/src/sandbox/mod.rs` | Re-export of `restrict_execute` via `pub use linux::...`; `Sandbox::restrict_execute()` method | Nothing |
| `crates/nono/src/keystore.rs` | `CMD_URI_PREFIX` const + `is_cmd_uri()` fn + early-return error for `cmd://` in `load_secrets()` | Nothing |
| `crates/nono/src/scrub.rs` | `SENSITIVE_ENV_VARS` list additions (ANTHROPIC_API_KEY etc.); doc comment update | Nothing |
| `crates/nono-proxy/src/route.rs` | `endpoint_policy` field on route struct; `CompiledEndpointPolicy` import; `allows_all_without_l7()` call in condition | All tls_intercept refs; `#[cfg(test)]` test additions that reference `endpoint_policy: None` struct literals (may apply cleanly or may fail if `endpoint_policy` field is newly required) |
| `crates/nono-proxy/src/server.rs` | `shutdown_tx.send(true)` call | `approval_backends`, `credential_capture_backend`, `nonce_resolver` fields; `start_with_approval()` and `start_with_approval_registry()` functions — these require types absent in the fork |

**SKIP entirely — 3 categories:**
- `crates/nono-cli/src/tool-sandbox/` directory — absent in fork
- `crates/nono-proxy/src/tls_intercept/` directory — absent in fork
- `crates/nono-cli/src/exec_strategy.rs` timeout refactor (removal of `wait_for_child_with_startup_timeout`) — entangled with tool-sandbox; would break compile
- `crates/nono/src/supervisor/types.rs` `ApprovalRequest` enum addition — tool-sandbox plumbing

**CRITICAL COMPILER DEPENDENCY:** The `record_network_event()` boxing change (`Box::new(event)`) means
any call site that passes `NetworkAuditEvent` without boxing will break. Search all callers:

```bash
grep -rn "record_network_event" crates/ --include="*.rs"
```

Apply the boxing at ALL call sites in the same commit.

**CRITICAL: audit.rs test section requires special handling:**
The test import must stay as `use crate::supervisor::{ApprovalDecision, CapabilityRequest}` (fork
type). Skip the `ApprovalRequest` rename. Also skip the struct-literal field additions
(`endpoint_policy_action`, `approval_backend`, `credential_capture_*`, `upstream`) because
`CapabilityDecision` in the fork does NOT have those fields.

**Cross-target note (D-03):** `sandbox/linux.rs`, `sandbox/mod.rs` contain `#[cfg(target_os = "linux")]`
blocks. Record PARTIAL→96.

### Cluster C (`9b37dc52`) — Custom Credentials Explicit-Activation Refactor (split extraction)

**Strategy: `git cherry-pick --no-commit 9b37dc52` then selectively stage only the credentials_intent fix.**

**Files touched (3):**
1. `crates/nono-cli/src/proxy_runtime.rs` — the conflict file
2. `crates/nono-cli/src/network_policy.rs` — custom_credentials auto-activation removal
3. `crates/nono-cli/src/launch_runtime.rs` — `is_active` method on `CredentialProxyIntent`

**APPLY from `proxy_runtime.rs`:**

The `credentials_intent` block fix in upstream: changing `if has_credentials {` to
`if has_credentials || !prepared.custom_credentials.is_empty() {`. However, the fork's
`proxy_runtime.rs` does NOT have a `credentials_intent` variable — the fork uses a different
structural shape (`ProxyLaunchOptions` has `credentials: Vec<String>` and
`custom_credentials: HashMap<...>` as separate flat fields, not a `CredentialProxyIntent` wrapper).

**ACTUAL APPLY:** The upstream's `credentials_intent` fix maps to the fork's existing `custom_credentials`
handling. In the fork, `ProxyLaunchOptions` already carries `custom_credentials` (lines 126, 102 in
launch_runtime.rs). The "fix" that IS applicable to the fork's shape is confirming that
`custom_credentials` is always propagated into `ProxyLaunchOptions` — which the fork already does
(line 126: `custom_credentials: prepared.custom_credentials.clone()`). This hunk may be a **no-op
in the fork** structurally. The planner must verify by inspecting the upstream diff against fork shape.

**PRESERVE (non-negotiable, DO NOT apply):**
- Lines 90-119 of fork's `proxy_runtime.rs`: the `active` computation including
  `|| !prepared.custom_credentials.is_empty()` at lines 95 and 118.
- Test `proxy_activates_with_custom_credentials_only` (line 503) — assertion `opts.active == true`
  must remain. The upstream renames this to `test_proxy_is_inactive_when_only_custom_credentials_are_set`
  with inverted assertion — DO NOT apply.

**SKIP from `network_policy.rs`:**
The fork's `resolve_credentials()` already returns early if `service_names.is_empty()` (line 188).
The upstream removes `custom_credentials.keys()` from `all_names` initialization — but the fork
never had that auto-add pattern. The fork's `all_names` construction already requires explicit
service names. Applying the Cluster C `network_policy.rs` change would be a no-op or could break
custom credential resolution. **SKIP this hunk.**

**SKIP from `launch_runtime.rs`:**
The upstream's `is_active` refactor operates on `CredentialProxyIntent` (a struct in upstream's
`ProxyLaunchOptions.credentials: Option<CredentialProxyIntent>`). The fork's `ProxyLaunchOptions`
has `active: bool` (a flat bool field set in `prepare_proxy_launch_options`). The struct shapes are
incompatible — the upstream's `launch_runtime.rs` hunk does not apply. **SKIP.**

---

## Baseline Capture Mechanics (D-04)

[VERIFIED: `cargo test --workspace` run at HEAD `0e3d1a68`]

**Command to capture the baseline:**

```bash
cargo test --workspace 2>&1 | tee ci-logs-local/baseline-95/baseline-before-cherry-picks.txt
```

**Known-red Windows baseline (5 tests total) — verified at `0e3d1a68`:**

| Crate | Test Name | Failure Type |
|-------|-----------|-------------|
| `nono` lib | `sandbox::windows::tests::try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails` | panics — env-sensitive DACL test (pre-existing Phase 74 code) |
| `nono-cli` | `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` | pre-existing |
| `nono-cli` | `protected_paths::tests::blocks_child_directory_capability` | pre-existing |
| `nono-cli` | `protected_paths::tests::blocks_parent_directory_capability` | pre-existing |
| `nono-cli` | `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root` | pre-existing |

**Gate rule (D-04):** After each cherry-pick wave, `cargo test --workspace` must show the SAME 5 red
tests (or fewer). Any NEW failure name = blocker; must be fixed before proceeding.

**Baseline file location:** Create `ci-logs-local/baseline-95/` directory. Write baseline BEFORE the
first cherry-pick. Commit the baseline file (or note it in the plan summary so Phase 96 can reference it).

---

## Fork-Invariant Verification Checklist (UPST10-03)

Three invariants must be explicitly verified after all cherry-picks land. One checklist entry per
invariant in the VALIDATION.md / plan SUMMARY.

### Invariant 1: AppContainer/WFP/Broker Windows Backend

**Files:** `crates/nono-cli/src/exec_strategy_windows/` (all: `dacl_guard.rs`, `labels_guard.rs`,
`launch.rs`, `mod.rs`, `network.rs`, `restricted_token.rs`, `supervisor.rs`)

**Verification method:**

```bash
git diff HEAD~N -- crates/nono-cli/src/exec_strategy_windows/
# Must be empty (zero output) — no cherry-pick touches these files
```

**Why:** All 3 cherry-pick commits are `windows-touch: no` per the ledger. But the planner MUST
explicitly verify this post-cherry-pick rather than trusting the ledger declaration.

**Additional check:** `crates/nono/src/sandbox/windows.rs` must also be unmodified:

```bash
git diff HEAD~N -- crates/nono/src/sandbox/windows.rs
```

**Guard:** The `try_set_mandatory_label` test (still red as pre-existing baseline) confirms the
Windows backend code is unchanged — if the baseline changes character (e.g., new panic message),
investigate.

### Invariant 2: ADR-86 Audit/Diagnostics Library-Boundary Carve-Out

**Files:**
- `crates/nono/src/audit.rs` — must NOT have `records_verified: event_count > 0` changed
- `crates/nono-cli/src/diagnostic/` — must remain as the UX rendering layer (untouched by this sync)
- `bindings/c/src/` — CR-01 carve-out; must be untouched

**Verification method:**

```bash
# CR-02: records_verified line must remain 'event_count > 0'
grep -n "records_verified" crates/nono/src/audit.rs
# Expected: line ~1435: records_verified: event_count > 0

# Guard test must still exist and pass
cargo test -p nono --lib -- audit::tests::verify_empty_log_with_no_stored_metadata_is_not_valid
# Expected: test ... ok

# CR-01: no changes to bindings/c/src/
git diff HEAD~N -- bindings/c/src/
# Expected: empty output

# ADR-86 boundary: DiagnosticFormatter stays CLI-side
ls crates/nono-cli/src/diagnostic/
# Must include: formatter.rs, mod.rs, etc.
```

**Acceptable state for Cluster B audit.rs APPLY:** The new `SandboxRuntimeAuditEvent` /
`CommandPolicyAuditEvent` structs are additive — they extend `AuditEventPayload` without touching
the `verify_audit_log` function body. The ADR-86 invariant is about the boundary between library and
CLI, not about the specific event types in the library.

### Invariant 3: exec_strategy_windows/ Denial-Rendering Fork (ADR-86 D-03 Carve-Out)

**Files:** `crates/nono-cli/src/exec_strategy_windows/` (specifically `launch.rs` and `network.rs`
which contain Windows denial rendering)

**Verification method:**

```bash
# Confirm the carve-out files are byte-for-byte unchanged
git diff HEAD~N -- crates/nono-cli/src/exec_strategy_windows/launch.rs
git diff HEAD~N -- crates/nono-cli/src/exec_strategy_windows/network.rs
# Both must be empty (no diff)

# Spot-check: NonoError methods diagnostic_code() / remediation() still bridge correctly
grep -n "diagnostic_code\|remediation" crates/nono/src/error.rs | head -5
# Must show method implementations (Phase 86 output)
```

**Note:** ADR-86 confirms this window (v0.64.0..v0.65.1) has NO library-boundary change. The
audit/diagnostics boundary carve-out is at the same location as when Phase 86 established it.

### SC4 — Security-Relevant Will-Sync Commit Notes

Per UPST10-03 SC4, each security-relevant will-sync commit needs a dedicated verification note.
Only Cluster A is security-relevant in this window:

**Cluster A security note:** `9ce74e92` fixes 4 bugs in AF_UNIX pathname mediation (deadlock,
wrong jt offsets, rate-limiter starvation, dup2 bypass). The fix moves IPC handshake BEFORE BPF
filter installation, eliminating fd-based exemptions. The fork's equivalent Windows path
(`exec_strategy_windows/`) is unaffected (AF_UNIX mediation is Linux-only, using Landlock + seccomp
BPF). Verification: after Cluster A lands, `socket_access_run.rs` integration tests in nono-cli
should compile (they're Linux cfg-gated so they won't run on Windows but must not cause compile
errors under `cargo build --workspace`).

---

## PARTIAL→96 Hand-Off Record

Per D-03, cross-target clippy is deferred to Phase 96. The Phase 96 checklist must include:

| Commit | Files with cfg-gated Unix blocks | Phase 96 verification required |
|--------|----------------------------------|-------------------------------|
| `9ce74e92` (Cluster A) | `crates/nono/src/sandbox/linux.rs`, `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` | `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `--target x86_64-apple-darwin` |
| `11fd10e0` (Cluster B, applied hunks) | `crates/nono/src/sandbox/linux.rs`, `crates/nono/src/sandbox/mod.rs` | Same cross-target clippy gates |
| Phase 95 manual commit(s) | Any file where manual edits were needed for conflict resolution | Same cross-target clippy gates |

**Record format for the PARTIAL→96 deferral** (to be placed in each plan's SUMMARY.md under
`## PARTIAL Deferrals`):

```
Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain
(x86_64-unknown-linux-gnu, x86_64-apple-darwin). Commits [9ce74e92, 11fd10e0-applied-hunks]
touch cfg-gated Unix code. The live GH Actions Linux Clippy / macOS Clippy lanes on the
head SHA are the decisive signals per .planning/templates/cross-target-verify-checklist.md.
UPST10-02 marked PARTIAL pending Phase 96 XTGT verification.
```

**Carry-forward PARTIAL→CI items from earlier phases** (ZTL-04 AWS_* strip from v3.2; SEC-01/SEC-02
AF_UNIX guards from v3.1) — Phase 95 does NOT resolve these; Phase 96 is the resolution vehicle.

---

## Common Pitfalls

### Pitfall 1: Straight Cherry-Pick of Cluster B Will Not Compile

**What goes wrong:** `git cherry-pick 11fd10e0` applies the full commit including the
`ApprovalRequest` enum addition to `supervisor/types.rs` and the `exec_strategy.rs` timeout
refactor that removes `wait_for_child_with_startup_timeout`. The fork's code still uses
`CapabilityRequest` and `wait_for_child_with_startup_timeout`, so the cherry-pick produces a
compile error.

**Why it happens:** Cluster B is a 91-file squash commit that assumes upstream's codebase (which has
`ApprovalRequest`, `CredentialProxyIntent`, tool-sandbox plumbing). The fork diverged significantly
from upstream on these internal types.

**How to avoid:** Use `git cherry-pick --no-commit 11fd10e0` and then `git checkout -- <file>` on
files that must be skipped. Stage only the approved hunks. See Cluster B hunk table above.

**Warning signs:** Compile errors mentioning `ApprovalRequest`, `CredentialProxyIntent`,
`tool_sandbox`, `wait_for_child_with_startup_timeout` (if removed), or `tls_intercept`.

### Pitfall 2: Cluster C `network_policy.rs` Change Is a No-Op Conflict

**What goes wrong:** The upstream removes `custom_credentials.keys()` from `all_names` to prevent
auto-activation. The fork's `resolve_credentials()` already returns early on `service_names.is_empty()`
(line 188), so `all_names` was never populated from custom_credentials keys in the fork's code path.
Applying the upstream hunk could either be a no-op or could silently break a test.

**How to avoid:** SKIP the `network_policy.rs` hunk entirely. The fork's behavior is already correct
for the fork's semantics.

### Pitfall 3: Forgetting DCO Sign-Off on Manual Commits

**What goes wrong:** `git cherry-pick -x` automatically adds the upstream author + committer info;
DCO `Signed-off-by` must still be added as a trailer. Manual replay commits have no `-x` trailer at
all if not specified. Missing DCO breaks the signed-off requirement.

**How to avoid:** For cherry-picks: add `--trailer "Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>"`
or use `-s`. For manual commits: always use `git commit -s`. Verify: `git log --format="%B" HEAD | grep Signed-off-by`.

### Pitfall 4: `make ci` vs `make build` + `make test` Sequencing

**What goes wrong:** `make build` passes but `make ci` fails because `cargo fmt --check` catches
unstaged format changes from conflict resolution.

**Why it happens:** Conflict-resolution edits often introduce formatting inconsistencies.

**How to avoid:** Run `make fmt` before `make ci` after every manual edit. The CLAUDE.md note
(memory `feedback_fmt_check_in_verify_gate`) documents this: "clippy green ≠ rustfmt-clean."

### Pitfall 5: Cluster B audit.rs Test Section CapabilityDecision Field Mismatch

**What goes wrong:** If the test section of audit.rs is applied, it tries to add fields like
`endpoint_policy_action: None`, `approval_backend: None` to `CapabilityDecision` struct literals.
The fork's `CapabilityDecision` in `supervisor/types.rs` does NOT have these fields (they're
tool-sandbox machinery added in Cluster B). The compile error looks like "missing field
`endpoint_policy_action` in struct `CapabilityDecision`".

**How to avoid:** SKIP the `@@ -1390` and `@@ -1428` and `@@ -1602` test hunks from the Cluster B
audit.rs diff. Only apply the `@@ -81` (new event types) and `@@ -357` (new recorder methods) hunks.

### Pitfall 6: Boxing NetworkAuditEvent Breaking Existing Call Sites

**What goes wrong:** Cluster B changes `record_network_event(event: NetworkAuditEvent)` to box the
event internally: `AuditEventPayload::Network { event: Box::new(event) }`. This changes the
`AuditEventPayload::Network` variant. Any pattern match on `AuditEventPayload::Network { event }` in
the fork that expects `event: NetworkAuditEvent` (not `Box<NetworkAuditEvent>`) will fail.

**How to avoid:** When applying this hunk, search for ALL `AuditEventPayload::Network` usage:

```bash
grep -rn "AuditEventPayload::Network\|record_network_event" crates/ --include="*.rs"
```

Update the variant definition AND all match arms AND call sites atomically in the same commit.

---

## Code Examples

### Cherry-pick with -x and DCO trailer (Cluster A)

```bash
# Source: fork convention established in Phases 86-89
git cherry-pick -x 9ce74e92
# After resolving any conflicts:
git cherry-pick --continue --no-edit
# Then add DCO (if not already added by --continue):
git commit --amend --trailer "Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>" --no-edit
```

### Baseline capture command

```bash
mkdir -p ci-logs-local/baseline-95
cargo test --workspace 2>&1 | tee ci-logs-local/baseline-95/baseline-before-cherry-picks.txt
grep "FAILED\|test result" ci-logs-local/baseline-95/baseline-before-cherry-picks.txt
```

### Manual hunk extraction for Cluster B

```bash
# Stage the cherry-pick without committing
git cherry-pick --no-commit 11fd10e0

# Reset files that must be skipped entirely
git checkout -- crates/nono/src/supervisor/types.rs
git checkout -- crates/nono-cli/src/exec_strategy.rs
# ... (other skip files)

# For audit.rs: reset the test section only (keep struct additions)
# This requires careful patch editing — use 'git add -p' to select hunks
git add -p crates/nono/src/audit.rs

# Commit with DCO
git commit -s -m "feat(audit): absorb Cluster B shared-surface additions (audit event types, restrict_execute) — split from 11fd10e0

Absorbs additive SandboxRuntimeAuditEvent/CommandPolicyAuditEvent types and
restrict_execute() from upstream 11fd10e0 feat(sandbox): tool sandbox (#1105).
Skips tool-sandbox subsystem dir, tls_intercept/ hunks, ApprovalRequest type
plumbing, and exec_strategy.rs timeout refactor (tool-sandbox machinery absent
in fork).

Cherry-picked from upstream nolabs-ai/nono 11fd10e0
Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>"
```

### Post-cherry-pick fork-invariant spot check

```bash
# Verify exec_strategy_windows/ untouched
git diff HEAD~3 -- crates/nono-cli/src/exec_strategy_windows/
# Expected: (empty)

# Verify CR-02 expression preserved
grep "records_verified" crates/nono/src/audit.rs | grep "event_count"
# Expected: records_verified: event_count > 0

# Run CR-02 guard test
cargo test -p nono --lib -- audit::tests::verify_empty_log_with_no_stored_metadata_is_not_valid
# Expected: test ... ok

# Run Cluster F proxy guard test
cargo test -p nono-cli --lib -- proxy_runtime::tests::proxy_activates_with_custom_credentials_only
# Expected: test ... ok
```

---

## sigstore-verify Dep Bump (Claude's Discretion Evaluation)

[VERIFIED: upstream commit `9e084cbb` inspected; fork's Cargo.toml checked]

Current fork version: `sigstore-verify = "0.8.0"` (`crates/nono/Cargo.toml` line 49).
Upstream bumped to `0.9.0` in commit `9e084cbb` (minor version, semver).

**Recommendation:** Fold into Phase 97 with the rest of Cluster D. Reasons:
1. D-04 (no-new-failures) is unknown until tested — the bump is a semver-minor but changes 179
   Cargo.lock lines. Testing a Cargo.lock change mid-cherry-pick introduces confounding factors.
2. The sigstore-verify dep is used for attestation/signing; a minor-version bump could change API
   surface. Phase 97 (version bump + release prep) is the correct vehicle for this risk surface.
3. No security advisory on 0.8.0 was identified; this is not a RUSTSEC-grade blocker.

If the planner overrides this recommendation, the bump is: edit `crates/nono/Cargo.toml` line 49 to
`sigstore-verify = "0.9.0"` and run `cargo update -p sigstore-verify` to regenerate Cargo.lock.

---

## Environment Availability

[VERIFIED: checked on dev host at research time]

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `git` | cherry-pick operations | ✓ | system git | — |
| `rustup` + Rust 1.82 | `cargo build/test/clippy` | ✓ | 1.82+ (confirmed by prior phases) | — |
| `upstream` remote (`nolabs-ai/nono`) | SHA references | ✓ | configured; objects fetched Phase 94 | — |
| `x86_64-unknown-linux-gnu` target | cross-target clippy (D-03) | PARTIAL | deferred to Phase 96 | PARTIAL→96 per checklist |
| `x86_64-apple-darwin` target | cross-target clippy (D-03) | PARTIAL | deferred to Phase 96 | PARTIAL→96 per checklist |
| `ci-logs-local/baseline-95/` directory | baseline capture | must create | `mkdir -p` | trivial |

**Missing dependencies with no fallback:** None — Phase 95 only requires Windows-native tooling.

**Missing dependencies with fallback:** Cross-target toolchains — PARTIAL→96.

---

## Validation Architecture

`workflow.nyquist_validation: true` in `.planning/config.json` — include this section.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (cargo test) |
| Config file | Cargo.toml (workspace) |
| Quick run command | `cargo test --workspace 2>&1` |
| Full suite command | `make ci` (clippy + fmt + tests) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Notes |
|--------|----------|-----------|-------------------|-------|
| UPST10-02 | All will-sync SHAs absorbed, DCO-signed | manual verify | `git log --oneline HEAD~5..HEAD` | Check `-x` trailer and `Signed-off-by` on each absorb commit |
| UPST10-02 | No new test failures introduced | automated | `cargo test --workspace 2>&1 \| diff - ci-logs-local/baseline-95/baseline-before-cherry-picks.txt` | New failures = blocker |
| UPST10-03 | CR-02 expression preserved | unit | `cargo test -p nono --lib -- audit::tests::verify_empty_log_with_no_stored_metadata_is_not_valid` | Must remain `ok` |
| UPST10-03 | Cluster F proxy guard preserved | unit | `cargo test -p nono-cli --lib -- proxy_runtime::tests::proxy_activates_with_custom_credentials_only` | Must remain `ok` |
| UPST10-03 | exec_strategy_windows/ untouched | manual diff | `git diff HEAD~N -- crates/nono-cli/src/exec_strategy_windows/` | Must be empty |
| UPST10-03 | ADR-86 boundary maintained | manual + unit | `grep "records_verified" crates/nono/src/audit.rs` + CR-02 test | `event_count > 0` must appear |
| UPST10-03 | make build green | automated | `make build` | Zero new compile errors |
| UPST10-03 | make test green (no new failures) | automated | `cargo test --workspace` | 5 known-red baseline allowed; 0 new |
| UPST10-03 | SC4: security-relevant absorb notes documented | manual | checklist in SUMMARY.md | Cluster A AF_UNIX fix note required |

### Sampling Rate

- **Per wave commit:** `cargo test -p nono --lib && cargo test -p nono-cli --lib` (fast, ~30s)
- **Per wave merge:** `make ci` (full: clippy + fmt + tests; ~2-3min on Windows)
- **Phase gate:** Full suite green + fork-invariant checklist all checked before `/gsd:verify-work`

### Wave 0 Gaps

None — existing test infrastructure covers all phase requirements. No new test files required.
The key guard tests (`verify_empty_log_with_no_stored_metadata_is_not_valid`,
`proxy_activates_with_custom_credentials_only`) already exist. The `socket_access_run.rs`
integration tests added by Cluster A are Linux cfg-gated and will not run on Windows (correct;
they will be Phase 96's cross-target concern).

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Not modified in this phase |
| V3 Session Management | no | Not modified in this phase |
| V4 Access Control | yes | Cluster A fixes 4 bugs in AF_UNIX pathname mediation (dup2 bypass, deadlock, wrong jt offsets, rate-limiter starvation) — sandbox access control on Linux |
| V5 Input Validation | no | Not modified in this phase |
| V6 Cryptography | partial | Cluster B adds `cmd://` URI scheme detection in keystore (prevents cmd:// from being loaded directly; redirects to supervisor path) — boundary enforcement |

### Known Threat Patterns for This Absorb

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| dup2 bypass (Cluster A bug 4) | Elevation of Privilege | Fixed: BPF filter installed AFTER IPC handshake; pure allowlist eliminates fd-based exemptions |
| Rate-limiter starvation (Cluster A bug 3) | Denial of Service | Fixed: non-AF_UNIX calls bypass rate limiter in AfUnixOnly mode |
| Proxy activation inversion (Cluster C conflict) | Security Feature Bypass | Preserve fork's fail-secure: `|| !prepared.custom_credentials.is_empty()` in active predicate |
| CR-02 audit bypass (Cluster B hit) | Tampering | Preserve: `records_verified: event_count > 0` — additive types do not touch this line |

---

## Open Questions (RESOLVED)

> All three are **execution-time unknowns** (resolvable only by running git against the live
> tree at cherry-pick time), each with a prescribed mitigation wired into a specific plan task.
> None blocks planning; `cargo check --workspace` in the owning task's acceptance criteria is the
> backstop that catches any miss.

1. **Cluster B `route.rs` `endpoint_policy` field — does the fork compile with the new field?**
   - What we know: `RouteConfig` in `nono-proxy/src/route.rs` is the struct; Cluster B adds
     `endpoint_policy: CompiledEndpointPolicy` field
   - What's unclear: whether existing struct literals elsewhere in the fork create `RouteConfig`
     without the new field (would cause compile error)
   - Recommendation: Before applying the `route.rs` hunk, grep for `RouteConfig {` across the
     codebase and add `endpoint_policy: None` to each site.
   - **RESOLVED:** Plan 95-02 Task 1 Step 6 pre-checks all `RouteConfig {` construction sites and
     adds `endpoint_policy: None` to each before committing; compile failure is caught by
     `cargo check --workspace` in Task 1 acceptance criteria. Resolved at execution time per the
     prescribed pre-check.

2. **Cluster A conflict in `exec_strategy.rs` — exact conflict shape at cherry-pick time**
   - What we know: Both Cluster A and the fork touch `execute_supervised()` in different ways
   - What's unclear: Whether the conflict is cleanly resolvable or requires manual study
   - Recommendation: Run `git cherry-pick -x 9ce74e92` first (possibly with `--strategy-option=theirs`
     then manual restore of fork-divergent lines); inspect `git diff ORIG_HEAD` to understand scope
   - **RESOLVED:** Plan 95-01 Task 2 resolves at cherry-pick time — run `git cherry-pick -x 9ce74e92`,
     accept the upstream IPC-ordering hunks (Cluster A is the security fix; no fork-preserve override
     applies to its mediation logic), restore any fork-divergent surrounding lines, and verify scope
     via `git diff ORIG_HEAD`. Exact conflict shape is determined at cherry-pick time per the
     fork-preserve rule.

3. **NetworkAuditEvent boxing — call site count**
   - What we know: `record_network_event` signature changes to box the event internally
   - What's unclear: How many call sites exist and whether any match on `AuditEventPayload::Network`
   - Recommendation: Run `grep -rn "AuditEventPayload::Network\|record_network_event" crates/` before
     applying this hunk; count the sites; update all atomically
   - **RESOLVED:** Plan 95-02 Task 1 Step 5 greps all `AuditEventPayload::Network` / `record_network_event`
     sites and updates them atomically in the same commit; `cargo check --workspace` in Task 1
     acceptance criteria catches any missed site. Resolved at execution time per the prescribed grep.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `git cherry-pick 11fd10e0` will fail to compile due to `ApprovalRequest`/`CapabilityRequest` mismatch and `wait_for_child_with_startup_timeout` removal | Conflict Surface Map, Pitfall 1 | Low risk — the structural mismatch is clearly visible in the diff and fork code |
| A2 | Cluster C `network_policy.rs` hunk is a no-op for the fork (fork already returns early on `service_names.is_empty()`) | Cluster C conflict surface | Medium risk — if the fork's behavior differs from analysis, applying the skip could miss a needed fix |
| A3 | sigstore-verify 0.9.0 bump should fold into Phase 97 (no RUSTSEC advisory on 0.8.0) | sigstore-verify section | Low risk — no RUSTSEC found; recommendation is conservative |

**All critical claims in this research were verified via direct code inspection or `git show`/`git log` commands. No unverified claims drive the split strategy.**

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Cherry-pick all of large squash commits | Manual hunk extraction for split-disposition commits | Phase 85+ (established pattern) | Required for Cluster B given tool-sandbox machinery absent in fork |
| `will-sync` all security fixes | Split disposition for commits that also contain non-syncable machinery | Phase 85 (DR-09 protocol) | Cluster B and C both get split disposition |

---

## Sources

### Primary (HIGH confidence)

- `git show 9ce74e92 --stat` and `git show 9ce74e92 -- <file>` — Cluster A file list and diff content [VERIFIED: live git commands]
- `git show 11fd10e0 --stat` and per-file diffs — Cluster B split surface analysis [VERIFIED: live git commands]
- `git show 9b37dc52` and per-file diffs — Cluster C conflict analysis [VERIFIED: live git commands]
- `cargo test --workspace` at HEAD `0e3d1a68` — baseline-red test set [VERIFIED: live test run]
- `crates/nono-cli/src/proxy_runtime.rs` (fork HEAD) — confirmed fork structural shape of ProxyLaunchOptions [VERIFIED: Read tool]
- `crates/nono/src/audit.rs` (fork HEAD) — confirmed `records_verified: event_count > 0` at line 1435 [VERIFIED: Read + grep]
- `git remote -v` — upstream remote confirmed as `nolabs-ai/nono` [VERIFIED: live command]
- `git cat-file -t 9ce74e92 / 11fd10e0 / 9b37dc52` — all three SHAs present in local object store [VERIFIED: live commands]
- `.planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md` — cluster routing and per-file tables [VERIFIED: Read tool]
- `.planning/phases/95-upstream-absorb-fork-invariant-verify/95-CONTEXT.md` — locked decisions D-01..D-04 [VERIFIED: Read tool]
- `proj/ADR-86-library-boundary-convergence.md` — invariant definitions [VERIFIED: Read tool]
- `proj/ADR-87-cr02-audit-bypass.md` — CR-02 expression definition [VERIFIED: Read tool]

### Secondary (MEDIUM confidence)

- Cluster B `route.rs` apply guidance — based on diff inspection; exact compatibility with fork's `RouteConfig` usages not fully enumerated (Open Question 1)
- Cluster A `exec_strategy.rs` conflict shape — inferred from diff inspection; exact conflict resolution may require trial at cherry-pick time (Open Question 2)

---

## Metadata

**Confidence breakdown:**
- SHA inventory and reachability: HIGH — verified via `git cat-file -t`
- Cluster A conflict analysis: HIGH — diff inspected; supervisor IPC change is clear
- Cluster B split guidance: HIGH for skip decisions; MEDIUM for exact apply hunks (route.rs, server.rs need compile verification)
- Cluster C split guidance: HIGH — structural incompatibility (CredentialProxyIntent absent in fork) is clear from code inspection
- Baseline-red test set: HIGH — verified via live test run
- Fork-invariant verification methods: HIGH — exact file paths and grep commands verified

**Research date:** 2026-06-26
**Valid until:** 2026-07-26 (stable domain; fork state is a git ref that doesn't change)
