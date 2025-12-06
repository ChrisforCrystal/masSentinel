# Quickstart: Sentinel-Rust Sidecar

## 前置要求

- Rust 1.75+
- Go 1.21+ (用于控制面 Mock)
- Docker & Kubernetes (Kind/Minikube)
- `protoc` (Protocol Buffers Compiler)

## 1. 构建项目

```bash
# 构建 Rust Sidecar
cd mas-sentinel-sidecar
cargo build --release

# 构建 Go Control Plane
cd mas-sentinel-controller
go build -o controller main.go
```

## 2. 本地运行验证 (TCP Proxy)

```bash
# 1. 启动 Sidecar (监听 15001)
./target/release/mas-sentinel-sidecar

# 2. 模拟 iptables 重定向 (需 Root 权限或在 Docker 中测试)
# 建议使用 Docker 环境进行完整测试
```

## 3. 部署到 Kubernetes

```bash
# 1. 构建镜像
docker build -t mas-sentinel-sidecar:v1 -f deploy/Dockerfile.sidecar .

# 2. 部署 Demo 应用 (带 Sidecar)
kubectl apply -f deploy/demo-app.yaml

# 3. 验证流量
kubectl exec -it demo-app -- curl google.com
```
