# TODO: release signed with untrusted POC cert → broker non-functional on a clean host

**Captured:** 2026-06-11 (Phase 66 WR-02 EDR UAT, clean-host install) — **headline finding**
**Severity:** high — the public release's supervised/broker path does not work out-of-the-box
**Source:** `.planning/phases/66-wr-02-edr-human-uat/66-HUMAN-UAT.md` (findings)

## Problem
v0.62.2 is Authenticode-signed with a **self-signed `CN=nono Test Signing` POC cert**. On any
clean host that cert is untrusted → `Get-AuthenticodeSignature` returns `InvalidSignature`
(`0x800B0109` CERT_E_UNTRUSTEDROOT). The D-32-12 broker self-trust gate (correctly, fail-secure)
then **refuses to spawn the broker**, so `nono run --profile claude-code` (the supervised path)
**fails out-of-the-box**. We could only proceed in the UAT by manually importing the POC cert into
LocalMachine\Root + TrustedPublisher.

## Implication
The fork's public releases are effectively **dev/POC artifacts** for the supervised path — they
do not run on a clean machine without the operator trusting a self-signed cert. This is the most
material WR-02 finding.

## Fix options
- Sign releases with a **publicly-trusted** code-signing cert (EV/OV, e.g. via Azure Trusted
  Signing — note the existing `trusted-signing-smoke.yml` workflow), so Authenticode is `Valid`
  on any host, OR
- If keeping the POC cert for now: **document** the limitation prominently + ship the cert and an
  import step, and/or provide a `nono setup --trust-broker` helper. Cross-reference
  `docs/cli/development/windows-signing-guide.mdx`.

## Acceptance
`nono run --profile claude-code -- ...` spawns the broker on a clean Win11 host with no manual
cert-trust step (or the limitation is explicitly documented + a supported trust path exists).
