# Phase 82: Fleet Deployment Infrastructure - Context

**Gathered:** 2026-06-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Make nono **silently installable and self-provisioning across a corporate Windows fleet**. An
admin runs `msiexec /i nono.msi /qn /norestart` (under SYSTEM/SCCM/Intune) with no interactive
prompts and correct exit codes; afterward **every** user can invoke `nono` from a fresh shell with
zero manual steps. The MSI lays down only machine-global state; everything that must be user-owned
or per-user is auto-provisioned at first run in the user's own context.

Delivers (DEPLOY-01..06): silent `/qn` install with correct exit codes (0 / 3010 / 1603) and
SYSTEM-context safety; machine-wide PATH; user-owned `WRITE_OWNER` scratch at first run; POC root
cert into the machine + per-user trust stores so the supervised/broker + proxy-TLS paths work on a
clean host; non-fatal atomic service install; and a `nono health` JSON verdict command for fleet
diagnostics. Each requirement ships an unattended `scripts/verify-dark.ps1 --gate
deploy-silent-install` verdict (Dark Factory mandate, carried from v2.13).

**Not in this phase:** the HKLM policy *reader* and egress enforcement (Phase 83); the telemetry
emitter / SecurityEventLayer (Phase 84). Phase 82 only **provisions the physical prerequisites**
those phases build on (the `HKLM\SOFTWARE\Policies\nono` sentinel key, the Event Log source, the
`%PROGRAMDATA%\nono\` root).

</domain>

<decisions>
## Implementation Decisions

### VC++ Runtime — eliminate the clean-host 1603 rollback
- **D-01:** Build all three Windows binaries (`nono.exe`, `nono-wfp-service.exe`,
  `nono-shell-broker.exe`) with the **static CRT** (`-C target-feature=+crt-static`). No
  `vc_redist.x64.exe` is ever required on the target host; the install can no longer roll back on
  `STATUS_DLL_NOT_FOUND` (`0xC0000135`). This is the root-cause fix for the `msi-vcredist-prereq`
  todo, and it is what actually makes DEPLOY-01 "silent install works" true on a fresh host.
- **D-02:** Set the flag in **`.cargo/config.toml`** under `[target.x86_64-pc-windows-msvc]` so
  **every** Windows build — local dev, `make ci`, and the release MSI — is static. Guarantees the
  binary CI tests is byte-for-byte the binary shipped, closing the project's recurring
  "dev-host build ≠ shipped artifact" trap (e.g. the stale `C:\Program Files\nono\nono.exe`).
- **D-03 (accepted tradeoff):** Static CRT means the C runtime is no longer serviced by Windows
  Update — a CRT CVE requires rebuilding/reshipping nono rather than an OS patch. Accepted for the
  fleet-deploy reliability win. Note also: any future C/C++ dependency with dynamic-CRT assumptions
  would surface across all builds at once (single blast radius, by design).

### Root-Cert Trust Install (DEPLOY-05)
- **D-04 (machine store):** A **deferred, SYSTEM-context MSI custom action** imports the POC root
  cert via `certutil -addstore -f Root <cert>` **and** `certutil -addstore -f TrustedPublisher
  <cert>` (the latter satisfies the D-32-12 Authenticode broker self-trust gate). Chosen over the
  WiX util/iis `Certificate` element (finicky, IIS-extension-bound, can't easily also hit
  TrustedPublisher). The custom action is **non-fatal** — a cert-import failure does not roll back
  the product; `nono health` reports the degraded cert state instead.
- **D-05 (per-user + Node):** A SYSTEM MSI **cannot** write any user's `CurrentUser\Root`, and
  Node ignores the Windows cert store by default. So the **first-run-in-user-context provisioner**
  (see D-08) idempotently imports the POC cert into `CurrentUser\Root` if missing, and sets
  **`NODE_EXTRA_CA_CERTS`** pointing at a shipped PEM at `%PROGRAMDATA%\nono\` (passed to confined
  Node engines). This makes TLS-through-proxy succeed from PowerShell/CryptoAPI, Node.js, and
  nono-cli (rustls/native-certs) with no manual import. Resolves the `poc-cert-broker-clean-host`
  todo's clean-host trust path (real publicly-trusted Azure Trusted Signing remains DIST-SIGN-01,
  out of scope).

### `nono health` Command (DEPLOY-06)
- **D-06 (exit contract):** **Tri-state exit codes** — `0` = healthy (all subsystems OK), `1` =
  degraded-but-usable (e.g. WFP service stopped, machine policy not configured, cert/scratch
  missing), `2` = broken (PATH missing, exe can't self-locate, install incomplete). The full JSON
  verdict is **always printed to stdout** regardless of exit code, so SCCM/Intune compliance
  scripts can distinguish "remediate later" from "reinstall now". Satisfies success criterion 5
  ("degraded-service path produces non-zero `nono health`").
- **D-07 (inspected subsystems):** The JSON reports **all four** groups:
  (a) **install + version** — INSTALLFOLDER present, `nono.exe` self-locates, installed version +
  MSI ProductCode/UpgradeCode (drives `broken`/exit 2);
  (b) **WFP service** — install + running state via SCM query (stopped/failed → `degraded`/exit 1);
  (c) **machine policy** — presence/readability of `HKLM\SOFTWARE\Policies\nono`
  (configured / not-configured / unreadable — forward-looking for Phase 83);
  (d) **scratch + cert + PATH** — user-owned scratch provisioned (DEPLOY-03), POC cert in
  `LocalMachine\Root` + `CurrentUser\Root` (DEPLOY-05), machine PATH entry present (DEPLOY-02).

### Scratch Provisioning (DEPLOY-03)
- **D-08 (trigger + location):** Auto-provision on the **first `nono run` in user context** —
  create `%LOCALAPPDATA%\nono\<workspace>` with `WRITE_OWNER` set to the invoking user so the R-B3
  user-ownership guard passes, idempotently, with **no manual `nono setup` step**. The **MSI
  provisions only the machine-global `%PROGRAMDATA%\nono\`** (never user scratch — SYSTEM-context
  MSI would write to `C:\Windows\system32\config\systemprofile\...` and fail every user's R-B3
  guard; this is research Pitfall 4). `%LOCALAPPDATA%` chosen over `%USERPROFILE%\.nono`; both
  reliably support WRITE_OWNER (unlike drive-root `C:\poc\*`), LOCALAPPDATA is the more
  conventional app-scratch root.

### Unifying architectural decision
- **D-09:** Implement **one idempotent first-run-in-user-context provisioner** that does all three
  things the SYSTEM MSI cannot: (1) create the user-owned scratch (D-08), (2) import the POC cert
  into `CurrentUser\Root` (D-05), (3) set/propagate `NODE_EXTRA_CA_CERTS` (D-05). Single code path,
  single idempotency/"already provisioned" check, single failure-reporting surface that
  `nono health` reads.

### Claude's Discretion
- Exact JSON field names/shape of the `nono health` verdict (subsystem keys, nesting) — within the
  D-06/D-07 contract.
- Exact MSI custom-action sequencing/conditions for the cert import (deferred + impersonate=no),
  and how the PEM/CER artifacts are staged into the WiX harvest.
- Whether the Event Log **source for the `nono` CLI** (needed by Phase 84) is registered now in the
  MSI alongside the existing `nono-wfp-service` source, or deferred to Phase 84 — planner's call;
  if registered now, mirror the existing raw-RegistryKey `EventMessageFile`/`TypesSupported`
  pattern at `build-windows-msi.ps1:259`.
- Mechanism for first-run idempotency marker (registry sentinel vs filesystem marker under
  `%LOCALAPPDATA%\nono\`).

### Folded Todos
- **`msi-vcredist-prereq`** (`.planning/todos/pending/20260611-msi-vcredist-prereq.md`) — clean
  Win11 host (no VC++ runtime) → machine MSI `1603` rollback because `nono.exe` (`0xC0000135`) and
  `nono-wfp-service.exe` can't load, so the service-start times out and the whole install rolls
  back. **Folded:** resolved by D-01/D-02 (static CRT) as the primary fix; the existing `Vital=no`
  non-fatal service install is belt-and-suspenders.
- **`poc-cert-broker-clean-host`**
  (`.planning/todos/pending/20260611-poc-cert-broker-clean-host.md`) — releases are
  Authenticode-signed with a self-signed `CN=nono Test Signing` POC cert; on a clean host that cert
  is untrusted (`0x800B0109` CERT_E_UNTRUSTEDROOT), so the broker self-trust gate (correctly,
  fail-secure) refuses to spawn and `nono run --profile claude-code` fails out-of-the-box.
  **Folded:** resolved by D-04 (machine `Root` + `TrustedPublisher` import) and D-05 (per-user
  import). Real publicly-trusted signing (Azure Trusted Signing) remains **DIST-SIGN-01, out of
  scope** for v3.0.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & roadmap
- `.planning/REQUIREMENTS.md` §Deployment — DEPLOY-01..06 full text + decided scope locks
  (WiX-not-MSIX, POC-cert path, ProgramData root) and the Out-of-Scope table.
- `.planning/ROADMAP.md` §"Phase 82" — goal + 5 success criteria + the `deploy-silent-install` gate.

### Research (grounds this phase; planner should skip a separate research-phase per the flag)
- `.planning/research/SUMMARY.md` §"Phase 82" — build order, "skip deep research-phase / use
  existing build-windows-msi.ps1 as base" flag, Pitfalls 4/5/13 mapping, rustls_native_certs gap.
- `.planning/research/ARCHITECTURE.md` — integration points (`build-windows-msi.ps1` modifications,
  ProgramData root, machine-wide PATH Environment, WiX CertificateRef, util:EventSource).
- `.planning/research/PITFALLS.md` — Pitfall 4 (SYSTEM-context scratch → systemprofile), Pitfall 5
  (non-atomic service install + health verdict), Pitfall 13 (root-cert TLS trust-path matrix across
  PowerShell/Node/rustls).

### Folded todos
- `.planning/todos/pending/20260611-msi-vcredist-prereq.md` — VC++ runtime / 1603 rollback.
- `.planning/todos/pending/20260611-poc-cert-broker-clean-host.md` — POC-cert clean-host trust.

### Code & build (the surfaces this phase modifies)
- `scripts/build-windows-msi.ps1` — **GENERATED .wxs source**; edit the here-strings here, NOT the
  emitted `dist/windows/*.wxs`. Existing anchors: machine PATH `Environment` (`:395`), non-fatal
  `ServiceInstall Vital=no` (`:239`), `nono-wfp-service` Event Log source (`:259`),
  MajorUpgrade/UpgradeCode (`:343-348`).
- `scripts/validate-windows-msi-contract.ps1` — MSI contract assertions; update alongside the .ps1.
- `crates/nono-cli/src/cli.rs` / `main.rs` — add the `health` subcommand + the first-run provisioner
  hook on the `run` path.
- `.cargo/config.toml` — static-CRT rustflags target stanza (D-02).

### Verification
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol
  for any cfg-gated Unix code touched (health command will be cfg-heavy).
- `scripts/verify-dark.ps1` — add the `deploy-silent-install` gate (Dark Factory close signal).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `build-windows-msi.ps1` is a here-string MSI generator already producing machine + user scope
  variants — extend it; do not author raw .wxs. (memory: `windows_msi_wxs_is_generated`)
- Existing `nono-wfp-service` Application Event Log source registration (raw RegistryKey
  `EventMessageFile`/`TypesSupported`, `:259`) is the proven pattern to copy if a `nono` CLI source
  is registered now for Phase 84.
- The R-B3 user-ownership primitives from Phase 60/72 (`grant_sid_write_on_path`,
  `path_is_owned_by_current_user`, `GetEffectiveRightsFromAclW` mask check,
  `icacls /setowner *<SID>`) are the building blocks for the first-run scratch WRITE_OWNER grant.
- `nono setup` / `--trust-broker` verb concept already referenced in the poc-cert todo and signing
  guide — the cert-import logic can live in testable Rust reused by both the MSI CA and first-run.

### Established Patterns
- **Library/CLI boundary is sacrosanct:** all of this lands in `nono-cli` (and the build scripts).
  The `nono` library stays policy-free — no registry reads, no cert logic, no health command.
- **Configuration load failures must be fatal / fail-secure** — but cert + service + scratch
  provisioning are *non-fatal* to the install and *reported* by `nono health` (degraded, not
  abort). The fail-secure rule applies to the Phase 83 policy reader, not Phase 82 provisioning.
- **Path footguns:** use path-component comparison; `%USERPROFILE%`/`%LOCALAPPDATA%`/`%TEMP%`
  support WRITE_OWNER, drive-root `C:\poc\*` does not (memory:
  `windows_mandatory_label_write_owner`).

### Integration Points
- MSI `INSTALLFOLDER` → machine PATH (`HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\
  Environment`) already wired (`Environment System=yes`); verify it satisfies DEPLOY-02 from a
  fresh shell.
- First-run provisioner hooks the `nono run` entry path before sandbox apply.
- `nono health` reads SCM (service), HKLM (policy sentinel + PATH), cert stores, and the scratch
  marker — a read-only diagnostic, no mutation.

</code_context>

<specifics>
## Specific Ideas

- The static-CRT + first-run-provisioner pairing is the spine: MSI = machine-global only; first run
  = everything user-owned. Keep that split visible in the plan.
- `verify-dark.ps1 --gate deploy-silent-install` must cover: silent install under SYSTEM context;
  scratch owned by the *target user* not SYSTEM; degraded-service path → non-zero `nono health`;
  TLS trust verified across PowerShell (CryptoAPI), Node, and nono-cli (rustls/native-certs).
- Clean-host UAT remains operator-gated tech-debt; the scripted gate is the close signal on a
  single dev host (carried Dark Factory standard).

</specifics>

<deferred>
## Deferred Ideas

- **Real publicly-trusted code signing (Azure Trusted Signing / DIST-SIGN-01)** — explicitly out of
  scope; the POC-cert trust path is the v3.0 answer. Belongs to a future distribution milestone.
- **MSIX packaging** — permanently out of scope (cannot package the LocalSystem WFP service /
  kernel driver). Stay on WiX MSI.
- **Live machine-policy reload (`RegNotifyChangeKeyValue`)** — EGRESS-FU-01; v3.0 is
  read-at-startup. Not Phase 82's concern (and the policy reader itself is Phase 83).
- **`macos-rlimit-as-setrlimit-fails`** todo — surfaced as a low-score (0.2) keyword match only;
  reviewed and **not folded** (macOS resource-limit defect, unrelated to Windows fleet deploy).

</deferred>

---

*Phase: 82-fleet-deployment-infrastructure*
*Context gathered: 2026-06-18*
