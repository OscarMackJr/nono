# Phase 72: nono-py Binding + In-Process-Exec Proof - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-14
**Phase:** 72-nono-py-binding-in-process-exec-proof
**Areas discussed:** Proof platform, API shape, LangChain realism, Contract doc home

---

## Proof platform (and Shape B / SC2 resolution)

This area went through several reframings as the user introduced new context (zt-infra.org).

### Initial platform question
| Option | Description | Selected |
|--------|-------------|----------|
| Linux only (Landlock) | Cleanest; only place Shape B is OS-enforced; matches zt-infra runtime | |
| Linux + macOS | Both Unix backends enforce self-confine | |
| Linux + Windows | Linux for Shape B, Windows broker for Shape A | |
| Windows | Milestone-theme match; SC2 unprovable in-process (apply preview-only) | ✓ |

**User's choice:** Windows.
**Notes:** User introduced **zt-infra.org** — a fail-closed agent security control plane (`POST /actions` allow/deny + crypto audit) that lists `nono` as an execution sandbox; its integration is a FUTURE phase. Despite the architectural constraint (Windows native has no Landlock/Seatbelt; `Sandbox::apply` is preview-only on Windows), the user chose Windows and stated the principle: **"We cannot deliver a fake proof. Windows native will never have Sandbox/Landlock capability — we can only achieve equivalent Windows functionality with several Windows-only components."**

### Follow-up: how to honor "Windows" without a fake SC2 proof
| Option | Description | Selected |
|--------|-------------|----------|
| Split: Win=Shape A, Linux=Shape B | Each shape proven where enforceable | |
| Windows-only, re-scope SC2 | Shape A only; defer Shape B | |
| Windows-only, BUILD real Shape B (spike first) | Net-new born-confined self-re-exec, gated on an in-phase Win11 spike | ✓ |
| Reconsider → Linux | Back to deployment-aligned Linux | |

**User's choice:** Spike Shape B inside 72, then build.
**Notes:** Phase 72 gates on a short in-phase spike proving the confined self-re-exec is sound on a real Win11 host (no privileged handle escapes before confinement); THEN build `confined_run` + `confine` and reword SC2/SC3 from Landlock language to the Windows-equivalent (born-confined at entrypoint). The leaky mid-life self-IL-drop is rejected as the mechanism. This overrides the roadmap's "skip research-phase for 72" line.

---

## API shape

### API route (where confined_run/confine reach the broker)
| Option | Description | Selected |
|--------|-------------|----------|
| Wrapper over nono.exe (CLI) | Shell out to installed broker; reuse audited trust gate; Unix keeps Sandbox::apply | ✓ |
| Promote broker into nono lib | Direct call, no subprocess; larger refactor of audited code | |
| Let me describe it | — | |

**User's choice:** Wrapper over nono.exe (CLI). Windows-only fork; Unix keeps `Sandbox::apply` path.
**Notes:** Matches the architecture's "binding is just a convenience wrapper over `nono run`."

### Policy input
| Option | Description | Selected |
|--------|-------------|----------|
| Profile name + allow overrides | Engine = profile (Phase 71 model) | |
| Raw CapabilitySet only | Caller assembles coverage by hand | |
| Both (profile OR caps) | Accept either; two code paths | ✓ |

**User's choice:** Both (profile OR caps).

---

## LangChain realism

| Option | Description | Selected |
|--------|-------------|----------|
| Real langchain dep, in examples/ | Real PythonREPLTool optional extra + runnable example + driving test | ✓ |
| Real langchain, test-only | Real dep but no shipped example | |
| Faithful exec() stand-in | Minimal harness, no langchain dependency | |

**User's choice:** Real langchain dep, runnable `examples/15_langchain_confined.py` + driving test.

---

## Contract doc home

### Where the E1–E5 doc lives
| Option | Description | Selected |
|--------|-------------|----------|
| nono repo proj/ (canonical) + link | `proj/DESIGN-engine-abstraction.md`, linked from nono-py docs | ✓ |
| nono-py docs only | Next to the binding; risks looking Python-specific | |
| Both repos, full copies | Max discoverability; copies drift | |

**User's choice:** nono repo `proj/` canonical + link from nono-py docs.

### E5 ↔ zt-infra mapping
| Option | Description | Selected |
|--------|-------------|----------|
| Yes — document the mapping | E5 maps to zt-infra `POST /actions` allow/deny + audit (forward-compat) | ✓ |
| Note as forward-ref only | One-line mention, no mapping detail | |
| Leave it out of 72 | Generic E1–E5, no zt-infra | |

**User's choice:** Yes — document the E5 → `POST /actions` fail-closed mapping now (integration itself is a future phase).

---

## Claude's Discretion

- Exact spike pass/fail instrumentation (observing "no privileged handle escapes before confinement" on the live host).
- `network.block` on/off for the file-confinement proof (`block:false` variant acceptable for file-only).
- Exact patch version of the 0.62.x `nono` / `nono-proxy` pin (match the published fork crate version).

## Deferred Ideas

- Full zt-infra.org integration (`POST /actions` client/adapter) — future phase.
- Linux/macOS Shape-B proof — Unix `Sandbox::apply` path stays in the binding, but Windows is the Phase 72 proof target.
- Promoting the broker into the `nono` lib — rejected this phase; may re-surface for the daemon (74).
- nono-ts parity — Phase 75 (the contract doc is authored binding-neutral so 75 reuses it).
