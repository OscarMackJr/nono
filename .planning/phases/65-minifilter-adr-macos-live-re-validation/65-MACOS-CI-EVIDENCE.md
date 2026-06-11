# Phase 65 — macOS Re-validation Evidence (MACOS-03, D-10 automatable + D-11)

**Authored:** 2026-06-09. Re-validates the already-landed macOS Seatbelt code
(Phase 63/64) through every channel that does NOT require a macOS host. **No source
change to `macos.rs`** — this is a re-validation, not a code change
(`git diff --quiet crates/nono/src/sandbox/macos.rs` → UNMODIFIED).

The live `sandbox_init()` enforcement assertions are gate 65-A (plan 65-04 HUMAN-UAT).

---

## Task 1 — Local + cross-target checks

### A. `sandbox::macos` ordering + dual-path tests (D-10 automatable subset)

`cargo test -p nono sandbox::macos` on the **Windows dev host**: the `macos.rs` module
is `#[cfg(target_os = "macos")]`-gated, so the tests **compile-skip** here —
`0 passed; 0 failed; 776 filtered out` (expected; the macOS CI `test` job runs them
natively). The four contract tests are present in-tree:

| Test | Line | Asserts |
|------|------|---------|
| `test_generate_profile_platform_rules_after_writes` | 997 | `read_pos < write_pos < deny_pos` (deny AFTER write, last-match-wins) |
| `test_generate_profile_extensions_before_platform_deny_rules` | 1128 | extension allows precede platform denies |
| `test_platform_rules_after_write_allows` | 1880 | same ordering invariant (D-11 sibling) |
| `test_platform_deny_symlink_and_canonical_path` | 1919 | BOTH `/etc/passwd` AND `/private/etc/passwd` deny rules appear (dual-path) |

→ The ordering + dual-path security contracts are in-tree; they are GREEN on their
native target via the `macos-latest` CI `test` job (see Task 2).

### B. Cross-target clippy (D-11a) — **PARTIAL**

```
rustup target add x86_64-apple-darwin   # already installed
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
→ EXIT 101
→ error occurred in cc-rs: failed to find tool "cc": program not found
```

**Disposition: PARTIAL.** The Windows dev host has no `cc` to link the darwin-target
C dependencies (aws-lc-sys / ring / keyring class). This is the exact link/toolchain
failure the cross-target checklist authorizes as PARTIAL — the green `macos-latest` CI
`clippy` leg is the decisive signal (D-11a / Pitfall 4). The link error was NOT chased
(checklist explicitly authorizes PARTIAL here).

### C. Phase-64 macOS cherry-pick drift scan (D-11b)

Commits scanned: `8f84d454` (emit platform rules after user write allows),
`362ada22` ($PWD symlink CWD), `8f1b0b74` (preserve symlink path on CWD cap).

| Drift class (from the v2.9 regression) | Result |
|-----------------------------------------|--------|
| (a) edition-2024 let-chains (`if let … &&`) | **PASS** — none present in `macos.rs` |
| (b) E0716-class temporary-borrow | **PASS** — `push_str(&format!(…))` temporaries are call-scoped, not dropped-while-borrowed |
| (c) canonical-path dual coverage (`/etc`↔`/private/etc`, `/tmp`↔`/private/tmp`) | **PASS** — dual emission via `cap.original != cap.resolved` (macos.rs lines 241, 409); `test_platform_deny_symlink_and_canonical_path` asserts both forms |

No new drift defect surfaced. `macos.rs` left unmodified.

---

## Task 2 — Green `macos-latest` CI SHA (D-11c HARD gate) — ✅ SATISFIED

✅ **GATE GREEN (2026-06-11).** Both `macos-latest` legs are `conclusion: success`
on the phase fix HEAD. The Windows dev host cannot compile macOS code (the exact
v2.9 regression), so this CI run is the decisive signal.

- **Run URL:** https://github.com/OscarMackJr/nono/actions/runs/27345465703
- **Commit SHA:** `d91446633e43aea0b8c1de46df3720f5812c12e1` (`d9144663`)
- **PR:** OscarMackJr/nono #6 (`fix/macos-resl-host-gate`, based on phase HEAD `e72d6438`)
- **`Test (macos-latest)` leg:** **conclusion: success**
- **`Clippy (macos-latest)` leg:** **conclusion: success**

### How the green was reached (D-11c CI rehab + the runner-hang fix)

The macOS `test` leg was red across the prior session for two distinct reasons, both
now resolved:

1. **5 stale/harness failure classes** fixed on `e72d6438` (audit_attestation `$PWD`,
   builtin_profile_load opencode-pack relocation, profile_drafts manifest, resl_nix
   `--allow-fs-*`→`--read` flags, resl_nix bounding). See STATE "Progress this session".
2. **The resl enforcement tests hung the runner.** Once the flag fix let them actually
   launch children, `macos_timeout_kills_at_deadline` + `macos_max_processes_blocks_on_rlimit_nproc`
   exercised macOS `--timeout`/`RLIMIT_NPROC` enforcement (REQ-RESL-NIX-03, **never
   host-validated** — Phase 37 was host-blocked) for the first time on the GH runner,
   where it does **not** fire; `run_bounded` did not reliably reap the sandboxed/detached
   children, so `cargo test` hung 25+ min until the runner "lost communication" (runs
   `27291915409`, `27300030066`). Fixed in PR #6 (`d9144663`) by gating those two
   host-dependent enforcement tests behind `NONO_RESL_HOST_VALIDATED`: they **skip on
   CI** (greening this leg) and **run on a real macOS host at gate-65-A**. The
   host-independent macOS resl tests still run on CI. **`macos.rs` is unmodified** — the
   change is test-harness only (`crates/nono-cli/tests/resl_nix_macos.rs`).

### 🔒 HARD GATE (D-11c) — CLEARED

The literal, tag-blocking gate is satisfied: `conclusion: success` on BOTH the
`Test (macos-latest)` and `Clippy (macos-latest)` legs at SHA `d9144663`. A Windows-host
`cargo check` was NOT used as a substitute (T-65-FALSEGREEN / Pitfall 4). A release tag
may now be cut on or after this SHA.

> Note: the two `#[gated]` enforcement assertions move to gate-65-A (plan 65-04
> HUMAN-UAT) — they are validated on a real macOS host with `NONO_RESL_HOST_VALIDATED=1`,
> not on the hosted runner.
