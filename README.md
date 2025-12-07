# Sentinel-Rust Sidecar (Mesh Lite)

A lightweight, non-intrusive traffic governance sidecar built with Rust and Sentinel.

## Features

- **Transparent Interception**: Uses iptables to redirect outbound TCP traffic to the sidecar.
- **Sentinel Governance**: Enforces flow control rules (QPS) using `sentinel-core`.
- **Dynamic Configuration**: Fetches rules from a Go-based Control Plane via gRPC.
- **High Performance**: Built on Rust, Tokio, and Hyper.

## Build & Run

### Prerequisites

- Rust 1.75+
- Go 1.21+
- Protobuf Compiler (`protoc`)

### 1. Build Sidecar (Rust)

```bash
cd mas-sentinel-sidecar
cargo build --release
```

### 2. Build Control Plane (Go)

```bash
cd mas-sentinel-controller
make proto
go build -o bin/controller main.go
```

### 3. Run Locally

Start Control Plane:
```bash
./mas-sentinel-controller/bin/controller
```

Start Sidecar:
```bash
./mas-sentinel-sidecar/target/release/mas-sentinel-sidecar
```

### 4. Deploy to Kubernetes

See `deploy/` directory for Dockerfile and Kubernetes manifests.

```bash
# Build image
docker build -t mas-sentinel-sidecar:v1 -f deploy/Dockerfile.sidecar .

# Deploy
kubectl apply -f deploy/demo-app.yaml
```

## Verification (验证)

### 1. Verify Sentinel Rate Limiting (验证限流)
The Sidecar is configured with a hardcoded rule: **Limit `1.1.1.1` to 1 QPS**.

You can run the provided script to verify this behavior:
```bash
chmod +x verify_limit.sh
./verify_limit.sh
```

**Expected Output:**
```text
Target Pod: mas-sentinel-demo-ebpf-xxxxx
Sending 5 requests to 1.1.1.1...
Request 1:
200
Request 2:
429
Request 3:
429
...
```
- **200**: Request passed (1 QPS bucket).
- **429**: Request blocked by Sidecar (Sentinel Rule).

### 2. Verify eBPF Scoping (验证隔离性)
To ensure the Sidecar only affects its own container:
```bash
# Check Sidecar logs for container cgroup detection
kubectl logs -l app=mas-sentinel-demo-ebpf -c mas-sentinel-sidecar | grep "Detected cgroup"
```
