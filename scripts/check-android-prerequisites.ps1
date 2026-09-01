# scripts/check-android-prerequisites.ps1
# Verifies the Android (Plan 4) toolchain. Exits 0 only when every probe passes.
# Pinned versions mirror apps/android/gradle/libs.versions.toml and the Plan 4 spec.
#
#   -ProbeFixture <path>   read probe results from a JSON file instead of probing
#   -Json                  emit machine-readable { ok, probes[], remediation[] }
#
# NOTE: PowerShell variable names are case-insensitive, so every probe's local
# boolean/string result uses a distinct name from the pinned-version constants.
param(
    [string]$ProbeFixture = $null,
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ReqPlatform = 'android-37'
$ReqBuildTools = '36.0.0'
$ReqNdk = '28.2.13676358'
$ReqJdkMajor = '17'
$ReqGradleMajor = '9'
$ReqCargoNdkMajor = '4'
$ReqUniffiMajor = '0'

function New-Probe {
    param([string]$Code, [string]$Expected, [string]$Actual, [bool]$Ok)
    return [pscustomobject]@{ code = $Code; expected = $Expected; actual = $Actual; ok = $Ok }
}

function Format-Actual {
    param($Value)
    if ($null -eq $Value -or $Value -eq '') { return '' }
    return [string]$Value
}

function Get-SdkRoot {
    $candidates = @($env:ANDROID_HOME, $env:ANDROID_SDK_ROOT)
    foreach ($c in $candidates) {
        if ($c -and (Test-Path $c)) { return $c }
    }
    $defaultPath = Join-Path $env:LOCALAPPDATA 'Android\Sdk'
    if (Test-Path $defaultPath) { return $defaultPath }
    return $null
}

function Get-JavaMajor {
    $java = Get-Command java -ErrorAction SilentlyContinue
    if ($null -eq $java) { return $null }
    $ver = (& java -version 2>&1 | Select-Object -First 1)
    if ($ver -match '"(\d+)') { return $Matches[1] }
    return $null
}

function Get-CmdVersion {
    param([string]$CmdName, [string[]]$ArgList = @('--version'))
    $cmd = Get-Command $CmdName -ErrorAction SilentlyContinue
    if ($null -eq $cmd) { return $null }
    return (& $cmd.Name @ArgList 2>$null | Select-Object -First 1)
}

# --- probes ---
function Get-Probes {
    $sdkPath = Get-SdkRoot

    # ANDROID_HOME
    $homeProbe = New-Probe -Code 'ANDROID_HOME' -Expected "set (e.g. $env:LOCALAPPDATA\Android\Sdk)" -Actual (Format-Actual $sdkPath) -Ok ($null -ne $sdkPath)

    # sdkmanager
    $smPath = $null
    if ($sdkPath) {
        $smPath = Get-ChildItem -Path (Join-Path $sdkPath 'cmdline-tools') -Recurse -Filter 'sdkmanager.bat' -ErrorAction SilentlyContinue |
            Select-Object -First 1
    }
    $smProbe = New-Probe -Code 'SDKMANAGER' -Expected 'cmdline-tools\latest\bin\sdkmanager.bat' -Actual ($(if ($smPath) { $smPath.FullName } else { '' })) -Ok ($null -ne $smPath)

    # platform android-37
    $platOk = if ($sdkPath) { Test-Path (Join-Path $sdkPath "platforms/$ReqPlatform/android.jar") } else { $false }
    $platProbe = New-Probe -Code 'SDK_PLATFORM' -Expected $ReqPlatform -Actual ($(if ($platOk) { $ReqPlatform } else { 'missing' })) -Ok $platOk

    # build-tools 36.0.0
    $btOk = if ($sdkPath) { Test-Path (Join-Path $sdkPath "build-tools/$ReqBuildTools/aapt2.exe") } else { $false }
    $btProbe = New-Probe -Code 'BUILD_TOOLS' -Expected $ReqBuildTools -Actual ($(if ($btOk) { $ReqBuildTools } else { 'missing' })) -Ok $btOk

    # ndk 28.2.13676358
    $ndkOk = if ($sdkPath) { Test-Path (Join-Path $sdkPath "ndk/$ReqNdk/source.properties") } else { $false }
    $ndkProbe = New-Probe -Code 'NDK' -Expected $ReqNdk -Actual ($(if ($ndkOk) { $ReqNdk } else { 'missing' })) -Ok $ndkOk

    # JDK 17
    $jdkActual = Get-JavaMajor
    $jdkProbe = New-Probe -Code 'JDK' -Expected $ReqJdkMajor -Actual (Format-Actual $jdkActual) -Ok ($jdkActual -eq $ReqJdkMajor)

    # gradle 9.x
    $gv = Get-CmdVersion -CmdName 'gradle' -ArgList @('-version')
    $gOk = $false
    if ($gv) { $gOk = ($gv -match "Gradle (\d+)") -and ($Matches[1] -eq $ReqGradleMajor) }
    $gProbe = New-Probe -Code 'GRADLE' -Expected "$ReqGradleMajor.x" -Actual (Format-Actual ($gv -join ' ')) -Ok $gOk

    # cargo-ndk 4.x (invoked as `cargo ndk`, a cargo subcommand)
    $cnOut = (& cargo ndk --version 2>$null | Select-Object -First 1)
    $cnOk = $false
    if ($cnOut) { $cnOk = ($cnOut -match '(\d+)\.') -and ($Matches[1] -eq $ReqCargoNdkMajor) }
    $cnProbe = New-Probe -Code 'CARGO_NDK' -Expected "$ReqCargoNdkMajor.x" -Actual (Format-Actual $cnOut) -Ok $cnOk

    # uniffi-bindgen (workspace tool crate tools/uniffi-bindgen)
    $toolCrate = Join-Path (Split-Path -Parent $PSScriptRoot) 'tools/uniffi-bindgen/Cargo.toml'
    $ubOk = Test-Path $toolCrate
    $ubProbe = New-Probe -Code 'UNIFFI_BINDGEN' -Expected "tools/uniffi-bindgen (pinned)" -Actual ($(if ($ubOk) { 'present' } else { 'missing' })) -Ok $ubOk

    # rust android targets
    $need = @('aarch64-linux-android', 'x86_64-linux-android')
    $expected = $need -join ','
    $installed = @(& rustup target list --installed 2>$null | ForEach-Object { $_.Trim() })
    $missing = @($need | Where-Object { $installed -notcontains $_ })
    $tgtProbe = New-Probe -Code 'RUST_TARGETS' -Expected $expected -Actual ($installed -join ',') -Ok ($missing.Count -eq 0)

    return @($homeProbe, $smProbe, $platProbe, $btProbe, $ndkProbe, $jdkProbe, $gProbe, $cnProbe, $ubProbe, $tgtProbe)
}

function Get-Remediation {
    param([array]$Probes)
    $map = @{
        'ANDROID_HOME'   = "Install Android Studio, or set ANDROID_HOME to an existing SDK (see docs/development/android-toolchain.md)"
        'SDKMANAGER'     = "sdkmanager 'cmdline-tools;latest'"
        'SDK_PLATFORM'   = "sdkmanager 'platforms;$ReqPlatform'"
        'BUILD_TOOLS'    = "sdkmanager 'build-tools;$ReqBuildTools'"
        'NDK'            = "sdkmanager 'ndk;$ReqNdk'"
        'JDK'            = "Install Microsoft OpenJDK 17 and set JAVA_HOME (winget install --id Microsoft.OpenJDK.17 -e)"
        'GRADLE'         = "Install Gradle $ReqGradleMajor.x (winget install --id Gradle.Gradle -e), or rely on ./gradlew wrapper"
        'CARGO_NDK'      = "cargo install cargo-ndk"
        'UNIFFI_BINDGEN' = "Restore the tools/uniffi-bindgen workspace crate (uniffi with the cli feature)"
        'RUST_TARGETS'   = "rustup target add aarch64-linux-android x86_64-linux-android"
    }
    $items = @()
    foreach ($p in $Probes) {
        if (-not $p.ok) {
            $items += [pscustomobject]@{ code = "$($p.code)_MISSING"; message = $map[$p.code] }
        }
    }
    return $items
}

# --- gather ---
if ($ProbeFixture) {
    $raw = Get-Content -Raw $ProbeFixture
    $probes = @(($raw | ConvertFrom-Json).probes)
} else {
    $probes = @(Get-Probes)
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
        Write-Output ("{0} {1,-16} expected={2,-16} actual={3}" -f $mark, $p.code, $p.expected, $p.actual)
    }
    Write-Output ''
    if ($ok) {
        Write-Output 'All Android prerequisites satisfied.'
    } else {
        Write-Output 'Missing Android prerequisites and remediation:'
        foreach ($r in $remediation) {
            Write-Output ("  - {0}: {1}" -f $r.code, $r.message)
        }
    }
}

if ($ok) { exit 0 } else { exit 1 }
