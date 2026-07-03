---
phase: 103-azure-clean-host-vm-iac-new-verify-dark-gates
plan: 01
subsystem: infra
tags: [azure, bicep, arm, iac, powershell, trusted-launch, nsg, vm]

# Dependency graph
requires:
  - phase: 101-verify-gate-hardening-azure-profile-confirmation
    provides: Azure Trusted Signing profile confirmed PublicTrust; ArtifactNono/RG_Nono live subscription context
provides:
  - Self-contained Bicep module (main.bicep) declaring a Gen2 + Trusted-Launch Windows 11 VM with its own VNet/Subnet/NSG/PublicIP/NIC
  - deploy.ps1 / teardown.ps1 wrapper scripts implementing an ephemeral create-to-use-to-teardown lifecycle against a dedicated RG_Nono_CleanHost resource group
  - Live-resolution pattern for image SKU (az vm image list-skus) and operator IP (curl, never az rest) that Phase 106 will reuse for the real deploy
affects: [106-clean-host-uat, 104-release-cut]

# Tech tracking
tech-stack:
  added: [Azure Bicep 0.44.1 (manually installed at ~/.azure/bin/bicep.exe per 103-RESEARCH.md Pitfall 1 workaround)]
  patterns:
    - "Bicep no-default required params to structurally forbid stale hardcoded config (vmImageSku, operatorIpCidr)"
    - "Ephemeral dedicated-RG lifecycle (RG_Nono_CleanHost) kept fully separate from the persistent RG_Nono signing-account RG"
    - "curl (never az rest / az's own Python HTTP client) for any live lookup against a non-management.azure.com host on this corporate-TLS-intercepted network"

key-files:
  created:
    - scripts/azure/clean-vm/main.bicep
    - scripts/azure/clean-vm/deploy.ps1
    - scripts/azure/clean-vm/teardown.ps1
  modified: []

key-decisions:
  - "main.bicep is fully self-contained (own VNet/Subnet/NSG/PublicIP/NIC) — RG_Nono has no reusable networking to attach to, and this avoids ever coupling the ephemeral VM fixture to the permanent ArtifactNono signing account resource group"
  - "vmImageSku and operatorIpCidr are required Bicep parameters with NO default value — the only path to a value is deploy.ps1's live az vm image list-skus / curl resolution, making a stale hardcoded SKU or a wildcard NSG rule structurally impossible to author by mistake"
  - "NSG authors exactly two rules: AllowRdpFromOperator (scoped to the operatorIpCidr parameter) and DenyAllOtherInbound (explicit defense-in-depth) — never a wildcard any-address CIDR"
  - "deploy.ps1/teardown.ps1 default to a dedicated ephemeral RG_Nono_CleanHost, never the pre-existing RG_Nono"
  - "Comments describing the forbidden literal patterns (wildcard CIDR, az's REST passthrough, TLS-bypass flags, async-delete flag) were phrased to avoid containing the literal grep-checked strings themselves, since the plan's acceptance criteria grep the whole file for those substrings with zero tolerance"

patterns-established:
  - "Pattern 1: Bicep no-default-param security gate — forbid unsafe defaults at the schema level, not just by convention"
  - "Pattern 2: az CLI wrapper scripts resolve all security-sensitive values (SKU, IP) live, immediately before the operation they gate, minimizing drift windows"

requirements-completed: [CHOST-01]

# Metrics
duration: 5min
completed: 2026-07-03
---

# Phase 103 Plan 01: Azure Clean-Host VM IaC Summary

**Authored a self-contained Gen2 + Trusted-Launch Windows 11 Bicep module (own VNet/NSG/PublicIP/NIC, no-default SKU/IP params) plus deploy.ps1/teardown.ps1 wrappers implementing an ephemeral RG_Nono_CleanHost create-to-use-to-teardown lifecycle — author + `az bicep build` validated only, no live Azure calls made.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-07-03T18:24:39Z
- **Completed:** 2026-07-03T18:28:34Z
- **Tasks:** 2
- **Files modified:** 3 (all newly created)

## Accomplishments
- `scripts/azure/clean-vm/main.bicep`: Gen2 + Trusted-Launch (`securityType: 'TrustedLaunch'`, `secureBootEnabled`/`vTpmEnabled: true`) Windows 11 VM, self-contained VNet/Subnet/NSG/PublicIP/NIC, `vmImageSku`/`operatorIpCidr` required params with no default. Compiles cleanly via `az bicep build -f main.bicep --stdout` (exit 0, valid ARM JSON, zero linter warnings).
- `scripts/azure/clean-vm/deploy.ps1`: resolves the Windows 11 SKU live via `az vm image list-skus` (regex-filtered, sorted descending, throws on no match — never a hardcoded fallback), resolves the operator's public IP live via `curl.exe` as the last step before deploy, creates the ephemeral `RG_Nono_CleanHost` resource group, assembles an ARM parameters-file JSON with the admin password converted from `[securestring]` only transiently, and removes the temp params file in a `finally` block.
- `scripts/azure/clean-vm/teardown.ps1`: the only supported cleanup path — prompts for explicit resource-group-name confirmation unless `-Force`, then runs a blocking (non-async) `az group delete`.
- All plan-specified grep/exit-code acceptance criteria pass (see Verification below). Neither script was executed live.

## Task Commits

Each task was committed atomically:

1. **Task 1: Install Bicep CLI (already present) + author and validate main.bicep** - `011a69d9` (feat)
2. **Task 2: Author deploy.ps1 and teardown.ps1 (ephemeral lifecycle, never invoked live)** - `9b8200f3` (feat)

**Plan metadata:** (this commit, following SUMMARY.md creation)

## Files Created/Modified
- `scripts/azure/clean-vm/main.bicep` - Self-contained Bicep module: VNet/Subnet/NSG/PublicIP/NIC/VM, Gen2 + Trusted-Launch, no-default `vmImageSku`/`operatorIpCidr` params
- `scripts/azure/clean-vm/deploy.ps1` - `az` CLI wrapper: live SKU + operator-IP resolution, ephemeral RG create, secure parameter-file deploy
- `scripts/azure/clean-vm/teardown.ps1` - `az` CLI wrapper: ephemeral RG deletion, the only supported cleanup path

## Decisions Made
- Bicep CLI was already installed at `~/.azure/bin/bicep.exe` (version 0.44.1) from Phase 103 research — the Pitfall 1 manual-download workaround did not need to be re-run; confirmed via `az bicep version` before authoring, per the plan's explicit "check first, reuse" instruction.
- Resource symbolic names (`vnet`, `nsg`, `pip`, `nic`, `vm`) use literal ARM `name` values (e.g. `nono-clean-vnet`) rather than parameterized names, since this is a throwaway ephemeral RG containing only these five resources — no naming-collision risk exists within `RG_Nono_CleanHost`.
- NSG resource is declared before `vnet` in file order (matching the plan's numbered narrative: (1) VNet references (2) NSG) — Bicep resolves the dependency graph via symbolic references regardless of textual declaration order, so this forward-reference works without an explicit `dependsOn`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Rephrased explanatory comments that accidentally contained the exact grep-forbidden literal strings**
- **Found during:** Task 1 and Task 2, immediately after first `az bicep build`/grep verification pass
- **Issue:** The plan's acceptance criteria grep the entire file for forbidden literal substrings (`0.0.0.0/0` in `main.bicep`; `az rest`, `--insecure`, `no-wait` in `deploy.ps1`/`teardown.ps1`) with zero tolerance. My first-draft header/inline comments *described* these forbidden patterns by name for documentation purposes (e.g. "NEVER `az rest`", "never authors `0.0.0.0/0`", "no `--no-wait`"), which caused the literal-string greps to find false-positive matches inside comments — even though no actual NSG rule, `az` invocation, or CLI flag in the executable code used the forbidden pattern.
- **Fix:** Reworded all such comments to describe the same security constraint without using the literal grep-checked substring (e.g. "never a wildcard any-address CIDR" instead of the literal `0.0.0.0/0`; "the Azure CLI's generic REST passthrough subcommand" instead of the literal `az rest`; "never queues the deletion asynchronously" instead of the literal `--no-wait`). No functional/security behavior changed — only comment wording.
- **Files modified:** `scripts/azure/clean-vm/main.bicep`, `scripts/azure/clean-vm/deploy.ps1`, `scripts/azure/clean-vm/teardown.ps1`
- **Verification:** Re-ran every acceptance-criteria grep after the rewording; all now pass (0 occurrences where required, ≥1 where required). `az bicep build` and the PowerShell parser syntax-check were both re-run and still exit 0.
- **Committed in:** `011a69d9` (main.bicep fix folded into the Task 1 commit before it was ever committed) and `9b8200f3` (deploy.ps1/teardown.ps1 fix folded into the Task 2 commit before it was ever committed) — no separate fix-up commit was needed since the rewording happened before either task's single commit.

---

**Total deviations:** 1 auto-fixed (1 bug — comment-only false-positive grep matches, no code/behavior change)
**Impact on plan:** Zero scope creep; purely wording adjustments to satisfy the plan's own literal-string acceptance criteria while preserving full documentation intent.

## Issues Encountered
None beyond the deviation above.

## User Setup Required
None - no external service configuration required. Bicep CLI was already present on the dev host from Phase 103 research; no fresh install was needed.

## Next Phase Readiness
- `scripts/azure/clean-vm/main.bicep`/`deploy.ps1`/`teardown.ps1` are authored, locally validated (`az bicep build` exit 0; PowerShell parser exit 0), and ready for Phase 106 (clean-host UAT) to actually invoke `deploy.ps1` against a live Azure subscription.
- No live deploy, no live resource creation, no live `az deployment group create`/`az group delete` call was made this phase — matches the plan's explicit author+lint-only scope.
- Remaining Phase 103 scope (per 103-RESEARCH.md/103-PATTERNS.md): `scripts/gates/trusted-signed-assertion.ps1` and `scripts/gates/broker-spawn-on-clean-host.ps1` (new verify-dark gates) are separate plans within this phase, not covered by this plan.

---
*Phase: 103-azure-clean-host-vm-iac-new-verify-dark-gates*
*Completed: 2026-07-03*

## Self-Check: PASSED

All created files confirmed present on disk; both task commit hashes (`011a69d9`, `9b8200f3`) confirmed present in `git log --oneline --all`.
