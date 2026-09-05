# Windows Toolchain Setup

> 本机 Windows 开发/构建 `tongpin-todo` 所需的完整工具链：共享 Rust 核心 + Windows 桌面客户端
> （Tauri 2）。所有版本均为精确钉扎；变更版本时须同步更新检查器、`rust-toolchain.toml` 与本
> 文档三处。

## 一、钉扎版本（Pinned versions）

### 共享核心

| 组件 | 版本 |
|------|------|
| Rust / Cargo | 1.98.0（`rust-toolchain.toml`） |
| Node.js | 24.x |
| npm | 11.x |
| MSVC | Visual Studio Build Tools 2022（Desktop development with C++） |
| Rust targets | `x86_64-pc-windows-msvc`、`aarch64-linux-android`、`x86_64-linux-android` |

### Windows 桌面客户端（Plan 5）

| 组件 | 版本 | 用途 |
|------|------|------|
| Tauri（Rust crate） | 2.x（workspace `tauri = "2"`） | 桌面运行时框架 |
| tauri-cli | 2.x（`@tauri-apps/cli` devDependency） | `tauri build` / `tauri dev` |
| WebView2 运行时 | 常青版（Evergreen） | 内嵌 WebView（Tauri 自动引导安装） |
| WiX Toolset | 由 `tauri build` 自动下载 | MSI 安装器打包 |
| NSIS | 由 `tauri build` 自动下载 | NSIS 安装器打包 |
| signtool.exe | Windows SDK（10.0.22621+） | Authenticode 代码签名（可选） |

---

## 二、安装 Rust

```powershell
winget install --id Rustlang.Rustup -e
```

`rustup` 安装到 `~/.cargo/bin`，确保其在用户 `PATH` 中。项目根目录的 `rust-toolchain.toml`
会自动选用 1.98.0 并安装 `rustfmt`/`clippy` 组件与三个 target。

---

## 三、安装 MSVC 链接器

安装 **Visual Studio Build Tools 2022** 并勾选 **Desktop development with C++** 工作负载，提供
`link.exe` 与 Windows SDK 头文件/库。这是链接 `x86_64-pc-windows-msvc` 宿主目标的前提。

---

## 四、安装 Node.js 与前端依赖

```powershell
winget install --id OpenJS.NodeJS.LTS -e
cd apps/windows
npm install
```

前端依赖包括 `@tauri-apps/cli`（提供 `npm run tauri` 命令）、Vite、React、MUI。

---

## 五、安装 WebView2 运行时

Tauri 打包时配置 `webviewInstallMode: downloadBootstrapper`，安装器会在目标机器上自动下载并
引导安装常青版 WebView2 运行时。开发机本地运行 `tauri dev` 前，请先安装：

```powershell
winget install --id Microsoft.EdgeWebView2Runtime -e
```

---

## 六、安装代码签名工具（可选）

仅发布正式签名安装器时需要。安装 **Windows SDK**（含 `signtool.exe`），并准备代码签名证书
`.pfx`。未提供证书时 `scripts/build-windows.ps1` 会跳过签名并提示（详见
`docs/release/windows-release-evidence-matrix.md`）。

```powershell
# 校验 signtool 可用
signtool /?
```

---

## 七、验证（只读，不修改环境）

```powershell
# 共享核心
node --version
npm --version
rustc --version
cargo --version
rustfmt --version
cargo clippy --version
rustup target list --installed

# Windows 桌面
cd apps/windows
npx tauri --version

# 权威门禁
pwsh -NoProfile -File scripts/check-prerequisites.ps1
```

`check-prerequisites.ps1` 在全部探针通过时退出 `0`，否则打印精确的补救清单。契约测试
（`scripts/tests/check-prerequisites.Tests.ps1`）钉扎了 fixture 模式行为。

---

## 八、卸载 / 重装安全

- Rust：`rustup self uninstall` 卸载工具链，不触碰仓库。
- Visual Studio Build Tools：经「应用与功能」卸载；下次运行检查器会重新探测 `link.exe`。
- WebView2：经「应用与功能」卸载「Microsoft Edge WebView2 Runtime」。
- 任何变更后重跑检查器确认钉扎状态。
