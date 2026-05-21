# Phase 50: Corp-network TUF refresh via OS root store - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-21
**Phase:** 50-corp-network-tuf-refresh-via-os-root-store-replace-or-wrap-t
**Areas discussed:** Code placement / module shape, `tough` API surface, Test fixture transport, Cross-platform code path strategy

---

## Code Placement / Module Shape

### Q1: Where should the new TUF chain-walk code live?

| Option | Description | Selected |
|--------|-------------|----------|
| (b) New sibling module in nono-cli | `crates/nono-cli/src/trust_refresh.rs` — setup.rs delegates via single call. Phase 49 precedent for trust-flow logic in nono-cli. Tests colocated. | ✓ |
| (a) Inline in setup.rs | All chain-walk code as private fns in `crates/nono-cli/src/setup.rs`. Pro: single file. Con: setup.rs grows 200-400 lines. | |
| (c) Move into nono library | New module `crates/nono/src/trust/refresh.rs`. Pro: trust-root logic colocated. Con: forces nono lib to add ureq dep, breaking P32-CHK-002 / D-32-15 HTTP-free invariant. | |

**User's choice:** (b) New sibling module in nono-cli (Recommended)
**Notes:** Locked. Mapped to D-50-01.

### Q2: Free function or SetupRunner method?

| Option | Description | Selected |
|--------|-------------|----------|
| Free function | `pub fn refresh_production_trusted_root() -> nono::Result<TrustedRoot>`. Setup.rs handles [X/N] header + write. Swap-in for `TrustedRoot::production()`. | ✓ |
| SetupRunner method | Add as method on SetupRunner. Consistent with Phase 49 `from_file_step` idiom but forces struct internal exposure for tests. | |
| You decide | Lock during draft. | |

**User's choice:** Free function (Recommended)
**Notes:** Locked. Mapped to D-50-02.

---

## `tough` API Surface

### Q3: How to plug ureq into tough?

| Option | Description | Selected |
|--------|-------------|----------|
| (a) `tough::RepositoryLoader` + custom Transport | Define `UreqTransport(ureq::Agent)` impl `tough::Transport`. tough owns spec compliance; nono owns transport only. Lockfile already has tough 0.22.0 transitively. | ✓ |
| (b) Lower-level `tough::schema::Signed<Root>` loop | Walk chain manually using `tough::schema` primitives. Full control. Con: any bug breaks trust anchor; doubles spec-compliance review burden. | |

**User's choice:** (a) `tough::RepositoryLoader` + custom Transport impl (Recommended)
**Notes:** Locked. Mapped to D-50-04 + D-50-05. Surface (b) explicitly rejected; hand-rolled TUF signature verification forbidden by SPEC Req 3.

### Q4: Embedded v14 anchor source?

| Option | Description | Selected |
|--------|-------------|----------|
| sigstore-trust-root 0.7.0 PRODUCTION_TUF_ROOT const | Use `sigstore_trust_root::PRODUCTION_TUF_ROOT` const bytes (verified at `tuf.rs:60`). Stays synced with upstream. | ✓ |
| Embed our own copy via include_bytes! | Ship `crates/nono-cli/data/sigstore-tuf-root-v14.json` + `include_bytes!`. We own refresh cadence forever — recreates Phase 49 staleness class. | |
| Read anchor from disk at runtime | Release-asset mirror. Ops can rotate without rebuild. Con: new I/O step + first-run failure mode. | |

**User's choice:** Pull from sigstore-trust-root 0.7.0 const (Recommended)
**Notes:** Locked. Mapped to D-50-06. Verified `pub const PRODUCTION_TUF_ROOT: &[u8] = include_bytes!("../repository/tuf_root.json");` exists at sigstore-trust-root-0.7.0/src/tuf.rs:60.

---

## Test Fixture Transport

### Q5: Hermetic unit-test transport shape?

| Option | Description | Selected |
|--------|-------------|----------|
| (b) In-memory transport mock | `StaticMapTransport(HashMap<String, Vec<u8>>)` impl `tough::Transport`. No sockets, no ports, no race conditions. Same trait surface as production. <10ms per test. | ✓ |
| (a) Localhost HTTP server | tiny_http or axum on ephemeral port per test. Pro: real ureq path. Con: port-conflict flakes on parallel CI; dev-dep; Windows Defender interference; slower. | |
| (c) Snapshot-replay against pre-captured CDN responses | Capture real `*.root.json` once → fixtures → in-memory replay. Maximum fidelity. Con: snapshots rot at every Sigstore rotation. | |

**User's choice:** (b) In-memory transport mock (Recommended)
**Notes:** Locked. Mapped to D-50-08.

### Q6: TLS handshake test in-phase?

| Option | Description | Selected |
|--------|-------------|----------|
| Trust the dep — no TLS test | ureq+platform-verifier is widely-used and audited. TLS-handshake test requires localhost TLS + test CA install. HUMAN-UAT on corp-network host is dispositive. | ✓ |
| Add one TLS smoke test | One test against `https://valid-isrgrootx1.letsencrypt.org/`. Catches future ureq/platform-verifier regression. Network flake; opt-in only. | |

**User's choice:** Trust the dep — no TLS test in this phase (Recommended)
**Notes:** Locked. Mapped to D-50-09.

---

## Cross-platform Code Path Strategy

### Q7: OS gating policy?

| Option | Description | Selected |
|--------|-------------|----------|
| (a) Single cross-platform path | Call new code unconditionally on Linux + macOS + Windows. ureq+platform-verifier consults OS root store on each. Simpler. D-21 held by zero-behavior-regression, not zero-file-diff. | ✓ |
| (b) `#[cfg(target_os = "windows")]`-gate the new path | Windows uses new code; Linux/macOS keep `TrustedRoot::production()`. Strictest D-21 reading. Two paths to maintain; Linux corp bugs need new phase. | |
| (c) Cross-platform-capable but Linux/macOS default to upstream | Effectively (b) but with build hygiene. Lighter middle ground. | |

**User's choice:** (a) Single cross-platform path — always use the new code (Recommended)
**Notes:** Locked. Mapped to D-50-11 + D-50-12. SPEC's "Windows-only" reinterpreted as USER-IMPACT scope, not CODE-GATING scope.

### Q8: Future Linux corp-CA failure resolution path?

| Option | Description | Selected |
|--------|-------------|----------|
| Already covered by (a) | Single cross-platform path means future Linux reports are auto-resolved. No additional work. | ✓ |
| Open a new ticket then; defer | Treat as future bug to triage when reported. Matches v2.6 drain-then-sync. | |
| Backport in a v2.6.x patch | Ship patch lifting Windows-only gate when reported. | |

**User's choice:** Already covered by (a) (Recommended if (a) chosen)
**Notes:** Locked. Confirmation of D-50-12 — single-path covers Linux too without additional work.

---

## Claude's Discretion

- Error type mapping (which `NonoError` variant to wrap tough errors in) — suggested `NonoError::Setup(format!("Sigstore TUF refresh failed: {e}"))` for shape continuity with existing `refresh_trust_root_step` error wrapping.
- ureq Agent configuration knobs (timeout, retry, redirect policy) — researcher/planner picks standard defaults.
- Doc-update granularity for `windows-poc-handoff.mdx` — researcher/planner picks inline patch vs. small rewrite. Acceptance is "describes v0.53.x+ corp-network refresh works natively".

## Deferred Ideas

- **Upstream sigstore-rs PR (Surface (b))** — community-good contribution; not on v2.6 critical path.
- **Online freshness-probe with new HTTP client** — out of scope; Phase 32 D-32-03 future work, reuse `trust_refresh::*` helpers when implemented.
- **CI MITM proxy test rig** — explicit out-of-scope per round-2 choice.
- **Linux corp-CA UX docs** — auto-covered by D-50-11; no preemptive doc work.
