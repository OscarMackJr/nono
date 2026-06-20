# Phase 89: Proxy Hardening Sync - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-20
**Phase:** 89-proxy-hardening-sync
**Areas discussed:** CONNECT-path fixes, Intent refactor, customCredentials + AWS stub, 403+audit & allow_domain

---

## CONNECT-path fixes (#1048/#1091, #1151)

### Reconcile bar — port vs no-op

| Option | Description | Selected |
|--------|-------------|----------|
| Test-driven gap proof | Default no-op; require per-commit equivalence note + a fork test proving the intent is already met; port only if a test exposes a real gap | ✓ |
| Port intent always | Re-implement each fix's intent regardless of apparent equivalence | |
| Document-equivalence only | Reviewer judgment + ledger note, no new test | |

**User's choice:** Test-driven gap proof
**Notes:** Fork already honors `external_proxy` in CONNECT (server.rs:534-559) and runs lenient undici auth in connect.rs — upstream fixes target tls_intercept behavior the fork achieves differently. Prove the gap before adding duplicate fork code.

### #1151 reactive-auth keep-open

| Option | Description | Selected |
|--------|-------------|----------|
| Port only if test fails | Write keep-open behavioral test; skip if fork already keeps open or lenient auth makes 407-retry moot; implement only if test proves a drop | ✓ |
| Always implement keep-open | Hard-implement 407-then-keep-open handshake regardless | |
| Out of scope — lenient supersedes | Declare lenient auth a deliberate supersession, skip with no test | |

**User's choice:** Port only if test fails
**Notes:** Consistent with the test-driven bar above. Fork's lenient auth may make the reactive 407 handshake moot.

---

## Intent refactor (bd4b6b7f / #1199)

| Option | Description | Selected |
|--------|-------------|----------|
| Defer refactor, keep behavior | Skip the structural refactor (won't-sync, architecture divergence); implement #1197's fix directly against the fork's build_proxy_config_from_flags path | ✓ |
| Adopt minus TlsInterceptIntent | Port intent/activation separation, omit TlsInterceptIntent | |
| Adopt fully with stub intent | Port everything including a no-op TlsInterceptIntent placeholder | |

**User's choice:** Defer refactor, keep behavior
**Notes:** Refactor is organizational, conflicts with the fork's EffectiveProxySettings model, and carries an unbackable TlsInterceptIntent. No behavioral payoff this milestone; a stub would violate the no-dead-code standard.

---

## customCredentials + AWS stub (#1197 / 724bb207)

| Option | Description | Selected |
|--------|-------------|----------|
| Activation-only, keep 501 | Land the custom_credentials activation-gate fix; leave AWS SigV4 as the honest 501 stub | ✓ |
| Promote AWS SigV4 to real | Implement real SigV4 signing this phase | |
| 501 → clear error + audit | Keep unimplemented but upgrade to a diagnostic-quality denial | |

**User's choice:** Activation-only, keep 501
**Notes:** #1197 is activation, not signing. The bug is precise — activation gate (proxy_runtime.rs:90-116) checks credentials/network_profile/allow_domain/upstream_proxy but not custom_credentials. SigV4 is a separate feature beyond Cluster F.

---

## 403+audit & allow_domain (#1077, #1132)

| Option | Description | Selected |
|--------|-------------|----------|
| Verify shadowing on fork model | Test the allow_domain-shadows-credential-catch-all scenario against the fork's prefix-keyed RouteStore; skip the RouteSelection refactor if no shadow; fix directly if it reproduces | ✓ |
| Port RouteSelection abstraction | Adopt upstream's RouteSelection enum + select_route regardless | |
| Assume present, document only | Judge both already satisfied, record equivalence, skip without tests | |

**User's choice:** Verify shadowing on fork model
**Notes:** #1077 appears already present (reverse.rs:96-114 endpoint-rule default-deny 403+audit). #1132's RouteSelection abstraction doesn't map to the fork's prefix-keyed model — fix the fork's selection directly if the shadow reproduces, don't import the upstream abstraction.

---

## Claude's Discretion

- Exact wording/structure of each behavioral-equivalence test.
- Whether the `--block-net` override branch needs the same `custom_credentials` predicate as the active branch (likely yes for consistency).
- ProxyDiagnostic surface usage for any newly-added denial (only if a fix adds a denial path).

## Deferred Ideas

- Real AWS SigV4 request signing — its own future phase.
- bd4b6b7f intent/activation refactor + TlsInterceptIntent — won't-sync; revisit only if a tls_intercept/ module is ever revived.
- Diagnostic-quality upgrade of the AWS 501 — considered, not taken; candidate polish if AWS work resumes.
- `76b7b695` forward_inner_request refactor — fully won't-sync (lives entirely in the absent tls_intercept/ module).
