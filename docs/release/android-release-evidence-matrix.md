# Android 发布证据矩阵（Android Release Evidence Matrix）

> Plan 4 Android 客户端的构建、签名与发布证据。每条证据需在真实构建/设备上实测并记录。

## 一、构建流水线证据

`scripts/build-android.ps1` 四步端到端流水线，契约测试 `scripts/tests/build-android.Tests.ps1`
（fixture 模式）已覆盖其成功/失败分支。

| # | 步骤 | 脚本 | 契约测试 | 结果 |
|---|------|------|---------|------|
| 1 | 工具链检查 | `check-android-prerequisites.ps1` | `check-android-prerequisites.Tests.ps1` | ⬜ |
| 2 | Kotlin 绑定生成 | `generate-kotlin-bindings.ps1` | 版本契约校验 | ⬜ |
| 3 | 交叉编译（arm64-v8a + x86_64） | `cargo ndk` | — | ⬜ |
| 4 | Gradle 装配 | `gradle assembleDebug` / `assembleRelease` | `build-android.Tests.ps1` | ⬜ |

## 二、签名证据

| 场景 | 命令 | 预期 | 证据 | 结果 |
|------|------|------|------|------|
| 无 keystore 时 release 回退 debug 签名 | `gradle assembleRelease` | 产出可安装 APK（非商店发布） | 构建日志 | ⬜ |
| 有 keystore 时 release 正式签名 | 配置 `keystore.properties` 后 `assembleRelease` | `apksigner verify` 通过 | 签名日志 | ⬜ |
| 密钥不入库 | `git ls-files` | 无 `*.jks` / `keystore.properties` | 清单 | ⬜ |

签名校验命令：

```powershell
# Android SDK build-tools 自带
apksigner verify --print-certs app/build/outputs/apk/release/app-release.apk
```

## 三、APK 产物证据

| 产物 | ABI | 预期 | 结果 |
|------|-----|------|------|
| `app-debug.apk` | arm64-v8a + x86_64 | 两 ABI 各含 `libtodo_uniffi.so` | ⬜ |
| `app-release.apk` | arm64-v8a + x86_64 | 混淆开启（minify + shrink） | ⬜ |

验证 ABIs：

```powershell
unzip -l app/build/outputs/apk/release/app-release.apk | Select-String "lib/.*/libtodo_uniffi.so"
```

## 四、设备运行证据

| 场景 | 设备 | 预期 | 证据 | 结果 |
|------|------|------|------|------|
| 首次安装启动 | Android 13+ 真机 | JNI 加载成功，任务列表显示 | 截图/日志 | ⬜ |
| 新建/编辑任务 | Android 13+ 真机 | 本地持久化，重启仍在 | 截图 | ⬜ |
| 前后台切换触发同步 | Android 13+ 真机 | 生命周期触发网络同步 | 日志 | ⬜ |
| mDNS 发现 | Android ↔ Windows | 互相发现对端 | 日志 | ⬜ |

## 五、发布前人工确认

- [ ] `docs/development/android-toolchain.md` 与检查器/构建脚本版本钉扎一致
- [ ] `scripts/check-android-prerequisites.Tests.ps1` 与 `build-android.Tests.ps1` 全绿
- [ ] 签名密钥已备份（离线存储），`keystore.properties` 未入版本库
- [ ] Git tag 已打（`v0.1.0`）并推送 `origin/main`

## 结论模板

```
测试日期：____
构建环境：JDK 17 / Gradle 9.4.1 / AGP 9.2.0 / NDK 28.2.13676358
APK 版本：versionName 0.1.0 / versionCode 1
通过项：__ / __
阻塞问题：
- [ ] ____
```
