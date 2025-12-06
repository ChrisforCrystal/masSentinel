# 功能规格说明书: eBPF Traffic Redirection

**版本**: 0.1 | **状态**: Draft | **作者**: Antigravity

## 1. 概述
本特性旨在将 `masSentinel` Sidecar 的流量劫持机制从 `iptables` 迁移至 eBPF。通过在内核层直接处理连接重定向，减少用户态与内核态的切换开销，并消除 `iptables` 规则链匹配带来的延迟，从而实现更高性能的 Service Mesh 数据面。

## 2. 核心需求 (Functional Requirements)

| ID | 描述 (Description) | 优先级 | 备注 |
| :--- | :--- | :--- | :--- |
| **FR-010** | **Transparent Interception**: 系统必须使用 eBPF 程序 (Attach to Cgroup hooks) 拦截所有从应用容器发出的 TCP `connect` 请求。 | P0 | 替代 `iptables -t nat -A OUTPUT` |
| **FR-011** | **Traffic Redirection**: 对于被拦截的流量，系统必须将其目标地址重写为 Sidecar 的监听地址 (Loopback:15001)。 | P0 | 替代 `iptables REDIRECT` |
| **FR-012** | **Original Destination Preservation**: 系统必须保留原始目标 IP 和端口，并提供机制供 Sidecar 应用获取该信息。 | P0 | 替代 `getsockopt(SO_ORIGINAL_DST)` |
| **FR-013** | **Loop Avoidance**: 系统必须能够识别并忽略来自 Sidecar 自身进程的流量，防止无限循环。 | P0 | 替代 `iptables -m owner --uid-owner` |
| **FR-014** | **Protocol Support**: MVP 阶段仅需支持 TCP (IPv4)。 | P0 | IPv6 为 P2 |

## 3. 非功能需求 (Non-Functional Requirements)

| ID | 描述 (Description) | 优先级 | 备注 |
| :--- | :--- | :--- | :--- |
| **NFR-001** | **Performance**: 相比 iptables 方案，eBPF 方案在并发连接建立时的延迟 (P99) 降低至少 10%。 | P1 | Benchmark required |
| **NFR-002** | **Kernel Compatibility**: 必须支持 Linux Kernel 5.10+ (常见的云原生 OS 版本，如 COS, Bottlerocket)。 | P0 | 依赖 CO-RE |
| **NFR-003** | **Observability**: eBPF 程序应暴露基本的计数器 (Metrics)，如拦截连接数。 | P2 | Map counter |

## 4. 接口规范

### 4.1 Sidecar <-> eBPF Map 交互
为了传递原始目标地址，Sidecar 原生代码需查询 eBPF Map。

**Map 定义 (伪代码)**:
```c
struct key {
    u32 cookie; // Socket Cookie
};
struct value {
    u32 original_ip;
    u16 original_port;
};
BPF_MAP_TYPE_LRU_HASH(name=connect_map, key=key, value=value, max_entries=65535);
```

**Sidecar 逻辑**:
1. 接收到新连接 (Accepted Socket)。
2. 获取 Socket Cookie (`getsockopt(SO_COOKIE)`).
3. 使用 Cookie 查询 BPF Map，获取 `original_ip` 和 `original_port`。
4. 如果 Map 中不存在 (fallback)，则尝试 `SO_ORIGINAL_DST` (以便兼容旧模式，可选)。

## 5. 限制与假设
- **假设**: 宿主机内核开启了 BTF 支持 (用于 CO-RE)。
- **限制**: Sidecar 容器需要特权模式 (`securityContext.privileged: true`) 或至少 `CAP_BPF` + `CAP_NET_ADMIN` + 挂载 `/sys/fs/bpf`。
