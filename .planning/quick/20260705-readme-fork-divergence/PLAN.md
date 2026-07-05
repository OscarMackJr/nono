---
type: quick
slug: readme-fork-divergence
date: 2026-07-05
---

# Quick Task: Rewrite README to reflect fork divergence

**Request:** This fork will probably never be merged back to the original. It is thousands of
commits diverged, its Windows-native features diverge significantly, its install is different, as is
its ZT-Infra integration. Modify the README to reflect the current state.

## Scope

Replace the inherited upstream README (which pointed at `always-further/nono`, `brew install nono`,
`docs.nono.sh`, and treated Windows as a limited preview) with a fork-accurate one.

## Ground truth gathered before editing

- Remote: `origin = OscarMackJr/nono`, `upstream = nolabs-ai/nono`.
- Crates renamed to fork-owned family: `nono-sandbox`, `nono-sandbox-proxy`, `nono-sandbox-cli`;
  bins `nono` / `nono-agentd` / `nono-wfp-service` / `nono-shell-broker`; FFI `nono-ffi`.
- Bindings: PyPI `nono-sandbox` (nono-py), npm `@oscarmackjr/nono-ts` (nono-ts).
- Version `0.66.1` (leapfrogs upstream 0.66.0).
- Windows backend: AppContainer + WFP, Job Objects, supervisor, Low-IL broker, `nono-agentd`,
  `nono-wfp-service`, sandbox-the-tools hook pattern. Live `nono shell`/`wrap` TUI still OS-blocked.
- ZT-Infra: two-key AND signed policy overrides (offline ECDSA + live `POST /actions`); core stays
  policy-free, emits attestation telemetry only.
- CI workflow `ci.yml` exists in fork (badge resolves). Logo asset present.

## Accuracy constraints

- First live registry publish + publicly-trusted-signed release NOT done yet (in-progress v3.5) →
  README must say "build from source", NOT `cargo install`/`pip install`.
- Keep upstream attribution (Sigstore origin) honest; frame as an independent hard fork.
