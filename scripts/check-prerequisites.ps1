# scripts/check-prerequisites.ps1
# Verifies the shared-core toolchain. Exits 0 only when every probe passes.
#
#   -ProbeFixture <path>   read probe results from a JSON file instead of probing
#   -Json                  emit machine-readable { ok, probes[], remediation[] }
param(
    [string]$ProbeFixture = $null,
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function New-Probe {
    param([string]$Code, [string]$Expected, [string]$Actual, [bool]$Ok)
    return [pscustomobject]@{ code = $Code; expected = $Expected; actual = $Actual; ok = $Ok }
}

function Get-VersionProbe {
    param([string]$Code, [string]$CmdName, [string]$Expected)
    $cmd = Get-Command $CmdName -ErrorAction SilentlyContinue
    if ($null -eq $cmd) {
        return New-Probe -Code $Code -Expected $Expected -Actual '' -Ok $false
    }
    $ver = (& $CmdName --version 2>$null | Select-Object -First 1)
    if ($ver -match '^v?(\d+)\.') {
        return New-Probe -Code $Code -Expected $Expected -Actual $ver -Ok ($Matches[1] -eq ($Expected -replace '\..*', ''))
    }
    return New-Probe -Code $Code -Expected $Expected -Actual $ver -Ok $false
}

function Get-RustProbe {
    $expected = '1.98.0'
    $rustc = Get-Command rustc -ErrorAction SilentlyContinue
    if ($null -eq $rustc) {
        return New-Probe -Code 'RUST' -Expected $expected -Actual '' -Ok $false
    }
    $ver = (& rustc --version 2>$null | Select-Object -First 1)
    if ($ver -match 'rustc (\d+\.\d+\.\d+)') {
        return New-Probe -Code 'RUST' -Expected $expected -Actual $Matches[1] -Ok ($Matches[1] -eq $expected)
    }
    return New-Probe -Code 'RUST' -Expected $expected -Actual $ver -Ok $false
}

function Get-CargoProbe {
    $expected = '1.98.0'
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if ($null -eq $cargo) {
        return New-Probe -Code 'CARGO' -Expected $expected -Actual '' -Ok $false
    }
    $ver = (& cargo --version 2>$null | Select-Object -First 1)
    if ($ver -match 'cargo (\d+\.\d+\.\d+)') {
        return New-Probe -Code 'CARGO' -Expected $expected -Actual $Matches[1] -Ok ($Matches[1] -eq $expected)
    }
    return New-Probe -Code 'CARGO' -Expected $expected -Actual $ver -Ok $false
}

function Get-MsvcLinkerProbe {
    $expected = 'link.exe'
    $link = Get-Command link.exe -ErrorAction SilentlyContinue
    if ($null -ne $link) {
        return New-Probe -Code 'MSVC_LINKER' -Expected $expected -Actual $link.Source -Ok $true
    }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path $vswhere) {
        $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
        if ($vsPath) {
            $linkPath = Get-ChildItem -Path "$vsPath\VC\Tools\MSVC" -Recurse -Filter link.exe -ErrorAction SilentlyContinue |
                Where-Object { $_.FullName -match 'Hostx64\\x64' } |
                Select-Object -First 1
            if ($linkPath) {
                return New-Probe -Code 'MSVC_LINKER' -Expected $expected -Actual $linkPath.FullName -Ok $true
            }
        }
    }
    return New-Probe -Code 'MSVC_LINKER' -Expected $expected -Actual '' -Ok $false
}

function Get-RustTargetsProbe {
    $need = @('x86_64-pc-windows-msvc', 'aarch64-linux-android', 'x86_64-linux-android')
    $expected = $need -join ','
    $rustup = Get-Command rustup -ErrorAction SilentlyContinue
    if ($null -eq $rustup) {
        return New-Probe -Code 'RUST_TARGETS' -Expected $expected -Actual '' -Ok $false
    }
    $installed = @(& rustup target list --installed 2>$null | ForEach-Object { $_.Trim() })
    $missing = @($need | Where-Object { $installed -notcontains $_ })
    if ($missing.Count -eq 0) {
        return New-Probe -Code 'RUST_TARGETS' -Expected $expected -Actual $expected -Ok $true
    }
    return New-Probe -Code 'RUST_TARGETS' -Expected $expected -Actual ($installed -join ',') -Ok $false
}

function Get-Remediation {
    param([array]$Probes)
    $map = @{
        'NODE'          = 'Install Node.js 24.x (winget install --id OpenJS.NodeJS.LTS -e)'
        'NPM'           = 'npm is bundled with Node.js; reinstall Node.js 24.x'
        'RUST'          = 'Install Rust 1.98.0 (winget install --id Rustlang.Rustup -e)'
        'CARGO'         = 'Cargo ships with Rust; reinstall Rust 1.98.0'
        'MSVC_LINKER'   = 'Install Visual Studio Build Tools 2022 with the Desktop development with C++ workload'
        'RUST_TARGETS'  = 'Run: rustup target add aarch64-linux-android x86_64-linux-android'
    }
    $items = @()
    foreach ($p in $Probes) {
        if (-not $p.ok) {
            $items += [pscustomobject]@{ code = "$($p.code)_MISSING"; message = $map[$p.code] }
        }
    }
    return $items
}

# --- gather probes ---
if ($ProbeFixture) {
    $raw = Get-Content -Raw $ProbeFixture
    $probes = @(($raw | ConvertFrom-Json).probes)
} else {
    $probes = @(
        (Get-VersionProbe -Code 'NODE' -CmdName 'node' -Expected '24.x'),
        (Get-VersionProbe -Code 'NPM' -CmdName 'npm' -Expected '11.x'),
        (Get-RustProbe),
        (Get-CargoProbe),
        (Get-MsvcLinkerProbe),
        (Get-RustTargetsProbe)
    )
}

$remediation = @(Get-Remediation -Probes $probes)
$ok = -not (@($probes | Where-Object { -not $_.ok }).Count -gt 0)

if ($Json) {
    $result = [pscustomobject]@{ ok = $ok; probes = $probes; remediation = $remediation }
    $result | ConvertTo-Json -Depth 4
} else {
    Write-Output ''
    foreach ($p in $probes) {
        $mark = if ($p.ok) { '[OK]  ' } else { '[MISS]' }
        Write-Output ("{0} {1,-14} expected={2,-12} actual={3}" -f $mark, $p.code, $p.expected, $p.actual)
    }
    Write-Output ''
    if ($ok) {
        Write-Output 'All prerequisites satisfied.'
    } else {
        Write-Output 'Missing prerequisites and remediation:'
        foreach ($r in $remediation) {
            Write-Output ("  - {0}: {1}" -f $r.code, $r.message)
        }
    }
}

if ($ok) { exit 0 } else { exit 1 }
