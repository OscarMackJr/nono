# Phase 34 Outcomes

**Phase:** 34-upst3-upstream-v0-41-v0-52-sync-execution
**Date closed:** 2026-05-12
**Upstream range absorbed:** v0.41.0..v0.52.0
**Terminal plan:** 34-10 (this artifact is produced at 34-10 close per D-34-A3)

## Cluster disposition summary (from Phase 33 DIVERGENCE-LEDGER.md)

- **8 `will-sync` clusters:** C2, C4, C5, C7, C8, C9, C10, C12 — landed via Plans 34-00..34-08 (cherry-pick chains with D-19 trailers; per-plan PRs per D-34-D1).
- **2 `fork-preserve` clusters:**
  - C6 (Pack migration + claude-code/codex registry relocation, v0.44.0) — Plan 34-09 D-20 manual replay; preserves fork's v2.1 Phase 18.1-03 claude-code wiring.
  - C11 (Proxy TLS interception + audit-context, v0.51.0) — Plan 34-10 split execution: 1 clean replay (`9300de9` audit-context shape into Phase 23 REQ-AUD-05 ledger envelope) + 4 documentation-only commits (`149abde`, `879562c`, `8db8919`, `dcf2d29` — TLS-interception machinery; non-port preserves fork's Windows credential-injection rewrite).
- **2 `won't-sync` clusters:** C1, C3 — documented below per D-34-A3.

## Won't-sync clusters

Per D-34-A3 (Phase 34 CONTEXT.md): "Won't-sync clusters documented as one inline
ledger update (no dedicated plan). Clusters C1 (PTY attach/detach) and C3
(Unix-socket capability) get explicit `won't-sync` rows in Phase 34's
plan-close ledger update so future audits can see they were considered and
rejected with rationale. No code change, no separate plan."

This file (`34-PHASE-OUTCOMES.md`) is the chosen artifact shape — co-located
with the Phase 34 directory rather than mutating the audit-complete Phase 33
`DIVERGENCE-LEDGER.md`. The shape was chosen for two reasons: (a) Phase 33 is
an audit-complete artifact and should not be mutated post-close; (b)
co-locating the outcome summary with the Phase 34 directory aids future-audit
traceability.

### C1 — PTY attach/detach polish (v0.41.0)

**Disposition:** won't-sync

**Commits in scope (per Phase 33 DIVERGENCE-LEDGER.md):**

| sha | subject | upstream-tag |
|-----|---------|--------------|
| `2ac3409` | feat(pty): enhance detach notice and terminal cleanup | v0.41.0 |
| `95f2218` | fix(pty-proxy): ensure full scrollback on reattach for normal screen | v0.41.0 |
| `d0fa303` | feat(pty): preserve outer terminal scrollback on attach | v0.41.0 |
| `e3fdcb9` | fix(cli): improve attach/detach scrollback and alt-screen | v0.41.0 |
| `e8c848f` | Update crates/nono-cli/src/pty_proxy.rs | v0.41.0 |
| `fef06f3` | feat(pty-proxy): scroll viewport to native scrollback on detach | v0.41.0 |
| `be05217` | fix(signals): prevent signal swallowing | v0.41.0 |

**Rationale (verbatim from Phase 33 DIVERGENCE-LEDGER.md cluster 1 row):**

> Upstream changes touch `crates/nono-cli/src/pty_proxy.rs` (cross-platform
> PTY proxy used on Linux/macOS attach paths); the fork's Windows attach path
> lives in `pty_proxy_windows.rs` (D-11 excluded; ConPTY-based, structurally
> different from upstream's portable_pty primitives). The Unix-side
> scrollback/alt-screen behavior is consumed only by macOS attach in the fork
> (Linux is a POC); the fork's own Phase 17 live-stream attach work (v2.1)
> already satisfied the user-visible scrollback requirement on the supported
> Windows path. Cherry-picking would add Unix attach polish that does not
> flow into Windows ConPTY behavior. Per CONTEXT Specifics §5 ("upstream
> churn not relevant to fork").

**Decision rationale cites:**

- **D-11 (Phase 24 CONTEXT.md):** `*_windows.rs` + `exec_strategy_windows/`
  are drift-tool filtered. The fork's `pty_proxy_windows.rs` (ConPTY attach
  path) is structurally distinct from upstream's `pty_proxy.rs`
  (portable_pty primitives). Upstream's scrollback polish lives in
  cross-platform code paths that the fork's Windows attach does not traverse.
- **Phase 17 (v2.1) live-stream attach** already satisfied the user-visible
  scrollback requirement on the supported Windows path. No outstanding
  fork-side gap for upstream's polish to close.

**Future re-evaluation trigger:** if the fork ever unifies its Windows
attach path with a portable_pty-equivalent abstraction, re-audit this
disposition against the then-current upstream `pty_proxy.rs` shape.

### C3 — Unix-socket capability (v0.42.0)

**Disposition:** won't-sync

**Commits in scope (per Phase 33 DIVERGENCE-LEDGER.md):**

| sha | subject | upstream-tag |
|-----|---------|--------------|
| `85708ca` | feat(cli): add --allow-unix-socket flag family + profile schema | v0.42.0 |
| `a9a8b6c` | feat(capability): add UnixSocketCapability and UnixSocketMode | v0.42.0 |
| `1d789aa` | fix(supervisor(linux)): allow pathname af_unix sockets in network seccomp | v0.42.0 |
| `a87c6ae` | chore: release v0.42.0 | v0.42.0 |

**Rationale (verbatim from Phase 33 DIVERGENCE-LEDGER.md cluster 3 row):**

> Upstream adds `UnixSocketCapability` + `UnixSocketMode` +
> `--allow-unix-socket` flag family + Linux seccomp `af_unix` plumbing. The
> capability shape is Unix-specific (Windows IPC uses Named Pipes — see
> Phase 18 AIPC pipe/socket brokering); adding a `UnixSocketCapability` to
> `crates/nono/` would expose an enum variant that no Windows backend can
> honor and would violate D-19 (no library mutation in this audit; a
> sync-time addition would need its own Windows-no-op handling decision).
> Fork users on Windows do not consume Unix sockets; macOS users get
> unsigned Unix-socket access today via the broader macOS Seatbelt
> allowlist — a typed capability is not a regression. Per CONTEXT
> Specifics §5.

**Decision rationale cites:**

- **D-19 / D-34-E2 (atomic commit-per-semantic-change; no library
  mutation in this audit):** A typed `UnixSocketCapability` lands in
  `crates/nono/src/capability.rs` (the library). Adding the enum variant
  would either expose a no-op match arm on the Windows backend (violating
  fail-secure: "On any error, deny access. Never silently degrade to a
  less secure state.") or require a parallel Windows IPC capability
  decision that is out of Phase 34 scope.
- **Phase 18 AIPC pipe/socket brokering** already addresses the fork's
  Windows IPC needs via Named Pipes. A Unix-socket-typed capability is not
  the right abstraction for the fork's Windows surface; macOS users
  already get Unix-socket access via the broader Seatbelt allowlist, so
  no fork-side user-visible regression results from non-porting.

**Future re-evaluation trigger:** if a future phase decides to define a
cross-platform "stream socket" capability that abstracts over Unix sockets
(Linux/macOS) and Named Pipes (Windows), upstream's `UnixSocketCapability`
shape becomes a candidate to absorb as the Linux/macOS arm of that
abstraction. Until then, the fork's Phase 18 AIPC Named-Pipe path is the
canonical Windows IPC capability surface.

---

*Phase 34 closes with all 12 cluster dispositions resolved. Future UPST
phases (UPST4, v0.53.0+) fire per the Phase 33 ADR's "per upstream release,
lazily-evaluated" cadence rule.*
