---
phase: 50-corp-network-tuf-refresh
fixed_at: 2026-05-22T00:00:00Z
review_path: .planning/phases/50-corp-network-tuf-refresh-via-os-root-store-replace-or-wrap-t/50-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 50: Code Review Fix Report

**Fixed at:** 2026-05-22
**Source review:** `.planning/phases/50-corp-network-tuf-refresh-via-os-root-store-replace-or-wrap-t/50-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope (Critical + Warning): 5
- Fixed: 5
- Skipped: 0

All five WARNING findings from the Phase 50 code review were addressed. No CRITICAL findings existed. INFO findings (IN-01..IN-04) were out of scope (`fix_scope: critical_warning`) and remain as tracked maintainability concerns.

Verification protocol applied to each commit: `cargo build -p nono-cli`, `cargo clippy -p nono-cli --no-deps -- -D warnings -D clippy::unwrap_used`, and `cargo test -p nono-cli --bin nono trust_refresh` (6 hermetic tests). A final round of `cargo test -p nono-cli --test setup_trust_root` (10 tests, 1 network-gated ignored) also passed end-to-end.

## Fixed Issues

### WR-01: Path A skips the trusted_root freshness gate that Path B enforces

**Files modified:** `crates/nono-cli/src/setup.rs`
**Commit:** `b91fb1f0`
**Applied fix:** Added a call to `nono::trust::bundle::check_trusted_root_freshness(&trusted_root, &cache_path)` inside `refresh_trust_root_step`, between the `block_on(refresh_production_trusted_root())` call and the `serde_json::to_string_pretty` + `std::fs::write` cache-write sequence. The error is wrapped as `NonoError::Setup("fetched Sigstore trusted root failed freshness check: {e}")` to distinguish it from the `--from-file` variant ("Sigstore trusted root at {path} failed freshness check: {e}"). Path A now enforces the same D-32-03 tlog `validFor.end` contract as Path B, closing the asymmetric supply-chain gap.

### WR-02: Datastore cleanup on transient failures defeats TUF incremental caching

**Files modified:** `crates/nono-cli/src/trust_refresh.rs`
**Commit:** `d439533d` (combined with WR-03 per review guidance)
**Applied fix:** Added `let datastore_existed = tokio::fs::metadata(&datastore_dir).await.is_ok();` BEFORE `tokio::fs::create_dir_all(&datastore_dir).await`, and gated the cleanup arm on `!datastore_existed`. Pre-existing tuf-cache directories now survive transient failures (network glitches, proxy 403s, ureq timeouts), preserving tough's incremental-update state across invocations. The original D-50-07 fail-clean intent is preserved for brand-new datastores created in the same invocation. Module-level docstring updated to record the new semantics.

### WR-03: Sync remove_dir_all in async context blocks the tokio executor

**Files modified:** `crates/nono-cli/src/trust_refresh.rs`
**Commit:** `d439533d` (combined with WR-02 per review guidance — "Combine with WR-02's conditional cleanup for full effect")
**Applied fix:** Replaced `std::fs::remove_dir_all(&datastore_for_cleanup)` with `tokio::fs::remove_dir_all(&datastore_for_cleanup).await`. The cleanup now uses the same async file-system API as the surrounding `tokio::fs::create_dir_all` and `tokio::fs::metadata` calls. Multi-threaded runtimes (future GUI/TUI wrappers) no longer block their executor threads on directory removal, and the consistent async contract makes the function safer for any future caller.

### WR-04: `refresh_trust_root_phase_index` is shared by two distinct steps

**Files modified:** `crates/nono-cli/src/setup.rs`, `crates/nono-cli/tests/setup_trust_root.rs`
**Commit:** `8ccdb6aa`
**Applied fix:** Renamed `refresh_trust_root_phase_index` to `trust_root_provisioning_phase_index` so the dual-purpose nature is visible at the call sites in `refresh_trust_root_step` and `from_file_step`. Added a `debug_assert!(!(self.refresh_trust_root && self.from_file.is_some()), ...)` at the top of the method body to encode the mutual-exclusion contract that previously lived only in clap (`conflicts_with = "refresh_trust_root"` in cli.rs:2382). Updated the doc comment to explain the invariant and the failure mode if the clap declaration is ever loosened. Updated one stale comment in `tests/setup_trust_root.rs:234` that referenced the old method name. Release builds compile out the assert, so the original behavior is preserved on the hot path.

### WR-05: tokio JoinError surfaces as opaque `TransportErrorKind::Other`

**Files modified:** `crates/nono-cli/src/trust_refresh.rs`
**Commit:** `4479ed22`
**Applied fix:** Applied the "better still" variant from the review (standard `tokio::task::spawn_blocking` pattern). Replaced the catch-all `Err(e) => return Err(TransportError::new_with_cause(TransportErrorKind::Other, ..., e))` with two arms: `Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic())` propagates panics in the blocking task to the supervisor's panic hook (yielding a real stack trace), and `Err(e) => { tracing::error!(...); panic!(...) }` panics loudly on cancellation (which cannot occur in normal operation while we await the JoinHandle synchronously). Either path is now structurally distinct from "actual TLS / proxy / network failure" in the user-visible output — an operator following the corp-network caveats in `windows-poc-handoff.mdx` will no longer mistake an internal bug for a proxy issue.

---

_Fixed: 2026-05-22_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
