# scripts/release.ps1
# Optimized release build plus optional Windows Authenticode signing.
#
#   -Json             emit machine-readable { ok, built, signed }
#   -Certificate <p>  path to a .pfx/.p12 certificate (enables signing)
#   -TimestampUrl <u> timestamp server (default http://timestamp.digicert.com)
param(
    [switch]$Json,
    [string]$Certificate = $null,
    [string]$TimestampUrl = 'http://timestamp.digicert.com'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# --- 1. release build ---
& cargo build --release --workspace 2>&1 | Out-Null
$buildOk = ($LASTEXITCODE -eq 0)

$signed = $false
if ($buildOk -and $Certificate -and (Test-Path $Certificate)) {
    $signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($null -ne $signtool) {
        $exes = Get-ChildItem -Path target/release -Filter '*.exe' -ErrorAction SilentlyContinue
        foreach ($exe in $exes) {
            & $signtool.Source sign /f $Certificate /fd SHA256 /tr $TimestampUrl /td SHA256 $exe.FullName 2>&1 | Out-Null
        }
        $signed = ($LASTEXITCODE -eq 0)
    }
}

$ok = $buildOk

if ($Json) {
    [pscustomobject]@{ ok = $ok; built = $buildOk; signed = $signed } | ConvertTo-Json -Depth 3
} else {
    Write-Output ''
    if ($buildOk) {
        Write-Output '[PASS] cargo build --release --workspace'
    } else {
        Write-Output '[FAIL] cargo build --release --workspace'
    }
    if ($Certificate) {
        if ($signed) {
            Write-Output '[PASS] Authenticode signing'
        } else {
            Write-Output '[INFO] signing skipped (no signtool.exe or certificate issue)'
        }
    } else {
        Write-Output '[INFO] no -Certificate provided; skipping signing'
    }
    Write-Output ''
}

if ($ok) { exit 0 } else { exit 1 }
