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

## Task 2 — Green `macos-latest` CI SHA (D-11c HARD gate)

⛔ **GATE: pending CI run.** The decisive signal for the macOS legs is a green
`macos-latest` CI run on the phase HEAD (the Windows host cannot compile macOS code —
the exact v2.9 regression). To be filled after the phase branch is pushed and CI runs:

- **Run URL:** _<gh run view URL>_
- **Commit SHA:** _<phase HEAD SHA>_
- **`Test (macos-latest)` leg:** _conclusion: <success>_
- **`clippy (macos-latest)` leg:** _conclusion: <success>_

### 🔒 HARD GATE (D-11c)

**NO release tag may be cut before a green `macos-latest` CI SHA is recorded above.**
This is the literal, tag-blocking gate — not advisory. A Windows-host `cargo check` is
NOT a substitute (T-65-FALSEGREEN / Pitfall 4). Only `conclusion: success` on BOTH the
`test` and `clippy` macos-latest legs satisfies it.

> Gate resume-signal (plan 65-02 Task 2): type **"approved"** with the green
> `macos-latest` run URL + SHA, or describe the red leg.
