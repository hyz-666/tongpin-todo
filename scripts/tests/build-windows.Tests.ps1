# scripts/tests/build-windows.Tests.ps1
# Framework-free contract tests for scripts/build-windows.ps1
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
$Builder = Join-Path $Root 'scripts/build-windows.ps1'
$Fixtures = Join-Path $PSScriptRoot 'fixtures/build-windows'

# --- Test 1: all-pass fixture yields exit 0, ok=true, built=true, signed=true ---
$out1 = & pwsh -NoProfile -File $Builder -Fixture (Join-Path $Fixtures 'all-pass.json') -Json 2>&1
$code1 = $LASTEXITCODE
Assert-True ($code1 -eq 0) "all-pass exit 0 (got $code1)"
$json1 = ($out1 -join "`n") | ConvertFrom-Json
Assert-True ($json1.ok -eq $true) "all-pass ok=true"
Assert-True ($json1.built -eq $true) "all-pass built=true"
Assert-True ($json1.signed -eq $true) "all-pass signed=true"
Assert-True (@($json1.steps).Count -eq 2) "all-pass has two steps (BUILD + SIGN)"
Assert-True (@($json1.artifacts).Count -ge 2) "all-pass reports >=2 artifacts"

# --- Test 2: build-fail fixture yields nonzero exit, ok=false, built=false ---
$out2 = & pwsh -NoProfile -File $Builder -Fixture (Join-Path $Fixtures 'build-fail.json') -Json 2>&1
$code2 = $LASTEXITCODE
Assert-True ($code2 -ne 0) "build-fail nonzero exit (got $code2)"
$json2 = ($out2 -join "`n") | ConvertFrom-Json
Assert-True ($json2.ok -eq $false) "build-fail ok=false"
Assert-True ($json2.built -eq $false) "build-fail built=false"
$build = @($json2.steps | Where-Object { $_.code -eq 'BUILD' })
Assert-True ($build[0].ok -eq $false) "build-fail BUILD ok=false"

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
