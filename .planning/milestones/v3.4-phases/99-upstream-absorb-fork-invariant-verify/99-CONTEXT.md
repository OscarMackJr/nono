# Phase 99: Upstream Absorb + Fork-Invariant Verify - Context

**Gathered:** 2026-06-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb the **will-sync clusters identified by the Phase 98 DIVERGENCE-LEDGER** for the
`nolabs-ai/nono` `v0.65.1..v0.66.0` window (`1d1c88c9..d817ed53`) into the fork, in
dependency order, and **prove** the Windows security model (AppContainer/WFP/broker), the
ADR-86 policy-free-library boundary, and the local cross-target clippy gates are
unregressed. Requirements: UPST11-02, UPST11-03, UPST11-04.

**The Phase 98 ledger is the binding scope input** (the phase goal says so explicitly:
"All will-sync clusters *from the Phase 98 ledger*"). The actual in-scope set is:

| Cluster | Disposition | Commits | Phase 99 action |
|---|---|---|---|
| A — NetworkIntent + contradictory-flag guard | full-sync-adopt (ADR-98 Accepted) | `72bcfd66` (#1225), `d457ecc3` (#1263) | Apply both; preserve WSL2ProxyFallback + CompiledEndpointPolicy deviations; **cross-target clippy MANDATORY** (only windows-touch cluster) |
| D — 9P filesystem warning | will-sync | `5b8e94da` | Additive `sandbox/linux.rs` diagnostic; cross-target clippy gate |
| E — org-ref migration (always-further→nolabs-ai) | will-sync | `c808f000` | Per-file review; **preserve `OscarMackJr/nono` fork-identity URLs** |
| F — sigstore-trust-root 0.8.0→0.9.0 | will-sync | `2e64798d` | Bump in `crates/nono/Cargo.toml`; sigstore-verify/sigstore-sign cascade check; Cargo.lock regen |
| G — proxy docs + deprecated-flag metadata | will-sync | `a4d68189` | `cli.rs`/`cli_bootstrap.rs`/`token.rs` accuracy fixes (incl. stale `X-Nono-Token`) |
| C — proxy HTTP/2 + endpoint routing | **split** | `cdeeb5b9` (#983), `46bcfbb9`, `08ca19a8` | Apply applicable hunks only; **drop `tls_intercept/` + `h2_forward`/`h2_probe`** |

**Explicitly OUT of scope** (won't-sync per ledger): Cluster B (tool-sandbox #1268/#1271/#1253/#1249 —
fork has no `tool-sandbox/` dir) and Cluster H (release metadata — Phase 100 leapfrog floor `0.66.1`).

**Not in scope:** crate version leapfrog, release pipeline, PyPI blocker (all Phase 100); adopting
the tool-sandbox subsystem; any TLS-interception capability.

</domain>

<decisions>
## Implementation Decisions

### Scope authority + SC/REQUIREMENTS reconciliation
- **D-01:** The **Phase 98 ledger is the binding scope** for Phase 99 — not the preliminary PR
  enumeration in the ROADMAP Phase 99 Success Criteria / REQUIREMENTS UPST11-02/03 (those came from
  the `260629-toe` quick-task guess off PR #1293's description and are now superseded by the
  commit-level audit).
- **D-02:** **Reconcile the stale SC text as a tracked Phase 99 task.** Edit ROADMAP Phase 99 SC #1/#2
  and REQUIREMENTS UPST11-02/UPST11-03 so they describe the ledger reality: tool-sandbox
  (#1268/#1271/#1253/#1249) is **won't-sync** (not "absorb"); PRs that are not will-sync clusters in
  the actual `v0.65.1..v0.66.0` window (#1127, #1207, #1213, #1229, #1232, #1251, #1247, #1246, #1235)
  are dropped or annotated as out-of-window/N-A with a one-line ledger cross-reference. Goal: the
  Phase 99 verifier checks against truth, not the preliminary list. Keep the per-PR→cluster mapping
  visible so the change is auditable.
  - **Caveat for the planner:** before deleting any SC-listed PR, confirm against the ledger window
    that it is genuinely out-of-window / already-synced (v3.3) — do not silently drop a PR that the
    audit may have folded under a different SHA. The ledger's per-cluster commit tables + D-03
    re-confirmation are the reference.

### Cluster C (split) absorption strategy
- **D-03:** **Hand-replay only the applicable hunks** for Cluster C. Replay `pool.rs` + the shared
  proxy surface (`route.rs` / `server.rs` / `reverse.rs`) + endpoint-route wiring. **Never stage**
  `tls_intercept/` or the `h2_forward` / `h2_probe` hunks (the fork has no TLS-interception surface —
  this is the Cluster F carve-out). Cite the upstream SHA in each commit trailer.
- **D-04:** **Verify `CompiledEndpointPolicy` compatibility** against the Phase 95 fork divergence as
  part of Cluster C absorption (the endpoint-wire commit `46bcfbb9` touches the shared endpoint-policy
  surface the fork already carries). The Phase 89 fail-secure proxy divergence must remain intact.

### Absorption mechanics (clusters A, D, E, F, G)
- **D-05:** **cherry-pick with `-x` trailer where it applies cleanly; fall back to manual replay only
  on conflict.** This preserves upstream authorship provenance for the clean clusters.
- **D-06:** **One atomic, DCO-signed commit per upstream commit** (`Signed-off-by: Oscar Mack Jr
  <oscar.mack.jr@gmail.com>`). No squashing across commits; no `will-sync` ledger row left open.
- **D-07:** **Dependency order: Cluster A (#1225 `72bcfd66`, then `d457ecc3`) first**, then D / E / F / G.
  Cluster C after A (it builds on the proxy/endpoint surface). Fold **Cargo.lock regeneration + the
  sigstore-verify/sigstore-sign cascade check into Cluster F**.

### Verification depth + Cluster A deviations
- **D-08:** **Author focused regression tests** that prove the two ADR-98-named Cluster A deviations
  survive #1225 adoption: (a) `WSL2ProxyFallback` preservation in `profile/mod.rs`, and (b)
  `CompiledEndpointPolicy` compatibility in `proxy_runtime.rs`. These are new, targeted tests — not a
  full new suite per cluster.
- **D-09:** **Run the existing gates in addition to the new tests:** the Phase 89 proxy guard tests
  (`denied_endpoint_returns_403_and_audit`, `allow_domain_endpoint_route_does_not_shadow_credential_route`),
  the linux.rs seccomp tests (`proxy_no_v4_seccomp` / `proxy_v4_no_seccomp`), **both** Unix cross-target
  clippy gates (`cross clippy` x86_64-unknown-linux-gnu + direct-binary `cargo-zigbuild clippy`
  x86_64-apple-darwin, `-D warnings -D clippy::unwrap_used`, **no PARTIAL→CI**), `make ci` (clippy +
  fmt + tests), and a code-review + verifier pass.
- **D-10:** Produce an explicit **fork-invariant carve-out checklist** with one entry each for:
  AppContainer/WFP/broker Windows backends, the ADR-86 policy-free-library boundary, and the
  `exec_strategy_windows/` denial-rendering carve-out — none marked regressed.

### Claude's Discretion
- Wave/plan breakdown is the planner's call. A natural shape: (1) SC reconciliation + Cluster A adopt
  with deviations + tests; (2) will-sync D/E/F/G; (3) Cluster C split hand-replay + endpoint-policy
  compat; (4) full fork-invariant verify gate (clippy both-gates + make ci + carve-out checklist +
  code-review/verifier). Planner may merge/reorder so long as D-07 dependency order and D-09 gates hold.
- Exact test names/locations for the D-08 deviation tests are the executor's call.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 98 audit outputs (the binding scope inputs)
- `.planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md` — **the binding scope**:
  per-commit cluster classification (A–H), windows-touch flags, six carve-out re-touch verdicts,
  per-cluster Phase 99 action notes, completeness sweep. Read the Cluster Summary table + each
  will-sync/split cluster body + the `## ADR Review` downstream-routing section.
- `proj/ADR-98-network-intent-disposition.md` — Cluster A #1225 disposition (**Accepted —
  full-sync-adopt**); names the Phase 99 deviations (WSL2ProxyFallback, CompiledEndpointPolicy) and
  the non-regression facts (library `NetworkMode::ProxyOnly` untouched; Windows enforcement reads
  derived `WindowsNetworkPolicyMode`).

### Fork invariants to preserve
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library boundary + the
  `exec_strategy_windows/` D-02 denial-rendering carve-out (referenced from CLAUDE.md).
- `CLAUDE.md` — cross-target clippy MUST/NEVER rules, library/CLI boundary table, path-security and
  unwrap policy, DCO sign-off requirement.
- `.planning/templates/cross-target-verify-checklist.md` — single source of truth for the two local
  cross-target clippy gates (pinned image tag, per-gate decision tree). Referenced by D-09.

### Milestone tracking (to reconcile per D-02)
- `.planning/ROADMAP.md` §"Phase 99" — SC #1/#2 to reconcile against the ledger.
- `.planning/REQUIREMENTS.md` — UPST11-02 / UPST11-03 / UPST11-04 to reconcile/verify.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets / Surfaces touched
- `crates/nono-cli/src/` — Cluster A NetworkIntent enum lands here (CLI-side; `capability_ext.rs`,
  `supervised_runtime.rs`, `command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`,
  `output.rs`, `terminal_approval.rs` carry the 7 cfg(windows) blocks).
- `crates/nono-cli/src/profile/mod.rs` — `WSL2ProxyFallback` (D-08 deviation test target).
- `crates/nono-cli/src/proxy_runtime.rs` — `CompiledEndpointPolicy` (D-04 + D-08 compat/test target).
- `crates/nono-proxy/src/` — Cluster C shared surface (`pool.rs` new module, `route.rs`, `server.rs`,
  `reverse.rs`); `tls_intercept/` is **absent in the fork and must stay absent**.
- `crates/nono/src/sandbox/linux.rs` — Cluster D additive 9P warning; also home of the SEC-01 AF_UNIX
  no-grant EPERM filter + cgroup-v2 module that prior syncs dropped — **do not regress** (cross-target
  clippy is the structural guard; Windows-host clippy is blind to cfg(linux)).
- `crates/nono/Cargo.toml` — Cluster F sigstore-trust-root exact-pin.

### Established Patterns / Constraints
- **Workspace = 5 crates** (nono, nono-cli, nono-proxy, nono-shell-broker, nono-ffi). Version/dep
  changes touch all relevant `Cargo.toml` + internal path-dep pins (see `[[project_workspace_crates]]`).
- **Library is policy-free** — Cluster A must NOT push CLI intent types into `crates/nono` core
  (ledger re-export scan confirmed NetworkIntent is `pub(crate)` within nono-cli only; keep it that way).
- **`--workspace --all-targets` is the real local gate** (a `--bin nono` gate hides nono-ffi
  exhaustive-match breaks).

### Integration Points
- Cluster A (#1225) is the deepest/first absorb; everything else layers after. Cluster C endpoint
  wiring intersects the Phase 95 `CompiledEndpointPolicy` shared surface and the Phase 89 fail-secure
  proxy divergence — both must survive.

</code_context>

<specifics>
## Specific Ideas

- The fork-identity URLs `OscarMackJr/nono` must be preserved during the Cluster E org-ref migration —
  only upstream `always-further → nolabs-ai` references get rewritten, never the fork's own identity.
- Cross-target clippy is non-negotiable here specifically because Cluster A touches 7 cfg(windows)
  files and Cluster D touches cfg(linux) — exactly the drift a Windows-host clippy cannot see.

</specifics>

<deferred>
## Deferred Ideas

- **Cluster B (tool-sandbox) carry-forward** — #1268/#1271/#1253/#1249 are won't-sync because the fork
  lacks the `tool-sandbox/` directory. Revisit only if/when the fork adopts the tool-sandbox subsystem
  (its own future phase). Recorded in the ledger; not Phase 99 work.
- **Cluster H release metadata / 0.66.1 leapfrog** — Phase 100 (release reconcile). Ledger anchors
  `0.66.1` as the collision-free floor above upstream `0.66.0`.
- **TLS-interception capability** — the dropped `tls_intercept/` + `h2_forward`/`h2_probe` hunks. The
  fork deliberately diverges (fail-secure proxy, no TLS-intercept). Not on any roadmap.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` — clean-host MSI distribution concern (host-gated); belongs to
  release/distribution work, not upstream-code absorb.
- `20260611-poc-cert-broker-clean-host.md` — clean-host POC-cert/broker concern (host-gated); same
  rationale. Both remain pending.

</deferred>

---

*Phase: 99-upstream-absorb-fork-invariant-verify*
*Context gathered: 2026-06-30*
