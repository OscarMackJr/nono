---
title: Upstream nono v0.44 → v0.59 Gap Analysis (Windows-native fork)
date: 2026-05-27
quick_id: 260527-sgo
---

# Upstream v0.44 → v0.59 Gap Analysis

**Scope:** Enumerate functionality added in UPSTREAM `github.com/always-further/nono`
releases v0.44 → v0.59 that is absent, partial, or Windows-stubbed in this
Windows-native fork (`C:\Users\OMack\Nono`, fork version `0.57.3`). The fork tracks
its own version line that is NOT aligned with upstream release numbers — this
document deliberately does not conflate the two.

> **Headline:** The fork has already AUDITED and SYNCED the entire upstream
> **v0.44 → v0.57.0** window through prior UPST3/UPST5/UPST6 phases. The genuine
> forward gap is **upstream v0.58.0 + v0.59.0** (the not-yet-executed "UPST7"
> backlog). A secondary, smaller gap set consists of items that were explicitly
> *deferred* or dispositioned `won't-sync` / `fork-preserve` *within* the
> v0.44–v0.57 window; most v0.44–v0.54 deferrals have since been closed in fork
> phases 36–50, leaving only a short residue.

---

## Section 1 — Sync high-water mark (confirmed)

**Confirmed high-water mark: upstream `v0.57.0`** — fully audited AND synced.

Cited sources (repo-local):

| Claim | Source line |
|---|---|
| UPST6 audit window = `v0.54.0..v0.57.0`; fork baseline = `v0.54.0` | `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md` frontmatter: `range: v0.54.0..v0.57.0`, `fork_baseline: v0.54.0 (Phase 43 + 45 UPST5 sync point)` |
| All 42 v0.55–v0.57 commits cherry-picked/replayed into fork history | `.planning/phases/48-upst6-sync-execution/48-VERIFICATION.md`: Truth #1 VERIFIED — "40 `Upstream-commit:` + 2 `Upstream-replayed-from:` = 42 upstream commits accounted for across all 9 clusters"; REQ-UPST6-02 SATISFIED |
| Post-v0.57.0 explicitly deferred to a future cycle ("UPST7") | UPST6 ledger Headline: "**Strictly silent on post-v0.57.0 per D-47-A4.** The 19 known post-v0.57.0 commits between `10cec984` and the audit-open upstream/main HEAD `807fca38` are deferred to UPST7" |
| v0.41–v0.54 history coverage chain | `.planning/quick/20260512-upstream-fork-release-grid/RESULT.md`: Phase 20 (v0.37 era), Phase 22 (v0.38–v0.40), Phase 34 (v0.41–v0.52), Phase 42/43 (UPST5 → v0.54). v0.41–v0.43 paper-trail backfill in `47-…/DIVERGENCE-LEDGER-v041-v043-backfill.md` |

**Was anything in v0.44–v0.59 already partially synced?** Yes — *everything* through
v0.57.0 is synced. The v0.44–v0.57 window was absorbed across fork phases 22/34/42/43/47/48.
Within that window several upstream features were *deferred at absorption time*
(`P34-DEFER-*` tracked in the release-grid RESULT.md). Verified against the current
tree, most have since been closed:

| Deferred item (origin) | Status in fork today | Evidence (file present in tree) |
|---|---|---|
| P34-DEFER-04b-2 profile drafts (`--draft`, `profile promote`) | **CLOSED** (Phase 36.5) | `crates/nono-cli/tests/profile_drafts_test.rs`; `cli.rs` has 16 draft/promote refs |
| P34-DEFER-04b-1 deprecated_schema module | **CLOSED** | `crates/nono-cli/src/deprecated_schema.rs` (348 LOC) |
| P34-DEFER-06-1 `yaml_merge` wiring | **CLOSED** | `crates/nono-cli/src/wiring.rs` (542 LOC); `crates/nono-cli/tests/yaml_merge_reversal.rs` |
| P34-DEFER-08a-1 Windows env-filter wiring | **CLOSED** | `exec_strategy_windows/launch.rs` consumes `allowed_env_vars`/`deny_vars`; `env_sanitization.rs` (392 LOC) shared into the Windows path |
| C1/C3 PTY scrollback + Unix-socket capability (Phase 34 `won't-sync`) | **N/A on Windows** (intentional) | Unix-only; Windows uses ConPTY + Named-Pipe AIPC |

The residual v0.44–v0.57 deferrals that remain genuinely open are minor (e.g.
`b5f0a3ab` deep ExecConfig refactor / `bbdf7b85` escape-quote structured-property
wiring, both macOS-learn-diagnostics-oriented; full `wiring.rs` idempotent JSON-merge
abstraction). They are NOT the focus of this analysis — the team's forward work is
v0.58 → v0.59.

---

## Section 2 — Gap matrix (grouped by theme)

Gaps below are the upstream **v0.58.0** + **v0.59.0** net-new capabilities (the UPST7
backlog), cross-referenced against the current fork tree. "In fork?" = result of
CHANGELOG + targeted grep of `crates/`. Upstream PR numbers were not surfaced in the
CHANGELOG bullets for most of these; where a `#NNN` appears it is cited, otherwise the
upstream tag is the checkable anchor.

Legend for Windows applicability: `cross-platform-core` / `windows-applicable (port)` /
`needs-windows-equivalent-design` / `unix-only-N/A`.

### Theme: credentials / keystore

| Feature/Change | Upstream tag | PR/ref | In fork? | Windows applicability | Notes |
|---|---|---|---|---|---|
| **Bitwarden credential source — `bw://` URI scheme** | v0.58.0 | — | **no** | cross-platform-core | Zero matches for `bw://`/`Bitwarden` in `crates/`. New keystore source alongside fork's existing `keyring://` / `env://` / `file://`. Pure Rust over the `bw` CLI / Bitwarden API; ports directly. Note CHANGELOG hardening: secret fields wrapped in `Zeroizing<String>` with in-place truncation — fork's `zeroize` posture aligns. |

### Theme: profiles / policy / packs

| Feature/Change | Upstream tag | PR/ref | In fork? | Windows applicability | Notes |
|---|---|---|---|---|---|
| **Profiles specify a target binary** (`--profile` binds an expected executable) | v0.58.0 | — | **no** | cross-platform-core | Zero matches for `target_binary` in `crates/`. Profile-schema addition + validation; cross-platform. Touches `profile/mod.rs` + `nono-profile.schema.json` (both fork-shared) — schema-collision check needed vs fork's canonical-sections. |
| **JSONC support for profile files** (comments / trailing commas) | v0.58.0 | — | **no** | cross-platform-core | Zero matches for `jsonc`/`JSONC`. Upstream restored a `jsonc-parser` dependency (v0.59 bug-fix). Pure parser swap on the profile-load path; cross-platform. |
| **`opencode` profile extracted from built-ins** | v0.59.0 | — | **partial / verify** | cross-platform-core | Fork carries an `opencode` profile already (pre-existing). Upstream's change is a *relocation* of the built-in into the registry-pack mechanism (same family as the v0.44 codex/claude-code pack migration the fork absorbed). Likely a no-op-to-small port; verify against fork's `policy.json`/pack layout. |
| Pack/profile verification hardening tail | v0.55–v0.57 | — | **yes** | — | Already synced via Phase 48 C1 (shadowing checks, pack signer identity verify). Listed only to mark the boundary; not a gap. |

### Theme: network / proxy / filtering

| Feature/Change | Upstream tag | PR/ref | In fork? | Windows applicability | Notes |
|---|---|---|---|---|---|
| **`allow_domain` accepts URL with path** (host + path scoping) | v0.59.0 | — | **no** | cross-platform-core | Fork's `network_policy.rs` only does `collect_allow_domain_port_warnings` (host:port detection); no path parsing. Proxy-layer enforcement in `nono-proxy` — cross-platform. NOT the same as the older v0.27-era `endpoint_rules` (which fork has). |
| **Fine-grained method + path restrictions in `allow_domain`** | v0.59.0 | — | **no** | cross-platform-core | New HTTP method+path matching in the proxy filter. Builds on the path-scoping item above. Enforced in `nono-proxy/src/{route,filter,server}.rs`; cross-platform. Highest-value net-new network feature in the window. |
| **TLS-intercept endpoint rules enforced before credential selection** | v0.59.0 | — | **partial / verify** | cross-platform-core | Ordering/security fix in the proxy CONNECT route. Fork has TLS-interception preserved as a fork-divergent surface (Phase 34 C11 `fork-preserve`); this upstream re-ordering may or may not apply cleanly — diff-inspect against fork's credential-injection rewrite. |
| **Proxy 502 handling: upstream error preserved, 502 reason-line sanitized, audit entry on connect failure** | v0.58.0 | — | **no / verify** | cross-platform-core | Proxy error-path hardening (`nono-proxy/src/server.rs`). Cross-platform; composes with fork's audit-event surface (Phase 23). Verify fork's proxy error path. |

### Theme: supervisor / exec / PTY / IPC

| Feature/Change | Upstream tag | PR/ref | In fork? | Windows applicability | Notes |
|---|---|---|---|---|---|
| **Session lifecycle hooks** (`session_hooks` profile field; run hooks at session start/stop) | v0.58.0 | — | **no** | needs-windows-equivalent-design | Zero matches for `session_hook`/`session_hooks` in `crates/`. Upstream gated `hook_runtime` **unix-only** ("hook_runtime module gated unix-only") — so the upstream impl is Unix-shell-oriented. The *capability* (run a vetted hook at session boundaries) is desirable on Windows but needs a Windows execution design (broker-spawned, Low-IL, no `fork`/`sh`). Treat as design + port. |
| **fd-based IPC replaced with named socket for URL-open helpers** + supervisor-loop keep-alive / read-timeout fixes / blocking-mode accepted connections | v0.58.0 | — | **no / needs-design** | needs-windows-equivalent-design | A cluster of supervisor IPC robustness changes (named unix socket for `URL open`, 5s read timeout, keep-alive when child closes IPC, `UnixSocketCapability` granted for supervisor socket in child sandbox). Upstream uses af_unix; the fork's Windows IPC is Named-Pipe AIPC (Phase 18). The robustness *intent* (don't drop the supervisor loop on transient child-socket close; bounded read timeouts) is portable to the Named-Pipe path but is not a literal cherry-pick. macOS/Linux side is cross-platform-core. |
| **`$PWD` captured for symlink CWD without `--workdir`** / symlink path preserved when adding CWD capability (macOS) | v0.58.0 | — | **no / verify** | unix-only-N/A (macOS-specific) | Two macOS-side CWD-symlink fixes. macOS-only; not Windows-relevant. Compose with fork's macOS sandbox layer if/when macOS is exercised. |
| **macOS: platform rules emitted after user write allows** | v0.58.0 | — | **no / verify** | unix-only-N/A | macOS Seatbelt ordering fix. Not Windows-relevant. |

### Theme: CLI / UX / diagnostics

| Feature/Change | Upstream tag | PR/ref | In fork? | Windows applicability | Notes |
|---|---|---|---|---|---|
| **Timeout constants centralized; user-facing timeouts configurable** | v0.59.0 | — | **partial** | cross-platform-core | Fork already absorbed `--startup-timeout` + `NONO_STARTUP_TIMEOUT` (Phase 48 C2). v0.59 generalizes the timeout-constant story (more user-facing timeouts configurable). Likely a small additive port on top of the synced startup-timeout surface. |
| **Suppressed denials annotated; `[save skipped]` annotation in suppress-save-prompt paths** | v0.59.0 | — | **no / verify** | cross-platform-core | Diagnostic/UX polish on denial + save-prompt output. Cross-platform; small. |
| **Canonical denial paths pre-computed (reduce filesystem I/O)** | v0.59.0 | — | **no / verify** | cross-platform-core | Diagnostic perf refactor in the denial path. Cross-platform; small. Aligns with fork's `try_canonicalize` work. |
| **Access-mode splitting via `rfind` (+ test coverage); overflow checks tightened** | v0.59.0 | — | **no / verify** | cross-platform-core | Small parsing/arithmetic-safety fixes. Cross-platform. Overflow-check tightening aligns with fork's `checked_`/`saturating_` arithmetic standard. |

### Theme: packaging / distribution / CI

| Feature/Change | Upstream tag | PR/ref | In fork? | Windows applicability | Notes |
|---|---|---|---|---|---|
| **Release artifact attestation + GitHub attestation workflow** (`actions/attest-build-provenance`) | v0.58.0 | — | **partial / different mechanism** | windows-applicable (port) | Upstream added build-provenance attestation to its release workflow. Fork has its own sigstore/TUF + Authenticode + MSI signing pipeline (Phases 28/32/49/50) and a known-broken `release.yml` (per project memory). This is CI-only and fork-divergent; evaluate whether upstream's attest-build-provenance composes with the fork's signing story rather than cherry-picking verbatim. |
| **PR size labels / PR summary workflow / artifact-attestation job reorder** | v0.58.0 | — | **no** | cross-platform-core (CI-only) | Repo-hygiene CI; low value to port, fork-divergent CI. |
| **`java_runtime` group + `java-dev` profile** | v0.58.0 | — | **no** | cross-platform-core | New built-in policy group/profile (Java toolchain paths). Cross-platform policy data; small additive port to `policy.json`. Note: paths are Unix-flavored upstream — Windows JDK paths would need fork-side platform-conditional entries (fork has `platform.rs` for this). |

### Theme: dependencies (bundled for completeness, not phase-worthy)

| Feature/Change | Upstream tag | In fork? | Notes |
|---|---|---|---|
| `landlock` 0.4.4→0.4.5, `sigstore` →0.8.0, `rcgen` →0.14.8, `shlex` →2.0.1, `serde_json`, `similar`, several docker/* actions | v0.58–v0.59 | n/a | Per fork's standing convention, dep bumps are absorbed selectively / on the fork's own cadence. `landlock` and `sigstore` bumps are the only security-relevant ones; the rest are routine. Not phase-worthy on their own. |

---

## Section 3 — Proposed phase buckets

Clusters of the `no` / `partial` gaps that are `windows-applicable` or
`needs-windows-equivalent-design`, sized for roadmap planning. (macOS-only and
CI-only items are excluded as non-Windows-actionable.)

1. **UPST7 audit + straight-cherry-pick wave** — *small/medium.* The standard
   audit→sync cadence for upstream `v0.58.0..v0.59.0` (the deferred 19-commit
   backlog). Covers the cross-platform-core straight ports: JSONC profile parsing,
   `target_binary` profile field, `opencode` pack relocation, timeout-constant
   generalization, denial/diagnostic polish (suppressed-denial annotations, canonical
   denial-path precompute, access-mode rfind, overflow-checks), `java_runtime`
   group/profile, proxy 502 hardening. Mirrors Phase 48's shape; most are direct
   cherry-picks modulo schema-collision checks.

2. **Network: fine-grained `allow_domain` path + method restrictions** — *medium.*
   The marquee net-new in the window. Add URL-with-path parsing and HTTP method+path
   matching to the proxy filter (`nono-proxy`), plus the TLS-intercept "endpoint rules
   before credential selection" ordering fix. Cross-platform-core but touches the
   fork-divergent TLS-interception surface (Phase 34 C11 fork-preserve) — needs diff
   inspection, so scope it as its own slice.

3. **Bitwarden `bw://` credential source** — *small.* New keystore backend alongside
   `keyring://`/`env://`/`file://`. Pure Rust, cross-platform-core, isolated surface.

4. **Session lifecycle hooks (Windows-equivalent design)** — *medium/large.* Upstream's
   `hook_runtime` is unix-only; the fork needs a Windows-safe hook execution design
   (broker-spawned, Low-IL, no shell/fork assumption). Pair the cross-platform schema
   (`session_hooks` profile field) with a `needs-windows-equivalent-design` ADR for the
   Windows execution path. Highest design risk in the window.

5. **Supervisor IPC robustness (named-socket / keep-alive / timeouts)** — *medium.*
   Port the supervisor-loop hardening intent (don't drop the loop on transient child
   IPC close; bounded read timeouts; blocking-mode accepted connections) to the fork's
   Named-Pipe AIPC path. Unix side is cross-platform-core; Windows side is a
   translate-not-cherry-pick. Could fold into bucket 1 if Windows-translation cost is low.

6. **(Optional / CI) Release attestation alignment** — *small, low priority.* Decide
   whether upstream's `attest-build-provenance` composes with the fork's existing
   sigstore/TUF + Authenticode + MSI signing story, or is superseded by it. CI-only;
   likely a documentation/decision task rather than a port. Note the fork's `release.yml`
   is independently flagged as broken in project memory — sequence accordingly.

---

## Section 4 — Confidence & gaps in THIS analysis

**Confidence: high on the high-water mark, medium-high on the v0.58/v0.59 feature set.**

- **High-water mark (v0.57.0):** High confidence — triangulated from three independent
  repo-local artifacts (UPST6 ledger frontmatter, Phase 48 verification report with
  per-commit trailer counts, and the release-grid RESULT.md coverage chain). The fork's
  own `CHANGELOG.md` carries upstream sections through `[0.57.0]`, consistent with this.

- **v0.58.0 contents:** Medium-high. Retrieved from the upstream `main`
  `CHANGELOG.md` (raw) and corroborated by the GitHub Releases page. Both sources
  agree on the headline features (session hooks, `bw://`, JSONC, `target_binary`,
  named-socket IPC, proxy 502). PR numbers were not present in the CHANGELOG bullets,
  so refs are anchored to the tag rather than `#NNN`.

- **v0.59.0 contents:** Medium. **`v0.59.0` is dated 2026-05-27 — i.e. TODAY.** It is a
  freshly-cut release and may still be settling (e.g. the `jsonc-parser` "dependency
  restored" bug-fix in v0.59 suggests v0.58 churn). The feature list (allow_domain
  path/method restrictions, configurable timeouts, opencode extraction, denial-path
  precompute) was retrieved from the same two sources and they agree, but a re-fetch
  closer to roadmap-planning time is advisable in case patch releases (v0.59.x) land.

**FLAGGED — needs manual review:**

- **`v0.59.x` patch releases (none observed yet):** because v0.59.0 cut today, any
  v0.59.1+ would not appear here. Re-check the Releases page before committing UPST7 scope.
- **PR numbers for v0.58/v0.59 features:** not surfaced in the CHANGELOG; only `#881`
  (a v0.55 PTY fix, already synced) appeared. If the team wants per-feature PR traceability
  for UPST7, run `git log --oneline v0.57.0..v0.59.0` against `upstream/main` (the UPST6
  ledger's locked drift-tool invocation pattern applies: `make check-upstream-drift
  ARGS="--from v0.57.0 --to v0.59.0 --format json"`).
- **`opencode` profile (v0.59) "extracted from built-ins":** marked partial/verify — the
  fork already ships an `opencode` profile but the upstream change is a relocation into
  the pack mechanism; the exact fork-side delta needs a diff against `policy.json` / pack
  layout, not assumed.
- **Residual v0.44–v0.57 deferrals (`b5f0a3ab` ExecConfig refactor, `bbdf7b85`
  escape-quote wiring, full `wiring.rs` idempotent JSON-merge abstraction):** these were
  NOT re-verified line-by-line in this pass; they are macOS-learn-diagnostics-oriented and
  out of the v0.58/v0.59 forward-scope. Listed in Section 1 for completeness only.

**No features were fabricated.** Where a feature's fork status could not be settled by a
quick grep (proxy 502 path, TLS-intercept ordering, opencode relocation, timeout
generalization), it is marked `partial / verify` or `no / verify` rather than asserted.
