# scripts/sbom.ps1
# Generate a CycloneDX software bill of materials. Requires cargo-cyclonedx.
#
#   -Output <path>   output JSON path (default .release/sbom.cdx.json)
#   -Json            emit machine-readable { ok, tool, path }
param(
    [string]$Output = ".release/sbom.cdx.json",
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$tool = Get-Command cargo-cyclonedx -ErrorAction SilentlyContinue
if ($null -eq $tool) {
    if ($Json) {
        [pscustomobject]@{ ok = $false; tool = $false; path = $Output } | ConvertTo-Json -Depth 3
    } else {
        Write-Output '[MISS] cargo-cyclonedx not installed.'
        Write-Output '  Install: cargo install cargo-cyclonedx'
    }
    exit 1
}

$dir = Split-Path -Parent $Output
if ($dir -and -not (Test-Path $dir)) {
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
}

& cargo cyclonedx --all --format json --output-directory $dir 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    if ($Json) {
        [pscustomobject]@{ ok = $false; tool = $true; path = $Output } | ConvertTo-Json -Depth 3
    } else {
        Write-Output 'cargo cyclonedx failed.'
    }
    exit 1
}

if ($Json) {
    [pscustomobject]@{ ok = $true; tool = $true; path = $Output } | ConvertTo-Json -Depth 3
} else {
    Write-Output "[PASS] SBOM written to $Output"
}
exit 0
