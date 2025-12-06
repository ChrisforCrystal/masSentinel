# 实施计划: Sentinel-Rust Sidecar (Mesh Lite)

**分支**: `001-sentinel-rust-sidecar` | **日期**: 2025-12-06 | **规格**: [spec.md](./spec.md)
**输入**: 功能规格说明书 `/specs/001-sentinel-rust-sidecar/spec.md`

**注意**: 本模板由 `/speckit.plan` 命令填充。参见 `.specify/templates/commands/plan.md` 了解执行工作流。

## 摘要

本项目旨在构建一个轻量级、无侵入的流量治理 Sidecar，基于 Rust 实现。核心功能包括透明流量拦截（iptables）、Sentinel 流量控制（限流/熔断）以及指标上报。Sidecar 将作为 Kubernetes Pod 中的辅助容器运行，通过 iptables 劫持业务容器的出站流量，并在转发前执行 Sentinel 规则检查。

## 技术上下文

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**语言/版本**: Rust 1.75+ (Edition 2021)
**主要依赖**: 
- `tokio` (Async Runtime)
- `hyper` (HTTP)
- `tonic` (gRPC)
- `sentinel-rs` (Traffic Governance)
- `libc` / `socket2` (System Calls)
**存储**: N/A (Stateless Sidecar)
**测试**: `cargo test` (Unit), Docker Compose / Kind (Integration)
**目标平台**: Linux (Kubernetes Pod), x86_64/arm64
**项目类型**: System Software / Network Proxy
**性能目标**: < 1ms P99 Latency Overhead, < 50MB Memory
**约束**: 必须处理 iptables 重定向流量 (SO_ORIGINAL_DST)，必须兼容 Sentinel Dashboard。
**规模/范围**: MVP 阶段支持 TCP 流量和基础流控规则。

## 章程检查 (Constitution Check)

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. 零侵入性**: 使用 iptables 透明拦截，无需修改业务代码。
- [x] **II. 高性能与低开销**: 使用 Rust + Tokio 实现，目标极低延迟。
- [x] **III. Sentinel 生态兼容性**: 计划集成 `sentinel-rs` 并对接 Sentinel Dashboard。
- [x] **IV. 故障安全运行**: 需设计 Fail-open 机制（虽然 MVP 可能先关注核心功能，但架构需预留）。
- [x] **V. Kubernetes 原生**: 部署方式为 Sidecar 注入。

## 项目结构

### 文档 (本功能)

```text
specs/001-sentinel-rust-sidecar/
├── plan.md              # 本文件
├── research.md          # Phase 0 输出
├── data-model.md        # Phase 1 输出
├── quickstart.md        # Phase 1 输出
├── contracts/           # Phase 1 输出
└── tasks.md             # Phase 2 输出
```

### 源代码 (仓库根目录)

```text
# Rust Workspace Structure
Cargo.toml
mas-sentinel-sidecar/       # Rust Sidecar crate
├── src/
│   ├── main.rs            # Entry point
│   ├── proxy/             # TCP Proxy logic
│   │   ├── mod.rs
│   │   └── inbound.rs     # (Future)
│   │   └── outbound.rs    # Core interception logic
│   ├── sentinel/          # Sentinel integration
│   │   ├── mod.rs
│   │   └── rules.rs       # Rule management
│   ├── config/            # Configuration & Control Plane Client
│   │   ├── mod.rs
│   │   └── grpc.rs        # gRPC Client for ConfigService
│   └── utils/             # Helper functions (e.g., socket opts)
└── tests/                 # Integration tests

mas-sentinel-controller/    # Go Control Plane (Mock/Simple)
├── main.go
├── api/
│   └── v1/
│       └── config.proto   # gRPC definition
└── pkg/
    └── server/            # gRPC Server implementation

deploy/                    # Kubernetes manifests
├── sidecar.yaml
├── demo-app.yaml
└── init-iptables.sh       # Traffic interception script
```

**结构决策**: 采用 Rust Workspace (如果未来有更多 Rust 组件) 或独立 Crate。由于控制面是 Go，可能是一个混合仓库。暂定 `mas-sentinel-sidecar` 为 Rust 项目根目录。

## 复杂度跟踪

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| N/A | | |
