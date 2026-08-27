# Windows Toolchain Setup

This document records how to install and verify the shared-core toolchain pinned by
`rust-toolchain.toml` and `scripts/check-prerequisites.ps1`. All versions below are
the exact pins; do not drift without updating the checker and this document together.

## Pinned versions

| Component      | Version                       |
| -------------- | ----------------------------- |
| Node.js        | 24.x                          |
| npm            | 11.x                          |
| Rust / Cargo   | 1.98.0                        |
| MSVC           | Visual Studio Build Tools 2022 (Desktop C++) |
| Rust targets   | `x86_64-pc-windows-msvc`, `aarch64-linux-android`, `x86_64-linux-android` |

## Install Rust

```powershell
winget install --id Rustlang.Rustup -e
```

`rustup` installs into `~/.cargo/bin`; ensure it is on the user `PATH` so new terminals
see `cargo`, `rustc`, and `rustup`.

Install the pinned cross-compilation targets for Android (used by later plans):

```powershell
rustup target add aarch64-linux-android x86_64-linux-android
```

## Install the MSVC linker

Install Visual Studio Build Tools 2022 and select the **Desktop development with C++**
workload. This provides `link.exe` plus the Windows SDK headers/libs needed to link
the host target.

## Verify (uninstall-safe)

These commands only read the environment; they never modify it.

```powershell
node --version
npm --version
rustc --version
cargo --version
rustfmt --version
cargo clippy --version
rustup target list --installed
```

Run the authoritative gate:

```powershell
pwsh -NoProfile -File scripts/check-prerequisites.ps1
```

It exits `0` only when every probe passes and prints a precise remediation list
otherwise. The contract tests (`scripts/tests/check-prerequisites.Tests.ps1`) pin the
fixture-mode behavior and are run before real probes in CI.

## Uninstall / reinstall safety

- Rust: `rustup self uninstall` removes the toolchain without touching the repository.
- Visual Studio Build Tools: uninstall via **Apps & features**; `link.exe` resolution
  is re-detected by the checker on the next run.
- Re-run the checker after any change to confirm the exact pinned state.
