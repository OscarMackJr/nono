# Upstream Parity Strategy (continue / split-windows / freeze-at-v0.52)

**Status:** Accepted
**Date:** 2026-05-11
**Phase:** 33 (v2.4 windows-parity-upstream-0-52-divergence)
**Decision IDs:** D-33-A1, D-33-A2, D-33-A3, D-33-B1, D-33-B2, D-33-B3, D-33-C1, D-33-C2, D-33-C3, D-33-C4, D-33-D1, D-33-D2
**Related artifact:** [DIVERGENCE-LEDGER.md](../../.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md)

## Context

The fork's last upstream sync was Phase 22 UPST2 (v0.38–v0.40.1, shipped 2026-04-28). Upstream `always-further/nono` is at v0.52.0 and has accumulated twelve minor releases of feature divergence: **97 non-merge commits** grouped into **12 themed clusters** per the Wave 1 audit (see [DIVERGENCE-LEDGER.md § Cluster Summary](../../.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md#cluster-summary)). The audit's dispositions break down as **8 `will-sync` clusters**, **2 `fork-preserve` clusters** (pack migration v0.44 + proxy TLS-interception v0.51 — both manual-replay per D-20 to protect Windows-specific wiring), and **2 `won't-sync` clusters** (PTY attach polish v0.41 — ConPTY structurally different per D-11; Unix-socket capability v0.42 — Unix-only by construction, would violate D-19 if added to `crates/nono/`).

Phase 25's HUMAN-UAT (2026-05-10) surfaced G-25-DRIFT-01: a speculative hypothesis that all four RESL flags shipped by Phase 25 (`--memory`, `--cpu-percent`, `--max-processes`, `--timeout`) were deprecated or renamed in upstream v0.52. **The Wave 1 audit empirically disproved this specific hypothesis.** Zero commits matching the four RESL flag rename keywords appear anywhere in v0.40.1..v0.52.0 (see [DIVERGENCE-LEDGER.md § Headline](../../.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md#headline) — `CRITICAL audit finding`); upstream still ships the four flags under their original names at HEAD `54f7c32a` (audit date 2026-05-11). Phase 25's source-level closure remains intact regardless (the cgroup v2 / setrlimit backends correctly enforce against the values they receive — backend correctness is independent of flag naming). G-25-DRIFT-01 was the *premise* that motivated the audit; the audit *disproved* the specific RESL-rename hypothesis but uncovered substantive divergence elsewhere — the 12 themed clusters above — that justifies the strategic decision regardless of the originating premise.

Beyond the parity gap, this ADR resolves a strategic question that's been growing since v2.1: the fork has accumulated significant Windows-only surface area — broker-process architecture (Phase 31 `crates/nono-shell-broker/` and `WindowsTokenArm::BrokerLaunch` dispatch arm), WFP service + filtering (Phases 6/9), ConPTY shell (Phase 8/30 `pty_proxy_windows.rs`), Authenticode chain-walker (Phase 28 `parse_signer_subject` / `parse_thumbprint`), Sigstore broker self-trust-anchor (Phase 32), TUF cached-root (Phase 32 `load_production_trusted_root`), and the NONO_TEST_HOME seam (Phase 27.1) — none of which has an upstream analog. The drift tool's D-11 path filter (`*_windows.rs` + `exec_strategy_windows/` excluded) is structurally blind to this surface; the Wave 1 ledger enumerates it manually per D-33-A3. Every upstream sync from here on will hit fork-only files. The question is whether continued bidirectional parity is sustainable, or whether the Windows-specific work belongs in a separate downstream that periodically pulls from upstream rather than chasing parity.

This ADR scores three options against five criteria using the audit data as evidence and picks one.

## Goals

This ADR commits to:

- A scored, falsifiable strategic verdict among three named options: `continue` / `split-windows` / `freeze-at-v0.52`.
- Per-option scoring across five equal-weighted criteria (D-33-C1):
  - **Maintenance cost** — per-sync labor, cherry-pick conflict count, manual-replay rate.
  - **Security posture** — Windows-only hardening vs upstream's threat model coverage.
  - **User clarity** — single-CLI-surface vs split-surface confusion.
  - **Contributor velocity** — Windows-PR latency, cross-platform-PR latency, PR review burden.
  - **Roadmap optionality** — which option keeps the most v2.4+ doors open vs forecloses options.
- Qualitative Low/Med/High scoring (D-33-C2) with 1-2 sentence rationale per cell — no false-precision integer scale.
- A documented tiebreaker (D-33-C3) grounded in PROJECT.md core value ("Windows security must be as structurally impossible and feature-complete as Unix platforms ... dangerous bits ... kernel-enforced") favoring the security-posture column when aggregate L/M/H shapes tie.
- A `Future audit cadence` consequence so downstream maintainers know when `make check-upstream-drift` should run going forward under the chosen option.
- A clear handoff to the UPST3-sync follow-up phase (Phase 34, queued in ROADMAP by Wave 3 / Plan 33-03) for the actual cherry-pick / manual-replay work the ledger catalogues.

## Non-goals

This ADR explicitly does NOT commit to:

- Executing any cherry-picks, manual replays, or code changes that close divergences. — The UPST3-sync follow-up phase (Phase 34, queued in ROADMAP by Wave 3 of this phase) does that.
- Closing G-25-DRIFT-01. — Wave 1's audit empirically disproved the specific RESL-rename hypothesis, but the gap's `status: open` field stays open per D-33-D2 until the UPST3-sync follow-up either lands renames (if upstream introduces them) or formally re-classifies the gap as `closed: no-divergence` once the audit-walk finding is recorded.
- Per-row will-sync vs fork-preserve dispositions. — The ledger handles per-cluster dispositions; this ADR scores the strategic question on top of those dispositions.
- Touching `crates/nono/`. — The D-19 byte-identical invariant holds trivially for this docs-only phase (`git diff --name-only -- crates/nono/` returns zero lines).
- Forecasting the v2.5+ milestone or any downstream sync's specific scope. — The ADR's consequences are the cadence rule; specific phase plans are their own work.

## Decision Table

| Option | Maintenance cost | Security posture | User clarity | Contributor velocity | Roadmap optionality | Verdict |
|--------|-----------|------------------|--------------|---------------------|---------------------|---------|
| **A (chosen) — Continue bidirectional parity** | **Med** — Per-sync labor sustains: 8 of 12 audited clusters carry `will-sync` (97 commits to absorb in the UPST3-sync phase). Phase 22 UPST2 precedent of 78 commits across 5 clusters shows the per-sync labor is sustainable but non-trivial; Phase 24's drift-audit tooling already in place makes per-release audits manageable. | **High** — Continued parity preserves the option to evolve Windows-only hardening (broker IL ladder, Authenticode chain-walker, TUF cached-root, WFP filtering, NONO_TEST_HOME seam) alongside upstream's threat model rather than letting either side drift. Upstream security fixes (e.g., v0.49 trust-scan path-traversal hardening, v0.42 NO_PROXY allow-domain hole, v0.46 deny-overlap re-validation) flow into Windows on the same release cadence. | **High** — Single CLI surface across Linux/macOS/Windows; matches the project Core Value documented in PROJECT.md ("Every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees"). One docs URL, one installer namespace, one `nono` binary identity. | **Med** — Drift-audit + cherry-pick gate adds review burden per release; Phase 24's drift-tool infrastructure (`make check-upstream-drift`) makes per-release audits manageable rather than per-PR. Per-sync labor surfaces in dedicated UPST*-sync phases between releases (the 8 `will-sync` clusters queued for Phase 34 are the labor-shape). | **High** — All v2.4+ doors stay open: re-merge with upstream, downstream split later if costs balloon, or freeze-at-vN as a future ADR — none are foreclosed by choosing A now. The chosen path is the only one of the three that preserves the option to reverse to either of the other two later. | **Accepted** |
| B — Split Windows into nono-windows fork | **Low** — Sync labor externalizes: nono-windows downstream pulls from upstream periodically rather than chasing per-release parity. | **High** — The 6-seam Windows-only hardening enumeration (broker dispatch, broker self-trust-anchor, TUF cached-root, Authenticode chain-walker, NONO_TEST_HOME seam, plus the `crates/nono-shell-broker/` crate) is unchanged — it just lives in a separate repo. | **Low** — "Which `nono` am I running?" confusion: end users would face two binary identities (upstream Linux/macOS `nono` vs downstream `nono-windows`) with subtly different CLI surfaces and docs URLs; violates PROJECT.md Core Value's "every command ... should work on Windows with equivalent security guarantees" framing. | **Low** — Cross-platform PRs need two reviews and two commit chains (one per repo); Windows-only PRs land in the downstream repo with separate CI infrastructure. Per-feature labor roughly doubles for any change crossing the platform boundary. | **Low** — Structurally hard to reverse: workspace splits are one-way operations (rejoining requires history-rewrite work plus migration of the downstream's standalone history). | Rejected: split foreclosure cost > parity labor saving. The 6-seam fork-only surface enumerated below is large enough to justify continued evolution, but the workspace split is a one-way operation; user clarity LOW and contributor velocity LOW outweigh the maintenance-cost saving. |
| C — Freeze fork at v0.52, stop chasing upstream | **Low** — Zero sync labor; fork becomes its own thing at v0.52 baseline. | **Med** — Windows hardening static at v0.52 baseline; upstream security fixes (e.g., the v0.49 trust-scan path-traversal fixes, v0.42 NO_PROXY hole closure) don't flow in. The cross-platform attack surface that Windows shares with Linux/macOS would stop receiving upstream's security evolution. | **Med** — Divergence documented but expected; users would understand "fork-of-nono frozen at v0.52" as a stable target, but every upstream feature lands as a fork-specific re-implementation rather than an absorbed change. | **High** — Fork becomes its own thing; no more cross-platform PR review burden; no more drift audits. Contributor velocity on fork-only features peaks. | **Low** — Forecloses re-merge with upstream; future strategic options (re-converge, security catch-up sync) require either a new ADR Superseding this one or a full fresh audit of however many releases have accumulated by then. | Rejected: forecloses upstream security flow-in. Roadmap optionality LOW; per D-33-C3 tiebreaker (PROJECT.md core value — "dangerous bits ... kernel-enforced"), the security-posture column leans against an option that statically freezes the cross-platform attack surface at v0.52 while upstream's threat model continues to evolve. |

The chosen option's aggregate L/M/H shape (Med, High, High, Med, High) dominates option B (Low, High, Low, Low, Low) and option C (Low, Med, Med, High, Low) on the 5-criterion sum: A has 3 High + 2 Med + 0 Low; B has 1 High + 0 Med + 4 Low; C has 1 High + 2 Med + 2 Low. The tiebreaker (D-33-C3) is not strictly needed — A's aggregate dominates without invoking it — but D-33-C3 is named explicitly in the Decision section so future maintainers re-evaluating can audit the reasoning trail.

## Decision

**Option A — Continue bidirectional parity in this repo.** Every upstream release continues to be sync-audited via Phase 24's `make check-upstream-drift` tooling and absorbed in dedicated UPST*-sync phases (with `fork-preserve` clusters manual-replayed per D-20 and `won't-sync` clusters explicitly documented). Windows-only surface continues to grow alongside upstream's cross-platform surface within the same workspace.

The aggregate L/M/H shape (Med, High, High, Med, High) dominates option B (Low, High, Low, Low, Low) and option C (Low, Med, Med, High, Low) without invoking the D-33-C3 tiebreaker. Per the tiebreaker as documented (PROJECT.md core value — "Windows security must be as structurally impossible and feature-complete as Unix platforms ... dangerous bits ... kernel-enforced ... without compromising the supervisor-led security model") — the security-posture column would lean toward A even if A and B had tied; A's High security-posture verdict reflects that continued parity is the only option of the three that lets upstream security fixes flow into Windows on the same release cadence while the fork-only Windows hardening continues to evolve.

The pick is reversible. If the per-sync labor ever exceeds the parity benefit (e.g., the 97-commit / 8-cluster will-sync queue grows materially in the next release cycle), a future ADR can Supersede this one and pivot to option B or C. Choosing A now preserves that future optionality; choosing B or C now would foreclose it.

### Fork-only surface area (D-33-A3 evidence)

The decision rests in part on the size and ownership of the fork-only Windows surface area. Per [DIVERGENCE-LEDGER.md § Fork-only surface area](../../.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md#fork-only-surface-area):

- **`crates/nono-shell-broker/`** crate — Phase 31 Low-IL broker process (Windows-only by design; landed in the Cargo workspace `members` array post-v0.40.1).
- **Phase 27.1 NONO_TEST_HOME seam** — `crates/nono-cli/src/cli_bootstrap.rs` (cross-platform conditional but introduced post-v0.40 with no upstream analog).
- **Phase 28 Authenticode chain-walker** — `parse_signer_subject` + `parse_thumbprint` helpers within `crates/nono-cli/src/exec_strategy_windows/` (D-11 excluded from drift-tool walks).
- **Phase 31 broker dispatch** — `WindowsTokenArm::BrokerLaunch` arm in `crates/nono-cli/src/exec_strategy_windows/launch.rs` ~L1246-1438 (D-11 excluded).
- **Phase 32 Sigstore TUF cached-root** — `crates/nono/src/trust/bundle.rs::load_production_trusted_root` (cross-platform per D-32-15 but introduced post-v0.40 with no upstream analog).
- **Phase 32 broker self-trust-anchor** — verify gate at `crates/nono-cli/src/exec_strategy_windows/launch.rs` ~L1246+ (Windows-only; D-11 excluded).
- **8 `*_windows.rs` files** (verified at audit time via `git ls-files | grep -E '_windows\.rs$'`): `exec_identity_windows.rs`, `learn_windows.rs`, `open_url_runtime_windows.rs`, `pty_proxy_windows.rs`, `session_commands_windows.rs`, `trust_intercept_windows.rs`, `exec_identity_windows.rs` (Windows-only test), `crates/nono/src/supervisor/socket_windows.rs`. Plus the entire `crates/nono-cli/src/exec_strategy_windows/` subtree (D-11 directory-glob excluded).

None of this surface has an upstream analog. Continued parity (option A) preserves the option to evolve this Windows-only hardening alongside upstream's threat model rather than letting either side drift. The fork-only surface is large enough that any cherry-pick / manual-replay against `fork-preserve` clusters (2 of 12 audited clusters — pack migration v0.44 and proxy TLS-interception v0.51) must explicitly avoid deleting fork-only wiring per D-20; the size of this enumeration is the primary security-posture evidence for choosing A over either alternative.

## Consequences

### Positive

- **Single CLI surface preserved.** Users on Linux, macOS, and Windows continue to install one `nono` binary, read one set of docs, and run one CLI; matches PROJECT.md Core Value.
- **Upstream security fixes flow into Windows on release cadence.** v0.42 NO_PROXY hole, v0.46 deny-overlap re-validation, v0.47 path-canonicalization fallback, v0.49 trust-scan path-traversal hardening — all reach the Windows attack surface via the same UPST*-sync phases that absorb cross-platform refactors.
- **Drift-audit tooling pays off per release.** Phase 24's `make check-upstream-drift` + the ledger convention established by Phase 33 Wave 1 (12 themed clusters / 97 commits / 2-tier disposition tables) become the recurring audit shape; per-release audit labor is bounded and the methodology is documented.
- **Windows-only hardening continues to evolve.** Broker dispatch, broker self-trust-anchor, Authenticode chain-walker, TUF cached-root, NONO_TEST_HOME seam stay in the same workspace where upstream's cross-platform refactors can be considered alongside them; future phases can extend or refactor either set without cross-repo coordination.
- **All v2.4+ strategic doors remain open.** Re-merge with upstream, downstream split later if labor balloons, freeze-at-vN as a future Superseding ADR — none are foreclosed.

### Negative

- **Per-sync labor sustains and grows with fork-only surface.** Each future upstream release triggers a drift audit + UPST*-sync phase; the 97-commit / 12-cluster v0.41–v0.52 audit is the labor shape, and as the fork-only Windows surface grows (e.g., future broker hardening, additional `*_windows.rs` runtime arms), cherry-pick conflicts in cross-platform files will grow proportionally.
- **2 `fork-preserve` clusters require manual replay** rather than clean cherry-pick (D-20 pattern): pack migration v0.44 (Phase 18.1-03 widening would be deleted by cherry-pick) and proxy TLS-interception v0.51 (Windows credential-injection rewrite would be deleted). Each requires a maintainer to read the upstream commit + the fork-only wiring it would overwrite, and replay the *intent* of the upstream change without the *form*.
- **1 `won't-sync` cluster (Unix-socket capability v0.42) leaves a permanent typed-capability gap.** Upstream has `UnixSocketCapability` + `--allow-unix-socket`; the fork structurally cannot adopt either (Unix-only by construction; would violate D-19 if added to `crates/nono/`). Fork users on Linux see a missing flag relative to upstream's docs; this stays documented as intentional non-port.
- **Drift-audit + cherry-pick review burden recurs per release.** Each UPST*-sync phase ships its own PR with audit-driven scope; reviewer attention is concentrated rather than amortized.

### Future audit cadence

Every upstream minor release (v0.53.0, v0.54.0, ...) triggers a drift audit via `make check-upstream-drift ARGS="--from v0.52.0 --to v0.5N.0 --format json"` (with `--from` advancing on every UPST*-sync close). UPST*-sync phases land between releases as needed (Phase 34 will be UPST3 for v0.41–v0.52; subsequent releases get UPST4, UPST5, ...). Audits are not on a fixed time schedule — they're triggered by upstream releases — and a single audit can absorb multiple minor releases at once if cherry-pick labor warrants it (Phase 33 itself absorbed 12 minor releases). The audit cadence is "per upstream release, lazily-evaluated"; if upstream goes quiet for a quarter, no audit fires; if upstream ships v0.53 and v0.54 in quick succession, a single UPST4 phase can cover both. The Phase 24 drift-tool infrastructure remains the source of truth for "what diverged"; the per-cluster disposition pattern established by Phase 33 Wave 1 is the recurring artifact shape.

## Alternatives Considered

### Option B — Split Windows-only surface into a separate downstream

This option would have moved the entire fork-only Windows surface — `crates/nono-shell-broker/`, the `*_windows.rs` files, the `exec_strategy_windows/` subtree, the broker self-trust-anchor and TUF cached-root logic, and the NONO_TEST_HOME seam — into a dedicated `always-further/nono-windows` (or similarly-named) downstream repo. That repo would periodically pull from upstream `always-further/nono` rather than chasing per-release parity, externalizing the per-sync labor.

The Decision Table scores B with maintenance-cost **Low** (sync labor externalizes), security-posture **High** (Windows hardening unchanged, just relocated), user-clarity **Low** ("which `nono` am I running?" confusion), contributor-velocity **Low** (cross-platform PRs need two reviews / two commit chains), and roadmap-optionality **Low** (workspace splits are structurally one-way operations). The aggregate (1 High / 0 Med / 4 Low) is dominated by option A's (3 High / 2 Med / 0 Low).

The decisive evidence against B is the structural irreversibility cost. Workspace splits cannot be cheaply undone: rejoining requires history-rewrite work, migration of the downstream's standalone CI infrastructure, and reconciliation of any divergent dependency pinning that accumulated in the meantime. Choosing B now would foreclose the option to reverse to A or to evolve to C; choosing A now preserves the option to pivot to B later if per-sync labor ever exceeds the parity benefit. The user-clarity LOW cell is also load-bearing — PROJECT.md Core Value explicitly frames the project as "every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees," which a two-binary-identity world directly violates.

### Option C — Freeze fork at v0.52, stop chasing upstream

This option would have accepted the v0.52 baseline as the upstream-parity terminus: future upstream releases would not be absorbed; the fork would become its own thing at v0.52 with all subsequent upstream work re-implemented as fork-specific features rather than absorbed changes.

The Decision Table scores C with maintenance-cost **Low** (zero sync labor), security-posture **Med** (Windows hardening static; upstream security fixes don't flow in), user-clarity **Med** (divergence documented but expected), contributor-velocity **High** (no upstream parity gate on any PR), and roadmap-optionality **Low** (forecloses re-merge with upstream). The aggregate (1 High / 2 Med / 2 Low) is dominated by option A's (3 High / 2 Med / 0 Low).

The decisive evidence against C is the security-posture Med cell. Per D-33-C3 tiebreaker, PROJECT.md Core Value ("dangerous bits ... kernel-enforced") leans the security column when aggregate shapes are close. C statically freezes the cross-platform attack surface at v0.52 while upstream's threat model continues to evolve — the v0.49 trust-scan path-traversal hardening, the v0.46 deny-overlap re-validation, the v0.42 NO_PROXY hole closure, and all future upstream security fixes would never reach the fork's Windows users on the cross-platform code path they share. The roadmap-optionality LOW cell amplifies this: any future strategic re-evaluation would require either a fresh full audit (against whatever upstream baseline has accumulated by then) or a Superseding ADR rebuilding the case from scratch. Continued parity at the cost of per-sync labor (option A's Med maintenance cell) is a structurally better trade than freezing security posture in exchange for zero labor (option C's Med security cell).

## References

### Internal

- [`33-SPEC.md`](../../.planning/phases/33-windows-parity-upstream-0-52-divergence/33-SPEC.md) — Locked requirements REQ-1..5 (REQ-2 is this ADR's acceptance contract).
- [`33-CONTEXT.md`](../../.planning/phases/33-windows-parity-upstream-0-52-divergence/33-CONTEXT.md) — Decisions D-33-A1..A3 (audit invocation + fork-only surface), D-33-B1..B3 (ledger schema), D-33-C1..C4 (ADR scoring methodology), D-33-D1..D2 (G-25-DRIFT-01 + UPST3 placeholder).
- [`DIVERGENCE-LEDGER.md`](../../.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md) — Wave 1 audit artifact (12 themed clusters / 97 commits / 8 will-sync + 2 fork-preserve + 2 won't-sync + fork-only surface enumeration); scoring evidence.
- [`25-HUMAN-UAT.md` § G-25-DRIFT-01](../../.planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md) — The gap that motivated the audit; Wave 3 / Plan 33-03 appends the `Update (Phase 33, 2026-05-11):` section recording the empirical-disproof finding.
- [`PROJECT.md` § Key Decisions](../../.planning/PROJECT.md) — Where this decision's summary row lives (Wave 3 / Plan 33-03 writes the row at the 3-column Key Decisions table, locked target per Wave 0 Open Question 1 resolution).

### Related ADRs (convention references)

- [`audit-bundle-target.md`](audit-bundle-target.md) — Phase 27.2 ADR (closest convention match per D-33-C4; this ADR mirrors its plain-text header + six-section structure).
- [`aipc-unix-futures.md`](aipc-unix-futures.md) — Phase 25 ADR (per-option scoring/verdict pattern).
- [`broker-trust-anchor.md`](broker-trust-anchor.md) — Phase 32 ADR (options + scoring + decision + consequences pattern).
- [`sigstore-tuf-cache.md`](sigstore-tuf-cache.md) — Phase 32 companion ADR (cross-cutting decision documentation).
