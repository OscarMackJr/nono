# Phase 65 — macOS Live `sandbox_init()` HUMAN-UAT (gate 65-A, MACOS-03 D-10 live)

**Status:** ⛔ **OPEN / BLOCKS PHASE CLOSE.** No macOS host is confirmed available at
authoring time (2026-06-09). This checklist stages the four live `sandbox_init()` deny
assertions that only a real macOS host can confirm. It is **close-blocking**: gate 65-A
stays OPEN until run on a real macOS host. **Do NOT mark any item `pass` without real
host output** — Windows/CI evidence is NOT a substitute (Pitfall 4 / T-65-NOHOST).

**Host:** _<hostname>_  |  **macOS version:** _<sw_vers>_  |  **Date:** _<run date>_

The unit tests + CI prove the Seatbelt profile *generation* contract; only a real macOS
host proves the OS actually *enforces* the denies. These four assertions are the runtime
confirmation of what `crates/nono/src/sandbox/macos.rs` asserts at compile time.

> Build the workspace on the host first (`make build` / `cargo build -p nono-cli`).

---

## Assertion 1 — Dry-run deny-after-allow ordering

Mirrors `macos.rs` lines ~1026–1030 (`read_pos < write_pos < deny_pos`, last-match-wins).

```
nono run --dry-run --profile claude-code
```

**Expected:** `file-write*` allow rules emitted, THEN `platform (deny ...)` lines AFTER
them (deny overrides the preceding write-allows).

**Result:** [ ] pass  /  [ ] blocked

```
<paste dry-run output here — confirm deny lines appear after the write-allow lines>
```

---

## Assertion 2 — SSH private key blocked

```
nono run --profile claude-code -- cat ~/.ssh/id_rsa
```

**Expected:** non-zero exit + "Operation not permitted" / sandbox deny — **NOT the key
contents**.

**Result:** [ ] pass  /  [ ] blocked

```
<paste output here — MUST show a deny, NOT the private key>
```

---

## Assertion 3 — Dual-path `/etc/hosts` AND `/private/etc/hosts` blocked

Mirrors `test_platform_deny_symlink_and_canonical_path` (macos.rs line 1919). macOS
`/etc` is a symlink to `/private/etc`; a deny on one form alone does not cover the other.

```
nono run --profile claude-code -- cat /etc/hosts
nono run --profile claude-code -- cat /private/etc/hosts
```

**Expected:** **BOTH** blocked (non-zero exit + sandbox deny for each).

**Result:** [ ] pass  /  [ ] blocked

```
<paste BOTH outputs here — both must be blocked>
```

---

## Assertion 4 — Library tests green on the host

```
make test-lib        # cargo test -p nono
```

**Expected:** all `sandbox::macos` tests pass on the host (the ordering + dual-path
contract tests run natively here).

**Result:** [ ] pass  /  [ ] blocked

```
<paste the cargo test summary line here — all sandbox::macos tests passing>
```

---

## Assertion 5 — resl resource-limit ENFORCEMENT (gated off CI; runs here)

`macos_timeout_kills_at_deadline` and `macos_max_processes_blocks_on_rlimit_nproc`
(REQ-RESL-NIX-03) exercise the macOS `--timeout` watchdog and `RLIMIT_NPROC`
enforcement. These were **never validated on a real host** (Phase 37 was host-blocked),
and they **do not fire on the GitHub macOS runner** — where they hang the runner to
death. PR #6 (`d9144663`) gated them behind `NONO_RESL_HOST_VALIDATED` so they **skip on
CI** and **run here, on a real macOS host**. This assertion is their real validation
venue.

```
NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos -- \
  macos_timeout_kills_at_deadline macos_max_processes_blocks_on_rlimit_nproc --nocapture
```

**Expected:** BOTH pass with the var set — `--timeout 5s` kills `sleep 60` at ~5s (≥3s,
<10s), and `--max-processes 5` makes the 6th `fork()` fail (EAGAIN / non-zero child
exit). If either HANGS or exits in ~0s, the enforcement is NOT firing — record it as a
real REQ-RESL-NIX-03 defect (do not flip to pass).

**Result:** [ ] pass  /  [ ] blocked

```
<paste the cargo test summary — both tests passing with NONO_RESL_HOST_VALIDATED=1>
```

---

## Sign-off

- **Gate 65-A (live `sandbox_init()` enforcement):** _PASS / FAIL / blocked: no macOS host_
- **Host / macOS version / date:** _<stamp>_
- **Green `macos-latest` CI SHA (D-11c HARD gate, runs alongside this live gate):**
  ✅ **SATISFIED 2026-06-11** — run
  [`27345465703`](https://github.com/OscarMackJr/nono/actions/runs/27345465703), SHA
  `d9144663` (PR #6), `Test` + `Clippy` macos-latest legs both `success`. Detail in
  [`65-MACOS-CI-EVIDENCE.md`](65-MACOS-CI-EVIDENCE.md). The CI HARD gate is **closed**;
  this live gate (65-A, incl. Assertion 5) remains the other half that closes MACOS-03.

> **This checklist BLOCKS phase close.** It ships OPEN (no macOS host confirmed at
> authoring). Resume-signal (plan 65-04 Task 2): type **"approved"** with the four
> pasted results + the host/macOS-version stamp, or **"blocked: no macOS host"** to keep
> gate 65-A open. Do not flip on Windows/CI evidence (the CI HARD gate is a *separate*,
> complementary signal — it does not substitute for live enforcement).
