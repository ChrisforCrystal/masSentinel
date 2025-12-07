# Building eBPF Program

## Prerequisites

1. **Rust Nightly Toolchain**:
   ```bash
   rustup toolchain install nightly
   rustup component add rust-src --toolchain nightly
   ```

2. **bpf-linker**:
   ```bash
   cargo install bpf-linker
   ```

## Build

To build the eBPF kernel program:

```bash
cargo +nightly build --package mas-sentinel-ebpf --target bpfel-unknown-none -Z build-std=core --release
```

The binary will be at: `target/bpfel-unknown-none/release/mas-sentinel-ebpf`

## Run

Run the sidecar (userspace) with privileges (sudo) to load the eBPF program:

```bash
sudo ./target/release/mas-sentinel-sidecar
```

## Architecture Summary (SockOps Migration)

We use a **3-Phase Correlation Mechanism** to support concurrent traffic interception without race conditions.

### 1. Data Structures
*   **`COOKIE_ORIG_DST` (Kernel-Internal)**: Temporary staging map keyed by **Socket Cookie**.
*   **`CONNECT_ORIG_DST` (Shared)**: Final lookup map keyed by **Source IP + Port**.

### 2. Execution Flow
1.  **Intercept (`connect4`)**:
    *   Intercepts `connect()` syscall.
    *   Saves `OriginalDst` to `COOKIE_ORIG_DST` using the **Socket Cookie**.
    *   Redirects traffic to Sidecar (`127.0.0.1:15001`).

2.  **Bridging (`handle_sockops`)**:
    *   Triggered when TCP State becomes `ESTABLISHED`.
    *   Reads `OriginalDst` from `COOKIE_ORIG_DST` (using Cookie).
    *   Writes `OriginalDst` to `CONNECT_ORIG_DST` using **Source IP + Port** (Network Byte Order).

3.  **Proxy (Sidecar)**:
    *   Accepts the connection.
    *   Looks up `OriginalDst` in `CONNECT_ORIG_DST` using the client's **Source IP + Port**.
