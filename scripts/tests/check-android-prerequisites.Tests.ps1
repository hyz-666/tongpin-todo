# scripts/tests/check-android-prerequisites.Tests.ps1
# Framework-free contract tests for scripts/check-android-prerequisites.ps1
# Runs the checker as a subprocess so its `exit` does not terminate this test.
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Script:Pass = 0
$Script:Fail = 0
$Script:Failures = @()

function Assert-True {
    param([bool]$Condition, [string]$Name)
    if ($Condition) {
        $Script:Pass++
    } else {
        $Script:Fail++
        $Script:Failures += $Name
    }
}

$Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$Checker = Join-Path $Root 'scripts/check-android-prerequisites.ps1'
$Fixtures = Join-Path $PSScriptRoot 'fixtures/android-prerequisites'

$Supported = Join-Path $Fixtures 'supported.json'
$MissingNdk = Join-Path $Fixtures 'missing-ndk.json'

# --- Test 1: supported fixture yields exit 0 and ok=true ---
$out1 = & pwsh -NoProfile -File $Checker -ProbeFixture $Supported -Json 2>&1
$code1 = $LASTEXITCODE
Assert-True ($code1 -eq 0) "supported fixture exit 0 (got $code1)"
$json1 = ($out1 -join "`n") | ConvertFrom-Json
Assert-True ($json1.ok -eq $true) "supported fixture ok=true"
Assert-True (@($json1.probes).Count -eq 10) "supported fixture has 10 probes"

# --- Test 2: missing-ndk fixture yields nonzero exit, NDK ok=false, remediation NDK_MISSING ---
$out2 = & pwsh -NoProfile -File $Checker -ProbeFixture $MissingNdk -Json 2>&1
$code2 = $LASTEXITCODE
Assert-True ($code2 -ne 0) "missing-ndk fixture nonzero exit (got $code2)"
$json2 = ($out2 -join "`n") | ConvertFrom-Json
$ndkProbe = @($json2.probes | Where-Object { $_.code -eq 'NDK' })
Assert-True (@($ndkProbe).Count -eq 1) "missing-ndk has exactly one NDK probe"
Assert-True ($ndkProbe[0].ok -eq $false) "missing-ndk NDK probe ok=false"
$remCodes = @($json2.remediation | ForEach-Object { $_.code })
Assert-True ($remCodes -contains 'NDK_MISSING') "missing-ndk remediation contains NDK_MISSING"

# --- Test 3: missing-ndk fixture leaves other probes intact ---
$smProbe = @($json2.probes | Where-Object { $_.code -eq 'SDKMANAGER' })
Assert-True ($smProbe[0].ok -eq $true) "missing-ndk SDKMANAGER still ok=true"

# --- summary ---
Write-Output "PASS=$($Script:Pass) FAIL=$($Script:Fail)"
if ($Script:Fail -gt 0) {
    Write-Output "FAILURES:"
    $Script:Failures | ForEach-Object { Write-Output "  - $_" }
    exit 1
}
exit 0
