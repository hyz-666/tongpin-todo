# scripts/coverage.ps1
# Line-coverage gate. Requires cargo-llvm-cov (bundles llvm-tools-preview).
#
#   -Threshold <pct>   minimum line coverage to pass (default 80)
#   -Json              emit machine-readable { ok, tool, coverage, threshold }
#   -Fixture <p>       read a coverage summary from a JSON file instead of running
param(
    [double]$Threshold = 80.0,
    [switch]$Json,
    [string]$Fixture = $null
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-CoverageSummary {
    if ($Fixture) {
        $raw = Get-Content -Raw $Fixture
        return ($raw | ConvertFrom-Json)
    }
    # Run cargo llvm-cov and parse the TOTAL line from the summary.
    $out = & cargo llvm-cov --workspace --summary-only 2>&1
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        throw "cargo llvm-cov failed: $($out -join ' ')"
    }
    $line = $out | Where-Object { $_ -match '^\s*TOTAL' } | Select-Object -First 1
    if (-not $line) {
        throw "could not parse TOTAL line from llvm-cov output"
    }
    # The coverage percentage is the 4th whitespace-separated column.
    $fields = $line -split '\s+' | Where-Object { $_ -ne '' }
    $pct = [double]($fields[3].TrimEnd('%'))
    return [pscustomobject]@{ line = $pct }
}

$tool = Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue
if ($null -eq $tool -and -not $Fixture) {
    $coverage = 0.0
    $missing = $true
} else {
    $missing = $false
    try {
        $summary = Get-CoverageSummary
        $coverage = $summary.line
    } catch {
        if ($Json) {
            [pscustomobject]@{ ok = $false; tool = $false; coverage = $null; threshold = $Threshold; error = $_.Exception.Message } | ConvertTo-Json -Depth 3
        } else {
            Write-Output "coverage failed: $($_.Exception.Message)"
        }
        exit 2
    }
}

$ok = (-not $missing) -and ($coverage -ge $Threshold)

if ($Json) {
    [pscustomobject]@{ ok = $ok; tool = (-not $missing); coverage = $coverage; threshold = $Threshold } | ConvertTo-Json -Depth 3
} else {
    Write-Output ''
    if ($missing) {
        Write-Output "[MISS] cargo-llvm-cov not installed."
        Write-Output "  Install: rustup component add llvm-tools-preview; cargo install cargo-llvm-cov"
    } else {
        $mark = if ($ok) { '[PASS]' } else { '[FAIL]' }
        Write-Output ("{0} line coverage {1:N2}% (threshold {2}%)" -f $mark, $coverage, $Threshold)
    }
    Write-Output ''
}

if ($ok) { exit 0 } else { exit 1 }
