# tongpin-todo 项目文档

> 本地优先（local-first）的待办事项系统核心。Rust 实现，目标覆盖 Windows 与 Android。

---

## 1. 项目概述

tongpin-todo 是一个**本地优先、可跨设备同步**的待办事项应用核心。数据首先存储在本地（SQLCipher 加密的 SQLite），通过 CRDT 操作日志实现确定性合并，再经由局域网同步在设备间流转。

**核心设计原则**：

- **本地优先** —— 所有数据本地持久化，离线可用，不依赖服务器可达性。
- **加密存储** —— 数据库用 SQLCipher（AES-256）加密，密钥由 OS 保管。
- **确定性合并** —— 所有变更表示为「操作」（operation），带 HLC 时钟 + 设备 ID，可确定性合并，无需中央协调。
- **业务与传输解耦** —— LAN 同步、发现、配对均通过端口（port）注入，domain/storage/core 不依赖任何具体传输。

---

## 2. 当前状态

**Plan 1–5 均已 100% 完成。**

| 里程碑 | 状态 |
|--------|------|
| Plan 1：本地核心数据 | ✅ 完成（10/10 Task） |
| Plan 2：LAN 同步与安全 | ✅ 完成（10/10 Task） |
| Plan 3：跨设备发布 | ✅ 完成（10/10 Task） |
| Plan 4：Android 手机/平板 | ✅ 完成（10/10 Task） |
| Plan 5：Windows MVP 客户端 | ✅ 完成（10/10 Task） |

- Git 提交：57 个（`ffd1586` → `01eb645`）
- 测试：**304 个 Rust 测试全绿**，`cargo fmt` + `cargo clippy -D warnings` 全部通过
- Android 契约测试：`check-android-prerequisites.Tests.ps1`（8 项）+ `build-android.Tests.ps1`（11 项）全绿
- Windows 契约测试：`build-windows.Tests.ps1`（14 项）全绿
- 统一门禁：`scripts/verify.ps1`（fmt + clippy + test）
- 仓库：https://github.com/hyz-666/tongpin-todo

---

## 3. 技术栈

| 类别 | 选型 |
|------|------|
| 语言 | Rust 1.98（edition 2024） |
| 数据库 | rusqlite 0.40 + bundled SQLCipher（`SQLCipher-vendored-openssl`） |
| 全文搜索 | SQLite FTS5（trigram tokenizer） |
| 序列化 | serde / serde_json / ciborium |
| UUID | uuid 1.25（v7，时间有序） |
| Unicode 归一化 | unicode-normalization |
| 加密 | argon2（Argon2id）+ chacha20poly1305（XChaCha20-Poly1305） |
| 并发 | std::sync（Mutex/mpsc/atomic） |
| FFI | UniFFI 0.32（proc-macro 方式） |
| 测试 | proptest + tempfile |
| Android UI | Kotlin 2.2.20 + Jetpack Compose（BOM 2025.06.01，Material 3） |
| Android 构建 | AGP 9.2.0 / Gradle 9.4.1 / JDK 17 / compileSdk 37 / minSdk 31 / NDK 28.2.13676358 |
| Android FFI 桥接 | JNA 5.16.0（UniFFI Kotlin 绑定）+ `cargo ndk` 交叉编译 |
| 绑定生成 | `tools/uniffi-bindgen`（workspace 工具 crate，锁定 uniffi cli feature） |
| Windows 桌面框架 | Tauri 2 + WebView2（目标 Windows 10/11） |
| Windows UI | Vite + React + TypeScript + MUI（Material 3）+ Zustand |
| Windows 密钥存储 | Windows DPAPI（`CryptProtectData` / `CryptUnprotectData`） |

---

## 4. 架构

Workspace 采用分层 crate 结构，依赖方向单向（domain ← storage ← core ← uniffi）：

```
crates/
├── todo-domain     # 领域层：模型、命令、操作、验证、时钟、ID
├── todo-storage    # 持久层：加密 SQLite、迁移、物化、查询、搜索、日历
├── todo-core       # 核心层：命令分发、远端应用、查询、备份恢复、门面、订阅
├── todo-uniffi     # FFI 契约层：UniFFI 边界（Windows/Android）
├── todo-crypto     # 预留：规范化、身份、签名（Plan 2）
├── todo-protocol   # 预留：帧、版本协商、消息类型（Plan 2）
├── todo-discovery  # 预留：发现提示、候选生命周期（Plan 2）
└── todo-testkit    # 共享测试支持
```

**分层职责**：

| 层 | 职责 | 关键模块 |
|----|------|---------|
| domain | 不可变领域类型 + 纯函数 | `model`（Task/List/Tag/Subtask）、`command`（22 变体）、`operation`（操作+合并）、`clock`（HLC）、`ids`（EntityId/DeviceId/OperationId）、`register`（字段寄存器）、`validation`（归一化校验） |
| storage | 加密持久化 + 投影 | `connection`（SQLCipher key）、`migration`（版本化）、`schema`（20 表）、`materialize`（物化）、`repository`（表操作）、`query`/`search`/`calendar`、`health`/`quarantine` |
| core | 唯一写入口 + 公共 API | `dispatch`（命令→操作）、`apply`（远端批应用）、`query`（视图）、`backup`/`recovery`、`api`（CoreHandle）、`subscription`/`runtime` |
| uniffi | FFI 安全边界 | `types`/`error`/`api` |

---

## 5. 已完成功能（Plan 1）

### Task 1 — 工具链与骨架
Workspace + Rust 1.98 + `check-prerequisites.ps1` 工具链检查器。

### Task 2 — 领域模型
- 实体：Task、Subtask、List、Tag
- HLC 时钟（Hybrid Logical Clock）、UUIDv7 EntityId、32 字节 DeviceId
- Unicode 归一化（NFKC + case fold），保证「买牛奶」与「買牛奶」等价

### Task 3 — 命令集
22 个命令变体（CreateTask/SetTaskField/SetTaskCompleted/DeleteTask/RestoreTask/…），带完整验证（标题/列表名/标签名非空 + 长度上限）。

### Task 4 — 操作与合并
- 操作：`SetField` / `Delete` / `Restore`
- 字段寄存器（field registers）：每个字段每代一个值 + 版本戳
- 生命周期：generation + deleted，确定性合并（LWW 按 HLC）

### Task 5 — 加密存储
- SQLCipher 加密 SQLite（AES-256，`PRAGMA key`）
- 20 张表 schema（meta、operations、field_registers、tasks、subtasks、lists、tags、tombstones、conflict_history…）
- 版本化迁移（`APPLICATION_ID = "TPTD"`，降级拒绝）

### Task 6 — 原子命令分发
- `dispatch(command)`：单事务内分配 sequence + HLC → 签名 → 插 operations → 物化 → 更新前沿 → commit
- `apply_remote_batch`：成员/签名/缺口/幂等/陈旧代校验，commit 后才 ACK

### Task 7 — 查询与搜索
- Smart list（Inbox/Today/Tomorrow/Next7Days/Completed/自定义列表）+ 稳定游标分页（上限 200）
- FTS5 trigram 搜索（≥3 码点）+ 短查询 bounded scan + HTML 转义 + code-point 高亮
- 日历月视图、回收站、冲突历史

### Task 8 — 恢复与备份
- `ReplicaState = Ready | ReadOnlyLowSpace | Recovering | Unavailable`
- 低空间转只读 + 迟滞恢复；损坏文件隔离（quarantine）
- Backup v1：Argon2id + XChaCha20-Poly1305 认证加密（排除私钥与数据库密钥）

### Task 9 — 公共 API
- `CoreHandle`：dispatch/query/backup/subscribe/runtime_status/close
- 有界订阅（16 队列 + 单调修订号 + 慢消费者丢弃 + 幂等取消）
- `RuntimeStatus`（per-peer reachability/ack）

### Task 10 — UniFFI 契约
- `Core` FFI 对象 + `CoreErrorCode` 14 稳定分类
- `contracts/core-api-version.json` = `{ coreApi:1, schema:1, protocolMajor:1, protocolMinor:0 }`
- `generate-kotlin-bindings.ps1`（版本校验 + 构建 cdylib + uniffi-bindgen 生成 Kotlin）

---

## 6. 数据模型（核心概念）

### 实体
| 实体 | 说明 |
|------|------|
| Task | 任务（title/description/due_date/due_time/priority/completed/list_id/tags） |
| Subtask | 子任务（parent_task_id） |
| List | 列表（name/color/icon） |
| Tag | 标签（name + normalized_name） |

### 操作（CRDT）
```
Operation {
  entity: EntityId          # 目标实体
  kind: Task|Subtask|List|Tag
  parent: Option<EntityId>  # 子任务父级
  stamp: VersionStamp       # generation + HLC + device + operation id
  payload: SetField | Delete | Restore
}
```

### 时钟与 ID
- **HLC**（`physical_millis + logical`）：跨设备因果排序
- **EntityId**：UUIDv7（时间有序，可当分页游标）
- **OperationId**：`(origin_device_id, sequence)`，全局唯一

### 投影（物化表）
操作写入后同步物化到业务表（tasks/subtasks/lists/tags）与字段寄存器，供查询/搜索/日历直接读取，不重新计算合并规则。

---

## 7. 关键设计决策

1. **CRDT 操作日志**：变更先记为不可变操作，再物化为投影；合并确定性（LWW + HLC），无需冲突解决对话框。
2. **单事务分发**：`dispatch` 从分配序列号到提交全在一个 SQLite 事务内，失败即整体回滚。
3. **SQLCipher 文件快照不可用于备份**：`VACUUM INTO` 生成加密副本、`Backup API` 直接报错——最终用 JSON 序列化业务数据做备份快照。
4. **`&Connection` 关联函数**：解决 rusqlite `Transaction` 的借用冲突（事务内传 `&tx`，事务外传 `&conn`）。
5. **订阅背压**：有界 channel + 单调修订号，慢消费者丢事件但可检测 gap 重查询。

---

## 8. 测试与质量

- **99 个测试全绿**，覆盖：领域合并、命令验证、迁移原子性、分发回滚、远端应用（签名/缺口/幂等）、查询分页、搜索高亮、日历、回收站、低空间、损坏恢复、备份往返、订阅、FFI 契约。
- `cargo fmt --all --check` 通过
- `cargo clippy --workspace --all-targets -- -D warnings` 通过

**Android（Plan 4）质量门禁**：

- `scripts/check-android-prerequisites.ps1` 工具链检查器（探测 + 补救，配契约测试）
- `scripts/generate-kotlin-bindings.ps1` 版本契约校验 + cdylib 构建 + Kotlin 绑定生成
- `scripts/build-android.ps1` 四步端到端构建（检查 → 绑定 → `cargo ndk` → `gradle`）
- 契约测试：`check-android-prerequisites.Tests.ps1`、`build-android.Tests.ps1`（fixture 模式，全绿）

---

## 9. 版本历史

| Commit | 内容 |
|--------|------|
| `ffd1586` | 固定并验证共享核心工具链 |
| `a05e7e8` | 建立受限 Rust workspace |
| `0a3315d` | 定义验证命令、身份与时钟 |
| `dfdc979` | 确定性合并与生命周期语义 |
| `d3fb565` | 加密 schema 与原子迁移 |
| `d8d9cf0` | 原子命令分发与持久投影 |
| `af4b794` | 分页任务/搜索/日历/恢复视图 |
| `0f181d4` | 故障安全存储恢复与加密备份 |
| `e68675c` | 修订版本地优先服务 API |
| `4d47bb6` | 冻结共享核心 FFI 契约 |

**Plan 4 — Android 手机/平板**：

| Commit | 内容 |
|--------|------|
| `78dc9c4` | 搭建 Gradle 工程骨架 + 工具链检查器 |
| `32f50d3` | 新增 workspace uniffi-bindgen 工具 crate 并重新生成绑定 |
| `64bb95d` | JNI 原生库加载器与会话包装 |
| `1d4794c` | Keystore 密钥与设备身份提供者 |
| `57886f5` | 仓储层与 UI 领域模型 |
| `0afb338` | Compose 主题、任务列表界面与应用装配 |
| `411af22` | 任务编辑对话框（新建/编辑） |
| `1e2a80e` | 接入生命周期与网络同步触发 |
| `24f3cdf` | mDNS 发现与局域网同步管理器 |
| `e007325` | 端到端构建脚本 + 工具链文档 |
| `da90521` | release 签名配置 + 发布证据矩阵 + gitignore 修正 |

**Plan 5 — Windows MVP 客户端**：

| Commit | 内容 |
|--------|------|
| `b3c4f3f` | 启动 Plan 5 计划文档 |
| `7b12d67` | Tauri 2 工程骨架（Vite + React + MUI） |
| `315cae8` | Rust 命令层 + DPAPI 密钥管理 |
| `4a266d8` | 前端数据层（invoke 封装 + Zustand + TS 类型） |
| `e23667a` | 任务列表 UI（智能列表导航 + 完成切换/删除） |
| `2455ce9` | 搜索与任务编辑（FTS + 编辑对话框） |
| `2a8069e` | 后台 LAN 同步集成（discovery + SyncOrchestrator + 状态指示） |
| `cfacfb9` | 发布构建与签名（tauri build + Authenticode） |
| `88a6381` | Windows 发布证据矩阵 + 构建门禁 |
| `92e9654` | Windows 工具链文档 + PROJECT.md 更新 |
| `01eb645` | 命令层契约测试 + camelCase 字段 serde 修复（统一门禁收尾） |

---

## 10. 下一步计划

| Plan | 内容 | 关键点 |
|------|------|--------|
| **Plan 5** | Windows MVP 客户端 | ✅ 完成，详见 `docs/development/plan5-windows-client.md` |
