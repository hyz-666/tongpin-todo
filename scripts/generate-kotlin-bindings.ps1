# scripts/generate-kotlin-bindings.ps1
# Builds the cdylib and generates Kotlin bindings for the pinned UniFFI contract.
# Fails if contracts/core-api-version.json differs from the Rust build constants.
param(
    [string]$OutputPath = "generated/kotlin"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot

# 1. Verify the version contract matches the Rust constants.
$contractPath = Join-Path $Root 'contracts/core-api-version.json'
$contract = Get-Content -Raw $contractPath | ConvertFrom-Json
$expected = @{ coreApi = 1; schema = 1; protocolMajor = 1; protocolMinor = 0 }
foreach ($k in $expected.Keys) {
    if ($contract.$k -ne $expected[$k]) {
        throw "contracts/core-api-version.json field '$k' = $($contract.$k), expected $($expected[$k])"
    }
}

# 2. Build the cdylib.
Push-Location $Root
try {
    cargo build -p todo-uniffi --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
} finally {
    Pop-Location
}

# 3. Locate the built library.
$libName = if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'todo_uniffi.dll' } else { 'libtodo_uniffi.so' }
$libPath = Join-Path $Root "target/release/$libName"
if (-not (Test-Path $libPath)) {
    throw "built library not found: $libPath"
}

# 4. Generate Kotlin bindings using the workspace-pinned bindgen tool.
# Since uniffi 0.29 the `uniffi_bindgen` crate ships no binary; we invoke the
# workspace tool crate (tools/uniffi-bindgen) which pins the exact uniffi
# version via `uniffi::uniffi_bindgen_main()`.
$outDir = Join-Path $Root $OutputPath
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

& cargo run -p uniffi-bindgen -- generate --library $libPath --language kotlin --out-dir $outDir
if ($LASTEXITCODE -ne 0) { throw 'uniffi-bindgen generate failed' }

Write-Output "Kotlin bindings written to $outDir"
