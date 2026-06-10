# 65-04 SUMMARY — macOS live `sandbox_init()` HUMAN-UAT (MACOS-03)

**Status:** Task 1 COMPLETE; Task 2 **gate 65-A OPEN / BLOCKS PHASE CLOSE** (no macOS host).

## What was built

`.planning/.../65-HUMAN-UAT.md` — a close-blocking gate-65-A checklist staging the four
live `sandbox_init()` deny assertions (D-10 live subset):
1. `nono run --dry-run --profile claude-code` → deny-after-allow ordering visible
2. `nono run --profile claude-code -- cat ~/.ssh/id_rsa` → blocked (not the key)
3. `cat /etc/hosts` AND `cat /private/etc/hosts` → BOTH blocked (dual-path)
4. `make test-lib` → all `sandbox::macos` tests green on the host

Each item has a command, expected output, `pass/blocked` field, raw-output paste block,
and host/date/macOS-version stamp slots. Sign-off references the green `macos-latest` CI
SHA (D-11c) in `65-MACOS-CI-EVIDENCE.md`.

## Verification (Task 1, automated — all PASS)

- `id_rsa`, `/private/etc/hosts`, `--dry-run --profile claude-code`, `make test-lib`,
  `gate 65-A`, `macos-latest` all present ✓
- BOTH `/etc/hosts` and `/private/etc/hosts` present (3 `/etc/hosts` occurrences) ✓
- No item pre-marked `pass`; gate ships OPEN ✓

## Task 2 — gate 65-A OPEN (close-blocking)

**No macOS host is available this session** (confirmed: infra-access answer excluded a
macOS host). The live `sandbox_init()` enforcement assertions have **NO automatable
substitute** — CI runs the unit tests but does NOT execute the live `nono run -- cat
~/.ssh/id_rsa` deny path (Pitfall 4 / T-65-NOHOST). Gate 65-A is recorded
**`blocked: no macOS host`** and **BLOCKS phase close** until a real macOS host runs the
checklist. Not flipped on Windows/CI evidence (fail-secure).

## Self-Check: PASSED (Task 1) — Task 2 gate 65-A OPEN/blocking

## Phase-close impact

⛔ **Phase 65 cannot fully close until gate 65-A runs on a real macOS host.** The
automatable MACOS-03 subset (plan 65-02: tests + clippy-PARTIAL + cherry-pick scan +
green CI SHA) is closeable; the live-enforcement subset stays OPEN here.
