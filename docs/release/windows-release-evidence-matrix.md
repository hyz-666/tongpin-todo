# Windows 发布证据矩阵（Windows Release Evidence Matrix）

> Plan 5 Windows 桌面客户端的构建、签名与发布证据。每条证据需在真实构建/设备上实测并记录。
> 自动化契约测试已覆盖的部分以 ✅ 标注；需真实 `tauri build` / 设备运行的以 ⬜ 待填。

## 一、构建流水线证据

`scripts/build-windows.ps1` 两步端到端流水线（BUILD → SIGN），契约测试
`scripts/tests/build-windows.Tests.ps1`（fixture 模式）已覆盖其成功/失败/跳过签名分支。

| # | 步骤 | 脚本 | 契约测试 | 结果 |
|---|------|------|---------|------|
| 1 | 前端构建（`tsc` + `vite`） | `npm run build`（tauri `beforeBuildCommand`） | — | ✅ 已通过 |
| 2 | Tauri 打包（MSI + NSIS） | `build-windows.ps1` → `tauri build` | `build-windows.Tests.ps1` | ⬜ 待 `tauri build` 实测 |
| 3 | 代码签名（可选） | `build-windows.ps1 -Certificate <pfx>` | 同上（跳过分支） | ⬜ 待签名实测 |

契约测试结果：`build-windows.Tests.ps1` → **PASS=14 FAIL=0**（BUILD/SIGN 成功与失败分支、JSON 结构契约）。

## 二、签名证据

| 场景 | 命令 | 预期 | 证据 | 结果 |
|------|------|------|------|------|
| 无证书时跳过签名并提示 | `build-windows.ps1`（无 `-Certificate`） | `signed=false`，SIGN 步骤 ok 且输出“skipping” | 脚本日志 | ✅ 契约已覆盖 |
| 有 `.pfx` 时 Authenticode 签名 | `build-windows.ps1 -Certificate <pfx> -CertificatePassword <p>` | 安装器含有效数字签名 | 签名日志 | ⬜ |
| 证书不入库 | `git ls-files` | 无 `*.pfx` / `*.p12` / 密码明文 | 清单 | ⬜ |

签名校验命令：

```powershell
# Windows SDK signtool 或 PowerShell
signtool verify /pa /v "apps/windows/src-tauri/target/release/bundle/nsis/tongpin-todo_0.1.0_x64-setup.exe"
Get-AuthenticodeSignature "apps/windows/src-tauri/target/release/bundle/nsis/tongpin-todo_0.1.0_x64-setup.exe"
```

## 三、安装器产物证据

| 产物 | 格式 | 预期 | 结果 |
|------|------|------|------|
| NSIS 安装器 | `.exe`（`bundle/nsis/`） | currentUser 安装、无管理员提权 | ⬜ |
| MSI 安装器 | `.msi`（`bundle/msi/`） | zh-CN 语言、静默安装可用 | ⬜ |
| 主程序 | `bundle/.../tongpin-todo.exe` | 含 WebView2 引导（downloadBootstrapper） | ⬜ |

产物清单命令：

```powershell
Get-ChildItem "apps/windows/src-tauri/target/release/bundle" -Recurse -Include *.msi,*.exe | Select-Object FullName, Length
```

## 四、设备运行证据

| 场景 | 环境 | 预期 | 证据 | 结果 |
|------|------|------|------|------|
| 首次安装启动 | Windows 10/11 | 打开核心、任务列表显示 | 截图 | ⬜ |
| 新建/编辑/搜索任务 | Windows 10/11 | 本地持久化，重启仍在 | 截图 | ⬜ |
| DPAPI 密钥保护 | Windows 10/11 | `device_id.dpapi` / `db_key.dpapi` 生成且不可明文读取 | 文件清单 | ⬜ |
| 后台 LAN 同步状态指示 | Windows ↔ Android/Windows | 状态指示 离线/同步中/已连接 正确切换 | 截图 | ⬜ |
| mDNS 互相发现 | Windows ↔ Android | 双方服务可见 | 日志 | ⬜ |

## 五、发布前人工确认

- [ ] `docs/development/windows-toolchain.md` 与 `build-windows.ps1` 的依赖版本钉扎一致
- [ ] `scripts/tests/build-windows.Tests.ps1` 全绿
- [ ] 代码签名证书已备份（离线存储），`.pfx` / 密码未入版本库
- [ ] `cargo build --release -p tongpin-windows` 通过
- [ ] Git tag 已打（`v0.1.0`）并推送 `origin/main`

## 结论模板

```
测试日期：____
构建环境：Windows 10/11 / Rust (stable) / Node 22 / tauri-cli 2.x / WebView2 运行时
安装器版本：0.1.0
通过项：__ / __
阻塞问题：
- [ ] ____
```
