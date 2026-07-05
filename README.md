<div align="center">

<img src="assets/nono-logo.png" alt="nono logo" width="600"/>

<p>
  <a href="https://opensource.org/licenses/Apache-2.0"><img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg" alt="License"/></a>
  <a href="https://github.com/OscarMackJr/nono/actions/workflows/ci.yml"><img src="https://github.com/OscarMackJr/nono/actions/workflows/ci.yml/badge.svg" alt="CI Status"/></a>
  <img src="https://img.shields.io/badge/Platform-Windows%20native%20%7C%20Linux%20%7C%20macOS-informational.svg" alt="Platform support"/>
</p>

</div>

> [!IMPORTANT]
> **This is an independent, hard-diverged fork.**
> `OscarMackJr/nono` began as a downstream fork of the upstream nono project (originally by the
> creator of [Sigstore](https://sigstore.dev), now at
> [`nolabs-ai/nono`](https://github.com/nolabs-ai/nono)) and has since diverged by thousands of
> commits. It will most likely **never** be merged back upstream. The core sandbox primitive still
> tracks upstream through periodic sync cycles, but the **Windows-native backend, the install /
> distribution model, and the ZT-Infra signed-override integration are fork-only and diverge
> significantly** from anything upstream ships. Where this README and the upstream one disagree,
> **this one describes what is actually in this repository.**

> [!WARNING]
> Early alpha / not yet audited for production use. Active development may cause breakage. The first
> publicly-trusted-signed release and registry publish are still being finalized (see
> **Install** below) — until then, build from source.

---

nono wraps any AI agent or process in a kernel-isolated sandbox in seconds. No hypervisor, no
infrastructure required: a single binary, zero added latency, flexible enough for a solo developer's
workflow or a fleet of agents at scale. Unauthorized operations are made *structurally impossible* by
the OS kernel — Landlock (Linux), Seatbelt (macOS), and **AppContainer + Windows Filtering Platform
(WFP)** on Windows.

## What makes this fork different

This fork's headline investment is **bringing the Windows backend to functional parity with the Unix
platforms** — real, kernel-enforced isolation rather than a limited preview — plus a two-key signed
policy-override integration with an external Zero-Trust control plane.

| Area | This fork |
|------|-----------|
| **Windows isolation** | Per-run **AppContainer (lowbox)** for filesystem/process confinement + **WFP** kernel-level network enforcement (per-package-SID egress filtering). Job Objects, a supervisor-led process model, and a **Low-IL broker** (`nono-shell-broker`) for spawning restricted shell children. |
| **Windows services / daemons** | `nono-agentd` (persistent multi-tenant agent daemon) and `nono-wfp-service` (drives real kernel `FwpmFilterAdd0` WFP filters for confined-egress on the daemon path). |
| **Windows "sandbox-the-tools"** | Run an agent (e.g. Claude Code) at Medium integrity and confine **each individual tool call** through a `PreToolUse` hook that re-invokes `nono run` — the recommended Windows pattern. |
| **Distribution** | Fork-owned package identities (`nono-sandbox*`), a signed Windows **MSI**, and **Azure Trusted Signing** for a publicly-trusted release. |
| **ZT-Infra signed overrides** | Capability-policy overrides gated behind a **two-key AND**: an offline ECDSA P-256 signature *and* a live allow decision from an external [ZT-Infra](https://github.com/nolabs-ai/nono) control plane (`POST /actions`). The Rust core stays policy-free and only emits attestation telemetry. |

**Windows boundary (honest status):** direct execution, `setup`, `--dry-run`, blocked-network, and
supervised flows work natively. Live interactive `nono shell` / `nono wrap` TUIs remain OS-blocked
(`0xC0000142`) on Windows today — use `--dry-run` to inspect policy, or the "sandbox-the-tools"
hook pattern for real per-tool confinement.

---

## Install

> The fork reserves its own package names (`nono-sandbox`, `nono-sandbox-proxy`, `nono-sandbox-cli`
> on crates.io; `nono-sandbox` on PyPI; `@oscarmackjr/nono-ts` on npm) but the **first live publish +
> publicly-trusted-signed release is still in progress**. Prefer building from source for now.

**Build from source (all platforms):**
```bash
git clone https://github.com/OscarMackJr/nono.git
cd nono
cargo build --release --workspace   # produces target/release/nono(.exe)
```

**Windows (MSI):** built via `scripts/build-windows-msi.ps1`; release MSIs are Authenticode-signed
through Azure Trusted Signing. Run `nono setup --check-only` to see what the current host supports.

The installed binary/library/repository names are still `nono` — only the *published package
identities* were renamed to the fork-owned `nono-sandbox` family.

---

## Quick Start

```bash
# Any CLI agent -- just put your command after --
$ nono run --profile claude-code -- claude

# or detached, with atomic snapshots
$ nono run --detached --profile claude-code --rollback -- claude
Started detached session 7a6a652f7273fe60.
Attach with: nono attach 7a6a652f7273fe60

# Any given command
nono run -- python3 my_agent.py
nono run --read /data -- npx @modelcontextprotocol/server-filesystem /data
```

Built-in profiles for popular agents ship in the CLI; you can also define your own.

## Library

The core is a Rust library (`nono-sandbox` on crates.io, imported as `nono`) that embeds into any
application. Policy-free — it applies only what clients explicitly request.

```rust
use nono::{CapabilitySet, Sandbox};

let mut caps = CapabilitySet::new();
caps.allow_read("/data/models")?;
caps.allow_write("/tmp/workspace")?;

Sandbox::apply(&caps)?;  // Irreversible -- kernel-enforced from here on
```

Language bindings live in sibling repositories:
[Python (`nono-sandbox` on PyPI)](https://github.com/OscarMackJr/nono-py) ·
[TypeScript (`@oscarmackjr/nono-ts` on npm)](https://github.com/OscarMackJr/nono-ts).

## Workspace layout

| Crate / member | Role |
|----------------|------|
| `crates/nono` (`nono-sandbox`) | Core library — pure sandbox primitive, no built-in policy. |
| `crates/nono-cli` (`nono-sandbox-cli`) | CLI binary (`nono`) + `nono-agentd`, `nono-wfp-service` bins. Owns all security policy, profiles, hooks, UX. |
| `crates/nono-proxy` (`nono-sandbox-proxy`) | Network-filtering proxy — domain/endpoint allowlisting + credential injection. |
| `crates/nono-shell-broker` | Medium-IL broker for spawning Low-IL nono shell children on Windows. |
| `crates/nono-fltmgr-client` | Windows minifilter (filter-manager) client — kernel-driver spike surface. |
| `bindings/c` (`nono-ffi`) | C FFI bindings + generated `nono.h`. |

## Key Features

| Feature | Description |
|---------|-------------|
| **Kernel sandbox** | Landlock (Linux) + Seatbelt (macOS) + AppContainer/WFP (Windows). Irreversible, inherited by children. |
| **Windows-native parity** | AppContainer confinement, WFP kernel egress filtering, Job Objects, supervisor model, Low-IL broker, multi-tenant daemon. |
| **Credential injection** | Proxy mode keeps API keys outside the sandbox entirely. Keystore, 1Password, Apple Passwords. |
| **ZT-Infra signed overrides** | Two-key AND gate (offline ECDSA + live ZT-Infra allow) for auditable, attested policy overrides. |
| **Attestation** | Sigstore-based signing/verification of instruction files (SKILLS.md, CLAUDE.md, etc.). |
| **Network filtering** | Allowlist host + L7 endpoint filtering via local proxy. Cloud metadata endpoints hard-denied. |
| **Snapshots** | Content-addressable rollback with SHA-256 dedup and Merkle-tree integrity. |
| **Audit logs** | Verifiable logs of all agent actions, with optional remote upload and monitoring. |
| **Multiplexing** | Run multiple agents in parallel with separate sandboxes; attach/detach to long-running agents. |

## Relationship to upstream

The core sandbox primitive is periodically synced from upstream
([`nolabs-ai/nono`](https://github.com/nolabs-ai/nono)) via explicit divergence-audit + absorb
cycles; the fork's crate versions **leapfrog** upstream's to stay collision-free (currently
`0.66.1`). Upstream's own documentation at `docs.nono.sh` describes the shared core, but **does not
cover** this fork's Windows-native backend, install/distribution model, or ZT-Infra integration —
those are documented in-repo under `proj/`, `docs/`, and `.planning/`.

## Contributing

AI-assisted contributions are welcome, but you must understand and carefully review any AI-generated
code before submitting — security is paramount. All commits require a DCO `Signed-off-by` line.

## Security

**Security is non-negotiable in this codebase.** If you discover a vulnerability, please **do not
open a public issue** — follow the process in
[`SECURITY.md`](https://github.com/OscarMackJr/nono/security).

## License

Apache-2.0. Retains upstream copyright and attribution.
