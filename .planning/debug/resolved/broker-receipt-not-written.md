---
status: resolved
trigger: "CR-06 (Phase 118 code review): the broker writes no enforcement receipt on the successful launch path"
created: 2026-09-06
updated: 2026-09-06
severity: critical
requirement: RCPT-01
---

# Debug: broker writes no receipt on the success path

## ⚠ READ FIRST — re-baseline before investigating

**All evidence below was gathered at commit `2a66143d`. HEAD is now `aa48cdbb`, and three
fix commits since then touched exactly this code path:**

- `c731c4e0` (CR-02) — rewrote `BrokerReceiptWriter::new` to read the existing segment and resume
  the chain, failing closed on an unparseable tail.
- `b555c392` (CR-03) — `NONO_SESSION_ID` is now pushed onto `broker_env_pairs`, so the broker no
  longer sees an empty session id.
- `aa48cdbb` (CR-04) — `broker_receipt_sink_dir()` now returns `Result` and REJECTS a base that is
  owned by the current user.

**Your first action must be to reproduce at current HEAD, not to trust the evidence below.** The
behavior may have changed, and CR-04 in particular introduces a new fail-closed path that did not
exist when the measurements were taken. If the symptom is gone, that is itself the finding — say so
rather than hunting for a cause that no longer applies.

## Symptoms

**Expected:** RCPT-01 requires every confined session to emit a receipt. D-15 specifies TWO receipts
per broker-arm session — `nono.exe`'s own, plus the broker's `<session_id>.broker.jsonl`.
`record_broker_receipt`'s own doc comment (`crates/nono-shell-broker/src/main.rs:746-750`) claims it
is "Called at EVERY point [`run`] decides `Ran` or `Refused`, so a receipt exists before the child
could ever execute an instruction."

**Actual:** Zero `*.broker.jsonl` files exist anywhere on disk. Each broker session produced exactly
ONE receipt, from `nono.exe`:

```
b20cf38788b791ba.jsonl | entry_path=DirectCli token_arm=BrokerLaunchNoPty outcome=Ran
dbedffbeb593558d.jsonl | entry_path=DirectCli token_arm=BrokerLaunchNoPty outcome=Ran
efe66db47bf1b65f.jsonl | entry_path=DirectCli token_arm=BrokerLaunchNoPty outcome=Ran
6d66a008cb7b6af9.jsonl | entry_path=DirectCli token_arm=BrokerLaunchNoPty outcome=Ran
```

**Errors:** NONE — and that silence is the core anomaly. No `tracing::warn!`, no error output, in a
run whose broker `INFO` lines were reaching the console normally.

**Timeline:** Broker receipt code landed 2026-08-16 (`c7a237ef`, Plan 118-06). Never observed
working. Measured broken across 5 consecutive sessions on 2026-09-05/06. Whether it EVER worked is
unknown — no prior run is recorded.

**Reproduction** (real PowerShell — git-bash/MSYS fails broker runs with GLE=87):

```powershell
mkdir "$env:USERPROFILE\nono-probe" -Force | Out-Null
cd "$env:USERPROFILE\nono-probe"
& C:\Users\OMack\Nono\target\debug\nono.exe run -p claude-code --allow-cwd -- cmd /c echo hi
Get-ChildItem "$env:PROGRAMDATA\nono\receipts" -Filter *.broker.jsonl   # expect: 0 items = bug reproduced
```

`-p claude-code` selects `BrokerLaunchNoPty` via the profile's `windows_low_il_broker: true`.
Confirm the broker arm was actually taken by looking for `broker: spawned child app_container=true`
in the output — without that line you exercised a different arm and the result means nothing.

## The contradiction to resolve

1. `record_broker_receipt` (`main.rs:758`) **was called** — see Eliminated #5.
2. Every failure path inside it emits a `tracing::warn!` — none appeared.
3. Success creates the file via `OpenOptions::create(true).append(true)` — no file exists.

All three cannot hold simultaneously. One of the three assumptions is wrong; find which.

## Eliminated hypotheses (do not re-investigate without new evidence)

- hypothesis: Stale broker binary (the trap from quick task `260815-gfd` Cluster C, where a
  June-dated `nono-shell-broker.exe` hid a fail-closed rejection for 2.5 months)
  evidence: `target/debug/nono-shell-broker.exe` is dated 2026-09-04 23:56; broker source last
  changed 2026-08-16 (`c7a237ef`). Binary is 19 days NEWER than the source.
  status: ELIMINATED

- hypothesis: The broker resolves a different sink directory than `nono.exe`
  evidence: `broker_receipt_sink_dir()` (`main.rs:566-569`) read `PROGRAMDATA` with the identical
  `unwrap_or_else(|_| r"C:\ProgramData")` fallback as `resolve_sink_dir()`. Even a cleared child
  environment lands on the same literal path. A filesystem-wide
  `find /c/ProgramData /c/Users/OMack/AppData -name "*.broker.jsonl"` returned nothing.
  status: ELIMINATED (but re-verify — `aa48cdbb` changed this function)

- hypothesis: The receipt code is behind a Cargo feature not enabled in the debug build
  evidence: `crates/nono-shell-broker/Cargo.toml` has one feature (`layer-fault-injection`, for
  fault tests). `serde`/`serde_json` are unconditional dependencies. No `#[cfg(feature = ...)]`
  guards the receipt module.
  status: ELIMINATED

- hypothesis: The `tracing::warn!` fires but is filtered out, hiding a real failure
  evidence: `record_broker_receipt` is inside `mod broker` (declared `main.rs:35`) — the SAME module
  whose `INFO` lines appear in console output. The subscriber uses
  `EnvFilter::try_from_default_env()` falling back to `EnvFilter::new("info")` (`main.rs:2741-2747`),
  and `warn` >= `info`, so a warning would print. A run with `NONO_LOG=debug` also showed no
  receipt-related output whatsoever.
  status: ELIMINATED

- hypothesis: The `Ran` emission at `main.rs:1448` is on a branch not taken by the AppContainer /
  no-PTY arm
  evidence: There is NO `fn` boundary between line 1448 and line 1493 (`grep '^    fn '` over
  1100-1500 returns nothing), so both are in the same function body at the same nesting depth. Line
  1493 logs `"broker: spawned child"`, which DID appear in every run — so control flow passed
  through 1448 first.
  status: ELIMINATED

## Suggested next actions

1. **Reproduce at current HEAD** (see the re-baseline warning above). If fixed, identify which of
   `c731c4e0`/`b555c392`/`aa48cdbb` fixed it and why — a silent fix is still a finding worth
   understanding, because it means we did not know what was broken.
2. If still broken, instrument `record_broker_receipt` directly — a `tracing::error!` (or
   `eprintln!`) at entry, after the guard call, and after the write — to establish which of the
   three assumptions above is false. This is the cheapest way to break the contradiction.
3. Check whether the broker process has self-degraded to Low integrity by the time it reaches
   line 1448. The broker "self-degrades to Low IL and spawns the actual sandboxed child" — if the
   degradation happens BEFORE the receipt write, a Low-IL process writing into the sink may be
   refused. Note the sink's label is Low with mask `0x6` (`NO_READ_UP | NO_EXECUTE_UP`, no
   `NO_WRITE_UP`), so the label itself should not block a write — but the DACL and the process's
   restricted token are separate questions worth measuring rather than reasoning about.
4. Confirm whether `ensure_broker_receipt_sink_guarded` succeeds. It early-`return`s on failure with
   only a warning, and it is the first thing `record_broker_receipt` does.

## Constraints (project rules — non-negotiable)

- This is a security-critical sandboxing codebase. Fail secure on any error; never silently degrade.
- No `.unwrap()`/`.expect()` outside `#[cfg(test)]` — enforced by `-D clippy::unwrap_used`.
- Crate names are `nono-sandbox` / `nono-sandbox-cli` / `nono-shell-broker`. **`-p nono` and
  `-p nono-cli` DO NOT RESOLVE and a wrong selector EXITS 0 HAVING RUN ZERO TESTS.** Assert
  "exit 0 AND test count >= 1" on every test claim.
- Known pre-existing baseline: `-p nono-sandbox-cli --bin nono` = 1729 passed / 12 failed. Three
  integration tests fail for reasons already adjudicated as pre-existing. Do not chase these.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` is
  GREEN at HEAD. If you make it red, you broke it.
- Do NOT modify `.planning/STATE.md`, `.planning/ROADMAP.md`, or `.planning/REQUIREMENTS.md`.
- Do NOT run any `gsd-sdk` state writer.
- Commits need a DCO sign-off: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`

## Current Focus

ROOT CAUSE CONFIRMED (2026-09-05, at HEAD `aa48cdbb`).

reasoning_checkpoint:
  hypothesis: >
    The broker's `%PROGRAMDATA%` is NOT the machine-wide `C:\ProgramData`. `nono-cli` hands the
    broker the CONFINED CHILD's sanitized environment (`broker_env_pairs = env_pairs.clone()`,
    `launch.rs:2263`), in which `append_windows_runtime_env` has rewritten
    `PROGRAMDATA = <runtime_root>\programdata` (`launch.rs:940`). The broker resolves its receipt
    sink from that variable, so it wrote `<session>.broker.jsonl` into
    `<workdir>\.nono-runtime\programdata\nono\receipts` — never into `C:\ProgramData\nono\receipts`.
  confirming_evidence:
    - "Live run at HEAD printed: `%PROGRAMDATA% (C:\\Users\\OMack\\nono-probe\\.nono-runtime\\programdata) is owned by the current user` — the broker's own resolver naming the redirected path."
    - "`C:\\Users\\OMack\\nono-probe\\.nono-runtime\\programdata\\nono\\receipts\\unknown-session.broker.jsonl` EXISTS on disk with exactly 4 `entry_path=Broker outcome=Ran` records — one per reported session."
    - "`launch.rs:2263` `broker_env_pairs = env_pairs.clone()`; `env_pairs` comes from `build_child_env`, which strips real PROGRAMDATA (skip list, `launch.rs:759`) and re-adds the redirect (`launch.rs:940`)."
    - "Broker spawns its child with `lpEnvironment = NULL` (`main.rs:1253`) — the broker's env IS the confined child's env, so the redirect is deliberate and load-bearing."
  falsification_test: >
    If this were false, no `*.broker.jsonl` would exist under any `.nono-runtime\programdata` tree,
    and the broker's warn would have named `C:\ProgramData` rather than the workdir-local path.
  fix_rationale: >
    The broker must learn the real, validated ProgramData base over a channel the confined-child env
    redirection does not touch. Restoring real `PROGRAMDATA` on `broker_env_pairs` is NOT viable — it
    would un-redirect the confined child too. Passing it as a broker command-line argument keeps it
    out of the child's environment entirely and matches how every other security-relevant broker
    input (`--app-container-name`) is already delivered.
  blind_spots: >
    Only the `BrokerLaunchNoPty` arm was exercised live; `BrokerLaunch` (PTY) shares the same
    `broker_env_pairs` construction so it is affected identically, but was not measured.

WHICH ASSUMPTION WAS FALSE: #3. The file DID exist — the search looked in the wrong directory.
Eliminated hypothesis #2 was retired on faulty evidence: it assumed a *cleared* child environment
would land on the same literal fallback path. The environment is not cleared, it is REWRITTEN.

next_action: implement the `--receipt-sink-base` wire-contract fix (broker + nono-cli), then re-run
the live repro and confirm a `*.broker.jsonl` appears in `C:\ProgramData\nono\receipts`.

## Evidence

- timestamp: 2026-09-05T21:06Z
  observation: Broker session with `--allow $sink` printed the receipt contents and exited 0
  (`app_container=true`, `child_exit_code=0`), confirming the broker arm was genuinely taken, while
  no `*.broker.jsonl` was created.

- timestamp: 2026-09-05T22:03Z
  checked: Rebuilt `nono.exe` + `nono-shell-broker.exe` at HEAD `aa48cdbb` (both binaries' mtimes
  now post-date the HEAD commit time 20:57).
  found: The pre-existing `nono.exe` on disk (17:57) PREDATED all four CR fix commits, so every
  measurement in the prior session's evidence was taken against a nono.exe without CR-01..CR-04.
  implication: Re-baseline was mandatory, as the session file warned.

- timestamp: 2026-09-05T22:11Z
  checked: Live repro at HEAD from PowerShell (`Start-Process -WorkingDirectory %USERPROFILE%\nono-probe`,
  `nono.exe run -p claude-code --allow-cwd -- cmd /c echo hi`).
  arm_observed: `broker: spawned child child_pid=76828 app_container=true` — the BrokerLaunchNoPty
  arm WAS exercised. `broker: child exited child_exit_code=0`.
  found: The broker emitted a WARN that the prior session never saw:
  `broker: failed to resolve a trustworthy receipt sink directory — degrading visibly (D-04)`
  `error=... %PROGRAMDATA% (C:\Users\OMack\nono-probe\.nono-runtime\programdata) is owned by the
  current user ...`
  implication: DECISIVE. The broker's `%PROGRAMDATA%` is the sandbox-local redirect, not
  `C:\ProgramData`. CR-04 (`aa48cdbb`) did not fix the bug — it made the pre-existing misdirection
  VISIBLE for the first time by rejecting the user-owned redirected base.

- timestamp: 2026-09-05T22:12Z
  checked: `ls -laR C:\Users\OMack\nono-probe\.nono-runtime\programdata`
  found: `nono\receipts\unknown-session.broker.jsonl`, 4084 bytes, 4 JSONL records, all
  `entry_path=Broker outcome=Ran`, pids 70124/53592/66900/55916 — one per reported session. All
  carry `sequence=0` (the CR-02 defect) and `session_id=""` (the CR-03 defect), hence the
  `unknown-session` filename.
  implication: The receipts were ALWAYS being written. Assumption #3 of the contradiction ("no file
  exists") was the false one. Eliminated hypothesis #2 was retired on faulty evidence — the prior
  filesystem search covered `/c/ProgramData` and `/c/Users/OMack/AppData` but NOT the workdir.

- timestamp: 2026-09-05T22:15Z
  checked: `launch.rs:2263` / `build_child_env` (`launch.rs:731-859`) /
  `append_windows_runtime_env` (`launch.rs:940`) / broker `main.rs:1253`.
  found: `broker_env_pairs = env_pairs.clone()` where `env_pairs` is the CONFINED CHILD's sanitized
  environment: `PROGRAMDATA` is on the strip list (`launch.rs:759`) and re-added as
  `runtime_root.join("programdata")` (`launch.rs:940`). The broker spawns its own child with
  `lpEnvironment = NULL` — so the broker's environment IS the confined child's environment.
  implication: The redirect on the broker's env is load-bearing for the sandbox. Restoring real
  `PROGRAMDATA` on `broker_env_pairs` would un-redirect the confined child and is NOT an acceptable
  fix. The real base must reach the broker over a non-environment channel.

- timestamp: 2026-09-05T22:08Z
  checked: Harness artifact worth recording — `nono` prefers `$PWD` over `current_dir()`
  (`sandbox_prepare.rs:403-415`). git-bash exports `PWD`, which leaks through `powershell.exe` into
  `nono.exe` and overrides `-WorkingDirectory`.
  found: Without `Remove-Item Env:PWD`, `--allow-cwd` grants the git-bash cwd, and the run aborts
  with "execution directory outside supported allowlist" (or hangs labelling the whole repo).
  implication: Any future repro script for this bug MUST clear `PWD`/`OLDPWD` first.

- timestamp: 2026-09-06T00:12Z
  observation: Run with `NONO_LOG=debug` produced broker `INFO` lines
  (`console attach probe`, `AppContainer profile registered`, `token/AppContainer setup complete`,
  `spawned child`, `child exited`) with NO receipt/sink/warn output between `spawned child` and
  `child exited` — a 15ms window in which the emission should have occurred.

## Eliminated

(see "Eliminated hypotheses" section above — 5 entries)

**RETRACTION — Eliminated #2 was wrong.** "The broker resolves a different sink directory than
`nono.exe`" was the CORRECT hypothesis. It was retired on faulty evidence: the reasoning assumed the
broker's environment was *cleared* (so `PROGRAMDATA` would be absent and the `C:\ProgramData`
fallback would apply). It is not cleared — it is REWRITTEN, to the confined child's sandbox-local
redirect. The confirming filesystem search also did not cover the workdir, which is where the
receipts actually were.

## Resolution

root_cause: >
  `nono-cli` builds the broker's environment as a clone of the CONFINED CHILD's sanitized
  environment (`exec_strategy_windows/launch.rs`, `broker_env_pairs = env_pairs.clone()`), in which
  `append_windows_runtime_env` has rewritten `PROGRAMDATA` to the sandbox-local redirect
  `<runtime_root>\programdata`. `nono-shell-broker`'s `broker_receipt_sink_dir()` resolved its
  receipt sink from that variable, so every `<session_id>.broker.jsonl` was written to
  `<workdir>\.nono-runtime\programdata\nono\receipts` instead of `C:\ProgramData\nono\receipts`.
  Before CR-04 the misdirected write SUCCEEDED silently (hence no warning and no file where anyone
  looked); after CR-04 the user-owned redirected base is correctly rejected, which finally made the
  long-standing misdirection visible. The redirect could not simply be undone on the broker's
  environment: the broker spawns its own child with `lpEnvironment = NULL`, so the broker's
  environment IS the confined child's environment.

fix: >
  Move the receipt-sink base off the environment and onto argv. `nono-cli` resolves and validates
  the machine-wide `%ProgramData%` root ONCE (new `receipt_sink::resolve_sink_base()`, which
  `resolve_sink_dir()` now also routes through, so there is a single validated resolution) and
  passes it to the broker as `--receipt-sink-base` on BOTH broker arms via a new
  `push_broker_receipt_sink_base_arg` helper. `nono-shell-broker` gains a
  `BrokerArgs::receipt_sink_base` field and `broker_receipt_sink_dir` now takes the base as a
  parameter — it can no longer read the environment at all, and FAILS CLOSED (no fallback) when the
  flag is absent. CR-04's absolute-path and not-owned-by-current-user validation is retained on the
  broker side as defense in depth. argv is the correct channel because `nono.exe` is its only
  writer and, unlike an environment variable, it is not inherited by the confined child.

verification: >
  Live run at HEAD+fix, broker arm confirmed by `broker: spawned child app_container=true`:
  `C:\ProgramData\nono\receipts\ff41046fd73df08c.broker.jsonl` (and, after the follow-up refactor,
  `5754259428069866.broker.jsonl`) now exist, each correlated with `nono.exe`'s own
  `<same-session-id>.jsonl` — D-15's two-receipts-per-session pair, satisfying RCPT-01. Broker
  receipt content: `entry_path=Broker outcome=Ran`, 13-row layer census, real session id (no longer
  `unknown-session`). No degrade warning emitted.
  Gates: `cargo clippy --workspace --all-targets --all-features -D warnings -D clippy::unwrap_used`
  GREEN; `cargo fmt --all --check` GREEN; `-p nono-shell-broker --bin nono-shell-broker` 50 passed
  / 0 failed; `-p nono-sandbox-cli --bin nono` 1730 passed / 12 failed (baseline 1729/12 — exactly
  +1 for the new test, same 12 known pre-existing failures).
  Regression tests (7 new): broker-side fail-closed-without-base, reject-relative-base,
  reject-user-owned-base, join-shape, and two argv round-trip tests; plus a CLASS-level nono-cli
  test asserting that `build_child_env`'s `PROGRAMDATA` is a redirect AND that the broker argv
  therefore carries the real base out-of-band. That last test was perturbation-proved: neutering
  `push_broker_receipt_sink_base_arg` makes it FAIL with
  `left: [] right: ["--receipt-sink-base", "C:\ProgramData"]`.

files_changed:
  - crates/nono-shell-broker/src/main.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/receipt_sink.rs

known_gaps:
  - The PTY arm (`WindowsTokenArm::BrokerLaunch`) received the identical fix but was NOT exercised
    live — only `BrokerLaunchNoPty` was. Both arms build `broker_env_pairs` the same way, so the
    defect and the fix apply identically, but this is reasoning, not measurement.
  - The four pre-fix receipts remain stranded at
    `C:\Users\OMack\nono-probe\.nono-runtime\programdata\nono\receipts\unknown-session.broker.jsonl`.
    No migration is attempted; they are evidence, not production data.

## RESOLUTION (2026-09-06, commit `4d0c5ded`)

root_cause: `broker_env_pairs` cloned the CONFINED CHILD's sanitized environment, in which
`append_windows_runtime_env` had rewritten `PROGRAMDATA` to the per-workdir runtime redirect. The
broker resolved its sink to `<workdir>\.nono-runtime\programdata\nono\receipts` and wrote there.
Assumption #3 of "the contradiction to resolve" was the false one — the file existed, in a directory
nobody searched. **Eliminated hypothesis #2 is RETRACTED: it was correct, and was retired on faulty
evidence** (it assumed the child env was inherited-unchanged or cleared; it was rewritten, and the
confirming filesystem sweep never covered the workdir).

fix: sink base moved off the environment onto argv (`--receipt-sink-base`), pushed on BOTH arms via
`push_broker_receipt_sink_base_arg` (`launch.rs:2579` PTY, `:2930` no-PTY).
`broker_receipt_sink_dir()` can no longer read the environment and fails closed with no fallback.

verification: live on BOTH arms, measured not by construction.
  - BrokerLaunchNoPty — D-15 pair `5754259428069866` written to `%PROGRAMDATA%`.
  - BrokerLaunch (PTY) — `token_arm=BrokerLaunch` confirmed in the supervisor receipt;
    `60990aea032cfab8.jsonl` + `60990aea032cfab8.broker.jsonl` both present;
    `nono receipt verify` exit 0 with BOTH segments chain-intact; negative control clean.
  - Gates: clippy `--workspace --all-targets --all-features` exit 0; fmt clean;
    `-p nono-shell-broker` 50+2 passed; `--bin nono receipt` 36 passed;
    full `--bin nono` 1730 passed / 12 failed (documented baseline held).

files_changed: crates/nono-cli/src/exec_strategy_windows/launch.rs,
crates/nono-cli/src/receipt_sink.rs, crates/nono-shell-broker/src/main.rs

## Residual items (NOT fixed by this session)

1. Four pre-fix receipts stranded under `nono-probe\.nono-runtime\programdata\nono\receipts\`
   (`unknown-session.broker.jsonl`, 4 records) — no migration attempted.
2. `nono shell` interactive rendering appears broken on this host: the child spawns and the broker
   blocks in `WaitForSingleObject`, but keystrokes do not echo. Orthogonal to this bug — the receipt
   is written before `ResumeThread`, which is why full evidence was obtainable from a shell that
   never rendered. Not a Phase 118 item; logged so it does not evaporate.
3. Harness note: `nono` prefers `$PWD` over `current_dir()` (`sandbox_prepare.rs:403-415`), and
   git-bash's exported `PWD` leaks through PowerShell and overrides `-WorkingDirectory`. Any repro
   script must `Remove-Item Env:PWD` first.
