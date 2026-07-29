# Phase 108: UPST12 Divergence Audit - Context

**Gathered:** 2026-07-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Produce an authoritative, per-commit `108-DIVERGENCE-LEDGER.md` for the upstream
`nolabs-ai/nono` window `v0.66.0..v0.69.0` (tags `v0.67.0`, `v0.67.1`, `v0.68.0`,
`v0.69.0`), classifying every substantive commit (**adopt / adapt / skip / split**) with a
`windows-touch` flag, a `security-relevant` flag, and an ADR-review verdict per cluster —
and settle the `deny_domain` (#1374) default-deny posture in a standalone ADR.

**This phase audits, classifies, and decides `deny_domain` — only.** No cherry-picks, no
version bump, no absorb work. The ledger + `ADR-108` are the sole deliverables and gate
Phases 109/110/111 (and, per D-16, a proposed new Phase 112).

The `upstream` remote already points at `nolabs-ai/nono` (repointed in v3.3 Phase 94);
`upstream-legacy` retains `always-further/nono`. No remote relocation work this cycle.

</domain>

<decisions>
## Implementation Decisions

### Window, Provenance & Measured Shape

- **D-01:** Audit window is upstream `v0.66.0..v0.69.0`. The fork's confirmed upstream
  high-water mark is **v0.66.0** (absorbed in v3.4 / UPST11, Phases 98-100); crate version
  is `0.66.1`.
- **D-02:** Pin the window tip **SHAs, not tag names**, in the ledger's Reproduction block
  (`git fetch upstream` + `git ls-remote --tags`). SHA-not-tag guard — Phase 85/94/98
  precedent.
- **D-03:** The ledger resolves to **commits, not PRs** (carried forward from v3.4 D-03).
  This window proves why: REQUIREMENTS names *7* tool-sandbox refinement PRs, but **20
  commits** actually touch the tool-sandbox surface.
- **D-04:** The `260727-jkn` map's dispositions are a **starting hypothesis to
  re-confirm**, never ground truth (carried forward from v3.4 D-03). Its "~40 substantive
  commits" estimate is already known to be wrong — see the measured baseline below.

**Measured baseline (taken 2026-07-29 during discussion; re-verify at audit time, do not
copy forward blindly):**

| Measure | Value |
|---|---|
| Non-merge commits in window | **100** |
| — CODE (touches source) | 68 |
| — DEPS (only `Cargo.toml`/`Cargo.lock`) | 15 |
| — CI (only `.github/`, `Makefile`, `scripts/`) | 11 |
| — DOCS (only `docs/`, `CHANGELOG`, `*.md`/`*.mdx`) | 6 |
| Commits touching the tool-sandbox surface | 20 (8 pure, **12 entangled**) |
| Non-tool-sandbox code commits mapping to NO v3.6 requirement | **~28** (approximate — D-19 requires hand-verification) |

For scale: v3.4's UPST11 window was 14 substantive + 6 noise = 20 commits. This window is
roughly **3.4× the substantive volume** and the largest since UPST9.

### tool-sandbox Fencing (the highest-risk classification call)

- **D-05:** The 12 commits touching **both** tool-sandbox and other code get a **`split`**
  verdict with a **per-commit, path-level disposition** naming exactly which paths absorb
  and which defer. (`split` is already a sanctioned 4th disposition — v3.4 precedent.)
  A blanket "defer anything touching tool-sandbox" rule is **forbidden**: it would
  demonstrably drop `d5803b99` (port ranges #1398 = **PROF-03**), `d4927f95` (`@git:*`
  tokens = **PROF-02**), `ea334d2b` (musl build fix), and `8a4237f2` (seccomp
  `SeccompPolicy` refactor, 18 files incl. `bindings/c`).
- **D-06:** The deferral boundary is defined as an **explicit module set**, not a directory
  prefix: `tool-sandbox/`, `crates/nono-cli/src/command_policy.rs`,
  `crates/nono-cli/src/lineage_cgroup.rs`. All three are **absent from the fork** (verified
  2026-07-29; `command_policies` exists only in
  `crates/nono-cli/data/profile-authoring-guide.md`, zero `.rs` wiring).
  The ledger MUST record the measurement that the union of these three equals the
  directory-only set (**true for this window — 20 either way**), and the **next sync MUST
  re-measure rather than inherit that equality**. It holds by coincidence, not by rule:
  `command_policy.rs` already lives outside the directory.
- **D-07:** **Per-commit residue accounting** is mandatory for every `split` and `defer`
  commit: the ledger lists *every* path in the commit and marks each
  `absorb` / `defer` / `noise`. **A path appearing in no bucket is a ledger defect.** This
  makes silent loss structurally detectable instead of dependent on reviewer attention —
  it is the concrete mitigation for the "cluster isolation can be empirically false"
  anti-pattern.
- **D-08:** Phase 108 **flags and names paths**; it does **not** pre-compute hunk-level
  patches. Hunk extraction happens in the absorb phases where it can be compiled and
  tested. This preserves the phase's own "no cherry-picks" boundary and avoids stale
  pre-computed patches.
- **D-09:** The 7 named refinement PRs (#1280/#1322/#1325/#1384/#1394/#1413/#1417) are
  recorded **DEFERRED→v3.7** with the reason (base subsystem absent), *and* the ledger
  records the full 20-commit surface so v3.7 inherits the real work-list rather than the
  PR-level approximation.

### `deny_domain` (#1374) Posture — the headline decision

- **D-10:** `deny_domain` gets a **standalone ADR**: `proj/ADR-108-deny-domain-posture.md`
  (mirrors `ADR-86`/`ADR-98` phase-number naming). Independently citable by Phase 109,
  survives ledger archival. The ledger cross-references it; it does not duplicate the
  analysis inline.
- **D-11:** Phase 108 **settles** the adopt-vs-adapt call with a recommendation — it does
  not merely frame it. Direct v3.4 Phase 98 precedent (the audit wrote ADR-98 and handed
  Phase 99 a decided disposition). The audit already has the diff in hand.
- **D-12:** **Posture = ADAPT.** Absorb `deny_domain` as an **additional deny layer only**,
  evaluated before the allowlist. **Reject upstream's "activates the proxy on its own"
  path**: a deny-only profile MUST still fail closed, never become allow-all-except.

  *Grounding (verified 2026-07-29):* the fork's `crates/nono/src/net_filter.rs` already has
  `deny_hosts: Vec<String>`, a hardcoded cloud-metadata `DENY_HOSTS`, and a
  `FilterResult::Deny` arm. #1374 adds `deny_suffixes` (wildcard deny) plus a
  `with_denied_hosts(&[String])` builder — the domain list is **caller-supplied**, so
  **ADR-86 is not breached**: policy stays CLI-side, the library gains mechanism only. The
  real risk is posture: `HostFilter.strict` is documented *"when true, an empty allowlist
  denies instead of allowing"* and constructors set `strict: false`, so a profile carrying
  only `deny_domain` yields an empty allowlist that **allows** — default-allow. NET-01
  explicitly requires composing "without weakening default-deny".

  The ADR MUST still show its work (both options + fork-invariant impact); D-12 is the
  recommendation to argue for, not a licence to skip the analysis.
- **D-13:** The ADR is scoped to **`deny_domain` only**. SPIFFE (#1272) and the
  SigV4/sibling-route fixes (#1430/#1437) are additive with no fork-invariant conflict —
  ordinary cluster rows.

### Ledger Granularity & Noise Floor

- **D-14:** **Cluster-first, per-commit only where it matters.** Group all 100 commits into
  thematic clusters (NET, PROF, CORE, tool-sandbox, deps, CI, docs, security/residual),
  then resolve to per-commit rows **only** for clusters marked `will-sync` or `split`.
  Preserves per-commit rigor exactly where cherry-picks happen without 100 hand-written
  rows.
- **D-15:** Noise classification is **file-set based**, with the conventional-commit prefix
  used **only as a cross-check** to flag disagreements for review. The two disagree in this
  window: 38 commits carry `chore:`/`ci:`/`docs:`/`build:` prefixes but only 32 are
  non-CODE. A `chore:` that touches source is precisely the commit that gets wrongly
  skipped.
- **D-16:** The **15 dependency-only commits are their own cluster**, not noise —
  security-relevant, reconciled against the fork's own pins, `cargo audit` run. Precedent:
  v3.4 Cluster F (`sigstore-trust-root` pin) was dependency-only and produced a real
  dual-version cascade; the fork also carries its own `quinn-proto`/RUSTSEC pin.
- **D-17:** The **11 CI-only commits are ADAPT-by-default — never adopt verbatim**, each
  reviewed individually. Precedent: ADR-100, where upstream CI assumed a release-please bot
  the fork does not use, so verbatim adoption would have added a permanently-false `if:`
  condition (a silent false-assurance gap). The fork's `release.yml` now also carries
  Trusted Signing wiring. Blanket-skip is equally wrong — it would miss portable fixes like
  the musl build fix (`ea334d2b`).

### Requirement Coverage Gap & Overflow

- **D-18:** **The milestone under-scoped the window.** ~28 non-tool-sandbox code commits map
  to none of v3.6's 12 requirements, including a security-relevant subset: AWS auth for the
  MiTM proxy (#1195), declarative sandboxed OAuth capture + capture-boundary hardening,
  NVIDIA procfs mediation hardening (#1284), a trust-policy `predicate` field, the Linux
  execute-restriction `Refer` grant (#1397), seccomp supervisor-ancestry for orphaned
  descendants, the standalone `nono proxy` command (#1261), and the `allow_vars` env-strip
  fix (#1204).

  **Resolution: propose a new Phase 112 — "Security + Residual Sync."** Direct v3.1 Phase 87
  ("Security Sync") precedent. These fixes get their own fork-invariant review rather than
  riding along inside a release phase. Folding them into Phase 111 would mix security review
  with a release cut — the combination v3.1 deliberately separated.
- **D-19:** **Phase 108's SC4 cannot be satisfied as written** ("maps each will-sync cluster
  onto Phase 109/110/111"). This is treated as a **legitimate audit finding, not a phase
  failure** — surfacing it is arguably the audit's whole purpose. The ledger reports the
  gap, names the unmapped clusters, and **proposes a roadmap amendment** (new Phase 112 +
  its requirement). **The amendment requires operator approval before Phase 109 planning
  begins.** Phase 108 does not silently rewrite the roadmap.
- **D-20:** The ledger carries a **`security-relevant` flag** alongside the existing
  `windows-touch` flag, so the security subset is queryable as a set and Phase 112 inherits
  a precise work-list rather than a judgment call. Cross-cluster security fixes (e.g. the
  Linux `Refer` grant) would otherwise be hard to spot.
- **D-21:** **Every code commit's requirement mapping is hand-verified.** The 42-mapped /
  28-unmapped split recorded above came from a crude keyword matcher and is a **hypothesis
  only** (consistent with D-04). False-positive "mapped" commits are the dangerous
  direction — they look covered and silently are not.

### Carried Forward (not re-litigated)

- **D-22:** The mandatory **carve-out re-touch check** (Phase 94 D-04/D-05, expanded in 98
  D-07) runs again: for each accumulated fork carve-out, `git log <window> -- <exact path>`,
  and a **zero-hit result is recorded explicitly** as "clean — no re-touch in window."
  **Silence is not evidence.**
- **D-23:** Fork invariants that constrain every disposition in this ledger: the **Windows
  security model** (AppContainer / Low-IL / WFP / Job Object) and the **ADR-86 policy-free
  library boundary** must not regress. Any cfg-gated Unix touch flags a cross-target clippy
  requirement for the absorb phase (`cross` linux-gnu + `cargo-zigbuild` apple-darwin, both
  GREEN locally — **no PARTIAL→CI**, retired in v3.3 Phase 96).

### Claude's Discretion

- Cluster naming and count within the D-14 scheme.
- Ledger table layout and column ordering, provided `windows-touch` (D-23),
  `security-relevant` (D-20), disposition, and residue accounting (D-07) are all present.
- The exact internal structure of `ADR-108`, provided it shows both options and their
  fork-invariant impact before landing on the D-12 recommendation.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Scope source (authoritative for this milestone)
- `.planning/quick/260727-jkn-map-macos-0-69-parity-gap-phases-for-the/260727-jkn-PLAN.md` — the parity matrix + phase/milestone map v3.6 implements. **Hypothesis, not ground truth (D-04)** — its "~40 substantive commits" estimate measured 100.
- `.planning/quick/260727-jkn-map-macos-0-69-parity-gap-phases-for-the/260727-jkn-upstream-macos-inventory.md` — upstream feature/PR inventory v0.64→v0.69.
- `.planning/quick/260727-jkn-map-macos-0-69-parity-gap-phases-for-the/260727-jkn-fork-windows-coverage.md` — fork HEAD coverage (PRESENT/PARTIAL/ABSENT).

### Milestone definition
- `.planning/REQUIREMENTS.md` — v3.6 section: UPST12-01, NET-01..03, PROF-01..04, CORE-01/02, VERIFY-01, RLS-14 + architecture invariants + out-of-scope table.
- `.planning/ROADMAP.md` — Phase 108 goal + SC1-SC4 (note **SC4 is unsatisfiable as written** — D-19).

### Prior-ledger shape (the template to mirror)
- `.planning/milestones/v3.4-phases/98-upst11-divergence-audit/98-CONTEXT.md` — the closest analog; D-01..D-07 structure, carve-out re-touch check.
- `.planning/milestones/v3.4-phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md` — ledger format, cluster table, per-cell ADR verdicts.
- `proj/ADR-98-network-intent-disposition.md` — the standalone-ADR precedent ADR-108 mirrors.

### Fork invariants
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free library boundary and the Windows denial-rendering carve-out.
- `CLAUDE.md` — Library-vs-CLI boundary table; cross-target clippy MUST/NEVER; path-security and permission-scope rules.
- `.planning/templates/cross-target-verify-checklist.md` — single source of truth for the two cross-target clippy gates (no PARTIAL→CI).

### Code surfaces this audit reasons about
- `crates/nono/src/net_filter.rs` — `HostFilter`, `deny_hosts`, `DENY_HOSTS`, `FilterResult::Deny`, `strict` (the D-12 grounding).
- `crates/nono-cli/data/profile-authoring-guide.md` — the fork's only `command_policies` reference (docs-only, zero wiring — D-06).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **v3.4 Phase 98 ledger + ADR-98 pair** — the exact deliverable shape this phase reproduces at ~3.4× scale. Cluster table, `windows-touch` column, per-cell ADR verdicts, Reproduction block with pinned SHAs.
- **Existing deny mechanism in `net_filter.rs`** — `deny_hosts` + `DENY_HOSTS` + `FilterResult::Deny` already exist, so #1374 is an extension of a live surface, not a new subsystem.
- **`git log <window> -- <exact path>`** — the carve-out re-touch primitive from Phase 94/98, reused unchanged.

### Established Patterns
- **Four dispositions:** adopt / adapt / skip / split. `split` was validated in v3.4 and is load-bearing here (D-05).
- **Standalone ADR per headline decision** — `ADR-86`, `ADR-98`, now `ADR-108`. Phase-number naming; independently citable; survives archival.
- **Fork-ahead is a real category** — Job Object resource caps already exceed upstream's cgroup-only implementation, so CORE-02 is flag-alignment, not enforcement work.

### Integration Points
- Ledger dispositions feed Phases 109 (NET), 110 (PROF), 111 (CORE/VERIFY/RLS), and the proposed 112 (Security + Residual).
- `ADR-108` is a hard input to Phase 109 planning — 109 should not be planned before it lands.
- The D-19 roadmap amendment is an operator gate between Phase 108 and Phase 109 planning.

</code_context>

<specifics>
## Specific Ideas

- The four commits named in D-05 (`d5803b99`, `d4927f95`, `ea334d2b`, `8a4237f2`) are the
  concrete proof that PR-level tool-sandbox deferral is unsafe. Keep them cited in the
  ledger as the worked example justifying `split`.
- The `security-relevant` flag (D-20) should make the D-18 list reproducible as a query, so
  Phase 112's scope is derived from the ledger rather than re-litigated.
- The D-06 union measurement ("20 either way") is deliberately recorded as a **coincidence
  to re-test**, not a rule to inherit — the phrasing matters for whoever runs UPST13.

</specifics>

<deferred>
## Deferred Ideas

- **tool-sandbox subsystem + its 7 refinement PRs → v3.7** (Windows Tool-Sandbox Parity).
  Out of scope by REQUIREMENTS; the ledger records the full 20-commit surface (D-09) so
  v3.7 starts from real data.
- **Windows `tool-sandbox/platform/windows.rs` driver → v3.7.** Needs the
  SCM_RIGHTS→handle-passing + peer-auth spike and an adopt-vs-formalize ADR.
- **WFP daemon-path-only reachability / filter-leak hardening** — fork-internal, surfaced by
  the parity map; not upstream-parity work, tracked separately.
- **Gray areas raised but not discussed** (available if Phase 109/110 planning needs them):
  the `platform_overrides` adoption call (the `260727-jkn` map pre-declares full-adopt —
  worth confirming rather than inheriting); how the carve-out re-touch check scales at 100
  commits; whether the SPIFFE/SPIRE `testdata/spire/` fixtures and `scripts/spiffe-mock-server.py`
  come across with NET-02; and how CORE-02's "fork is already ahead" claim gets verified
  rather than asserted.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` — matched at score 0.2 on the keyword "phase" only. Host-gated v3.5 distribution item, already scoped to v3.5 Phase 106. Not related to an upstream divergence audit.
- `20260611-poc-cert-broker-clean-host.md` — same: score 0.2, keyword-only match, owned by v3.5 Phase 106.

</deferred>

---

*Phase: 108-upst12-divergence-audit*
*Context gathered: 2026-07-29*
