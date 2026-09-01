# scripts/tests/build-android.Tests.ps1
# Framework-free contract tests for scripts/build-android.ps1
# Runs the builder as a subprocess so its `exit` does not terminate this test.
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
$Builder = Join-Path $Root 'scripts/build-android.ps1'
$Fixtures = Join-Path $PSScriptRoot 'fixtures/build-android'

# --- Test 1: all-pass fixture yields exit 0 and ok=true, four steps ---
$out1 = & pwsh -NoProfile -File $Builder -Fixture (Join-Path $Fixtures 'all-pass.json') -Json 2>&1
$code1 = $LASTEXITCODE
Assert-True ($code1 -eq 0) "all-pass exit 0 (got $code1)"
$json1 = ($out1 -join "`n") | ConvertFrom-Json
Assert-True ($json1.ok -eq $true) "all-pass ok=true"
Assert-True (@($json1.steps).Count -eq 4) "all-pass has four steps"

# --- Test 2: check-fail fixture yields nonzero exit, CHECK ok=false ---
$out2 = & pwsh -NoProfile -File $Builder -Fixture (Join-Path $Fixtures 'check-fail.json') -Json 2>&1
$code2 = $LASTEXITCODE
Assert-True ($code2 -ne 0) "check-fail nonzero exit (got $code2)"
$json2 = ($out2 -join "`n") | ConvertFrom-Json
Assert-True ($json2.ok -eq $false) "check-fail ok=false"
$check = @($json2.steps | Where-Object { $_.code -eq 'CHECK' })
Assert-True (@($check).Count -eq 1) "check-fail has one CHECK step"
Assert-True ($check[0].ok -eq $false) "check-fail CHECK ok=false"

# --- Test 3: JSON summary exposes code/label/ok/exit_code/duration_ms ---
$step = $json1.steps[0]
Assert-True ($null -ne $step.code) "step has code"
Assert-True ($null -ne $step.label) "step has label"
Assert-True ($null -ne $step.exit_code) "step has exit_code"
Assert-True ($null -ne $step.duration_ms) "step has duration_ms"

# --- summary ---
Write-Output "PASS=$($Script:Pass) FAIL=$($Script:Fail)"
if ($Script:Fail -gt 0) {
    Write-Output "FAILURES:"
    $Script:Failures | ForEach-Object { Write-Output "  - $_" }
    exit 1
}
exit 0
