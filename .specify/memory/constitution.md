<!--
SYNC IMPACT REPORT
Version Change: [NEW] 1.0.0
Modified Principles:
- Added: I. 零侵入性 (Zero Intrusiveness)
- Added: II. 高性能与低开销 (High Performance & Low Overhead)
- Added: III. Sentinel 生态兼容性 (Sentinel Ecosystem Compatibility)
- Added: IV. 故障安全运行 (Fail-Safe Operation)
- Added: V. Kubernetes 原生 (Kubernetes Native)
Added Sections:
- 技术栈与架构 (Technology Stack & Architecture)
- 开发与质量 (Development & Quality)
Templates Requiring Updates:
- None (Standard templates apply)
-->
# Sentinel-Rust Sidecar (Mesh Lite) 章程

## 核心原则

### I. 零侵入性 (Zero Intrusiveness)
Sidecar 必须在无需修改业务应用代码的情况下运行。流量拦截通过 iptables/eBPF 透明实现。应用对 Sidecar 的存在无感知。这确保了治理层与业务逻辑的解耦。

### II. 高性能与低开销 (High Performance & Low Overhead)
使用 Rust 和异步 I/O (Tokio/Hyper) 实现。Sidecar 引入的请求路径延迟必须极低（目标 < 1ms 开销）。资源使用（CPU/内存）必须保持在低水平，以便部署在每个 Pod 旁而不产生显著的成本影响。

### III. Sentinel 生态兼容性 (Sentinel Ecosystem Compatibility)
必须完全支持 Sentinel 控制平面（控制台）和数据平面协议。规则配置（限流、熔断）和指标上报必须与现有的 Sentinel 基础设施兼容。尽可能复用现有的 Sentinel 概念和数据模型。

### IV. 故障安全运行 (Fail-Safe Operation)
Sidecar 绝不能成为单点故障。如果 Sidecar 遇到内部错误、崩溃或高负载，流量理想情况下应绕过它（Fail-open）或被优雅处理，以最大限度地减少对业务可用性的影响。稳定性至关重要。

### V. Kubernetes 原生 (Kubernetes Native)
专为 Kubernetes 环境设计。注入通过 Mutating Webhook 处理。配置应在适用的情况下利用 K8s 原生机制（CRDs, ConfigMaps），同时保持与 Sentinel 控制台的兼容性。

## 技术栈与架构

**技术栈**:
- **语言**: Rust (Edition 2021+)
- **异步运行时**: Tokio
- **HTTP/网络**: Hyper, Tower
- **流量拦截**: iptables (通过 Init Container)
- **控制平面**: Sentinel Dashboard (现有)
- **部署**: Kubernetes Sidecar (Mutating Webhook)

**架构上下文**:
- **数据平面**: Rust Sidecar 拦截来自业务容器的出站流量。
- **控制平面**: Sidecar 从 Sentinel Dashboard/Nacos 拉取规则并推送指标。
- **拦截**: `iptables` 将流量重定向到 Sidecar 端口（例如 15001）。

## 开发与质量

**测试策略**:
- **单元测试**: 核心逻辑（规则匹配、令牌桶算法）的高覆盖率。
- **集成测试**: 验证网络代理、规则效果和控制台交互。
- **基准测试**: 定期进行性能基准测试，确保满足延迟和吞吐量目标。

**代码质量**:
- **Linting**: `cargo clippy` 必须通过且无警告。
- **格式化**: 必须应用 `cargo fmt`。
- **安全性**: 尽量减少 `unsafe` 代码的使用；如有必要，必须经过严格的理由说明和审查。

## 治理

本章程定义了 Sentinel-Rust Sidecar 项目不可协商的架构和设计原则。所有贡献必须遵守这些原则。

**修订流程**:
- 对本章程的更改需要正式提案并经项目维护者审查。
- 版本控制遵循语义化版本 (MAJOR.MINOR.PATCH)。
- 原则的破坏性更改需要提升 MAJOR 版本。

**版本**: 1.0.0 | **批准日期**: 2025-12-06 | **最后修订**: 2025-12-06
