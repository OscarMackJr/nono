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

## Sign-off

- **Gate 65-A (live `sandbox_init()` enforcement):** _PASS / FAIL / blocked: no macOS host_
- **Host / macOS version / date:** _<stamp>_
- **Green `macos-latest` CI SHA (D-11c HARD gate, runs alongside this live gate):**
  see [`65-MACOS-CI-EVIDENCE.md`](65-MACOS-CI-EVIDENCE.md) — record the green
  `macos-latest` run URL + SHA there; the two gates together close MACOS-03.

> **This checklist BLOCKS phase close.** It ships OPEN (no macOS host confirmed at
> authoring). Resume-signal (plan 65-04 Task 2): type **"approved"** with the four
> pasted results + the host/macOS-version stamp, or **"blocked: no macOS host"** to keep
> gate 65-A open. Do not flip on Windows/CI evidence (the CI HARD gate is a *separate*,
> complementary signal — it does not substitute for live enforcement).
