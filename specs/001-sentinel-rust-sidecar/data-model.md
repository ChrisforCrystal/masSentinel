# Data Model: Sentinel-Rust Sidecar

## Entities

### FlowRule (流控规则)

对应 Sentinel 的流控规则。

| Field | Type | Description |
|-------|------|-------------|
| `resource` | String | 资源名称 (e.g., "google.com", "/api/v1/users") |
| `count` | Float | 限流阈值 (QPS) |
| `grade` | Enum | 限流模式 (1: QPS, 0: Thread) |
| `limit_app` | String | 调用来源限制 (default: "default") |
| `strategy` | Enum | 调用关系限流策略 (Direct, Related, Chain) |
| `control_behavior` | Enum | 流控效果 (Reject, WarmUp, RateLimiter) |

### SidecarConfig (Sidecar 配置)

| Field | Type | Description |
|-------|------|-------------|
| `app_name` | String | 应用名称 |
| `control_plane_addr` | String | 控制平面地址 |
| `sidecar_port` | Integer | Sidecar 监听端口 (default: 15001) |

## State Transitions

- **Rule Update**: Control Plane -> gRPC Stream -> Sidecar -> Sentinel Core (Update Rules)
