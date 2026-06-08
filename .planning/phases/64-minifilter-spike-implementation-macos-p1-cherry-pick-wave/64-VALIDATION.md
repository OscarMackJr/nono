---
phase: 64
slug: minifilter-spike-implementation-macos-p1-cherry-pick-wave
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-08
---

# Phase 64 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Two tracks: Track A (Windows minifilter spike — VM/manual-only evidence) and
> Track B (macOS P1 cherry-picks — automated unit + cross-target).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | None — workspace-level |
| **Quick run command** | `cargo test -p nono -- sandbox::macos` |
| **Full suite command** | `make test` |
| **Cross-target gate** | `cargo clippy -p nono -p nono-cli --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` |
| **Estimated runtime** | ~60 seconds (Track B unit suite); Track A is VM-side manual |

---

## Sampling Rate

- **After every Track B task commit:** Run `cargo test -p nono -- sandbox::macos`
- **After every Track B plan wave:** Run `make test`
- **Track A tasks:** No automated tests — VM-side `fltmc instances` + scripted deny harness capture evidence to the SC1 artifact
- **Before `/gsd:verify-work`:** Track B unit tests green + `x86_64-apple-darwin` clippy green + Track A VM evidence captured
- **Max feedback latency:** ~60 seconds (Track B)

---

## Per-Task Verification Map

| Item | Track | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|------|-------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| Scripted harness asserts `ERROR_ACCESS_DENIED` (5) on deny-target | A | DRV-01 | T-63-03 | Open of deny-target refused at kernel boundary | Manual (VM) | Harness script on Azure VM | ❌ W0 (new script) | ⬜ pending |
| `fltmc instances` shows driver at chosen altitude | A | DRV-01 / DRV-03 | T-63-05 | Driver registered at non-colliding Activity-Monitor altitude | Manual (VM) | VM-side PowerShell check | ❌ W0 (new script) | ⬜ pending |
| `NonoIpcRequest` Rust size matches C `_Static_assert` | A | DRV-02 | T-63-04 | Static layout assertion compiles | Unit (compile-time) | `cargo build -p nono-fltmgr-client` | ❌ W0 (new crate) | ⬜ pending |
| User-mode client receives path+PID, returns allow/deny | A | DRV-02 | T-63-02 | Driver enforces returned decision | Integration (VM) | Manual on Azure VM | Manual-only | ⬜ pending |
| Driver builds + test-signs + loads on VM | A | DRV-03 | T-63-01 | `SERVICE_DEMAND_START` + snapshot rollback safeguard | Manual (VM) | `fltmc instances` output | Manual-only | ⬜ pending |
| Platform deny rules appear AFTER write-allows | B | MACOS-02 | T-64-10 | Last-match-wins: deny overrides allow | Unit | `cargo test -p nono -- sandbox::macos::tests::test_platform_rules_after_write_allows` | ❌ W0 (new test) | ⬜ pending |
| Deny rules cover both `/etc/...` and `/private/etc/...` | B | MACOS-02 | T-64-11 | No symlink bypass of canonical path | Unit | `cargo test -p nono -- sandbox::macos::tests::test_platform_deny_symlink_and_canonical_path` | ❌ W0 (new test) | ⬜ pending |
| Existing ordering test updated to post-fix ordering (renamed in Plan 01) | B | MACOS-02 | T-64-10 | `read_pos < write_pos < deny_pos` | Unit | `cargo test -p nono -- sandbox::macos::tests::test_generate_profile_platform_rules_after_writes` | ✅ exists (renamed from `test_generate_profile_platform_rules_between_reads_and_writes`), wrong assertion pre-Plan 01 | ⬜ pending |
| Cross-target clippy passes on cherry-picked files | B | MACOS-02 | — | No cfg-gated Unix drift | Cross-target build | `cargo clippy -p nono -p nono-cli --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✅ target installed | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Track B:
- [ ] `crates/nono/src/sandbox/macos.rs` — new tests `test_platform_rules_after_write_allows`, `test_platform_deny_symlink_and_canonical_path`
- [ ] `crates/nono/src/sandbox/macos.rs` — rename existing `test_generate_profile_platform_rules_between_reads_and_writes` to `test_generate_profile_platform_rules_after_writes` (line ~998) and update assertions to post-fix ordering (`read_pos < write_pos < deny_pos`)

Track A:
- [ ] `crates/nono-fltmgr-client/` — new Cargo workspace member (`Cargo.toml` + `src/lib.rs` skeleton with the `#[repr(C)] NonoIpcRequest` static-layout assertion + `src/main.rs` CLI binary accepting deny-path as argv[1])
- [ ] Root `Cargo.toml` — add `"crates/nono-fltmgr-client"` to `[workspace] members`
- [ ] Scripted deny harness (PowerShell or tiny exe) provisioned on the VM

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Driver test-signs + loads at chosen altitude | DRV-01 / DRV-03 | Kernel driver load requires Secure-Boot-OFF VM + testsigning | `makecert → inf2cat → signtool → certmgr → bcdedit /set testsigning on → pnputil /add-driver`; capture `fltmc instances`/`fltmc filters` |
| Deny-target open refused (`ERROR_ACCESS_DENIED`) | DRV-01 | Requires loaded kernel driver intercepting `IRP_MJ_CREATE` | Run deny harness on VM; assert exact Win32 error 5 |
| Kernel↔user policy round-trip enforced | DRV-02 | Requires both driver loaded + `nono_fltmgr_client.exe` running on VM | Run `nono_fltmgr_client.exe C:\nono-deny-test\secret.txt`, trigger create on deny-target, observe path+PID delivery + enforced decision |
| `aarch64-apple-darwin` clippy/build | MACOS-02 | Cross-toolchain NOT installed on dev host (D-12) | Mark PARTIAL; defer to live macOS CI per `.planning/templates/cross-target-verify-checklist.md` |

---

## Validation Sign-Off

- [x] All Track B tasks have automated verify or Wave 0 dependencies
- [x] Track A VM-only behaviors documented as manual with capture instructions
- [x] Sampling continuity: no 3 consecutive automated-eligible tasks without automated verify
- [x] Wave 0 covers all MISSING references (new crate + new/updated macOS tests)
- [x] No watch-mode flags
- [x] Feedback latency < 60s (Track B)
- [x] `nyquist_compliant: true` set in frontmatter (all automated-eligible tasks have `<automated>` elements; Track A tasks are manual-only by design — no automated gate exists for kernel driver load or VM deny-harness execution)

**Approval:** pending
