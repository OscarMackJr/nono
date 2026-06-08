# Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-08
**Phase:** 64-Minifilter Spike Implementation + macOS P1 Cherry-pick Wave
**Areas discussed:** Deny-target & deny proof, Rust client + IPC struct, macOS cherry-pick mechanics, VM + signing pipeline proof

---

## Deny-target & deny proof (DRV-01)

### Proof method
| Option | Description | Selected |
|--------|-------------|----------|
| Scripted test harness | Harness opens a dedicated deterministic path and asserts `ERROR_ACCESS_DENIED` (5); output + `fltmc instances` captured to SC artifact | ✓ |
| Manual interactive attempt | Manual `Get-Content`/`type`; capture console transcript + screenshot | |

**User's choice:** Scripted test harness
**Notes:** Repeatable, unambiguous evidence preferred. Deny target = a single dedicated deterministic throwaway path (POC depth — one hard-coded deny path).

---

## Rust client + IPC struct (DRV-02)

### Client home
| Option | Description | Selected |
|--------|-------------|----------|
| Standalone spike crate | Dedicated `#[cfg(windows)]` crate; isolates throwaway spike code from `nono-cli` | ✓ |
| Inside nono-cli (cfg-windows) | `#[cfg(windows)]` module in `nono-cli` reusing its windows-sys dep | |
| Example/bin in existing crate | `examples/` or extra bin target | |

**User's choice:** Standalone spike crate

### IPC message struct
| Option | Description | Selected |
|--------|-------------|----------|
| Minimal POC fields | Fixed-size path buffer + PID + desired-access/op + `static_assert(sizeof)` | ✓ |
| Minimal + ABI insurance | Add version/reserved + request-id/sequence fields | |

**User's choice:** Minimal POC fields

### Crate scope
| Option | Description | Selected |
|--------|-------------|----------|
| Workspace member (cfg-windows) | New workspace member, all code `#[cfg(windows)]`; `cargo test` runs the layout assertion | ✓ |
| Excluded from workspace | Out-of-workspace crate built ad hoc on the VM | |

**User's choice:** Workspace member (cfg-windows)
**Notes:** Compiles to nothing on Linux/macOS CI; gets clippy/test coverage for the static-layout assertion.

---

## macOS cherry-pick mechanics (MACOS-02)

### Conflict handling
| Option | Description | Selected |
|--------|-------------|----------|
| Manual port + trailer | Apply the fix at the fork's correct call-site, keep the verbatim `Upstream-commit:` trailer, note site divergence; diff-inspect each site | ✓ |
| Abort & checkpoint | Stop on first non-clean cherry-pick and surface to the user | |

**User's choice:** Manual port + trailer

### Cross-target verification timing
| Option | Description | Selected |
|--------|-------------|----------|
| Phase 64, PARTIAL-if-missing | Run apple-darwin clippy/build in Phase 64; mark PARTIAL + defer to live CI if toolchain absent (CLAUDE.md MUST rule) | ✓ |
| Defer to Phase 65 | Phase 64 lands cherry-picks + Linux/Windows-host checks only; macOS cross-target deferred | |

**User's choice:** Phase 64, PARTIAL-if-missing
**Notes:** Live macOS-host re-validation + green-CI hard gate remains Phase 65 (MACOS-03).

---

## VM + signing pipeline proof (DRV-01/03)

### VM strategy
| Option | Description | Selected |
|--------|-------------|----------|
| Reprovision via 63 scripts | Fresh Azure VM from Phase 63 scripts (same image, Standard security type, SB/HVCI off); pre-load snapshot; full pipeline; capture proof | ✓ |
| Different approach | Different provider or local Hyper-V | |

**User's choice:** Reprovision via 63 scripts

### README scope (SC4)
| Option | Description | Selected |
|--------|-------------|----------|
| Both pipelines, full commands | C driver build+test-sign+load AND Rust client build/run, with exact commands + VM prereqs | ✓ |
| Driver build only | `drivers/README.md` covers the C driver; client documented in its own crate README | |

**User's choice:** Both pipelines, full commands

---

## Claude's Discretion

- Exact deny-target path string + harness language (PowerShell vs tiny Rust/C exe).
- Exact `NonoIpcRequest` field widths + the static-assert value `N`.
- The chosen altitude number within the Activity-Monitor band (after `fltmc filters` enumeration).
- The spike crate's name + exact directory.
- C-side static assertion mechanism (C11 `_Static_assert` vs WDK-compatible check).

## Deferred Ideas

- DRV-04 go/no-go ADR + measured round-trip latency → Phase 65.
- MACOS-03 live macOS re-validation + green-CI hard gate → Phase 65.
- EDR-01/02 HUMAN-UAT under a real EDR → Phase 66.
- DRV-PROD-01 production EV/WHQL signing + MSI-bundling → future milestone (gated on DRV-04).
- `729697c2` `--trust-proxy-ca` (P2) + non-macOS UPST8 slice → deferred.
- `NonoIpcRequest` version/request-id ABI-insurance fields → production-ADR consideration.
- Official Microsoft altitude assignment → tracked, pending Microsoft reply.
