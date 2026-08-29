# scripts/audit.ps1
# Dependency audit: RustSec vulnerability scan (cargo-audit) plus license and
# ban checks (cargo-deny, optional). Exits 0 only when no advisories/licenses
# are violated.
#
#   -Json        emit machine-readable { ok, tools{audit,deny} }
#   -SkipDeny    skip the cargo-deny step (it is optional)
param(
    [switch]$Json,
    [switch]$SkipDeny
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$auditTool = Get-Command cargo-audit -ErrorAction SilentlyContinue
$denyTool = Get-Command cargo-deny -ErrorAction SilentlyContinue

$auditOk = $false
if ($null -ne $auditTool) {
    & cargo audit 2>&1 | Out-Null
    $auditOk = ($LASTEXITCODE -eq 0)
}

$denyOk = $true
$denyPresent = ($null -ne $denyTool)
if ($denyPresent -and -not $SkipDeny) {
    & cargo deny check 2>&1 | Out-Null
    $denyOk = ($LASTEXITCODE -eq 0)
}

$ok = $auditOk -and $denyOk

if ($Json) {
    [pscustomobject]@{
        ok = $ok
        tools = [pscustomobject]@{
            audit = [pscustomobject]@{ present = ($null -ne $auditTool); ok = $auditOk }
            deny  = [pscustomobject]@{ present = $denyPresent; ok = $denyOk }
        }
    } | ConvertTo-Json -Depth 4
} else {
    Write-Output ''
    if ($null -eq $auditTool) {
        Write-Output '[MISS] cargo-audit not installed.'
        Write-Output '  Install: cargo install cargo-audit'
    } else {
        $mark = if ($auditOk) { '[PASS]' } else { '[FAIL]' }
        Write-Output "$mark cargo-audit (RustSec vulnerability scan)"
    }
    if ($denyPresent -and -not $SkipDeny) {
        $mark = if ($denyOk) { '[PASS]' } else { '[FAIL]' }
        Write-Output "$mark cargo-deny (licenses and bans)"
    } elseif (-not $denyPresent) {
        Write-Output '[INFO] cargo-deny not installed (optional); skipping license check.'
        Write-Output '  Install: cargo install cargo-deny'
    }
    Write-Output ''
    if ($ok) { Write-Output 'Dependency audit passed.' } else { Write-Output 'Dependency audit failed.' }
}

if ($ok) { exit 0 } else { exit 1 }
