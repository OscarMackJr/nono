---
quick_id: 260729-nh4
slug: crossbeam-epoch-rustsec-2026-0204
status: complete
date: 2026-07-29
files_modified:
  - Cargo.lock
---

# SUMMARY — RUSTSEC-2026-0204 closed

**Status:** complete. Advisory cleared, workspace build green, zero source changes.

## What changed

One command: `cargo update -p crossbeam-epoch --precise 0.9.20`.

Resulting diff — `Cargo.lock` only, 2 insertions / 2 deletions:

```
-version = "0.9.18"
+version = "0.9.20"
-checksum = "5b82ac4a3c2ca9c3460964f020e1402edd5753411d7737aa39c3714ad1b5420e"
+checksum = "2d6914041f254d6e9176c01941b21115dcfb7089e55135a35411081bd106ef3f"
```

Byte-identical to upstream `373a67ae` (#1369). No `Cargo.toml` touched — `crossbeam-epoch` is a
transitive dependency only.

## Verification

| Check | Before | After |
|---|---|---|
| `cargo audit` matches for `RUSTSEC-2026-0204` / `crossbeam-epoch` | present, with full dep tree to shipped binaries | **0** |
| `crossbeam-epoch` version in `Cargo.lock` | 0.9.18 | **0.9.20** |
| `cargo build --workspace --all-targets` | — | **Finished `dev` profile** (3m 46s) |
| Pre-existing audit warnings | 5 (async-std, fxhash, paste, rustls-pemfile, anyhow) | unchanged — none introduced, none claimed fixed |

`cargo-audit` 0.22.1, scanning 567 crate dependencies.

## Notes

- The remaining `cargo audit` output consists of *warnings* (unmaintained / unsound), not
  vulnerabilities. They predate this change and are explicitly out of scope. This task does not
  claim to have fixed them.
- `make ci` was not run — the change cannot affect clippy or fmt (lockfile-only), and the repo has
  documented pre-existing Windows baseline test failures unrelated to this bump
  (`nono_cli_windows_baseline_test_failures`). `cargo build --workspace --all-targets` is the
  proportionate gate here.
- Cross-target clippy was not run: no cfg-gated Unix code is touched, so CLAUDE.md's cross-target
  requirement is not triggered.

## Relationship to Phase 112

Phase 112's **SC3** prioritizes this exact commit and allows for it being closed out-of-band by a
quick task. When Phase 112 is planned, SC3 should be satisfied by **recording this closure and
confirming `cargo audit` is clean** — not by re-absorbing `373a67ae`, which would now be a no-op
against the lockfile.

The rest of the DEPS cluster (18 remaining commits) is untouched and stays in Phase 112's scope.

## Provenance

Found by Phase 108's divergence audit while classifying the DEPS cluster — the direct payoff of
CONTEXT decision **D-16**, which ruled dependency-only commits a security-relevant cluster rather
than noise.
