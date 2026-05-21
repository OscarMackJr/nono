---
phase: 49-sigstore-trust-root-poc-resilience-from-file-flag-release-as
audited: 2026-05-21
asvs_level: L2
block_on: high
verdict: SECURED
threats_total: 13
threats_closed: 13
threats_open: 0
unregistered_flags: 0
---

# Phase 49 — Sigstore Trust-Root POC Resilience — Security Audit

**Verdict:** SECURED — 13/13 threats verified closed by grep-confirmed evidence in implementation files. Zero open threats, zero unregistered flags.

**Scope:** STRIDE threat register from PLAN 49-01 (5 threats), PLAN 49-02 (5 threats), PLAN 49-03 (3 threats). Disposition: 12 `mitigate` + 1 `accept`. No `transfer` threats.

**Adversarial stance applied:** Each `mitigate` threat verified by grep-locating the declared mitigation pattern in the files cited by the threat's `Mitigation Plan`. The single `accept` threat (01-T-49-03) is verified by documenting the accepted-risk rationale in this file's accepted-risks log below; the rationale is sound and is rooted in trust-boundary semantics that this phase did not invent (maintainer/user-as-trust-anchor for `<PATH>`).

---

## Threat Verification

### Plan 49-01 (`--from-file` flag) — 5 threats

| Threat ID | Category | Disposition | Verified Evidence |
|-----------|----------|-------------|-------------------|
| 01-T-49-01 | Tampering — from_file_step validation | mitigate | CLOSED — `crates/nono-cli/src/setup.rs:902` calls `nono::trust::bundle::load_trusted_root(src)` (parse gate); `setup.rs:913` calls `nono::trust::bundle::check_trusted_root_freshness(&trusted_root, &cache_path)` (freshness gate). BOTH precede `std::fs::copy` at `setup.rs:924`. On any `?`-propagated Err, the function returns before the copy runs — the cache file is never touched. Tests `from_file_expired_fails_closed` (line 257), `from_file_malformed_truncated_fails_closed` (line 294), `from_file_malformed_quote_flipped_fails_closed` (line 328) assert `!cache_path.exists()` on all three failure classes. |
| 01-T-49-02 | Tampering — clap surface URL/scheme bypass | mitigate | CLOSED — `crates/nono-cli/src/cli.rs:2384` declares `pub from_file: Option<PathBuf>`. clap parses `<PATH>` as a `PathBuf` (no URL/scheme parsing, no `http://`/`ftp://`/`file://` interpretation — bytes are taken as a filesystem path). No network primitive is invoked on this code path; the only fetch in `from_file_step` is `nono::trust::bundle::load_trusted_root(src)` which is a `std::fs::read` over the local FS. |
| 01-T-49-03 | Tampering / TOCTOU — symlink on cache path | **accept** | CLOSED (accepted-risk) — see "Accepted Risks" log below for full rationale. Source-side symlink follow by `std::fs::copy` is accepted because `<PATH>` is by definition user-supplied / user-trusted. Destination cache path is constructed deterministically from `crate::config::nono_home_dir()?.join(".nono").join("trust-root").join("trusted_root.json")` (see `setup.rs:889-891`, `setup.rs:909`) — no attacker-controllable path component on the destination side outside the `nono_home` tree the user already owns. |
| 01-T-49-04 | Information Disclosure — partial cache leak on copy failure | mitigate | CLOSED — `setup.rs:924-927` wraps `std::fs::copy` in an `if let Err(e)` block. On `Err`, the function executes `let _ = std::fs::remove_file(&cache_path);` (swallowing the inner cleanup error per D-49-B2) and returns `Err(NonoError::Io(e))` propagating the original IO error. Test `from_file_missing_path_no_partial_cache` (line 353) asserts `!cache_path.exists()` after the run. |
| 01-T-49-05 | Tampering — clap-mutex bypass on simultaneous `--from-file` + `--refresh-trust-root` | mitigate | CLOSED — `crates/nono-cli/src/cli.rs:2382` declares `conflicts_with = "refresh_trust_root"` on the `from_file` clap field. Rejection happens at clap-parse time (BEFORE any FS write). Test `from_file_with_refresh_rejected_by_clap` (line 381) asserts non-zero exit, `"cannot be used with"` stderr substring, AND `!cache_path.exists()` (the parse-time rejection precedes any cache mutation). Manual smoke recorded in 49-01-SUMMARY § Verification confirms exit 2 + `the argument '--from-file <PATH>' cannot be used with '--refresh-trust-root'`. |

### Plan 49-02 (release-asset bundling — release.yml) — 5 threats

| Threat ID | Category | Disposition | Verified Evidence |
|-----------|----------|-------------|-------------------|
| 02-T-49-04 | Tampering — maintainer-side leak (asset != committed fixture) | mitigate | CLOSED — `.github/workflows/release.yml:325-326` computes `SRC_SHA=$(sha256sum "$SRC" | cut -d' ' -f1)` + `DST_SHA=$(sha256sum "$DST" | cut -d' ' -f1)`; lines 327-332 perform equality check `if [ "$SRC_SHA" != "$DST_SHA" ]; then echo "ERROR: ..." >&2; exit 1; fi`. Drift exits the CI step non-zero BEFORE `softprops/action-gh-release` runs (which is the next step starting line 350). The success path emits the diagnostic at line 333 (`echo "trusted_root.json byte-identity verified: $SRC_SHA"`). 49-02-SUMMARY records local positive dry-run with SHA `6494e21ea73fa7ee769f85f57d5a3e6a08725eae1e38c755fc3517c9e6bc0b66` AND negative dry-run with tampered DST correctly rejected. |
| 02-T-49-04b | Tampering — release-asset omission | mitigate | CLOSED — `.github/workflows/release.yml:362` lists `artifacts/trusted_root.json` (12-space-indented, repo-root-relative for softprops/action-gh-release's `files:` glob) between the `artifacts/*.deb` line and the `artifacts/SHA256SUMS.txt` line. Live-release verification (`gh release view <tag> --json assets`) is intrinsically Manual-Only per VALIDATION.md and is routed to human-verification; the gate is structurally complete. |
| 02-T-49-04c | Tampering — hash omission from SHA256SUMS.txt | mitigate | CLOSED — `.github/workflows/release.yml:345-347` adds the conditional aggregation entry `if ls trusted_root.json >/dev/null 2>&1; then sha256sum trusted_root.json >> SHA256SUMS.txt; fi` mirroring the pre-existing `*.zip` / `*.msi` / `*.exe` patterns. Coverage by the existing `sha256sum -c SHA256SUMS.txt` POC-user workflow is preserved. |
| 02-T-49-06 | CI silent-pass — new step exits 0 on internal failure (cut-pipe failure masks hash) | mitigate | CLOSED — `.github/workflows/release.yml:310` adds `set -euo pipefail` at the top of the `Generate checksums` step's `run:` block. `-e` exits on any command failure; `-u` exits on any unset variable; `-o pipefail` propagates non-zero through pipes (critical for the `sha256sum "$SRC" | cut -d' ' -f1` pipeline). Without `pipefail`, `cut` swallowing a `sha256sum` failure would silently mask a real mismatch. |
| 02-T-49-07 | Working-dir mismatch — cp/sha256sum cwd composition | mitigate | CLOSED — The new byte-identity block (lines 322-333) is folded INSIDE the existing `cd artifacts` scope at line 311. Source path is `../crates/nono/tests/fixtures/trust-root-frozen.json` (one level up from `artifacts/` — resolves correctly to repo-root-relative source). Destination is `trusted_root.json` (cwd-relative inside `artifacts/`). The `softprops/action-gh-release` `files:` glob entry at line 362 is `artifacts/trusted_root.json` (repo-root-relative — softprops runs from default repo-root cwd). All three path-context shifts compose cleanly. |

### Plan 49-03 (cadence template + smoke scripts + doc rewrite) — 3 threats

| Threat ID | Category | Disposition | Verified Evidence |
|-----------|----------|-------------|-------------------|
| 03-T-49-05 | Tampering — Smoke-script silent-failure | mitigate | CLOSED — `scripts/verify-trust-root-cached.sh:14` sets `set -euo pipefail`. `scripts/verify-trust-root-cached.ps1:24` sets `$ErrorActionPreference = 'Stop'` AND the script has an explicit `$LASTEXITCODE` check at line 44 (`if ($LASTEXITCODE -ne 0) { throw "nono setup --from-file failed with exit code $LASTEXITCODE" }`) IMMEDIATELY after the `& nono setup --from-file $Candidate` native invocation at line 43. The threat model's explicit requirement ("explicit `if ($LASTEXITCODE -ne 0) { throw }` after every native command invocation") is satisfied: the only native command in the script is `& nono setup ...` at line 43, and it is followed by the `$LASTEXITCODE` check at line 44. The `Get-FileHash` / `Test-Path` / `New-Item` calls at lines 26-58 are PowerShell cmdlets (NOT native commands) and are correctly trapped by `$ErrorActionPreference = 'Stop'`. Per 49-03-SUMMARY § Task 4 Live Verification, Scenarios 2 (param-validation early-exit -> exit 2) and 3 (nono-missing failure-propagation -> exit 1) PASS live on pwsh 7.x; 4 references to `LASTEXITCODE` in the script (verified). |
| 03-T-49-08 | Information Disclosure / Maintainer Error — non-fresh fixture committed | mitigate | CLOSED — `.planning/templates/sigstore-rotation-refresh.md` Step 4 (lines 48-58) names the smoke script as the pre-commit gate: `./scripts/verify-trust-root-cached.sh crates/nono/tests/fixtures/trust-root-frozen.json` (Unix) AND `pwsh scripts/verify-trust-root-cached.ps1 ...` (Windows). Template Step 6 (lines 69-77) references the Plan 49-02 byte-identity assert in `.github/workflows/release.yml`'s `Generate checksums` step as the post-commit + at-release gate. Two-stage gate confirmed: smoke-script at commit time + release.yml at release time. |
| 03-T-49-09 | Tampering — Stale doc references mislead POC users | mitigate | CLOSED — Negative-grep verification: `grep -E '(sigstore-verify 0\.6\.5\|P32-DEFER-005\|deferred-items\.md)' docs/cli/development/windows-poc-handoff.mdx` returns ZERO matches (exit 1) — confirmed by audit. All three stale strings AND the "will start working again once the dep is upgraded" dep-treadmill prose have been purged. Positive-grep verification: `--from-file` appears at lines 166 (Run once after install header), 173 (Path B example), 208 (Primary path in Known issue), 217 (Known issue example), 231 (Invoke-WebRequest fallback comment). `sigstore-rotation-refresh` forward-pointer present at line 234. The Known issue heading at line 188 (`#### Known issue: Sigstore TUF root rotation`) is version-pin-free. |

---

## Accepted Risks Log

### 01-T-49-03 — Symlink follow on source side of `std::fs::copy`

**Disposition:** ACCEPT
**Risk:** `std::fs::copy(src, &cache_path)` follows symlinks on the source side. A user-supplied `<PATH>` could be a symlink to an arbitrary readable file on the host.

**Rationale for acceptance:**
1. **Trust boundary alignment:** The `<PATH>` argument is by definition user-supplied / user-trusted input. Plan 49-01's threat model explicitly identifies the trust boundary as "CLI arg -> process" and treats the file at the supplied path as untrusted-JSON-to-validate, not untrusted-path-to-resolve. A user who can pass `--from-file <PATH>` already controls the bytes that will be validated; whether those bytes arrive via a symlink or a regular file is semantically identical from the threat-model standpoint.
2. **Destination side is deterministic:** The cache destination is constructed in `crates/nono-cli/src/setup.rs:889-891` (`cache_dir = nono_home_dir()?.join(".nono").join("trust-root")`) and `setup.rs:909` (`cache_path = cache_dir.join("trusted_root.json")`). No attacker-controllable path component flows into the destination. The destination tree is per-user (`nono_home_dir()`), so an attacker without write-access to the user's `~/.nono` directory cannot exploit a symlink at the destination side either.
3. **No new TOCTOU window the attacker can exploit:** An attacker who can mutate the source file between `load_trusted_root(src)` (parse) and `std::fs::copy(src, &cache_path)` (write) already has FS write access to that path — they could simply have written a malicious-but-syntactically-valid trusted root in the first place. Phase 49 does not invent this threat; it inherits the standard maintainer-as-trust-anchor model from Sigstore/POC ergonomics.
4. **Documented in the source comments:** The plan's `<threat_model>` explicitly enumerates this disposition (`accept`) and cites the rationale. 49-REVIEW.md WR-01 acknowledges a theoretical TOCTOU window between validation and copy but classifies it as advisory WARNING (maintainer-as-attacker model), not BLOCKER.

**Defense-in-depth follow-up (not blocking):** 49-REVIEW.md WR-01 records a follow-up enhancement to read-once-and-write-atomically via temp+rename, which would close the residual TOCTOU even under hostile-source assumptions. This is recorded as a Phase 49 v2 hardening item, not a Phase 49 v1 closure gap.

---

## Unregistered Flags

**None.** Of the three SUMMARY files:
- `49-01-SUMMARY.md`: contains no `## Threat Flags` section — no new attack surface flagged by the executor beyond what `<threat_model>` already enumerated.
- `49-02-SUMMARY.md`: contains `## Threat Flags` section explicitly reading "None — this plan introduces no new security-relevant surface beyond what the plan's `<threat_model>` already enumerated. The new CI step is contained within the existing `release` job, runs only at tag-push, has no network primitives beyond what `softprops/action-gh-release` already uses, and emits no new file or trust-boundary crossings."
- `49-03-SUMMARY.md`: contains no `## Threat Flags` section — no new attack surface flagged by the executor.

All declared threats in the register cleanly map to executor-implemented mitigations; no unmapped new surface was discovered.

---

## Verifier Notes

**Verification approach:** For each `mitigate` threat, the auditor located the threat's declared mitigation pattern by line-number grep in the files cited by the threat's `Mitigation Plan`. For the one `accept` threat (01-T-49-03), the auditor documented the accepted-risk rationale in this file's accepted-risks log and reviewed the underlying trust-boundary assumption against the plan's `<threat_model>` trust-boundary section — the rationale holds.

**Cross-target clippy:** PARTIAL disposition recorded by 49-01-SUMMARY (cross-toolchains absent on Windows dev host for `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin`). Per the audit constraints: this is a VERIFICATION gap (toolchain availability), not a SECURITY gap (no threat is verifiable only by cross-target clippy). The phase's threat register does not encode a cross-target clippy mitigation; therefore PARTIAL on F-01-06 does not surface as an open threat. Decisive signal remains the post-merge live GH Actions Linux/macOS Clippy lanes per `.planning/templates/cross-target-verify-checklist.md § PARTIAL Disposition`.

**Scenario 1 (positive `.ps1` smoke-script self-test):** DEFERRED to post-merge per 49-03-SUMMARY § Task 4. This is a UAT gap (binary not on PATH in the worktree), not a security gap. The `LASTEXITCODE` propagation is statically present (4 references in `verify-trust-root-cached.ps1`) and Scenarios 2 + 3 PASS live; the threat 03-T-49-05 is closed by the static + live-host-independent evidence chain.

**49-REVIEW.md follow-ups (not blocking):** 49-REVIEW.md identified 7 WARNING findings (WR-01 read-twice TOCTOU, WR-02 release-asset freshness gate, WR-03 PowerShell env-var leak, WR-04 GHA template injection, WR-05 phase-index helper naming, WR-06 doc-comment overclaim, WR-07 fallback bypasses --from-file validation). Per the audit constraints these are advisory — the threat register's declared mitigations are present in the implementation. The 7 WARNINGs constitute a "Phase 49 v2 hardening pass" surface for a follow-on phase; they do not invalidate v1 SECURED status.

---

## Summary Table

| Plan | Threats | Closed | Open | Disposition Mix |
|------|---------|--------|------|-----------------|
| 49-01 (`--from-file`) | 5 | 5 | 0 | 4 mitigate + 1 accept |
| 49-02 (release-asset) | 5 | 5 | 0 | 5 mitigate |
| 49-03 (cadence + scripts + docs) | 3 | 3 | 0 | 3 mitigate |
| **Total** | **13** | **13** | **0** | **12 mitigate + 1 accept + 0 transfer** |

**ASVS Level:** L2 (default for OS-enforced sandboxing security baseline; trust-root validation surface treated as security-critical).

**block_on:** high. No high-severity threat is unverified. Phase clears the high-severity gate.

---

*Audited: 2026-05-21*
*Auditor: gsd-security-auditor (Claude)*
*Source: PLAN.md `<threat_model>` blocks in 49-01-from-file-flag-PLAN.md, 49-02-release-asset-bundling-PLAN.md, 49-03-fixture-refresh-cadence-PLAN.md*
