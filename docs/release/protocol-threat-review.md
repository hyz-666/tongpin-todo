# 协议与威胁模型一致性评审报告

> 评审日期：2026-09-06　评审人：SeniorDeveloper（AI）
> 范围：`docs/protocol/lan-sync-v1.md` 与 `docs/protocol/security-threat-model.md` 对照 Rust 核心库实现逐条核对。
> 方法：以文档常量/消息/状态机/威胁缓解为锚点，定位到 `todo-protocol` / `todo-crypto` / `todo-domain` / `todo-core` / `todo-discovery` 源码与测试逐项比对。

## 一、结论速览

- 协议与威胁模型**主体实现高度一致**：常量、12 种消息编码、操作签名规范、状态机、协商逻辑、SAS、身份绑定、撤销、去重、验签先于存储、严格 CBOR、DoS 上限等全部有对应实现且与文档吻合。
- 发现 **1 个 P1 缺陷（已修复）** 与 **3 个 P2 文档-实现留白**。

| 级别 | 问题 | 状态 |
|------|------|------|
| P1 | mDNS service type 三处不一致（Android 用 `_tongpin-todo`，Windows/文档用 `_tptodo`），导致跨平台互发现失效 | ✅ 已修复 |
| P2 | discovery hint 窗口「15 分钟」未接线到时间→窗口映射 | 记录待后续实现 |
| P2 | conflict 保留「≥30 天」无清理阈值实现 | 记录待后续实现 |
| P2 | 帧序列号「每方向单调不回绕」运行时强制未在传输层实现（MVP 传输 stub） | 记录待后续实现 |

## 二、协议一致性核对（lan-sync-v1.md）

### 常量表

| 文档项 | 实现 | 结果 |
|--------|------|------|
| Service type `_tptodo._tcp.local` | Windows `_tptodo._tcp.local.`；Android 原 `_tongpin-todo._tcp.` | ⚠️ 不一致（见问题 1，已修） |
| Protocol major 1 / minor 0 | `PROTOCOL_MAJOR=1`（canonical.rs） | ✅ |
| Noise first pairing `Noise_XX_25519_ChaChaPoly_SHA256` | `NOISE_XX` 常量 | ✅ |
| Noise reconnect `Noise_IK_25519_ChaChaPoly_SHA256` | `NOISE_IK` 常量 | ✅ |
| Pairing expiry 120s | `PAIRING_EXPIRY_SECS=120` | ✅ |
| Discovery hint window 15 min（prev/cur/next） | `expected_hints` 生成 3 窗口；但 15min 时长映射未接线 | ⚠️ P2 |
| Connect timeout 5s | `CONNECT_TIMEOUT_MS=5_000` | ✅ |
| Noise handshake timeout 10s | `HANDSHAKE_TIMEOUT_MS=10_000` | ✅ |
| HELLO timeout 5s | `HELLO_TIMEOUT_MS=5_000` | ✅ |
| Heartbeat 10s / dead 30s | `HEARTBEAT_INTERVAL_MS=10_000` / `DEAD_AFTER_MS=30_000` | ✅ |
| Chunk ≤256 ops / ≤512 KiB | `MAX_CHUNK_OPERATIONS=256` / `MAX_CHUNK_SIZE=512*1024` | ✅ |
| Flow-control ≤32 unacked / ≤8 MiB | `DEFAULT_IN_FLIGHT=32` / `DEFAULT_CIPHERTEXT_BUDGET=8*1024*1024` | ✅ |
| Missing-chunk retries 1/2/4/8/16 + full jitter | `CHUNK_RETRY_MS=[1,2,4,8,16]s` + `jittered()` | ✅ |
| Dial backoff 1s,2s,5s,10s,30s,60s,2m,5m cap | `DIAL_BACKOFF_MS=[1,2,5,10,30,60,120,300]s` | ✅ |
| Conflict retention ≥30 days | `conflict_history` 表记录，但无 30 天清理阈值 | ⚠️ P2 |
| Tombstone ≥30 days + 所有活跃成员 ACK | `TOMBSTONE_MIN_AGE_DAYS=30` + `tombstone_collectable(age, all_acked)` | ✅ |

### 消息编码（MessageV1 12 种）

| Code | 消息 | 实现（codec.rs） | 结果 |
|------|------|------|------|
| 0 Hello / 1 HelloAck | `[code, minor, schema, capabilities, device_id(32B)]` | ✅ 字段与顺序一致 | ✅ |
| 2–9 数据类 | `[code, data]` | ✅（VersionSummary/RangeRequest/OperationChunk/Ack/Nack/SnapshotOffer/SnapshotRequest/SnapshotChunk） | ✅ |
| 10 Heartbeat / 11 Close | `[10]` / `[11]` | ✅ | ✅ |

加密帧 `[protocol_major, session_id(16B), sequence, message]` → `encode_frame` 4 字段 CBOR 数组 ✅。

### 操作签名（CanonicalOperationV1）

`[major, entity(16B), kind, parent(16B|null), generation, hlc_physical, hlc_logical, origin_device(32B), origin_sequence, payload]` 十字段 ✅（canonical.rs `encode_operation` 顺序完全一致）。
payload：`SetField=[0,field,value]` / `Delete=[1]` / `Restore=[2]` ✅。
域分隔 `tptodo.operation.v1` → `DOMAIN_SEPARATOR` ✅。

### 状态机

| 文档 | 实现 | 结果 |
|------|------|------|
| 副本 Ready→ReadOnlyLowSpace→Recovering→Unavailable | `ReplicaState`（recovery.rs）+ dispatch 低空间降级 | ✅ |
| peer Offline→Dialing→Handshaking→Syncing\|Backoff\|Incompatible\|Revoked | `SessionState`（session.rs） | ✅ |
| 调度意图 5 态 | `SchedulerIntent`（scheduler.rs） | ✅ |
| 配对 Offered→Connecting→SasPendingBoth→Paired | `PairingState`（pairing.rs） | ✅ |

### 协商

major 不同或 schema 不重叠 → `ProtocolIncompatible` ✅；minor 取小 ✅；capabilities 取交集 ✅；上限取下限 ✅；`1<<63` 必需特性 → `UnknownRequiredFeature` ✅（negotiation.rs）。

## 三、威胁模型核对（security-threat-model.md）

| 威胁 | 缓解实现 | 结果 |
|------|------|------|
| 伪造 mDNS 广播 | 发现提示为 HMAC 衍生值（hint.rs），不授信任 | ✅ |
| 中间人（首次配对） | Noise XX + 六位 SAS（sas.rs，无偏 rejection sampling） | ✅ |
| 中间人（重连） | Noise IK + pinned 静态公钥 | ✅ |
| 密钥替换 | `IdentityBinding{device_id, signing_public, noise_static_public}`（identity.rs） | ✅ |
| 撤销设备重连 | `revoked` 集合 + discovery secret 轮换（revocation.rs rekey envelopes） | ✅ |
| 重放操作 | device+sequence 去重（apply.rs `inserted==false`→duplicated）+ 前沿缺口检测 `OriginGap` | ✅ |
| 帧重放/错序 | Frame 有 sequence 字段，但单调性运行时强制未实现（传输 stub） | ⚠️ P2 |
| 未授权写入 | `apply_remote_batch`：`is_member`→`verify` 先于任何 `insert`/`materialize` | ✅ |
| 畸形/超大 CBOR | 严格解码：拒绝尾随字节、字段数不符、非定长（canonical.rs / codec.rs） | ✅ |
| 拒绝服务 | 帧 64 KiB / chunk 512 KiB / 32 并发 / 8 MiB 上限（limits.rs） | ✅ |
| causal cutoff | `is_post_cutoff(seq, cutoff)` + 重建按 origin 截止（rebuild.rs） | ✅ |

## 四、问题详情

### 问题 1（P1，已修复）：mDNS service type 三处不一致

- 文档：`_tptodo._tcp.local`
- Windows `discovery.rs`：`_tptodo._tcp.local.`（正确，带 mDNS 尾点）
- Android `DiscoveryService.kt`：`_tongpin-todo._tcp.`（**service 名不一致**）

后果：Windows 与 Android 端注册/发现的 DNS-SD 服务名不同，**跨平台互发现失效**，LAN 同步在异构设备间不可用。
修复：Android `SERVICE_TYPE` 改为 `_tptodo._tcp.`（保留 Android 隐含 `.local.` 的平台格式，仅统一 service 名）。

### 问题 2（P2）：discovery hint「15 分钟」窗口时长未接线

`derive_hint`/`expected_hints` 接受抽象 `window: u64`，但「当前时间 → window」的 15 分钟时长映射未在代码中找到，`todo-core` 亦未调用 hint 派生（MVP 阶段发现链路 stub 化）。协议语义正确，仅缺时间分桶接线。

### 问题 3（P2）：conflict 保留「≥30 天」无清理阈值

`conflict_history` 表已记录被替换值，但没有对应 30 天的清理/保留阈值实现（tombstone 有 `TOMBSTONE_MIN_AGE_DAYS=30`，conflict 无同类常量）。

### 问题 4（P2）：帧序列号单调性运行时未强制

`Frame.sequence` 字段存在并编码/解码保留，但「每方向从 0 单调、不回绕」的运行时强制逻辑在传输层缺失（MVP 传输为 stub，`SyncService::sync_peer` 直接驱动编排器至 `Complete`）。

## 五、建议

1. **P1 已修复**，建议后续在 Android 真机上验证 Windows↔Android 互发现。
2. P2 三项均为「协议规范已定义、MVP 传输层未接线」的已知留白，与 `plan5-windows-client.md` 中「Noise 握手 + 操作传输不在 MVP」的边界一致，建议在后续版本（如 Plan 6 传输落地）补齐：
   - hint 窗口 15 分钟时间分桶；
   - conflict_history 30 天清理；
   - 传输层帧序列号单调校验。
