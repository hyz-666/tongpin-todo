# scripts/tests/check-prerequisites.Tests.ps1
# Framework-free contract tests for scripts/check-prerequisites.ps1
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
$Checker = Join-Path $Root 'scripts/check-prerequisites.ps1'
$Fixtures = Join-Path $PSScriptRoot 'fixtures/prerequisites'

$Supported = Join-Path $Fixtures 'supported.json'
$MissingRust = Join-Path $Fixtures 'missing-rust.json'

# --- Test 1: supported fixture yields exit 0 and ok=true ---
$out1 = & pwsh -NoProfile -File $Checker -ProbeFixture $Supported -Json 2>&1
$code1 = $LASTEXITCODE
Assert-True ($code1 -eq 0) "supported fixture exit 0 (got $code1)"
$json1 = ($out1 -join "`n") | ConvertFrom-Json
Assert-True ($json1.ok -eq $true) "supported fixture ok=true"

# --- Test 2: missing-rust fixture yields nonzero exit, RUST ok=false, remediation RUST_MISSING ---
$out2 = & pwsh -NoProfile -File $Checker -ProbeFixture $MissingRust -Json 2>&1
$code2 = $LASTEXITCODE
Assert-True ($code2 -ne 0) "missing-rust fixture nonzero exit (got $code2)"
$json2 = ($out2 -join "`n") | ConvertFrom-Json
$rustProbe = @($json2.probes | Where-Object { $_.code -eq 'RUST' })
Assert-True (@($rustProbe).Count -eq 1) "missing-rust has exactly one RUST probe"
Assert-True ($rustProbe[0].ok -eq $false) "missing-rust RUST probe ok=false"
$remCodes = @($json2.remediation | ForEach-Object { $_.code })
Assert-True ($remCodes -contains 'RUST_MISSING') "missing-rust remediation contains RUST_MISSING"

# --- summary ---
Write-Output "PASS=$($Script:Pass) FAIL=$($Script:Fail)"
if ($Script:Fail -gt 0) {
    Write-Output "FAILURES:"
    $Script:Failures | ForEach-Object { Write-Output "  - $_" }
    exit 1
}
exit 0
