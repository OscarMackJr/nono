---
phase: 260522-cui
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - docs/cli/development/windows-poc-handoff.mdx
  - docs/cli/development/windows-service-packaging.mdx
  - docs/cli/development/windows-network-enforcement.mdx
autonomous: true
requirements:
  - QUICK-CUI-DOCS-01  # POC handoff Option C reflects machine-MSI WFP build/install path
  - QUICK-CUI-DOCS-02  # Step 3 pre-flight no longer claims WFP-missing is "expected"
  - QUICK-CUI-DOCS-03  # Cross-doc references (service-packaging, network-enforcement) drop "if/when" framing
tags: [windows, msi, wfp, docs, poc, handoff]

must_haves:
  truths:
    - "POC users following Option C learn how to build BOTH the user-scope MSI (no WFP) AND the machine-scope MSI (with WFP) and which to pick based on whether they need --block-net / --network-profile / --allow-domain."
    - "The Step 1 Option C build command no longer omits nono-wfp-service from the cargo build invocation when targeting machine scope."
    - "The Step 1 Option C section shows a machine-scope `build-windows-msi.ps1` invocation that passes BOTH `-ServiceBinaryPath` AND `-DriverBinaryPath` (the script's scope-coherence guard requires both-or-neither)."
    - "The `<Warning>` block at handoff.mdx:66-68 is replaced with content that explains the user-vs-machine tradeoff explicitly (kernel driver cannot load from per-user LocalAppData; LocalSystem service needs admin install) rather than the stale claim that machine MSI registers the service 'only when you explicitly pass `-ServiceBinaryPath`' (the RELEASE pipeline now passes both flags automatically — only LOCAL machine MSI builds still need them)."
    - "Step 2 (Stage on the test machine) covers the admin-install path: `msiexec /i nono-...-machine.msi` and notes the elevation prompt POC users will see."
    - "Step 3 pre-flight reframes the WFP readiness line: `missing binary` is expected ONLY for user-scope/portable-zip installs; machine-MSI installs should report a registered+ready WFP service. The current `— **expected** for the POC` framing is replaced with branching guidance keyed on install path."
    - "Step 3 (or a sibling section) documents the post-install one-time `nono setup --install-wfp-service --install-wfp-driver --start-wfp-driver --start-wfp-service` command sequence required after a machine MSI install (MSI lands the .sys file; CLI registers it as a kernel driver per quick task 260522-c9c's third decision)."
    - "windows-service-packaging.mdx no longer says 'if and when service-capable packaging is promoted' — that promotion shipped in quick task 260522-c9c. The intro is reframed to describe the actual machine-MSI contract."
    - "windows-network-enforcement.mdx's 'When the service and driver both probe as present...' framing is updated to acknowledge the machine MSI ships them by default; user-scope MSI deliberately omits them."
  artifacts:
    - path: "docs/cli/development/windows-poc-handoff.mdx"
      provides: "Updated Option C with both user-scope and machine-scope build paths; updated Step 2 install paths; updated Step 3 WFP-readiness framing; replacement Warning block"
      contains: ["-ServiceBinaryPath", "-DriverBinaryPath", "nono-wfp-service", "nono-wfp-driver", "Scope machine", "install-wfp-driver"]
    - path: "docs/cli/development/windows-service-packaging.mdx"
      provides: "Reframed intro that drops 'if and when' aspirational language; describes the actual machine-MSI service+driver contract as shipped in quick task 260522-c9c"
      contains: ["machine MSI", "nono-wfp-service", "nono-wfp-driver"]
    - path: "docs/cli/development/windows-network-enforcement.mdx"
      provides: "Updated 'When the service and driver both probe as present' paragraph (around line 129) to reflect that the machine MSI ships both binaries by default and the user MSI deliberately omits them"
      contains: ["machine MSI", "user MSI", "deliberately"]
  key_links:
    - from: "docs/cli/development/windows-poc-handoff.mdx"
      to: "quick task 260522-c9c (commits 169c56d7 + 5c457929)"
      via: "Commit message references in the doc update commit (NOT inline doc links — those would couple the doc to internal planning state)"
      pattern: "260522-c9c"
    - from: "docs/cli/development/windows-poc-handoff.mdx § Step 1 Option C"
      to: "scripts/build-windows-msi.ps1 `-ServiceBinaryPath` + `-DriverBinaryPath` params"
      via: "PowerShell example invocation in the doc"
      pattern: "-ServiceBinaryPath\\s+.+-DriverBinaryPath"
    - from: "docs/cli/development/windows-poc-handoff.mdx § Step 3"
      to: "`nono setup --install-wfp-service` / `--install-wfp-driver` CLI flags"
      via: "Post-install command example, gated on 'if you installed the machine MSI and need WFP'"
      pattern: "install-wfp-(service|driver)"
---

<objective>
Update the Windows POC handoff documentation so POC users who need WFP-based
network filtering (the `--block-net`, `--network-profile`, `--allow-domain`
shapes that exercise the runtime activation path in
`exec_strategy_windows::network::probe_wfp_runtime`) know to install the
machine-scope MSI and run the post-install service+driver registration
commands. Quick task 260522-c9c (commits `169c56d7` + `5c457929`) shipped the
MSI fix — the machine-scope MSI now bundles both `nono-wfp-service.exe` and
`nono-wfp-driver.sys` at `INSTALLFOLDER` and the release+CI pipelines pass
both flags automatically. The handoff doc, however, still tells POC users to
build the user-scope MSI (which deliberately excludes WFP by design), which
is exactly what caused this session's POC user to hit `Expected WFP backend
service binary is missing` at runtime when they tried `--block-net`.

Purpose: Close the documentation gap between "the MSI fix shipped" (done
2026-05-22) and "POC users know how to use it" (this task). Without this
update, the next POC user who needs network filtering will hit the same
runtime fail-closed dead-end and have no documented recovery path.

Output: Three docs updated inline (handoff is the primary; service-packaging
and network-enforcement get drift fixes). No code changes.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/quick/260522-c9c-msi-install-missing-nono-wfp-service-bin/260522-c9c-SUMMARY.md
@docs/cli/development/windows-poc-handoff.mdx
@docs/cli/development/windows-service-packaging.mdx
@docs/cli/development/windows-network-enforcement.mdx
@scripts/build-windows-msi.ps1

<interfaces>
<!-- Key contracts the executor needs. Extracted from the codebase so the -->
<!-- executor does not need to re-explore the source. -->

From `scripts/build-windows-msi.ps1` (lines 1-38, the param block):
```powershell
param(
    [Parameter(Mandatory = $true)] [string]$VersionTag,
    [Parameter(Mandatory = $true)] [string]$BinaryPath,
    [Parameter(Mandatory = $true)] [string]$BrokerPath,        # Phase 31 Plan 04, mandatory
    [ValidateSet("machine", "user")] [string]$Scope = "machine",
    [string]$OutputDir = "dist/windows",
    [string]$Manufacturer = "Luke Hinds",
    [string]$ServiceBinaryPath = "",                           # WFP service exe — machine scope ONLY
    [string]$DriverBinaryPath  = "",                           # quick task 260522-c9c, WFP driver .sys
    [switch]$EmitOnly
)
```

From `scripts/build-windows-msi.ps1` (lines 171-190, the scope-coherence guards):
```powershell
# user scope + WFP flags  → throws "WFP service/driver binaries are machine-scope only."
# machine scope + service XOR driver → throws "Machine-scope MSI requires both -ServiceBinaryPath and -DriverBinaryPath, or neither."
# machine scope + BOTH    → emits cmpWfpServiceExe + cmpWfpDriverSys + cmpEventLogSource
# machine scope + NEITHER → emits no WFP components (machine MSI without WFP backend)
```

From `crates/nono-cli/src/cli.rs` (lines 646-651, the post-install command surface):
```
nono setup --install-wfp-service     # Register the Windows WFP service
nono setup --install-wfp-driver      # Register the Windows WFP kernel driver
nono setup --start-wfp-service       # Start the registered service
nono setup --start-wfp-driver        # Start the registered driver
nono setup --install-wfp-service --install-wfp-driver --start-wfp-driver --start-wfp-service
                                     # All four in one go
```

From `dist/windows/nono-user.wxs` (260522-c9c explanatory comment):
The user-scope MSI deliberately omits cmpWfpServiceExe and cmpWfpDriverSys.
Forward reference: `exec_strategy_windows::network::probe_wfp_runtime`.

From quick task 260522-c9c decision #3:
> Kernel driver registration (sc create ... type=kernel /
> SERVICE_KERNEL_DRIVER) is handled POST-INSTALL by the existing CLI command
> `nono setup --install-wfp-driver`, NOT by WiX. WiX's `<ServiceInstall>`
> only models user-mode services and cannot represent kernel drivers; the
> MSI's responsibility is solely to land the .sys file at a well-known
> sibling path.

This means the machine MSI is NECESSARY BUT NOT SUFFICIENT for WFP: after
install, the POC user STILL needs to run the four `nono setup --*-wfp-*`
commands once, elevated.

From the driver path convention (260522-c9c decision #1):
The driver source for local machine-MSI builds is the checked-in pre-signed
copy at `crates/nono-cli/data/windows/nono-wfp-driver.sys` — NOT the
`target/x86_64-pc-windows-msvc/release/nono-wfp-driver.sys` dev artifact
(which is not WHQL-signed).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Update windows-poc-handoff.mdx — Option C build/install paths, Warning rewrite, Step 3 reframe</name>
  <files>docs/cli/development/windows-poc-handoff.mdx</files>
  <action>
Make four coordinated inline edits to `docs/cli/development/windows-poc-handoff.mdx`.
DO NOT rewrite the document — preserve the existing Option A/B/C structure and
the Step 1..7 flow. All edits are additive or targeted replacements.

**Edit 1 — Step 1 Option C (lines 44-79): split into user-scope and machine-scope sub-paths.**

Currently Option C shows ONE build invocation (user-scope MSI). Replace it
with TWO sub-paths under Option C:

- **Sub-path C.1 — Per-user MSI (no WFP):** Keep the existing build command
  at line 53 (`cargo build --release --target x86_64-pc-windows-msvc -p
  nono-cli -p nono-shell-broker`) and the existing `build-windows-msi.ps1`
  invocation at lines 56-61 (`-Scope user`) verbatim. Add a one-line note
  above it: "Use this if your POC users only need filesystem sandboxing —
  no `--block-net`, no `--network-profile`, no `--allow-domain`."

- **Sub-path C.2 — Machine MSI (with WFP backend):** New invocation.
  - Cargo build command: `cargo build --release --target
    x86_64-pc-windows-msvc -p nono-cli -p nono-shell-broker -p
    nono-wfp-service`. Add a brief note explaining this also builds
    `nono-wfp-service.exe` under `target\x86_64-pc-windows-msvc\release\`.
  - Mention the kernel driver source explicitly: the local build path uses
    the checked-in pre-signed copy at
    `crates\nono-cli\data\windows\nono-wfp-driver.sys` (NOT the
    `target/...` dev artifact — Windows refuses to load unsigned kernel
    drivers in production). Cite this as the same convention the CI/release
    pipeline uses per quick task 260522-c9c.
  - `build-windows-msi.ps1` invocation with ALL of:
    `-VersionTag v0.37.1-poc.1`, `-BinaryPath
    .\target\x86_64-pc-windows-msvc\release\nono.exe`, `-BrokerPath
    .\target\x86_64-pc-windows-msvc\release\nono-shell-broker.exe`,
    `-ServiceBinaryPath
    .\target\x86_64-pc-windows-msvc\release\nono-wfp-service.exe`,
    `-DriverBinaryPath
    .\crates\nono-cli\data\windows\nono-wfp-driver.sys`, `-Scope machine`,
    `-OutputDir dist\windows`.
  - Note: scope-coherence guard in build-windows-msi.ps1 throws if you pass
    `-ServiceBinaryPath` without `-DriverBinaryPath` or vice-versa.
  - Output path: `dist\windows\nono-v0.37.1-poc.1-x86_64-pc-windows-msvc-machine.msi`.
  - One-line decision aid: "Use this if your POC users need WFP-enforced
    network filtering (`--block-net`, `--network-profile`, `--allow-domain`)
    or if the agent will issue requests that exercise the WFP runtime
    activation path. Requires admin install."

**Edit 2 — `<Warning>` block at lines 66-68: rewrite it.**

The existing Warning says the machine MSI registers `nono-wfp-service` "only
when you explicitly pass `-ServiceBinaryPath`" and tells users to stay on
`-Scope user` for the POC. This is half-stale: the RELEASE pipeline now
passes both flags automatically (per quick task 260522-c9c commit
`5c457929`), and the POC user this session did need WFP and hit a runtime
fail-closed because the user-scope MSI was the only documented path.

Replace it with a `<Note>` (not `<Warning>` — the new content is
informational, not a hazard) that:
- States the per-user vs machine MSI tradeoff explicitly: per-user is
  simpler (no admin install, no service to undo) but deliberately excludes
  WFP because (a) the kernel driver cannot load from per-user
  `LocalAppData`, and (b) the LocalSystem WFP service requires admin
  install.
- Notes that the official release MSI (downloaded from GitHub Releases) is
  pre-built with both flags via the CI pipeline; local machine-MSI builds
  must pass `-ServiceBinaryPath` AND `-DriverBinaryPath` together (the
  script's scope-coherence guard throws on XOR).
- Cross-references Step 3 for the post-install one-time service+driver
  registration command sequence (machine MSI lands the binaries but kernel
  driver registration is done by the CLI per the WiX `<ServiceInstall>`
  limitation).

**Edit 3 — Step 3 pre-flight (around line 110): reframe WFP readiness line.**

Currently line 110 says: `WFP readiness: missing binary — **expected** for
the POC. The claude-code profile does not require WFP; this only matters for
domain-level network filtering, which is out of scope for the POC.`

Replace with branching guidance keyed on install path:

- **Per-user MSI / portable zip:** `WFP readiness: missing binary` is the
  expected state. WFP-backed network filtering (`--block-net`,
  `--network-profile`, `--allow-domain`) is intentionally unsupported on
  this install path — the kernel driver and LocalSystem service cannot ship
  to per-user LocalAppData.
- **Machine MSI (fresh install, before post-install setup):** `WFP
  readiness: missing service` is the expected state. The MSI landed
  `nono-wfp-service.exe` and `nono-wfp-driver.sys` at `INSTALLFOLDER` (per
  quick task 260522-c9c) but kernel driver registration happens
  post-install via the CLI. Run the four-command setup sequence below.
- **Machine MSI (after running post-install setup):** `WFP readiness: ok`.
  Network filtering shapes work.

Then add a new sub-section (heading: `Post-install WFP registration (machine
MSI only)`) with the elevated PowerShell command:

```powershell
# Admin PowerShell, one-time after machine MSI install:
nono setup --install-wfp-service --install-wfp-driver --start-wfp-driver --start-wfp-service
```

Explain briefly: the MSI lands the binaries; the CLI command performs the
kernel-driver `sc create ... type=kernel` registration that WiX's
`<ServiceInstall>` cannot represent (per quick task 260522-c9c decision #3).
Subsequent `nono setup --check-only` runs should report `WFP readiness: ok`.

**Edit 4 — Step 6 handoff table (lines 503-516): update the "Install" row
and "Not yet supported on this build" row.**

- "Install" row: split into two lines or two rows for per-user vs machine
  MSI. Per-user: `msiexec /i nono-vX-...-user.msi` (no admin). Machine:
  `msiexec /i nono-vX-...-machine.msi` (admin prompt) followed by the
  four-command post-install registration sequence.
- "Not yet supported on this build" row: the existing text says
  "Domain-level network filtering ... needs WFP service, which the per-user
  MSI does not ship today." That's true for per-user but stale for machine.
  Update to: "Per-user MSI deliberately excludes WFP; for `--block-net`,
  `--network-profile`, or `--allow-domain`, install the machine MSI and run
  the post-install registration commands (see Step 3). `nono shell`
  per-session WFP differentiation remains waived on the broker path; falls
  back to AppID-based filtering."

Keep all existing language about `nono shell` security envelope, Sigstore
trust root, etc. — those sections are accurate and out of scope here.

Cross-reference quick task 260522-c9c in the commit message (NOT in the doc
body — keep the doc free of internal planning slugs).
  </action>
  <verify>
    <automated>cd /c/Users/OMack/Nono && grep -c -- "-ServiceBinaryPath" docs/cli/development/windows-poc-handoff.mdx | grep -v "^0$" &amp;&amp; grep -c -- "-DriverBinaryPath" docs/cli/development/windows-poc-handoff.mdx | grep -v "^0$" &amp;&amp; grep -c "install-wfp-driver" docs/cli/development/windows-poc-handoff.mdx | grep -v "^0$" &amp;&amp; grep -c "Scope machine" docs/cli/development/windows-poc-handoff.mdx | grep -v "^0$" &amp;&amp; grep -c "nono-wfp-service" docs/cli/development/windows-poc-handoff.mdx | grep -v "^0$"</automated>
  </verify>
  <done>
    - Option C contains both a user-scope (existing) and a machine-scope (new) build path.
    - Machine-scope invocation includes `-ServiceBinaryPath`, `-DriverBinaryPath`, and `-Scope machine`.
    - Machine-scope cargo build includes `-p nono-wfp-service`.
    - Stale `<Warning>` block at lines 66-68 is replaced with a `<Note>` that explains user-vs-machine tradeoff.
    - Step 3 has branching WFP-readiness guidance keyed on install path AND a post-install registration sub-section with the four-command CLI sequence.
    - Step 6 handoff table updated for both install paths and corrected WFP-not-supported language.
    - File remains valid MDX (no broken JSX/MDX tags introduced).
    - No internal planning slugs (`260522-c9c`, `.planning/`, `quick task`) leak into the doc body.
  </done>
</task>

<task type="auto">
  <name>Task 2: Fix cross-doc drift in windows-service-packaging.mdx and windows-network-enforcement.mdx</name>
  <files>docs/cli/development/windows-service-packaging.mdx, docs/cli/development/windows-network-enforcement.mdx</files>
  <action>
Two surgical fixes — both files have language that pre-dates quick task
260522-c9c and now describes the wrong state of the world.

**Edit 1 — `docs/cli/development/windows-service-packaging.mdx` intro and "Machine MSI only" section (lines 6-34):**

The current intro (line 8) says: "This document describes the intended
Windows service-packaging contract for `nono-wfp-service` **if and when**
service-capable packaging **is promoted into the supported machine-MSI
release path**." That `if and when` is now stale — the promotion shipped in
quick task 260522-c9c (commits `169c56d7` + `5c457929`).

Rewrite the intro and the "Machine MSI only" section to describe the
**actual** machine-MSI contract as shipped:
- The machine-scope MSI (`nono-vX.Y.Z-...-machine.msi`) bundles both
  `nono-wfp-service.exe` and the pre-signed `nono-wfp-driver.sys` at
  `[InstallFolder]` (typically `C:\Program Files\nono\`).
- The MSI's `<ServiceInstall>` directive registers the user-mode service
  (`nono-wfp-service`) with SCM as demand-start under `LocalSystem`.
- The kernel driver (`nono-wfp-driver`, type `SERVICE_KERNEL_DRIVER`) is
  registered POST-INSTALL via the CLI command `nono setup
  --install-wfp-driver`, because WiX's `<ServiceInstall>` directive cannot
  represent kernel drivers.
- The user-scoped MSI (`...-user.msi`) deliberately omits both binaries
  (kernel driver cannot load from per-user `LocalAppData`; LocalSystem
  service requires admin install). The runtime probe in
  `exec_strategy_windows::network::probe_wfp_runtime` fail-closes with a
  directive message if WFP runtime activation is attempted from a
  user-scope install.

The "What is not in scope" section at the bottom (lines 183-189) currently
says "Driver packaging (`nono-wfp-driver.sys`) — the driver artifact is not
part of the current public MSI release contract." That is now FALSE for
machine scope. Update it:
- Remove or rewrite the driver bullet to clarify: the pre-signed driver IS
  bundled in the machine MSI (per quick task 260522-c9c); only auto-start
  remains out of scope.
- Keep the auto-start-at-boot bullet (demand-start is still the deliberate
  default).
- Keep the user-MSI-no-service bullet (still correct).

Preserve all the SCM attribute tables, lifecycle commands (`sc.exe query`,
`Get-Service`), upgrade/uninstall sections, troubleshooting, etc. — those
are still accurate. Only the framing (`if and when`, "not in scope") needs
to flip from aspirational to descriptive.

**Edit 2 — `docs/cli/development/windows-network-enforcement.mdx` lines 128-145:**

The current paragraph at line 129 says: "When the service and driver both
probe as present, the Windows runtime performs backend activation by
running `nono-wfp-service --probe-runtime-activation` with a JSON
activation request over stdio."

That technical content is correct; the missing context is HOW the service
and driver get to "probe as present" today. Add one sentence (around line
129 or as a parenthetical) that clarifies: the machine MSI ships both
binaries to `[InstallFolder]` and `nono setup --install-wfp-service
--install-wfp-driver --start-wfp-service --start-wfp-driver` registers them
with SCM post-install. The user-scope MSI deliberately omits both.

Also check the bullets at lines 191-198 — the "this repo now ships the
first backend artifacts" framing for `nono-wfp-service` and
`nono-wfp-driver.sys` is still accurate as written. The "current
post-build probe results land on those service/driver readiness failures"
sentence around line 198 should add a clarifying clause: "...unless the
operator has run the four-command post-install registration sequence on a
machine MSI install."

Do NOT rewrite the design-spike narrative, the WFP layer enumeration, or
the `WfpNetworkBackend` / `FirewallRulesNetworkBackend` split discussion —
those are accurate and load-bearing for the design context.

For both files: keep commit message reference to quick task 260522-c9c so
the change is traceable. Do not insert internal planning slugs into the
doc body.
  </action>
  <verify>
    <automated>cd /c/Users/OMack/Nono && grep -c "if and when" docs/cli/development/windows-service-packaging.mdx | grep -E "^(0|1)$" &amp;&amp; ! grep -q "if and when service-capable packaging is promoted" docs/cli/development/windows-service-packaging.mdx &amp;&amp; grep -c "machine MSI" docs/cli/development/windows-service-packaging.mdx | grep -v "^0$" &amp;&amp; grep -c "machine MSI" docs/cli/development/windows-network-enforcement.mdx | grep -v "^0$"</automated>
  </verify>
  <done>
    - `windows-service-packaging.mdx` no longer contains the literal phrase "if and when service-capable packaging is promoted" (the load-bearing stale claim).
    - `windows-service-packaging.mdx` intro and "Machine MSI only" section describe the actual shipped contract (machine MSI bundles both binaries; user MSI deliberately omits both).
    - `windows-service-packaging.mdx` "What is not in scope" section no longer lists driver packaging as out of scope (only auto-start-at-boot remains).
    - `windows-network-enforcement.mdx` line ~129 paragraph clarifies how the service/driver reach "probe as present" via machine MSI + post-install setup.
    - `windows-network-enforcement.mdx` user-scope MSI exclusion is documented.
    - Both files remain valid MDX.
    - No internal planning slugs in doc body.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| docs → POC operator → POC user | Docs influence what POC operators build and what install commands POC users run. Stale docs caused this session's runtime fail-closed; corrected docs prevent recurrence. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-260522-cui-01 | Information Disclosure | POC handoff doc | accept | Doc is already public; this update does not add new sensitive info. Internal planning slugs (`260522-c9c`, `.planning/...`) are deliberately kept OUT of the doc body — only in commit messages — to avoid coupling external docs to internal planning state. |
| T-260522-cui-02 | Tampering | Doc accuracy regression | mitigate | Both tasks have grep-based `<verify>` blocks that fail if key load-bearing phrases ("if and when service-capable packaging is promoted", "-ServiceBinaryPath", "install-wfp-driver", etc.) are absent or remain in their stale form. CI doc lint is out of scope (no MDX linter in this repo's CI today), so the grep gates ARE the regression-detection layer. |
| T-260522-cui-03 | Denial of Service | Operator misled into wrong install path | mitigate | The whole purpose of this task: the current per-user-only POC path silently denies WFP at runtime when POC users try `--block-net`. The updated doc gives operators a documented machine-MSI path with clear post-install steps, so the runtime fail-closed is no longer a surprise. The Step 3 branching guidance keyed on install path is the operator-facing recovery surface. |
</threat_model>

<verification>
After Task 1 and Task 2 complete:

1. **Both grep gates from each task pass.** (See `<verify>` blocks above.)
2. **MDX render check** — Manually scan all three files for unbalanced JSX
   tags (`<Note>`, `<Warning>`, code fences). Mintlify/Next.js MDX parsers
   are strict about unclosed tags. If a `<Note>` is opened, it must close.
3. **No internal planning slugs leak into doc bodies.**
   ```bash
   ! grep -i "quick task" docs/cli/development/windows-poc-handoff.mdx
   ! grep -i "260522-c9c" docs/cli/development/windows-poc-handoff.mdx
   ! grep -i "\.planning/" docs/cli/development/windows-poc-handoff.mdx
   ! grep -i "260522-c9c" docs/cli/development/windows-service-packaging.mdx
   ! grep -i "260522-c9c" docs/cli/development/windows-network-enforcement.mdx
   ```
   (Commit messages MUST reference `260522-c9c` for traceability; doc
   bodies MUST NOT.)
4. **Cross-document consistency** — handoff.mdx machine MSI build command
   uses the same `-DriverBinaryPath
   .\crates\nono-cli\data\windows\nono-wfp-driver.sys` path that
   service-packaging.mdx describes, and that network-enforcement.mdx's
   "service and driver both probe as present" framing references. A spot
   check: all three docs converge on "machine MSI ships both, user MSI
   ships neither, post-install CLI registers the driver."
5. **`nono setup` command surface matches `crates/nono-cli/src/cli.rs`
   lines 646-651.** The exact flags
   `--install-wfp-service / --install-wfp-driver / --start-wfp-service /
   --start-wfp-driver` MUST match the CLI surface verbatim — typos here
   would send POC users to nonexistent flags. Verify by grep:
   ```bash
   for flag in install-wfp-service install-wfp-driver start-wfp-service start-wfp-driver; do
     grep -q -- "--$flag" docs/cli/development/windows-poc-handoff.mdx || echo "MISSING: $flag"
     grep -q -- "$flag" crates/nono-cli/src/cli.rs || echo "CLI DRIFT: $flag not in cli.rs"
   done
   ```
</verification>

<success_criteria>
- POC user reading the updated handoff doc end-to-end can: (a) understand
  the per-user vs machine MSI tradeoff, (b) build the right MSI for their
  use case, (c) install it correctly, (d) run the post-install
  registration sequence if they picked machine, and (e) interpret the
  `nono setup --check-only` WFP-readiness output correctly for their
  install path.
- A POC user attempting `nono run --block-net` will EITHER have a working
  WFP backend (because they installed the machine MSI and ran the setup
  commands per Step 3) OR get an actionable directive message that points
  them to the machine-MSI install path (because they're on user-scope and
  WFP is correctly fail-closed-unavailable).
- The two cross-reference docs (`windows-service-packaging.mdx`,
  `windows-network-enforcement.mdx`) no longer contradict
  `windows-poc-handoff.mdx` on the machine-MSI WFP contract.
- All three docs survive Mintlify/Next.js MDX render without parser
  errors.
- Single commit with message `docs(windows): update POC handoff with
  machine-MSI WFP install path (follow-up to 260522-c9c)` plus DCO
  sign-off, touching exactly the three files in `files_modified`.
</success_criteria>

<output>
After completion, create `.planning/quick/260522-cui-update-poc-msi-build-install-instruction/260522-cui-SUMMARY.md`
documenting which sections were edited, the exact phrasing changes for the
`<Warning>` → `<Note>` swap and the WFP-readiness reframe, and a self-check
asserting the verify gates all pass.
</output>
