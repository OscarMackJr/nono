# scripts/gates/telemetry-event-emit.ps1
#
# Phase 84 Plan 04 - telemetry-event-emit gate (SC-1 Application-log + SC-3 no raw paths + SC-5 ETW)
#
# CONTRACT (cloned from scripts/gates/egress-policy-deny.ps1 — structural twin):
# this gate exports exactly two functions, dot-sourced by scripts/verify-dark.ps1.
# The gate RETURNS its verdict object — it MUST NOT call exit and MUST NOT call
# Persist-Verdict. Only the runner owns exit-code mapping (PASS=0 / FAIL=2 /
# SKIP_HOST_UNAVAILABLE=3 / harness-internal=4) and the persist-before-emit (WR-04).
#
#   Test-Precondition -> $null (preconditions met, run Invoke-Gate)
#                      | "reason string" (SKIP_HOST_UNAVAILABLE - exit 3, Invoke-Gate never runs)
#   Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                        verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
#                        a `throw` here = harness-internal error (exit 4), never a silent PASS
#
# WHAT THIS PROVES (satisfies TELEM-01, TELEM-02, TELEM-03, SC-1, SC-3, SC-5):
#   SC-1 (Application-log entry with correct EventID + named JSON fields):
#     Seeds a fresh path-deny event by running a nono confinement command that
#     hits a denied path. Queries the Windows Application Event Log under the
#     'nono' source for EventID in {10001..10005} in the last 5 minutes. Parses
#     the event body as JSON and asserts the named fields required for the event's
#     type. Universal: EventType, AgentPid, SessionId, ChainHead. Conditional (serde
#     skip_serializing_if): PathHash for path-deny (10001), Host for network-deny
#     (10002). The gate triggers a path-deny, so it asserts the universal set +
#     PathHash (NOT Host, which is legitimately absent for path-deny) — CR-01 fix.
#     NOTE: EventID 10003 (LabelViolation) is RESERVED-but-unemitted in Phase 84
#     (Option B decision from 84-03-SUMMARY). Only 10001/10002/10004/10005 are
#     emittable by the three wired sources. The gate asserts Id -ge 10001 -and
#     Id -le 10005 (range query), accepting whichever of the wired IDs appears.
#
#   SC-3 (no raw path strings in event body):
#     Asserts that the event body (JSON) does NOT contain raw Windows user path
#     strings (e.g. C:\Users\alice\secret.txt). The PathHash field carries a
#     hex-encoded SHA-256 hash of the salted path (D-08), never the raw path.
#     Pattern checked: 'C:\\Users\\' and broad Windows path pattern.
#     (Pitfall 11 prevention — secret/path leakage into Event Log.)
#
#   SC-5 (ETW provider detectable via logman):
#     Runs `logman query providers` and asserts 'nono' appears in the output.
#     tracing-etw LayerBuilder::new("nono") registers the ETW provider on first
#     use in init_tracing(). If nono has been run and produced a security event,
#     the provider should appear in the logman provider registry.
#
# WHY SKIP (not FAIL) when prerequisites are absent:
#   SC-1 requires admin elevation to query the Application Event Log via Get-WinEvent.
#   Additionally, if neither a recent nono event exists NOR nono is on PATH to seed one,
#   the gate cannot produce a verdict and must SKIP cleanly.
#   SC-5 requires logman (Windows built-in) and admin for full provider enumeration.
#   On a dev host without these, the gate SKIPs cleanly — never emits a false PASS.
#
# INVOCATION RULE (MEMORY durable):
#   pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit
#   NEVER: pwsh -Command "<bare path>" (swallows exit N -> 1)

# ---------------------------------------------------------------------------
# Gate configuration
# ---------------------------------------------------------------------------

$script:EventLogSource      = 'nono'
$script:EventIdMin          = 10001
$script:EventIdMax          = 10005
$script:EventIdPathDeny     = 10001
$script:EventIdNetworkDeny  = 10002
# EventID 10003 (LabelViolation) is RESERVED-but-unemitted in Phase 84 (Option B — 84-03-SUMMARY.md)
$script:EventIdHookFail     = 10004
$script:EventIdTelemetryDeg = 10005
$script:TelemetryGateName   = 'telemetry-event-emit'
# SC-1 JSON fields that must appear in the event body (D-11 named EventData minimum).
# Universal fields are present on EVERY SecurityEvent. PathHash and Host are
# event-type-CONDITIONAL: SecurityEvent uses serde `skip_serializing_if = Option::is_none`,
# so a path-deny event (10001) carries PathHash but NOT Host, and a network-deny event
# (10002) carries Host but NOT PathHash. Asserting all six unconditionally made the gate
# FAIL its own happy path (the gate triggers a path-deny, which has no Host field) — CR-01.
$script:RequiredJsonFieldsUniversal = @('EventType', 'AgentPid', 'SessionId', 'ChainHead')
# SC-3 raw path patterns (must NOT appear in event body):
$script:RawPathPatternUser  = 'C:\\Users\\'
# Broader Windows path pattern: drive-letter colon backslash, at least one component of 4+ chars
# This catches raw paths not covered by the user-path check but avoids false-positives on
# hex PathHash values (which contain no colon-backslash).
$script:RawPathPatternBroad = '[A-Za-z]:\\[A-Za-z][A-Za-z0-9]*\\[A-Za-z0-9_.-]{4,}'
# Window for recent event query (minutes)
$script:EventWindowMinutes  = 5

# ---------------------------------------------------------------------------
# Local assertion helper (throw-on-failure, harness-internal only).
# A throw = harness-internal error (exit 4). Use ONLY for "gate cannot run at all".
# Confinement/policy results are verdict objects, never throws.
# ---------------------------------------------------------------------------

function Assert-True {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$Condition,

        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if (-not $Condition) { throw $Message }
}

# ---------------------------------------------------------------------------
# Helper: Invoke-TriggerDenial
# Runs a minimal nono command that will attempt to access C:\Windows\System32\config\SAM
# (a path that is always denied under the claude-code profile: SYSTEM-only access,
# denied by Landlock on Linux, Seatbelt on macOS, and AppContainer on Windows).
# Seeds the Application log with a fresh path-deny event (EventID 10001).
# Returns $true if the trigger ran (nono is on PATH), $false if nono is not on PATH.
# NOTE: nono will exit non-zero (sandbox denial) — this is EXPECTED and NOT a gate error.
# ---------------------------------------------------------------------------

function Invoke-TriggerDenial {
    $nono = Get-Command nono -ErrorAction SilentlyContinue
    if ($null -eq $nono) { return $false }
    $nonoExe = $nono.Source

    try {
        $proc = Start-Process -FilePath $nonoExe `
            -ArgumentList 'run', '--profile', 'claude-code', '--', 'cmd', '/c', 'type', 'C:\Windows\System32\config\SAM' `
            -NoNewWindow -Wait -PassThru `
            -RedirectStandardOutput "$env:TEMP\nono-telem-trigger-stdout.txt" `
            -RedirectStandardError  "$env:TEMP\nono-telem-trigger-stderr.txt" `
            -ErrorAction SilentlyContinue
        # Non-zero exit = denial occurred (expected)
        return $true
    } catch {
        # Process spawn failed — nono may not support 'run' on this install
        return $false
    }
}

# ---------------------------------------------------------------------------
# Gate contract
# ---------------------------------------------------------------------------

function Test-Precondition {
    # Return $null when all preconditions met; return a reason string -> SKIP_HOST_UNAVAILABLE.

    # 1. Admin check: Get-WinEvent on the Application log requires admin on some systems;
    #    more importantly, logman query providers (SC-5) requires admin-level access for
    #    reliable provider enumeration. SKIP if not admin.
    $identity  = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        return 'telemetry-event-emit gate requires elevation (Get-WinEvent Application log + logman query providers both need admin) — re-run from an elevated shell'
    }

    # 2. Check if nono Application Event Log source is registered OR nono is on PATH.
    #    If neither, we cannot produce a verdict.
    $recentEvent = $null
    try {
        $recentEvent = Get-WinEvent -FilterHashtable @{
            LogName      = 'Application'
            ProviderName = $script:EventLogSource
            MaxEvents    = 1
        } -ErrorAction SilentlyContinue
    } catch {
        $recentEvent = $null
    }

    $nonoOnPath = ($null -ne (Get-Command nono -ErrorAction SilentlyContinue))

    if ($null -eq $recentEvent -and -not $nonoOnPath) {
        return 'nono Application Event Log source not registered and nono not on PATH — install nono MSI or run nono to register the source, then re-run'
    }

    return $null
}

function Invoke-Gate {
    # telemetry-event-emit gate: SC-1 (Application-log + EventID + named JSON fields),
    #                            SC-3 (no raw path strings in event body),
    #                            SC-5 (ETW provider detectable via logman).
    # NEVER calls exit. NEVER calls Persist-Verdict. Returns exactly one verdict object.

    # Native tools write progress to stderr; do not promote to terminating errors.
    $ErrorActionPreference = 'Continue'

    $stamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')

    # ==========================================================================
    # SC-1: Application-log entry assertion.
    #
    # Step 1: Attempt to seed a fresh event via Invoke-TriggerDenial.
    # Step 2: Query Application log for a nono security event in the last N minutes.
    # Step 3: Parse the event body as JSON and assert all six named fields.
    # ==========================================================================

    # Seed a fresh denial event (non-blocking if nono not on PATH — we fall back to
    # querying for a recent pre-existing event).
    $triggered = Invoke-TriggerDenial
    # Small delay to let the event appear in the log after the process exits.
    if ($triggered) { Start-Sleep -Milliseconds 1500 }

    $windowStart = (Get-Date).AddMinutes(-$script:EventWindowMinutes)
    $events = $null
    try {
        $events = Get-WinEvent -FilterHashtable @{
            LogName      = 'Application'
            ProviderName = $script:EventLogSource
            StartTime    = $windowStart
        } -ErrorAction SilentlyContinue | Where-Object {
            $_.Id -ge $script:EventIdMin -and $_.Id -le $script:EventIdMax
        }
    } catch {
        $events = $null
    }

    $eventCount = if ($null -eq $events) { 0 } elseif ($events -is [array]) { $events.Count } else { 1 }

    if ($eventCount -eq 0) {
        return [ordered]@{
            gate      = $script:TelemetryGateName
            verdict   = 'FAIL'
            reason    = "SC-1 FAILED: no nono security events found in Application log (EventID $($script:EventIdMin)-$($script:EventIdMax)) in the last $($script:EventWindowMinutes) minutes — run a confined nono command to generate a denial event, then re-run the gate"
            detail    = [ordered]@{
                assertion       = 'SC-1'
                triggered       = $triggered
                windowStartUtc  = $windowStart.ToString('yyyy-MM-ddTHH:mm:ssZ')
                eventCount      = 0
            }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # Take the first (most recent) event.
    $firstEvent = if ($events -is [array]) { $events[0] } else { $events }
    $eventId    = $firstEvent.Id
    $body       = $firstEvent.Message

    # Parse the body as JSON.
    $parsed = $null
    try {
        $parsed = $body | ConvertFrom-Json
    } catch {
        return [ordered]@{
            gate      = $script:TelemetryGateName
            verdict   = 'FAIL'
            reason    = "SC-1 FAILED: event body (EventID $eventId) is not valid JSON — the SecurityEventLayer serialization may have failed"
            detail    = [ordered]@{
                assertion      = 'SC-1'
                step           = 'json-parse'
                eventId        = $eventId
                bodyExcerpt    = ($body.Substring(0, [Math]::Min(200, $body.Length)))
                parseError     = $_.ToString()
            }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # Build the event-type-conditional required-field set (CR-01 fix).
    # Universal fields are always present; PathHash is required only for a path-deny
    # event (10001) and Host only for a network-deny event (10002), because the
    # SecurityEvent schema omits the inapplicable Option field via skip_serializing_if.
    $requiredFields = @($script:RequiredJsonFieldsUniversal)
    if ($eventId -eq $script:EventIdPathDeny) {
        $requiredFields += 'PathHash'
    } elseif ($eventId -eq $script:EventIdNetworkDeny) {
        $requiredFields += 'Host'
    }

    # Assert the conditional SC-1 named fields are present.
    foreach ($field in $requiredFields) {
        $fieldValue = $null
        try { $fieldValue = $parsed.$field } catch { $fieldValue = $null }
        if ($null -eq $fieldValue) {
            return [ordered]@{
                gate      = $script:TelemetryGateName
                verdict   = 'FAIL'
                reason    = "SC-1 FAILED: required field '$field' is missing from event body (EventID $eventId) — expected fields for this event type: $($requiredFields -join ', ')"
                detail    = [ordered]@{
                    assertion      = 'SC-1'
                    step           = 'field-check'
                    missingField   = $field
                    eventId        = $eventId
                    requiredFields = $requiredFields -join ', '
                    bodyExcerpt    = ($body.Substring(0, [Math]::Min(200, $body.Length)))
                    parsedFields   = ($parsed | Get-Member -MemberType NoteProperty | Select-Object -ExpandProperty Name) -join ', '
                }
                timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
            }
        }
    }

    # SC-1 passed.
    $sc1Pass       = $true
    $bodyExcerpt   = ($body.Substring(0, [Math]::Min(200, $body.Length)))

    # ==========================================================================
    # SC-3: No raw path strings in event body.
    #
    # The PathHash field should be a hex string (e.g. "3fa8c12b...") — NOT a raw
    # Windows path like "C:\Users\alice\secret.txt". Pitfall 11 prevention (D-08).
    # Check two patterns:
    #   (a) User path pattern: 'C:\Users\' (most sensitive)
    #   (b) Broad Windows path pattern: drive-letter colon backslash + component 4+ chars
    # ==========================================================================

    $rawUserPathFound  = ($body -match [regex]::Escape($script:RawPathPatternUser))
    $rawBroadPathFound = ($body -match $script:RawPathPatternBroad)

    if ($rawUserPathFound) {
        return [ordered]@{
            gate      = $script:TelemetryGateName
            verdict   = 'FAIL'
            reason    = "SC-3 FAILED: raw Windows user path string found in event body (matched 'C:\\Users\\' pattern) — secret/path leakage detected (Pitfall 11). PathHash field must be a hex hash, not the raw path."
            detail    = [ordered]@{
                assertion      = 'SC-3'
                matchPattern   = $script:RawPathPatternUser
                eventId        = $eventId
                bodyExcerpt    = $bodyExcerpt
                sc1Pass        = $sc1Pass
            }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    if ($rawBroadPathFound) {
        return [ordered]@{
            gate      = $script:TelemetryGateName
            verdict   = 'FAIL'
            reason    = "SC-3 FAILED: raw Windows path string found in event body (matched broad Windows path pattern) — path leakage detected (Pitfall 11). Paths must be hashed (D-08), not emitted raw."
            detail    = [ordered]@{
                assertion      = 'SC-3'
                matchPattern   = $script:RawPathPatternBroad
                eventId        = $eventId
                bodyExcerpt    = $bodyExcerpt
                sc1Pass        = $sc1Pass
            }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # SC-3 passed.
    $sc3Pass = $true

    # ==========================================================================
    # SC-5: ETW provider assertion.
    #
    # `logman query providers` lists registered ETW providers. The tracing-etw
    # LayerBuilder::new("nono") in init_tracing() registers the provider on first use.
    # If nono has been run and produced security events, the provider should appear.
    # ==========================================================================

    $etwOutput = $null
    try {
        $etwOutput = logman query providers 2>&1 | Out-String
    } catch {
        $etwOutput = ''
    }

    $etwFound = ($etwOutput -match '(?i)nono')

    if (-not $etwFound) {
        return [ordered]@{
            gate      = $script:TelemetryGateName
            verdict   = 'FAIL'
            reason    = "SC-5 FAILED: ETW provider 'nono' not detectable via logman — tracing-etw LayerBuilder registration may have failed, or no nono process has run on this host to register the provider. Run `nono run --profile claude-code -- cmd /c echo test` to register, then re-run."
            detail    = [ordered]@{
                assertion      = 'SC-5'
                logmanOutputExcerpt = ($etwOutput.Substring(0, [Math]::Min(400, $etwOutput.Length))).Trim()
                sc1Pass        = $sc1Pass
                sc3Pass        = $sc3Pass
                eventId        = $eventId
            }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # SC-5 passed.
    $sc5Pass = $true

    # ==========================================================================
    # All three assertions passed — PASS verdict.
    # ==========================================================================

    return [ordered]@{
        gate      = $script:TelemetryGateName
        verdict   = 'PASS'
        reason    = "SC-1 proven: Application log contains nono security event (EventID $eventId) with all required named JSON fields for its type ($($requiredFields -join ', ')). SC-3 proven: event body contains no raw Windows path strings (PathHash is a hex hash). SC-5 proven: ETW provider 'nono' detectable via logman."
        detail    = [ordered]@{
            assertion           = 'SC-1 + SC-3 + SC-5'
            sc1Pass             = $sc1Pass
            sc1EventId          = $eventId
            sc1EventCount       = $eventCount
            sc1BodyExcerpt      = $bodyExcerpt
            sc1FieldsVerified   = $requiredFields -join ', '
            sc3Pass             = $sc3Pass
            sc3RawUserPath      = $rawUserPathFound
            sc3RawBroadPath     = $rawBroadPathFound
            sc5Pass             = $sc5Pass
            sc5EtwProviderFound = $etwFound
            triggered           = $triggered
            windowStartUtc      = $windowStart.ToString('yyyy-MM-ddTHH:mm:ssZ')
        }
        timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
    }
}
