---
quick_id: 260522-rct
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - crates/nono-cli/Cargo.toml
  - scripts/build-windows-msi.ps1
  - dist/windows/nono-machine.wxs
  - dist/windows/nono-user.wxs
autonomous: true
requirements: [QUICK-260522-rct]

must_haves:
  truths:
    - "Cargo workspace authors array includes Oscar Mack Jr with email"
    - "deb maintainer and copyright strings credit both authors"
    - "MSI build script default Manufacturer credits both authors"
    - "Shipped .wxs reference snapshots credit both authors in Package/SummaryInformation/ARPCONTACT"
    - "Inline comments and XML structure in .wxs files are preserved exactly"
  artifacts:
    - path: "Cargo.toml"
      provides: "Workspace authors metadata"
      contains: 'authors = ["Luke Hinds", "Oscar Mack Jr <oscar.mack.jr@gmail.com>"]'
    - path: "crates/nono-cli/Cargo.toml"
      provides: ".deb package metadata"
      contains: 'maintainer = "Luke Hinds and Oscar Mack Jr"'
    - path: "scripts/build-windows-msi.ps1"
      provides: "MSI build script default Manufacturer"
      contains: '[string]$Manufacturer = "Luke Hinds and Oscar Mack Jr"'
    - path: "dist/windows/nono-machine.wxs"
      provides: "Per-machine MSI reference snapshot"
      contains: 'Manufacturer="Luke Hinds and Oscar Mack Jr"'
    - path: "dist/windows/nono-user.wxs"
      provides: "Per-user MSI reference snapshot"
      contains: 'Manufacturer="Luke Hinds and Oscar Mack Jr"'
  key_links:
    - from: "crates/nono-cli/Cargo.toml [package]"
      to: "Cargo.toml [workspace.package].authors"
      via: "authors.workspace = true (already wired; new co-author propagates automatically)"
      pattern: "authors.workspace = true"
    - from: "scripts/build-windows-msi.ps1 -Manufacturer param"
      to: "Generated WXS Manufacturer attributes"
      via: "Script regenerates wxs on run; reference snapshots are hand-maintained and MUST stay in sync"
      pattern: "Manufacturer="
---

<objective>
Add Oscar Mack Jr as co-author in all shipped metadata surfaces (Cargo workspace authors, .deb maintainer/copyright, MSI Manufacturer default, and the committed .wxs reference snapshots) so credit appears in shipped artifacts going forward.

Purpose: User requested co-author credit in shipped metadata. Decisions on author string format and which files to edit were locked by the orchestrator via AskUserQuestion.

Output: Five files edited via mechanical string replacement, single atomic commit. No rebuild performed (user will rebuild MSIs separately).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@.planning/STATE.md

## Locked decisions from orchestrator (do NOT revisit)

1. **Author string format**:
   - Workspace `authors` array: `["Luke Hinds", "Oscar Mack Jr <oscar.mack.jr@gmail.com>"]`
     (Cargo convention: per-author entries with optional `<email>`. Oscar's email is included; Luke's existing entry has no email — preserve as-is.)
   - All other surfaces (.deb maintainer, .deb copyright body, MSI Manufacturer, .wxs Manufacturer/ARPCONTACT): `"Luke Hinds and Oscar Mack Jr"` (single joint string, no emails).
   - .deb copyright full value: `"2026, Luke Hinds and Oscar Mack Jr"` (year + joint string).

2. **.wxs files MUST be edited in place** (not regenerated via `scripts/build-windows-msi.ps1`).
   - The script strips inline comments — earlier today this regression was caught and reverted (commit `eba99cdd`).
   - Use Edit/string-replace ONLY. Preserve all `<!-- ... -->` comments (notably the 260522-c9c WFP service/driver/event-log component comments in `nono-machine.wxs`) and current XML structure exactly.

3. **Scope boundary**: No new Windows PE resource embedding (no winres/build.rs additions). The user's "should say so" wording refers to existing metadata surfaces only.

4. **No rebuild as part of this task**. Source/text edits + atomic commit only. User will rebuild MSIs separately.

## Verified current state (Read-confirmed line numbers and exact strings)

`Cargo.toml` line 14:
```
authors = ["Luke Hinds"]
```

`crates/nono-cli/Cargo.toml` lines 16-17 (inside `[package.metadata.deb]`):
```
maintainer = "Luke Hinds"
copyright = "2026, Luke Hinds"
```

`scripts/build-windows-msi.ps1` line 20:
```
[string]$Manufacturer = "Luke Hinds",
```

`dist/windows/nono-machine.wxs` lines 5, 11, 16 (three occurrences):
```
      Manufacturer="Luke Hinds"
        Manufacturer="Luke Hinds" />
    <Property Id="ARPCONTACT" Value="Luke Hinds" />
```

`dist/windows/nono-user.wxs` lines 5, 11, 16 — identical three-occurrence pattern.

All other files outside these five are out of scope.
</context>

<tasks>

<task type="auto">
  <name>Task 1: Update Cargo, deb, and PowerShell metadata strings</name>
  <files>Cargo.toml, crates/nono-cli/Cargo.toml, scripts/build-windows-msi.ps1</files>
  <action>
Perform exactly these mechanical Edit calls (per locked orchestrator decisions; do not re-derive strings).

**Edit 1 — `Cargo.toml`:**
- old_string: `authors = ["Luke Hinds"]`
- new_string: `authors = ["Luke Hinds", "Oscar Mack Jr <oscar.mack.jr@gmail.com>"]`

**Edit 2 — `crates/nono-cli/Cargo.toml`:**
- old_string: `maintainer = "Luke Hinds"`
- new_string: `maintainer = "Luke Hinds and Oscar Mack Jr"`

**Edit 3 — `crates/nono-cli/Cargo.toml`:**
- old_string: `copyright = "2026, Luke Hinds"`
- new_string: `copyright = "2026, Luke Hinds and Oscar Mack Jr"`

**Edit 4 — `scripts/build-windows-msi.ps1`:**
- old_string: `[string]$Manufacturer = "Luke Hinds",`
- new_string: `[string]$Manufacturer = "Luke Hinds and Oscar Mack Jr",`
  (Keep trailing comma — line 20 is mid-param-list.)

Do NOT touch any other file in this task. Do NOT rebuild.
  </action>
  <verify>
    <automated>grep -n 'Oscar Mack Jr' Cargo.toml crates/nono-cli/Cargo.toml scripts/build-windows-msi.ps1 | grep -v '^#' | wc -l | grep -q '^[[:space:]]*4$'</automated>
    Expected: exactly 4 matches (1 in Cargo.toml authors, 2 in nono-cli/Cargo.toml [maintainer + copyright], 1 in build-windows-msi.ps1 default param).

    Sanity follow-up (manual one-liner): `grep -n 'Luke Hinds' Cargo.toml crates/nono-cli/Cargo.toml scripts/build-windows-msi.ps1` — every remaining match MUST be on the same line as "Oscar Mack Jr" (i.e., no orphan solo-Luke strings left behind in these three files).
  </verify>
  <done>
- `Cargo.toml` workspace authors is the two-entry array with Oscar's email.
- `crates/nono-cli/Cargo.toml` `[package.metadata.deb]` maintainer and copyright both credit both authors (copyright preserves the `2026, ` year prefix).
- `scripts/build-windows-msi.ps1` line 20 default Manufacturer credits both authors and retains its trailing comma.
- No other file modified.
- `cargo metadata --format-version 1 --no-deps` (optional sanity, not required for task closure) parses without complaint — TOML is syntactically valid.
  </done>
</task>

<task type="auto">
  <name>Task 2: Update WXS reference snapshots in place (preserve comments and structure)</name>
  <files>dist/windows/nono-machine.wxs, dist/windows/nono-user.wxs</files>
  <action>
Edit BOTH .wxs files in place using mechanical string replacement. DO NOT regenerate via `scripts/build-windows-msi.ps1` — the script strips inline comments (commit `eba99cdd` reverted exactly that regression earlier today). The 260522-c9c WFP service/driver/event-log comments in `nono-machine.wxs` MUST survive intact.

For EACH of `dist/windows/nono-machine.wxs` AND `dist/windows/nono-user.wxs`, perform these three Edit calls (the three Luke-Hinds occurrences are at lines 5, 11, and 16 in both files):

**Edit A — Package Manufacturer attribute (line 5, indented inside `<Package` element):**
- old_string: `      Manufacturer="Luke Hinds"`
- new_string: `      Manufacturer="Luke Hinds and Oscar Mack Jr"`

**Edit B — SummaryInformation Manufacturer attribute (line 11, deeper indent, self-closing tag follows on same line):**
- old_string: `        Manufacturer="Luke Hinds" />`
- new_string: `        Manufacturer="Luke Hinds and Oscar Mack Jr" />`

**Edit C — ARPCONTACT Property (line 16, full Property element):**
- old_string: `    <Property Id="ARPCONTACT" Value="Luke Hinds" />`
- new_string: `    <Property Id="ARPCONTACT" Value="Luke Hinds and Oscar Mack Jr" />`

Indentation in old_string MUST match exactly (Edit A uses 6 spaces, Edit B uses 8 spaces, Edit C uses 4 spaces — verified via Read). All other lines, comments, and structure remain untouched.

Apply all three edits to `nono-machine.wxs` first, then all three to `nono-user.wxs`.
  </action>
  <verify>
    <automated>grep -c 'Luke Hinds and Oscar Mack Jr' dist/windows/nono-machine.wxs dist/windows/nono-user.wxs | grep -v ':0$' | grep -c ':3$' | grep -q '^2$'</automated>
    Expected: each .wxs file contains exactly 3 occurrences of the joint string (one per replaced location), and grep counts both files at `:3`.

    Follow-up structural checks (run as second verify step):
    - `grep -c 'Manufacturer="Luke Hinds"' dist/windows/nono-machine.wxs dist/windows/nono-user.wxs` MUST return `:0` for both files (no solo-Luke Manufacturer strings remain).
    - `grep -c '<!--' dist/windows/nono-machine.wxs` MUST be unchanged from the pre-edit count (orchestrator note: 260522-c9c inline comments around WFP service/driver/event-log components must survive — quick sanity is "comment count today equals comment count before edits"; if the executor wants belt-and-suspenders, run `git diff --stat dist/windows/` and confirm only line-level changes on the three target lines per file, no block deletions).
  </verify>
  <done>
- Both `dist/windows/nono-machine.wxs` and `dist/windows/nono-user.wxs` contain exactly 3 occurrences of `Luke Hinds and Oscar Mack Jr` (Package Manufacturer, SummaryInformation Manufacturer, ARPCONTACT Value).
- Zero remaining occurrences of the solo string `Manufacturer="Luke Hinds"` or `ARPCONTACT" Value="Luke Hinds"` in either file.
- All inline `<!-- ... -->` comments (especially 260522-c9c WFP comments in nono-machine.wxs) survive intact.
- `git diff dist/windows/` shows ONLY the six target lines changed (3 per file) — no structural or comment churn.
- Script `scripts/build-windows-msi.ps1` was NOT executed.
  </done>
</task>

</tasks>

<verification>
After both tasks complete, run as a phase-level sanity sweep:

```bash
# Joint string appears in exactly the 8 expected slots across all five files:
#   1 Cargo.toml + 2 crates/nono-cli/Cargo.toml + 1 scripts/build-windows-msi.ps1 + 3 nono-machine.wxs + 3 nono-user.wxs = 10
grep -rn 'Luke Hinds and Oscar Mack Jr' Cargo.toml crates/nono-cli/Cargo.toml scripts/build-windows-msi.ps1 dist/windows/nono-machine.wxs dist/windows/nono-user.wxs | wc -l
# Expected: 10

# No remaining solo-Luke metadata strings in the five touched files (note: Cargo.toml authors line still contains "Luke Hinds" as the first array entry — that is correct; the check below is scoped to standalone metadata strings):
grep -n '"Luke Hinds"' Cargo.toml dist/windows/nono-machine.wxs dist/windows/nono-user.wxs
# Expected: only the Cargo.toml authors line `["Luke Hinds", "Oscar Mack Jr <oscar.mack.jr@gmail.com>"]` — zero matches in either .wxs.

# Toml/PowerShell/XML syntactic sanity (best-effort; not strictly required):
cargo metadata --format-version 1 --no-deps >/dev/null 2>&1 && echo "Cargo metadata parses"
```

No clippy/fmt/test runs required — this task only touches metadata strings and a build script default. Per CLAUDE.md "After every session, run these commands" — `make ci` is NOT required here because no Rust source was modified; the workspace `authors` change is metadata-only and does not affect compilation or lints.

(If the user requests it post-commit, `make build-cli` will succeed unchanged — authors metadata does not gate compilation.)
</verification>

<success_criteria>
1. All five files contain the joint string in their respective slots (10 total occurrences across the five files).
2. `.wxs` inline comments preserved (no regression of the `eba99cdd` revert).
3. No file outside the five listed in `files_modified` is modified.
4. Single atomic commit covers all five files with a DCO sign-off line.
5. No rebuild performed (deferred to user per locked decision #4).
</success_criteria>

<next_steps_note>
Not part of this plan; informational only:

When the user wants the new Manufacturer string in shipped MSIs, they will need to:
1. Rebuild `nono-cli` (workspace authors propagates automatically via `authors.workspace = true`).
2. Re-run `scripts/build-windows-msi.ps1` for both `-Scope machine` and `-Scope user` to regenerate signed MSIs with the new default Manufacturer.
3. Rebuild .deb via `cargo deb -p nono-cli` to pick up the new maintainer/copyright.

These rebuild steps are intentionally OUT OF SCOPE for this quick task per locked decision #4.
</next_steps_note>

<output>
After completion, create `.planning/quick/260522-rct-add-oscar-mack-jr-as-co-author-in-shippe/260522-rct-SUMMARY.md` capturing:
- The 10 string replacements performed (5 files, 1+2+1+3+3 slots)
- Confirmation that .wxs inline comments survived (e.g., `grep -c '<!--' dist/windows/nono-machine.wxs` before/after)
- The single commit SHA + sign-off
- Reminder that MSI/.deb rebuild is deferred to a follow-up at user's request
</output>
