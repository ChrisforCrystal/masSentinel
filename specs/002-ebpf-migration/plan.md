# 实施计划: eBPF Traffic Redirection (Migration from iptables)

**分支**: `002-ebpf-migration` | **日期**: 2025-12-07 | **规格**: [spec.md](./spec.md)
**输入**: `specs/001-sentinel-rust-sidecar` (Existing iptables implementation)

## 摘要

本项目旨在通过 **eBPF (Extended Berkeley Packet Filter)** 技术替换现有的 `iptables` 流量劫持方案。
目前 Sidecar 使用 `iptables` 的 `REDIRECT` 模式将业务容器的出站流量重定向到 Sidecar 监听端口 (15001)。尽管成熟，但 `iptables` 在高并发下存在性能瓶颈（如 conntrack 开销、规则线性匹配）。
使用 eBPF 可以更高效地在内核层完成流量重定向，极大降低延迟，并为未来的高级可观测性打下基础。

## 技术上下文

**语言/工具链**: 
- **eBPF 开发框架**: Rust (`aya` 或 `libbpf-rs`)。考虑到 Sidecar 是 Rust 编写，优先使用 `aya` 以保持工具链统一 (Pure Rust)。
- **内核要求**: Linux Kernel 5.4+ (需支持 BPF CO-RE, sock_ops 等)。
  > [!NOTE]
  > 已验证本地 Kind 环境 (OrbStack) 运行 Kernel 6.17，且开启了必要 BPF 配置，完全支持开发验证。
- **Hook 点**: 
    - `cgroup/connect4`: 用于拦截并重写出站连接的目标地址 (connect syscall)。
    - `sock_ops`: (可选) 用于更深层次的 socket 重定向或加速 (sockmap)。
    - `getsockopt`: (可选) 用于 Sidecar 获取原始目标地址，或者通过 eBPF Map 传递。

**主要挑战**:
1.  **Original Destination**: `iptables` 自动保留了原始目标地址 (`SO_ORIGINAL_DST`)。eBPF 重写 destination IP 后，Sidecar 需要一种机制获取原始目标地址。通常做法是 eBPF 将 `(cookie, local_port)` -> `original_dst` 存入 BPF Map，Sidecar 查询 Map。
2.  **兼容性**: 需确保在 Kubernetes 环境下与 CNI 插件兼容。

## 关键变更

### 1. eBPF Program Implementation
- **Loader**: 集成到 Sidecar 启动流程中，自动加载 eBPF 程序。
- **Kernel Program**: 
    - 挂载到 cgroup (`connect4` hook)。
    - logic: 检查 `skb->protocol` == TCP。
    - logic: 忽略 Sidecar 自身流量 (Based on UID or Socket Cookie to avoid loops)。
    - logic: 将目标地址重写为 `127.0.0.1:15001`。
    - logic: 保存原始目标地址到 LRU Map。

### 2. Sidecar Logic Update
- 移除 `SO_ORIGINAL_DST` 系统调用获取原始地址的逻辑（或保留作为 fallback）。
- 新增查询 eBPF Map 获取原始目标地址的逻辑。

### 3. Deployment Update
- 移除 `init-iptables.sh` 或将其改为 `init-ebpf` (加载器)。
- Sidecar 容器可能需要更高的权限 (`CAP_BPF`, `CAP_PERFMON`, `CAP_NET_ADMIN`)，通常需要特权模式或特定的 Capabilities。

## 章程检查 (Constitution Check)

- [x] **I. 零侵入性**: 维持对业务代码的零侵入。
- [x] **II. 高性能**: eBPF 预期比 iptables 拥有更低的 overhead。
- [x] **III. Kubernetes 原生**: 需注意特权容器的安全性配置。

## 项目结构 (预计)

```text
mas-sentinel-sidecar/
├── src/
│   ├── bpf/               # eBPF loader & userspace logic
│   ├── proxy/
│   │   └── outbound.rs    # Update to support Map lookup
├── ebpf/                  # eBPF Kernel Code (Rust via Aya)
│   ├── src/
│   │   └── main.rs        # eBPF program logic
│   └── Cargo.toml
```

## 任务拆解

详见 [tasks.md](./tasks.md)
