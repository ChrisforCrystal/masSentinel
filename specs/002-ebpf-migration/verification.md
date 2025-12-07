# eBPF 验证方案

本指南用于验证 masSentinel Sidecar 的 eBPF 流量劫持功能是否正常工作。

## 1. 环境准备

确保您的本地环境满足以下条件：
*   **Kubernetes 集群**: Kind 或 Minikube (需支持挂载宿主机 `/sys/fs/bpf`)。
*   **Docker**: 用于构建镜像。
*   **权限**: 部署账户需有权限创建 Privileged 容器。

## 2. 构建与发布

由于 eBPF 程序需要特殊的编译工具链，请务必使用专门的 Dockerfile 构建。

```bash
# 1. 构建镜像 (包含 eBPF 内核码 + 用户态代理)
docker build -t mas-sentinel-sidecar:ebpf-latest -f deploy/Dockerfile.ebpf .

# 2. (Kind 用户) 将镜像加载到集群
kind load docker-image mas-sentinel-sidecar:ebpf-latest
```

## 3. 部署应用

使用更新后的部署清单，它包含了特权模式配置和各级挂载。

```bash
# 部署 Demo 应用和 Sidecar
kubectl apply -f deploy/demo-app-ebpf.yaml

# 等待 Pod 启动
kubectl wait --for=condition=ready pod -l app=mas-sentinel-demo-ebpf --timeout=60s
```

## 4. 验证清单

### A. 验证 eBPF 加载 (Sidecar Logs)
查看 Sidecar 日志，确认 BPF 程序成功挂载。

```bash
kubectl logs -l app=mas-sentinel-demo-ebpf -c mas-sentinel-sidecar
```

**预期日志**:
> `INFO mas_sentinel_sidecar::bpf: Loading eBPF program from /app/mas-sentinel-ebpf`
> `INFO mas_sentinel_sidecar::bpf: eBPF connect4 attached to /sys/fs/cgroup`

### B. 验证流量拦截 (Traffic Interception)
查看 Sidecar 日志，确认是否通过 Map 成功获取了原始目标地址。

```bash
# 持续观察日志
kubectl logs -f -l app=mas-sentinel-demo-ebpf -c mas-sentinel-sidecar
```

**预期日志 (当业务容器 curl google.com 时)**:
> `DEBUG mas_sentinel_sidecar::proxy::outbound: Found original dst via eBPF Map: 142.250.x.x:80`
> `INFO mas_sentinel_sidecar::proxy::outbound: Intercepted connection to 142.250.x.x:80`

如果看到 "Fallback to SO_ORIGINAL_DST"，说明 eBPF 拦截失败或 Map 查询失败。

### C. 验证业务连通性 (App Logs)
确保业务容器的网络请求没有因为拦截而中断。

```bash
kubectl logs -l app=mas-sentinel-demo-ebpf -c demo-app
```

**预期日志**:
> `Accessing Google...`
> `HTTP/1.1 200 OK` (或 301 Moved)

## 5. 故障排查

如果 Sidecar 启动失败或报错 "Permission denied":
1.  检查 `deploy/demo-app-ebpf.yaml` 中是否开启了 `privileged: true`。
2.  检查宿主机是否挂载了 cgroup v2 (`mount | grep cgroup2`)。

如果拦截不生效 (日志无反应):
1.  确认 Pod 的 UID 是否为 `1337` (Sidecar) 和 非 `1337` (App)。
2.  进入容器查看 BPF Map 状态 (需安装 bpftool):
    `bpftool map dump name CONNECT_ORIG_DST`
