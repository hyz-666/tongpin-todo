# Plan 5 — Windows MVP 客户端（Tauri 2 + React）

> 状态：**进行中**（2026-09-02 启动）

## 1. 背景

Plan 1–4 已完成 Rust 核心（`todo-core` 的 `CoreHandle` 门面：`open`/`dispatch`/`list_tasks`/
`search_tasks`/`close`/`backup`/`subscribe`/`runtime_status`）、SQLCipher 加密存储、LAN 同步协议、
mDNS 发现、SAS 配对，以及 Android（Kotlin + Compose）客户端。Plan 5 将这些能力以**完整桌面产品
形态**交付到 Windows。

`apps/windows/src-tauri/` 已存在 `tongpin-windows` crate（纯 Rust 库），实现了
`platform/`（DNS-SD 发现、listener、network_monitor）与 `sync_runtime.rs`，但尚未 Tauri 化。

## 2. 目标范围（MVP）

- **本地优先**：任务 CRUD + 智能列表（Inbox/Today/Tomorrow/Next7Days/Completed/自定义）+ FTS 搜索。
- **后台同步**：LAN 同步后台自动运行，界面仅提供最小状态指示；设备发现/配对细节走日志，不做配对 UI。
- **加密存储**：SQLCipher，密钥与设备身份由 Windows DPAPI 保护。
- **发布**：MSI/NSIS 安装器 + 代码签名。

**明确不在 MVP**：设备发现/配对/成员管理的图形界面（沿用 Android 的同步协议与后端，但 UI 最小化）。

## 3. 技术决策

| 项 | 决策 |
|----|------|
| 桌面框架 | Tauri 2（最新稳定版） |
| 前端 | Vite + React + TypeScript |
| UI 组件库 | MUI（Material 3，与 Android Compose 视觉一致） |
| 状态管理 | Zustand |
| 后端桥接 | 直接依赖 `todo-core`，经 `#[tauri::command]` + `invoke()`（**不走 UniFFI**） |
| 密钥/身份存储 | Windows DPAPI（`CryptProtectData` / `CryptUnprotectData`） |
| 目标平台 | Windows 10 / 11（WebView2 运行时） |

## 4. Task 划分（10 个）

| # | Task | 内容 |
|---|------|------|
| 1 | Tauri 2 工程骨架 | Cargo bin 入口 + `tauri.conf.json` + capabilities + Vite React TS 前端 + MUI |
| 2 | Rust 命令层 | `CoreHandle` → `#[tauri::command]`，DPAPI 密钥管理，Tauri State 持有实例 |
| 3 | 前端数据层 | `invoke` 封装 + Zustand store + TS 类型对齐 |
| 4 | 任务列表 UI | 智能列表导航 + 任务列表 + 完成切换/删除 |
| 5 | 搜索与编辑 | FTS 搜索框 + 任务编辑对话框（标题/描述/优先级/日期） |
| 6 | 后台同步集成 | 复用 `platform` + `sync_runtime`，前后台触发 + 最小状态指示 |
| 7 | 发布构建 | MSI/NSIS 打包 + Windows 代码签名配置 |
| 8 | 发布证据矩阵 | Windows release evidence matrix + release checklist |
| 9 | 工具链与项目文档 | `windows-toolchain.md` + PROJECT.md 收尾 |
| 10 | 统一门禁验证 | `cargo fmt/clippy/test` + 前端 build + `tauri build` |

## 5. 预期产出

1. 可安装运行的 Windows 桌面应用（`.msi` / `.exe`）。
2. 完整任务管理 UI + 后台 LAN 同步。
3. `docs/release/windows-release-evidence-matrix.md` + 更新的 release checklist。
4. `docs/development/windows-toolchain.md` 工具链文档。

## 6. 风险与约束

- **Tauri 构建较重**：需 WebView2 运行时 + `tauri-cli` + 前端依赖安装，本地首次构建耗时长。
- **DPAPI 为 Windows 专属**：`tongpin-windows` 已是 Windows 目标 crate，可安全引入 `windows` 依赖。
- **同步 UI 最小化**：后台同步复用 Plan 2/3 的 `SyncOrchestrator`/`PairingFlow`，MVP 不暴露交互。
- **契约一致性**：命令层必须与 `contracts/core-api-version.json`（coreApi=1）保持一致。
