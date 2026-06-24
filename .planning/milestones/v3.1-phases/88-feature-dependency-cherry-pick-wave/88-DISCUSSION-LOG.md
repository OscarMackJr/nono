# Phase 88: Feature + Dependency Cherry-Pick Wave - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-20
**Phase:** 88-feature-dependency-cherry-pick-wave
**Areas discussed:** XDG state-dir vs Windows path, Dep bump policy & pin sync, Profile namespace rename, CR-01 FFI fix scope

---

## XDG state-dir vs Windows path (FEAT-02)

### Module reconciliation
| Option | Description | Selected |
|--------|-------------|----------|
| Adopt upstream module, redirect fork helpers | Cherry-pick state_paths.rs as single source of truth; fork config/mod.rs helpers delegate to it | ✓ |
| Extend fork helpers, port only net-new | Keep config/mod.rs as home; port only net-new migration logic | |
| You decide | Researcher/planner picks | |

### Windows path location
| Option | Description | Selected |
|--------|-------------|----------|
| %LOCALAPPDATA%\nono (match v3.0 provisioner) | Explicit Windows arm mapping reconciled with scratch-space provisioner | ✓ |
| Keep ~/.nono on Windows | Continue home-dir subtree on Windows, XDG only on Unix | |
| You decide | Planner verifies against provisioner | |

### Migration aggressiveness
| Option | Description | Selected |
|--------|-------------|----------|
| One-time auto-migrate (move), fail-secure | Move state once on first run; abort on any error | ✓ |
| Fallback-read only, no move | Read new, fall back to legacy, never move | |
| You decide | Planner follows upstream approach | |

**User's choice:** Adopt upstream module + %LOCALAPPDATA%\nono + one-time auto-migrate fail-secure.
**Notes:** Fork already has partial XDG helpers in config/mod.rs; dirs::state_dir() returns None on Windows so the Windows arm needs explicit mapping. The only Windows-touch decision in the wave.

---

## Dep bump policy & pin sync (DEPS-02)

### Pin policy
| Option | Description | Selected |
|--------|-------------|----------|
| Lock to exact upstream targets | cargo update -p to exact ledger-named versions | |
| Resolve latest-compatible | cargo update broadly within loose specs; hard-bump typify | ✓ |
| You decide | Planner per-dep | |

### Bump commit structure
| Option | Description | Selected |
|--------|-------------|----------|
| One DEPS commit for all bumps | Single atomic commit: typify spec + all lockfile bumps | ✓ |
| Split typify from lockfile bumps | typify isolated, lockfile separate | |
| You decide | Planner groups by code fallout | |

### 5-crate sync guarantee
| Option | Description | Selected |
|--------|-------------|----------|
| Explicit checklist gate in the plan | Explicit 5-crate path-dep pin verification before make ci | ✓ |
| Rely on make ci to catch it | Trust the build | |
| You decide | Planner decides | |

**User's choice:** Resolve latest-compatible + one DEPS commit + explicit 5-crate checklist gate.
**Notes:** Scout confirmed x509-parser and time are transitive (not direct deps) → lockfile-only; typify 0.6→0.7 is the only direct Cargo.toml spec edit. No contradiction with "latest-compatible" since transitive bumps land via cargo update.

---

## Profile namespace rename (Cluster L / FEAT-06)

### Adoption strategy
| Option | Description | Selected |
|--------|-------------|----------|
| Adopt namespace + keep bare-name aliases | Rename to namespace but alias old bare names for back-compat | ✓ |
| Adopt namespace wholesale (breaking) | Full rename, no aliases | |
| Defer the rename, absorb other FEAT-06 bits | Skip 6d88638e; still absorb #1113/#1136 | |

### Fork-only profiles
| Option | Description | Selected |
|--------|-------------|----------|
| Namespace fork-only profiles consistently | Apply convention to nono-ts-wfp-test-*, swival, Windows profiles too | ✓ |
| Leave fork-only profiles as-is | Only namespace upstream-shared profiles | |
| You decide | Planner decides | |

**User's choice:** Adopt namespace + bare-name aliases; namespace fork-only profiles consistently.
**Notes:** Fork has bare names in policy.json + builtin.rs and fork-only profiles upstream lacks; wholesale rename would break --profile invocations and Windows mappings. FEAT-06's CI-provider discovery + truthy-env bool flags absorbed regardless.

---

## CR-01 FFI fix scope

### Fix scope
| Option | Description | Selected |
|--------|-------------|----------|
| Clear-on-entry across all FFI entry points | Reset LAST_DIAGNOSTIC_CODE at start of every extern "C" fn | ✓ |
| Fix only the observed stale paths | Patch just the flagged paths | |
| You decide | Planner picks | |

### Test + commit structure
| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated FFI test + standalone fork-divergence commit | FFI test + own commit + ledger addendum (CR-02 pattern) | ✓ |
| Fold in without separate record | Inline fix, no ledger note | |
| You decide | Planner decides | |

**User's choice:** Clear-on-entry across all FFI entry points + dedicated test + standalone fork-divergence commit recorded in ledger.
**Notes:** CR-01 is a fork fix on inherited code (not a cherry-pick); upstream-identical at a6aa5995. Mirrors the Phase 87 CR-02 addendum pattern for future-sync conflict warning.

---

## Claude's Discretion

- set_vars (FEAT-01) env-name validation internals (requirement locks reject PATH + NONO_ prefix).
- AWS auth (FEAT-03) mutual-exclusion enforcement location (profile load vs proxy route config).
- $PACK_DIR / source_pack propagation details (FEAT-05); pack-verification dry-run skip internals.
- typify-0.7 codegen split decision if fallout is non-trivial.

## Deferred Ideas

- Cluster F proxy hardening → Phase 89 (PROXY-01/02); shares credential path with this phase's AWS auth.
- TlsInterceptIntent assessment (bd4b6b7f) → Phase 89.
- policy.json go_runtime go-build cache group (5413a0b3) → future policy.json sync pass (noise per drift filter).
