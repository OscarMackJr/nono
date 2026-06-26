---
phase: 97
slug: release-engineering-leapfrog-pipeline-runbook
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-26
---

# Phase 97 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register authored at plan time across 97-01..97-04 PLAN.md; mitigations independently
> verified in implementation by gsd-security-auditor (verdict: SECURED, 17/17 CLOSED).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| fork tree → crates.io / PyPI / npm registries | A version-string snapshot becomes the immutable identity of a published artifact; an inconsistency is uncorrectable post-publish (yank only). | release version strings, manifests |
| Nono repo → sibling binding repos (nono-py, nono-ts) | Cross-repo version drift produces a binding that pins a non-existent core version. | version + path-dep pins |
| CI runner → released MSI/binary assets | Signing happens here; wrong order ships unsigned payloads to end users who trust the installer. | Authenticode signatures, MSI payloads |
| GitHub Actions startup validator → workflow graph | A malformed reusable-workflow call fails the whole release at 0s, silently shipping nothing. | workflow YAML graph |
| homebrew formula → release tarball source repo | The download-url decides which GitHub repo's tarball brew fetches. | tarball download URL |
| dry-run script → crates.io / PyPI / npm | A mis-typed command (publish without --dry-run, twine upload) would push a real immutable artifact during validation. | publish commands, registry tokens |
| local prepared tree → PUBLIC GitHub repo | A push can leak private build_notes/ or .gsd/ into a public repo that must stay public pending minifilter-altitude approval. | private planning artifacts |
| operator intent → release execution | Without a gated runbook the operator may push out-of-order or skip the dry-run. | release command sequence |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-97-01 | Tampering | version + path-dep pins across 11 manifests | mitigate | Six version-family crates at `version = "0.66.0"`; zero `0.62.2`/loose `0.62` pins across all tracked Cargo.toml/pyproject.toml/package.json (incl. siblings). `crates/nono/Cargo.toml:3` et al. | closed |
| T-97-02 | Tampering | Cargo.lock regeneration | mitigate | nono-family lock entries all 0.66.0 (`Cargo.lock:2371-2516`); no external crate drift. | closed |
| T-97-03 | Repudiation | leapfrog target choice | accept | 0.66.0 documented as first SemVer > upstream nolabs-ai/nono v0.65.1 (97-01-SUMMARY:28). | closed |
| T-97-04 | Tampering | MSI payload signing order | mitigate | "Sign Windows binaries (pre-package)" (`release.yml:167`) precedes "Package (Windows)" (`:186`); admin-extract "Verify MSI payload signatures" gate (`:281`) fails closed (`exit 1` at 296/307/311-313). | closed |
| T-97-05 | Spoofing | MSI wrapper vs payload signature | mitigate | Both wrapper "Verify Authenticode signatures" (`release.yml:259`) and payload admin-extract gate (`:281`) retained. | closed |
| T-97-06 | Denial of Service | release.yml startup validation | mitigate | No `uses: ./.github/workflows/image-build.yml` job (only an explanatory NOTE warning against re-adding it, `:474-481`); all 5 build legs present; valid YAML. | closed |
| T-97-07 | Tampering | homebrew tarball source URL | mitigate | download-url = `OscarMackJr/nono` (`release.yml:532`); zero `always-further/nono`, zero `nolabs-ai/nono` in release.yml or build-windows-msi.ps1. | closed |
| T-97-08 | Tampering | accidental live publish during dry-run | mitigate | Only `--dry-run` / twine check / `npm publish --dry-run` in `release-dry-run.ps1` (77/185); zero `twine upload`, no token-bearing publish path. | closed |
| T-97-09 | Information disclosure | registry tokens in logs | mitigate | No `--token`/credential reference anywhere in `release-dry-run.ps1` (246 lines); SAFETY INVARIANTS block (`:24-28`). | closed |
| T-97-10 | Repudiation | silent skip of a binding leg | mitigate | Absent toolchains recorded as `SKIP [SKIP_HOST_UNAVAILABLE]` with verbatim signature (maturin 110-113, twine 138-141, npm 178-180); crates.io leg always-runs, hard FAIL → `exit 1` (242-244). | closed |
| T-97-11 | Information disclosure | build_notes/ or .gsd/ leaked into PUBLIC push | mitigate | `release-readiness.ps1:175-206` checks staged (`git diff --cached`) and tracked (`git ls-files`) → FAIL verdict; RELEASE-RUNBOOK.md embeds verbatim pre-push checklist (`:13-28`). | closed |
| T-97-12 | Tampering | operator skips dry-run / readiness gate | mitigate | RELEASE-RUNBOOK.md "Mandatory Pre-Push Gates" (`:32-89`): dry-run Step 1 + readiness Step 2 required-green before push Step 3. | closed |
| T-97-13 | Elevation of privilege | accidental live publish from runbook | mitigate | RELEASE-RUNBOOK.md "Status: PREPARE-ONLY" (`:3`), documentation-only; "None of the push commands above were executed by this milestone" (`:220-232`). | closed |
| T-97-01-SC | Tampering | npm/pip/cargo installs | accept | Version-string changes only; `cargo update --workspace` rewrote exactly 6 lock entries, no new package (97-01-SUMMARY `tech_stack.added: []`). | closed |
| T-97-02-SC | Tampering | npm/pip/cargo installs | accept | URL/registry-key string edits only (97-02-SUMMARY `tech_stack.added: []`). | closed |
| T-97-03-SC | Tampering | npm/pip/cargo installs | accept | Dry-run validation script only, builds from existing lockfile/sdist (97-03-SUMMARY `tech_stack.added: []`). | closed |
| T-97-04-SC | Tampering | npm/pip/cargo installs | accept | PowerShell gate + markdown doc only (97-04-SUMMARY `tech_stack.added: []`). | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-97-01 | T-97-03 | 0.66.0 is the first SemVer strictly greater than upstream nolabs-ai/nono v0.65.1; the fork must tag past upstream's highest. Rationale auditable in 97-01-SUMMARY. | gsd-security-auditor | 2026-06-26 |
| AR-97-02 | T-97-01-SC / T-97-02-SC / T-97-03-SC / T-97-04-SC | No package-manager install in any Phase 97 plan — version strings, URL edits, and validation scripts only; the existing lockfile is reused, so no new package enters the tree. | gsd-security-auditor | 2026-06-26 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-26 | 17 | 17 | 0 | gsd-security-auditor (Phase 97 secure-phase, State B create) |

### Auditor notes (informational, non-blocking, out of declared threat scope)

- `nono-ts/Cargo.toml:8-9` still carries `repository`/`homepage = https://github.com/always-further/nono` metadata — a sibling-repo cosmetic URL, not within T-97-07's declared scope (release.yml + build-windows-msi.ps1) and not a security surface. Operator may optionally clean before the npm publish.
- Sibling-repo version edits (nono-py, nono-ts) are on disk at 0.66.0 but, per 97-01-SUMMARY, are uncommitted in their separate repos pending operator action. This is the planned hand-off, not a gap.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-26
