# scripts/build-windows.ps1
# Builds the Windows desktop client end-to-end:
#   1. bundle the Tauri app (frontend build + cargo release + MSI/NSIS installers)
#   2. optionally sign the installers with Windows Authenticode (.pfx)
#
# Code signing is skipped (with a notice) when no certificate is provided or
# signtool.exe is unavailable. Requires the Windows toolchain per
# docs/development/windows-toolchain.md.
#
#   -Json                    emit machine-readable { ok, built, signed, artifacts[], steps[] }
#   -Certificate <p>         path to a .pfx/.p12 code-signing certificate
#   -CertificatePassword <s> .pfx password (passed to signtool /p)
#   -TimestampUrl <u>        timestamp server (default http://timestamp.digicert.com)
#   -Fixture <p>             read results from JSON instead of building (contract tests)
param(
    [switch]$Json,
    [string]$Certificate = $null,
    [string]$CertificatePassword = $null,
    [string]$TimestampUrl = 'http://timestamp.digicert.com',
    [string]$Fixture = $null
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot
$WindowsRoot = Join-Path $Root 'apps/windows'
$BundleDir = Join-Path $WindowsRoot 'src-tauri/target/release/bundle'

function Invoke-Step {
    param([string]$Code, [string]$Label, [scriptblock]$Command)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $output = & $Command 2>&1
    $exitCode = $LASTEXITCODE
    $sw.Stop()
    return [pscustomobject]@{
        code        = $Code
        label       = $Label
        ok          = ($exitCode -eq 0)
        exit_code   = $exitCode
        duration_ms = $sw.ElapsedMilliseconds
        output      = ($output | Out-String).Trim()
    }
}

if ($Fixture) {
    $raw = Get-Content -Raw $Fixture
    # NOTE: use a local name that does not collide (case-insensitively) with the
    # `[string] $Fixture` parameter, otherwise the parsed object is coerced back
    # to a string.
    $parsed = $raw | ConvertFrom-Json
    $steps = @($parsed.steps)
    $built = [bool]$parsed.built
    $signed = [bool]$parsed.signed
    $artifacts = @($parsed.artifacts)
} else {
    $steps = @()
    $steps += Invoke-Step -Code 'BUILD' -Label 'tauri build (MSI + NSIS)' -Command {
        Push-Location $WindowsRoot
        try { & npm run tauri build } finally { Pop-Location }
    }
    $built = (@($steps | Where-Object { $_.code -eq 'BUILD' -and $_.ok }).Count -gt 0)

    $artifacts = @()
    if ($built) {
        $artifacts = @(Get-ChildItem -Path $BundleDir -Recurse -Include '*.msi', '*.exe' -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
    }

    # --- signing (conditional) ---
    $signed = $false
    if ($built -and $Certificate -and (Test-Path $Certificate)) {
        $signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
        if ($null -ne $signtool) {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            $signOut = @()
            $allOk = $true
            foreach ($artifact in $artifacts) {
                $args = @('sign', '/f', $Certificate, '/fd', 'SHA256', '/tr', $TimestampUrl, '/td', 'SHA256')
                if ($CertificatePassword) { $args += @('/p', $CertificatePassword) }
                $args += $artifact
                $signOut += (& $signtool.Source @args 2>&1 | Out-String)
                if ($LASTEXITCODE -ne 0) { $allOk = $false }
            }
            $sw.Stop()
            $signed = $allOk
            $steps += [pscustomobject]@{
                code        = 'SIGN'
                label       = 'Authenticode signing'
                ok          = $allOk
                exit_code   = if ($allOk) { 0 } else { 1 }
                duration_ms = $sw.ElapsedMilliseconds
                output      = ($signOut -join "`n").Trim()
            }
        } else {
            $steps += [pscustomobject]@{
                code        = 'SIGN'
                label       = 'Authenticode signing (skipped)'
                ok          = $true
                exit_code   = 0
                duration_ms = 0
                output      = 'signtool.exe not found; skipping signing'
            }
        }
    } else {
        $steps += [pscustomobject]@{
            code        = 'SIGN'
            label       = 'Authenticode signing (skipped)'
            ok          = $true
            exit_code   = 0
            duration_ms = 0
            output      = 'no -Certificate provided; skipping signing'
        }
    }
}

$ok = $built -and -not (@($steps | Where-Object { -not $_.ok }).Count -gt 0)

if ($Json) {
    $summary = $steps | ForEach-Object {
        [pscustomobject]@{
            code = $_.code; label = $_.label; ok = $_.ok;
            exit_code = $_.exit_code; duration_ms = $_.duration_ms
        }
    }
    [pscustomobject]@{
        ok = $ok; built = $built; signed = $signed;
        artifacts = @($artifacts); steps = @($summary)
    } | ConvertTo-Json -Depth 4
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
        Write-Output "Windows build complete. signed=$signed"
        $artifacts | ForEach-Object { Write-Output "  $_" }
    } else {
        Write-Output 'Windows build failed. See docs/development/windows-toolchain.md for remediation.'
    }
}

if ($ok) { exit 0 } else { exit 1 }
