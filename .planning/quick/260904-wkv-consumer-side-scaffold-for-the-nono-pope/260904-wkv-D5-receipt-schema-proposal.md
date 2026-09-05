# PROPOSAL FOR JOINT REVIEW — NOT DECIDED

This document is nono's proposal for contract §7 D-5 ("Receipt schema residency"). It is **not
decided** — the contract's own header requires joint review by the nono maintainers, the popeye
maintainers, and the program lead before any option here is adopted. Nothing in this repository
reads or validates against the schema file this proposal ships alongside it, and nothing in
`crates/nono/src/session_credential.rs` imports or depends on it.

## The Contract's D-5 Wording

Per `fiskroad/contracts/NONO_POPEYE_SESSION_CREDENTIAL_CONTRACT_v0.1.md` §7: "The receipt fields
this contract requires (`session_id`, `key_refs`, window) live in the endpoint-detection schema
family — same file as `endpoint-detection/0.1` or a sibling `endpoint-receipt/0.1`. nono's
proposal, reviewed jointly." (Paraphrased above the quoted defining clause; the contract states
the decision is "nono's proposal" to make, which is what this document is.)

## Option A: Extend endpoint-detection/0.1

Add the session-credential receipt fields (`session_id`, `key_refs`, session window,
layer-attestation reference) directly into the existing `endpoint-detection/0.1` schema family,
as additional properties alongside whatever detection-event fields that schema already carries.

A repo-wide search of this repository (`Grep` for `endpoint-detection` under `crates/`) found
**no such schema under `crates/` in this repo** — the string appears only in two prose
mentions inside `.planning/NONO_AGENT_RUNTIME_CHARTER_v0.1.md` (lines 38 and 75), one of which
lists `endpoint-detection/0.1` detection emission as a charter §7 entry criterion that is **not
yet landed**. This option is therefore **not locally exercisable today** in this repository —
there is nothing under `crates/` to extend. This is a factual, repo-scoped finding; it makes no
claim about whether `endpoint-detection/0.1` exists in popeye's repository or in the shared
`fiskroad` vocabulary specification.

## Option B (preferred): Sibling endpoint-receipt/0.1

Define a new, sibling schema — `endpoint-receipt/0.1` — dedicated to session-credential-join
receipts, distinct from whatever `endpoint-detection/0.1` eventually becomes. **Rationale:**
detections are instantaneous events (a single blocked-attempt, single-timestamp fact), while
credential-join receipts are session-window claims that can *grow* over a session's life via
renewal (contract §4.2's "new key, same session, new `key_ref`" chain) — a fundamentally
different write lifetime than a one-shot detection record. Coupling the two into one schema
risks the exact "startup-only claim quietly stretched" drift that D-20 already warns against for
`EnforcementReceipt` in `crates/nono/src/receipt.rs` (a receipt designed and tested as a
single, immutable, startup-only artifact would face the same pressure to grow a mid-session
field the moment a session-window `end` or a renewal chain needed representing) — plus the
concrete, present-day fact that Option A is not locally expressible in this repository at all,
since `endpoint-detection/0.1` does not exist here to extend.

The schema for the preferred option (`260904-wkv-endpoint-receipt-0.1.schema.json`, alongside
this file) covers exactly: `session_id`, `key_refs[]` (the renewal chain, one entry per issued
key under that session), a session window (`session_window.start` / `session_window.end`, `end`
nullable for an in-progress session), and `layer_attestations_ref` — a **reference**, never
inlined content, to the layer-attestation census `EnforcementReceipt` already owns. This last
point mirrors the existing "never duplicate a fact that already has a canonical home" reasoning
`SessionOutcome`'s doc comment states in `crates/nono/src/receipt.rs` for why it does not carry
a `Refused { layer: LayerId }` payload: which layer refused is discoverable from the receipt's
own `layers` census, so duplicating it into a second type would create a second, potentially
inconsistent source of truth. The same discipline applies here — a session-credential receipt
should point at the layer-attestation census that already exists, never re-state it.

## Non-Adoption Statement

This schema file is a `.planning/` artifact only. Nothing in `crates/` reads, loads, validates
against, or otherwise references `260904-wkv-endpoint-receipt-0.1.schema.json`. The interface
stub built in `crates/nono/src/session_credential.rs` (Task 2/3 of this quick task's plan) does
not import it, does not depend on it, and does not construct any value shaped like it. D-5
remains an open joint-review decision; this document records nono's *proposed* answer for that
review to accept, modify, or reject.
