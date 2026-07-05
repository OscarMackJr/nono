---
type: quick-summary
slug: readme-fork-divergence
date: 2026-07-05
status: complete
---

# Summary: Rewrite README to reflect fork divergence

## What changed

`README.md` fully rewritten from the inherited upstream copy to a fork-accurate one:

- **Prominent fork notice** — states this is an independent hard fork of `nolabs-ai/nono`, thousands
  of commits diverged, will most likely never merge back; where this README and upstream's disagree,
  this one is authoritative.
- **Badges/links** repointed from `always-further/nono` → `OscarMackJr/nono` (CI badge → fork
  `ci.yml`); dropped the `docs.nono.sh` badge (upstream-owned domain) and the upstream Discord/
  agent-sign badges.
- **"What makes this fork different"** table — Windows AppContainer + WFP, Job Objects, supervisor,
  Low-IL broker, `nono-agentd`/`nono-wfp-service`, sandbox-the-tools, fork-owned distribution,
  ZT-Infra signed overrides.
- **Honest Windows boundary** — native direct/setup/dry-run/blocked-network/supervised work; live
  `nono shell`/`wrap` TUI still OS-blocked (`0xC0000142`).
- **Install** rewritten to build-from-source (registry publish + signed release noted in-progress),
  removing the misleading `brew install nono`.
- **Library/bindings** updated to `nono-sandbox` (crates.io/PyPI) and `@oscarmackjr/nono-ts` (npm).
- Added **workspace layout** table (7 members incl. `nono-fltmgr-client`, `tools/sign-fixture`) and a
  **Relationship to upstream** section (leapfrog versioning, sync cycles, what upstream docs cover).

## Accuracy guardrails honored

- Did **not** advertise `cargo install`/`pip install` — packages reserved but not yet published
  (first live publish is the in-progress v3.5 milestone).
- Kept upstream Sigstore attribution and Apache-2.0 copyright retention.

## Verification

Facts cross-checked against live repo: `git remote -v`, crate `name =` fields, `Cargo.toml`
version, `crates/nono-cli/src/bin/`, `.github/workflows/`, sibling `nono-py`/`nono-ts` manifests.
Documentation-only change — no build/test impact.
