# Phase 56: Fine-grained Network Filtering - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-05
**Phase:** 56-fine-grained-network-filtering
**Areas discussed:** CLI + profile syntax, Denial & path-match semantics, nono why --host output, Default posture for partial scope

---

## CLI + profile syntax

### Q1 — Relation to upstream #960 syntax
| Option | Description | Selected |
|--------|-------------|----------|
| Adopt upstream verbatim | Match upstream flag/URL syntax exactly; parity + clean future syncs | ✓ |
| Upstream syntax + fork guardrails | Upstream syntax with fork-specific validation/error messaging | |
| Fork-tuned surface | Fork-specific divergent surface | |

### Q2 — Profile JSON field this phase?
| Option | Description | Selected |
|--------|-------------|----------|
| Both CLI + profile field | Add equivalent path/method field to profile schema alongside CLI | ✓ |
| CLI flag only this phase | Defer profile-schema field to a later phase | |
| Match upstream exactly | Mirror whatever surfaces #960 introduced | |

**Notes:** Research item — confirm from the `0ced085` diff whether #960 already shipped a profile field. If CLI-only upstream, the fork still extends the schema (consistent with upstream CLI semantics, not divergent). Must respect the Phase 55 JSONC + dual-key fail-closed deserializer path.

---

## Denial & path-match semantics

### Q1 — Path match
| Option | Description | Selected |
|--------|-------------|----------|
| Path-prefix match | Component-wise prefix; `/v1` allows `/v1/*` | ✓ |
| Exact path match | `/v1` allows only `/v1` | |
| Match upstream's choice | Mirror 75b2265 | |

### Q2 — Path normalization / traversal hardening
| Option | Description | Selected |
|--------|-------------|----------|
| Canonicalize + fail-secure | Decode, collapse `.`/`..`, normalize slashes; deny on residual traversal/ambiguity | ✓ |
| Normalize, lenient | Normalize but fall through to host-level allow on oddities | |
| Match upstream exactly | Adopt upstream normalization as-is | |

### Q3 — Denial shape on path/method mismatch
| Option | Description | Selected |
|--------|-------------|----------|
| 403 + audit entry | HTTP 403 + pre-credential audit/trace entry (SC2) | ✓ |
| Reuse existing denial path | Route through current host-denial shape + audit | |
| You decide (impl detail) | Planner picks, as long as hard-deny + pre-credential audit | |

**Notes:** Strong fail-secure posture. Research item — if upstream normalization is more lenient than the fork's, prefer the fork's stricter posture and document the divergence.

---

## nono why --host output

### Q1 — What to show for scoped hosts
| Option | Description | Selected |
|--------|-------------|----------|
| List the scoping rules | List each allowed path-prefix + methods (SC3) | ✓ |
| Rules + effective denial hint | Rules plus a one-line note on what's denied | |
| Match existing why format | Minimal extension of current structure | |

### Q2 — Add a request-level allow/deny tester?
| Option | Description | Selected |
|--------|-------------|----------|
| No — keep SC3 minimal | Surface rules only; note tester as deferred idea | ✓ |
| Yes — add request-test now | Evaluate a concrete path+method this phase | |

**Notes:** Request-level tester deferred to a future phase (new diagnostic capability beyond SC3).

---

## Default posture for partial scope

### Q1 — Bare entry meaning + bare/scoped combination
| Option | Description | Selected |
|--------|-------------|----------|
| Bare = allow-all; most-permissive wins | Bare keeps allow-all (backward compat); bare+scoped → union, bare re-opens host | ✓ |
| Bare = allow-all; scoped narrows per-host | Any scoped entry makes non-matching denied; coexisting bare = conflict | |
| Match upstream's combine rule | Adopt `partition_allow_domain` precedence | |

### Q2 — No-method scoped entry
| Option | Description | Selected |
|--------|-------------|----------|
| All methods on that path | Method unspecified = any method on the matched prefix | ✓ |
| Safe methods only (GET/HEAD) | Read-only default | |
| Match upstream default | Mirror #960 | |

**Notes:** Research item — confirm `partition_allow_domain` uses the same most-permissive combine rule; if upstream narrows-per-host, surface before locking. WATCH-ITEM captured in CONTEXT.md: most-permissive-wins means a stray bare entry silently widens a scoped host; `nono why` + docs should make effective openness obvious.

---

## Claude's Discretion
- Exact 403 vs proxy-internal denial encoding (hard-deny + pre-credential audit required).
- Method matching mechanics (case-normalization, multiple methods per entry) — follow #960; default case-insensitive method set per entry.

## Deferred Ideas
- `nono why` request-level allow/deny tester (`--host X --path /v1 --method POST`) — future phase.

## Carried forward (not re-discussed — locked by Phase 54 audit)
- C3 sync (`0ced085`+`75b2265`), TLS-intercept ordering fork-preserve verdict (`22e6c40` rider only), `credential.rs` byte-identical (`c9f25164`), `rcgen` `8e78daf` won't-sync.
