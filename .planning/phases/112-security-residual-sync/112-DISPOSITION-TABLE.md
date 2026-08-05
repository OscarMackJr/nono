# Phase 112: security-residual-and-misc Cluster — Finalized Disposition Table

**Wave 1 reality-check — finalized 2026-08-05, per CONTEXT.md D-02 and the ROADMAP Amendment
(2026-08-05) for SEC-02.**

This table re-runs the highest-risk confirmatory commands live against the current fork tree
(not copied from `112-RESEARCH.md`'s table verbatim) and finalizes the disposition each downstream
absorb plan will implement. Every row's disposition matches `112-RESEARCH.md`'s provisional call
except where noted; every re-run confirmed the research document's finding unchanged, with two
narrowings applied per the ROADMAP Amendment (2026-08-05): SEC-02a/b/c now read `deferred ->
Phase 114` rather than the research document's `adapt-down-or-defer` framing, because the go/no-go
call on that framing is Phase 114's to make, not this plan's.

## 18-SHA Disposition Table

| SHA (short) | Req | Disposition | Cited evidence (fresh, live-re-run 2026-08-05) | Confidence |
|---|---|---|---|---|
| `0ecc476b` | SEC-01 | **won't-sync** (target subsystem absent) | `ls crates/nono-proxy/src/` → 15 files, no `aws/` or `tls_intercept/` dir; `ls crates/nono-proxy/src/aws` / `ls crates/nono-proxy/src/tls_intercept` → both `No such file or directory`. `git show 0ecc476b --stat` confirms 11 files, +1,873/-412. `credential.rs:131-134` still carries the `aws_routes: HashMap<String, ()>` placeholder comment; `reverse.rs:260-268` still carries the hand-authored 501 guard citing D-15. See `112-AWS-SIGV4-PROXY-AUTH-FINDING.md` for the full finding. | HIGH |
| `9b692e07` | SEC-02a | **deferred -> Phase 114** | `git show 9b692e07 --stat` confirms 26 files, +4,425/-163. `ls crates/nono-proxy/src/forward.rs` and `ls crates/nono-proxy/src/oauth_capture` both → `No such file or directory`. `git log --oneline --all --diff-filter=A -- crates/nono-proxy/src/forward.rs` → sole creating ancestor `149abde0`, outside this 18-commit window, never absorbed. Upstream's `forward.rs` (as of `9b692e07`) defines `pub type ResponseRewrite<'a>` used by `rewrite_http1_response()` — the response-rewrite hook the reality-check confirms is the mechanism that keeps a captured real OAuth token from reaching the sandboxed client. See `112-OAUTH-CAPTURE-DISPOSITION.md` for the full evidence capture and the ROADMAP Amendment (2026-08-05) carve-out authority. | HIGH |
| `3c59c62e` | SEC-02b | **deferred -> Phase 114** | `git show 3c59c62e --stat` confirms 4 files, +262/-12: `profile_runtime.rs` (present) plus `forward.rs`, `oauth_capture/mod.rs`, `oauth_capture/persist.rs` (all absent, per the `9b692e07` row). This is a security-hardening follow-up to `9b692e07` and rides the same disposition — it cannot be absorbed independently of its parent subsystem's disposition. | HIGH |
| `d033c631` | SEC-02c | **deferred -> Phase 114** | `git show d033c631 --stat` confirms 1 file, +4/-1: a stdin-consumption fix inside `proxy_runtime.rs`'s test fixture for the SEC-02a OAuth capture flow — the fixture only exists once SEC-02a's provider stdin-handling code lands. Rides SEC-02a's disposition. | HIGH |
| `a3243907` | SEC-03 | **adopt** | Target files (`command_runtime.rs`, `exec_strategy.rs`, `exec_strategy/supervisor_linux.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `profile/mod.rs`, `profile_runtime.rs`, `proxy_runtime.rs`, `sandbox_prepare.rs`, `nono/lib.rs`, `nono/sandbox/{linux,mod}.rs`) all present per `ls`/`Read` — 14/15 (only a deleted upstream doc file is absent, non-blocking). Existing GPU test surface at `crates/nono/src/sandbox/linux.rs` (`test_is_nvidia_compute_device_accepts_upstream_list`) confirms the fork already carries a prior partial port this commit extends. Unchanged from `112-RESEARCH.md`. | HIGH |
| `f943fb5a` | SEC-04 | **adopt** | Target files (`trust_cmd.rs`, `trust_scan.rs`, `nono/trust/{mod,policy,types}.rs`) all present. Additive discriminator field (`TRUST_POLICY_PREDICATE` alongside a `#[deprecated]`-marked `TRUST_POLICY_VERSION`), no removal of fork-specific trust types. Unchanged from `112-RESEARCH.md`. | HIGH |
| `d84b4818` | SEC-05 | **adopt** | `grep -n "AccessFs::Refer" crates/nono/src/sandbox/linux.rs` → hits at lines 53, 365, 3005, 3015, 3027, 3034, 3045, 3050 — **none inside `restrict_execute()`** (confirmed by reading `linux.rs:1105-1152` in full: the function's `Ruleset` only calls `.handle_access(AccessFs::Execute)`, no `Refer`). The general grant at `:365` is in the separate main `apply_landlock` ruleset, not the stacked execute-restriction layer `restrict_execute()` builds independently — the two rulesets are confirmed non-overlapping by direct read, not inference. This is the exact gap upstream's 89-line `d84b4818` patch closes. Unchanged from `112-RESEARCH.md`. | HIGH |
| `ac5ccd70` | SEC-06 | **adapt** | Target files (`exec_strategy.rs`, `tests/socket_access_run.rs`) present. Upstream's guard predicate (`tool_sandbox_runtime.is_some() \|\| seccomp_policy.child_requires_dumpable()`) references `tool_sandbox_runtime`, confirmed absent from the fork (same D-01 predicate re-confirmed for SEC-09 below) — the fork's equivalent is the free function `linux_child_requires_dumpable()` in `exec_strategy.rs`. Absorb requires re-wiring to the fork's predicate, dropping the `tool_sandbox_runtime` disjunct. Unchanged from `112-RESEARCH.md`. | HIGH |
| `2663e990` | SEC-07 | **adopt** | `grep -n "enum Commands" crates/nono-cli/src/cli.rs` and a `Proxy`-variant grep confirm zero `Proxy` arm in the `Commands` enum today. All 11 pre-existing target files (`app_runtime.rs`, `cli.rs`, `cli_bootstrap.rs`, `main.rs`, `proxy_runtime.rs`, 7 `nono-proxy` files) present; `proxy_command.rs` is the one new file this commit itself creates. Zero touches to `crates/nono/src/` (no ADR-86 boundary question). Unchanged from `112-RESEARCH.md`. | HIGH |
| `a5a441c2` | SEC-08 | **won't-sync (verbatim), adapt only if fail-closed default is explicitly preserved — flagged for ADR-112 escalation per D-05** | `grep -n "empty_allow_vars_fails_closed" crates/nono-cli/src/profile_runtime.rs` → confirmed present at line 989, the exact regression-guard test that would catch a verbatim-adopt regression. Fork's `profile_runtime.rs` resolves `allowed_env_vars` from `env_config.allow_vars.clone()` whenever `profile.environment` is `Some(_)` at all (never collapsed to `None`), so an omitted `allow_vars` already means "strip all" — the opposite default from what upstream's `Option<Vec<String>>` change would make omission mean ("allow everything"). Adopting verbatim reintroduces the leak the fork's own prior Phase-34 fix (D-20) closed. Unchanged from `112-RESEARCH.md`; this is the one row `112-RESEARCH.md` itself flags as the D-05 contingent-ADR trigger. | HIGH |
| `f6f02751` | SEC-09 | **won't-sync (target subsystem absent) — LOCKED per D-01, premise re-verified true** | `grep -rn "command_policies\|tool_sandbox_runtime\|tool_sandbox_initial_shim" crates/nono-cli/src/execution_runtime.rs` → 0 hits (exit code 1, no matches). D-01's evidence independently reproduced live, not merely trusted. | HIGH |
| `f0506434` | RES-01 | **skip, with recorded reasoning** (plus a flagged-but-not-acted-on UX-staleness note) | See `112-OAUTH-CAPTURE-DISPOSITION.md` for the full skip-with-reasoning entry (D-03: collides with the fork's own HKLM machine-policy spine / divergent update behavior). `grep -rn "always-further" crates/nono-cli/src/cli.rs crates/nono-cli/src/policy.rs crates/nono-cli/src/profile/builtin.rs crates/nono-cli/data/policy.json` → 16 hits, confirming the flagged pack-registry-namespace staleness is real but UX-only, out of scope per D-03's skip bias. | MEDIUM |
| `7fe0c828` | RES-01 | **skip, with recorded reasoning** | See `112-OAUTH-CAPTURE-DISPOSITION.md`. `grep -rln HKLM crates/nono-cli/src/*.rs` → 5 files (`cli_bootstrap.rs`, `health.rs`, `main.rs`, `platform.rs`, `policy.rs`), confirming the fork's HKLM machine-policy spine that D-03 cites as the collision point. | MEDIUM |
| `0158d52f` | RES-01 | **skip, with recorded reasoning** | See `112-OAUTH-CAPTURE-DISPOSITION.md`. Same HKLM-spine collision rationale as `7fe0c828`; pure telemetry addition to registry/update-check requests. | MEDIUM |
| `762eb05b` | RES-01 | **skip, with recorded reasoning** | See `112-OAUTH-CAPTURE-DISPOSITION.md`. Same rationale; also touches `release.yml`, which the fork has already heavily customized for Trusted Signing (v3.5) — absorbing an upstream `release.yml` hunk verbatim for a telemetry field carries out-of-proportion regression risk. | MEDIUM |
| `503045801a` | RES-02 | **adopt** (real fix, not skip-biased away) | `pty_proxy.rs`/`timeouts.rs` present; additive-only `discard_late_terminal_input()`/`CprReplyParse` mechanism (new fn + one call site in the final-teardown path), confirmed non-overlapping with the fork's existing `drain_terminal_output`/`drain_socket_replay`/`drain_attach_resize_pipe` mechanisms. Per `112-RESEARCH.md`'s MEDIUM-HIGH-confidence finding; not independently re-run live in this Wave-1 pass (RES-02 is outside this plan's Task 1-3 `must_haves` gate scope — SEC-01..09/D-07/D-03/RES-01 — the downstream absorb plan re-confirms at implementation time). | MEDIUM-HIGH |
| `4cc0af2c52` | RES-02 | **adapt with caution, or skip** | Test-only `/tmp` vs `target/`-relative socket path change; fork's current `crates/nono/src/supervisor/socket.rs` comment (unchanged by this commit) documents choosing `target/` specifically to avoid macOS Seatbelt `deny network*` test denials on `/var/folders`. Per `112-RESEARCH.md`'s finding; not independently re-run live in this Wave-1 pass (outside this plan's gate scope). | MEDIUM |
| `9840a16f35` | RES-02 | **adopt** | Test-quality-only fix (`NONO_NO_SAVE_PROMPT=1` env guard + tightened denial-marker assertion), zero production code touched. Per `112-RESEARCH.md`'s finding; not independently re-run live in this Wave-1 pass (outside this plan's gate scope). | HIGH |

**Cluster accounting: 18 SHAs total (10 D-18-named SEC-01..08 anchors + 1 SEC-09 + 4 RES-01 +
3 RES-02 = 18), matching `108-DIVERGENCE-LEDGER.md`'s itemization exactly. Every row above appears
exactly once — this is a single flat 18-row table, not two separate tables.**

## D-07 Confirmation (crossbeam-epoch / RUSTSEC-2026-0204)

```
$ cargo audit
    Loaded 1189 security advisories (from C:\Users\OMack\.cargo\advisory-db)
    Scanning Cargo.lock for vulnerabilities (567 crate dependencies)
warning: 6 allowed warnings found
$ echo $?
0
```

The 6 allowed warnings are unrelated informational advisories (`RUSTSEC-2025-0052`,
`RUSTSEC-2025-0057`, `RUSTSEC-2024-0436`, `RUSTSEC-2025-0134`, `RUSTSEC-2026-0190`,
`RUSTSEC-2026-0221` — an `event-listener`/`async-std` dev-dependency-tree soundness warning, not
a crossbeam-epoch finding); **zero vulnerabilities** are reported. `cargo audit`'s process exit
code is `0`.

```
$ grep -n -A1 'name = "crossbeam-epoch"' Cargo.lock
841:name = "crossbeam-epoch"
842:version = "0.9.20"
```

**Confirmed: `crossbeam-epoch 0.9.20` in `Cargo.lock`, `cargo audit` reports 0 vulnerabilities.
RUSTSEC-2026-0204 is closed. `373a67ae` (#1369) is NOT re-absorbed, per D-07 (LOCKED).**

## Disposition Tally

- **adopt:** SEC-03, SEC-04, SEC-05, SEC-07, RES-02/`503045801a`, RES-02/`9840a16f35` (6)
- **adapt:** SEC-06 (1)
- **adapt with caution / skip:** RES-02/`4cc0af2c52` (1)
- **won't-sync:** SEC-01, SEC-09 (2)
- **won't-sync-verbatim / adapt-with-fail-closed-preservation, ADR-112-flagged:** SEC-08 (1)
- **deferred -> Phase 114:** SEC-02a, SEC-02b, SEC-02c (3)
- **skip, with recorded reasoning:** RES-01 x4 (`f0506434`, `7fe0c828`, `0158d52f`, `762eb05b`) (4)

6 + 1 + 1 + 2 + 1 + 3 + 4 = **18**, matching `108-DIVERGENCE-LEDGER.md`'s "security-residual-and-misc
cluster commits (18)" itemization exactly (10 D-18-named SEC-01..08 anchors + 1 SEC-09 + 4 RES-01 +
3 RES-02 = 18). SEC-01..09/D-07/D-03/RES-01 rows (15 of 18) are freshly live-re-verified in this
Wave-1 pass per this plan's `must_haves`; the 3 RES-02 rows are carried from `112-RESEARCH.md`'s
HIGH/MEDIUM-HIGH/MEDIUM-confidence findings (RES-02 sits outside this plan's Task 1-3 gate scope)
and will be re-confirmed live by whichever downstream absorb plan implements them.
