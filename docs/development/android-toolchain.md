# Android Toolchain Setup

This document records how to install and verify the Android (Plan 4) toolchain pinned by
`apps/android/gradle/libs.versions.toml`, `apps/android/app/build.gradle.kts`, and
`scripts/check-android-prerequisites.ps1`. All versions below are the exact pins; do not
drift without updating the checker, the Gradle files, and this document together.

## Pinned versions

| Component      | Version                                    | Where pinned                                        |
| -------------- | ------------------------------------------ | --------------------------------------------------- |
| JDK            | 17 (Microsoft OpenJDK)                     | `check-android-prerequisites.ps1` (`ReqJdkMajor`)   |
| Gradle         | 9.x (wrapper 9.4.1)                        | `gradle-wrapper.properties`                         |
| AGP            | 9.2.0                                      | `gradle/libs.versions.toml`                         |
| Kotlin         | 2.2.20                                     | `gradle/libs.versions.toml`                         |
| Compose BOM    | 2025.06.01                                 | `gradle/libs.versions.toml`                         |
| compileSdk     | 37 (`platforms;android-37`)                | `app/build.gradle.kts`                              |
| build-tools    | 36.0.0                                     | `check-android-prerequisites.ps1` (`ReqBuildTools`) |
| minSdk         | 31                                         | `app/build.gradle.kts`                              |
| NDK            | 28.2.13676358                              | `check-android-prerequisites.ps1` (`ReqNdk`)        |
| cargo-ndk      | 4.x                                        | `check-android-prerequisites.ps1` (`ReqCargoNdkMajor`) |
| UniFFI         | 0.32 (workspace-pinned)                    | root `Cargo.toml` / `tools/uniffi-bindgen`          |
| Rust targets   | `aarch64-linux-android`, `x86_64-linux-android` | `check-android-prerequisites.ps1`               |

## Install the JDK

```powershell
winget install --id Microsoft.OpenJDK.17 -e
```

Ensure `JAVA_HOME` points at the JDK 17 home and `java` is on `PATH`. The checker reads
`java -version` and requires major version `17`.

## Install the Android SDK

Install **Android Studio** (easiest) or the command-line tools only. Either way you need
`ANDROID_HOME` set and the following packages present:

```powershell
# one-time: point at the SDK root
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"

# install the command-line tools first, then the pinned packages
sdkmanager "cmdline-tools;latest"
sdkmanager "platforms;android-37"
sdkmanager "build-tools;36.0.0"
sdkmanager "ndk;28.2.13676358"
```

`sdkmanager.bat` lives under `cmdline-tools\latest\bin`; the checker looks for
`platforms/android-37/android.jar`, `build-tools/36.0.0/aapt2.exe`, and
`ndk/28.2.13676358/source.properties`.

## Install Gradle

```powershell
winget install --id Gradle.Gradle -e
```

The wrapper is already committed (`apps/android/gradle/wrapper/gradle-wrapper.properties`
pins `gradle-9.4.1-bin.zip`), so `./gradlew` works without a global install. The checker's
`GRADLE` probe requires a global `gradle` at major 9 only because `scripts/build-android.ps1`
invokes `gradle` directly; if you rely on the wrapper instead, edit that script to call
`./gradlew`.

## Install cargo-ndk and Rust targets

```powershell
cargo install cargo-ndk
rustup target add aarch64-linux-android x86_64-linux-android
```

`cargo-ndk` is a cargo subcommand (invoked as `cargo ndk`), not a standalone executable —
the checker probes it with `cargo ndk --version` for that reason.

## UniFFI bindings (no global install)

Since UniFFI 0.29 the `uniffi_bindgen` crate ships **no binary**, so `cargo install
uniffi_bindgen` fails with "no binaries". The repo instead provides a workspace tool crate
`tools/uniffi-bindgen` that pins the exact uniffi version and calls
`uniffi::uniffi_bindgen_main()`. Bindings are generated with:

```powershell
cargo run -p uniffi-bindgen -- generate --library <lib> --language kotlin --out-dir <dir>
```

`scripts/generate-kotlin-bindings.ps1` wraps this and writes into
`apps/android/app/src/main/java/com/tongpin/todo/todo_uniffi.kt`.

## Verify (uninstall-safe)

```powershell
pwsh -NoProfile -File scripts/check-android-prerequisites.ps1
```

Exits `0` only when every probe passes and prints a precise remediation list otherwise. The
contract tests (`scripts/tests/check-android-prerequisites.Tests.ps1`) pin the fixture-mode
behavior.

## Build the APK

```powershell
# debug
pwsh -NoProfile -File scripts/build-android.ps1 -Configuration Debug

# release (requires signing; see below)
pwsh -NoProfile -File scripts/build-android.ps1 -Configuration Release
```

The builder runs four steps: toolchain check → Kotlin binding generation → `cargo ndk`
cross-compile (arm64-v8a + x86_64 into `app/src/main/jniLibs`) → `gradle assembleDebug` /
`assembleRelease`. Its contract tests are `scripts/tests/build-android.Tests.ps1`.

## Release signing

`app/build.gradle.kts` does not embed signing secrets. The `release` build type loads a
`signingConfigs.release` block **only when** a `keystore.properties` file exists next to
`app/build.gradle.kts` (git-ignored); otherwise it falls back to debug signing so local
`assembleRelease` still produces an installable (but not store-publishable) APK.

Create `apps/android/app/keystore.properties`:

```properties
storeFile=../keystore/tongpin.jks
storePassword=...
keyAlias=tongpin
keyPassword=...
```

`storeFile` is resolved relative to the `app/` module, so `../keystore/tongpin.jks` maps to
`apps/android/keystore/tongpin.jks`. Generate the keystore with:

```powershell
keytool -genkeypair -v -keystore apps/android/keystore/tongpin.jks `
  -alias tongpin -keyalg RSA -keysize 4096 -validity 10000 `
  -storetype JKS
```

Both `keystore.properties` and the `keystore/` directory (and any `*.jks` / `*.keystore`)
are git-ignored — never commit them.

## Known environment notes

- **PowerShell variable names are case-insensitive** — the checker prefixes pinned-version
  constants with `Req*` (`$ReqNdk`, `$ReqJdkMajor`, …) to avoid colliding with probe locals.
- **`($x -or '')` returns a boolean, not a string** — the checker uses a `Format-Actual`
  helper to stringify probe values before printing.
- **cargo-ndk is not a standalone executable** — probe it via `cargo ndk --version`.
- **Defender may lock `target/`** during heavy cross-compiles; if `cargo ndk` fails with a
  file-lock error, pause real-time scanning or exclude `target/` and retry.
