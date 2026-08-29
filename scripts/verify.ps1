# scripts/verify.ps1
# Unified quality gate: fmt + clippy + test. Exits 0 only when every step passes.
#
#   -Json         emit machine-readable { ok, steps[] }
#   -SkipTest     skip the test step (fast iterations)
#   -Fixture <p>  read step results from a JSON file instead of running cargo
param(
    [switch]$Json,
    [switch]$SkipTest,
    [string]$Fixture = $null
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Clear-CargoLocks {
    $locks = @(
        'target/debug/.cargo-build-lock',
        'target/debug/.cargo-artifact-lock',
        'target/debug/.cargo-lock'
    )
    foreach ($l in $locks) {
        if (Test-Path $l) {
            Remove-Item $l -Force -ErrorAction SilentlyContinue
        }
    }
}

function Invoke-Step {
    param([string]$Code, [string]$Label, [scriptblock]$Command)
    Clear-CargoLocks
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $output = & $Command 2>&1
    $exitCode = $LASTEXITCODE
    $sw.Stop()
    return [pscustomobject]@{
        code      = $Code
        label     = $Label
        ok        = ($exitCode -eq 0)
        exit_code = $exitCode
        duration_ms = $sw.ElapsedMilliseconds
        output    = ($output | Out-String).Trim()
    }
}

if ($Fixture) {
    $raw = Get-Content -Raw $Fixture
    $steps = @(($raw | ConvertFrom-Json).steps)
} else {
    $steps = @()
    $steps += Invoke-Step -Code 'FMT' -Label 'cargo fmt --all --check' -Command { cargo fmt --all --check }
    $steps += Invoke-Step -Code 'CLIPPY' -Label 'cargo clippy --workspace --all-targets -- -D warnings' -Command { cargo clippy --workspace --all-targets -- -D warnings }
    if (-not $SkipTest) {
        $steps += Invoke-Step -Code 'TEST' -Label 'cargo test --workspace' -Command { cargo test --workspace }
    }
}

$ok = -not (@($steps | Where-Object { -not $_.ok }).Count -gt 0)

if ($Json) {
    $summary = $steps | ForEach-Object {
        [pscustomobject]@{
            code = $_.code; label = $_.label; ok = $_.ok;
            exit_code = $_.exit_code; duration_ms = $_.duration_ms
        }
    }
    $result = [pscustomobject]@{ ok = $ok; steps = @($summary) }
    $result | ConvertTo-Json -Depth 3
} else {
    Write-Output ''
    foreach ($s in $steps) {
        $mark = if ($s.ok) { '[PASS]' } else { '[FAIL]' }
        Write-Output ("{0} {1,-8} {2} ({3} ms)" -f $mark, $s.code, $s.label, $s.duration_ms)
        if (-not $s.ok -and $s.output) {
            $lines = $s.output -split "`r?`n"
            $tail = $lines | Select-Object -Last 12
            Write-Output ('  ' + ($tail -join "`n  "))
        }
    }
    Write-Output ''
    if ($ok) {
        Write-Output 'All checks passed.'
    } else {
        Write-Output 'Verification failed.'
    }
}

if ($ok) { exit 0 } else { exit 1 }
