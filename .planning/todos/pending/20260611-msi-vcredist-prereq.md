# TODO: clean-VM UAT — confirm machine MSI installs on a fresh Win11 host (no VC++ redist)

**Captured:** 2026-06-11 (Phase 66 WR-02 EDR UAT, clean-host install)
**Narrowed:** 2026-06-25 — code fix verified DONE; only host-gated UAT remains
**Severity:** low (was medium) — root-cause code fix landed; remaining item is empirical clean-host confirmation
**Source:** `.planning/phases/66-wr-02-edr-human-uat/66-HUMAN-UAT.md` (findings)
**Resolves phase:** 80 — Clean-Host Install UAT (v2.13; INST-01) — origin Phase 67 (v2.11; DIST-01/DIST-02), UAT carried forward
**Resolves phase (v3.0):** 82 — Fleet Deployment Infrastructure (DEPLOY-01 / DEPLOY-06)
**Resolves phase (v3.1):** 90 — v3.0 Host-Gated UAT Drain (DRAIN-01 clean-VM silent MSI install)

## Original problem
On a clean Win11 host (no VC++ runtime), the v0.62.2 **machine** MSI failed `1603`: both
`nono.exe` (`0xC0000135` STATUS_DLL_NOT_FOUND) and `nono-wfp-service.exe` couldn't load, so
the MSI's `ServiceControl` start of `nono-wfp-service` timed out (SCM 7009) and the entire
install rolled back. Installing `vc_redist.x64.exe` first resolved both.

## Code fix — DONE (verified 2026-06-25)
The DLL-dependency root cause is resolved by static-linking the CRT; this is shipped:

- **`+crt-static` is wired** for the Windows MSVC target — commit **`a517284b`**
  *"feat(80-01): wire +crt-static across all Windows MSVC build paths (D-03)"* (Phase 80 / INST-01).
  Target-scoped in `.cargo/config.toml` under `[target.x86_64-pc-windows-msvc]` (does not affect
  Linux/macOS). CI/release.yml apply the same via step-level `RUSTFLAGS` env (config stanza is
  silently dropped when `RUSTFLAGS` is set).
- **Build-tested with static CRT:** `cargo build --release --workspace` succeeds with the flag
  active (incl. `aws-lc-sys`) — verified 2026-06-25.
- **Binaries confirmed static:** `target/release/nono.exe` and `nono-wfp-service.exe` carry **no
  `vcruntime140.dll` / `api-ms-win-crt-runtime` import** — no dynamic VC++ CRT dependency.
- **Service-start is non-fatal:** the machine MSI's `<ServiceInstall>` uses `Vital="no"` +
  `ErrorControl="ignore"` (D-04), so a service-start failure no longer rolls back the product.

## Remaining — host-gated UAT only (cannot be done from the dev host)
Boot a **clean Win11 VM with no VC++ runtime installed** and confirm:
1. The machine MSI installs cleanly (no `1603`, no rollback) with no manual redist step.
2. `nono.exe` launches (no `0xC0000135`).
3. If a machine MSI built **with** `-ServiceBinaryPath` is tested, `nono-wfp-service` install/start
   does not fail the product (Vital="no" path).

## Acceptance
v0.62.2-equivalent machine MSI installs and runs cleanly on a fresh Win11 host with no manual
redist step — confirmed on a clean VM (INST-01 / DRAIN-01).
