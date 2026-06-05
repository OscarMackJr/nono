---
phase: 56
slug: fine-grained-network-filtering
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-04
---

# Phase 56 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from 56-RESEARCH.md "## Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner |
| **Config file** | `Makefile` targets (`make test`, `make test-cli`) |
| **Quick run command** | `cargo test -p nono-cli -p nono-proxy -- allow_domain partition endpoint` |
| **Full suite command** | `make test` (workspace) |
| **Estimated runtime** | ~quick <30s / full per `make test` |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli -p nono-proxy -- allow_domain partition endpoint`
- **After every plan wave:** Run `make test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds (quick), full suite at wave merge

---

## Per-Task Verification Map

> Filled per-plan during planning; rows below are the requirement→test seed from RESEARCH.md.

| Req / SC | Behavior | Test Type | Automated Command | File Exists |
|----------|----------|-----------|-------------------|-------------|
| REQ-NET-01 / SC1 | `--allow-domain https://api.example.com/v1` restricts to `/v1` prefix; disallowed path → HTTP 403, not silent pass-through | integration (proxy) | `cargo test -p nono-proxy -- endpoint` | ✅ (existing `config.rs` tests) |
| REQ-NET-01 / SC1 | Disallowed HTTP method on allowed path → hard denial | unit (reverse.rs) | `cargo test -p nono-proxy -- reverse` | ✅ (existing `reverse.rs` handler tests) |
| REQ-NET-01 / SC2 | Endpoint-rule deny rejected BEFORE credential injection; audit entry names host+path+method pre-credential | unit (reverse.rs audit path) | `cargo test -p nono-proxy -- audit` | ✅ (existing audit tests) |
| REQ-NET-01 / SC3 | `nono why --host api.example.com` lists path-prefix + method rules when host has scoped entries | unit (query_ext.rs) | `cargo test -p nono-cli -- query_network` | ⚠️ exists, needs extension |
| REQ-NET-01 / SC3 | URL-with-path input parsed for `nono why --host` | unit (query_ext.rs) | `cargo test -p nono-cli -- parse_host_input` | ❌ W0 (new fn needs test) |
| REQ-NET-01 / SC4 | `credential.rs` byte-identical to invariant `c9f25164` | verification | `git show HEAD:crates/nono-proxy/src/credential.rs \| sha256sum` | verification step |
| REQ-NET-01 / SC4 | Ported commits carry `Upstream-commit:` trailers | doc check | `git log --oneline -5 \| grep Upstream-commit` | verification step |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 gap tests (all available verbatim in the upstream commits — port them alongside the feature code):

- [ ] `crates/nono-cli/src/proxy_runtime.rs` — `parse_allow_domain_arg` unit tests (plain hostname, URL with path, root URL → plain, deep path). Source: `75b2265`.
- [ ] `crates/nono-cli/src/network_policy.rs` — `partition_allow_domain` unit tests (plain entries, with-endpoints, empty-endpoints-as-plain, rejects-empty-domain). Source: `0ced085`.
- [ ] `crates/nono-cli/src/profile/mod.rs` — `merge_allow_domain` + `AllowDomainEntry` deserialization tests (plain string, object with endpoints, backward-compat mixed array). Source: `0ced085`.
- [ ] `crates/nono-cli/src/query_ext.rs` — `parse_host_input` + `path_matches_endpoint_rules` tests. Source: `75b2265`.

*(Exact module paths are RESEARCH.md's proposed layout; planner reconciles against the fork's actual file names.)*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end: sandboxed child hits disallowed path and receives proxy denial | REQ-NET-01 / SC1 | Requires a live proxy + real child process; may exceed unit harness | Run `nono run --allow-domain https://api.example.com/v1 --method GET -- <client>`; attempt `/v2` and a `POST`; confirm denial + audit entry |

*Automated proxy integration tests cover the rule-evaluation core; the full live-child path is the manual backstop.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** satisfied (all tasks have automated verify; B2 fix applied 2026-06-04)
