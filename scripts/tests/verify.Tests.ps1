# scripts/tests/verify.Tests.ps1
# Framework-free contract tests for scripts/verify.ps1
# Runs the gate as a subprocess so its `exit` does not terminate this test.
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
$Verifier = Join-Path $Root 'scripts/verify.ps1'
$Fixtures = Join-Path $PSScriptRoot 'fixtures/verify'

# --- Test 1: all-pass fixture yields exit 0 and ok=true, three steps ---
$out1 = & pwsh -NoProfile -File $Verifier -Fixture (Join-Path $Fixtures 'all-pass.json') -Json 2>&1
$code1 = $LASTEXITCODE
Assert-True ($code1 -eq 0) "all-pass exit 0 (got $code1)"
$json1 = ($out1 -join "`n") | ConvertFrom-Json
Assert-True ($json1.ok -eq $true) "all-pass ok=true"
Assert-True (@($json1.steps).Count -eq 3) "all-pass has three steps"

# --- Test 2: clippy-fail fixture yields nonzero exit, CLIPPY ok=false ---
$out2 = & pwsh -NoProfile -File $Verifier -Fixture (Join-Path $Fixtures 'clippy-fail.json') -Json 2>&1
$code2 = $LASTEXITCODE
Assert-True ($code2 -ne 0) "clippy-fail nonzero exit (got $code2)"
$json2 = ($out2 -join "`n") | ConvertFrom-Json
Assert-True ($json2.ok -eq $false) "clippy-fail ok=false"
$clippy = @($json2.steps | Where-Object { $_.code -eq 'CLIPPY' })
Assert-True (@($clippy).Count -eq 1) "clippy-fail has one CLIPPY step"
Assert-True ($clippy[0].ok -eq $false) "clippy-fail CLIPPY ok=false"

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
