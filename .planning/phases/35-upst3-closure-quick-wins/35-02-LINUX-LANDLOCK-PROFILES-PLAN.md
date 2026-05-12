---
phase: 35-upst3-closure-quick-wins
plan: 02
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/nono-cli/src/profile_runtime.rs
autonomous: true
requirements:
  - REQ-PORT-CLOSURE-06
tags:
  - phase-35
  - port-closure
  - linux
  - landlock
  - p34-defer-09-1
  - d-19-cherry-pick

must_haves:
  truths:
    - "On first-run Linux invocation of `nono run` (clean install with `~/.config/nono/profiles/` absent), the profiles directory is created BEFORE Landlock applies its ruleset."
    - "The confusing `No such file or directory` error referencing `~/.config/nono/profiles/` is eliminated on Linux first-run."
    - "On macOS and Windows hosts, the pre-create hunk is a compile-time no-op (gated behind `#[cfg(target_os = \"linux\")]`); no observable behavior change."
    - "The pre-create call uses `crate::config::user_profiles_dir()` (XDG-aware fork helper, line 286 of `user.rs`) — NOT `dirs::home_dir()` (STATE.md Windows blocker)."
    - "Failures from `std::fs::create_dir_all` propagate via `From<io::Error> for NonoError` using `?` — no `.unwrap()` / `.expect()` per CLAUDE.md."
  artifacts:
    - path: "crates/nono-cli/src/profile_runtime.rs"
      provides: "New `#[cfg(target_os = \"linux\")]` helper function `pre_create_landlock_profiles_dir()` (or inline block) inserted into `prepare_profile` BEFORE the function returns the `PreparedProfile`. Calls `crate::config::user_profiles_dir()?` + `std::fs::create_dir_all(&path)?`."
      contains: "user_profiles_dir"
    - path: "crates/nono-cli/src/profile_runtime.rs"
      provides: "Linux-gated integration test `#[cfg(target_os = \"linux\")]` `#[test]` (or `#[cfg(all(test, target_os = \"linux\"))]` mod) that exercises the pre-create idempotency — runs in CI Linux lane, marked `#[ignore]` for Windows-host runs per D-35-D3."
      contains: "fn test_pre_create_landlock_profiles_dir_idempotent"
  key_links:
    - from: "crates/nono-cli/src/profile_runtime.rs::prepare_profile"
      to: "crates/nono-cli/src/config/user.rs::user_profiles_dir"
      via: "function call resolving `~/.config/nono/profiles/` via XDG-aware path resolution"
      pattern: "crate::config::user_profiles_dir\\(\\)"
    - from: "crates/nono-cli/src/profile_runtime.rs::prepare_profile (Linux only)"
      to: "std::fs::create_dir_all"
      via: "idempotent directory creation BEFORE the caller (sandbox_prepare.rs:298) builds the Landlock CapabilitySet"
      pattern: "std::fs::create_dir_all"
---

<objective>
Cherry-pick upstream `bdf183e9` (v0.44.0) `fix(package): harden re-pulls against user edits` — extracting ONLY the 15-line `profile_runtime.rs` Landlock pre-create hunk; the remaining 188/239 lines of upstream's `wiring.rs` work are deferred to Phase 36 (REQ-PORT-CLOSURE-04). The hunk pre-creates `~/.config/nono/profiles/` BEFORE Landlock applies its ruleset, eliminating the confusing `No such file or directory` first-run UX bug on Linux. Closes P34-DEFER-09-1.

**Purpose:** Landlock is strictly allow-list (cannot express deny-within-allow); it requires the parent directory of any granted child path to exist at ruleset apply time, even when the child path is explicitly granted write. Pre-creating the profiles directory before `restrict_self()` resolves the resolved path and locks it cleanly.

**Output:** A ~15-line `#[cfg(target_os = "linux")]` block in `crates/nono-cli/src/profile_runtime.rs::prepare_profile` that calls `crate::config::user_profiles_dir()?` and `std::fs::create_dir_all(&path)?`. One Linux-gated integration test verifying idempotency. The cherry-pick commit carries the verbatim D-19 6-line trailer block per D-35-A4.

**D-19 commit shape (per D-35-A4):** This is the ONLY Phase 35 commit with the full D-19 trailer block (verbatim shape from `.planning/templates/upstream-sync-quick.md`):
```
Upstream-commit: bdf183e9
Upstream-tag: v0.44.0
Upstream-author: <upstream author from `git show bdf183e9 --format=%an <%ae>'`>
Co-Authored-By: <same as Upstream-author>
Signed-off-by: <fork author full name> <email>
Signed-off-by: <fork author github handle> <email>
```
Lowercase `'a'` in `Upstream-author:`. Smoke check at close: `git log --format='%B' main~1..main | grep -c '^Upstream-commit: '` equals 1.

**Scope ceiling (D-35-A1 / D-34-B2):** ONLY the 15-line pre-create hunk. NO touching of upstream `bdf183e9`'s `wiring.rs` (188/239 LOC), NO touching of fork's `crates/nono-cli/src/wiring.rs` shim (Phase 36 territory), NO Landlock ruleset changes in `crates/nono/src/sandbox/linux.rs`.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/35-upst3-closure-quick-wins/35-CONTEXT.md
@.planning/phases/35-upst3-closure-quick-wins/35-PATTERNS.md
@.planning/templates/upstream-sync-quick.md
@docs/cli/development/upstream-drift.mdx

<interfaces>
<!-- Existing fork helpers Plan 35-02 calls into. Do not re-implement. -->

From `crates/nono-cli/src/config/user.rs` lines 286-292:
```rust
/// Get the path to user profiles directory
pub fn user_profiles_dir() -> Result<PathBuf> {
    let config_dir = super::user_config_dir().ok_or_else(|| {
        NonoError::ConfigParse("Could not determine user config directory".to_string())
    })?;

    Ok(config_dir.join("profiles"))
}
```

From `crates/nono-cli/src/profile_runtime.rs` lines 1-10 + 123-145 (existing `#[cfg(target_os = "linux")]` gate shape to mirror):
```rust
use crate::cli::SandboxArgs;
use crate::{hooks, profile};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) struct PreparedProfile {
    pub(crate) loaded_profile: Option<profile::Profile>,
    pub(crate) capability_elevation: bool,
    #[cfg(target_os = "linux")]
    pub(crate) wsl2_proxy_policy: profile::Wsl2ProxyPolicy,
    // ...
}

pub(crate) fn prepare_profile(
    args: &SandboxArgs,
    silent: bool,
    workdir: &Path,
) -> crate::Result<PreparedProfile> {
    // ... existing body returning Ok(PreparedProfile { ... })
}
```

Existing `From<std::io::Error> for NonoError` impl (in `crates/nono/src/error.rs`) propagates `io::Error` from `create_dir_all` via `?` cleanly — no manual `map_err` needed.

Existing in-tree analog at `crates/nono-cli/src/profile/builtin.rs` lines 447-451 (the canonical fork `create_dir_all` shape):
```rust
profiles_dir: &std::path::Path,
// ...
std::fs::create_dir_all(profiles_dir)?;
```
</interfaces>

</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Cherry-pick bdf183e9 — pre-create profiles dir before Landlock apply (Linux-only hunk)</name>
  <files>crates/nono-cli/src/profile_runtime.rs</files>
  <read_first>
    - crates/nono-cli/src/profile_runtime.rs (lines 1-160 — full picture of imports, `PreparedProfile` struct, `prepare_profile` function entry)
    - crates/nono-cli/src/config/user.rs (lines 280-310 — `user_profiles_dir` helper signature + return type)
    - crates/nono-cli/src/profile/builtin.rs (lines 440-455 — existing fork `create_dir_all` shape using `?` propagation)
    - crates/nono/src/sandbox/linux.rs (lines 850-880 — `restrict_self` apply site, to understand WHY pre-create must precede the ruleset)
    - .planning/phases/35-upst3-closure-quick-wins/35-PATTERNS.md (Pattern Assignments § "`profile_runtime.rs` (Plan 35-02)" — explicit Analog 1 + 2 + 3 callouts)
    - .planning/templates/upstream-sync-quick.md § "D-19 cherry-pick trailer block" — verbatim 6-line shape with lowercase `'a'` in `Upstream-author:`
    - Run `git show bdf183e9 -- crates/nono-cli/src/profile_runtime.rs` (in fork repo with `upstream` remote configured) to inspect the original 15-line hunk shape before transcribing
  </read_first>
  <behavior>
    - On Linux, calling `prepare_profile(...)` ensures `~/.config/nono/profiles/` exists on disk by the time the function returns `PreparedProfile`.
    - On macOS and Windows, the new code is a compile-time no-op (`#[cfg(target_os = "linux")]`).
    - Calling `prepare_profile` twice in succession on Linux does NOT error on the second call (idempotency — `create_dir_all` succeeds on already-existent directories).
    - If `user_profiles_dir()` returns `Err(NonoError::ConfigParse(...))` (cannot determine `$XDG_CONFIG_HOME` / `$HOME`), the error propagates via `?`.
    - If `create_dir_all` fails with `io::Error` (e.g., permission denied, parent doesn't exist), the error propagates via the `From<io::Error> for NonoError` impl.
  </behavior>
  <action>
    1. Ensure the fork repo has `upstream` remote pointing at `https://github.com/always-further/nono.git`. If not: `git remote add upstream https://github.com/always-further/nono.git && git fetch upstream`. Then verify access to `bdf183e9`: `git show bdf183e9 --stat`. Confirm the commit touches `crates/nono-cli/src/profile_runtime.rs` (Plan 35-02 picks up ONLY that hunk; upstream's `wiring.rs` changes are deferred to Phase 36 per REQ-PORT-CLOSURE-04).
    2. Attempt a focused cherry-pick: `git cherry-pick -n bdf183e9 -- crates/nono-cli/src/profile_runtime.rs`. The `-n` flag stages without committing; the path-filter form restricts to the one file. If git complains about the path-filter form on the available git version, fall back to:
       - `git show bdf183e9 -- crates/nono-cli/src/profile_runtime.rs > /tmp/bdf183e9-profile_runtime.patch`
       - `git apply --3way --reject /tmp/bdf183e9-profile_runtime.patch` (then resolve any `.rej` rejects manually)
    3. Inspect the staged hunk. Upstream's commit body / metadata may reference fork-divergent symbols (e.g., upstream might call a different path helper, or upstream may not have `user_profiles_dir` at all and may inline `dirs::home_dir().unwrap().join(...)`). The fork MUST:
       - Use `crate::config::user_profiles_dir()` (XDG-aware fork helper) instead of any `dirs::home_dir()` call upstream may have used (per CONTEXT § Deferred — `dirs::home_dir()` is the STATE.md Windows blocker; Plan 35-02 stays Windows-friendly even though the hunk is Linux-gated).
       - Use `?` propagation, NEVER `.unwrap()` / `.expect()` (CLAUDE.md § Coding Standards).
       - Wrap the entire new code in `#[cfg(target_os = "linux")]` so macOS and Windows are compile-time no-ops.
    4. The final shape (insert into `prepare_profile`, early in the function body BEFORE the `Ok(PreparedProfile { ... })` return), using a clearly-named helper:
       ```rust
       /// Plan 35-02 (REQ-PORT-CLOSURE-06 / P34-DEFER-09-1): cherry-pick of
       /// upstream `bdf183e9` (v0.44.0) — pre-create `~/.config/nono/profiles/`
       /// BEFORE the caller (sandbox_prepare.rs:298 → Sandbox::apply →
       /// landlock::restrict_self) locks the filesystem ruleset. Landlock is
       /// strictly allow-list and requires the parent directory of any
       /// granted child path to exist at ruleset-apply time, even when the
       /// child path is explicitly granted write. Without this pre-create,
       /// first-run `nono run` on a clean install (with `~/.config/nono/`
       /// missing) produces a confusing `No such file or directory` error
       /// pointing at the profiles path.
       ///
       /// macOS and Windows are compile-time no-ops (Seatbelt and Windows
       /// Job-Object sandbox have no equivalent restriction).
       #[cfg(target_os = "linux")]
       fn pre_create_landlock_profiles_dir() -> crate::Result<()> {
           let dir = crate::config::user_profiles_dir()?;
           std::fs::create_dir_all(&dir)?;
           Ok(())
       }
       ```
       Then INSIDE `prepare_profile` (line 123+), as the first action:
       ```rust
       #[cfg(target_os = "linux")]
       pre_create_landlock_profiles_dir()?;
       ```
    5. Run `cargo build -p nono-cli --target x86_64-unknown-linux-gnu` (or on Linux host: `cargo build -p nono-cli`) and verify clean compile. Also run on Windows host: `cargo build -p nono-cli` — expect NO compile error since both the helper and the call site are `#[cfg(target_os = "linux")]`.
    6. Stage ONLY `crates/nono-cli/src/profile_runtime.rs`. Verify via `git diff --cached --stat` that the staged change touches ONLY that one file. (Reject the cherry-pick if any other file slips in.)
    7. Compose the commit message:
       - **Subject:** `feat(35-02): pre-create profiles dir before Landlock apply (Linux)`
       - **Body:** Brief description of the fix + reference to closed deferral:
         ```
         Plan 35-02 (REQ-PORT-CLOSURE-06): cherry-pick of upstream bdf183e9
         (v0.44.0) — pre-create ~/.config/nono/profiles/ before Landlock
         ruleset apply. Closes P34-DEFER-09-1.

         Upstream's bdf183e9 commit also touched crates/nono-cli/src/wiring.rs
         (188/239 LOC of the upstream diff); the fork does not carry that
         file (Phase 36 REQ-PORT-CLOSURE-04 will absorb the wiring abstraction).
         Plan 35-02 scope is strictly the 15-line profile_runtime.rs hunk.
         ```
       - **Trailer block** (exactly one blank line between body and trailer; verbatim 6-line shape from `.planning/templates/upstream-sync-quick.md` § D-19 cherry-pick trailer block; lowercase `'a'` in `Upstream-author:`):
         ```
         Upstream-commit: bdf183e9
         Upstream-tag: v0.44.0
         Upstream-author: <Author Name> <author@example.com>
         Co-Authored-By: <Author Name> <author@example.com>
         Signed-off-by: <Fork Author Full Name> <fork@example.com>
         Signed-off-by: <fork-author-handle> <fork@example.com>
         ```
       Resolve `<Author Name>` / `<author@example.com>` from `git show bdf183e9 --format='%an <%ae>'`. Resolve fork-author signoff lines from existing recent commits on `main` (use `git log --format='%B' -1 main | tail -2` for the latest example).
    8. Commit: `git commit` (interactive — paste the composed message body + trailer block). Verify trailer shape:
       - `git log --format='%B' -1 main | grep -c '^Upstream-commit: '` returns 1.
       - `git log --format='%B' -1 main | grep -c '^Upstream-author: '` returns 1 (lowercase `'a'`).
       - `git log --format='%B' -1 main | grep -c '^Upstream-tag: v0.44.0'` returns 1.
       - `git log --format='%B' -1 main | grep -c '^Signed-off-by: '` returns 2.
  </action>
  <acceptance_criteria>
    - `grep -c 'fn pre_create_landlock_profiles_dir' crates/nono-cli/src/profile_runtime.rs` returns 1.
    - `grep -c 'cfg(target_os = "linux")' crates/nono-cli/src/profile_runtime.rs` increases by at least 2 vs pre-plan baseline (one on the helper, one on the call site; the existing `wsl2_proxy_policy` gates already in the file account for the baseline value).
    - `grep -c 'user_profiles_dir' crates/nono-cli/src/profile_runtime.rs` returns at least 1.
    - `grep -c 'dirs::home_dir' crates/nono-cli/src/profile_runtime.rs` returns 0 (no `dirs::home_dir()` call — STATE.md Windows blocker avoidance).
    - `grep -c '\.unwrap()\|\.expect(' crates/nono-cli/src/profile_runtime.rs` does NOT grow vs pre-plan baseline (CLAUDE.md no-unwrap policy).
    - `grep -c 'Plan 35-02 (REQ-PORT-CLOSURE-06' crates/nono-cli/src/profile_runtime.rs` returns at least 1.
    - `git diff --stat HEAD~1 HEAD --` shows exactly 1 file modified: `crates/nono-cli/src/profile_runtime.rs`. Verify with `git diff --stat HEAD~1 HEAD -- | grep -c '^.* | '` equals 1.
    - `git log --format='%B' -1 main | grep -c '^Upstream-commit: bdf183e9'` returns 1 (D-19 trailer present, abbreviated 8-char SHA per template § D-19 field rule 6).
    - `git log --format='%B' -1 main | grep -cE '^Upstream-author: '` returns 1 (lowercase `'a'`; verifies template § D-19 field rule 5).
    - `git log --format='%B' -1 main | grep -c '^Upstream-tag: v0.44.0'` returns 1.
    - `git log --format='%B' -1 main | grep -c '^Signed-off-by: '` returns 2 (DCO + GitHub attribution per template § D-19 field rule 4).
    - <automated>git log --format='%B' -1 main | grep -c '^Upstream-commit: bdf183e9'</automated>
  </acceptance_criteria>
  <verify>
    <automated>cargo build -p nono-cli &amp;&amp; git log --format='%B' -1 main | grep -c '^Upstream-commit: bdf183e9'</automated>
  </verify>
  <done>Pre-create hunk landed on `main` with verbatim D-19 6-line trailer block; exactly one file modified; no `dirs::home_dir()` introduced; no `.unwrap()` introduced; compiles clean on Windows + Linux + macOS (gated appropriately).</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Add Linux-gated integration test locking idempotency + first-run behavior</name>
  <files>crates/nono-cli/src/profile_runtime.rs</files>
  <read_first>
    - crates/nono-cli/src/profile_runtime.rs (the just-committed Task 1 changes plus the file's existing test module; if there is no existing `#[cfg(test)] mod tests` block in this file, look at the existing Plan 34-08a regression tests referenced in `34-08a-ENV-SURFACE-PORT-SUMMARY.md` § key_files.modified to confirm test-module location)
    - crates/nono-cli/src/config/user.rs (lines 280-310 — `user_profiles_dir()` to understand the path it returns + test-controllable env vars `XDG_CONFIG_HOME` / `HOME`)
    - CLAUDE.md § "Environment variables in tests" — save/restore guard pattern for `HOME` / `XDG_CONFIG_HOME` manipulation (tests run in parallel; an unrestored env var flakes unrelated tests)
    - .planning/phases/35-upst3-closure-quick-wins/35-CONTEXT.md § "D-35-D3" — defines CI Linux lane as the functional verification surface for Plan 35-02; dev-host Windows runs SKIP this test
  </read_first>
  <behavior>
    - `test_pre_create_landlock_profiles_dir_idempotent` (gated `#[cfg(target_os = "linux")]`):
      - Sets `XDG_CONFIG_HOME` to a fresh `tempfile::TempDir` via the save/restore guard from CLAUDE.md.
      - Calls `pre_create_landlock_profiles_dir()` — asserts `Ok(())`.
      - Asserts `<temp>/nono/profiles/` exists on disk via `std::path::Path::is_dir`.
      - Calls `pre_create_landlock_profiles_dir()` a second time — asserts `Ok(())` (idempotency).
      - Drops the env-var guard (which restores prior `XDG_CONFIG_HOME` / removes it if unset) and the `TempDir` (which cleans up the test directory).
    - Test is `#[cfg(target_os = "linux")]`; on Windows host it does not compile (no test to run); on Linux host it runs unconditionally in `cargo test`. CI Linux lane is the functional verification surface per D-35-D3.
    - NO `#[ignore]` attribute on the test (per D-35-D3 the Linux lane runs it; Windows-host clippy cross-target check + the existing local `cargo test --workspace --all-features` is what verifies it doesn't break Linux compile during the Windows dev cycle — the cross-target Linux clippy gate D-35-D2 step 3 catches drift).
  </behavior>
  <action>
    1. Locate or create the test module in `crates/nono-cli/src/profile_runtime.rs`. If a `#[cfg(test)] mod tests { ... }` block exists (from Plan 34-08a per the summary which lists 2 regression tests in `profile_runtime::tests`), add the new test inside it. If not, create a new `#[cfg(test)] mod tests { ... }` at the bottom of the file.
    2. Inside the test module, add an env-var save/restore guard struct (or reuse one if present from Plan 34-08a's regression tests):
       ```rust
       struct EnvGuard {
           key: String,
           prior: Option<std::ffi::OsString>,
       }

       impl EnvGuard {
           fn set(key: &str, value: &std::path::Path) -> Self {
               let prior = std::env::var_os(key);
               // Safety per CLAUDE.md: set_var is sound in single-threaded test setup;
               // the Drop impl unwinds the change before parallel tests resume.
               std::env::set_var(key, value);
               Self { key: key.to_string(), prior }
           }
       }

       impl Drop for EnvGuard {
           fn drop(&mut self) {
               match self.prior.take() {
                   Some(val) => std::env::set_var(&self.key, val),
                   None => std::env::remove_var(&self.key),
               }
           }
       }
       ```
    3. Add the test:
       ```rust
       /// Plan 35-02 (REQ-PORT-CLOSURE-06): regression test locking the
       /// idempotent + first-run-creates-dir invariant for the Landlock
       /// pre-create hunk. Runs in CI Linux lane (D-35-D3); compile-time
       /// no-op on Windows/macOS.
       #[cfg(target_os = "linux")]
       #[test]
       fn test_pre_create_landlock_profiles_dir_idempotent() {
           let tmp = tempfile::TempDir::new().expect("create tempdir");
           let _xdg_guard = EnvGuard::set("XDG_CONFIG_HOME", tmp.path());

           // First call — creates ~/.config/nono/profiles/
           super::pre_create_landlock_profiles_dir()
               .expect("first pre-create call must succeed on clean fixture");
           let expected = tmp.path().join("nono").join("profiles");
           assert!(
               expected.is_dir(),
               "Expected profiles dir at {} after first pre-create call",
               expected.display(),
           );

           // Second call — idempotent
           super::pre_create_landlock_profiles_dir()
               .expect("second pre-create call must succeed (idempotent on existing dir)");
           assert!(expected.is_dir(), "Profiles dir should still exist after second call");
       }
       ```
       Note: `.expect(...)` is permitted INSIDE test modules per CLAUDE.md § "Unwrap Policy" exception clause; the `clippy::unwrap_used` lint allows test code. Verify the existing tests in the file use `.expect(...)` rather than `?` — if they propagate via `?`, mirror that shape instead.
    4. On Windows host: verify `cargo test -p nono-cli --lib profile_runtime` compiles AND the test module is absent from the test list (because of `#[cfg(target_os = "linux")]`). Expected output: any pre-existing Linux-gated test module shows as empty on Windows.
    5. Cross-target Linux clippy gate (D-35-D2 step 3): `cargo clippy -p nono-cli --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`. Must exit 0.
    6. Cross-target Linux test build: `cargo test -p nono-cli --no-run --target x86_64-unknown-linux-gnu` — must compile (the test linker invocation may fail on Windows without a Linux-gnu linker installed; that's acceptable. The clippy gate is the load-bearing check).
    7. CI Linux lane verification: the actual functional run happens in CI; Plan 35-02 close on Windows requires CI Linux lane green for the new test (per D-35-D3). The plan SUMMARY records the CI run URL.
    8. Commit the test addition as a SEPARATE follow-up commit (since the Task 1 commit was a pure cherry-pick of upstream's 15 lines; the test is fork-local). Commit message:
       - **Subject:** `test(35-02): add Linux-gated idempotency test for pre-create hunk`
       - **Body:** Cite REQ-PORT-CLOSURE-06 + D-35-D3 (CI Linux lane verification rule).
       - **Trailer:** REGULAR DCO sign-off only (no D-19 trailer — this is fork-local test code, not an upstream cherry-pick).
  </action>
  <acceptance_criteria>
    - `grep -c 'fn test_pre_create_landlock_profiles_dir_idempotent' crates/nono-cli/src/profile_runtime.rs` returns 1.
    - `grep -c 'cfg(target_os = "linux")' crates/nono-cli/src/profile_runtime.rs` increases vs Task 1 baseline by at least 1 (the new test gate).
    - `grep -c 'struct EnvGuard' crates/nono-cli/src/profile_runtime.rs` returns 1 (or reuses an existing Plan 34-08a guard — verify by reading the existing test module).
    - On Windows host: `cargo build -p nono-cli` exits 0; `cargo test -p nono-cli --lib profile_runtime --no-run` exits 0.
    - On Windows host: `cargo clippy -p nono-cli --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` exits 0 (cross-target Linux clippy gate — D-35-D2 step 3).
    - `git log --format='%B' -1 main | grep -c '^Upstream-commit: '` returns 0 for THIS commit (regular DCO sign-off only; the D-19 trailer is exclusive to Task 1's cherry-pick commit).
    - `git log --format='%B' -2 main | grep -c '^Upstream-commit: bdf183e9'` returns 1 (Task 1's commit still carries the trailer; Task 2's commit does not).
    - <automated>cargo build -p nono-cli &amp;&amp; cargo clippy -p nono-cli --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used</automated>
  </acceptance_criteria>
  <verify>
    <automated>cargo build -p nono-cli</automated>
  </verify>
  <done>Linux-gated idempotency test added; compiles clean on Windows; clippy clean cross-target Linux + macOS. Test committed as a follow-up to Task 1's cherry-pick with regular DCO sign-off (no D-19 trailer). CI Linux lane URL recorded in plan SUMMARY.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| supervisor pre-sandbox → filesystem | The supervisor writes `~/.config/nono/profiles/` BEFORE Landlock locks the ruleset. The resolved path is fixed at the create-dir moment. |
| Symlink target on `XDG_CONFIG_HOME` / `HOME` → resolved path | If `$XDG_CONFIG_HOME` or `$HOME` is a symlink, the resolved target of `user_profiles_dir()` is the path Landlock locks. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-35-02-01 | Tampering (TOCTOU) | `pre_create_landlock_profiles_dir` → Landlock apply | mitigate | Directory creation runs in the supervisor BEFORE `Sandbox::apply()` → `restrict_self()`. Landlock then resolves the path and locks the ruleset against the resolved inode. A symlink swap between `create_dir_all` and `restrict_self` would either (a) hit the kernel's already-cached inode (path locked), or (b) the resolved target diverges from the operator's intent — but the operator-controlled path resolution is XDG-aware (`user_profiles_dir`) and uses fork-canonical resolution, not raw `dirs::home_dir()`. |
| T-35-02-02 | Information disclosure | Symlinked profiles dir | accept | If an attacker controls `$XDG_CONFIG_HOME` (e.g., a misconfigured shell), they could point it at a different directory before `nono run`. This is a pre-existing trust assumption of the entire XDG-aware path resolution — the new pre-create hunk does not change the threat model; it only ensures the resolved directory exists. |
| T-35-02-03 | Denial of service | `create_dir_all` permission denied | accept | If the user's filesystem layout makes `~/.config/nono/profiles/` un-creatable (e.g., read-only mount, immutable bit), `create_dir_all` returns `io::Error::PermissionDenied`, propagates as `NonoError`, and the supervisor exits cleanly. This is the desired fail-secure behavior — Landlock cannot operate without the directory. |
| T-35-02-04 | Elevation of privilege | Pre-sandbox write outside Landlock | accept | The `create_dir_all` runs in the supervisor process BEFORE the sandbox is applied. The supervisor itself is unsandboxed and trusted (this is the architecture). The pre-create operation is bounded by the operator's filesystem permissions — no elevation. |

</threat_model>

<verification_criteria>

## Phase 34 Close-Gate (D-35-D2 inherited verbatim — all 8 steps)

1. `cargo test --workspace --all-features` (Windows host) exits 0.
2. `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) exits 0.
3. `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` exits 0. **Load-bearing for Plan 35-02** (per D-35-D2): the new code lives ENTIRELY behind `#[cfg(target_os = "linux")]`, which Windows-host `cargo clippy` cannot lint. The cross-target Linux clippy gate is the only Windows-host check that catches drift in the Linux-gated hunk.
4. `cargo clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` exits 0 (symmetric coverage; ensures macOS is genuinely a no-op).
5. `cargo fmt --all -- --check` exits 0.
6. Phase 15 5-row detached-console smoke gate — NOT applicable to Plan 35-02 (Linux-only hunk; no Windows behavior change). Document as `N/A — Linux-only hunk` in SUMMARY.
7. `wfp_port_integration` test suite — passes or documented-skipped (no `wfp` surface touched; expect skip-due-to-no-change).
8. `learn_windows_integration` test suite — passes or documented-skipped (no Windows learn surface touched; expect skip-due-to-no-change).

## Plan-Specific Verification (D-35-A4 D-19 commit-shape gates)

- **PSV-1:** `git log --format='%B' main~2..main | grep -c '^Upstream-commit: bdf183e9'` returns exactly 1 (Task 1's cherry-pick commit carries the D-19 trailer; Task 2's test-addition commit does NOT).
- **PSV-2:** `git log --format='%B' main~2..main | grep -cE '^Upstream-author: '` returns 1 (lowercase `'a'` per template § D-19 field rule 5).
- **PSV-3:** `git log --format='%B' main~2..main | grep -c '^Upstream-tag: v0.44.0'` returns 1.
- **PSV-4:** `git log --format='%B' -1 main~1 | grep -c '^Signed-off-by: '` returns 2 (Task 1's commit has the two-Signed-off-by DCO + GitHub attribution shape per template § D-19 field rule 4).
- **PSV-5:** `git diff --stat HEAD~2 HEAD~1 -- | grep -c '^.* | '` returns 1 (Task 1 cherry-pick touched exactly one file: `crates/nono-cli/src/profile_runtime.rs` — no `wiring.rs` slip-through).
- **PSV-6:** `git diff --stat HEAD~2 HEAD -- crates/nono-cli/src/wiring.rs` returns empty (NO touching of `wiring.rs` — that's Phase 36 REQ-PORT-CLOSURE-04 territory).
- **PSV-7:** CI Linux lane URL recorded in SUMMARY; the run includes the green `test_pre_create_landlock_profiles_dir_idempotent` result (per D-35-D3 — Linux functional verification is CI-side).

## Acceptance Criteria Mapping (REQ-PORT-CLOSURE-06)

1. ✓ First-run `nono run` on Linux pre-creates `~/.config/nono/profiles/` before Landlock ruleset apply — covered by Task 1 wiring + Task 2 idempotency test.
2. ✓ Confusing "no such file or directory" error eliminated — covered transitively: Task 1 ensures the directory exists by the time Landlock evaluates the filesystem rules.

</verification_criteria>

<success_criteria>

- Plan 35-02 closes P34-DEFER-09-1 (Plan SUMMARY records the closure; consolidated `deferred-items.md` append belongs to Plan 35-03 per D-35-D4).
- One cherry-pick commit on `main` with verbatim D-19 6-line trailer (`Upstream-commit: bdf183e9`, lowercase `'a'`).
- One follow-up test-addition commit with regular DCO sign-off (no D-19 trailer).
- Exactly ONE file modified across both commits: `crates/nono-cli/src/profile_runtime.rs`.
- No `wiring.rs` edits, no Landlock ruleset edits in `crates/nono/src/sandbox/linux.rs`, no `dirs::home_dir()` calls.
- Cross-target Linux clippy + macOS clippy gates green (D-35-D2 steps 3 + 4 — load-bearing for the Linux-gated hunk).
- CI Linux lane run shows green `test_pre_create_landlock_profiles_dir_idempotent`.
- No `.unwrap()` / `.expect()` introduced in production code (test code may use `.expect()` per CLAUDE.md exception clause).

</success_criteria>

<output>
After completion, create `.planning/phases/35-upst3-closure-quick-wins/35-02-LINUX-LANDLOCK-PROFILES-SUMMARY.md` with:
- Frontmatter recording the two commit SHAs (Task 1 cherry-pick, Task 2 test-addition), `Upstream-commit: bdf183e9` trailer presence on Task 1's commit, CI Linux lane run URL with green status for `test_pre_create_landlock_profiles_dir_idempotent`.
- Body documenting: scope-trim (only 15-line `profile_runtime.rs` hunk; upstream's `wiring.rs` deferred to Phase 36), D-19 trailer mechanics (lowercase `'a'`, two Signed-off-by lines), close-gate disposition for each of the 8 D-35-D2 steps (steps 6/7/8 documented as `N/A — Linux-only hunk`).
- Closure-section ledger entry: marks P34-DEFER-09-1 as `closed-by-Phase-35-02`.
</output>
