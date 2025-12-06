# Research: Sentinel-Rust Sidecar

## 1. SO_ORIGINAL_DST 获取原地址

**Decision**: 使用 `libc` 或 `socket2` crate 调用 `getsockopt` 获取 `SO_ORIGINAL_DST`。
**Rationale**: iptables REDIRECT 模式下，原始目标地址被保存在 socket 选项中。这是标准 Linux 网络编程做法。
**Implementation**:
在 Rust 中，对于 `TcpStream` (Tokio)，需要获取其 RawFd，然后调用 `getsockopt`。
```rust
// 伪代码
let raw_fd = stream.as_raw_fd();
let addr = nix::sys::socket::getsockopt(raw_fd, nix::sys::socket::sockopt::OriginalDst)?;
```
需要验证 `nix` crate 是否支持 `OriginalDst`，或者直接使用 `libc`。

## 2. Sentinel-rs 集成

**Decision**: 使用 `sentinel-rs` crate。
**Rationale**: 官方 Rust 实现，提供流控、熔断等核心功能。
**Unknowns**:
- `sentinel-rs` 的 API 是否稳定？
- 如何动态加载规则？(通常通过 DataSource 接口)
- 与 Tokio 异步运行时的兼容性？(应该是支持的)

## 3. iptables 拦截脚本

**Decision**: 使用标准的 Istio 风格 iptables 规则。
**Rationale**: 成熟可靠。
**Rules**:
- Create chain `SENTINEL_OUTPUT`
- Output chain jumps to `SENTINEL_OUTPUT`
- Ignore UID 1337 (Sidecar)
- Redirect TCP to 15001

## 4. gRPC 协议定义

**Decision**: 定义简单的 `ConfigService`。
**Rationale**: MVP 阶段不需要复杂的 xDS 协议，只需简单的规则同步。
**Proto**:
```protobuf
syntax = "proto3";
package config.v1;

service ConfigService {
  rpc WatchConfig (WatchRequest) returns (stream ConfigResponse);
}

message WatchRequest {
  string app_name = 1;
}

message ConfigResponse {
  repeated FlowRule rules = 1;
}

message FlowRule {
  string resource = 1;
  double count = 2;
  // ... other fields
}
```
