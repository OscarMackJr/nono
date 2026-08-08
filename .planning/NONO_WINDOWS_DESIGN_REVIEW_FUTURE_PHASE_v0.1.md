# nono Windows-Native: Pre-1.0 Design Review And A Future Phase

Version: 0.1
Status: Design input — a candidate future phase, not a 1.0 blocker list
Author's context: this reviews the Windows-native fork (`OscarMackJr/nono`) against the upstream Unix design (`nolabs-ai/nono`) and the state of the art in Windows application isolation. It is deliberately independent of the current nono feature specifications and the existing `windows-feature-gap-matrix` — those track parity item by item; this asks the higher-order question of whether the *model* has gaps the parity list cannot surface.
Primary post-read action: decide which of §5's candidate phase items are 1.0-adjacent versus genuinely deferrable, and whether §3's containment-model question deserves an ADR before 1.0 ships.

---

## 1. What The Fork Actually Changed (so the gaps are legible)

Upstream nono contains a process in a tool sandbox using Unix primitives: **Landlock** (filesystem access) and **seccomp**-style exec gating on Linux, **seatbelt** (`sandbox-exec` profiles) on macOS. These are kernel-enforced, per-process, and — critically — *deny-by-construction*: the process cannot reach what the policy did not grant, because the kernel refuses the syscall.

The Windows-native fork could not port those primitives because Windows has no equivalent single mechanism. Instead it assembles a composite:

| Concern | Upstream (Unix) | Fork (Windows-native) |
|---|---|---|
| Filesystem confinement | Landlock / seatbelt, kernel-enforced per process | AppContainer + **low-integrity** process + restricted token + DACL guards, plus a runtime redirection scheme for tool caches; a **minifilter** driver spike (Phase 63/64) for the parts integrity levels cannot cover |
| Exec / child-process gating | seccomp-style exec gate | Restricted token with `WRITE_RESTRICTED`, AppContainer SID per session, command-aware launch preflight |
| Network egress | proxy + **TLS interception** (`tls_intercept/`: CA, cert cache, h2 probe) | **WFP** driver for process-scoped egress + proxy doing **CONNECT host-filtering** with credential injection — *TLS interception dropped* |
| Kernel↔user IPC | not required (syscall filters are in-kernel) | ring-buffer + worker-thread pattern, finite-timeout `FltSendMessage`, documented BSOD-avoidance contract |

This is a genuinely thoughtful adaptation — the minifilter design gate (BSOD threat register, altitude discipline, fail-direction decision) is better engineering than most Windows security tooling ships with. The gaps below are not criticisms of that work; they are the consequences of the composite model that a per-item parity matrix does not make visible.

---

## 2. The Structural Observation

Upstream's guarantee is **one mechanism, deny-by-construction**. The fork's guarantee is **several mechanisms, deny-by-composition** — filesystem confinement is the *intersection* of integrity level, AppContainer profile, DACL, and (eventually) the minifilter; egress is WFP *and* the proxy.

Composition is not weaker in principle. But it has three properties a single kernel gate does not, and each is a place a gap can hide:

1. **A composite fails at its seams.** The whole is only as strong as the interaction between layers, and the interactions are where the assumptions live.
2. **A composite has ambiguous fail-direction.** When one layer is unavailable — driver not loaded, integrity label not applied, WFP filter not installed — does the system fail closed, fail open, or continue in a reduced mode that *looks* enforced? Upstream cannot ask this question; the fork must answer it for every layer.
3. **A composite is harder to attest.** "The kernel denied the syscall" is a provable statement. "The intersection of four Windows mechanisms denied this access" is a claim that requires testing each mechanism *and* their conjunction.

Everything in §3 and §4 is an instance of one of those three.

---

## 3. Functional Gaps Worth Considering Before 1.0

### G-WIN-1 · Fail-direction is decided per-layer but not as a system (seam risk)

The minifilter spike explicitly defers its production fail-direction (`T-63-02`: spike fails open to avoid locking the test VM; production is "a separate decision"). That is correct for a spike. But the same question exists for *every* layer — what happens when the WFP filter is absent, when the token could not be restricted, when the integrity label did not apply — and there is no single document stating the system-level answer.

The failure mode this creates is the worst kind: **a nono that reports "enforcing" while one layer is silently inert.** The fork's own `nono_state.md` describes exactly this class of bug being closed once (the "honesty gap" where `Sandbox::apply()` returned `UnsupportedPlatform` while the CLI enforced via WFP) — which is evidence the seam is real, not hypothetical.

*Consider before 1.0:* a single fail-direction contract covering all layers, and a startup self-attestation that refuses to report "enforcing" unless each expected layer confirms it is active. The principle is the program-wide one: **observation of enforcement must be as trustworthy as the enforcement.**

### G-WIN-2 · Egress content-blindness (dropped TLS interception)

Upstream can inspect *what* leaves over TLS. The fork tunnels TLS transparently (`connect.rs`: "relaying bytes bidirectionally, transparent TLS tunnel") and enforces at the **host** level — allowlist plus credential injection. That is a defensible and in some ways cleaner choice (no CA to distribute, no interception liability), and for the Fiskroad use case — force traffic through the gateway, block direct providers — host-level is sufficient.

But it is a real capability gap against upstream, and it bounds what nono can ever claim:

- nono can enforce *where* a request goes and *what credential* it carries. It cannot see *what the request contains*.
- Data-loss patterns that upstream could catch by inspection — a prompt exfiltrating a secret to an *allowlisted* host — are invisible to a host-filtering proxy.

*Consider before 1.0:* not necessarily building interception, but **stating the boundary explicitly** in the security model — "nono governs destination and credential, not payload" — so no downstream consumer (Fiskroad included) over-claims content control. If payload inspection is ever needed, it is a large, CA-distribution-shaped project and should be scoped as its own phase, not retrofitted.

### G-WIN-3 · The minifilter is the load-bearing gap, and it is a spike

Integrity levels and DACLs cannot express "this process may read `C:\project\src` but not `C:\project\.env`" — same directory, different files, no integrity distinction. Only the minifilter can. Per the design doc it is currently a **compile-ready skeleton with an empty callback array** (Phase 63), with real logic deferred to Phase 64.

Until Phase 64 lands, the fork's filesystem confinement has a category of policy it cannot enforce that upstream enforces trivially with Landlock. This is the single largest functional delta and it is already known — the point here is that **1.0's filesystem guarantee should be described in terms of what the shipped mechanism covers**, not what the design intends. A per-file read policy that "will be enforced in Phase 64" is not a 1.0 guarantee.

### G-WIN-4 · Driver deployment is a distribution and trust surface upstream never had

Landlock and seatbelt are in the OS. The fork ships **two kernel drivers** (WFP, and the minifilter once real). That introduces, on every managed workstation:

- **Signing.** Production kernel drivers need an EV certificate and, for the minifilter, a Microsoft-assigned altitude (`T-63-04` names the range discipline; the assignment request to `fsfcomm@microsoft.com` is a real, slow, external dependency).
- **Deployment.** Driver install requires admin; fleet rollout via Intune is a different mechanism than a user-mode agent, with different failure modes.
- **Kernel blast radius.** A user-mode sandbox that crashes kills a process. A minifilter that crashes BSODs the workstation. The design doc's BSOD register is exactly right; the *operational* consequence — a bad driver update taking out a fleet — is a deployment-phase risk that deserves a rollback story before wide rollout.

*Consider before 1.0:* a driver lifecycle document — signing chain, altitude status, staged-rollout plan, and kernel-crash rollback — as a first-class deliverable alongside the code. This is Fiskroad-relevant too: nono's fleet deployment is on the program's critical path for the endpoint-coverage claim.

### G-WIN-5 · Attestation gap — proving enforcement to another system

Fiskroad's whole model is that controls produce evidence. nono's natural output is a detection event (blocked egress). But the composite model raises a question upstream never had to answer: **can nono prove, at a point in time, that a given process was actually contained by all expected layers?**

Today the honest answer is partial — you can observe blocks, but "this process ran under a restricted token AND a low-integrity label AND an AppContainer profile AND WFP coverage" is a conjunction that is asserted at launch, not continuously attested. For an endpoint control feeding a governance program, the difference between "we configured enforcement" and "we can show enforcement held" is the difference the whole program is built on.

*Consider before 1.0:* a per-session enforcement receipt — which layers were confirmed active for this contained process — emitted in the same content-free shape as detection events. This is the nono-side analogue of the Answer Trace Envelope: not what the process did, but under what containment it did it.

---

## 4. Techniques From The State Of The Art Worth Evaluating

Independent of upstream, these are Windows isolation techniques a pre-1.0 review should at least rule in or out in writing:

- **Windows Sandbox / Windows Server Containers (HCS).** Hardware-assisted or container isolation is a stronger boundary than integrity levels for the highest-sensitivity tool runs. Heavyweight and not per-file, but worth a recorded decision on whether a "strong isolation" tier belongs in the model for untrusted tools.
- **Restricted User Mode / Protected Process Light (PPL).** Not for sandboxing the target, but for protecting *nono's own supervisor* from the process it contains — a contained process that can tamper with its supervisor is a containment escape. Worth confirming the supervisor's own protection posture.
- **Windows Filtering Platform ALE layers beyond connect-time.** WFP can filter at more than the connection layer; if egress policy ever needs to react to more than destination, the mechanism is already present and worth knowing the limits of.
- **Event Tracing for Windows (ETW) as an enforcement-adjacent signal.** The fork already uses ETW for telemetry (`telemetry/windows.rs`). ETW-based detection of sandbox-escape attempts (process injection, token manipulation) is a defense-in-depth layer that complements rather than replaces the primary controls.
- **AppContainer capability profiles as positive grants.** The fork uses AppContainer for isolation; AppContainer's *capability* model can also express positive network/device grants declaratively, which may simplify some policy the proxy currently carries.
- **WDAC (Windows Defender Application Control) for exec gating.** A policy-driven, OS-native exec-allowlist that could complement the restricted-token launch preflight, and is fleet-manageable via the same MDM path as the drivers.

None of these is a 1.0 requirement. The value of the review is a written "considered, deferred, because —" for each, so the 1.0 security model is a set of decisions rather than a set of omissions.

---

## 5. A Candidate Future Phase

Framed as a phase rather than a backlog, sequenced by dependency:

**Phase N — "Composite Integrity": making the multi-layer model as trustworthy as a single kernel gate.**

1. **Fail-direction contract (G-WIN-1).** One document, every layer, with the system-level answer to "what if this layer is absent." Startup self-attestation refuses to claim enforcement it cannot confirm. *Smallest, highest-leverage, arguably 1.0-adjacent.*
2. **Enforcement receipts (G-WIN-5).** Per-session, content-free attestation of which layers held. The nono analogue of the trace envelope. *Directly serves Fiskroad's evidence model.*
3. **Minifilter Phase 64 to production (G-WIN-3).** Per-file policy, the largest functional parity gain against upstream. Gated behind its own BSOD-avoidance contract, which already exists.
4. **Driver lifecycle and fleet story (G-WIN-4).** Signing, altitude, staged rollout, kernel-crash rollback. Blocking for wide deployment even if not for code-complete.
5. **Security-model boundary statement (G-WIN-2).** Write down what nono governs (destination, credential, containment) and what it does not (payload). Cheap, and it prevents over-claiming downstream.
6. **State-of-the-art decision log (§4).** Each technique ruled in or out, in writing.

Items 1, 2 and 5 are documentation and modest code, and each raises the *trustworthiness* of what already exists rather than adding surface. Items 3 and 4 are the substantial engineering, and both are already partly scoped in the fork's own phase plan. Item 6 is an afternoon that prevents a category of future "why didn't we consider X" conversation.

---

## 6. The One-Sentence Version

The fork successfully rebuilt a single-mechanism Unix sandbox as a composite Windows one; the work still owed before 1.0 is less about adding layers and more about making the composite **prove** it is enforcing — a fail-direction contract, an enforcement receipt, and an honest boundary statement — which is also exactly the shape the Fiskroad program needs from every control it depends on.
