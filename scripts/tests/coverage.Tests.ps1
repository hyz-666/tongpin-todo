# scripts/tests/coverage.Tests.ps1
# Framework-free contract tests for scripts/coverage.ps1
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
$Coverage = Join-Path $Root 'scripts/coverage.ps1'
$Fixtures = Join-Path $PSScriptRoot 'fixtures/coverage'

# --- Test 1: pass fixture (86.4%) with threshold 80 yields exit 0 and ok=true ---
$out1 = & pwsh -NoProfile -File $Coverage -Fixture (Join-Path $Fixtures 'pass.json') -Threshold 80 -Json 2>&1
$code1 = $LASTEXITCODE
Assert-True ($code1 -eq 0) "pass fixture exit 0 (got $code1)"
$json1 = ($out1 -join "`n") | ConvertFrom-Json
Assert-True ($json1.ok -eq $true) "pass fixture ok=true"
Assert-True ($json1.coverage -eq 86.4) "pass fixture coverage=86.4"

# --- Test 2: fail fixture (64.2%) with threshold 80 yields exit 1 and ok=false ---
$out2 = & pwsh -NoProfile -File $Coverage -Fixture (Join-Path $Fixtures 'fail.json') -Threshold 80 -Json 2>&1
$code2 = $LASTEXITCODE
Assert-True ($code2 -ne 0) "fail fixture nonzero exit (got $code2)"
$json2 = ($out2 -join "`n") | ConvertFrom-Json
Assert-True ($json2.ok -eq $false) "fail fixture ok=false"

# --- Test 3: without the tool installed (no fixture), reports tool=false ---
$out3 = & pwsh -NoProfile -File $Coverage -Json 2>&1
$code3 = $LASTEXITCODE
Assert-True ($code3 -ne 0) "missing tool nonzero exit (got $code3)"
$json3 = ($out3 -join "`n") | ConvertFrom-Json
Assert-True ($json3.tool -eq $false) "missing tool reports tool=false"

# --- summary ---
Write-Output "PASS=$($Script:Pass) FAIL=$($Script:Fail)"
if ($Script:Fail -gt 0) {
    Write-Output "FAILURES:"
    $Script:Failures | ForEach-Object { Write-Output "  - $_" }
    exit 1
}
exit 0
