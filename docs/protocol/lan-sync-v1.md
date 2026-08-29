# LAN Sync Protocol v1

## 概览

无服务器的局域网多主同步协议。所有变更表示为带签名的**操作**（operation），通过版本摘要交换只传输缺失部分，ACK 只在持久化后推进。

## 常量

```text
Service type                  _tptodo._tcp.local
Protocol                      major 1, minor 0
Noise first pairing           Noise_XX_25519_ChaChaPoly_SHA256
Noise reconnect               Noise_IK_25519_ChaChaPoly_SHA256
Pairing expiry                120 seconds
Discovery hint window         15 minutes; accept previous/current/next
Connect timeout               5 seconds
Noise handshake timeout       10 seconds
Authenticated HELLO timeout   5 seconds
Heartbeat                     every 10 seconds; dead after 30 seconds
Chunk limits                  <=256 operations and <=512 KiB encoded
Flow-control default          <=32 unacked chunks and <=8 MiB ciphertext
Missing-chunk retries         about 1/2/4/8/16 seconds with full jitter
Dial backoff                  1s,2s,5s,10s,30s,60s,2m, then 5m cap
Conflict retention            at least 30 days
Tombstone collection          at least 30 days plus every active member ACK
```

## 消息

`MessageV1` 共 12 种，编码为固定字段 CBOR 数组：

| Code | 消息 | 载荷 |
|------|------|------|
| 0 | `Hello` | `[0, protocol_minor, schema, capabilities, device_id(32B)]` |
| 1 | `HelloAck` | `[1, protocol_minor, schema, capabilities, device_id(32B)]` |
| 2 | `VersionSummary` | `[2, data]` |
| 3 | `RangeRequest` | `[3, data]` |
| 4 | `OperationChunk` | `[4, data]` |
| 5 | `Ack` | `[5, data]` |
| 6 | `Nack` | `[6, data]` |
| 7 | `SnapshotOffer` | `[7, data]` |
| 8 | `SnapshotRequest` | `[8, data]` |
| 9 | `SnapshotChunk` | `[9, data]` |
| 10 | `Heartbeat` | `[10]` |
| 11 | `Close` | `[11]` |

加密帧：`[protocol_major, session_id(16B), sequence, message]`，2 字节长度前缀 + Noise 密文。

## 操作签名

`sign_operation` 对 `tptodo.operation.v1 || canonical_unsigned_bytes` 签名（Ed25519）。
`CanonicalOperationV1` 是 10 字段定序 CBOR 数组：

```text
[protocol_major, entity(16B), kind, parent(16B|null), generation,
 hlc_physical, hlc_logical, origin_device(32B), origin_sequence, payload]
```

payload：`SetField = [0, field, value]`、`Delete = [1]`、`Restore = [2]`。

## 状态转换

- **副本**：`Ready → ReadOnlyLowSpace → Recovering → Unavailable`
- **每 peer 连接**：`Offline → Dialing → Handshaking → Syncing | Backoff | Incompatible | Revoked`
- **调度意图**：`ForegroundActive | WindowsTray | AndroidFgs | OsDeferred | ProcessStopping`
- **配对**：`Offered → Connecting → SasPendingBoth → Paired`（可 `Cancelled`/`Expired`/`Failed`）

## 协商

- major 不同或 schema 不重叠 → `ProtocolIncompatible`，不应用任何数据。
- minor 取双方较小值；capabilities 取交集；资源上限取下限。
- 最高位（1<<63）标记**必需**特性：任一方缺失该位 → `UnknownRequiredFeature`。

## 兼容策略

主版本不兼容时拒绝连接；次版本向后兼容（取小）。未知必需特性拒绝，未知可选特性忽略。

## 不变量

1. ACK 只在验证 + 幂等应用 + 前沿更新 + 事务提交后推进。
2. 压缩只在覆盖水位线的固定快照之后进行。
3. 长离线的未撤销成员阻塞墓碑回收——仅凭时间不足以删除。
4. 网络可用性永不阻塞本地的有效编辑。
