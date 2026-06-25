# TODO: macOS `setrlimit(RLIMIT_AS, N)` fails in the supervised child — `--memory` enforcement broken

**Captured:** 2026-06-12 (Phase 68 debug host probe P-B on Oscars-MacBook-Pro)
**Severity:** high — `--memory` (RLIMIT_AS) enforcement is non-functional on macOS
**Source:** `.planning/debug/macos-resl-not-firing.md` (defect D2)
**Relation to Phase 68:** Phase 68 BLOCKER (defect D2). Pre-dates Phase 68 (the RLIMIT_AS child-arm
block existed before this work); first exercised by the Phase 68 D-09 bonus memory test.

## Problem
On a real macOS host, `nono run --memory 32m -- python3 -c "x=bytearray(256*1024*1024); print('ALLOCATED')"`
prints to stderr:
```
nono: setrlimit(RLIMIT_AS) failed in pre-exec child; aborting
```
i.e. the supervised child arm's RLIMIT_AS block (`crates/nono-cli/src/exec_strategy.rs:1003`,
`setrlimit(Resource::RLIMIT_AS, limit, limit).is_err()` → `MSG_RLIMIT_AS_FAIL` + `libc::_exit(126)`)
fires: `setrlimit(RLIMIT_AS, 32 MiB)` returns an error in the forked child, so the child aborts before
exec. `--memory` is therefore not enforced (and the child dies with 126 instead of running under the cap).

## Root-cause hypotheses (investigate during re-plan)
- macOS may reject lowering RLIMIT_AS below the process's current virtual size (the forked nono child
  already maps far more than 32 MiB) — EINVAL/EPERM. The fix may need to set the limit later (closer to
  exec, after trimming mappings) or accept that RLIMIT_AS on macOS arm64 cannot be set this low.
- macOS RLIMIT_AS support has historically been weak/quirky; confirm whether a larger `--memory` value
  (e.g. 256m, above the child's baseline VM) succeeds where 32m fails.
- Check the Direct-path analog (`supervisor_macos.rs::install_pre_exec`, nix `setrlimit(Resource::RLIMIT_AS,...)`)
  for the same failure.

## Acceptance
`nono run --memory <N> -- <child>` enforces RLIMIT_AS on a real macOS host (the D-09 test
`macos_memory_limit_kills_at_rlimit_as` passes with `NONO_RESL_HOST_VALIDATED=1`), OR macOS RLIMIT_AS
limitations are characterized and `--memory` behavior is documented/handled fail-secure with a clear rationale.

## RESOLVED — 2026-06-25 (acceptance branch 2)
Already fixed by commit **`648c5856`** *"fix(68-02): D2 downgrade RLIMIT_AS abort to warn-and-continue
on macOS; flip D-09 test assertion"* (Phase 68-02). The abort-with-`_exit(126)` behavior this todo
describes no longer exists. Verified in current code 2026-06-25:

- **Supervised child arm** (`crates/nono-cli/src/exec_strategy.rs:1129`): `setrlimit(RLIMIT_AS)` failure
  emits `MSG_RLIMIT_AS_WARN` ("not enforced on macOS (best-effort); continuing") to stderr and continues
  — no longer aborts.
- **Direct-path analog** (`crates/nono-cli/src/exec_strategy/supervisor_macos.rs:277`): same best-effort
  handling (`is_err()` → continue to exec).
- **Root cause characterized + documented:** dyld pre-maps several hundred MiB of VAS; `setrlimit`
  below current VAS usage returns EINVAL; macOS arm64 does not reliably enforce RLIMIT_AS. See the
  `## RLIMIT_AS vs RSS` section in `supervisor_macos.rs` and the inline `D2 (Phase 68-02)` comments.
- **`--max-processes` (RLIMIT_NPROC) remains fail-closed** on macOS; only the RLIMIT_AS arm is best-effort.

Disposition: best-effort warn-and-continue (operator-acknowledged 2026-06-25). RLIMIT_AS is a resource
cap, not part of nono's OS-enforced isolation boundary (Seatbelt), so best-effort is acceptable. The
host-gated D-09 test (`NONO_RESL_HOST_VALIDATED=1`) remains the real-Mac confirmation point.
