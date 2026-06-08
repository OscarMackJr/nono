# Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit - Pattern Map

**Mapped:** 2026-06-06
**Files analyzed:** 7 deliverables (2 tracks)
**Analogs found:** 3 strong / 7 (4 have NO in-repo analog — by design; this is groundwork producing new artifact types)

## Orientation

This is **GROUNDWORK** producing artifacts, not Rust source. There are no controllers/services/components. The "roles" below are artifact roles (WDK build project, design/threat-model doc, audit ledger, pointer stub) and the "data flow" column is repurposed as the production mode (compile-target, content-assertion doc, read-only-git audit, cross-ref stub).

**Two correctness corrections the planner MUST absorb (paths in CONTEXT/CLAUDE.md are stale):**

1. **`.planning/adr/` DOES NOT EXIST.** Verified: `ls .planning/adr/` → no such directory; `**/adr/**/*.md` glob → zero hits. The repo's real ADR home is **`docs/architecture/*.md`** (6 existing ADRs). D-09's "pointer stub in `.planning/adr/`" should either create that dir fresh OR (recommended, falls-out-cleaner per D-09's own escape hatch) live as a `docs/architecture/` ADR/pointer consistent with the 6 siblings. Flag this to the planner explicitly — do not silently write into a non-existent convention.
2. **`proj/DESIGN-*.md` DOES NOT EXIST.** Verified: `ls proj/` → no such directory; `**/DESIGN-*.md` → only phase SUMMARY files, no design docs. CLAUDE.md's "References" section cites `proj/DESIGN-library.md` etc. but those are not in the tree. The closest *real* design/threat-model convention is **`docs/architecture/*.md`** (the ADR set). Use those as the DESIGN.md analog, NOT `proj/DESIGN-*`.

## File Classification

| Deliverable | Role (artifact) | Production Mode | Closest Analog | Match Quality |
|-------------|-----------------|-----------------|----------------|---------------|
| `drivers/nono-fltmgr/nono-fltmgr.c` | WDK kernel skeleton (C) | compile-to-`.sys` | NONE in-repo — RESEARCH.md scaffold + MS `nullFilter` | no-analog |
| `drivers/nono-fltmgr/nono-fltmgr.vcxproj` (+`.filters`) | WDK MSBuild project | compile-to-`.sys` | NONE in-repo — RESEARCH.md + MS `nullFilter.vcxproj` | no-analog |
| `drivers/nono-fltmgr/nono-fltmgr.inf` | driver install config | INF (declarative) | NONE in-repo — RESEARCH.md + MS `nullFilter.inf` | no-analog |
| `drivers/nono-fltmgr/DESIGN.md` | design / threat-model doc (pre-code gate) | content-assertion doc | `docs/architecture/broker-trust-anchor.md` (threat-model ADR) | role-match |
| `.planning/adr/` pointer stub (D-09) | cross-reference stub | pointer | `docs/architecture/*.md` ADR header + `**Related ADR:**` line | role-match (see correction 1) |
| `63-DIVERGENCE-LEDGER.md` | upstream-divergence audit | read-only-git audit | `54-DIVERGENCE-LEDGER.md` + `42-…/DIVERGENCE-LEDGER.md` | **exact** |
| SC1 captures (`msinfo32`, `bcdedit /enum all`) | reproducibility evidence | VM-state capture | NONE in-repo (new artifact category) | no-analog |
| Altitude-request status artifact | human-gated record | content record | NONE in-repo (new artifact category) | no-analog |

---

## Pattern Assignments

### `63-DIVERGENCE-LEDGER.md` (audit ledger — HIGHEST-VALUE, exact analog)

**Primary analog:** `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` (most recent shape + disposition vocabulary + the C14 verdict this phase supersedes)
**Secondary analog:** `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` (the `windows-touch`-column template MACOS-01 mirrors with a `macos-only` column; also the first cycle where the touch-column actually fired)

**Frontmatter pattern** (copy from Phase 54 lines 1-14, adapt range + ledger_type + add a macos scope note):
```yaml
---
phase: 63-minifilter-spike-groundwork-macos-divergence-ledger-audit
plan: <NN>
ledger_type: macos-audit          # was: upst7-audit
range: v0.57.0..v0.61.2           # NOTE: v0.61.2 (3e605f27) NOT in local store — git fetch upstream --tags FIRST
upstream_head_at_audit: <sha-after-fetch>
refetch_date: 2026-06-06
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'make check-upstream-drift ARGS="--from v0.57.0 --to v0.61.2 --format json"'
fork_baseline: <UPST7 sync point>
total_unique_commits: <from drift tool AFTER fetch>
date: 2026-06-06
---
```
Phase 42's frontmatter (lines 1-13) is an equivalent shape with `slug:`/`status:`/`type: audit-only` keys instead — Phase 54's key set is the more current one to copy.

**Required sections, in order** (Phase 54 structure — all present there):
- `# … Divergence Ledger` title
- `## Headline` — disposition breakdown counts + the marquee finding. **For Phase 63 this MUST state the Phase-54-C14 supersession** (won't-sync → will-sync, v2.10 macOS-parity scope change). See Phase 54 lines 18-37 for the prose shape.
- `## Reproduction` — verbatim invocation, JSON output path (gitignored, NOT committed), the sha pin assertion, the auditor-rerun recipe. Phase 54 lines 39-50; Phase 42 lines 26-37.
- `## Cluster Summary` — the master table (see column pattern below) + per-cluster subsections.
- `## ADR review` — L/M/H verdict table across security/windows/maintenance/divergence/contributor (Phase 54 lines 317-336). For Phase 63 the ADR under review is `docs/architecture/upstream-parity-strategy.md`.
- `## Empirical cross-check` — ≥4 fork-shared files walked against the upstream log to prove no drift-tool gap (Phase 54 lines 338-376). **D-11 requires this be the re-export/call-site diff-inspect, not `--name-only`.**
- `## Cross-cluster re-export deps detected` — the `feedback_cluster_isolation_invalid` closure (Phase 54 lines 378-394).

**Cluster Summary table header** (copy EXACTLY — Phase 54 line 54, then swap `windows-touch` → `macos-only` per D-12/Phase-42 precedent):
```
| cluster_id | theme | commits | disposition | macos-only | rationale |
|------------|-------|---------|-------------|------------|-----------|
```

**Per-commit row table header** (copy EXACTLY — Phase 54 line 81 / Phase 42 line 63, swap column):
```
| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
```

**Disposition vocabulary** (the four locked values — Phase 54 uses all of them):
`will-sync` / `fork-preserve` / `won't-sync` / `split`

**Sample disposition row to mirror** — the exact C14 cluster this phase OVERRIDES (Phase 54 lines 300-315). The three P1 SHAs appear here as `won't-sync`; Phase 63 re-dispositions them `will-sync` (D-13):
```
| sha     | subject                                                       | upstream-tag | categories | files-changed | (macos-only) |
| 8f84d45 | fix(macos): emit platform rules after user write allows       | v0.58.0      | other      | 1             | yes          |
| 362ada2 | fix(sandbox): use $PWD to capture symlink CWD without --workdir| v0.58.0      | other      | 1             | yes          |
| 8f1b0b7 | fix(sandbox): preserve symlink path when adding CWD capability | v0.58.0      | other      | 1             | yes          |
```
(Note: Phase 54 marked these `windows-touch: no`; under the new `macos-only` column they flip to `yes`. The 7-hex form `8f84d454` etc. in CONTEXT/RESEARCH is the full SHA; Phase 54 abbreviates to 7 — match the tool output.)

**Cluster-subsection prose pattern** (per cluster: bold `**Commits:**`, `**Disposition:**`, `**Windows-touch:**`→`**macOS-only:**`, `**Rationale:**`, then a `**Cross-cluster re-export check:**` line for every will-sync cluster). See Phase 54 C3 (lines 111-128) for the canonical fully-worked will-sync cluster with the diff-inspect note, and C5 (lines 146-167) for the function-call-cross-cluster-dep case the pub-use scan alone misses.

**Diff-inspect note requirement (D-11 / RESEARCH Pattern 2):** every will-sync candidate's note must answer "does the fork's call site that produces the affected rule match the upstream site the commit patches?" The audit targets are:
- `crates/nono/src/sandbox/macos.rs::generate_profile` (the `8f84d454` ordering site; also `c6730e43` java_runtime 2-line touch)
- `crates/nono-cli/src/sandbox_prepare.rs` `resolved_workdir`/`cwd_canonical` (the `362ada22`+`8f1b0b74` symlink-CWD site)
- `crates/nono-cli/src/policy.rs` `add_platform_rule` call sites (shared deny/allow emission)

---

### `drivers/nono-fltmgr/DESIGN.md` (design / threat-model doc — role-match analog)

**Analog:** `docs/architecture/broker-trust-anchor.md` (a security/threat-model ADR; same "this doc gates dangerous code" purpose as the BSOD-avoidance gate). Secondary: `docs/architecture/sigstore-tuf-cache.md`, `upstream-parity-strategy.md`.

**Header convention to follow** (all 6 `docs/architecture/*.md` use this exact block — broker-trust-anchor.md lines 1-7):
```markdown
# <Title>

**Status:** Accepted            # or "draft" / "prototype slice" — Phase 63 doc is a pre-code gate, likely "Accepted (pre-code gate)"
**Date:** 2026-06-06
**Phase:** 63 (v2.10 …)
**Requirement:** DRV-03 (partial)        # some ADRs use **Decision IDs:** instead
**Related ADR:** [<sibling>](…)          # used to cross-link paired docs (e.g. broker-trust-anchor ↔ sigstore-tuf-cache)
```
Then a `## Context` section (every sibling opens with one — see broker-trust-anchor.md lines 9-17 for the "here is the danger and why this doc exists" framing).

**Threat-model table pattern** — copy the STRIDE-style structure already used in RESEARCH.md § Security Domain "Known Threat Patterns" (RESEARCH lines 504-513). That table IS the DESIGN.md's required content backbone. The doc MUST specify (D-10, asserted by grep at SC3):
- ring-buffer + worker-thread IPC pattern
- **no driver-originated file I/O** — no `ZwCreateFile`/`NtCreateFile`; `FltCreateFile` on own instance only
- finite `FltSendMessage` timeout (≈500 ms → `STATUS_TIMEOUT`, never infinite)
- `NonPagedPoolNx` for callback-reachable allocations
- IRQL assertions (`NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL)`)
- chosen altitude band (FSFilter Activity Monitor 360000–389999) + AV-range avoidance (320000–329998)
- Microsoft altitude-request status (date + `pending`)

The "options considered → decision" prose shape (broker-trust-anchor.md lines 19+, "Three trust-anchor options were considered…") is the right model if the planner writes DESIGN.md as an early draft of the DRV-04 go/no-go ADR (D-09 / Claude's-Discretion permits this).

---

### `.planning/adr/` pointer stub (D-09 — role-match, PATH CORRECTION NEEDED)

**Analog:** the `**Related ADR:**` cross-link line every paired `docs/architecture/*.md` carries (e.g. sigstore-tuf-cache.md line 6 ↔ broker-trust-anchor.md line 7 point at each other).

**Correction:** `.planning/adr/` does not exist (verified). The planner has two clean options, both consistent with D-09's "planner may flip which side is canonical":
- (a) create the pointer as a `docs/architecture/` entry consistent with the 6 siblings, with `**Related ADR:**` → `drivers/nono-fltmgr/DESIGN.md`; OR
- (b) if a `.planning/adr/` convention is genuinely wanted, create the directory fresh AND state in the stub that it's a new convention.
The pointer stub itself is one-line-plus-header: title + `**Status:**`/`**Date:**`/`**Phase:**` + a single sentence pointing at the canonical DESIGN.md. No deeper analog needed.

---

### `drivers/nono-fltmgr/` WDK scaffold — `.c` / `.vcxproj` / `.inf` (NO in-repo analog)

**Analog:** NONE. This is entirely new C/WDK code; `drivers/` does not exist in the tree and is deliberately NOT a Cargo workspace member. There is no Rust or in-repo C analog to copy from.

**Follow instead:**
- **RESEARCH.md § Code Examples (lines 257-371)** — prescriptive skeleton already written: `DriverEntry`+`FltRegisterFilter`+`FltStartFiltering`+`NonoFltUnload` (`.c`), the INF with `StartType = 3` (SERVICE_DEMAND_START, D-06) / `LoadOrderGroup "FSFilter Activity Monitor"` / `Instance1.Altitude = "370020"` placeholder, and the `msbuild … /p:Configuration=Release /p:Platform=x64` SC2 invocation.
- **Microsoft `nullFilter` sample** `[CITED: github.com/microsoft/Windows-driver-samples/tree/main/filesys/miniFilter/nullFilter]` — copy `.vcxproj` `ConfigurationType=Driver` + `DriverType=WDM` + Spectre-mitigation flags + WDK `.props`/`.targets` imports rather than hand-authoring (RESEARCH "Don't Hand-Roll" lines 202-209).

**Hard scope guard (Pitfall C / RESEARCH lines 194-195, 240-242):** the `.c` skeleton must register an EMPTY callbacks array (`{ IRP_MJ_OPERATION_END }`) and do NO file I/O. Any `FLT_OPERATION_REGISTRATION` body, `FltCreateCommunicationPort`, or `ZwCreateFile` is Phase-64 scope creep and seeds the Pitfall-2 recursion-BSOD pattern DESIGN.md forbids. The produced `.sys` is a throwaway VM-local artifact — commit only the `drivers/nono-fltmgr/` source, NOT the binary.

**Do NOT touch:** `crates/nono-cli/data/windows/nono-wfp-driver.sys` (out-of-scope placeholder; the spike's `nono-fltmgr.sys` is a separate, non-MSI-bundled driver).

---

### SC1 captures + altitude-request artifact (NO in-repo analog)

New artifact categories with no precedent in the repo. SC1 = `msinfo32` export + `bcdedit /enum all` captured ON the Azure VM proving Secure Boot OFF / HVCI off / TESTSIGNING-capable (D-03). Altitude-request artifact = a content record of the email's send-date + `pending` status (D-07, human-gated send). Follow the RESEARCH § Validation Architecture "Phase Requirements → Test Map" (lines 469-475) for what each must contain; no copy-source needed.

---

## Shared Patterns

### Reproducible-audit frontmatter (the drift-tool pin)
**Source:** `54-DIVERGENCE-LEDGER.md` lines 1-14 (and `42-…` lines 1-13).
**Apply to:** the DIVERGENCE-LEDGER only.
The pinned drift-tool sha `0834aa664fbaf4c5e41af5debece292992211559` + locked `make check-upstream-drift` invocation + `upstream_head_at_audit` + `refetch_date` make the audit regenerable. The raw JSON is gitignored, NOT committed — the cluster tables are the canonical artifact. **Phase-63-specific precondition:** `git fetch upstream --tags` + `git cat-file -t 3e605f27` (must print `commit`) BEFORE running the tool — v0.61.2 is absent locally and the range silently truncates otherwise (RESEARCH Pitfall E).

### docs/architecture ADR header block
**Source:** all 6 `docs/architecture/*.md` (canonical: `broker-trust-anchor.md` lines 1-7).
**Apply to:** `DESIGN.md` + the pointer stub.
`# Title` → `**Status:**` / `**Date:**` / `**Phase:**` / (`**Requirement:**` or `**Decision IDs:**`) / optional `**Related ADR:**` → `## Context`. This is the fork's house style for any design/threat-model/decision doc; the DESIGN.md should match it so a planning reader recognizes the shape.

### Diff-inspect re-export/call-site audit (NOT `--name-only`)
**Source:** `54-DIVERGENCE-LEDGER.md` C3 (clean, lines 111-128) + C5 (function-call cross-cluster dep, lines 146-167) + `## Cross-cluster re-export deps detected` (lines 378-394); memory `feedback_cluster_isolation_invalid`.
**Apply to:** every will-sync cluster in the DIVERGENCE-LEDGER.
For each candidate, `git show <sha>` and inspect (a) added `pub use`/`pub mod`/`extern crate`/`pub(crate)` symbols AND (b) function-call dependencies on symbols from other clusters. Phase 43 proved a `--name-only`-isolated commit had a cross-cluster `pub use` prereq; Phase 54 proved a *function-call* one (C5→C3 `partition_allow_domain`) the pub-use scan alone missed.

---

## No Analog Found

| Deliverable | Role | Why no analog | Planner should use |
|-------------|------|---------------|--------------------|
| `nono-fltmgr.c` | WDK kernel skeleton | First C/WDK code in repo; `drivers/` is new | RESEARCH.md §Code Examples skeleton + MS `nullFilter` sample |
| `nono-fltmgr.vcxproj` (+`.filters`) | WDK MSBuild project | No WDK project precedent | Copy MS `nullFilter.vcxproj` (Driver/WDM config) |
| `nono-fltmgr.inf` | driver install config | No INF precedent | RESEARCH.md §Code Examples INF + MS `nullFilter.inf` |
| SC1 `msinfo32`/`bcdedit` captures | VM-state evidence | New artifact category | RESEARCH §Validation Test Map content spec |
| Altitude-request status artifact | human-gated record | New artifact category | RESEARCH D-07/D-08 content (band + AV-range + send-date + pending) |

---

## Metadata

**Analog search scope:** `.planning/phases/*/DIVERGENCE-LEDGER.md` (42, 54), `docs/architecture/*.md` (6 ADRs), `proj/` (absent), `.planning/adr/` (absent), `drivers/` (absent), `crates/nono/src/sandbox/macos.rs`, `crates/nono-cli/src/{policy,sandbox_prepare}.rs` (audit targets, not copy-from sources).
**Files scanned:** 2 ledger templates fully read; 6 architecture-ADR headers read; 4 directory-existence checks.
**Path corrections surfaced:** `.planning/adr/` and `proj/DESIGN-*.md` do not exist — real home is `docs/architecture/`.
**Pattern extraction date:** 2026-06-06
