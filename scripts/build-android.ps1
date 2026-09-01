# scripts/build-android.ps1
# Builds the Android client end-to-end:
#   1. verify the pinned Android toolchain
#   2. generate UniFFI Kotlin bindings into the app source tree
#   3. cross-compile todo-uniffi for arm64-v8a + x86_64 via cargo-ndk
#   4. assemble the APK with Gradle
#
# Requires the Android SDK/NDK/JDK17/Gradle per docs/development/android-toolchain.md.
#
#   -Configuration <Debug|Release>   build variant (default Debug)
#   -Json                            emit machine-readable { ok, steps[] }
#   -Fixture <p>                     read step results from a JSON file instead of
#                                    running the pipeline (used by contract tests)
param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug',
    [switch]$Json,
    [string]$Fixture = $null
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot
$AndroidRoot = Join-Path $Root 'apps/android'
$JniLibs = Join-Path $AndroidRoot 'app/src/main/jniLibs'
$CheckScript = Join-Path $PSScriptRoot 'check-android-prerequisites.ps1'
$BindScript = Join-Path $PSScriptRoot 'generate-kotlin-bindings.ps1'

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
    $steps = @(($raw | ConvertFrom-Json).steps)
} else {
    $gradleTask = if ($Configuration -eq 'Release') { 'assembleRelease' } else { 'assembleDebug' }

    $steps = @()
    $steps += Invoke-Step -Code 'CHECK' -Label 'check-android-prerequisites' -Command {
        & pwsh -NoProfile -File $CheckScript
    }
    $steps += Invoke-Step -Code 'BINDGEN' -Label 'generate-kotlin-bindings (app/src/main/java)' -Command {
        & pwsh -NoProfile -File $BindScript -OutputPath (Join-Path $AndroidRoot 'app/src/main/java')
    }
    $steps += Invoke-Step -Code 'CARGO_NDK' -Label 'cargo ndk (arm64-v8a + x86_64)' -Command {
        $cargoArgs = @('ndk', '-t', 'arm64-v8a', '-t', 'x86_64', '-o', $JniLibs, 'build', '-p', 'todo-uniffi')
        if ($Configuration -eq 'Release') { $cargoArgs += '--release' }
        Push-Location $Root
        try { & cargo @cargoArgs } finally { Pop-Location }
    }
    $steps += Invoke-Step -Code 'GRADLE' -Label "gradle $gradleTask" -Command {
        Push-Location $AndroidRoot
        try { & gradle $gradleTask } finally { Pop-Location }
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
    $result = [pscustomobject]@{ ok = $ok; configuration = $Configuration; steps = @($summary) }
    $result | ConvertTo-Json -Depth 3
} else {
    Write-Output ''
    foreach ($s in $steps) {
        $mark = if ($s.ok) { '[PASS]' } else { '[FAIL]' }
        Write-Output ("{0} {1,-10} {2} ({3} ms)" -f $mark, $s.code, $s.label, $s.duration_ms)
        if (-not $s.ok -and $s.output) {
            $lines = $s.output -split "`r?`n"
            $tail = $lines | Select-Object -Last 12
            Write-Output ('  ' + ($tail -join "`n  "))
        }
    }
    Write-Output ''
    if ($ok) {
        Write-Output "Android build complete ($Configuration)."
    } else {
        Write-Output "Android build failed. See docs/development/android-toolchain.md for remediation."
    }
}

if ($ok) { exit 0 } else { exit 1 }
