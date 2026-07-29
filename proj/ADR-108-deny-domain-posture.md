# ADR-108: `deny_domain` (#1374) Disposition — Adopt-Verbatim vs Adapt Deny-Layer-Only

**Status:** Accepted
**Phase:** 108 — UPST12 Divergence Audit
**Date:** 2026-07-29
**Authors:** Phase 108 execution

---

## Context

Upstream `nolabs-ai/nono` commit `3b207eeb` ("feat(proxy): add deny_domain to block domains
through the proxy (#1374)", authored by Aleks, 2026-07-07) introduces a `--deny-domain` CLI flag
and a `deny_domain` profile key to complement the existing `allow_domain`/`--allow-domain`
surface. This ADR settles whether the fork adopts `3b207eeb` verbatim (full-sync-adopt) or
absorbs it as an additive deny mechanism with a fork-specific fail-closed guard (adapt).

The decision gates Phase 109 (NET-01 absorb). It is settled here, before any cherry-pick, per
CONTEXT.md D-11 — the audit already has the diff in hand and must recommend, not merely frame.
See the DIVERGENCE-LEDGER
(`.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`), NET cluster, for the
full per-commit audit of this window's network commits (including SPIFFE #1272 and the
SigV4/sibling-route fixes #1430/#1437 — explicitly out of scope for this ADR per D-13); this ADR
is the standalone analysis, independently citable after ledger archival, that resolves the
`deny_domain` posture alone.

### Upstream `3b207eeb` shape (from `git show 3b207eeb`)

Upstream's own commit message states:

> Adds --deny-domain CLI flag and deny_domain profile key to complement
> allow_domain. Users can now say "allow everything except these domains"
> without enumerating an allowlist. Deny rules are evaluated before the
> allowlist, support wildcard subdomains (*.example.com), and **activate the
> proxy on their own**. Conflicts with --allow-net at parse time. Port-suffix
> warnings are emitted for deny entries consistent with allow-domain behavior.

The phrase **"activate the proxy on their own"** is the specific upstream behavior D-12 rejects:
under upstream's design, configuring `deny_domain` alone — with no `allow_domain` and no other
proxy-activating flag — is sufficient to turn the proxy on.

Upstream's diff to `crates/nono/src/net_filter.rs` adds:

- `deny_suffixes: Vec<String>` field on `HostFilter`, parallel to the existing `deny_hosts`
  field.
- `pub fn with_denied_hosts(mut self, denied: &[String]) -> Self` — a builder that splits
  caller-supplied entries on a leading `*` into `deny_suffixes` (wildcard) vs exact `deny_hosts`.
  The domain list is **caller-supplied**, not hardcoded — this is the fact that keeps the
  mechanism ADR-86-compliant (see Decision, below).
- A "1b. Check deny suffixes (wildcard match)" step in `check_host()`, evaluated immediately
  after the existing exact-match `deny_hosts` check, **before** the allowlist check.

### Current fork state of `crates/nono/src/net_filter.rs` (verified 2026-07-29, before any
absorb)

The fork's `HostFilter` already carries a deny mechanism — `deny_domain` is an extension of a
live surface, not a new subsystem:

- `deny_hosts: Vec<String>` field, populated at construction from the hardcoded, non-overridable
  `DENY_HOSTS` cloud-metadata list (`169.254.169.254`, `metadata.google.internal`,
  `metadata.azure.internal`).
- A `strict: bool` field, documented inline: `"When true, an empty allowlist denies instead of
  allowing."`
- `HostFilter::new()` and `HostFilter::allow_all()` both construct with `strict: false`.
- `HostFilter::new_strict(allowed_hosts: &[String])` exists (used today only by the
  `sc4_dns_component_matrix` test and `test_strict_filter_*` unit tests) and sets `strict: true`,
  but nothing in the current fork wires deny-only profile configuration to select it.
- `check_host()`'s step 3 — quoting the current inline comment verbatim — is: `// 3. Empty
  allowlist: deny when strict, allow otherwise.` Concretely: `if self.allowed_hosts.is_empty() &&
  self.allowed_suffixes.is_empty() { if self.strict { DenyNotAllowed } else { Allow } }`.
- A `FilterResult::Deny*` family already exists: `DenyHost`, `DenyLinkLocal`, `DenyNotAllowed`.

The consequence of these two facts together is the crux of this ADR: **a profile that configures
only `deny_domain` entries (no `allow_domain`) produces an empty `allowed_hosts`/
`allowed_suffixes` pair.** With `strict: false` (the default used by every constructor a
deny-only profile would reach today), step 3 returns `FilterResult::Allow` for any host not on
the deny list — i.e., **default-allow**. NET-01 requires composing network filters "without
weakening default-deny"; verbatim adoption of upstream's mechanism, wired through the fork's
existing `strict: false` constructors, would silently violate that requirement for exactly the
deny-only profile shape upstream's own commit message advertises as the headline use case
("allow everything except these domains").

---

## Fork Touchpoint Map

| File | Touchpoint | Role |
|------|-----------|------|
| `crates/nono/src/net_filter.rs` | `HostFilter` struct, `deny_hosts: Vec<String>`, hardcoded `DENY_HOSTS` (cloud-metadata, non-overridable), `strict: bool` field, `FilterResult::Deny{Host,LinkLocal,NotAllowed}` family, `check_host()` step-3 comment `// 3. Empty allowlist: deny when strict, allow otherwise.` | The library-side mechanism `deny_domain` extends. `HostFilter::new()`/`allow_all()` set `strict: false`; `HostFilter::new_strict()` exists but is unused by any deny-only construction path today. |
| `crates/nono-cli/src/network_policy.rs` | `pub struct NetworkPolicy`, `NetworkPolicyMeta`, `NetworkGroup`, `NetworkProfileDef`, `CredentialDef`, `ResolvedNetworkPolicy`; `pub fn load_network_policy`, `resolve_network_profile`, `resolve_groups`, `resolve_credentials`, `build_proxy_config(resolved: &ResolvedNetworkPolicy, extra_hosts: &[String]) -> ProxyConfig`, `expand_proxy_allow`, `partition_allow_domain`, `collect_allow_domain_port_warnings` | The CLI-side policy resolver that constructs `ProxyConfig`/`HostFilter` inputs from profile + CLI flags. `build_proxy_config` currently has no `deny_domain`/`deny_suffixes` parameter and no strict-selection logic — this is where the D-12 fail-closed rule must be wired in Phase 109. Zero `deny_domain` references today (confirmed: `3b207eeb` is not yet absorbed). |
| `crates/nono-cli/src/cli.rs` | `--allow-domain` (`long = "allow-domain"`, alias `allow-proxy`/`proxy-allow`, env `NONO_ALLOW_DOMAIN`, repeatable `Vec<String>` field `allow_proxy`), `--allow-net` (deprecated compat flag, `conflicts_with_all` includes `network_profile`, `allow_proxy`, `proxy_credential`, etc.), `--block-net` (`conflicts_with = "allow_net"`) | The flag-parsing surface a `--deny-domain` flag would extend by direct parallel to `--allow-domain`: same `help_heading = "NETWORK"`, same repeatable `Vec<String>` shape, and (per upstream's commit message) a `conflicts_with = "allow_net"` relationship analogous to `--block-net`'s existing conflict. |

---

## Options Considered

### Option A: Full-Sync-Adopt

**Description:** Cherry-pick `3b207eeb` verbatim: add `deny_suffixes` + `with_denied_hosts()` to
`HostFilter` in `crates/nono/src/net_filter.rs`, add the mirrored `--deny-domain` CLI flag and
`deny_domain` profile key, and preserve upstream's "activate the proxy on their own" trigger —
i.e., configuring `deny_domain` with no other proxy-activating flag turns the proxy on, and the
resulting `HostFilter` is built via the same `strict: false` constructors deny-only profiles
would reach today (upstream carries no separate strict-selection logic for deny-only
configuration; its `HostFilter` equivalent has no `new_strict`-style fail-closed path threaded
through `deny_domain`).

**Pros:**
- Zero adaptation work: upstream's diff to `net_filter.rs` applies with minimal conflict, since
  the fork's `deny_hosts`/`DENY_HOSTS`/`FilterResult::Deny` shape already matches upstream's
  starting point.
- Matches upstream's UX exactly — `--deny-domain "allow everything except these domains"` behaves
  identically to upstream, minimizing user-facing divergence and per-sync reconciliation friction
  on future `net_filter.rs` changes.
- The `deny_suffixes` field and `with_denied_hosts()` builder are themselves policy-free
  mechanism (caller-supplied domain list) — adopting them verbatim does not, on its own, breach
  ADR-86.

**Cons:**
- **Default-allow gap, verbatim.** A profile carrying only `deny_domain` entries produces an
  empty `allowed_hosts`/`allowed_suffixes` pair; `check_host()` step 3
  (`// 3. Empty allowlist: deny when strict, allow otherwise.`) evaluates `strict`, and every
  constructor a deny-only profile would reach under verbatim adoption sets `strict: false`. The
  result is `FilterResult::Allow` for any host not explicitly denied — the exact "allow
  everything except these domains" behavior upstream advertises, but one that silently violates
  NET-01's "compose without weakening default-deny" requirement for every host the operator did
  not think to name.
- **Auto-activation widens the fail-secure boundary.** "Activate the proxy on their own" means a
  single `deny_domain` entry — with no `allow_domain`, no `--block-net`, no other explicit
  network-restricting flag — is sufficient to change the sandbox's network posture. This
  contradicts the fork's existing proxy-activation model, where activation is gated by an
  explicit predicate (the pattern proven by
  `proxy_activates_with_custom_credentials_only`, Phase 89), not by the mere presence of any one
  network-related flag.
- Once shipped, this is a security regression that is hard to walk back without a breaking CLI
  change: users who adopt `deny_domain` expecting "allow everything except X" would need to be
  retrained if the fork later tightens the default.

---

### Option B: Adapt — Deny Layer Only

**Description:** Absorb `deny_suffixes` + `with_denied_hosts()` into `crates/nono/src/
net_filter.rs` as additive mechanism, unchanged from upstream's diff shape. In
`crates/nono-cli/src/network_policy.rs`, the resolver that builds the `HostFilter`/`ProxyConfig`
(today, `build_proxy_config`) is extended so that **whenever a profile configures
`deny_domain`/`deny_suffixes` without an explicit `allow_domain`, it selects a fail-closed
filter construction** — e.g. `HostFilter::new_strict()` or an equivalent "deny configured with no
allow implies strict" rule — instead of the default `strict: false` path. Upstream's "activates
the proxy on its own" trigger is rejected: proxy activation for a deny-only profile is gated
through the fork's existing activation predicate (the same style guard as
`proxy_activates_with_custom_credentials_only`), not through `deny_domain`'s mere presence.

**Pros:**
- Closes the default-allow gap at the point of construction: a deny-only profile fails closed
  (`strict: true` → empty allowlist denies) rather than silently allowing every non-denied host.
  This satisfies NET-01's "compose without weakening default-deny" requirement directly.
- Preserves ADR-86: the fail-closed rule lives in `crates/nono-cli/src/network_policy.rs`
  (CLI-side policy), not in `crates/nono/src/net_filter.rs` (library mechanism). The library
  still applies only what the caller constructs — `with_denied_hosts()` remains caller-supplied
  mechanism; the *decision* to select `new_strict()` for a deny-only profile is CLI policy, drawn
  from the same boundary line ADR-86 already established for every other security-relevant
  default in this codebase.
- Keeps proxy activation consistent with the fork's established model: one predicate function
  gates activation, evaluated the same way regardless of which specific flag (allow, deny, block,
  credential) is present — no new "any one flag turns on the proxy" special case.
- Still gets the upstream user-facing capability ("allow everything except these domains") — the
  syntax and wildcard-suffix matching are unchanged from upstream — just with a safe default
  instead of an unsafe one.

**Cons:**
- Requires genuinely new CLI-side logic (the deny-without-allow → strict selection rule) that
  upstream does not carry; Phase 109 cannot cherry-pick this piece, it must author it.
- Diverges from upstream's exact runtime semantics for the deny-only case: a fork user who reads
  upstream's `deny_domain` documentation and expects auto-activation-with-default-allow will see
  different (safer, but different) behavior on this fork. This divergence must be documented in
  the fork's own docs/profile-authoring guidance so it does not read as a bug report against
  upstream.
- Every future upstream sync touching `net_filter.rs`'s deny-handling must re-verify the
  fail-closed rule was not silently reintroduced to `strict: false` by an unrelated upstream
  refactor — an ongoing (small) reconciliation cost, tracked the same way ADR-86 tracks its
  Windows carve-out.

---

## Decision

**Adapt** (Option B).

Two facts, established by direct inspection of the live diff and the live fork code, settle the
question:

1. **ADR-86 is not breached by either option.** `with_denied_hosts(&[String])` takes a
   caller-supplied domain list; nothing about `deny_suffixes` or the deny-check step in
   `check_host()` embeds fork- or CLI-specific policy into the library. Absorbing the mechanism
   into `crates/nono/src/net_filter.rs` — under either Option A or B — keeps the library
   policy-free by construction, exactly as ADR-98's Category 1 analysis found for
   `NetworkMode::ProxyOnly`. This means the choice between options does not turn on the library
   boundary; it turns entirely on where the fail-closed decision is made and whether it is made
   at all.
2. **The default-allow gap is real and lives in the CLI resolver, not the library mechanism.**
   `HostFilter.strict` already exists, is already documented as the correct lever
   (`"when true, an empty allowlist denies instead of allowing"`), and `HostFilter::new_strict()`
   is already implemented and tested. The library does not need new code to support a safe
   deny-only posture — it needs the CLI-side `network_policy.rs` resolver to *select* the
   existing strict constructor when the profile shape (`deny_domain` present, `allow_domain`
   absent) calls for it. Verbatim adoption (Option A) leaves that selection unmade, and every
   deny-only profile silently lands on `strict: false` — this is the specific behavior D-12
   requires the fork to reject.

Because the fail-closed rule is CLI-side policy work rather than a library change, and because
NET-01 explicitly requires composing filters "without weakening default-deny," Option B is the
only option that satisfies the requirement the absorb phase is chartered to meet. Option A's
lower implementation cost does not offset shipping a documented default-allow gap on a
security-relevant surface.

---

## Consequences

### Phase 109 absorb implications (NET-01)

- (a) Absorb `deny_suffixes: Vec<String>` and `with_denied_hosts(&[String])` into
  `crates/nono/src/net_filter.rs`, additive and backward-compatible with the existing
  `deny_hosts`/`DENY_HOSTS` fields and the existing `FilterResult::Deny*` family — no removal or
  rename of current mechanism, matching upstream's diff shape for the library-side change.
- (b) The `crates/nono-cli/src/network_policy.rs` resolver **MUST** select a strict/fail-closed
  filter construction (`HostFilter::new_strict()` or an equivalent explicit rule) whenever a
  resolved profile carries `deny_domain`/`deny_suffixes` entries without an explicit
  `allow_domain`. This is a **named Phase 109 acceptance criterion** the absorb plan must
  implement and test — it is not implemented by this ADR or by Phase 108 (per D-08's audit-only
  boundary: this phase flags and names paths, it does not write the code).
- (c) Reject upstream's "activates the proxy on its own" trigger. Phase 109 must gate proxy
  activation for a `deny_domain`-only profile through the fork's existing activation predicate —
  the same style guard as `proxy_activates_with_custom_credentials_only` (Phase 89) — rather than
  through `deny_domain`'s mere presence as a new, separate activation path.
- (d) See `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`, NET cluster,
  for the full commit-level audit of `3b207eeb` plus its sibling NET commits (SPIFFE #1272,
  SigV4 #1430, sibling-route #1437) — this ADR does not duplicate that per-commit table.

---

## References

- Upstream commit: `3b207eeb` — `feat(proxy): add deny_domain to block domains through the proxy (#1374)`
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free library boundary this ADR
  confirms is preserved by both options
- `proj/ADR-98-network-intent-disposition.md` — the standalone-ADR precedent this document
  mirrors in structure and naming
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — NET cluster,
  full commit-level audit (not duplicated here)
