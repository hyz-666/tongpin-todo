# 发布清单（Release Checklist）

> tongpin-todo 跨设备发布前的质量门禁清单。每项通过后方可打发布 tag。

## 一、质量门禁（scripts/verify.ps1）

| # | 检查项 | 命令 | 通过标准 |
|---|--------|------|---------|
| 1 | 代码格式 | `cargo fmt --all --check` | 无差异 |
| 2 | 静态检查 | `cargo clippy --workspace --all-targets -- -D warnings` | 零警告 |
| 3 | 单元/集成测试 | `cargo test --workspace` | 全部通过 |

一键运行：`pwsh -File scripts/verify.ps1`（退出码 0 = 通过）。

## 二、覆盖率（scripts/coverage.ps1）

| 检查项 | 命令 | 阈值 |
|--------|------|------|
| 行覆盖率 | `pwsh -File scripts/coverage.ps1` | ≥ 80% |

前置：`rustup component add llvm-tools-preview` + `cargo install cargo-llvm-cov`。

## 三、依赖审计（scripts/audit.ps1）

| 检查项 | 命令 | 通过标准 |
|--------|------|---------|
| RustSec 漏洞 | `cargo audit` | 零 advisory |
| 许可证 / ban | `cargo deny check` | 零违规 |

一键运行：`pwsh -File scripts/audit.ps1`。
前置：`cargo install cargo-audit` + `cargo install cargo-deny`。

## 四、软件物料清单（scripts/sbom.ps1）

| 检查项 | 命令 | 产物 |
|--------|------|------|
| CycloneDX SBOM | `pwsh -File scripts/sbom.ps1` | `.release/sbom.cdx.json` |

前置：`cargo install cargo-cyclonedx`。

## 五、发布构建与签名（scripts/release.ps1）

| 检查项 | 命令 | 产物 |
|--------|------|------|
| 优化构建 | `cargo build --release --workspace` | `target/release/*` |
| 代码签名 | `pwsh -File scripts/release.ps1 -Certificate <pfx>` | 已签名 exe |

签名需代码签名证书（.pfx/.p12）；未提供证书时脚本跳过签名并提示。

## 六、跨设备一致性（已由自动化测试覆盖）

| 检查项 | 测试 | 通过标准 |
|--------|------|---------|
| 三副本收敛 | `crates/todo-core/tests/three_replica_sync.rs` | 全绿 |
| 敌对网络故障 | `crates/todo-core/tests/network_fault_matrix.rs` | 全绿 |
| 10k 操作确定性 | `crates/todo-core/tests/regressions/sync_seeds.rs` | 全绿 |
| 真实实例端到端 | `crates/todo-core/tests/end_to_end_sync.rs` | 全绿 |

## 七、发布前人工确认

- [x] `contracts/core-api-version.json` 版本号已确认（`coreApi: 1` 与命令层契约测试一致）
- [x] `docs/protocol/lan-sync-v1.md` 协议文档与实现一致（已逐条核对，见 `protocol-threat-review.md`；修复 mDNS service type 1 处）
- [x] `docs/protocol/security-threat-model.md` 威胁模型已评审（见 `protocol-threat-review.md`）
- [ ] 物理设备证据矩阵（见 `device-evidence-matrix.md`）已填写
- [x] Git tag 已打（`v0.1.0`）并推送 `origin/main`

## 八、Android 构建门禁（Plan 4）

| 检查项 | 命令 | 通过标准 |
|--------|------|---------|
| 工具链检查 | `pwsh -NoProfile -File scripts/check-android-prerequisites.ps1` | 退出码 0 |
| 端到端构建（debug） | `pwsh -NoProfile -File scripts/build-android.ps1 -Configuration Debug` | 四步全 PASS |
| 端到端构建（release） | `pwsh -NoProfile -File scripts/build-android.ps1 -Configuration Release` | 四步全 PASS |
| 契约测试 | `scripts/tests/check-android-prerequisites.Tests.ps1`、`scripts/tests/build-android.Tests.ps1` | 全绿 |

详细证据见 `docs/release/android-release-evidence-matrix.md`。

## 九、Windows 构建门禁（Plan 5）

| 检查项 | 命令 | 通过标准 |
|--------|------|---------|
| 前端构建 | `cd apps/windows && npm run build` | tsc + vite 零错误 |
| Rust 命令层编译 | `cargo check -p tongpin-windows` | 零错误 |
| 单元/集成测试 | `cargo test -p tongpin-windows` | 全绿 |
| 发布构建与签名 | `pwsh -NoProfile -File scripts/build-windows.ps1 [-Certificate <pfx>]` | BUILD 步骤 PASS（`signed` 视证书而定） |
| 契约测试 | `pwsh -NoProfile -File scripts/tests/build-windows.Tests.ps1` | PASS，FAIL=0 |

详细证据见 `docs/release/windows-release-evidence-matrix.md`。

## 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 0.1.0 | 2026-08-30 | 初始发布清单（Plan 3 建立） |
| 0.1.0 | 2026-09-01 | 新增 Android 构建门禁一节（Plan 4 Task 10） |
| 0.1.0 | 2026-09-05 | 新增 Windows 构建门禁一节（Plan 5 Task 8） |
| 0.1.0 | 2026-09-06 | Plan 5 收尾：统一门禁全绿，打 `v0.1.0` 发布 tag |
| 0.1.0 | 2026-09-06 | 协议与威胁模型一致性评审完成；修复 mDNS service type 跨平台不一致 |
